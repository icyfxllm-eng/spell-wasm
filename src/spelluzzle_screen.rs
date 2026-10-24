//! CC-SPELLUZZLE v1 F2, F6, F7, F8, F11, F13 -- the screen.
//!
//! Renders `spelluzzle::play::Play` and forwards taps; it decides nothing. Every
//! rule lives in `src/spelluzzle/`. This file owns what a pure core cannot: the
//! DOM, the audio (through the resolver, plus `api::prefetch_set` so no board is
//! shown with a word that cannot be played), the device storage, and the clock
//! that seeds a board.
//!
//! * R6 / I9: no telemetry and no request other than audio. There is no
//!   `set_mode` call and no event here.
//! * I8: the only keys written are the four in `spelluzzle::store`.
//! * Top-left exit on every screen of the mode; it is instant, because progress
//!   is saved on every action.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::boardgame_input::{self as input, Key};
use crate::spelluzzle::{
    bank,
    duel::{self, Duel, How, NpcDone, Opponent},
    fresh::{fresh_board, fresh_par, fresh_run, fresh_verified},
    gen::generate_after,
    lex::Lexicon,
    offer::{self, BoardType},
    mywords,
    par::{self, ParRecord},
    run::{self, RunRecord},
    pencil::{self, Pencil},
    play::{Outcome, Play, Snapshot},
    render, seeds,
    store::{self, BestStars, History, MyWordsSave, PencilSave, Prefs, Progress, RunSave, RunStore, Streak},
    types::{Tier, GEN_VERSION},
    view::{board_view, RuneState},
};
use std::collections::BTreeSet;
use crate::spelluzzle::types::Board;
use crate::{audio_boost, dom, feedback, flags, haptics, i18n, speech_out, storage, App};

const HOW_KEY: &str = "spell_spz_how_v1";
/// F13: a second card on the first board that has a silent word.
const HOW2_KEY: &str = "spell_spz_how2_v1";
/// F11: discard a board and draw another this many times when a clip will not load.
const AUDIO_TRIES: u32 = 3;

struct Ui {
    play: Play,
    lang: String,
    kid: bool,
    tier: Tier,
    seed: u64,
    previous: Vec<String>,
    finished: bool,
    keys: Vec<Vec<Key>>,
    /// F16: the player's own notes. Drawn over the board, never part of it (I14).
    pencil: Pencil,
    /// F16: the rune a long-press is waiting to mark.
    target: Option<u8>,
    /// F17: the rune whose cells are lit.
    highlight: Option<u8>,
    /// F18: runes the ripple is still holding back.
    pending: BTreeSet<u8>,
    /// F15: the record a Par board was built from (None on a Classic board).
    par_rec: Option<ParRecord>,
    /// F19: the Expedition this board belongs to.
    run: Option<RunState>,
    /// F21: a My Words board's own words, kept whole for saving.
    mw: Option<MyWordsSave>,
    /// F20: a Rune Duel in play (the `play` field is then an unused placeholder).
    duel: Option<DuelUi>,
}

/// A Duel on the table: the rules object plus what the player has half done.
struct DuelUi {
    d: Duel,
    selected: Option<usize>,
    /// The action chosen for the selected word, once the player has chosen one.
    mode: Option<How>,
    draft: Vec<char>,
    /// The last thing that happened, shown as a one-line verdict (D23).
    note: String,
}

/// An Expedition in play: three boards, one of them on screen.
struct RunState {
    rec: RunRecord,
    boards: Vec<Board>,
    snaps: Vec<Snapshot>,
    stars: Vec<u8>,
    done: Vec<bool>,
    cur: usize,
}

fn pencil_on() -> bool {
    flags::spelluzzle_pencil()
}
fn strip_on() -> bool {
    flags::spelluzzle_strip()
}
fn ripple_on() -> bool {
    flags::spelluzzle_ripple()
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LEX: RefCell<Option<(String, std::rc::Rc<Lexicon>)>> = const { RefCell::new(None) };
    static CTX: RefCell<(String, bool, Tier)> = RefCell::new((String::new(), false, Tier::Easy));
    static TOKEN: Cell<u32> = const { Cell::new(0) };
    static COUNTER: Cell<u64> = const { Cell::new(0) };
    /// Long-press timer token, and whether the last press fired (so its click is swallowed).
    static LP: Cell<u32> = const { Cell::new(0) };
    static LP_FIRED: Cell<bool> = const { Cell::new(false) };
    /// Ripple token: bumped to cancel a running ripple.
    static RIPPLE: Cell<u32> = const { Cell::new(0) };
    /// F22: the board type the start screen is set to, and the Par record in flight to `show`.
    static BTYPE: Cell<BoardType> = const { Cell::new(BoardType::Classic) };
    static PAR_REC: RefCell<Option<ParRecord>> = const { RefCell::new(None) };
    /// The run and My Words state in flight to `show`, and extra words to prefetch with a run.
    static RUN_CTX: RefCell<Option<RunState>> = const { RefCell::new(None) };
    static MW_CTX: RefCell<Option<MyWordsSave>> = const { RefCell::new(None) };
    static EXTRA_WORDS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// F21: the list the player picked.
    static LIST_ID: RefCell<Option<String>> = const { RefCell::new(None) };
    /// F20: the opponent chosen on the start screen, and the Duel in flight to `show`.
    static OPP: RefCell<Opponent> = const { RefCell::new(Opponent::Friend) };
    static DUEL_CTX: RefCell<Option<Duel>> = const { RefCell::new(None) };
    /// Bumped to cancel a pending computer turn.
    static NPC: Cell<u32> = const { Cell::new(0) };
}

/// F15: the first Par board shows its explainer once.
const PAR_HOW_KEY: &str = "spell_spz_par_how_v1";

/// F16: how long a press must be held to pencil a rune.
const LONG_PRESS_MS: i32 = 450;

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|u| u.borrow_mut().as_mut().map(f))
}

fn bump() -> u32 {
    TOKEN.with(|t| {
        t.set(t.get().wrapping_add(1));
        t.get()
    })
}

fn lexicon(lang: &str) -> Option<std::rc::Rc<Lexicon>> {
    LEX.with(|l| {
        let mut l = l.borrow_mut();
        if l.as_ref().map(|(k, _)| k.as_str()) != Some(lang) {
            *l = bank::load(lang, Vec::new()).ok().map(|x| (lang.to_string(), std::rc::Rc::new(x)));
        }
        l.as_ref().map(|(_, x)| x.clone())
    })
}

fn tr(key: &str, args: &[(&str, &str)]) -> String {
    i18n::tp(key, args)
}

fn today() -> String {
    crate::daily::today()
}

fn yesterday() -> String {
    let d = js_sys::Date::new_0();
    d.set_time(d.get_time() - 86_400_000.0);
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

/// A fresh seed on the device: the clock, a counter and the browser's randomness.
fn next_seed() -> u64 {
    let n = COUNTER.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    let r = (js_sys::Math::random() * 9_007_199_254_740_992.0) as u64;
    crate::spelldoku::rng::mix(js_sys::Date::now() as u64 ^ r.rotate_left(17), n)
}

// ------------------------------------------------------------------ wiring

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("szOpenBtn", move || open(&a));
    }
    dom::on_click("szExit", close);
    dom::on_click("szStart", begin_from_setup);
    dom::on_click("szNew", new_board);
    dom::on_click("szHowBtn", || show_how(true));
    dom::on_click("szHowOk", || show_how(false));
    dom::on::<web_sys::MouseEvent, _>("szTiers", "click", tier_tap);
    dom::on::<web_sys::MouseEvent, _>("szTypes", "click", type_tap);
    dom::on::<web_sys::MouseEvent, _>("szParBar", "click", board_tap);
    dom::on::<web_sys::MouseEvent, _>("szRunTabs", "click", run_tab_tap);
    dom::on_click("szNext", run_next);
    dom::on::<web_sys::MouseEvent, _>("szLists", "click", list_tap);
    dom::on::<web_sys::MouseEvent, _>("szOpp", "click", opp_tap);
    dom::on::<web_sys::MouseEvent, _>("szBoard", "click", board_tap);
    dom::on::<web_sys::MouseEvent, _>("szKb", "click", key_tap);
    // Phase E (each is a no-op while its flag is off).
    dom::on::<web_sys::Event, _>("szBoard", "pointerdown", lp_down);
    dom::on::<web_sys::Event, _>("szRunes", "pointerdown", lp_down);
    for id in ["szBoard", "szRunes"] {
        for kind in ["pointermove", "pointerleave"] {
            dom::on::<web_sys::Event, _>(id, kind, |_| lp_cancel());
        }
        for kind in ["pointerup", "pointercancel"] {
            dom::on::<web_sys::Event, _>(id, kind, |_| lp_release());
        }
        dom::on::<web_sys::Event, _>(id, "contextmenu", |e| {
            if pencil_on() {
                e.prevent_default();
            }
        });
    }
    dom::on::<web_sys::MouseEvent, _>("szRunes", "click", strip_tap);
    dom::on_click("szPencilClear", pencil_clear);
    // F18: any tap finishes a running ripple at once; input is never blocked.
    dom::on::<web_sys::Event, _>("szScreen", "click", |_| ripple_finish());
}

