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
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use unicode_normalization::UnicodeNormalization;

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

/// Offline syllable replay (Feature F7): speak `syllables` in order as ONE
/// native utterance, invoking `on_index` with the 0-based syllable index at each
/// AVSpeech word boundary (`willSpeakRangeOfSpeechString`) so the caller can
/// highlight the revealed spelling in exact sync. Returns the completion promise
/// (resolves on natural finish, rejects on cancel/failure). `None` when the
/// bridge isn't callable — the caller then drives the highlight from web timers.
pub fn speak_syllables(syllables: &[String], voice_id: &str, rate: f64, on_index: &Function) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "speakSyllables")?;
    let arr = js_sys::Array::new();
    for s in syllables {
        arr.push(&JsValue::from_str(s));
    }
    let args = js_sys::Array::of4(&arr, &JsValue::from_str(voice_id), &JsValue::from_f64(rate), on_index);
    f.apply(&obj, &args).ok()?.dyn_into::<Promise>().ok()
}

/// Stop any in-flight native utterance (fire-and-forget).
pub fn stop() {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "stop") {
            let _ = f.call0(&obj);
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

// ---- Say It (Feature F2): on-device speech recognition bridge ----

/// Parsed `speechCapabilities(lang)` verdict. `available` already implies
/// on-device support (the JS/Swift contract), so callers only branch on it;
/// `supports_on_device` is retained for diagnostics/telemetry and to make the
/// on-device-only invariant legible at the type level.
#[allow(dead_code)]
pub struct SpeechCapability {
    pub available: bool,
    pub supports_on_device: bool,
}

fn speech_capabilities_promise(lang: &str) -> Option<Promise> {
    call1_promise("speechCapabilities", lang)
}

/// Await `speechCapabilities(lang)`. Returns an all-false verdict whenever the
/// bridge is absent (web/PWA) or the call fails — so off-iOS the mode is uniformly
/// treated as UNAVAILABLE, never as a cue to try anything server-based.
pub async fn speech_capabilities(lang: &str) -> SpeechCapability {
    let fallback = SpeechCapability { available: false, supports_on_device: false };
    let Some(promise) = speech_capabilities_promise(lang) else { return fallback };
    let Ok(v) = JsFuture::from(promise).await else { return fallback };
    let available = get(&v, "available").and_then(|x| x.as_bool()).unwrap_or(false);
    let supports_on_device = get(&v, "supportsOnDevice").and_then(|x| x.as_bool()).unwrap_or(false);
    // Defensive: never report available without on-device support, even if a
    // future bridge regressed the invariant.
    SpeechCapability { available: available && supports_on_device, supports_on_device }
}

/// Kick off on-device listening for `lang`. Returns the JS promise resolving to
/// `{ transcription }` (on-device only) or rejecting with a code string
/// ("UNAVAILABLE" | "PERMISSION_DENIED" | "BUSY" | "AUDIO_ERROR" | "NO_SPEECH").
/// `None` when the bridge isn't callable (web) — caller treats as UNAVAILABLE.
pub fn start_listening(lang: &str) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "startListening")?;
    let opts = js_sys::Object::new();
    Reflect::set(&opts, &JsValue::from_str("lang"), &JsValue::from_str(lang)).ok()?;
    f.call1(&obj, &opts).ok()?.dyn_into::<Promise>().ok()
}

/// Outcome of an on-device listen attempt.
pub enum ListenOutcome {
    /// The recognizer's final on-device transcription.
    Heard(String),
    /// Rejected with one of the documented codes (or "UNAVAILABLE" if the bridge
    /// was missing / unparseable).
    Error(String),
}

