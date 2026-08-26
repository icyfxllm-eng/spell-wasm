//! Talks to the word-server backend: `/api/speak?word=` for pre-rendered
//! TTS audio on built-in English words, `/api/check` to double-check a
//! typed answer server-side. The word itself is still picked and known
//! client-side (see `words::EN_*`) — this backend doesn't hide it.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::HtmlAudioElement;

use crate::{audio_boost, dom, native_audio, native_lang, storage};

thread_local! {
    static CURRENT: RefCell<Option<(String, String, String, HtmlAudioElement)>> = RefCell::new(None);
    // The source that produced the last word audio ("server-cache" | "native-tts"
    // | "none"). Surfaced for QA and the whisper.cpp loopback harness.
    static LAST_SOURCE: RefCell<&'static str> = const { RefCell::new("") };
}

/// Which audio source last produced (or failed to produce) word audio.
pub fn last_audio_source() -> &'static str {
    LAST_SOURCE.with(|c| *c.borrow())
}
/// Records which source produced the audio — and owns the failure banner.
///
/// The banner used to be painted by the `<audio>` element's own onerror, which
/// made it a lie: the order is Pack -> ServerCache -> NativeTts, so a
/// ServerCache miss announced "couldn't reach the audio server" and NativeTts
/// then played the word perfectly. True about one source, false about the
/// outcome — and nothing cleared it, so it sat there for the rest of the word.
///
/// Only the router knows the outcome, so only the router may speak. "none" is
/// the wall: every source has been tried and none of them played.
fn set_source(s: &'static str) {
    LAST_SOURCE.with(|c| *c.borrow_mut() = s);
    let banner = crate::i18n::t("voice.audioFail");
    if s == "none" {
        dom::set_text("voiceNote", &banner);
    } else if dom::text("voiceNote") == banner {
        // Recovered on a later source. Clear only OUR message: `voiceNote` is
        // shared with the missing-browser-voice notice (game::update_voice_note),
        // which is still true and not ours to erase.
        dom::set_text("voiceNote", "");
    }
}

/// The ordered audio sources the router tries. "server-cache" = the pre-rendered
/// `/api/speak` mp3 (played natively or via `<audio>`); "native-tts" = on-device
/// AVSpeech (offline). One routing decision, in ONE place (doctrine).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Source {
    /// CC-OFFLINE-PACKS (BD-2): the verified-active language pack. Heads
    /// the ONE resolution order; a no-op instantly wherever no pack (or
    /// no native layer) exists, so web/streaming behavior is unchanged.
    Pack,
    ServerCache,
    NativeTts,
}

/// Pure config → source-order mapping (unit-tested). DEFAULT is server-primary /
/// native-fallback (Decision D1) — current behavior preserved, native TTS
/// rescues offline. Override values: "native-first" | "server-only" |
/// "native-only".
fn parse_source_order(cfg: Option<&str>) -> Vec<Source> {
    match cfg {
        Some("native-first") => vec![Source::Pack, Source::NativeTts, Source::ServerCache],
        Some("server-only") => vec![Source::ServerCache],
        Some("native-only") => vec![Source::NativeTts],
        _ => vec![Source::Pack, Source::ServerCache, Source::NativeTts],
    }
}

fn source_order() -> Vec<Source> {
    parse_source_order(storage::get_raw("spell_audio_src").as_deref())
}

#[cfg(test)]
mod router_tests {
    use super::*;

    #[test]
    fn default_is_server_primary_native_fallback() {
        // BD-2 I3: the pack heads the ONE order; server clip next, native
        // TTS as rescue. Pack no-ops instantly where no pack exists, so
        // D1's server-primary behavior is preserved on the web.
        assert_eq!(parse_source_order(None), vec![Source::Pack, Source::ServerCache, Source::NativeTts]);
        assert_eq!(parse_source_order(Some("garbage")), vec![Source::Pack, Source::ServerCache, Source::NativeTts]);
    }

    #[test]
    fn native_first_flips_the_order() {
        assert_eq!(parse_source_order(Some("native-first")), vec![Source::Pack, Source::NativeTts, Source::ServerCache]);
    }

    #[test]
    fn single_source_configs() {
        assert_eq!(parse_source_order(Some("server-only")), vec![Source::ServerCache]);
        assert_eq!(parse_source_order(Some("native-only")), vec![Source::NativeTts]);
    }
}