pub fn open(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (if s.lang == crate::consts::MINE { crate::consts::EN.to_string() } else { s.lang.clone() }, s.kid)
    };
    bump();
    UI.with(|u| *u.borrow_mut() = None);
    let lex = lexicon(&lang);
    let tiers: Vec<Tier> = offer::tiers_for(&lang, kid).into_iter().filter(|t| lex.as_ref().is_some_and(|l| seeds::usable(l, *t))).collect();
    // I11: nothing for a language that is not offered. The registry already hides the row.
    let Some(&tier) = tiers.first() else { return };
    CTX.with(|c| *c.borrow_mut() = (lang.clone(), kid, tier));
    let prefs: Prefs = storage::get_json(&store::prefs_key(store::profile(kid))).unwrap_or_default();
    BTYPE.with(|b| b.set(BoardType::from_name(&prefs.board_type).unwrap_or(BoardType::Classic)));
    for id in ["szBoard", "szKb"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            dom::set_lang_dir(&e, &lang);
        }
    }
    dom::set_hidden("szStartPane", false);
    dom::set_hidden("szPlay", true);
    dom::set_hidden("szNew", true);
    dom::set_hidden("szHow", true);
    dom::set_text("szNote", "");
    dom::remove_class("szScreen", "playing");
    render_setup();
    dom::add_class("szScreen", "show");
}

fn close() {
    bump();
    ripple_finish();
    speech_out::stop();
    save_progress();
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("szScreen", "show");
}

// ------------------------------------------------------------------ start

fn streak_now(kid: bool) -> u32 {
    let s: Streak = storage::get_json(&store::streak_key(store::profile(kid))).unwrap_or_default();
    s.current(&today(), &yesterday())
}

/// F22: the board types this start screen can offer right now. Classic always; Par when its flag is
/// on and verified Par boards exist for a tier the profile may open (I11: absent, never locked).
fn offered_types(lex: &Lexicon, lang: &str, kid: bool) -> Vec<BoardType> {
    let mut v = vec![BoardType::Classic];
    if flags::spelluzzle_par() && [Tier::Hard, Tier::Expert].iter().any(|t| offer::par_offered(lang, kid, *t) && !seeds::par_records(lex, *t).is_empty()) {
        v.push(BoardType::Par);
    }
    if flags::spelluzzle_expedition() && [Tier::Medium, Tier::Hard, Tier::Expert].iter().any(|t| offer::expedition_offered(lang, kid, *t) && !seeds::run_records(lex, *t).is_empty()) {
        v.push(BoardType::Expedition);
    }
    if flags::spelluzzle_mywords() && !my_lists(lang, kid).is_empty() {
        v.push(BoardType::MyWords);
    }
    if flags::spelluzzle_duel() && offer::duel_offered(lang, kid, Tier::Easy) {
        v.push(BoardType::Duel);
    }
    v
}

/// F21: the player's lists that can make a board: each with the words that match the bank, pass the
/// kid filter on a Jr profile, and are playable (3 to 9 cells, servable audio). Needs 3 or more.
fn my_lists(lang: &str, kid: bool) -> Vec<(String, String, Vec<String>)> {
    let lists = crate::word_lists::load();
    let mut out = Vec::new();
    for l in crate::word_lists::visible(&lists) {
        let mut words: Vec<String> = Vec::new();
        for e in &l.entries {
            if e.lang != lang {
                continue;
            }
            let Some(form) = crate::word_index::canonical(lang, &e.text) else { continue };
            let w = form.to_lowercase();
            let n = w.chars().count();
            if !(3..=9).contains(&n) || !w.chars().all(|c| c.is_alphabetic()) || words.contains(&w) {
                continue;
            }
            if crate::profanity::is_blocked(&w) || !crate::audio_verdict::servable(lang, &w, "normal") {
                continue;
            }
            if kid && !crate::kid_filter::kid_allowed(lang, &w) {
                continue;
            }
            words.push(w);
        }
        if words.len() >= mywords::MIN_LIST_WORDS {
            out.push((l.id.clone(), l.name.clone(), words));
        }
    }
    out
}

fn tiers_for_type(lex: &Lexicon, lang: &str, kid: bool, ty: BoardType) -> Vec<Tier> {
    match ty {
        BoardType::Classic => offer::tiers_for(lang, kid).into_iter().filter(|t| seeds::usable(lex, *t)).collect(),
        BoardType::Par => [Tier::Hard, Tier::Expert].into_iter().filter(|t| offer::par_offered(lang, kid, *t) && !seeds::par_records(lex, *t).is_empty()).collect(),
        BoardType::Expedition => [Tier::Medium, Tier::Hard, Tier::Expert].into_iter().filter(|t| offer::expedition_offered(lang, kid, *t) && !seeds::run_records(lex, *t).is_empty()).collect(),
        // F21: a My Words board has no tier of its own.
        BoardType::MyWords => Vec::new(),
        BoardType::Duel => [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert].into_iter().filter(|t| offer::duel_offered(lang, kid, *t)).collect(),
    }
}

fn render_setup() {
    let (lang, kid, mut tier) = CTX.with(|c| c.borrow().clone());
    dom::set_text("szTagline", &tr("spz.tagline", &[]));
    let Some(lex) = lexicon(&lang) else { return };
    // F22: the board-type row is absent when only Classic is offered.
    let types = offered_types(&lex, &lang, kid);
    let mut ty = BTYPE.with(Cell::get);
    if !types.contains(&ty) {
        ty = BoardType::Classic;
        BTYPE.with(|b| b.set(ty));
    }
    let th: String = if types.len() < 2 {
        String::new()
    } else {
        types
            .iter()
            .map(|b| {
                let key = match b {
                    BoardType::Classic => "spz.type.classic",
                    BoardType::Par => "spz.type.par",
                    BoardType::Expedition => "spz.type.expedition",
                    BoardType::MyWords => "top.myWords",
                    BoardType::Duel => "spz.type.duel",
                };
                format!("<button type=\"button\" class=\"sz-btn{}\" data-type=\"{}\">{}</button>", if *b == ty { " on" } else { "" }, b.name(), dom::escape_html(&tr(key, &[])))
            })
            .collect()
    };
    dom::set_html("szTypes", &th);
    // I11: a tier whose verified seeds do not match this bank is absent, never shown locked.
    let tiers = tiers_for_type(&lex, &lang, kid, ty);
    if !tiers.contains(&tier) {
        if let Some(&t) = tiers.first() {
            tier = t;
            CTX.with(|c| c.borrow_mut().2 = t);
        }
    }
    // Spell Jr has one fixed shape and no picker; a standard player with one tier sees none either.
    let h: String = if kid || tiers.len() < 2 {
        String::new()
    } else {
        tiers
            .iter()
            .map(|t| format!("<button type=\"button\" class=\"sz-btn{}\" data-tier=\"{}\">{}</button>", if *t == tier { " on" } else { "" }, t.name(), dom::escape_html(&tr(&format!("level.{}", t.name()), &[]))))
            .collect()
    };
    dom::set_html("szTiers", &h);
    // F21: the list picker, shown only for My Words.
    let lh: String = if ty == BoardType::MyWords {
        let chosen = LIST_ID.with(|l| l.borrow().clone());
        let lists = my_lists(&lang, kid);
        let first = chosen.filter(|c| lists.iter().any(|l| &l.0 == c)).or_else(|| lists.first().map(|l| l.0.clone()));
        LIST_ID.with(|l| *l.borrow_mut() = first.clone());
        let mut h = format!("<span class=\"sz-pick\">{}</span>", dom::escape_html(&tr("spz.mywords.pick", &[])));
        for (id, name, words) in &lists {
            h.push_str(&format!(
                "<button type=\"button\" class=\"sz-btn{}\" data-list=\"{}\">{} ({})</button>",
                if first.as_deref() == Some(id.as_str()) { " on" } else { "" },
                dom::escape_html(id),
                dom::escape_html(name),
                words.len()
            ));
        }
        h
    } else {
        String::new()
    };
    dom::set_html("szLists", &lh);
    // F20: who you play, and how hard the computer is.
    let oh: String = if ty == BoardType::Duel {
        let opp = OPP.with(|o| o.borrow().clone());
        let on = |b: bool| if b { " on" } else { "" };
        let mut h = format!(
            "<button type=\"button\" class=\"sz-btn{}\" data-opp=\"friend\">{}</button><button type=\"button\" class=\"sz-btn{}\" data-opp=\"cpu\">{}</button>",
            on(opp == Opponent::Friend),
            dom::escape_html(&tr("spz.duel.friend", &[])),
            on(opp != Opponent::Friend),
            dom::escape_html(&tr("spz.duel.cpu", &[]))
        );
        if let Opponent::Computer(dn) = opp {
            for (i, k) in ["bg.diff.easy", "bg.diff.normal", "bg.diff.tough"].iter().enumerate() {
                h.push_str(&format!("<button type=\"button\" class=\"sz-btn{}\" data-diff=\"{i}\">{}</button>", on(dn as usize == i), dom::escape_html(&tr(k, &[]))));
            }
        }
        h
    } else {
        String::new()
    };
    dom::set_html("szOpp", &oh);
    let n = streak_now(kid);
    dom::set_html("szStreak", &if n > 0 { crate::icons::label("flame", &tr("daily.streakDays", &[("n", &n.to_string())])) } else { String::new() });
}

