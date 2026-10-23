//! CC-SPELL-KEYBOARD O1 — "Use device keyboard".
//!
//! An accessibility setting (signed build default): the answer box on the main
//! play screen becomes a real text field, so the device's own keyboard types the
//! answer — for VoiceOver and Switch Control users, who cannot reach long-press
//! alternates, and for anyone whose own layout serves them better. Spell's
//! keyboard steps away; Check stays. Offered only with `keyboard.core`.
//!
//! What does not change:
//! * I1 — nothing corrects, predicts or capitalises: autocorrect, autocapitalise
//!   and spellcheck are off on the field, and the value is taken as typed.
//! * Typing, not dictation: every insertion is recorded in the same provenance
//!   ledger the on-screen keys use ("answerField"), so dictation and paste are
//!   still refused at submit. An IME composition (Japanese, Korean, Chinese)
//!   counts as typed, one character at a time, when it commits.
//! * Grading: the answer reaches the same `submit_guess` as any key press (I3).

use std::cell::Cell;

use wasm_bindgen::JsCast;

use crate::{dom, game, App};

const FIELD: &str = "answerDevice";
/// The provenance ledger's name for the main answer (shared with the keys).
const LEDGER: &str = "answerField";

thread_local! {
    static WAS_LIVE: Cell<bool> = const { Cell::new(false) };
}

/// Is the device keyboard the answer surface right now?
pub fn on() -> bool {
    dom::doc().body().is_some_and(|b| b.class_list().contains("device-kb"))
}

fn field() -> Option<web_sys::HtmlInputElement> {
    dom::doc().get_element_by_id(FIELD).and_then(|e| e.dyn_into().ok())
}

pub fn setup(app: &App) {
    if !dom::exists(FIELD) {
        return;
    }
    // Provenance: a typed character is one insert; a composition is counted when
    // it commits (below), because its in-progress text arrives in multi-character
    // chunks that would otherwise read as dictation.
    dom::on_before_input(FIELD, |ty, len| {
        if ty.starts_with("insertComposition") || !ty.starts_with("insert") {
            return;
        }
        crate::input_provenance::note_insert(LEDGER, &ty, len);
    });
    dom::on::<web_sys::Event, _>(FIELD, "compositionend", |e| {
        // CompositionEvent is not among this crate's web-sys features; read `data` directly.
        let data = js_sys::Reflect::get(e.as_ref(), &"data".into()).ok().and_then(|d| d.as_string()).unwrap_or_default();
        for _ in data.chars() {
            crate::input_provenance::note_insert(LEDGER, "insertText", 1);
        }
    });
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>(FIELD, "input", move |_| {
            if let Some(f) = field() {
                game::device_set_answer(&a, &f.value());
            }
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::KeyboardEvent, _>(FIELD, "keydown", move |e| {
            if e.key() == "Enter" && !e.is_composing() {
                e.prevent_default();
                game::submit_guess(&a);
            }
        });
    }
    // Tapping the answer box opens the keyboard (the field covers it, but a tap on
    // the letters' padding must land too).
    dom::on::<web_sys::MouseEvent, _>("spellbox", "click", |_| {
        if on() {
            if let Some(f) = field() {
                let _ = f.focus();
            }
        }
    });
}

/// Keep the field in step with the game: its value mirrors the answer (cleared
/// with each new word), and it is live exactly when the keys would be.
pub fn sync(app: &App) {
    let Some(f) = field() else { return };
    if !on() {
        WAS_LIVE.with(|w| w.set(false));
        if dom::doc().active_element().is_some_and(|a| a.id() == FIELD) {
            let _ = f.blur();
        }
        return;
    }
    let (answer, live) = {
        let s = app.borrow();
        (s.answer.clone(), game::can_type(&s))
    };
    if f.value() != answer {
        f.set_value(&answer);
    }
    f.set_disabled(!live);
    // A newly live word takes the focus, so the keyboard is ready without a tap
    // where the platform allows it (iOS opens it only inside a user gesture).
    // Never while a sheet or dialog is open: turning this setting on in Settings
    // with a word live behind it threw the system keyboard up over Settings.
    if live && !WAS_LIVE.with(|w| w.replace(true)) {
        if !overlay_open() {
            let _ = f.focus();
        }
    } else if !live {
        WAS_LIVE.with(|w| w.set(false));
    }
}

/// Is a sheet or dialog up over the play screen?
fn overlay_open() -> bool {
    dom::doc().query_selector(".scrim.show").ok().flatten().is_some()
}
