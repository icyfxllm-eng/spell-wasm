//! CC-GAME-SERVER F2/F5/F13 — the online client's network: the anonymous
//! session token, the room socket, resume, and reconnect.
//!
//! Nothing here runs until the player opens the online screen: with the build
//! flag off the screen cannot open, so no request ever reaches the room server
//! (A13.4). The token lives in the iOS Keychain through the app's own native
//! plugin (G-D11); on the web test build it lives in memory only. It is never
//! written to localStorage and never appears in a URL.

use std::cell::{Cell, RefCell};

use js_sys::{Function, Promise, Reflect};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{CloseEvent, Headers, MessageEvent, Request, RequestInit, RequestMode, Response, WebSocket};

use crate::online_model::PROTOCOL;

/// Server close codes the client acts on (server/spell-server/src/protocol.rs).
pub mod close {
    pub const UPDATE_REQUIRED: u16 = 4001;
    pub const SESSION: u16 = 4009;
    pub const REVOKED: u16 = 4010;
    pub const REPLACED: u16 = 4011;
    pub const NAME: u16 = 4014;
}

/// F13: with no room, two failed opens show the resting card.
pub const UNREACHABLE_AFTER: u32 = 2;
/// F5: a room seat is held 60 s; the client keeps trying that long.
pub const RECONNECT_WINDOW_MS: f64 = 60_000.0;

const TOKEN_KEY: &str = "online_token";

/// What the socket tells the screen.
pub enum Event {
    Frame(String),
    /// Opened and `hello` sent.
    Open,
    /// The socket closed. `code` is the WebSocket close code (1006 for a drop).
    Closed(u16),
    /// The session could not be minted or the server is unreachable.
    Unreachable,
}

thread_local! {
    static WS: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
    static MEM_TOKEN: RefCell<Option<String>> = const { RefCell::new(None) };
    static GEN: Cell<u64> = const { Cell::new(0) };
    static SINK: RefCell<Option<Box<dyn Fn(Event)>>> = const { RefCell::new(None) };
}

/// The room server's base URL (`window.SPELL_PLAY_BASE`).
pub fn play_base() -> String {
    web_sys::window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("SPELL_PLAY_BASE")).ok())
        .and_then(|v| v.as_string())
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://play.spellgame.net".into())
}

fn ws_url() -> String {
    let base = play_base();
    let ws = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base
    };
    format!("{ws}/v1/ws")
}

/// The app's own native plugin (iOS), when present.
fn kit() -> Option<JsValue> {
    let w = web_sys::window()?;
    let cap = Reflect::get(&w, &JsValue::from_str("Capacitor")).ok()?;
    let plugins = Reflect::get(&cap, &JsValue::from_str("Plugins")).ok()?;
    let kit = Reflect::get(&plugins, &JsValue::from_str("NativeLanguageKit")).ok()?;
    let has = Reflect::get(&kit, &JsValue::from_str("keychainGet")).ok()?;
    has.is_function().then_some(kit)
}

pub fn platform() -> &'static str {
    if kit().is_some() {
        "ios"
    } else {
        "web"
    }
}

async fn kit_call(method: &str, args: Value) -> Result<JsValue, JsValue> {
    let kit = kit().ok_or_else(|| JsValue::from_str("no native"))?;
    let f: Function = Reflect::get(&kit, &JsValue::from_str(method))?.dyn_into()?;
    let arg = js_sys::JSON::parse(&args.to_string())?;
    let p: Promise = f.call1(&kit, &arg)?.dyn_into()?;
    JsFuture::from(p).await
}

/// The stored token, if any.
pub async fn token() -> Option<String> {
    if kit().is_some() {
        let v = kit_call("keychainGet", json!({ "key": TOKEN_KEY })).await.ok()?;
        return Reflect::get(&v, &JsValue::from_str("value")).ok()?.as_string().filter(|s| !s.is_empty());
    }
    MEM_TOKEN.with(|t| t.borrow().clone())
}

async fn store_token(tok: &str) -> bool {
    if kit().is_some() {
        return kit_call("keychainSet", json!({ "key": TOKEN_KEY, "value": tok })).await.is_ok();
    }
    MEM_TOKEN.with(|t| *t.borrow_mut() = Some(tok.to_string()));
    true
}

pub async fn forget_token() {
    if kit().is_some() {
        let _ = kit_call("keychainDelete", json!({ "key": TOKEN_KEY })).await;
    }
    MEM_TOKEN.with(|t| *t.borrow_mut() = None);
}