fn type_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-type]").ok().flatten()) else { return };
    let Some(b) = el.get_attribute("data-type").and_then(|n| BoardType::from_name(&n)) else { return };
    BTYPE.with(|c| c.set(b));
    let (_, kid, _) = CTX.with(|c| c.borrow().clone());
    storage::set_json(&store::prefs_key(store::profile(kid)), &Prefs { board_type: b.name().to_string() });
    render_setup();
}

fn opp_tap(e: web_sys::MouseEvent) {
    let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    if let Some(el) = t.closest("[data-opp]").ok().flatten() {
        let o = if el.get_attribute("data-opp").as_deref() == Some("cpu") { Opponent::Computer(1) } else { Opponent::Friend };
        OPP.with(|c| *c.borrow_mut() = o);
    } else if let Some(el) = t.closest("[data-diff]").ok().flatten() {
        if let Some(i) = el.get_attribute("data-diff").and_then(|d| d.parse::<u8>().ok()) {
            OPP.with(|c| *c.borrow_mut() = Opponent::Computer(i.min(2)));
        }
    }
    render_setup();
}

fn list_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-list]").ok().flatten()) else { return };
    let Some(id) = el.get_attribute("data-list") else { return };
    LIST_ID.with(|l| *l.borrow_mut() = Some(id));
    render_setup();
}

fn tier_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-tier]").ok().flatten()) else { return };
    let Some(t) = el.get_attribute("data-tier").and_then(|n| Tier::from_name(&n)) else { return };
    CTX.with(|c| c.borrow_mut().2 = t);
    render_setup();
}

fn begin_from_setup() {
    let (_, _, tier) = CTX.with(|c| c.borrow().clone());
    begin(tier, true);
}

fn load_history(lang: &str, kid: bool) -> History {
    storage::get_json(&store::history_key(store::profile(kid), lang)).unwrap_or_default()
}

/// Resume the saved board for (profile, language, tier) if it still regenerates to
/// the same board, else draw a new one. Resume replaces; it never appends.
fn begin(tier: Tier, allow_resume: bool) {
    let (lang, kid, _) = CTX.with(|c| c.borrow().clone());
    let Some(lex) = lexicon(&lang) else { return };
    let profile = store::profile(kid);
    let token = bump();
    dom::set_text("szNote", "");
    dom::set_hidden("szStartPane", true);
    dom::set_hidden("szPlay", false);
    dom::set_hidden("szNew", false);
    dom::add_class("szScreen", "playing");
    dom::set_html("szBoard", "");
    dom::set_text("szMsg", "");
    // F11: show a loading state only if this takes more than 300 ms.
    dom::after_ms(300, move || {
        if TOKEN.with(Cell::get) == token && UI.with(|u| u.borrow().is_none()) {
            dom::set_text("szMsg", &tr("climb.loading", &[]));
        }
    });
    PAR_REC.with(|r| *r.borrow_mut() = None);
    if BTYPE.with(Cell::get) == BoardType::Par && par::offered_tier(tier) {
        begin_par(lex, lang, kid, tier, token, allow_resume);
        return;
    }
    RUN_CTX.with(|r| *r.borrow_mut() = None);
    MW_CTX.with(|r| *r.borrow_mut() = None);
    EXTRA_WORDS.with(|w| w.borrow_mut().clear());
    if BTYPE.with(Cell::get) == BoardType::Expedition && run::offered_tier(tier) {
        begin_run(lex, lang, kid, tier, token, allow_resume);
        return;
    }
    if BTYPE.with(Cell::get) == BoardType::MyWords {
        begin_mywords(lex, lang, kid, token, allow_resume);
        return;
    }
    DUEL_CTX.with(|r| *r.borrow_mut() = None);
    if BTYPE.with(Cell::get) == BoardType::Duel && duel::offered_tier(tier) {
        begin_duel(lex, lang, kid, tier, token, allow_resume);
        return;
    }
    let saved: Option<Progress> = if allow_resume { storage::get_json(&store::progress_key(profile, &lang, tier)) } else { None };
    if let Some(p) = saved {
        if let Ok(g) = generate_after(p.seed, tier, &lex, &p.previous) {
            if p.matches(g.board.hash()) {
                let play = Play::restore(g.board, &p.snapshot);
                ready(lex.clone(), lang.clone(), kid, tier, p.seed, p.previous.clone(), Some(play), token, 1);
                return;
            }
        }
        storage::remove(&store::progress_key(profile, &lang, tier));
    }
    draw(lex, lang, kid, tier, token, 1);
}

/// F15: resume the saved Par board if its record still rebuilds to the same board, else draw one.
fn begin_par(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, tier: Tier, token: u32, allow_resume: bool) {
    let profile = store::profile(kid);
    let key = store::par_progress_key(profile, &lang, tier);
    let saved: Option<Progress> = if allow_resume { storage::get_json(&key) } else { None };
    if let Some(p) = saved {
        if let (Some(par), true) = (p.par, p.par_words.len() == par::NW) {
            let mut w = [0u16; par::NW];
            w.copy_from_slice(&p.par_words);
            let rec = ParRecord { seed: p.seed, par, words: w };
            if let Some(board) = par::board_from_record(&lex, tier, &rec) {
                if p.matches(board.hash()) {
                    let play = Play::restore(board, &p.snapshot);
                    PAR_REC.with(|r| *r.borrow_mut() = Some(rec));
                    ready(lex, lang, kid, tier, p.seed, Vec::new(), Some(play), token, 1);
                    return;
                }
            }
        }
        storage::remove(&key);
    }
    let history = load_history(&lang, kid);
    let records = seeds::par_records(&lex, tier);
    let pick = |n: usize| (js_sys::Math::random() * n as f64) as usize;
    match fresh_par(&lex, tier, &records, &history, &history.last_words, pick) {
        Some((board, rec)) => {
            let seed = rec.seed;
            PAR_REC.with(|r| *r.borrow_mut() = Some(rec));
            ready(lex, lang, kid, tier, seed, Vec::new(), Some(Play::new(board)), token, 1);
        }
        None => unavailable(),
    }
}

/// F20: resume the saved match if it still rebuilds, else deal two boards for a new one.
fn begin_duel(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, tier: Tier, token: u32, allow_resume: bool) {
    let profile = store::profile(kid);
    let key = store::duel_key(profile, &lang);
    let saved: Option<duel::DuelSave> = if allow_resume { storage::get_json(&key) } else { None };
    if let Some(sv) = saved {
        if let Some(d) = Duel::restore(&lang, &sv) {
            if !d.match_over() {
                let words: Vec<String> = d.boards.iter().flat_map(|b| b.words()).collect();
                EXTRA_WORDS.with(|e| *e.borrow_mut() = words);
                let play = Play::new(d.boards[0].clone());
                let seed = d.boards[0].seed;
                DUEL_CTX.with(|r| *r.borrow_mut() = Some(d));
                ready(lex, lang, kid, d_tier(&sv.tier, tier), seed, Vec::new(), Some(play), token, 1);
                return;
            }
        }
        storage::remove(&key);
    }
    let opp = OPP.with(|o| o.borrow().clone());
    let s1 = next_seed();
    let Some(b1) = duel::generate_board(s1, tier, &lex, &[]) else { return unavailable() };
    let s2 = next_seed();
    let Some(b2) = duel::generate_board(s2, tier, &lex, &b1.words()) else { return unavailable() };
    let starter = (js_sys::Math::random() * 2.0) as u8;
    let d = Duel::new(vec![b1, b2], tier, opp, starter);
    let words: Vec<String> = d.boards.iter().flat_map(|b| b.words()).collect();
    EXTRA_WORDS.with(|e| *e.borrow_mut() = words);
    let play = Play::new(d.boards[0].clone());
    DUEL_CTX.with(|r| *r.borrow_mut() = Some(d));
    ready(lex, lang, kid, tier, s1, Vec::new(), Some(play), token, 1);
}

fn d_tier(saved: &str, fallback: Tier) -> Tier {
    Tier::from_name(saved).unwrap_or(fallback)
}

