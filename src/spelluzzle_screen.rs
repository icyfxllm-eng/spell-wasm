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
    fresh::{fresh_board, fresh_verified},
    gen::generate_after,
    lex::Lexicon,
    offer,
    pencil::{self, Pencil},
    play::{Outcome, Play},
    render, seeds,
    store::{self, BestStars, History, PencilSave, Progress, Streak},
    types::{Tier, GEN_VERSION},
    view::{board_view, RuneState},
};
use std::collections::BTreeSet;
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
}

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
    for id in ["szBoard", "szKb"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", &lang);
            let _ = e.set_attribute("dir", crate::consts::dir_attr(&lang));
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

fn render_setup() {
    let (lang, kid, tier) = CTX.with(|c| c.borrow().clone());
    dom::set_text("szTagline", &tr("spz.tagline", &[]));
    let lex = lexicon(&lang);
    // I11: a tier whose verified seeds do not match this bank is absent, never shown locked.
    let tiers: Vec<Tier> = offer::tiers_for(&lang, kid).into_iter().filter(|t| lex.as_ref().is_some_and(|l| seeds::usable(l, *t))).collect();
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
    let n = streak_now(kid);
    dom::set_text("szStreak", &if n > 0 { tr("daily.streakDays", &[("n", &n.to_string())]) } else { String::new() });
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
    let words = play.board.words();
    let l2 = lang.clone();
    crate::api::prefetch_set(words, l2, move |ok| {
        if TOKEN.with(Cell::get) != token {
            return; // the player left or asked for another board
        }
        if ok.iter().all(|x| *x) {
            show(play, lang, kid, tier, seed, previous);
        } else if attempt < AUDIO_TRIES {
            draw(lex, lang, kid, tier, token, attempt + 1);
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
    h.push(play.board.hash());
    h.last_words = play.board.words();
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
        *u.borrow_mut() = Some(Ui { play, lang, kid, tier, seed, previous, finished, keys, pencil, target: None, highlight: None, pending: BTreeSet::new() })
    });
    render_keys();
    save_progress();
    render();
    let has_silent = with_ui(|u| u.play.board.slots.iter().any(|s| s.kind == crate::spelluzzle::types::SlotKind::Silent)).unwrap_or(false);
    if storage::get_raw(HOW_KEY).as_deref() != Some("1") {
        show_how(true);
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
        let key = store::progress_key(store::profile(u.kid), &u.lang, u.tier);
        if u.finished {
            storage::remove(&key);
            return;
        }
        let p = Progress { seed: u.seed, previous: u.previous.clone(), gen_version: GEN_VERSION, hash: u.play.board.hash(), snapshot: u.play.snapshot() };
        storage::set_json(&key, &p);
    });
}

fn record_solved() {
    let Some((kid, lang, tier, stars)) = with_ui(|u| (u.kid, u.lang.clone(), u.tier, u.play.stars().map(|s| s.count()).unwrap_or(0))) else { return };
    let profile = store::profile(kid);
    let key = store::stars_key(profile, &lang, tier);
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

fn render_keys() {
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
        let msg = if solved { render::stars_html(&u.play, &tr) } else { String::new() };
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
    if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-del]").ok().flatten()).is_some() {
        delete();
        return;
    }
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-k]").ok().flatten()) else { return };
    let Some(Key::Unit(u)) = el.get_attribute("data-k").and_then(|d| Key::parse(&d)) else { return };
    let Some(c) = u.chars().next() else { return };
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
        let finale = with_ui(|u| {
            if u.finished {
                None
            } else {
                u.finished = true;
                u.play.finale_word().map(|w| (w, u.lang.clone()))
            }
        })
        .flatten();
        record_solved();
        // F6: the orb says the secret word for the first time, once.
        if let Some((w, l)) = finale {
            speak(&w, &l);
        }
    }
}

/// Test seam (observation only, like `currentWord`): the answers on the open board, as a
/// JSON array in slot order, or "[]" when none is open. The e2e suite reads it to know
/// what to type, and types through the real keys.
pub fn seam_words() -> String {
    with_ui(|u| serde_json::to_string(&u.play.board.words()).unwrap_or_default()).unwrap_or_else(|| "[]".to_string())
}
