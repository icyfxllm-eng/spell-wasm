//! Tactile feedback on the native build via `@capacitor/haptics`, reached
//! (no bundler) through `window.Capacitor.Plugins.Haptics`. Fire-and-forget:
//! every call is a no-op on the web (plugin absent) and never throws, so game
//! code can call it unconditionally.
//!
//! Mapping (see CLAUDE.md Phase 2): a light tap confirms a correct spelling; a
//! double-buzz marks an incorrect one. Kid Mode softens the negative so a
//! wrong answer never feels like a scolding.

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

fn plugin() -> Option<JsValue> {
    let win = web_sys::window()?;
    let cap = Reflect::get(&win, &JsValue::from_str("Capacitor")).ok()?;
    if cap.is_undefined() || cap.is_null() {
        return None;
    }
    let plugins = Reflect::get(&cap, &JsValue::from_str("Plugins")).ok()?;
    let h = Reflect::get(&plugins, &JsValue::from_str("Haptics")).ok()?;
    if h.is_undefined() || h.is_null() {
        None
    } else {
        Some(h)
    }
}

/// Calls `Haptics.<method>({ <key>: <value> })`, ignoring the returned promise.
fn call(h: &JsValue, method: &str, key: &str, value: &str) {
    let Ok(f) = Reflect::get(h, &JsValue::from_str(method)) else { return };
    let Ok(f) = f.dyn_into::<Function>() else { return };
    let opts = Object::new();
    let _ = Reflect::set(&opts, &JsValue::from_str(key), &JsValue::from_str(value));
    let _ = f.call1(h, &opts);
}

/// Light tap confirming a correct spelling.
pub fn correct() {
    if let Some(h) = plugin() {
        call(&h, "impact", "style", "LIGHT");
    }
}

/// Very light tick for an on-screen keyboard key press (native only; the game
/// has no key sound effect, so this is the tactile substitute).
pub fn key_tap() {
    if let Some(h) = plugin() {
        call(&h, "impact", "style", "LIGHT");
    }
}

/// Marks an incorrect spelling: a double-buzz normally, softened to a single
/// medium tap in Kid Mode.
pub fn incorrect(kid: bool) {
    if let Some(h) = plugin() {
        if kid {
            call(&h, "impact", "style", "MEDIUM");
        } else {
            call(&h, "notification", "type", "ERROR");
        }
    }
}