/// F19: resume the saved run if its record still rebuilds to the same boards, else draw one.
fn begin_run(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, tier: Tier, token: u32, allow_resume: bool) {
    let profile = store::profile(kid);
    let key = store::run_key(profile, &lang, tier);
    let st: RunStore = storage::get_json(&key).unwrap_or_default();
    if let (true, Some(save)) = (allow_resume, st.active.clone()) {
        if save.words.len() == run::RUN_WORDS && save.snaps.len() == 3 && save.stars.len() == 3 && save.done.len() == 3 {
            let mut w = [0u16; run::RUN_WORDS];
            w.copy_from_slice(&save.words);
            let rec = RunRecord { seed: save.seed, words: w };
            if let Some(boards) = run::boards_from_record(&lex, tier, &rec) {
                let cur = (save.cur as usize).min(2);
                let play = Play::restore(boards[cur].clone(), &save.snaps[cur]);
                let words: Vec<String> = boards.iter().flat_map(|b| b.words()).collect();
                EXTRA_WORDS.with(|e| *e.borrow_mut() = words);
                RUN_CTX.with(|r| *r.borrow_mut() = Some(RunState { rec: rec.clone(), boards, snaps: save.snaps.clone(), stars: save.stars.clone(), done: save.done.clone(), cur }));
                ready(lex, lang, kid, tier, rec.seed, Vec::new(), Some(play), token, 1);
                return;
            }
        }
        let mut cleared = st.clone();
        cleared.active = None;
        storage::set_json(&key, &cleared);
    }
    let history = load_history(&lang, kid);
    let records = seeds::run_records(&lex, tier);
    let pick = |n: usize| (js_sys::Math::random() * n as f64) as usize;
    match fresh_run(&lex, tier, &records, &history, &history.last_words, pick) {
        Some((boards, rec)) => {
            let play = Play::new(boards[0].clone());
            let words: Vec<String> = boards.iter().flat_map(|b| b.words()).collect();
            EXTRA_WORDS.with(|e| *e.borrow_mut() = words);
            let seed = rec.seed;
            RUN_CTX.with(|r| *r.borrow_mut() = Some(RunState { rec, boards, snaps: vec![Snapshot::default(); 3], stars: vec![0; 3], done: vec![false; 3], cur: 0 }));
            ready(lex, lang, kid, tier, seed, Vec::new(), Some(play), token, 1);
        }
        None => unavailable(),
    }
}

/// F21: resume the saved board for the chosen list, else build one from it.
fn begin_mywords(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, token: u32, allow_resume: bool) {
    let Some(list_id) = LIST_ID.with(|l| l.borrow().clone()) else { return };
    let profile = store::profile(kid);
    let key = store::mywords_key(profile, &lang, &list_id);
    let saved: Option<MyWordsSave> = if allow_resume { storage::get_json(&key) } else { None };
    if let Some(sv) = saved {
        if let Some(board) = mywords::board_from_words(&lex, sv.seed, &sv.words, &sv.from_list) {
            let play = Play::restore(board, &sv.snapshot);
            let seed = sv.seed;
            MW_CTX.with(|r| *r.borrow_mut() = Some(sv));
            ready(lex, lang, kid, Tier::Easy, seed, Vec::new(), Some(play), token, 1);
            return;
        }
        storage::remove(&key);
    }
    let Some(words) = my_lists(&lang, kid).into_iter().find(|l| l.0 == list_id).map(|l| l.2) else { return };
    let units: Vec<Vec<char>> = words.iter().map(|w| w.chars().collect()).collect();
    let previous = load_history(&lang, kid).last_words;
    // Prefer a board that shares at most two words with the last one (F11), among a few seeds.
    let mut best: Option<(usize, mywords::MyWordsBoard, u64)> = None;
    for _ in 0..6 {
        let seed = next_seed();
        if let Some(b) = mywords::generate(seed, &lex, &units, kid) {
            let shared = b.board.words().iter().filter(|w| previous.contains(w)).count();
            if shared <= 2 {
                best = Some((shared, b, seed));
                break;
            }
            if best.as_ref().map_or(true, |(s, _, _)| shared < *s) {
                best = Some((shared, b, seed));
            }
        }
    }
    match best {
        Some((_, b, seed)) => {
            let from_list = b.board.list.clone().unwrap_or_default();
            MW_CTX.with(|r| {
                *r.borrow_mut() = Some(MyWordsSave { seed, words: b.board.words(), from_list, snapshot: Snapshot::default() })
            });
            ready(lex, lang, kid, Tier::Easy, seed, Vec::new(), Some(Play::new(b.board)), token, 1);
        }
        None => {
            // "This list can't make a fair board yet": stay on the picker.
            dom::set_hidden("szStartPane", false);
            dom::set_hidden("szPlay", true);
            dom::remove_class("szScreen", "playing");
            dom::set_text("szNote", &tr("spz.mywords.unfair", &[]));
        }
    }
}

fn draw(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, tier: Tier, token: u32, attempt: u32) {
    let profile = store::profile(kid);
    let history = load_history(&lang, kid);
    let previous = history.last_words.clone();
    // Medium to Expert: only boards whose single answer was proved at build time against the
    // large word list (spelluzzle::seeds). A verified seed regenerates the same board from the
    // bank alone, so it is stored with no `previous` and resumes by plain regeneration.
    if seeds::needs_seeds(tier) {
        let list = seeds::verified(&lex, tier);
        let pick = |n: usize| (js_sys::Math::random() * n as f64) as usize;
        match fresh_verified(&lex, tier, &list, &history, &previous, pick) {
            Some((board, seed)) => ready(lex, lang, kid, tier, seed, Vec::new(), Some(Play::new(board)), token, attempt),
            None => unavailable(),
        }
        return;
    }
    match fresh_board(&lex, tier, &history, &previous, next_seed) {
        Ok((board, seed)) => {
            let mut play = Play::new(board);
            play.selected = None;
            ready(lex, lang, kid, tier, seed, previous, Some(play), token, attempt);
        }
        Err(_) => unavailable(),
    }
    let _ = profile;
}

/// I10: prefetch every word, and only then show the board. A clip that will not load
/// discards the board; after three boards the resolver's unavailable state shows.
#[allow(clippy::too_many_arguments)]
fn ready(lex: std::rc::Rc<Lexicon>, lang: String, kid: bool, tier: Tier, seed: u64, previous: Vec<String>, play: Option<Play>, token: u32, attempt: u32) {
    let Some(play) = play else { return };
    let mut words = play.board.words();
    // F19: a run fetches the audio for all sixteen words before its first board shows (I10).
    words.extend(EXTRA_WORDS.with(|e| e.borrow().clone()));
    words.sort();
    words.dedup();
    let l2 = lang.clone();
    crate::api::prefetch_set(words, l2, move |ok| {
        if TOKEN.with(Cell::get) != token {
            return; // the player left or asked for another board
        }
        if ok.iter().all(|x| *x) {
            show(play, lang, kid, tier, seed, previous);
        } else if attempt < AUDIO_TRIES {
            if par::offered_tier(tier) && BTYPE.with(Cell::get) == BoardType::Par {
                begin_par(lex, lang, kid, tier, token, false);
            } else if BTYPE.with(Cell::get) == BoardType::Expedition {
                begin_run(lex, lang, kid, tier, token, false);
            } else if BTYPE.with(Cell::get) == BoardType::MyWords {
                begin_mywords(lex, lang, kid, token, false);
            } else if BTYPE.with(Cell::get) == BoardType::Duel {
                begin_duel(lex, lang, kid, tier, token, false);
            } else {
                draw(lex, lang, kid, tier, token, attempt + 1);
            }
        } else {
            unavailable();
        }
    });
}

fn unavailable() {
    dom::set_html("szBoard", "");
    dom::set_html("szRunes", "");
    dom::set_hidden("szKb", true);
    dom::set_text("szMsg", &tr("tr.audioUnavailable", &[]));
}

