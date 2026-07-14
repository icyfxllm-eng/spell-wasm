//! F1 "Photo-to-word-list" UI flow (feature-flagged OFF by default via
//! `flags::photo_list`). Augments the existing "My Words" surface with a camera
//! affordance: photograph a spelling handout, the native VisionKit recognizer
//! (`native_lang`) reads it **on-device**, and the recognized words land in an
//! editable review screen before ANY of them are saved.
//!
//! Trust model: OCR output is never saved directly. Recognized text is parsed
//! for shape only (`native_lang::parse_candidates`), pre-flagged against the
//! standard gate for the review UI (`native_lang::gate_reason`), and the
//! confirmed set is pushed through the *exact* typed-importer save path
//! (`importer::extract_words` charset gate -> `profanity` screen -> `save_words`)
//! — see `crate::apply_saved_words`. The camera button is capability-driven
//! (shown only where the recognizer is `supported()`) and hidden in Kid Mode,
//! matching how other parent-only surfaces are gated (`climb::reflect_auth`).

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::{dom, flags, i18n, native_lang, App};

/// Show the camera button only when the feature is on, the native recognizer is
/// present, and we're not in Kid Mode. Called on init and whenever Kid Mode
/// toggles (`settings::apply_settings`). A no-op that leaves the button hidden
/// when the flag is off — preserving "flag OFF = zero diff".
pub fn reflect_visibility() {
    if !flags::photo_list() {
        return; // button keeps its default `btn-hide`; nothing to show.
    }
    let kid = dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false);
    let show = native_lang::supported() && !kid;
    dom::toggle_class("photoBtn", "btn-hide", !show);
}

/// Wire the photo flow. Entirely gated: when the flag is off we return before
/// touching a single listener, so the branch adds no behaviour to shipped
/// builds.
pub fn wire(app: &App) {
    if !flags::photo_list() {
        return;
    }

    // Capture -> recognize -> review.
    {
        let a = app.clone();
        dom::on_click("photoBtn", move || start_capture(&a));
    }

    // Confirm the reviewed set (routes through the standard save gate).
    {
        let a = app.clone();
        dom::on_click("photoConfirm", move || confirm(&a));
    }

    dom::on_click("photoCancel", || close());
    dom::on::<web_sys::Event, _>("photoScrim", "click", |e| {
        if dom::is_self_target(&e, "photoScrim") {
            close();
        }
    });

    // Chip interactions are delegated to the static container so they survive
    // each re-render of the chip list.
    dom::on::<web_sys::MouseEvent, _>("photoChips", "click", |e| {
        let target = match e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            Some(t) => t,
            None => return,
        };
        if target.class_list().contains("pchip-x") {
            if let Ok(Some(row)) = target.closest(".pchip-row") {
                row.remove();
            }
        }
    });
    // Live re-flag as the user edits a chip, so a fixed misread clears its flag
    // (and an edit into a bad word gets flagged) before they even hit save.
    dom::on::<web_sys::Event, _>("photoChips", "input", |e| {
        let input = match e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            Some(i) => i,
            None => return,
        };
        reflag(&input);
    });

    reflect_visibility();
}

fn start_capture(app: &App) {
    // Recognition language seeds Vision's recognitionLanguages; reuse the saved
    // "Speak in" language, defaulting to en-US.
    let lang = {
        let s = app.borrow();
        if !s.custom.speak_lang.is_empty() {
            s.custom.speak_lang.clone()
        } else {
            "en-US".to_string()
        }
    };
    let promise = match native_lang::recognize_word_list(&lang, "auto") {
        Some(p) => p,
        None => return, // recognizer vanished (shouldn't happen; button is gated).
    };

    // Busy state: the native picker + recognition can take a moment.
    dom::set_disabled("photoBtn", true);
    dom::set_text("feedback", &i18n::t("photo.reading"));
    dom::el("feedback").set_class_name("feedback");

    let a = app.clone();
    spawn_local(async move {
        let result = JsFuture::from(promise).await;
        dom::set_disabled("photoBtn", false);
        match result {
            Ok(val) => on_recognized(&a, &val),
            // Reject = user cancelled the picker or recognition failed. Stay
            // quiet on cancel; there's nothing to show.
            Err(_) => dom::set_text("feedback", ""),
        }
    });
}

