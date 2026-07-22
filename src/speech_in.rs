//! Thin manual bindings to the non-standard (webkit)SpeechRecognition API,
//! which has no `web-sys` bindings. We reach it via `js_sys::Reflect` instead
//! of writing raw JS.

use crate::consts::letter_name;
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

pub struct SpeechRec {
    inner: JsValue,
}

fn get_ctor() -> Option<Function> {
    let win = web_sys::window()?;
    for name in ["SpeechRecognition", "webkitSpeechRecognition"] {
        if let Ok(v) = Reflect::get(&win, &JsValue::from_str(name)) {
            if !v.is_undefined() && !v.is_null() {
                if let Ok(f) = v.dyn_into::<Function>() {
                    return Some(f);
                }
            }
        }
    }
    None
}

pub fn is_supported() -> bool {
    get_ctor().is_some()
}

impl SpeechRec {
    pub fn new() -> Option<Self> {
        let ctor = get_ctor()?;
        let inst = Reflect::construct(&ctor, &Array::new()).ok()?;
        Some(SpeechRec { inner: inst })
    }

    fn set_prop(&self, name: &str, value: &JsValue) {
        let _ = Reflect::set(&self.inner, &JsValue::from_str(name), value);
    }

    fn call0(&self, name: &str) {
        if let Ok(f) = Reflect::get(&self.inner, &JsValue::from_str(name)) {
            if let Ok(f) = f.dyn_into::<Function>() {
                let _ = f.call0(&self.inner);
            }
        }
    }

    pub fn set_lang(&self, lang: &str) {
        self.set_prop("lang", &JsValue::from_str(lang));
    }
    pub fn set_max_alternatives(&self, n: u32) {
        self.set_prop("maxAlternatives", &JsValue::from_f64(n as f64));
    }
    pub fn set_interim_results(&self, v: bool) {
        self.set_prop("interimResults", &JsValue::from_bool(v));
    }
    pub fn set_onresult(&self, cb: &Closure<dyn FnMut(JsValue)>) {
        self.set_prop("onresult", cb.as_ref().unchecked_ref());
    }
    pub fn set_onend(&self, cb: &Closure<dyn FnMut()>) {
        self.set_prop("onend", cb.as_ref().unchecked_ref());
    }
    pub fn set_onerror(&self, cb: &Closure<dyn FnMut()>) {
        self.set_prop("onerror", cb.as_ref().unchecked_ref());
    }
    pub fn start(&self) -> Result<(), JsValue> {
        if let Ok(f) = Reflect::get(&self.inner, &JsValue::from_str("start")) {
            if let Ok(f) = f.dyn_into::<Function>() {
                return f.call0(&self.inner).map(|_| ());
            }
        }
        Err(JsValue::from_str("no start()"))
    }
    pub fn stop(&self) {
        self.call0("stop");
    }
}

/// Pulls `event.results[0][0].transcript` out of the recognition result event.
pub fn extract_transcript(event: &JsValue) -> Option<String> {
    let results = Reflect::get(event, &JsValue::from_str("results")).ok()?;
    let r0 = Reflect::get(&results, &JsValue::from_f64(0.0)).ok()?;
    let alt0 = Reflect::get(&r0, &JsValue::from_f64(0.0)).ok()?;
    let transcript = Reflect::get(&alt0, &JsValue::from_str("transcript")).ok()?;
    transcript.as_string()
}

/// Mirrors the original `parseSpoken`: if every token looks like a spoken
/// letter name ("bee", "cee", ...) or is already a single character, treat
/// this as spelling-out-loud and join the decoded letters; otherwise just
/// concatenate the raw tokens.
pub fn parse_spoken(raw: &str) -> String {
    let tokens: Vec<String> = raw
        .trim()
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == '-')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    let spelled = tokens.len() > 1
        && tokens.iter().all(|t| t.chars().count() == 1 || letter_name(t).is_some());

    if spelled {
        tokens
            .iter()
            .map(|t| {
                if t.chars().count() == 1 {
                    t.clone()
                } else {
                    letter_name(t).map(|c| c.to_string()).unwrap_or_default()
                }
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        tokens.join("")
    }
}