/// Stop any backend word/sentence audio that's currently playing (used when
/// tearing a mode down, e.g. leaving head-to-head, so nothing keeps playing).
pub fn stop() {
    CURRENT.with(|c| {
        if let Some((_, _, _, audio)) = c.borrow_mut().take() {
            let _ = audio.pause();
        }
    });
}

/// Reads the backend's base URL from `window.SPELL_API_BASE`, set in
/// `index.html`. Deploying to a new backend (e.g. a Replit URL) is then a
/// one-line HTML edit — no Rust rebuild needed. Falls back to local dev
/// defaults if it's missing or blank.
pub fn api_base() -> String {
    if let Some(win) = web_sys::window() {
        if let Ok(v) = js_sys::Reflect::get(&win, &JsValue::from_str("SPELL_API_BASE")) {
            if let Some(s) = v.as_string() {
                let s = s.trim().to_string();
                if !s.is_empty() {
                    return s.trim_end_matches('/').to_string();
                }
            }
        }
    }
    "http://127.0.0.1:5000".to_string()
}

fn urlencode(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_else(|| s.to_string())
}

fn speak_url(word: &str, py: Option<&str>, variant: &str, lang: &str) -> String {
    // CC-ZH-TONE F6: `py` carries the forced pinyin reading. It rides in its
    // own parameter because `word` is validated as letters-and-marks only —
    // tone digits and the spaces between syllables would be rejected outright.
    let reading = py.map(|p| format!("&py={}", urlencode(p))).unwrap_or_default();
    format!(
        "{}/api/speak?word={}&variant={}&lang={}{}",
        api_base(),
        urlencode(word),
        variant,
        lang,
        reading
    )
}

/// Plays `word`'s audio (rewinding + reusing the element if it's already the
/// current word+variant, otherwise fetching fresh from `/api/speak`).
/// `variant` is `"normal"` (spoken twice with a pause) or `"slow"` (a single,
/// more slowly synthesized utterance) — both are real backend-rendered
/// clips, not a client-side speed trick. `rate` further adjusts playback
/// speed on top (used for the general "slower voice" setting). If the
/// backend can't produce audio for this word (network error, word
/// rejected, etc.), `on_fail` runs once so the caller can fall back to
/// another voice.
/// Plays `word`'s audio, preferring the native-audio path on the Capacitor
/// build (downloads the clip once, then plays it through the OS audio path —
/// no autoplay gate, works offline). Falls back to the browser `<audio>`
/// path below, which in turn calls `on_fail` (e.g. Web Speech) if even that
/// can't reach the audio server.
///
/// NativeAudio can't vary playback speed, so the native path ignores `rate`
/// (the mild 0.9/0.7 "voice speed" comfort tweak) and plays the clip at its
/// natural speed. The genuinely-slow need is met by the separate
/// server-rendered "slow" *variant*, which is a different clip and plays
/// natively just fine. `rate` still applies on the browser `<audio>` fallback
/// (and on the web build, which never takes the native path).
pub fn play_word(word: &str, variant: &str, rate: f64, lang: &str, on_fail: impl FnOnce() + 'static) {
    play_word_with(word, None, variant, rate, lang, on_fail);
}

/// CC-ZH-TONE F6 — as `play_word`, plus the forced reading.
///
/// Mandarin REQUIRES `py`: the server refuses a zh clip without one, because a
/// guessed polyphone is the failure this whole feature exists to remove.
///
/// zh also drops the on-device rescue from its source chain. Native TTS cannot
/// be handed a reading, so it would speak bare Hanzi and guess — exactly what
/// Invariant 4 forbids. The consequence is deliberate and worth stating: with
/// no pack and no server, Mandarin has NO audio rather than wrong audio.
pub fn play_word_with(
    word: &str,
    py: Option<&str>,
    variant: &str,
    rate: f64,
    lang: &str,
    on_fail: impl FnOnce() + 'static,
) {
    let mut order = source_order();
    if lang == crate::consts::ZH {
        order.retain(|s| *s != Source::NativeTts);
    }
    play_chain(
        order,
        0,
        word.to_string(),
        py.map(str::to_string),
        variant.to_string(),
        rate,
        lang.to_string(),
        Box::new(on_fail),
    );
}

