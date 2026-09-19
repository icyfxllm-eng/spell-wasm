//! CC-IMPOSTOR F1/F5 — the round loop, the streak, and the end screen.
//!
//! Hear the word, tap the real spelling. [`crate::impostor`] builds the cards;
//! this file plays them.
//!
//! # Streaks, not lives (D4)
//!
//! A wrong tap resets the streak and the set carries on. There is no
//! elimination anywhere in this mode, so a bad round costs momentum and never
//! the session.
//!
//! # Reveal-why (D5), and what it must never do
//!
//! A miss highlights the real card and shows a one-line trap label. That label
//! is displayed content and rides the audit: English has its trap lines today,
//! and a language whose lines are not cleared shows the highlight with NO quip.
//! There is deliberately no fallback text path: auto-generated grammar advice
//! in a language nobody has checked would be worse than silence.
//!
//! (Worded that way on purpose. The translator's closed-space gate greps src/
//! for the obvious two-word phrase for auto-translation and does not care
//! whether it finds it in code or in a comment — which is correct for a guard
//! whose entire job is that no such path exists. This comment tripped it once.)
//!
//! # What is not built
//!
//! D7's tier gating beyond the free easy set, F4's how-to cards, and D6's
//! analytics write. The round loop, the streak and the end screen are here.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::impostor::{self, Round, Tier};
use crate::{dom, haptics, i18n, speech_out, storage, App};

/// D4: flair thresholds, as config constants rather than magic numbers.
const FLAIR: [u32; 3] = [5, 10, 20];
const SET_LEN: usize = 10;

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static SET: RefCell<Vec<Round>> = RefCell::new(Vec::new());
    static AT: Cell<usize> = const { Cell::new(0) };
    static STREAK: Cell<u32> = const { Cell::new(0) };
    static BEST: Cell<u32> = const { Cell::new(0) };
    static RIGHT: Cell<u32> = const { Cell::new(0) };
    /// True while a miss is on screen: the next tap advances rather than answers.
    static REVEALING: Cell<bool> = const { Cell::new(false) };
}

fn best_key(lang: &str) -> String {
    format!("spell_impostor_best_{lang}")
}

/// A word surface with no direction is a bug (CC-RTL F1), and every card is one.
fn reflect_direction(lang: &str) {
    let dir = crate::consts::dir_attr(lang);
    for id in ["impCards", "impWhy", "impOverBody"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", lang);
            let _ = e.set_attribute("dir", dir);
        }
    }
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("impostorOpen", move || open(&a));
    dom::on_click("impExit", close);
    let a = app.clone();
    dom::on_click("impReplay", move || open(&a));
    let a = app.clone();
    dom::on_click("impOrb", move || speak(&a));
    // One delegated listener; the four cards are re-rendered every round.
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("impCards", "click", move |e| tap(&a, &e));
}

pub fn open(app: &App) {
    crate::telemetry::set_mode(crate::telemetry::schema::Mode::Impostor);
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    reflect_direction(&lang);
    dom::add_class("impostor", "show");
    dom::remove_class("impOver", "show");
    dom::set_text("impWhy", "");
    AT.with(|c| c.set(0));
    STREAK.with(|c| c.set(0));
    BEST.with(|c| c.set(0));
    RIGHT.with(|c| c.set(0));
    REVEALING.with(|c| c.set(false));

    // D7: the free tier is easy. I5 caps Kid Mode regardless of what is asked.
    let tier = impostor::effective_tier(Tier::Easy, kid);
    let seed = js_sys::Date::now() as u64;
    // CC-ONBOARD-JR I5: a Spell Jr set's real words are Easy + Medium, kid-safe.
    let set = impostor::set_of_ten_for(crate::experience::of_kid(kid), &lang, seed, tier);
    let empty = set.is_empty();
    SET.with(|s| *s.borrow_mut() = set);
    if empty {
        dom::set_html("impCards", "");
        dom::set_text("impWhy", &i18n::t("imp.unavailable"));
        return;
    }
    render_round(app);
}

fn close() {
    speech_out::stop();
    dom::remove_class("impostor", "show");
}

fn current() -> Option<Round> {
    SET.with(|s| s.borrow().get(AT.with(Cell::get)).cloned())
}