fn show(play: Play, lang: String, kid: bool, tier: Tier, seed: u64, previous: Vec<String>) {
    let profile = store::profile(kid);
    // F11: every board shown goes into the history, so abandoned boards count.
    let mut h = load_history(&lang, kid);
    let run_ctx = RUN_CTX.with(|r| r.borrow_mut().take());
    let mw_ctx = MW_CTX.with(|r| r.borrow_mut().take());
    let duel_ctx = DUEL_CTX.with(|r| r.borrow_mut().take());
    if let Some(d) = &duel_ctx {
        // Both legs go into the history, so a match's boards are not drawn again soon (F11).
        for b in &d.boards {
            h.push(b.hash());
        }
        h.last_words = d.boards.iter().flat_map(|b| b.words()).collect();
    }
    match &run_ctx {
        // A run goes into the history as one entry, with all sixteen words as "the last board".
        Some(r) => {
            h.push(run::run_hash(&r.boards));
            h.last_words = r.boards.iter().flat_map(|b| b.words()).collect();
        }
        None => {
            h.push(play.board.hash());
            h.last_words = play.board.words();
        }
    }
    storage::set_json(&store::history_key(profile, &lang), &h);
    let finished = play.solved();
    let keys = input::layout(&lang, kid, &[]);
    let pencil = if pencil_on() {
        let saved: Option<PencilSave> = storage::get_json(&store::pencil_key(profile, &lang, tier));
        saved.filter(|p| p.hash == play.board.hash()).map(|p| Pencil::from_all(&p.marks)).unwrap_or_default()
    } else {
        Pencil::default()
    };
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui { play, lang, kid, tier, seed, previous, finished, keys, pencil, target: None, highlight: None, pending: BTreeSet::new(), par_rec: PAR_REC.with(|r| r.borrow_mut().take()), run: run_ctx, mw: mw_ctx, duel: duel_ctx.map(|d| DuelUi { d, selected: None, mode: None, draft: Vec::new(), note: String::new() }) })
    });
    render_keys();
    save_progress();
    render();
    if with_ui(|u| u.duel.is_some()).unwrap_or(false) {
        duel_after_action();
        return;
    }
    let has_silent = with_ui(|u| u.play.board.slots.iter().any(|s| s.kind == crate::spelluzzle::types::SlotKind::Silent)).unwrap_or(false);
    let par = with_ui(|u| u.play.par()).flatten();
    if storage::get_raw(HOW_KEY).as_deref() != Some("1") {
        show_how(true);
    } else if let (Some(p), true) = (par, storage::get_raw(PAR_HOW_KEY).as_deref() != Some("1")) {
        dom::set_html("szHowText", &format!("<p>{}</p>", dom::escape_html(&tr("spz.par.how", &[("p", &p.to_string())]))));
        dom::set_hidden("szHow", false);
        storage::set_raw(PAR_HOW_KEY, "1");
    } else if has_silent && storage::get_raw(HOW2_KEY).as_deref() != Some("1") {
        show_how_silent();
    }
}

fn show_how_silent() {
    dom::set_html("szHowText", &format!("<p>{}</p>", dom::escape_html(&tr("spz.how.silent", &[]))));
    dom::set_hidden("szHow", false);
    storage::set_raw(HOW2_KEY, "1");
}

fn new_board() {
    // Abandoning is free: no stars, no other cost (F7).
    let Some((lang, kid, tier)) = with_ui(|u| (u.lang.clone(), u.kid, u.tier)) else {
        let (_, _, tier) = CTX.with(|c| c.borrow().clone());
        begin(tier, false);
        return;
    };
    storage::remove(&store::progress_key(store::profile(kid), &lang, tier));
    storage::remove(&store::par_progress_key(store::profile(kid), &lang, tier));
    // F19: "New expedition" abandons the run (the best total is kept).
    let rkey = store::run_key(store::profile(kid), &lang, tier);
    if let Some(mut st) = storage::get_json::<RunStore>(&rkey) {
        st.active = None;
        storage::set_json(&rkey, &st);
    }
    storage::remove(&store::duel_key(store::profile(kid), &lang));
    NPC.with(|n| n.set(n.get().wrapping_add(1)));
    // F21: a new My Words board for the same list.
    if let Some(id) = LIST_ID.with(|l| l.borrow().clone()) {
        storage::remove(&store::mywords_key(store::profile(kid), &lang, &id));
    }
    speech_out::stop();
    UI.with(|u| *u.borrow_mut() = None);
    begin(tier, false);
}

fn show_how(on: bool) {
    if on {
        dom::set_html(
            "szHowText",
            &["spz.how.runes", "spz.how.tap", "spz.how.amber"]
                .iter()
                .copied()
                .chain(pencil_on().then_some("spz.pencil.how"))
                .map(|k| format!("<p>{}</p>", dom::escape_html(&tr(k, &[]))))
                .collect::<String>(),
        );
    } else {
        storage::set_raw(HOW_KEY, "1");
    }
    dom::set_hidden("szHow", !on);
}

// ------------------------------------------------------------------ persistence

fn save_progress() {
    let _ = with_ui(|u| {
        let profile = store::profile(u.kid);
        // F20: one match in progress per profile and language.
        if let Some(du) = &u.duel {
            let key = store::duel_key(profile, &u.lang);
            if du.d.match_over() {
                storage::remove(&key);
            } else {
                storage::set_json(&key, &du.d.save());
            }
            return;
        }
        // F19: an Expedition saves as one run, with a snapshot per board.
        if let Some(r) = u.run.as_mut() {
            r.snaps[r.cur] = u.play.snapshot();
            let key = store::run_key(profile, &u.lang, u.tier);
            let mut st: RunStore = storage::get_json(&key).unwrap_or_default();
            st.active = if u.finished {
                None
            } else {
                Some(RunSave { seed: r.rec.seed, words: r.rec.words.to_vec(), cur: r.cur as u8, snaps: r.snaps.clone(), stars: r.stars.clone(), done: r.done.clone() })
            };
            storage::set_json(&key, &st);
            return;
        }
        // F21: a My Words board saves whole.
        if let Some(m) = u.mw.as_mut() {
            m.snapshot = u.play.snapshot();
            if let Some(id) = LIST_ID.with(|l| l.borrow().clone()) {
                let key = store::mywords_key(profile, &u.lang, &id);
                if u.finished {
                    storage::remove(&key);
                } else {
                    storage::set_json(&key, m);
                }
            }
            return;
        }
        let key = match &u.par_rec {
            Some(_) => store::par_progress_key(profile, &u.lang, u.tier),
            None => store::progress_key(profile, &u.lang, u.tier),
        };
        if u.finished {
            storage::remove(&key);
            return;
        }
        let (par, par_words) = match &u.par_rec {
            Some(r) => (Some(r.par), r.words.to_vec()),
            None => (None, Vec::new()),
        };
        let p = Progress { seed: u.seed, previous: u.previous.clone(), gen_version: GEN_VERSION, hash: u.play.board.hash(), snapshot: u.play.snapshot(), par, par_words };
        storage::set_json(&key, &p);
    });
}

fn record_solved() {
    let Some((kid, lang, tier, stars)) = with_ui(|u| (u.kid, u.lang.clone(), u.tier, u.play.stars().map(|s| s.count()).unwrap_or(0))) else { return };
    let profile = store::profile(kid);
    let (is_run, is_mw) = with_ui(|u| (u.run.is_some(), u.mw.is_some())).unwrap_or((false, false));
    // D45: My Words stars are shown when solved and not kept. A run keeps its total, below.
    if is_run || is_mw {
        let sk = store::streak_key(profile);
        let mut s: Streak = storage::get_json(&sk).unwrap_or_default();
        s.solved(&today(), &yesterday());
        storage::set_json(&sk, &s);
        return;
    }
    let is_par = with_ui(|u| u.par_rec.is_some()).unwrap_or(false);
    let key = if is_par { store::par_stars_key(profile, &lang, tier) } else { store::stars_key(profile, &lang, tier) };
    let mut best: BestStars = storage::get_json(&key).unwrap_or_default();
    best.record(stars);
    storage::set_json(&key, &best);
    let sk = store::streak_key(profile);
    let mut s: Streak = storage::get_json(&sk).unwrap_or_default();
    s.solved(&today(), &yesterday());
    storage::set_json(&sk, &s);
}

// ------------------------------------------------------------------ audio

fn speak(word: &str, lang: &str) {
    let (fallback, code) = (word.to_string(), lang.to_string());
    crate::api::play_word(word, "normal", 1.0, lang, move || speech_out::speak(&fallback, 1.0, &code));
}

// ------------------------------------------------------------------ render

/// CC-SPELL-KEYBOARD (`keyboard.core`): the same letter rows and delete key
/// drawn through the shared keyboard. Units only, as before (this board types
/// no tones or kana marks).
fn render_keys_shared() {
    use crate::spellkb::{Attr, Cap, Edges, Surface};
    let rows: Vec<Vec<Cap>> = with_ui(|u| {
        u.keys.iter().map(|row| row.iter().filter(|k| matches!(k, Key::Unit(_))).map(|k| Cap::key(k.clone())).collect()).collect()
    })
    .unwrap_or_default();
    // The delete key lives at the end of the last letter row, so the keyboard stays three rows tall.
    let del = format!("<button type=\"button\" class=\"sz-key sz-del\" data-del=\"1\" aria-label=\"{}\">\u{232b}</button>", dom::escape_html(&tr("aria.backspace", &[])));
    let s = Surface { host: "szKbKeys", row_class: "sz-krow", key_class: "sz-key", attr: Attr::Prefixed("data-k"), long_press: false };
    crate::spellkb::mount(s, &rows, Edges { lead: "", tail: &del }, shared_key);
}

