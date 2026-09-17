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

thread_local! {
    /// The study language the CURRENT review sheet classified under — set by
    /// `on_recognized`, read by `reflag` so a live edit re-classifies against
    /// the same banks the original classification used (Phase 3).
    static STUDY_LANG: std::cell::RefCell<String> = std::cell::RefCell::new(String::from("en"));
}

/// The ONE visibility rule for the camera button (pure — unit-tested below).
/// Phase 6: recognizer present AND not Kid Mode (Spell Jr stays camera-ABSENT
/// per the mode-hub absence-not-locks doctrine; the plan's parental-gate open
/// item resolves as "Kid-absent" until parent-managed child profiles exist)
/// AND the photo_ocr parent-premium is owned (a free account sees NO camera)
/// AND the study language has a sound recognition path (registry `ocr_support`).
fn button_allowed(recognizer: bool, kid: bool, photo_ocr: bool, ocr: crate::consts::OcrSupport) -> bool {
    recognizer && !kid && photo_ocr && ocr != crate::consts::OcrSupport::Unsupported
}

/// Show the camera button only when `button_allowed` says so. Called on init,
/// whenever Kid Mode toggles (`settings::apply_settings`), and on tools-hub
/// flag flips. A no-op that leaves the button hidden when the flag is off —
/// preserving "flag OFF = zero diff".
pub fn reflect_visibility(app: &App) {
    if !flags::photo_list() {
        return; // button keeps its default `btn-hide`; nothing to show.
    }
    let kid = dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false);
    // OCR support is judged for the language the import would classify under —
    // the study language, or the saved speak-lang's primary subtag for My Words
    // (the same rule `on_recognized` uses).
    let study = {
        let s = app.borrow();
        if s.lang == crate::consts::MINE {
            s.custom.speak_lang.split(['-', '_']).next().unwrap_or("en").to_lowercase()
        } else {
            s.lang.clone()
        }
    };
    let show = button_allowed(
        native_lang::supported(),
        kid,
        crate::play_hub::live_entitlements().photo_ocr,
        crate::consts::ocr_support(&study),
    );
    dom::toggle_class("photoBtn", "btn-hide", !show);
}

/// Wire the photo flow. Entirely gated: when the flag is off we return before
/// touching a single listener, so the branch adds no behaviour to shipped
/// builds.
pub fn wire(app: &App) {
    if !flags::photo_list() {
        return;
    }

    // Capture -> recognize -> review. The camera button opens a SOURCE CHOOSER
    // (Eric's request): take a photo with the camera, or pick an existing one
    // from the photo library — both feed the same recognition pipeline.
    dom::on_click("photoBtn", || {
        dom::add_class("photoSrcScrim", "show");
    });
    {
        let a = app.clone();
        dom::on_click("photoSrcCamera", move || {
            dom::remove_class("photoSrcScrim", "show");
            start_capture(&a, "camera");
        });
    }
    {
        let a = app.clone();
        dom::on_click("photoSrcLibrary", move || {
            dom::remove_class("photoSrcScrim", "show");
            start_capture(&a, "library");
        });
    }
    dom::on_click("photoSrcCancel", || dom::remove_class("photoSrcScrim", "show"));
    dom::on::<web_sys::Event, _>("photoSrcScrim", "click", |e| {
        if dom::is_self_target(&e, "photoSrcScrim") {
            dom::remove_class("photoSrcScrim", "show");
        }
    });

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
            return;
        }
        // F14: apply a proposed split. ONE tap, and only because the
        // parent read the pieces and chose them — the chip is replaced
        // by its pieces in place, each one an ordinary chip that
        // re-classifies and re-flags exactly like a recognized word.
        if target.class_list().contains("pchip-split") {
            let Some(pieces) = target.get_attribute("data-split") else { return };
            let Ok(Some(row)) = target.closest(".pchip-row") else { return };
            let Some(parent) = row.parent_element() else { return };
            let include = i18n::t("photo.include");
            let remove = i18n::t("photo.remove");
            let mut html = String::new();
            for piece in pieces.split(' ').filter(|p| !p.is_empty()) {
                html.push_str(&format!(
                    "<div class=\"pchip-row\">\
                       <input class=\"pchip-on\" type=\"checkbox\" checked aria-label=\"{inc}\" />\
                       <input class=\"pchip\" type=\"text\" value=\"{val}\" \
                         autocomplete=\"off\" autocorrect=\"off\" autocapitalize=\"off\" spellcheck=\"false\" />\
                       <button type=\"button\" class=\"pchip-x\" aria-label=\"{aria}\">\u{00d7}</button>\
                       <span class=\"pchip-flag\"></span>\
                     </div>",
                    inc = dom::escape_html(&include),
                    val = dom::escape_html(piece),
                    aria = dom::escape_html(&remove),
                ));
            }
            let _ = row.insert_adjacent_html("beforebegin", &html);
            row.remove();
            // Re-flag the freshly inserted chips so each piece shows its
            // own class/flag rather than an empty status.
            if let Ok(list) = parent.query_selector_all(".pchip") {
                for i in 0..list.length() {
                    if let Some(el) = list.get(i).and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                        reflag(&el);
                    }
                }
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
        // Only the TEXT chip re-classifies; the include checkbox also fires
        // `input` and must not be read as a word.
        if !input.class_list().contains("pchip") {
            return;
        }
        reflag(&input);
    });

    reflect_visibility(app);
}

