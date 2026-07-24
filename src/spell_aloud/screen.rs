//! The Spell Aloud MODE surface (CC-SPELL-ALOUD Phase 1 — spec F1, gate G-C).
//!
//! A *place* with kid-simple turn-taking: **push-and-hold** the mic and say letters,
//! each accepted letter fills one slot (with an optional spoken echo). It reuses the
//! Phase-0 parser (`parse`/`interpret`/`accept_into`) and the on-device letter
//! capture (`native_lang::start_letter_capture`) unchanged — this module is only the
//! surface and the buffer.
//!
//! REVIEW-GATED: reachable ONLY via the hidden `spellAloudOpen` entry until
//! activation (Phase 7). The shipped voice INPUT METHOD (the mic beside the answer
//! field, in `super`) is untouched — G-C keeps both. There is **no submit** here yet
//! (turn completion via `done` is Phase 2), and no target is ever read (answer-leak
//! invariant, G-A). No Climb/shield references.

use std::cell::{Cell, RefCell};

use wasm_bindgen_futures::{spawn_local, JsFuture};

use super::{accept_into, contextual_strings, interpret, Slot, SpellOutcome};
use crate::{dom, native_lang, App};

thread_local! {
    /// The letters accepted so far this turn (one entry per spoken letter/phrase).
    static BUFFER: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
    /// True while the mic is held (a capture is in flight).
    static HOLDING: Cell<bool> = const { Cell::new(false) };
}

/// Slow echo rate — say each accepted letter clearly, not as a word.
const ECHO_RATE: f64 = 0.85;

// ---- pure render (host-unit-tested) ---------------------------------------

/// The accepted slots as HTML boxes, one per letter/phrase. Pure.
fn slots_html(slots: &[Slot]) -> String {
    slots
        .iter()
        .map(|s| format!("<span class=\"sa-slot\">{}</span>", dom::escape_html(&s.letters)))
        .collect()
}

/// The full spelled string so far (slots concatenated).
fn assembled(slots: &[Slot]) -> String {
    slots.iter().map(|s| s.letters.as_str()).collect()
}

// ---- surface --------------------------------------------------------------

pub fn open(app: &App) {
    BUFFER.with(|b| b.borrow_mut().clear());
    set_status("");
    reflect(app);
    dom::add_class("spellAloud", "show");
}

pub fn close() {
    if HOLDING.with(Cell::get) {
        native_lang::stop_letter_capture();
        HOLDING.with(|h| h.set(false));
    }
    dom::remove_class("spellAloud", "show");
}

/// Render the surface from the current buffer. Idempotent.
pub fn reflect(app: &App) {
    let _ = app;
    dom::set_text("saTitle", &crate::i18n::t("tools.spellaloud.name"));
    dom::set_text("saMicLabel", &crate::i18n::t("voiceSpell.mic"));
    let slots = BUFFER.with(|b| b.borrow().clone());
    if slots.is_empty() {
        dom::set_html(
            "saSlots",
            &format!("<span class=\"sa-empty\">{}</span>", dom::escape_html(&crate::i18n::t("tools.spellaloud.desc"))),
        );
    } else {
        dom::set_html("saSlots", &slots_html(&slots));
    }
}

fn set_status(key: &str) {
    let text = if key.is_empty() { String::new() } else { crate::i18n::t(key) };
    dom::set_text("saStatus", &text);
}

fn stop_listening_ui() {
    HOLDING.with(|h| h.set(false));
    dom::remove_class("saMic", "listening");
}

/// Wire the surface once at startup — ONLY when the feature flag is on (I6).
pub fn wire(app: &App) {
    if !super::enabled() {
        return;
    }
    let a = app.clone();
    dom::on_click("spellAloudOpen", move || open(&a));
    dom::on_click("saClose", close);
    dom::on::<web_sys::Event, _>("spellAloud", "click", |e| {
        if dom::is_self_target(&e, "spellAloud") {
            close();
        }
    });

    // Push-and-hold: press starts capture, release (up / leave / cancel) finalizes.
    let a_press = app.clone();
    dom::on::<web_sys::Event, _>("saMic", "pointerdown", move |e| {
        e.prevent_default(); // don't also fire a synthetic mouse/scroll
        press(&a_press);
    });
    for kind in ["pointerup", "pointerleave", "pointercancel"] {
        dom::on::<web_sys::Event, _>("saMic", kind, |_| release());
    }
}