fn render_keys() {
    if crate::flags::keyboard_core() {
        render_keys_shared();
        return;
    }
    let html: String = with_ui(|u| {
        u.keys
            .iter()
            .map(|row| {
                let keys: String = row
                    .iter()
                    .filter(|k| matches!(k, Key::Unit(_)))
                    .map(|k| format!("<button type=\"button\" class=\"sz-key\" data-k=\"{}\">{}</button>", dom::escape_html(&k.data()), dom::escape_html(&k.face())))
                    .collect();
                format!("<div class=\"sz-krow\">{keys}</div>")
            })
            .collect::<Vec<String>>()
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                // The delete key lives at the end of the last letter row, so the keyboard stays three rows tall.
                if i + 1 == u.keys.len() {
                    let del = format!("<button type=\"button\" class=\"sz-key sz-del\" data-del=\"1\" aria-label=\"{}\">\u{232b}</button>", dom::escape_html(&tr("aria.backspace", &[])));
                    row.replacen("</div>", &format!("{del}</div>"), 1)
                } else {
                    row
                }
            })
            .collect()
    })
    .unwrap_or_default();
    dom::set_html("szKbKeys", &html);
}

fn render() {
    if with_ui(|u| u.duel.is_some()).unwrap_or(false) {
        duel_render();
        return;
    }
    dom::remove_class("szScreen", "duel");
    let _ = with_ui(|u| {
        let view = board_view(&u.play.board, &u.play.entries);
        // F18: runes the ripple has not reached yet still show their glyph.
        let mut shown = view.clone();
        for &r in &u.pending {
            if let Some(v) = shown.get_mut(r as usize) {
                *v = RuneState::Unknown;
            }
        }
        let opts = render::Opts {
            marks: if pencil_on() { u.pencil.visible(&view) } else { Default::default() },
            highlight: u.highlight,
            target: u.target,
        };
        // With every Phase E flag off this is the v1 call, so the markup is v1's (I13).
        let board = if u.pending.is_empty() && opts.marks.is_empty() && opts.highlight.is_none() && opts.target.is_none() {
            render::board_html(&u.play, &tr)
        } else {
            render::board_html_with(&u.play, &shown, &tr, &opts)
        };
        dom::set_html("szBoard", &board);
        // The one button that starts over says which kind of start it is.
        dom::set_text("szNew", &tr(if u.run.is_some() { "spz.run.new" } else { "spz.new" }, &[]));
        let runes = if strip_on() { render::strip_html(&u.play, &shown, &tr, &opts) } else { render::rune_key_html(&u.play, &tr) };
        dom::set_html("szRunes", &runes);
        let solved = u.play.solved();
        // The keyboard shows while a slot is open for typing, or a rune is waiting for its pencil
        // mark; the rune key or strip shows otherwise.
        let typing = (u.play.selected.is_some() || u.target.is_some()) && !solved;
        dom::set_hidden("szKb", !typing);
        dom::set_hidden("szRunes", typing);
        dom::toggle_class("szScreen", "typing", typing);
        dom::toggle_class("szScreen", "v11", pencil_on() || strip_on() || ripple_on());
        dom::set_hidden("szPencilClear", !(pencil_on() && !u.pencil.is_empty()));
        // F15: the Par header, the one Listen bar and the result line.
        let mut header = render::par_header(&u.play, &tr);
        // F19: "Board k of 3" (the third is "The Vault"), tabs to reopen solved boards, and the
        // way on once a board is solved.
        let (mut tabs, mut next) = (String::new(), false);
        if let Some(r) = &u.run {
            header = if r.cur == 2 { tr("spz.run.vault", &[]) } else { tr("spz.run.board", &[("k", &(r.cur + 1).to_string()), ("n", "3")]) };
            for i in 0..3usize {
                if i == r.cur || r.done[i] || (i > 0 && r.done[i - 1]) {
                    let label = if i == 2 { tr("spz.run.vault", &[]) } else { tr("spz.run.board", &[("k", &(i + 1).to_string()), ("n", "3")]) };
                    tabs.push_str(&format!(
                        "<button type=\"button\" class=\"sz-btn sz-tab{}\" data-board=\"{i}\" aria-label=\"{}\">{}</button>",
                        if i == r.cur { " on" } else { "" },
                        dom::escape_html(&label),
                        i + 1
                    ));
                }
            }
            next = r.done[r.cur] && r.cur < 2;
        }
        dom::set_html("szRunTabs", &tabs);
        dom::set_hidden("szNext", !next);
        dom::set_text("szParLine", &header);
        dom::set_hidden("szParLine", header.is_empty());
        dom::set_html("szParBar", &render::par_bar_html(&u.play, &tr));
        let mut msg = String::new();
        if solved {
            let line = render::par_result(&u.play, &tr);
            if !line.is_empty() {
                msg.push_str(&format!("<p class=\"sz-parres\">{}</p>", dom::escape_html(&line)));
            }
            msg.push_str(&render::stars_html(&u.play, &tr));
            if let Some(r) = &u.run {
                if u.finished {
                    msg.push_str(&format!("<p class=\"sz-parres\">\u{2605} {}/9</p>", r.stars.iter().map(|&s| s as u32).sum::<u32>()));
                }
            }
        }
        dom::set_html("szResult", &msg);
        dom::set_text("szMsg", &render::check_text(&u.play, &tr));
    });
}

// ------------------------------------------------------------------ Phase E: pencil, strip, ripple

fn save_pencil() {
    let _ = with_ui(|u| {
        if !pencil_on() {
            return;
        }
        let key = store::pencil_key(store::profile(u.kid), &u.lang, u.tier);
        if u.pencil.is_empty() {
            storage::remove(&key);
        } else {
            storage::set_json(&key, &PencilSave { hash: u.play.board.hash(), marks: u.pencil.all() });
        }
    });
}

/// The rune number on the element under a press, if it is an undecoded cell or strip entry.
fn pressed_rune(e: &web_sys::Event) -> Option<u8> {
    let el = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok())?;
    let el = el.closest("[data-rune]").ok().flatten()?;
    if el.get_attribute("data-state").as_deref() != Some("unknown") {
        return None;
    }
    el.get_attribute("data-rune").and_then(|r| r.parse::<u8>().ok())
}

fn lp_down(e: web_sys::Event) {
    // A new press: a click swallowed for an earlier long-press is no longer owed.
    LP_FIRED.with(|f| f.set(false));
    if !pencil_on() {
        return;
    }
    let Some(rune) = pressed_rune(&e) else { return };
    let token = LP.with(|c| {
        c.set(c.get().wrapping_add(1));
        c.get()
    });
    dom::after_ms(LONG_PRESS_MS, move || {
        if LP.with(Cell::get) != token {
            return;
        }
        LP_FIRED.with(|f| f.set(true));
        haptics::key_tap();
        with_ui(|u| {
            u.target = Some(rune);
            u.highlight = None;
        });
        render();
    });
}

fn lp_cancel() {
    LP.with(|c| c.set(c.get().wrapping_add(1)));
}

/// The release of a press. The click that follows a fired long-press is swallowed, but the
/// ripple of re-rendering can leave that click without a target, so the flag is cleared a beat
/// after the release rather than waiting for a click that may never arrive.
fn lp_release() {
    lp_cancel();
    if LP_FIRED.with(Cell::get) {
        dom::after_ms(80, || LP_FIRED.with(|f| f.set(false)));
    }
}

fn strip_tap(e: web_sys::MouseEvent) {
    if LP_FIRED.with(|f| f.replace(false)) {
        return; // the release of a long-press, not a tap
    }
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-rune]").ok().flatten()) else { return };
    let Some(r) = el.get_attribute("data-rune").and_then(|r| r.parse::<u8>().ok()) else { return };
    with_ui(|u| u.highlight = if u.highlight == Some(r) { None } else { Some(r) });
    render();
}

fn pencil_clear() {
    with_ui(|u| {
        u.pencil.clear_all();
        u.target = None;
    });
    save_pencil();
    render();
}

fn reduce_motion() -> bool {
    use js_sys::{Function, Reflect};
    let win = dom::window();
    let Some(f) = Reflect::get(&win, &"matchMedia".into()).ok().and_then(|f| f.dyn_into::<Function>().ok()) else { return false };
    let Ok(m) = f.call1(&win, &"(prefers-reduced-motion: reduce)".into()) else { return false };
    Reflect::get(&m, &"matches".into()).ok().and_then(|v| v.as_bool()).unwrap_or(false)
}

/// F18: finish a running ripple at once.
fn ripple_finish() {
    RIPPLE.with(|c| c.set(c.get().wrapping_add(1)));
    let had = with_ui(|u| {
        let had = !u.pending.is_empty();
        u.pending.clear();
        had
    })
    .unwrap_or(false);
    if had {
        render();
    }
}

