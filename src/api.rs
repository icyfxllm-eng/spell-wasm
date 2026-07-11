//! Talks to the word-server backend: `/api/speak?word=` for pre-rendered
//! TTS audio on built-in English words, `/api/check` to double-check a
//! typed answer server-side. The word itself is still picked and known
//! client-side (see `words::EN_*`) — this backend doesn't hide it.

use std::cell::RefCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::HtmlAudioElement;

use crate::{audio_boost, dom, native_audio, storage};

thread_local! {
    static CURRENT: RefCell<Option<(String, String, String, HtmlAudioElement)>> = RefCell::new(None);
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

fn speak_url(word: &str, variant: &str, lang: &str) -> String {
    format!("{}/api/speak?word={}&variant={}&lang={}", api_base(), urlencode(word), variant, lang)
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
    if native_audio::available() {
        let asset_id = native_audio::asset_id(word, variant, lang);
        let url = speak_url(word, variant, lang);
        if let Some(promise) = native_audio::play_word(&asset_id, &url) {
            let word = word.to_string();
            let variant = variant.to_string();
            let lang = lang.to_string();
            spawn_local(async move {
                // On reject (download failed, plugin error, …) fall through to
                // the browser <audio> path, which owns the final on_fail hop.
                if JsFuture::from(promise).await.is_err() {
                    play_word_html(&word, &variant, rate, &lang, on_fail);
                }
            });
            return;
        }
    }
    play_word_html(word, variant, rate, lang, on_fail);
}

fn play_word_html(word: &str, variant: &str, rate: f64, lang: &str, on_fail: impl FnOnce() + 'static) {
    let already_current =
        CURRENT.with(|c| c.borrow().as_ref().map(|(w, v, l, _)| w == word && v == variant && l == lang).unwrap_or(false));
    if already_current {
        CURRENT.with(|c| {
            if let Some((_, _, _, audio)) = c.borrow().as_ref() {
                audio.set_playback_rate(rate);
                audio.set_current_time(0.0);
                let _ = audio.play();
            }
        });
        return;
    }

    let url = speak_url(word, variant, lang);
    let Ok(audio) = HtmlAudioElement::new_with_src(&url) else {
        on_fail();
        return;
    };
    audio.set_playback_rate(rate);
    // crossOrigin is only needed to let the Web Audio gain graph
    // (audio_boost::wire) use this element's audio without tainting it —
    // and only when a boost is actually requested does that graph get
    // used at all (see audio_boost::wire). Setting it unconditionally
    // makes some browsers enforce a real CORS check even for same-origin
    // URLs, for no benefit at default settings — so it's set only when
    // it'll actually matter.
    if audio_boost::boost_requested() {
        audio.set_cross_origin(Some("anonymous"));
    }
    audio_boost::wire(&audio);

    let err_cb = Closure::once(move || {
        dom::set_text("voiceNote", &crate::i18n::t("voice.audioFail"));
        on_fail();
    });
    audio.set_onerror(Some(err_cb.as_ref().unchecked_ref()));
    err_cb.forget();

    let _ = audio.play();
    CURRENT.with(|c| *c.borrow_mut() = Some((word.to_string(), variant.to_string(), lang.to_string(), audio)));
}

/// Fire-and-forget warm-up of a word's normal-variant audio in the browser's
/// HTTP cache (the backend sends a long-lived `Cache-Control` header), so
/// that when the player actually reaches this word moments later,
/// `play_word` resolves instantly instead of waiting on a fresh TTS fetch.
pub fn preload_word(word: &str, lang: &str) {
    let url = speak_url(word, "normal", lang);
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
pub async fn fetch_meaning(word: &str, mask: bool) -> Result<(String, String, String), JsValue> {
    let url = format!("{}/api/meaning?word={}&mask={}", api_base(), urlencode(word), if mask { "1" } else { "0" });
    let text = storage::fetch_text(&url).await?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let get = |k: &str| json.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok((get("pos"), get("definition"), get("example")))
}

/// Plays the word's real (unmasked) example sentence via
/// `/api/sentence-audio` — audio doesn't reveal spelling the way on-screen
/// text would, so unlike the displayed sentence this is never masked.
pub fn play_sentence_audio(word: &str) {
    let url = format!("{}/api/sentence-audio?word={}", api_base(), urlencode(word));
    let Ok(audio) = HtmlAudioElement::new_with_src(&url) else { return };
    audio.set_cross_origin(Some("anonymous"));
    audio_boost::wire(&audio);
    let _ = audio.play();
}
