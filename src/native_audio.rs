//! Thin Rust view of the `window.SpellAudio` bridge defined in
//! `audio-native.js`. On the Capacitor build that object wraps the native
//! NativeAudio + Filesystem plugins; on the web it's a stub whose
//! `available()` returns false. Every entry point here degrades to a no-op /
//! `None` when the bridge is missing, so callers can fall back to the browser
//! `<audio>` path. See `api::play_word`.

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};

fn bridge() -> Option<JsValue> {
    let win = web_sys::window()?;
    let obj = Reflect::get(&win, &JsValue::from_str("SpellAudio")).ok()?;
    if obj.is_undefined() || obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

fn method(obj: &JsValue, name: &str) -> Option<Function> {
    Reflect::get(obj, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<Function>()
        .ok()
}

/// True only on the native build with the NativeAudio plugin present.
pub fn available() -> bool {
    (|| -> Option<bool> {
        let obj = bridge()?;
        let f = method(&obj, "available")?;
        let r = f.call0(&obj).ok()?;
        Some(r.as_bool().unwrap_or(false))
    })()
    .unwrap_or(false)
}

/// Stable key for a word+variant clip, shared as the NativeAudio assetId and
/// (sanitized) the on-disk cache filename.
pub fn asset_id(word: &str, variant: &str, lang: &str) -> String {
    format!("w:{lang}:{variant}:{word}")
}

/// Start native playback of a word clip, downloading+caching it on first use.
/// Returns the JS promise so the caller can await it and fall back on reject.
/// `None` means the bridge isn't callable at all (fall back immediately).
pub fn play_word(asset_id: &str, url: &str) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "playWord")?;
    let r = f
        .call2(&obj, &JsValue::from_str(asset_id), &JsValue::from_str(url))
        .ok()?;
    r.dyn_into::<Promise>().ok()
}

/// Fire-and-forget: download+cache a clip to on-device storage for later
/// (offline pack / warming the next word). No playback.
pub fn prefetch(asset_id: &str, url: &str) {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "prefetch") {
            let _ = f.call2(&obj, &JsValue::from_str(asset_id), &JsValue::from_str(url));
        }
    }
}