/// Await `startListening(lang)` into a parsed outcome. Never panics; a missing
/// bridge or unparseable result becomes `Error("UNAVAILABLE")`.
pub async fn start_listening_await(lang: &str) -> ListenOutcome {
    let Some(promise) = start_listening(lang) else { return ListenOutcome::Error("UNAVAILABLE".into()) };
    match JsFuture::from(promise).await {
        Ok(v) => match get(&v, "transcription").and_then(|x| x.as_string()) {
            Some(t) => ListenOutcome::Heard(t),
            None => ListenOutcome::Error("UNAVAILABLE".into()),
        },
        Err(e) => {
            // Capacitor rejects with an Error-like object; pull `.message` (the code).
            let code = get(&e, "message")
                .and_then(|x| x.as_string())
                .or_else(|| e.as_string())
                .unwrap_or_else(|| "UNAVAILABLE".into());
            ListenOutcome::Error(code)
        }
    }
}

/// Stop any in-flight listen (fire-and-forget). The pending `startListening`
/// promise then resolves with whatever was captured.
pub fn stop_listening() {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "stopListening") {
            let _ = f.call0(&obj);
        }
    }
}
/// True only on a build where the native VisionKit text recognizer is present
/// (iOS + the NativeLanguageKit plugin). Capability check — the camera
/// affordance is shown off this, never off a platform string.
pub fn supported() -> bool {
    (|| -> Option<bool> {
        let obj = bridge()?;
        let f = method(&obj, "supported")?;
        let r = f.call0(&obj).ok()?;
        Some(r.as_bool().unwrap_or(false))
    })()
    .unwrap_or(false)
}

/// Ask the native plugin to capture a photo (camera / library) and run
/// on-device text recognition, returning the JS promise that resolves to
/// `{ supported: bool, lines: string[] }`. `source` is "camera" or "library";
/// `lang` seeds Vision's `recognitionLanguages`. `None` means the bridge isn't
/// callable at all (fall back / hide the feature).
pub fn recognize_word_list(lang: &str, source: &str) -> Option<Promise> {
    let obj = bridge()?;
    let f = method(&obj, "recognizeWordList")?;
    let arg = js_sys::Object::new();
    let _ = Reflect::set(&arg, &JsValue::from_str("lang"), &JsValue::from_str(lang));
    let _ = Reflect::set(&arg, &JsValue::from_str("source"), &JsValue::from_str(source));
    let r = f.call1(&obj, &arg).ok()?;
    r.dyn_into::<Promise>().ok()
}

/// One-letter tokens that are legitimate words in the app's Latin languages
/// (Spanish "y/o/e/u/a", English "a"). Every other lone letter is OCR noise
/// (a stray stroke, a bullet the recognizer read as "l") and is dropped.
const ONE_LETTER_WORDS: &[char] = &['a', 'y', 'o', 'e', 'u'];

/// Strip the leading/trailing junk that clings to a handout token — list
/// numbering ("1.", "2)"), bullets ("•", "-", "*"), and surrounding
/// punctuation/quotes — by trimming any non-letter run from both ends. Internal
/// apostrophes and hyphens ("don't", "co-op") survive because they sit between
/// letters. Returns the trimmed slice (may be empty).
fn strip_edges(tok: &str) -> &str {
    tok.trim_matches(|c: char| !c.is_alphabetic())
}

/// Turn raw recognized lines into de-duplicated candidate words, applying only
/// *shape* cleanup — the trust gate (charset + profanity) is applied separately
/// so OCR output and typed input share one code path. Rules:
///   * split each line on whitespace;
///   * strip list numbering / bullets / edge punctuation;
///   * NFC-normalize;
///   * drop tokens containing an ASCII digit (page numbers, "H2O" noise);
///   * drop lone letters except the valid one-letter es/en words;
///   * de-dupe case-insensitively, preserving first-seen order and casing.
pub fn parse_candidates(lines: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        for raw_tok in line.split_whitespace() {
            // NFC first so a trailing combining mark composes onto its letter
            // ("e" + U+0301 -> "é") instead of being trimmed off as a non-letter;
            // this also matches the profanity screen, which works in NFC.
            let composed: String = raw_tok.nfc().collect();
            let trimmed = strip_edges(&composed);
            if trimmed.is_empty() {
                continue;
            }
            let word: String = trimmed.to_string();
            // A word must contain at least one letter and no ASCII digit —
            // spelling words don't carry digits; anything with one is noise.
            if !word.chars().any(|c| c.is_alphabetic()) {
                continue;
            }
            if word.chars().any(|c| c.is_ascii_digit()) {
                continue;
            }
            // Lone letters: only the handful of real one-letter words survive.
            let mut chars = word.chars();
            let first = chars.next();
            if let (Some(c), None) = (first, chars.next()) {
                if !ONE_LETTER_WORDS.contains(&c.to_ascii_lowercase()) {
                    continue;
                }
            }
            let key = word.to_lowercase();
            if seen.insert(key) {
                out.push(word);
                if out.len() >= 2000 {
                    return out;
                }
            }
        }
    }
    out
}

