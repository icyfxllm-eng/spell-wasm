//! Observation-only test seam for the Playwright E2E suite. Compiled ONLY under
//! `--features testseam`; a production build has no trace of it (CI proves this
//! by grepping the shipped bundle). It exposes `window.__spelltest`:
//!
//!   __spelltest.currentWord()      -> the answer string the player must type
//!   __spelltest.currentSpoken()    -> what TTS speaks (hanzi for zh, else = word)
//!   __spelltest.currentTier()      -> the active difficulty tier
//!   __spelltest.currentLang()      -> the active word language
//!   __spelltest.rate()             -> the live TTS playback rate
//!   __spelltest.photoReview(words) -> render the import review sheet
//!   __spelltest.pool(lang, tier)   -> the full word bank for (lang, tier), JSON
//!   __spelltest.build()            -> "testseam" marker string
//!   __spelltest.picWord()          -> the word Spell Picture is waiting on
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
        // F14. The photo review sheet is native-gated, so e2e cannot
        // reach it through the camera — and the split proposal lives
        // only there. This fabricates the OCR RESULT and lets the real
        // classifier and renderer run; it saves nothing and skips no
        // gate. Without it, F14's UI would ship having never rendered.
        let a = app.clone();
        let cb = Closure::<dyn Fn(js_sys::Array)>::new(move |words: js_sys::Array| {
            let list: Vec<String> = words.iter().filter_map(|w| w.as_string()).collect();
            crate::photo_list::seam_review_sheet(&a, list);
        });
        set(&obj, "photoReview", cb.into_js_value());
    }
    {
        // AUDITPASS F8. Derived state the player can already hear, so
        // reading it observes rather than bypasses — same category as
        // currentTier(). The Rust test pins the CONSTANT (0.55); this
        // lets e2e prove the constant actually reached a running
        // session, which is the failure this whole audit was about: a
        // value that is correct in source and never arrives.
        let a = app.clone();
        let cb = Closure::<dyn Fn() -> f64>::new(move || a.borrow().rate as f64);
        set(&obj, "rate", cb.into_js_value());
    }
    // Spell Picture keeps its own feed, so the base game's currentWord is
    // not the picture's. CC-FINALE Done #2 needs a picture played to
    // completion to reach the reveal at all. Absent on the site build with
    // the mode itself -- the wall spec asserts the mode is gone, and a seam
    // that named it would itself be a leak.
    #[cfg(not(feature = "web"))]
    {
        let cb = Closure::<dyn Fn() -> String>::new(crate::wordpic_screen::seam_current_word);
        set(&obj, "picWord", cb.into_js_value());
        let cb = Closure::<dyn Fn() -> String>::new(crate::wordpic_screen::seam_ladder);
        set(&obj, "picLadder", cb.into_js_value());
        let cb = Closure::<dyn Fn(String) -> js_sys::Promise>::new(|product: String| {
            wasm_bindgen_futures::future_to_promise(async move {
                crate::wordpic_screen::seam_export_png(product)
                    .await
                    .map(JsValue::from)
                    .map_err(|e| JsValue::from_str(&e))
            })
        });
        set(&obj, "picExportPng", cb.into_js_value());
    }
    // The export renderer's 1× SVG for the open picture (a Promise —
    // fonts are fetched). OBSERVE-only: renders the same bytes the Save
    // path would rasterize, saves nothing, touches no state.
    #[cfg(not(feature = "web"))]
    {
        let cb = Closure::<dyn Fn() -> js_sys::Promise>::new(|| {
            wasm_bindgen_futures::future_to_promise(async {
                crate::wordpic_screen::seam_export_svg()
                    .await
                    .map(JsValue::from)
                    .map_err(|e| JsValue::from_str(&e))
            })
        });
        set(&obj, "picExportSvg", cb.into_js_value());
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