/// Read `{ supported, lines }` off the resolved JS value, parse candidates, and
/// open the review screen.
fn on_recognized(app: &App, val: &wasm_bindgen::JsValue) {
    let lines = read_lines(val);
    let candidates = native_lang::parse_candidates(&lines);
    if candidates.is_empty() {
        dom::set_text("feedback", &i18n::t("photo.empty"));
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    build_lang_options(app);
    render_chips(&candidates);
    dom::set_text("photoNote", &i18n::t("photo.note"));
    dom::set_text("feedback", "");
    dom::add_class("photoScrim", "show");
}

/// Extract the `lines: string[]` field from the recognizer result. Anything
/// missing/mistyped yields an empty list (treated as "no words found").
fn read_lines(val: &wasm_bindgen::JsValue) -> Vec<String> {
    let mut out = Vec::new();
    let lines = match js_sys::Reflect::get(val, &wasm_bindgen::JsValue::from_str("lines")) {
        Ok(l) => l,
        Err(_) => return out,
    };
    if let Ok(arr) = lines.dyn_into::<js_sys::Array>() {
        for i in 0..arr.length() {
            if let Some(s) = arr.get(i).as_string() {
                out.push(s);
            }
        }
    }
    out
}

fn render_chips(candidates: &[String]) {
    let remove_label = i18n::t("photo.remove");
    let mut html = String::new();
    for word in candidates {
        let reason = native_lang::gate_reason(word);
        let flagged = if reason.is_some() { " flagged" } else { "" };
        let reason_label = reason.map(|r| i18n::t(r.i18n_key())).unwrap_or_default();
        html.push_str(&format!(
            "<div class=\"pchip-row{flagged}\">\
               <input class=\"pchip\" type=\"text\" value=\"{val}\" \
                 autocomplete=\"off\" autocorrect=\"off\" autocapitalize=\"off\" spellcheck=\"false\" />\
               <button type=\"button\" class=\"pchip-x\" aria-label=\"{aria}\">\u{00d7}</button>\
               <span class=\"pchip-flag\">{reason}</span>\
             </div>",
            flagged = flagged,
            val = dom::escape_html(word),
            aria = dom::escape_html(&remove_label),
            reason = dom::escape_html(&reason_label),
        ));
    }
    dom::set_html("photoChips", &html);
}

/// Recompute one chip's flag after an edit.
fn reflag(input: &web_sys::HtmlInputElement) {
    let row = match input.closest(".pchip-row") {
        Ok(Some(r)) => r,
        _ => return,
    };
    let reason = native_lang::gate_reason(input.value().trim());
    let _ = row.class_list().toggle_with_force("flagged", reason.is_some());
    if let Some(flag) = row.query_selector(".pchip-flag").ok().flatten() {
        let label = reason.map(|r| i18n::t(r.i18n_key())).unwrap_or_default();
        flag.set_text_content(Some(&label));
    }
}

/// Collect the current (edited, non-deleted) chip words in order.
fn collect_words() -> Vec<String> {
    let mut out = Vec::new();
    let list = match dom::el("photoChips").query_selector_all(".pchip") {
        Ok(l) => l,
        Err(_) => return out,
    };
    for i in 0..list.length() {
        if let Some(input) = list.get(i).and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok()) {
            let v = input.value();
            let t = v.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

fn confirm(app: &App) {
    // Route the reviewed chips through the EXACT typed-importer gate: charset
    // extraction, then the profanity screen. No shortcut, no duplicated gate.
    let text = collect_words().join("\n");
    let words = crate::importer::extract_words(&text);
    if words.is_empty() {
        dom::set_text("photoNote", &i18n::t("import.needWord"));
        return;
    }
    let (words, blocked) = crate::profanity::filter_allowed(words);
    if words.is_empty() {
        dom::set_text("photoNote", crate::profanity::rejection_message());
        return;
    }
    let speak_lang = dom::select("photoLang").value();
    let count = words.len();
    crate::apply_saved_words(app, words, speak_lang);
    close();
    let msg = if blocked > 0 {
        i18n::tp("import.savedSkipped", &[("n", &count.to_string()), ("b", &blocked.to_string())])
    } else {
        i18n::tp("import.saved", &[("n", &count.to_string())])
    };
    dom::set_text("feedback", &msg);
    dom::el("feedback").set_class_name("feedback good");
}

fn close() {
    dom::remove_class("photoScrim", "show");
    dom::set_html("photoChips", "");
}

/// Populate the review screen's "Speak in" select with the same language list
/// the typed importer uses.
fn build_lang_options(app: &App) {
    let s = app.borrow();
    let opts: String = crate::words::LANGUAGES
        .iter()
        .map(|(_, l)| format!("<option value=\"{}\">{}</option>", l.code, dom::escape_html(l.name)))
        .collect();
    dom::set_html("photoLang", &opts);
    let value = if !s.custom.speak_lang.is_empty() {
        s.custom.speak_lang.clone()
    } else {
        "en-US".to_string()
    };
    dom::select("photoLang").set_value(&value);
}