/// F18: reveal the newly decoded runes, one at a time, in strip order, from two views.
fn start_ripple(before: Vec<RuneState>, committed: usize) {
    let Some((r, kid)) = with_ui(|u| {
        let slots: Vec<&[u8]> = u.play.board.slots.iter().map(|s| s.runes.as_slice()).collect();
        let order = pencil::strip_order(&slots, u.play.board.n_runes());
        let after = board_view(&u.play.board, &u.play.entries);
        (pencil::ripple(&slots, &order, &before, &after, committed), u.kid)
    }) else {
        return;
    };
    if r.runes.is_empty() {
        dom::set_text("szCount", "");
        return;
    }
    dom::set_text("szCount", &tr("spz.ripple.cells", &[("n", &r.cells.to_string())]));
    if reduce_motion() {
        // One short cross-fade, one tick, the final count.
        audio_boost::tick(0, true);
        dom::add_class("szBoard", "sz-fade");
        dom::after_ms(pencil::REDUCED_MS as i32, || dom::remove_class("szBoard", "sz-fade"));
        return;
    }
    let token = RIPPLE.with(|c| {
        c.set(c.get().wrapping_add(1));
        c.get()
    });
    with_ui(|u| u.pending = r.runes.iter().copied().collect());
    let step = pencil::step_ms(r.runes.len()) as i32;
    for (i, rune) in r.runes.iter().copied().enumerate() {
        dom::after_ms(step * (i as i32 + 1), move || {
            if RIPPLE.with(Cell::get) != token {
                return;
            }
            with_ui(|u| {
                u.pending.remove(&rune);
            });
            audio_boost::tick(i as u32, kid);
            render();
        });
    }
}

// ------------------------------------------------------------------ input

fn board_tap(e: web_sys::MouseEvent) {
    if with_ui(|u| u.duel.is_some()).unwrap_or(false) {
        duel_tap(e);
        return;
    }
    if LP_FIRED.with(|f| f.replace(false)) {
        return; // the release of a long-press, not a tap
    }
    with_ui(|u| {
        u.target = None;
        u.highlight = None;
    });
    let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    if let Some(el) = target.closest("[data-listen]").ok().flatten() {
        let Some(i) = el.get_attribute("data-listen").and_then(|d| d.parse::<usize>().ok()) else { return };
        let word = with_ui(|u| u.play.listen(i).map(|w| (w, u.lang.clone()))).flatten();
        if let Some((w, l)) = word {
            speak(&w, &l);
        }
        save_progress();
        render();
        return;
    }
    if let Some(el) = target.closest("[data-clear]").ok().flatten() {
        let Some(i) = el.get_attribute("data-clear").and_then(|d| d.parse::<usize>().ok()) else { return };
        with_ui(|u| u.play.clear_word(i));
        save_progress();
        render();
        return;
    }
    let Some(el) = target.closest("[data-slot]").ok().flatten() else { return };
    let Some(i) = el.get_attribute("data-slot").and_then(|d| d.parse::<usize>().ok()) else { return };
    let word = with_ui(|u| u.play.select(i).map(|w| (w, u.lang.clone()))).flatten();
    if let Some((w, l)) = word {
        speak(&w, &l);
    }
    save_progress();
    render();
}

fn key_tap(e: web_sys::MouseEvent) {
    if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest(".skb-key").ok().flatten()).is_some() {
        return; // typed already by the shared keyboard (keyboard.core)
    }
    if with_ui(|u| u.duel.is_some()).unwrap_or(false) {
        duel_key(e);
        return;
    }
    if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-del]").ok().flatten()).is_some() {
        delete();
        return;
    }
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-k]").ok().flatten()) else { return };
    let Some(Key::Unit(u)) = el.get_attribute("data-k").and_then(|d| Key::parse(&d)) else { return };
    let Some(c) = u.chars().next() else { return };
    unit_key(c);
}

/// CC-SPELL-KEYBOARD (`keyboard.core`): a press from the shared keyboard. Delete
/// is not a key there; its click still reaches `key_tap` through `#szKb`.
fn shared_key(k: &Key) {
    let Key::Unit(u) = k else { return };
    let Some(c) = u.chars().next() else { return };
    if with_ui(|u| u.duel.is_some()).unwrap_or(false) {
        haptics::key_tap();
        duel_unit(c);
    } else {
        unit_key(c);
    }
}

/// One letter, outside a Duel, from either keyboard path.
fn unit_key(c: char) {
    haptics::key_tap();
    // F16: with a rune held, the next key is its pencil mark and nothing else.
    let marked = with_ui(|ui| match ui.target.take() {
        Some(r) => {
            ui.pencil.set(r, c);
            true
        }
        None => false,
    })
    .unwrap_or(false);
    if marked {
        save_pencil();
        render();
        return;
    }
    ripple_finish();
    let before = with_ui(|ui| board_view(&ui.play.board, &ui.play.entries));
    let slot = with_ui(|ui| ui.play.selected).flatten();
    let out = with_ui(|ui| ui.play.type_unit(c)).flatten();
    if let Some(o) = out {
        on_commit(o);
        if ripple_on() {
            if let (Some(b), Some(s)) = (before, slot) {
                start_ripple(b, s);
            }
        }
    }
    save_progress();
    render();
}

fn delete() {
    haptics::key_tap();
    // F16: backspace with a rune held clears its mark.
    let cleared = with_ui(|u| match u.target.take() {
        Some(r) => {
            u.pencil.clear(r);
            true
        }
        None => false,
    })
    .unwrap_or(false);
    if cleared {
        save_pencil();
        render();
        return;
    }
    with_ui(|u| u.play.backspace());
    save_progress();
    render();
}

fn on_commit(o: Outcome) {
    // F4: neutral for a clean commit, close for a new clash, success for a solved board; never a miss.
    if feedback::State::from(o) == feedback::State::Success {
        haptics::correct();
        crate::themes::note_correct();
        let finale = with_ui(|u| {
            if u.finished {
                return None;
            }
            let stars = u.play.stars().map(|s| s.count()).unwrap_or(0);
            match u.run.as_mut() {
                // F19: a solved board of a run unlocks the next; only the Vault ends the run.
                Some(r) => {
                    if r.done[r.cur] {
                        return None;
                    }
                    r.done[r.cur] = true;
                    r.stars[r.cur] = stars;
                    if r.cur == 2 {
                        u.finished = true;
                    }
                }
                None => u.finished = true,
            }
            u.play.finale_word().map(|w| (w, u.lang.clone()))
        })
        .flatten();
        // A whole run counts once, when the Vault falls; any other board when it is solved.
        let run_over = with_ui(|u| u.run.as_ref().map(|r| r.cur == 2)).flatten();
        match run_over {
            None => record_solved(),
            Some(true) => {
                record_run_total();
                record_solved();
            }
            Some(false) => {}
        }
        // F6: the orb says the secret word for the first time, once.
        if let Some((w, l)) = finale {
            speak(&w, &l);
        }
    }
}

/// F19 / D39: the run's total, out of nine, and the best one kept per language and tier.
fn record_run_total() {
    let Some((kid, lang, tier, total)) = with_ui(|u| (u.kid, u.lang.clone(), u.tier, u.run.as_ref().map(|r| r.stars.iter().map(|&s| s as u32).sum::<u32>()).unwrap_or(0))) else { return };
    let key = store::run_key(store::profile(kid), &lang, tier);
    let mut st: RunStore = storage::get_json(&key).unwrap_or_default();
    st.best = st.best.max(total.min(9) as u8);
    storage::set_json(&key, &st);
}

// ------------------------------------------------------------------ Expeditions

/// Put board `k` of the run on screen: the one you leave keeps its entries.
fn run_switch(k: usize) {
    let ok = with_ui(|u| {
        let Some(r) = u.run.as_mut() else { return false };
        if k > 2 || k == r.cur || !(r.done[k] || k == r.cur || (k > 0 && r.done[k - 1])) {
            return false;
        }
        r.snaps[r.cur] = u.play.snapshot();
        r.cur = k;
        u.play = Play::restore(r.boards[k].clone(), &r.snaps[k]);
        u.pencil = Pencil::default();
        u.target = None;
        u.highlight = None;
        u.pending.clear();
        true
    })
    .unwrap_or(false);
    if ok {
        speech_out::stop();
        save_progress();
        render();
    }
}

fn run_tab_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-board]").ok().flatten()) else { return };
    if let Some(k) = el.get_attribute("data-board").and_then(|d| d.parse::<usize>().ok()) {
        run_switch(k);
    }
}

fn run_next() {
    let next = with_ui(|u| u.run.as_ref().map(|r| r.cur + 1)).flatten();
    if let Some(k) = next {
        run_switch(k);
    }
}

// ------------------------------------------------------------------ Rune Duel

fn duel_name(du: &DuelUi, p: u8) -> String {
    if du.d.is_computer(p) {
        tr("spz.duel.cpuName", &[])
    } else {
        tr(if p == 0 { "spz.duel.p1" } else { "spz.duel.p2" }, &[])
    }
}