fn render_round(app: &App) {
    let Some(r) = current() else { return };
    let html: String = r
        .cards
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "<button type=\"button\" class=\"imp-card\" data-i=\"{i}\">{}</button>",
                dom::escape_html(c)
            )
        })
        .collect();
    dom::set_html("impCards", &html);
    dom::set_text("impWhy", "");
    render_streak();
    dom::set_text(
        "impProgress",
        &format!("{}/{}", AT.with(Cell::get) + 1, SET.with(|s| s.borrow().len())),
    );
    speak(app);
}

fn render_streak() {
    let n = STREAK.with(Cell::get);
    let flair = FLAIR.iter().rev().find(|t| n >= **t).map(|t| match t {
        20 => "\u{1F525}\u{1F525}\u{1F525}",
        10 => "\u{1F525}\u{1F525}",
        _ => "\u{1F525}",
    });
    dom::set_text("impStreak", &format!("{}{}", n, flair.unwrap_or("")));
}

/// F1: replay is free and unlimited — the word is the prompt, so hearing it
/// again must never cost anything.
fn speak(app: &App) {
    let Some(r) = current() else { return };
    let lang = LANG.with(|l| l.borrow().clone());
    let _ = app;
    speech_out::speak(&r.word, 0.9, &lang);
}

fn tap(app: &App, e: &web_sys::MouseEvent) {
    // A miss leaves the reveal on screen; the next tap anywhere advances it.
    if REVEALING.with(Cell::get) {
        REVEALING.with(|c| c.set(false));
        advance(app);
        return;
    }
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|t| t.closest("[data-i]").ok().flatten())
    else {
        return;
    };
    let Some(i) = el.get_attribute("data-i").and_then(|v| v.parse::<usize>().ok()) else {
        return;
    };
    let Some(r) = current() else { return };

    if i == r.correct {
        let n = STREAK.with(Cell::get) + 1;
        STREAK.with(|c| c.set(n));
        BEST.with(|c| c.set(c.get().max(n)));
        RIGHT.with(|c| c.set(c.get() + 1));
        let _ = el.class_list().add_1("right");
        haptics::correct();
        render_streak();
        advance(app);
    } else {
        // D4: the streak resets, the set continues. No elimination, anywhere.
        STREAK.with(|c| c.set(0));
        let _ = el.class_list().add_1("wrong");
        if let Some(right) = dom::doc().query_selector(&format!("[data-i=\"{}\"]", r.correct)).ok().flatten() {
            let _ = right.class_list().add_1("right");
        }
        haptics::key_tap();
        render_streak();
        reveal_why(&r, i);
        REVEALING.with(|c| c.set(true));
    }
}

/// D5/I4. The quip renders only where its strings cleared audit; elsewhere the
/// highlight stands alone. `i18n::t` would fall back to English for a missing
/// key, which is exactly the machine-fallback path I4 forbids — so the language
/// is checked before the lookup, not after.
fn reveal_why(r: &Round, picked: usize) {
    let lang = LANG.with(|l| l.borrow().clone());
    if lang != "en" {
        return;
    }
    if let Some(t) = r.trap_at(picked) {
        dom::set_text("impWhy", &i18n::t(t.label_key()));
    }
}

fn advance(app: &App) {
    let next = AT.with(Cell::get) + 1;
    if next >= SET.with(|s| s.borrow().len()) {
        finish();
        return;
    }
    AT.with(|c| c.set(next));
    render_round(app);
}

fn finish() {
    let lang = LANG.with(|l| l.borrow().clone());
    let best = BEST.with(Cell::get);
    let stored: u32 = storage::get_json(&best_key(&lang)).unwrap_or(0);
    if best > stored {
        storage::set_json(&best_key(&lang), &best);
    }
    let right = RIGHT.with(Cell::get);
    let total = SET.with(|s| s.borrow().len()).max(1) as u32;
    dom::set_text("impOverAcc", &format!("{}/{}", right, total));
    dom::set_text("impOverStreak", &best.to_string());
    dom::add_class("impOver", "show");
    speech_out::stop();
}

/// Exposed for the e2e harness and for symmetry with the other modes.
#[allow(dead_code)]
pub fn set_len() -> usize {
    SET_LEN
}