/// Why the standard save gate would reject this candidate, or `None` if it
/// passes. Reuses the exact checks the typed importer uses — this only
/// *reports* the verdict for the review screen; the real gate still runs at
/// save time (`importer::save_words` path), so a candidate can never be saved
/// on the strength of this advisory alone.
pub fn gate_reason(word: &str) -> Option<GateFail> {
    if crate::profanity::is_blocked(word) {
        return Some(GateFail::Blocked);
    }
    if crate::importer::extract_words(word).is_empty() {
        return Some(GateFail::Charset);
    }
    None
}

/// Machine-readable reason a candidate failed the gate, mapped to a localized
/// label by the review UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFail {
    /// Blocked by the profanity screen.
    Blocked,
    /// No usable letters for the current charset (empty after extraction).
    Charset,
}

impl GateFail {
    /// i18n key for the short reason shown on a flagged chip.
    pub fn i18n_key(self) -> &'static str {
        match self {
            GateFail::Blocked => "photo.flag.blocked",
            GateFail::Charset => "photo.flag.charset",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(lines: &[&str]) -> Vec<String> {
        parse_candidates(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn strips_numbering_bullets_and_trailing_punct() {
        assert_eq!(parse(&["1. apple"]), vec!["apple"]);
        assert_eq!(parse(&["2) banana"]), vec!["banana"]);
        assert_eq!(parse(&["• cherry,"]), vec!["cherry"]);
        assert_eq!(parse(&["- pear."]), vec!["pear"]);
        assert_eq!(parse(&["\"quote\""]), vec!["quote"]);
        // Numbering fused to the word with no space.
        assert_eq!(parse(&["3.grape"]), vec!["grape"]);
    }

    #[test]
    fn keeps_internal_apostrophes_and_hyphens() {
        assert_eq!(parse(&["don't"]), vec!["don't"]);
        assert_eq!(parse(&["co-op"]), vec!["co-op"]);
    }

    #[test]
    fn drops_pure_digits_and_digit_bearing_tokens() {
        assert_eq!(parse(&["12"]), Vec::<String>::new());
        assert_eq!(parse(&["H2O"]), Vec::<String>::new());
        assert_eq!(parse(&["3", "words", "42"]), vec!["words"]);
    }

    #[test]
    fn one_letter_words_kept_only_for_allowed_set() {
        // Allowed es/en one-letter words survive.
        assert_eq!(parse(&["a y o e u"]), vec!["a", "y", "o", "e", "u"]);
        // Case-insensitive on the allow-list.
        assert_eq!(parse(&["A"]), vec!["A"]);
        // Every other lone letter is dropped as noise.
        assert_eq!(parse(&["b c x l I"]), Vec::<String>::new());
    }

    #[test]
    fn multiple_words_per_line_and_dedup() {
        assert_eq!(parse(&["cat dog cat"]), vec!["cat", "dog"]);
        // Dedup is case-insensitive; first casing wins.
        assert_eq!(parse(&["Cat", "cat", "CAT"]), vec!["Cat"]);
    }

    #[test]
    fn nfc_normalizes_accented_words() {
        // Decomposed "café" (e + combining acute) folds to the composed form.
        let decomposed = "cafe\u{0301}";
        let got = parse(&[decomposed]);
        assert_eq!(got, vec!["café"]);
        assert_eq!(got[0].chars().count(), 4); // composed é, not e + mark
    }

    #[test]
    fn gate_flags_profanity_and_passes_clean_words() {
        assert_eq!(gate_reason("apple"), None);
        assert_eq!(gate_reason("fuck"), Some(GateFail::Blocked));
        // A non-letter blob (shouldn't survive parse, but gate is defensive).
        assert_eq!(gate_reason("---"), Some(GateFail::Charset));
    }
}

// ---- Spell It Out Loud: on-device LETTER capture (second recognition profile) ----
//
// Same mic, same on-device SFSpeechRecognizer as Say-It — a DIFFERENT profile:
// `contextualStrings` biased to the language's letter names, and the RAW token
// stream (partials included) delivered live via callbacks instead of a single
// final promise. The Swift plugin does ZERO parsing; the Rust `spell_aloud`
// parser owns all linguistic knowledge.

thread_local! {
    // Keep the JS-facing closures alive for the duration of a capture session.
    // Replaced on each start; dropped on stop, ending the subscriptions.
    static LETTER_CBS: RefCell<Option<[Closure<dyn FnMut(JsValue)>; 3]>> = const { RefCell::new(None) };
}

/// Start on-device letter capture for `lang`, biasing the recognizer with
/// `contextual` (the spoken letter-name forms). `on_token` fires for each partial
/// (the growing raw transcript), `on_final` once at the end, `on_error` with a
/// documented code. Returns false (and does not call any callback) only when the
/// bridge/plugin method is entirely absent — caller treats that as unavailable.
pub fn start_letter_capture(
    lang: &str,
    contextual: &[String],
    mut on_token: impl FnMut(String) + 'static,
    mut on_final: impl FnMut(String) + 'static,
    mut on_error: impl FnMut(String) + 'static,
) -> bool {
    let Some(obj) = bridge() else { return false };
    let Some(f) = method(&obj, "startLetterCapture") else { return false };

    let opts = js_sys::Object::new();
    if Reflect::set(&opts, &JsValue::from_str("lang"), &JsValue::from_str(lang)).is_err() {
        return false;
    }
    let arr = js_sys::Array::new();
    for s in contextual {
        arr.push(&JsValue::from_str(s));
    }
    let _ = Reflect::set(&opts, &JsValue::from_str("contextualStrings"), &arr);

    let tok_cb = Closure::wrap(Box::new(move |v: JsValue| {
        on_token(v.as_string().unwrap_or_default());
    }) as Box<dyn FnMut(JsValue)>);
    let fin_cb = Closure::wrap(Box::new(move |v: JsValue| {
        on_final(v.as_string().unwrap_or_default());
    }) as Box<dyn FnMut(JsValue)>);
    let err_cb = Closure::wrap(Box::new(move |v: JsValue| {
        on_error(v.as_string().unwrap_or_else(|| "AUDIO_ERROR".into()));
    }) as Box<dyn FnMut(JsValue)>);

    // startLetterCapture(opts, onToken, onFinal, onError)
    let args = js_sys::Array::of4(
        &opts,
        tok_cb.as_ref().unchecked_ref(),
        fin_cb.as_ref().unchecked_ref(),
        err_cb.as_ref().unchecked_ref(),
    );
    if f.apply(&obj, &args).is_err() {
        return false;
    }
    LETTER_CBS.with(|c| *c.borrow_mut() = Some([tok_cb, fin_cb, err_cb]));
    true
}

/// Stop any in-flight letter capture (fire-and-forget). The plugin finalizes and
/// fires `on_final`; the kept-alive closures are released afterwards.
pub fn stop_letter_capture() {
    if let Some(obj) = bridge() {
        if let Some(f) = method(&obj, "stopLetterCapture") {
            let _ = f.call0(&obj);
        }
    }
}