fn start_capture(app: &App, source: &str) {
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
    // Registry-driven recognizer profile (Phase 2, one accessor — consts::ocr_support):
    // Native → the language's own model, correction ON. EnglishFallback (fil/sw)
    // → the ENGLISH recognizer with correction OFF, so Vision can't "fix" a
    // Filipino word into a lookalike English one; the word banks validate (G-C).
    let primary = lang.split(['-', '_']).next().unwrap_or("en").to_lowercase();
    let (rec_lang, correction) = match crate::consts::ocr_support(&primary) {
        crate::consts::OcrSupport::EnglishFallback => ("en-US".to_string(), false),
        _ => (lang.clone(), true),
    };
    // `source` comes from the chooser: "camera" opens the camera (native falls
    // back to the library when no camera exists, e.g. simulator); "library"
    // opens the photo picker directly.
    let promise = match native_lang::recognize_word_list(&rec_lang, source, correction) {
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

/// Read `{ supported, lines }` off the resolved JS value, run the core
/// extract→classify pipeline for the STUDY language, and open the review screen.
fn on_recognized(app: &App, val: &wasm_bindgen::JsValue) {
    let lines = read_lines(val);
    // Classification language = the current study language (single source of
    // truth), NEVER a recognizer guess. "mine" (My Words) classifies under the
    // saved speak-lang's primary subtag instead — the list being extended.
    let study = {
        let s = app.borrow();
        if s.lang == crate::consts::MINE {
            s.custom.speak_lang.split(['-', '_']).next().unwrap_or("en").to_lowercase()
        } else {
            s.lang.clone()
        }
    };
    let candidates = crate::photo_import::extract_classified(&study, &lines);
    STUDY_LANG.with(|l| *l.borrow_mut() = study);
    if candidates.is_empty() {
        dom::set_text("feedback", &i18n::t("photo.empty"));
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    build_lang_options(app);
    render_chips(&candidates);
    // G-A disclosure: out-of-dictionary words are spoken with the DEVICE voice
    // (on-device AVSpeech — nothing leaves the phone). Shown only when the
    // sheet actually contains one.
    let any_custom = candidates.iter().any(|c| c.class == crate::photo_import::WordClass::Custom);
    let voice_note = if any_custom { i18n::t("photo.voiceNote") } else { String::new() };
    dom::set_text("photoVoiceNote", &voice_note);
    dom::set_text("photoNote", &i18n::t("photo.note"));
    dom::set_text("feedback", "");
    crate::lists_ui::populate("photoDest", "photoConfirm"); // F1/D1
    dom::add_class("photoScrim", "show");
}

/// TEST SEAM — render the review sheet from synthetic OCR lines.
///
/// The photo flow is native-gated (`native_lang::supported()` is false in
/// a browser), so the review sheet is unreachable in e2e through the
/// camera. F14's split proposal lives ENTIRELY in that sheet, and this
/// session has repeatedly shown that reasoning about DOM I cannot see is
/// how bugs ship. This drives the real `on_recognized` with real
/// classification and the real renderer — it fabricates the OCR result,
/// nothing downstream. Observe-only in the sense that matters: there is
/// no path here that saves a word or bypasses a gate.
#[cfg(feature = "testseam")]
pub fn seam_review_sheet(app: &App, words: Vec<String>) {
    let arr = js_sys::Array::new();
    for w in words {
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&o, &"text".into(), &w.into());
        let _ = js_sys::Reflect::set(&o, &"confidence".into(), &1.0f64.into());
        arr.push(&o);
    }
    let val = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&val, &"lines".into(), &arr);
    on_recognized(app, &val.into());
}

/// Extract the `lines: {text, confidence}[]` field from the recognizer result
/// (legacy plain-string entries read as confidence 1). Anything missing or
/// mistyped yields an empty list (treated as "no words found").
fn read_lines(val: &wasm_bindgen::JsValue) -> Vec<(String, f32)> {
    let mut out = Vec::new();
    let lines = match js_sys::Reflect::get(val, &wasm_bindgen::JsValue::from_str("lines")) {
        Ok(l) => l,
        Err(_) => return out,
    };
    if let Ok(arr) = lines.dyn_into::<js_sys::Array>() {
        for i in 0..arr.length() {
            let item = arr.get(i);
            if let Some(s) = item.as_string() {
                out.push((s, 1.0));
            } else {
                let text = js_sys::Reflect::get(&item, &wasm_bindgen::JsValue::from_str("text"))
                    .ok()
                    .and_then(|t| t.as_string());
                if let Some(text) = text {
                    let confidence = js_sys::Reflect::get(&item, &wasm_bindgen::JsValue::from_str("confidence"))
                        .ok()
                        .and_then(|c| c.as_f64())
                        .unwrap_or(1.0) as f32;
                    out.push((text, confidence));
                }
            }
        }
    }
    out
}

/// The one status label a chip shows, by priority: a gate flag ("not allowed" /
/// "can't read") beats the low-confidence hint beats the class label.
fn chip_status(class: crate::photo_import::WordClass, low: bool, reason: Option<native_lang::GateFail>) -> String {
    use crate::photo_import::WordClass;
    if let Some(r) = reason {
        return i18n::t(r.i18n_key());
    }
    if low {
        return i18n::t("photo.lowconf");
    }
    match class {
        WordClass::Custom => i18n::t("photo.class.custom"),
        WordClass::InDictionary => i18n::t("photo.class.known"),
        WordClass::Filtered => String::new(), // unreachable: Filtered always has a reason
    }
}

fn render_chips(candidates: &[crate::photo_import::Candidate]) {
    use crate::photo_import::WordClass;
    let remove_label = i18n::t("photo.remove");
    let include_label = i18n::t("photo.include");
    let mut html = String::new();
    for cand in candidates {
        let word = &cand.word;
        let reason = native_lang::gate_reason(word);
        let flagged = if reason.is_some() { " flagged" } else { "" };
        // Low-confidence chips render PRE-SHOWN, dimmed and editable — never
        // silently absent (spec: nothing is dropped for low confidence).
        let lowconf = if cand.confidence_low { " lowconf" } else { "" };
        let custom = if cand.class == WordClass::Custom { " custom" } else { "" };
        // Per-chip include toggle: flagged chips start OFF (they can't save
        // anyway until edited clean); everything else starts ON.
        let checked = if reason.is_none() { " checked" } else { "" };
        let status = chip_status(cand.class, cand.confidence_low, reason);
        // F14/D7: PROPOSE a split, never apply one. The PIECES are shown
        // rather than a count (Eric, 2026-08-06) because some proposals
        // are wrong — "Sundeep" segments to sun · deep — and reading them
        // is the only way a parent can tell. One tap swaps this chip for
        // the pieces; ignoring it leaves the chip exactly as it was.
        // Skipped on flagged chips: those must be edited clean first.
        let split = match crate::photo_import::propose_split(&STUDY_LANG.with(|l| l.borrow().clone()), word) {
            Some(pieces) if reason.is_none() => format!(
                "<button type=\"button\" class=\"pchip-split\" data-split=\"{joined}\">{label} {shown}</button>",
                joined = dom::escape_html(&pieces.join(" ")),
                label = dom::escape_html(&i18n::t("photo.split")),
                shown = dom::escape_html(&pieces.join(" \u{00b7} ")),
            ),
            _ => String::new(),
        };
        html.push_str(&format!(
            "<div class=\"pchip-row{flagged}{lowconf}{custom}\">\
               <input class=\"pchip-on\" type=\"checkbox\"{checked} aria-label=\"{inc}\" />\
               <input class=\"pchip\" type=\"text\" value=\"{val}\" \
                 autocomplete=\"off\" autocorrect=\"off\" autocapitalize=\"off\" spellcheck=\"false\" />\
               <button type=\"button\" class=\"pchip-x\" aria-label=\"{aria}\">\u{00d7}</button>\
               <span class=\"pchip-flag\">{status}</span>{split}\
             </div>",
            flagged = flagged,
            lowconf = lowconf,
            custom = custom,
            checked = checked,
            inc = dom::escape_html(&include_label),
            val = dom::escape_html(word),
            aria = dom::escape_html(&remove_label),
            status = dom::escape_html(&status),
            split = split,
        ));
    }
    dom::set_html("photoChips", &html);
}

/// Live re-classify one chip after an edit (Phase 3): the gate flag AND the
/// dictionary class update as the parent types, so a fixed misread clears its
/// flag (and re-labels "in dictionary") before they even hit save.
fn reflag(input: &web_sys::HtmlInputElement) {
    let row = match input.closest(".pchip-row") {
        Ok(Some(r)) => r,
        _ => return,
    };
    let word = input.value();
    let word = word.trim();
    let reason = native_lang::gate_reason(word);
    let study = STUDY_LANG.with(|l| l.borrow().clone());
    let class = crate::photo_import::classify_word(&study, word);
    let _ = row.class_list().toggle_with_force("flagged", reason.is_some());
    let _ = row
        .class_list()
        .toggle_with_force("custom", class == crate::photo_import::WordClass::Custom);
    // A human just read and edited this chip — low-confidence no longer applies.
    let _ = row.class_list().remove_1("lowconf");
    if let Some(flag) = row.query_selector(".pchip-flag").ok().flatten() {
        flag.set_text_content(Some(&chip_status(class, false, reason)));
    }
    // Editing into a blocked word un-includes the chip; editing clean re-includes
    // it (the natural intent after fixing a misread).
    if let Some(on) = row
        .query_selector(".pchip-on")
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        on.set_checked(reason.is_none());
    }
}

