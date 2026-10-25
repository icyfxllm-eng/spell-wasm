//! CC-GAME-SERVER F9 — the client half of ticket audio.
//!
//! The online word's clip is played ONLY from its ticket, never from the
//! word-keyed router in `api.rs` (Human / Pack / ServerCache / NativeTts): the
//! client does not know the word, and a fallback to any of those would need
//! it. When the ticket cannot be played, the player sees the explicit paused
//! state the server sends; there is no silent fallback.
//!
//! Native (iOS): `SpellAudio.playWord(assetId, url, {headers})`, with an asset
//! id made from the ticket, so the on-disk cache name carries no letter of the
//! word. Web (the Playwright build): fetch with the Bearer header, then play a
//! blob URL.

use std::cell::RefCell;

use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::HtmlAudioElement;

thread_local! {
    static CURRENT: RefCell<Option<(HtmlAudioElement, String)>> = const { RefCell::new(None) };
}

fn spell_audio() -> Option<JsValue> {
    let w = web_sys::window()?;
    let sa = Reflect::get(&w, &JsValue::from_str("SpellAudio")).ok()?;
    let has = Reflect::get(&sa, &JsValue::from_str("playWord")).ok()?;
    (has.is_function() && crate::native_audio::available()).then_some(sa)
}

/// Plays the clip a ticket names. `on_fail` runs if it cannot be played.
pub fn play(ticket: String, on_fail: impl FnOnce() + 'static) {
    stop();
    spawn_local(async move {
        let ok = match spell_audio() {
            Some(sa) => play_native(&sa, &ticket).await,
            None => play_web(&ticket).await,
        };
        if !ok {
            on_fail();
        }
    });
}

async fn play_native(sa: &JsValue, ticket: &str) -> bool {
    let Some(tok) = crate::online_net::ensure_token().await else { return false };
    let opts = Object::new();
    let headers = Object::new();
    let _ = Reflect::set(&headers, &JsValue::from_str("Authorization"), &JsValue::from_str(&format!("Bearer {tok}")));
    let _ = Reflect::set(&opts, &JsValue::from_str("headers"), &headers);
    let Ok(f) = Reflect::get(sa, &JsValue::from_str("playWord")).and_then(|f| f.dyn_into::<Function>()) else {
        return false;
    };
    let url = crate::online_net::ticket_url(ticket);
    let asset = format!("t:{ticket}");
    let Ok(p) = f.call3(sa, &JsValue::from_str(&asset), &JsValue::from_str(&url), &opts) else { return false };
    let Ok(p) = p.dyn_into::<Promise>() else { return false };
    JsFuture::from(p).await.is_ok()
}

async fn play_web(ticket: &str) -> bool {
    let Some(blob) = crate::online_net::fetch_ticket(ticket).await else { return false };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return false };
    let Ok(a) = HtmlAudioElement::new_with_src(&url) else { return false };
    CURRENT.with(|c| *c.borrow_mut() = Some((a.clone(), url)));
    match a.play() {
        Ok(p) => JsFuture::from(p).await.is_ok(),
        Err(_) => false,
    }
}

/// Stops the current clip and frees its blob URL.
pub fn stop() {
    CURRENT.with(|c| {
        if let Some((a, url)) = c.borrow_mut().take() {
            let _ = a.pause();
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    });
}