/// `POST /v1/session`: a fresh anonymous identity.
async fn mint() -> Option<String> {
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_mode(RequestMode::Cors);
    let req = Request::new_with_str_and_init(&format!("{}/v1/session", play_base()), &init).ok()?;
    let w = web_sys::window()?;
    let resp: Response = JsFuture::from(w.fetch_with_request(&req)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text = JsFuture::from(resp.text().ok()?).await.ok()?.as_string()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let tok = v.get("token")?.as_str()?.to_string();
    store_token(&tok).await.then_some(tok)
}

/// The token for an authenticated request (ticket audio), minting if needed.
pub async fn ensure_token() -> Option<String> {
    match token().await {
        Some(t) => Some(t),
        None => mint().await,
    }
}

/// A random v4-shaped UUID for `cid`. It only has to be unique within a match.
pub fn cid() -> String {
    let mut b = [0u8; 16];
    for x in b.iter_mut() {
        *x = (js_sys::Math::random() * 256.0) as u8;
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Where socket events go (the online screen).
pub fn set_sink(f: impl Fn(Event) + 'static) {
    SINK.with(|s| *s.borrow_mut() = Some(Box::new(f)));
}

fn emit(e: Event) {
    SINK.with(|s| {
        if let Some(f) = s.borrow().as_ref() {
            f(e);
        }
    });
}

/// What `hello` says about this player.
#[derive(Clone)]
pub struct Hello {
    pub kid: bool,
    pub ui_lang: String,
    pub name: String,
    pub resume: Option<(String, u64)>,
}

/// Opens (or reopens) the socket and sends `hello`. A newer call supersedes
/// an older one still in flight.
pub fn open(h: Hello) {
    let gen = GEN.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    shut();
    spawn_local(async move {
        let Some(tok) = ensure_token().await else { return emit(Event::Unreachable) };
        if GEN.with(Cell::get) != gen {
            return;
        }
        let Ok(ws) = WebSocket::new(&ws_url()) else { return emit(Event::Unreachable) };
        let hello = hello_frame(&h, &tok);
        {
            let w2 = ws.clone();
            let on_open = Closure::<dyn FnMut()>::new(move || {
                let _ = w2.send_with_str(&hello);
                emit(Event::Open);
            });
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            on_open.forget();
        }
        {
            let on_msg = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                if GEN.with(Cell::get) != gen {
                    return;
                }
                if let Some(t) = e.data().as_string() {
                    emit(Event::Frame(t));
                }
            });
            ws.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            on_msg.forget();
        }
        {
            let on_close = Closure::<dyn FnMut(CloseEvent)>::new(move |e: CloseEvent| {
                if GEN.with(Cell::get) != gen {
                    return;
                }
                WS.with(|w| *w.borrow_mut() = None);
                emit(Event::Closed(e.code()));
            });
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            on_close.forget();
        }
        WS.with(|w| *w.borrow_mut() = Some(ws));
    });
}

fn hello_frame(h: &Hello, tok: &str) -> String {
    let build = web_sys::window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("SPELL_BUILD")).ok())
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "DEV".into());
    let mut body = json!({
        "protocol": PROTOCOL,
        "app_build": build,
        "platform": platform(),
        "profile_kind": if h.kid { "kid" } else { "adult" },
        "ui_lang": h.ui_lang,
        "token": tok,
    });
    // A10.3: a kid sends no name at all; the server makes one.
    if !h.kid {
        body["display_name"] = json!(h.name);
    }
    if let Some((room, last_seq)) = &h.resume {
        body["resume"] = json!({ "room": room, "last_seq": last_seq });
    }
    crate::online_model::frame("hello", &cid(), body)
}

/// Sends one client message; false if the socket is not open.
pub fn send(t: &str, body: Value) -> bool {
    WS.with(|w| match w.borrow().as_ref() {
        Some(ws) if ws.ready_state() == WebSocket::OPEN => {
            ws.send_with_str(&crate::online_model::frame(t, &cid(), body)).is_ok()
        }
        _ => false,
    })
}

pub fn is_open() -> bool {
    WS.with(|w| w.borrow().as_ref().is_some_and(|ws| ws.ready_state() == WebSocket::OPEN))
}

/// Closes the socket without reporting it (leaving the screen, or reopening).
pub fn shut() {
    WS.with(|w| {
        if let Some(ws) = w.borrow_mut().take() {
            ws.set_onclose(None);
            ws.set_onmessage(None);
            let _ = ws.close();
        }
    });
}

/// Ends every pending open (leaving the screen).
pub fn stop() {
    GEN.with(|g| g.set(g.get() + 1));
    shut();
}

/// `GET /v1/audio/{ticket}` as bytes for the web path (F9): the request
/// carries `Authorization: Bearer`, which an audio element cannot.
pub async fn fetch_ticket(ticket: &str) -> Option<web_sys::Blob> {
    let tok = ensure_token().await?;
    let init = RequestInit::new();
    init.set_mode(RequestMode::Cors);
    let headers = Headers::new().ok()?;
    headers.set("Authorization", &format!("Bearer {tok}")).ok()?;
    init.set_headers(&headers);
    let req = Request::new_with_str_and_init(&ticket_url(ticket), &init).ok()?;
    let w = web_sys::window()?;
    let resp: Response = JsFuture::from(w.fetch_with_request(&req)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    JsFuture::from(resp.blob().ok()?).await.ok()?.dyn_into().ok()
}

pub fn ticket_url(ticket: &str) -> String {
    format!("{}/v1/audio/{ticket}", play_base())
}

#[cfg(test)]
mod tests {
    // The network is exercised end to end by tests/e2e/specs/online.mjs
    // against a real spell-server; the pure parts are in online_model.
}