/// Collect the current chip words in order — edited, non-deleted, AND with
/// their include toggle ON (Phase 3: per-chip opt-out without deleting).
fn collect_words() -> Vec<String> {
    let mut out = Vec::new();
    let list = match dom::el("photoChips").query_selector_all(".pchip-row") {
        Ok(l) => l,
        Err(_) => return out,
    };
    for i in 0..list.length() {
        let Some(row) = list.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) else {
            continue;
        };
        let included = row
            .query_selector(".pchip-on")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|c| c.checked())
            .unwrap_or(true);
        if !included {
            continue;
        }
        if let Some(input) = row
            .query_selector(".pchip")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
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
    // Custom-word marking (Phase 4): the words leaving this sheet classified
    // OUT-OF-DICTIONARY for the sheet's study language — persisted so later
    // surfaces (device-voice audio, promote-only recheck) know which are custom.
    let study = STUDY_LANG.with(|l| l.borrow().clone());
    let custom_marks: Vec<String> = words
        .iter()
        .filter(|w| crate::photo_import::classify_word(&study, w) == crate::photo_import::WordClass::Custom)
        .cloned()
        .collect();
    // CC-MYWORDS-LISTS F1: the save lands in a list the player chose, and adds
    // to it. Nothing here can remove a word -- the destructive checkbox that
    // used to sit above this button is gone.
    let dest = crate::lists_ui::chosen("photoDest");
    let entries: Vec<(String, String)> =
        words.iter().map(|w| (w.clone(), speak_lang.clone())).collect();
    let (list_name, _) = crate::lists_ui::commit(dest, &entries, crate::word_lists::ListSource::Photo);
    crate::apply_saved_words(app, words, speak_lang, &custom_marks);
    close();
    let msg = if blocked > 0 {
        i18n::tp("import.savedSkipped", &[("n", &count.to_string()), ("b", &blocked.to_string())])
    } else {
        crate::lists_ui::saved_note(&list_name, count)
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

#[cfg(test)]
mod gating_tests {
    use super::button_allowed;
    use crate::consts::OcrSupport::{EnglishFallback, Native, Unsupported};

    /// Phase 6 / acceptance #6: the camera is ABSENT (not locked) for a free
    /// account, in Spell Jr, without a recognizer, or for a language with no
    /// sound OCR path — and shown only when every condition holds.
    #[test]
    fn camera_button_rule() {
        assert!(button_allowed(true, false, true, Native));
        assert!(button_allowed(true, false, true, EnglishFallback), "fallback langs keep the camera");
        assert!(!button_allowed(true, false, false, Native), "free account: no camera");
        assert!(!button_allowed(true, true, true, Native), "Spell Jr: camera absent");
        assert!(!button_allowed(false, false, true, Native), "no recognizer: no camera");
        assert!(!button_allowed(true, false, true, Unsupported), "unsupported language: hidden");
    }
}