/// Try source `order[i]`; each source's failure advances to the next, and the
/// last source's failure runs the caller's `on_fail`. Wall of the whole router.
#[allow(clippy::too_many_arguments)]
fn play_chain(
    order: Vec<Source>,
    i: usize,
    word: String,
    py: Option<String>,
    variant: String,
    rate: f64,
    lang: String,
    on_fail: Box<dyn FnOnce()>,
) {
    let src = match order.get(i) {
        Some(&s) => s,
        None => {
            set_source("none");
            on_fail();
            return;
        }
    };
    let (w, v, l, pyc) = (word.clone(), variant.clone(), lang.clone(), py.clone());
    let next: Box<dyn FnOnce()> =
        Box::new(move || play_chain(order, i + 1, w, pyc, v, rate, l, on_fail));
    match src {
        Source::Pack => play_pack(&word, &variant, rate, &lang, next),
        Source::ServerCache => play_server_cache(&word, py.as_deref(), &variant, rate, &lang, next),
        Source::NativeTts => play_native_tts(&word, &variant, rate, &lang, next),
    }
}

/// Source "server-cache": the pre-rendered `/api/speak` clip, played through the
/// native NativeAudio plugin when present, else the browser `<audio>` element.
/// Both mechanisms serve the same server-rendered source; `on_fail` advances the
/// router to the next source.
/// Source "pack": the verified-active offline pack (BD-2). With an
/// active pack this is the ONLY audio source that fires — zero network
/// even online (I1's twin: packs also cut server load by design).
fn play_pack(word: &str, variant: &str, rate: f64, lang: &str, on_fail: Box<dyn FnOnce()>) {
    let (word, variant, lang) = (word.to_string(), variant.to_string(), lang.to_string());
    spawn_local(async move {
        match crate::packs::src_for(&lang, &word, &variant).await {
            Some(src) => {
                if let Ok(audio) = HtmlAudioElement::new_with_src(&src) {
                    audio.set_playback_rate(rate);
                    if audio.play().is_ok() {
                        set_source("pack");
                        return;
                    }
                }
                on_fail();
            }
            None => on_fail(),
        }
    });
}

fn play_server_cache(word: &str, py: Option<&str>, variant: &str, rate: f64, lang: &str, on_fail: Box<dyn FnOnce()>) {
    if native_audio::available() {
        let asset_id = native_audio::asset_id(word, variant, lang);
        let url = speak_url(word, py, variant, lang);
        if let Some(promise) = native_audio::play_word(&asset_id, &url) {
            let word = word.to_string();
            let variant = variant.to_string();
            let lang = lang.to_string();
            let py = py.map(str::to_string);
            spawn_local(async move {
                if JsFuture::from(promise).await.is_err() {
                    // Native download/playback failed → try the <audio> mechanism
                    // for the same server clip; it owns the hop to `on_fail`.
                    play_word_html(&word, py.as_deref(), &variant, rate, &lang, on_fail);
                } else {
                    set_source("server-cache");
                }
            });
            return;
        }
    }
    play_word_html(word, py, variant, rate, lang, on_fail);
}

/// CC-PHOTO-IMPORT Phase 5 (gate G-A): speak a word ON-DEVICE ONLY — native
/// AVSpeech, no network, the word text never leaves the phone. The public
/// entry for custom-marked (out-of-dictionary) imports; `on_fail` runs when
/// no native voice exists (off-iOS, or no voice for `lang`) so the caller can
/// fall back to the browser voice — which is also on-device.
pub fn play_device_tts(word: &str, variant: &str, rate: f64, lang: &str, on_fail: impl FnOnce() + 'static) {
    play_native_tts(word, variant, rate, lang, Box::new(on_fail));
}

/// Source "native-tts": fully on-device AVSpeech synthesis (no network). Picks
/// the session voice for `lang` (Decision D3), then speaks. The server "slow"
/// variant has no native pre-render, so slowness is met by a lower rate here.
fn play_native_tts(word: &str, variant: &str, rate: f64, lang: &str, on_fail: Box<dyn FnOnce()>) {
    if !native_lang::available() {
        on_fail();
        return;
    }
    let eff_rate = if variant == "slow" { rate.min(0.7) } else { rate };
    let word = word.to_string();
    let lang = lang.to_string();
    spawn_local(async move {
        // BD-4: a fully-earned family voice (picked + approved + readout
        // gate passed + language-honest) outranks the catalog pick; anything
        // less falls through to the standard voice — no badge, no tease.
        let voice = match crate::family_voices::orb_voice(&lang) {
            Some(v) => v,
            None => match native_lang::session_voice(&lang).await {
                Some(v) => v,
                None => {
                    on_fail();
                    return;
                }
            },
        };
        match native_lang::speak(&word, &voice, eff_rate) {
            Some(promise) => {
                if JsFuture::from(promise).await.is_ok() {
                    set_source("native-tts");
                } else {
                    on_fail();
                }
            }
            None => on_fail(),
        }
    });
}

