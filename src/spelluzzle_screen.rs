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
    play::{Outcome, Play},
    render, seeds,
    store::{self, BestStars, History, Progress, Streak},
    types::{Tier, GEN_VERSION},
};
use crate::{dom, feedback, haptics, i18n, speech_out, storage, App};

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
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LEX: RefCell<Option<(String, std::rc::Rc<Lexicon>)>> = const { RefCell::new(None) };
    static CTX: RefCell<(String, bool, Tier)> = RefCell::new((String::new(), false, Tier::Easy));
    static TOKEN: Cell<u32> = const { Cell::new(0) };
    static COUNTER: Cell<u64> = const { Cell::new(0) };
}

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
    UI.with(|u| *u.borrow_mut() = Some(Ui { play, lang, kid, tier, seed, previous, finished, keys }));
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
            &["spz.how.runes", "spz.how.tap", "spz.how.amber"].iter().map(|k| format!("<p>{}</p>", dom::escape_html(&tr(k, &[])))).collect::<String>(),
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
        dom::set_html("szBoard", &render::board_html(&u.play, &tr));
        dom::set_html("szRunes", &render::rune_key_html(&u.play, &tr));
        let solved = u.play.solved();
        // The keyboard shows while a slot is open for typing; the rune key shows otherwise.
        let typing = u.play.selected.is_some() && !solved;
        dom::set_hidden("szKb", !typing);
        dom::set_hidden("szRunes", typing);
        dom::toggle_class("szScreen", "typing", typing);
        let msg = if solved { render::stars_html(&u.play, &tr) } else { String::new() };
        dom::set_html("szResult", &msg);
        dom::set_text("szMsg", &render::check_text(&u.play, &tr));
    });
}

// ------------------------------------------------------------------ input

fn board_tap(e: web_sys::MouseEvent) {
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
    let out = with_ui(|ui| ui.play.type_unit(c)).flatten();
    if let Some(o) = out {
        on_commit(o);
    }
    save_progress();
    render();
}

fn delete() {
    haptics::key_tap();
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
