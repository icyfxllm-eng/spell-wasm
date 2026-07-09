//! The custom on-screen keyboard — the only way to type an answer on touch
//! devices. There is no `<input>` anywhere in the answer flow, so the iOS
//! system keyboard (and its dictation key + autocorrect / QuickType bar) never
//! opens during a round: the anti-cheat property this module exists to
//! guarantee. Physical keyboards still work on desktop via a window-level
//! keydown that routes into the same answer state (game::type_char/backspace).

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::App;
use crate::{dom, game};

// QWERTY rows. The apostrophe/hyphen keys are appended to the last row but
// only shown for My Words (game::sync_keyboard toggles `show-punct`), since the
// built-in English tiers never contain them.
const ROW1: &str = "qwertyuiop";
const ROW2: &str = "asdfghjkl";
const ROW3: &str = "zxcvbnm";

pub fn setup(app: &App) {
    build_keys(app);
    wire_actions(app);
    wire_physical(app);
    wire_paste_guard();
    game::sync_keyboard(app);
}

fn key_button(ch: char, punct: bool) -> String {
    let cls = if punct { "kb-key kb-punct" } else { "kb-key" };
    let label = match ch {
        '\'' => "apostrophe".to_string(),
        '-' => "hyphen".to_string(),
        c => c.to_string(),
    };
    let face = if ch.is_ascii_alphabetic() { ch.to_ascii_uppercase().to_string() } else { ch.to_string() };
    format!("<button class=\"{cls}\" data-k=\"{ch}\" aria-label=\"{label}\">{face}</button>")
}

fn build_keys(app: &App) {
    let row = |letters: &str, punct: bool| {
        let mut h = String::new();
        for c in letters.chars() {
            h.push_str(&key_button(c, false));
        }
        if punct {
            h.push_str(&key_button('\'', true));
            h.push_str(&key_button('-', true));
        }
        h
    };
    dom::set_html("kbRow1", &row(ROW1, false));
    dom::set_html("kbRow2", &row(ROW2, false));
    dom::set_html("kbRow3", &row(ROW3, true));

    // Wire each letter/punctuation key to type its character.
    if let Ok(list) = dom::doc().query_selector_all("#gameKeyboard .kb-key[data-k]") {
        for i in 0..list.length() {
            let Some(node) = list.get(i) else { continue };
            let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() else { continue };
            let Some(ch) = el.get_attribute("data-k").and_then(|k| k.chars().next()) else { continue };
            let a = app.clone();
            let cb = Closure::<dyn FnMut()>::new(move || game::type_char(&a, ch));
            let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }
}

fn wire_actions(app: &App) {
    {
        let a = app.clone();
        dom::on_click("kbBackspace", move || game::backspace(&a));
    }
    {
        let a = app.clone();
        dom::on_click("kbSubmit", move || game::submit_guess(&a));
    }
}

/// Physical keyboard (desktop): route A–Z / Backspace / Enter into the answer
/// state — unless focus is in a real text field (the name / import boxes), so
/// those keep working normally.
fn wire_physical(app: &App) {
    let a = app.clone();
    dom::on_window::<web_sys::KeyboardEvent, _>("keydown", move |e| {
        if target_is_text_field(e.as_ref()) {
            return;
        }
        let key = e.key();
        if key == "Enter" {
            game::submit_guess(&a);
            return;
        }
        if key == "Backspace" {
            e.prevent_default();
            game::backspace(&a);
            return;
        }
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        // Exactly one printable char, and one we accept.
        let mut chars = key.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c.is_ascii_alphabetic() || c == '\'' || c == '-' {
                e.prevent_default();
                game::type_char(&a, c.to_ascii_lowercase());
            }
        }
    });
}

/// Defense-in-depth: block paste on the game screen (dictation/paste insert a
/// whole word at once). A real text field like the import box still accepts it.
fn wire_paste_guard() {
    dom::on_window::<web_sys::Event, _>("paste", |e| {
        if !target_is_text_field(&e) {
            e.prevent_default();
        }
    });
}

fn target_is_text_field(e: &web_sys::Event) -> bool {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
        return false;
    };
    matches!(el.tag_name().to_uppercase().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}
