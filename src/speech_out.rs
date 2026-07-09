use std::cell::RefCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{SpeechSynthesisUtterance, SpeechSynthesisVoice};

use crate::dom::{self, add_class, remove_class};

thread_local! {
    static VOICES: RefCell<Vec<SpeechSynthesisVoice>> = RefCell::new(Vec::new());
}

fn synth() -> Option<web_sys::SpeechSynthesis> {
    // On Android WebView `window.speechSynthesis` is absent, but the getter
    // returns `undefined` rather than throwing — so web_sys hands back
    // `Ok(<undefined>)`. Calling `.get_voices()` on that traps the whole WASM
    // module (aborting `start()` before event wiring runs, which silently
    // bricks the app). Reject undefined/null here so the Web Speech path is
    // simply treated as unavailable on platforms that lack it.
    let s = dom::window().speech_synthesis().ok()?;
    let v: &wasm_bindgen::JsValue = s.as_ref();
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(s)
    }
}

pub fn load_voices() {
    if let Some(s) = synth() {
        let arr = s.get_voices();
        let voices: Vec<SpeechSynthesisVoice> = arr.iter().filter_map(|v| v.dyn_into::<SpeechSynthesisVoice>().ok()).collect();
        VOICES.with(|v| *v.borrow_mut() = voices);
    }
}

pub fn voice_for_code(code: &str) -> Option<SpeechSynthesisVoice> {
    let base = code.split('-').next().unwrap_or(code);
    VOICES.with(|v| {
        let list = v.borrow();
        list.iter()
            .find(|voice| voice.lang() == code)
            .or_else(|| {
                list.iter().find(|voice| {
                    voice.lang().replace('_', "-").split('-').next().unwrap_or("") == base
                })
            })
            .cloned()
    })
}

/// Wire the browser's `voiceschanged` event to keep our cached voice list fresh,
/// then re-run `on_change` (e.g. to refresh the "no voice installed" hint).
pub fn setup_voice_loading<F: Fn() + 'static>(on_change: F) {
    load_voices();
    on_change();
    if let Some(s) = synth() {
        let cb = Closure::<dyn FnMut()>::new(move || {
            load_voices();
            on_change();
        });
        s.set_onvoiceschanged(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    }
}

/// Cancel any queued/speaking browser TTS (used on mode teardown).
pub fn stop() {
    if let Some(s) = synth() {
        s.cancel();
    }
}

pub fn speak(text: &str, rate: f32, code: &str) {
    let Some(s) = synth() else {
        dom::set_text("feedback", "This browser can't speak \u{2014} try Chrome or Edge.");
        dom::el("feedback").set_class_name("feedback bad");
        return;
    };
    s.cancel();
    let utter = SpeechSynthesisUtterance::new_with_text(text).unwrap();
    if let Some(v) = voice_for_code(code) {
        utter.set_voice(Some(&v));
    }
    utter.set_lang(code);
    utter.set_rate(rate);

    let start_cb = Closure::<dyn FnMut()>::new(|| add_class("orbWrap", "speaking"));
    utter.set_onstart(Some(start_cb.as_ref().unchecked_ref()));
    start_cb.forget();

    let end_cb = Closure::<dyn FnMut()>::new(|| remove_class("orbWrap", "speaking"));
    utter.set_onend(Some(end_cb.as_ref().unchecked_ref()));
    end_cb.forget();

    let err_cb = Closure::<dyn FnMut()>::new(|| remove_class("orbWrap", "speaking"));
    utter.set_onerror(Some(err_cb.as_ref().unchecked_ref()));
    err_cb.forget();

    s.speak(&utter);
}
