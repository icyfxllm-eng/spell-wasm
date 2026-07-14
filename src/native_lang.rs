//! Thin Rust view of the `window.SpellNativeLang` bridge defined in
//! `native-language-kit.js`. On the Capacitor build it wraps the native
//! NativeLanguageKit plugin (offline AVSpeech TTS, UITextChecker word check,
//! NLLanguageRecognizer detection); on web/PWA/Tauri it's a stub whose
//! `available()` is false and whose capability calls resolve to explicit
//! all-false shapes. Every entry point here degrades to `None`/no-op when the
//! bridge is missing, so callers keep exactly one code path.
//!
//! Doctrine: this is the ONLY place (besides the audio router) that reaches the
//! native capability layer — no platform conditionals leak into game logic.

use std::cell::RefCell;
use std::collections::HashMap;

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn bridge() -> Option<JsValue> {
    let win = web_sys::window()?;
    let obj = Reflect::get(&win, &JsValue::from_str("SpellNativeLang")).ok()?;
    if obj.is_undefined() || obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

fn method(obj: &JsValue, name: &str) -> Option<Function> {
    Reflect::get(obj, &JsValue::from_str(name)).ok()?.dyn_into::<Function>().ok()
}

fn get(obj: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(obj, &JsValue::from_str(key)).ok()
}

/// True only on the native build with the NativeLanguageKit plugin present.
pub fn available() -> bool {
    (|| -> Option<bool> {
        let obj = bridge()?;
        let f = method(&obj, "available")?;
        Some(f.call0(&obj).ok()?.as_bool().unwrap_or(false))
    })()
    .unwrap_or(false)
}

/// Offline TTS: speak `text` with `voice_id` at the game `rate`. Returns the JS
/// promise (resolves on completion, rejects on cancel/failure). `None` when the
/// bridge isn't callable — caller falls through to the next audio source.
pub fn speak(text: &str, voice_id: &str, rate: f64) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "speak")?;
    let r = f
        .call3(
            &obj,
            &JsValue::from_str(text),
            &JsValue::from_str(voice_id),
            &JsValue::from_f64(rate),
        )
        .ok()?;
    r.dyn_into::<Promise>().ok()
}

/// Stop any in-flight native utterance (fire-and-forget).
pub fn stop() {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "stop") {
            let _ = f.call0(&obj);
        }
    }
}

/// F3 — mirror the widget-state JSON (see `crate::widgets`) into the native App
/// Group container so the WidgetKit extension (and F4 App Intents) can read it.
/// Fire-and-forget; no-op when the bridge is absent (web/PWA/Android/Tauri).
pub fn sync_widget_state(state_json: &str) {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "syncWidgetState") {
            let _ = f.call1(&obj, &JsValue::from_str(state_json));
        }
    }
}

fn call1_promise(name: &str, arg0: &str) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, name)?;
    f.call1(&obj, &JsValue::from_str(arg0)).ok()?.dyn_into::<Promise>().ok()
}

/// `checkWord(word, lang)` → resolves `{supported, isWord}`. Returns `None` when
/// the bridge is absent (caller skips the dictionary gate).
pub fn check_word(word: &str, lang: &str) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "checkWord")?;
    f.call2(&obj, &JsValue::from_str(word), &JsValue::from_str(lang)).ok()?.dyn_into::<Promise>().ok()
}

/// `detectLanguage(text)` → resolves `{supported, lang, confidence}`.
pub fn detect_language(text: &str) -> Option<Promise> {
    call1_promise("detectLanguage", text)
}

/// `capabilities(lang)` → resolves the full CapabilityReport.
pub fn capabilities(lang: &str) -> Option<Promise> {
    call1_promise("capabilities", lang)
}

thread_local! {
    // Per-session chosen voice id per language (Decision D3: highest-quality
    // installed voice, stable per session). The Swift catalog already returns
    // voices best-quality-first, so index 0 is the pick.
    static SESSION_VOICE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// The session's chosen native voice id for `lang`, discovered once via
/// `capabilities` then cached. `None` when TTS is unavailable for the language.
pub async fn session_voice(lang: &str) -> Option<String> {
    if let Some(v) = SESSION_VOICE.with(|c| c.borrow().get(lang).cloned()) {
        return Some(v);
    }
    let report = JsFuture::from(capabilities(lang)?).await.ok()?;
    let tts = get(&report, "tts")?;
    if !get(&tts, "available")?.as_bool().unwrap_or(false) {
        return None;
    }
    let voices = js_sys::Array::from(&get(&tts, "voices")?);
    let first = voices.get(0);
    if first.is_undefined() {
        return None;
    }
    let id = get(&first, "id")?.as_string()?;
    SESSION_VOICE.with(|c| c.borrow_mut().insert(lang.to_string(), id.clone()));
    Some(id)
}

/// Result of the on-device word check, already awaited and parsed.
pub struct WordVerdict {
    pub supported: bool,
    pub is_word: bool,
}

/// Await `checkWord`. Returns `supported:false` whenever the bridge is absent or
/// the call fails, so the caller uniformly skips the gate in those cases.
pub async fn check_word_await(word: &str, lang: &str) -> WordVerdict {
    let fallback = WordVerdict { supported: false, is_word: false };
    let Some(promise) = check_word(word, lang) else { return fallback };
    let Ok(v) = JsFuture::from(promise).await else { return fallback };
    let supported = get(&v, "supported").and_then(|x| x.as_bool()).unwrap_or(false);
    let is_word = get(&v, "isWord").and_then(|x| x.as_bool()).unwrap_or(false);
    WordVerdict { supported, is_word }
}

/// What to do with a custom word UITextChecker doesn't recognize (Decision D2).
/// Behind ONE config flag so block/warn is a flip, not a rewrite.
#[derive(Clone, Copy, PartialEq)]
pub enum WordCheckPolicy {
    Off,
    Warn,
    Block,
}

/// Resolves the dictionary-gate policy. Override via localStorage
/// `spell_wordcheck_policy` ("off" | "warn" | "block"); the default is the
/// recommendation on file — WARN for adult flows, BLOCK in Kid Mode.
pub fn wordcheck_policy(kid: bool) -> WordCheckPolicy {
    match crate::storage::get_raw("spell_wordcheck_policy").as_deref() {
        Some("off") => WordCheckPolicy::Off,
        Some("warn") => WordCheckPolicy::Warn,
        Some("block") => WordCheckPolicy::Block,
        _ => {
            if kid {
                WordCheckPolicy::Block
            } else {
                WordCheckPolicy::Warn
            }
        }
    }
}

/// Await `detectLanguage`. Returns `(supported, lang, confidence)`.
pub async fn detect_language_await(text: &str) -> (bool, String, f64) {
    let Some(promise) = detect_language(text) else { return (false, String::new(), 0.0) };
    let Ok(v) = JsFuture::from(promise).await else { return (false, String::new(), 0.0) };
    let supported = get(&v, "supported").and_then(|x| x.as_bool()).unwrap_or(false);
    let lang = get(&v, "lang").and_then(|x| x.as_string()).unwrap_or_default();
    let confidence = get(&v, "confidence").and_then(|x| x.as_f64()).unwrap_or(0.0);
    (supported, lang, confidence)
}