fn play_word_html(word: &str, py: Option<&str>, variant: &str, rate: f64, lang: &str, on_fail: impl FnOnce() + 'static) {
    // Reuse the cached element only if it is still ALIVE. An HTMLMediaElement
    // latches its failure: once `error` is set, play() on that element can only
    // reject, forever. The old code compared the key alone, so one failed clip
    // meant every "hear it again" tap on that word re-failed against the same
    // corpse and never re-entered the router.
    let already_current = CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .map(|(w, v, l, a)| w == word && v == variant && l == lang && a.error().is_none())
            .unwrap_or(false)
    });

    let audio = if already_current {
        let a = CURRENT.with(|c| c.borrow().as_ref().map(|(_, _, _, a)| a.clone()));
        let Some(a) = a else { on_fail(); return };
        a.set_current_time(0.0);
        a
    } else {
        // Drop a dead element rather than replaying it.
        CURRENT.with(|c| {
            c.borrow_mut().take();
        });
        let url = speak_url(word, py, variant, lang);
        let Ok(a) = HtmlAudioElement::new_with_src(&url) else {
            on_fail();
            return;
        };
        // crossOrigin is only needed to let the Web Audio gain graph
        // (audio_boost::wire) use this element's audio without tainting it —
        // and only when a boost is actually requested does that graph get
        // used at all (see audio_boost::wire). Setting it unconditionally
        // makes some browsers enforce a real CORS check even for same-origin
        // URLs, for no benefit at default settings — so it's set only when
        // it'll actually matter.
        if audio_boost::boost_requested() {
            a.set_cross_origin(Some("anonymous"));
        }
        // Wired once per element, never on the reuse path — a second source
        // node on the same element would be a duplicate gain graph.
        audio_boost::wire(&a);
        a
    };
    audio.set_playback_rate(rate);

    // Two routes lead to failure — the element's `error` event and play()'s
    // rejected promise — and whichever arrives first must be the only one to
    // advance the router. `error` can also fire more than once, so the old
    // `Closure::once` here was a live abort: the second fire would invoke an
    // already-consumed FnOnce.
    let once: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(Some(Box::new(on_fail))));
    let advance = {
        let once = Rc::clone(&once);
        move || {
            if let Some(f) = once.borrow_mut().take() {
                f();
            }
        }
    };

    let on_err = advance.clone();
    let err_cb = Closure::wrap(Box::new(move || on_err()) as Box<dyn FnMut()>);
    audio.set_onerror(Some(err_cb.as_ref().unchecked_ref()));
    err_cb.forget();

    CURRENT
        .with(|c| *c.borrow_mut() = Some((word.to_string(), variant.to_string(), lang.to_string(), audio.clone())));

    match audio.play() {
        Ok(p) => spawn_local(async move {
            match JsFuture::from(p).await {
                // Only now has audio actually started. Recording the source
                // before this point reported a success that had not happened.
                Ok(_) => set_source("server-cache"),
                Err(e) => {
                    // AbortError means a NEWER play on this same element
                    // superseded ours (tapping replay faster than the clip
                    // loads). The word is playing — advancing the router here
                    // would stack native TTS on top of it.
                    let name = js_sys::Reflect::get(&e, &JsValue::from_str("name"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    if name != "AbortError" {
                        advance();
                    }
                }
            }
        }),
        Err(_) => advance(),
    }
}

/// Fire-and-forget warm-up of a word's normal-variant audio in the browser's
/// HTTP cache (the backend sends a long-lived `Cache-Control` header), so
/// that when the player actually reaches this word moments later,
/// `play_word` resolves instantly instead of waiting on a fresh TTS fetch.
pub fn preload_word(word: &str, lang: &str) {
    preload_word_with(word, None, lang);
}

/// F6: zh must warm with its reading, or the warm URL is not the URL the real
/// play requests and the warm-up is wasted (worse, it would 400).
pub fn preload_word_with(word: &str, py: Option<&str>, lang: &str) {
    let url = speak_url(word, py, "normal", lang);
    // On the native build, warming means downloading the clip to on-device
    // storage (so it's instant AND offline later); the browser HTTP-cache
    // warm-up below is redundant there.
    if native_audio::available() {
        native_audio::prefetch(&native_audio::asset_id(word, "normal", lang), &url);
        return;
    }
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    if let Ok(req) = web_sys::Request::new_with_str_and_init(&url, &opts) {
        if let Some(win) = web_sys::window() {
            let _ = win.fetch_with_request(&req);
        }
    }
}

/// Double-checks a typed answer against the backend. Since the backend
/// trusts whatever `word` it's given, this is only ever as strong as the
/// client sending the real target word — callers should still be prepared
/// to fall back to a local comparison if the request fails.
pub async fn check_answer(word: &str, answer: &str) -> Result<bool, JsValue> {
    let body = serde_json::json!({ "word": word, "answer": answer }).to_string();
    let text = storage::fetch_post_json(&format!("{}/api/check", api_base()), &body).await?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| JsValue::from_str(&e.to_string()))?;
    json.get("correct").and_then(|v| v.as_bool()).ok_or_else(|| JsValue::from_str("malformed /api/check response"))
}