/// Mic pressed — begin letter capture (idempotent while holding).
fn press(app: &App) {
    if !super::enabled() || HOLDING.with(Cell::get) {
        return;
    }
    let lang = app.borrow().lang.clone();
    // Same two conditions as the input method: per-language config + on-device bridge.
    if !crate::consts::voice_spell(&lang) || !native_lang::available() {
        return;
    }
    HOLDING.with(|h| h.set(true));
    dom::add_class("saMic", "listening");
    set_status("voiceSpell.listening");

    let ctx = contextual_strings(&lang);
    let a_fin = app.clone();
    let lang_f = lang.clone();
    let a_err = app.clone();
    let ok = native_lang::start_letter_capture(
        &lang,
        &ctx,
        |_partial| {}, // Phase 1 commits on release; live partial preview is later polish
        move |transcript| on_final(&a_fin, &lang_f, &transcript),
        move |code| on_error(&a_err, &code),
    );
    if !ok {
        stop_listening_ui();
        set_status("voiceSpell.didntCatch");
    }
}

/// Mic released — finalize the in-flight capture (the plugin fires `on_final`).
fn release() {
    if HOLDING.with(Cell::get) {
        native_lang::stop_letter_capture();
        stop_listening_ui();
    }
}

/// A capture finalized — fold the accepted letters into the buffer, echo the new
/// ones, and re-render. A whole word / babble / silence nudges instead.
fn on_final(app: &App, lang: &str, transcript: &str) {
    stop_listening_ui();
    let added = BUFFER.with(|b| accept_into(&mut b.borrow_mut(), lang, transcript));
    if added.is_empty() {
        set_status(match interpret(lang, transcript) {
            SpellOutcome::Nothing => "voiceSpell.didntCatch",
            _ => "voiceSpell.spellItOut",
        });
        return;
    }
    set_status("");
    reflect(app);
    crate::haptics::key_tap();
    echo(app, &added);
}

/// Capture error — never blocks; surface a gentle status.
fn on_error(app: &App, code: &str) {
    let _ = app;
    stop_listening_ui();
    match code {
        "PERMISSION_DENIED" => set_status("voiceSpell.needsMic"),
        "UNAVAILABLE" => set_status("voiceSpell.needsMic"),
        _ => set_status("voiceSpell.didntCatch"),
    }
}

/// Optional spoken echo of the newly accepted letters (on-device TTS). Spoken
/// space-separated so they read as individual letters, not a word. Best-effort.
fn echo(app: &App, letters: &str) {
    let lang = app.borrow().lang.clone();
    let spaced = letters.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
    spawn_local(async move {
        if let Some(voice) = native_lang::session_voice(&lang).await {
            if let Some(p) = native_lang::speak(&spaced, &voice, ECHO_RATE) {
                let _ = JsFuture::from(p).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(letters: &[&str]) -> Vec<Slot> {
        letters.iter().map(|l| Slot { letters: l.to_string() }).collect()
    }

    #[test]
    fn slots_html_renders_one_box_per_letter_and_escapes() {
        assert_eq!(slots_html(&[]), "");
        let html = slots_html(&slots(&["c", "a", "t"]));
        assert_eq!(html.matches("sa-slot").count(), 3);
        assert!(html.contains(">c</span>") && html.contains(">a</span>") && html.contains(">t</span>"));
        // multigraph slot renders both letters in one box
        let ll = slots_html(&slots(&["ll"]));
        assert_eq!(ll.matches("sa-slot").count(), 1);
        assert!(ll.contains(">ll</span>"));
    }

    #[test]
    fn assembled_concatenates_slots() {
        assert_eq!(assembled(&slots(&["c", "a", "t"])), "cat");
        assert_eq!(assembled(&slots(&["n", "i", "ñ", "o"])), "niño");
        assert_eq!(assembled(&[]), "");
    }

    /// The buffer accumulator (the Phase-1 core): letters fill slots across presses;
    /// a whole word contributes nothing (answer-leak-safe, no target).
    #[test]
    fn accept_into_accumulates_across_presses_and_rejects_whole_words() {
        let mut buf: Vec<Slot> = Vec::new();
        assert_eq!(accept_into(&mut buf, "en", "see ay"), "ca");
        assert_eq!(accept_into(&mut buf, "en", "tee"), "t");
        assert_eq!(assembled(&buf), "cat");
        // a whole word said mid-turn adds nothing and doesn't disturb the buffer
        assert_eq!(accept_into(&mut buf, "en", "elephant"), "");
        assert_eq!(assembled(&buf), "cat");
    }
}
