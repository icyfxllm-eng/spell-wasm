//! Observation-only test seam for the Playwright E2E suite. Compiled ONLY under
//! `--features testseam`; a production build has no trace of it (CI proves this
//! by grepping the shipped bundle). It exposes `window.__spelltest`:
//!
//!   __spelltest.currentWord()      -> the answer string the player must type
//!   __spelltest.currentSpoken()    -> what TTS speaks (hanzi for zh, else = word)
//!   __spelltest.currentTier()      -> the active difficulty tier
//!   __spelltest.currentLang()      -> the active word language
//!   __spelltest.pool(lang, tier)   -> the full word bank for (lang, tier), JSON
//!   __spelltest.build()            -> "testseam" marker string
//!
//! Per the harness contract, seams OBSERVE and never bypass filtering or
//! validation. There is deliberately no hook that types for the player, sets an
//! answer, or short-circuits the profanity/answer checks — E2E types via real
//! key clicks and reads the expected word here to know what to type.

use wasm_bindgen::prelude::*;

use crate::App;

pub fn install(app: &App) {
    let win = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let obj = js_sys::Object::new();

    let set = |obj: &js_sys::Object, name: &str, f: JsValue| {
        let _ = js_sys::Reflect::set(obj, &JsValue::from_str(name), &f);
    };

    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> String>::new(move || a.borrow().word.clone());
        set(&obj, "currentWord", cb.into_js_value());
    }
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> String>::new(move || {
            let s = a.borrow();
            if s.spoken.is_empty() { s.word.clone() } else { s.spoken.clone() }
        });
        set(&obj, "currentSpoken", cb.into_js_value());
    }
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> String>::new(move || a.borrow().cur_tier.clone());
        set(&obj, "currentTier", cb.into_js_value());
    }
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> String>::new(move || a.borrow().cur_lang.clone());
        set(&obj, "currentLang", cb.into_js_value());
    }
    {
        let cb = Closure::<dyn Fn(String, String) -> JsValue>::new(move |lang: String, tier: String| {
            let words: Vec<&str> = crate::words::tier_for(&lang, &tier).to_vec();
            let arr = js_sys::Array::new();
            for w in words {
                arr.push(&JsValue::from_str(w));
            }
            arr.into()
        });
        set(&obj, "pool", cb.into_js_value());
    }
    {
        let cb = Closure::<dyn Fn() -> String>::new(move || "testseam".to_string());
        set(&obj, "build", cb.into_js_value());
    }
    // Daily Challenge observation (OBSERVE-only, like the rest of the seam): the
    // 0-based cursor into the fixed set, the running correct count, and whether a
    // run is active. Lets E2E assert auto-advance/skip advance the index by
    // exactly one without bypassing validation or typing for the player.
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> f64>::new(move || a.borrow().daily.idx as f64);
        set(&obj, "dailyIdx", cb.into_js_value());
    }
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> f64>::new(move || a.borrow().daily.correct as f64);
        set(&obj, "dailyCorrect", cb.into_js_value());
    }
    {
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> bool>::new(move || a.borrow().daily.active);
        set(&obj, "dailyActive", cb.into_js_value());
    }

    let _ = js_sys::Reflect::set(win.as_ref(), &JsValue::from_str("__spelltest"), obj.as_ref());
    web_sys::console::warn_1(&"[testseam] window.__spelltest installed (DEV build only)".into());
}