/// Definition + example sentence for `word` from our own backend's
/// `/api/meaning`, which proxies dictionaryapi.dev server-side. `mask=true`
/// blanks the target word/inflections in both fields (used for the
/// pre-answer Definition/Sentence hints); `mask=false` returns the real
/// text (used for the existing post-answer reveal, once the round is over).
/// Routing through our backend — rather than calling dictionaryapi.dev
/// straight from the browser — means the masked hint's network response
/// itself never contains the unmasked word.
/// Study languages with a real definition source: English via dictionaryapi.dev,
/// the rest via en.wiktionary (both proxied by OUR backend — the word never goes
/// third-party from the device). zh is absent: the wiktionary endpoint omits
/// Chinese sections (verified 2026-07-27), so it gets no button, not a wrong one.
pub fn meaning_supported(lang: &str) -> bool {
    let base = lang.split(['-', '_']).next().unwrap_or(lang);
    matches!(
        base,
        "en" | "es" | "fr" | "de" | "pt" | "pl" | "vi" | "ko" | "ja" | "ru" | "ar" | "hi" | "sw" | "fil"
            | "zh" | "cmn" // bundled CC-CEDICT glosses (backend/zh_glosses.json)
    )
}

/// Example sentences exist for every meaning-supported language EXCEPT Chinese
/// (CC-CEDICT is glosses-only) — the Sentence hint hides rather than always
/// answering "no example found".
pub fn sentence_supported(lang: &str) -> bool {
    let base = lang.split(['-', '_']).next().unwrap_or(lang);
    meaning_supported(lang) && !matches!(base, "zh" | "cmn")
}

pub async fn fetch_meaning(word: &str, mask: bool, lang: &str) -> Result<(String, String, String), JsValue> {
    let url = format!(
        "{}/api/meaning?word={}&mask={}&lang={}",
        api_base(),
        urlencode(word),
        if mask { "1" } else { "0" },
        urlencode(lang)
    );
    let text = storage::fetch_text(&url).await?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let get = |k: &str| json.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok((get("pos"), get("definition"), get("example")))
}

/// Plays the word's real (unmasked) example sentence via
/// `/api/sentence-audio` — audio doesn't reveal spelling the way on-screen
/// text would, so unlike the displayed sentence this is never masked.
pub fn play_sentence_audio(word: &str, lang: &str) {
    let url = format!("{}/api/sentence-audio?word={}&lang={}", api_base(), urlencode(word), urlencode(lang));
    let Ok(audio) = HtmlAudioElement::new_with_src(&url) else { return };
    audio.set_cross_origin(Some("anonymous"));
    audio_boost::wire(&audio);
    // Fire-and-forget: there is no fallback chain for sentence audio, so a
    // failure has nowhere to go. Awaited-and-dropped rather than discarded,
    // so a rejection dies here instead of surfacing as an unhandled rejection.
    if let Ok(p) = audio.play() {
        spawn_local(async move {
            let _ = JsFuture::from(p).await;
        });
    }
}