fn duel_render() {
    let _ = with_ui(|u| {
        let Some(du) = u.duel.as_ref() else { return };
        let d = &du.d;
        dom::set_html("szBoard", &render::duel_board_html(d, du.selected, &du.draft, &tr));
        dom::add_class("szScreen", "duel");
        let legs = tr("spz.run.board", &[("k", &(d.leg + 1).to_string()), ("n", "2")]);
        let head = if d.match_over() {
            match d.winner() {
                Some(w) => tr("spz.duel.wins", &[("name", &duel_name(du, w))]),
                None => tr("spz.duel.draw", &[]),
            }
        } else {
            format!("{legs} \u{b7} {}", tr("spz.duel.turn", &[("name", &duel_name(du, d.turn))]))
        };
        dom::set_text("szParLine", &head);
        dom::set_hidden("szParLine", false);
        let t = d.totals();
        dom::set_text("szDuelHdr", &format!("{}: {} \u{b7} {}: {}", duel_name(du, 0), t[0], duel_name(du, 1), t[1]));
        dom::set_hidden("szDuelHdr", false);
        dom::set_html("szRunTabs", "");
        dom::set_html("szRunes", "");
        dom::set_hidden("szRunes", true);
        dom::set_hidden("szNext", true);
        dom::set_html("szResult", "");
        dom::set_text("szMsg", &du.note);
        dom::set_text("szNew", &tr("spz.duel.new", &[]));
        // The action bar: what the human may do now.
        let mine = !d.match_over() && !d.is_computer(d.turn);
        let mut bar = String::new();
        if mine {
            if let Some(i) = du.selected {
                use crate::spelluzzle::duel::SlotState;
                let st = d.slot_state(i);
                let secret = i + 1 == duel::WORDS;
                if du.mode.is_some() {
                    bar.push_str(&format!("<button type=\"button\" class=\"sz-btn on\" data-act=\"check\">{}</button>", dom::escape_html(&tr("btn.check", &[]))));
                } else if matches!(st, SlotState::Unheard | SlotState::Heard) {
                    if !secret {
                        bar.push_str(&format!("<button type=\"button\" class=\"sz-btn\" data-act=\"listen\">{}</button>", dom::escape_html(&tr("spz.duel.listen", &[]))));
                    }
                    if st == SlotState::Unheard {
                        let m = if secret { duel::SECRET_PTS } else { duel::UNHEARD_PTS };
                        bar.push_str(&format!("<button type=\"button\" class=\"sz-btn\" data-act=\"unheard\">{}</button>", dom::escape_html(&tr("spz.duel.unheard", &[("m", &m.to_string())]))));
                    }
                }
            }
            bar.push_str(&format!("<button type=\"button\" class=\"sz-btn\" data-act=\"pass\">{}</button>", dom::escape_html(&tr("spz.duel.pass", &[]))));
        }
        dom::set_html("szParBar", &bar);
        let typing = mine && du.mode.is_some();
        dom::set_hidden("szKb", !typing);
        dom::toggle_class("szScreen", "typing", typing);
    });
}

fn duel_tap(e: web_sys::MouseEvent) {
    let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    // The action bar sits outside the board but shares this handler.
    if let Some(el) = t.closest("[data-act]").ok().flatten() {
        let Some(act) = el.get_attribute("data-act") else { return };
        duel_act(&act);
        return;
    }
    let Some(el) = t.closest("[data-slot]").ok().flatten() else { return };
    let Some(i) = el.get_attribute("data-slot").and_then(|d| d.parse::<usize>().ok()) else { return };
    with_ui(|u| {
        if let Some(du) = u.duel.as_mut() {
            let open = !du.d.match_over() && !du.d.is_computer(du.d.turn) && !matches!(du.d.slot_state(i), crate::spelluzzle::duel::SlotState::Solved | crate::spelluzzle::duel::SlotState::Locked);
            // A word half-typed is dropped when another is chosen; the choice of action is made again.
            if open && du.selected != Some(i) {
                du.selected = Some(i);
                du.mode = None;
                du.draft.clear();
                du.note.clear();
            } else if open {
                du.selected = Some(i);
            }
        }
    });
    duel_render();
}

fn duel_act(act: &str) {
    match act {
        "listen" => {
            let r = with_ui(|u| {
                let du = u.duel.as_mut()?;
                let i = du.selected?;
                let w = du.d.listen(i).ok()?;
                du.mode = Some(How::Heard);
                du.draft.clear();
                du.note.clear();
                Some((w, u.lang.clone()))
            })
            .flatten();
            if let Some((w, l)) = r {
                speak(&w, &l);
            }
            duel_render();
        }
        "unheard" => {
            with_ui(|u| {
                if let Some(du) = u.duel.as_mut() {
                    du.mode = Some(How::Unheard);
                    du.draft.clear();
                    du.note.clear();
                }
            });
            duel_render();
        }
        "check" => duel_check(),
        "pass" => {
            with_ui(|u| {
                if let Some(du) = u.duel.as_mut() {
                    du.d.pass();
                    du.selected = None;
                    du.mode = None;
                    du.draft.clear();
                    du.note.clear();
                }
            });
            duel_after_action();
        }
        _ => {}
    }
}

fn duel_key(e: web_sys::MouseEvent) {
    let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    haptics::key_tap();
    if t.closest("[data-del]").ok().flatten().is_some() {
        with_ui(|u| {
            if let Some(du) = u.duel.as_mut() {
                du.draft.pop();
            }
        });
        duel_render();
        return;
    }
    let Some(el) = t.closest("[data-k]").ok().flatten() else { return };
    let Some(Key::Unit(u)) = el.get_attribute("data-k").and_then(|d| Key::parse(&d)) else { return };
    let Some(c) = u.chars().next() else { return };
    duel_unit(c);
}

/// A letter typed into the Duel draft, from either keyboard path.
fn duel_unit(c: char) {
    with_ui(|ui| {
        if let Some(du) = ui.duel.as_mut() {
            if let (Some(i), Some(_)) = (du.selected, du.mode) {
                // Filling the last cell does not submit: a Duel entry cannot be taken back, so it
                // waits for Check.
                if du.draft.len() < du.d.board().slots[i].answer.len() {
                    du.draft.push(c);
                }
            }
        }
    });
    duel_render();
}

fn duel_check() {
    let r = with_ui(|u| {
        let du = u.duel.as_mut()?;
        let (i, how) = (du.selected?, du.mode?);
        if du.draft.len() != du.d.board().slots[i].answer.len() {
            return None; // not a whole word yet
        }
        let typed = du.draft.clone();
        let out = du.d.submit(i, &typed, how).ok()?;
        du.selected = None;
        du.mode = None;
        du.draft.clear();
        // D23: the verdict, every turn.
        du.note = if out.correct { format!("\u{2713} +{}", out.points) } else { "\u{2717}".to_string() };
        Some(out.correct)
    })
    .flatten();
    match r {
        Some(true) => { haptics::correct(); crate::themes::note_correct(); }
        Some(false) => haptics::wrong(),
        None => return,
    }
    duel_after_action();
}

/// After anything that ends a turn: save, redraw, and hand the table to the computer if it is up.
fn duel_after_action() {
    save_progress();
    duel_render();
    let Some((over, cpu)) = with_ui(|u| u.duel.as_ref().map(|du| (du.d.match_over(), du.d.is_computer(du.d.turn)))).flatten() else { return };
    if over {
        record_duel_end();
        return;
    }
    if cpu {
        let token = NPC.with(|n| {
            n.set(n.get().wrapping_add(1));
            n.get()
        });
        // Its turn is one short beat: it lists nothing and never takes more than two seconds.
        dom::after_ms(1400, move || duel_npc_step(token));
    }
}

fn duel_npc_step(token: u32) {
    if NPC.with(Cell::get) != token {
        return;
    }
    let Some(lex) = with_ui(|u| lexicon(&u.lang)).flatten() else { return };
    let mut rng = crate::spelldoku::rng::Rng::new(next_seed());
    let done = with_ui(|u| {
        let du = u.duel.as_mut()?;
        if du.d.match_over() || !du.d.is_computer(du.d.turn) {
            return None;
        }
        let r = du.d.npc_turn(&lex, &mut rng);
        du.selected = None;
        du.mode = None;
        du.draft.clear();
        let name = tr("spz.duel.cpuName", &[]);
        du.note = match &r {
            NpcDone::Passed => format!("{name}: {}", tr("spz.duel.pass", &[])),
            NpcDone::Spelled { spelled, .. } => format!("{name} {}", if spelled.correct { format!("\u{2713} +{}", spelled.points) } else { "\u{2717}".to_string() }),
        };
        Some((r, u.lang.clone()))
    })
    .flatten();
    if let Some((NpcDone::Spelled { heard_word: Some(w), .. }, l)) = &done {
        // When it listens, the player hears the word too.
        speak(w, l);
    }
    if done.is_some() {
        duel_after_action();
    }
}

/// A finished match counts toward nothing but itself (D46: a Duel is not a streak day); the saved
/// match is cleared by `save_progress`.
fn record_duel_end() {}

/// Test seam (observation only, like `currentWord`): the answers on the open board, as a
/// JSON array in slot order, or "[]" when none is open. The e2e suite reads it to know
/// what to type, and types through the real keys.
pub fn seam_words() -> String {
    // In a Duel the leg on the table; otherwise the board in play.
    with_ui(|u| {
        let words = match &u.duel {
            Some(du) => du.d.board().words(),
            None => u.play.board.words(),
        };
        serde_json::to_string(&words).unwrap_or_default()
    })
    .unwrap_or_else(|| "[]".to_string())
}
