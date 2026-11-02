//! CC-ACCOUNTS Phase 2 -- the account client's network and token store.
//!
//! Tokens live in the iOS Keychain through the app's own plugin and nowhere else (I6): never
//! localStorage, never Preferences, never a URL, never a log line. Where there is no native
//! plugin (the browser test build) they live in memory for the life of the page.
//!
//! Nothing here runs until the account sheet or Settings → Account asks. With the flags off no
//! request is ever made (I1, I3): a failed call here returns a `Problem` and touches nothing else.

use std::cell::RefCell;
use std::collections::HashMap;

use js_sys::Reflect;
use serde_json::{json, Value};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

use crate::accounts_model::{self as am, Problem, Session};
use crate::online_net::{kit, kit_call};

const ACCESS: &str = "acct_access_v1";
const REFRESH: &str = "acct_refresh_v1";
const DEVICE: &str = "acct_device_v1";

thread_local! {
    static MEM: RefCell<HashMap<&'static str, String>> = RefCell::new(HashMap::new());
}

async fn kc_get(key: &'static str) -> Option<String> {
    if kit().is_some() {
        let v = kit_call("keychainGet", json!({ "key": key })).await.ok()?;
        return Reflect::get(&v, &JsValue::from_str("value")).ok()?.as_string().filter(|s| !s.is_empty());
    }
    MEM.with(|m| m.borrow().get(key).cloned())
}

async fn kc_set(key: &'static str, value: &str) -> bool {
    if kit().is_some() {
        return kit_call("keychainSet", json!({ "key": key, "value": value })).await.is_ok();
    }
    MEM.with(|m| m.borrow_mut().insert(key, value.to_string()));
    true
}

async fn kc_del(key: &'static str) {
    if kit().is_some() {
        let _ = kit_call("keychainDelete", json!({ "key": key })).await;
    }
    MEM.with(|m| m.borrow_mut().remove(key));
}

fn now_secs() -> f64 {
    js_sys::Date::now() / 1000.0
}

/// POST JSON. `(0, "")` when the request never got an answer.
async fn post(path: &str, body: &Value, bearer: Option<&str>) -> (u16, String) {
    let url = format!("{}{}", crate::api::api_base(), path);
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let Ok(headers) = Headers::new() else { return (0, String::new()) };
    let _ = headers.set("Content-Type", "application/json");
    if let Some(t) = bearer {
        let _ = headers.set("Authorization", &format!("Bearer {t}"));
    }
    opts.set_headers(&headers);
    opts.set_body(&JsValue::from_str(&body.to_string()));
    let Ok(req) = Request::new_with_str_and_init(&url, &opts) else { return (0, String::new()) };
    let Some(win) = web_sys::window() else { return (0, String::new()) };
    let Ok(resp) = JsFuture::from(win.fetch_with_request(&req)).await else { return (0, String::new()) };
    let Ok(resp) = resp.dyn_into::<Response>() else { return (0, String::new()) };
    let status = resp.status();
    let text = match resp.text() {
        Ok(p) => JsFuture::from(p).await.ok().and_then(|v| v.as_string()).unwrap_or_default(),
        Err(_) => String::new(),
    };
    (status, text)
}

async fn device_id() -> String {
    if let Some(d) = kc_get(DEVICE).await {
        return d;
    }
    let id: String = (0..4).map(|_| format!("{:08x}", (js_sys::Math::random() * 4_294_967_295.0) as u32)).collect();
    kc_set(DEVICE, &id).await;
    id
}

async fn store(s: &Session) {
    kc_set(ACCESS, &s.access_token).await;
    kc_set(REFRESH, &s.refresh_token).await;
}

async fn clear() {
    kc_del(ACCESS).await;
    kc_del(REFRESH).await;
}

/// `POST /api/accounts/email/request`. The service answers the same for every address, so a
/// success here says only that the request was accepted.
pub async fn request_code(email: &str) -> Result<(), Problem> {
    let (status, _) = post("/api/accounts/email/request", &json!({ "email": email.trim() }), None).await;
    if status == 200 { Ok(()) } else { Err(am::problem_for_status(status)) }
}

/// Only ever called after the age screen said adult: the `adult` flag is the attestation, and the
/// server refuses to create a row without it (A12).
pub async fn verify_code(email: &str, code: &str) -> Result<Session, Problem> {
    let body = json!({ "email": email.trim(), "code": code, "adult": true, "device_id": device_id().await });
    finish(post("/api/accounts/email/verify", &body, None).await).await
}

pub async fn siwa_available() -> bool {
    if kit().is_none() {
        return false;
    }
    match kit_call("siwaAvailable", json!({})).await {
        Ok(v) => Reflect::get(&v, &JsValue::from_str("available")).ok().and_then(|b| b.as_bool()).unwrap_or(false),
        Err(_) => false,
    }
}

pub async fn sign_in_apple() -> Result<Session, Problem> {
    let v = kit_call("signInWithApple", json!({})).await.map_err(|_| Problem::Rejected)?;
    let get = |k: &str| Reflect::get(&v, &JsValue::from_str(k)).ok().and_then(|x| x.as_string());
    let (Some(token), Some(nonce)) = (get("identity_token"), get("nonce")) else { return Err(Problem::Rejected) };
    let body = json!({
        "identity_token": token, "nonce": nonce, "authorization_code": get("authorization_code"),
        "adult": true, "device_id": device_id().await,
    });
    finish(post("/api/accounts/siwa", &body, None).await).await
}

async fn finish((status, text): (u16, String)) -> Result<Session, Problem> {
    if status != 200 {
        return Err(am::problem_for_status(status));
    }
    let s = am::parse_session(&text).ok_or(Problem::Rejected)?;
    store(&s).await;
    Ok(s)
}

/// A usable access token, refreshed first when it is close to expiry. `None` when signed out, when
/// the refresh token is dead, or when the service cannot be reached; none of those is an error the
/// player needs to see (I3).
pub async fn access_token() -> Option<String> {
    let access = kc_get(ACCESS).await?;
    if !am::needs_refresh(&access, now_secs()) {
        return Some(access);
    }
    let refresh = kc_get(REFRESH).await?;
    let (status, text) = post("/api/accounts/refresh", &json!({ "refresh_token": refresh }), None).await;
    match status {
        200 => {
            let s = am::parse_session(&text)?;
            store(&s).await;
            Some(s.access_token)
        }
        // The service says this chain is over (expired, revoked, or replayed): forget it.
        401 => {
            clear().await;
            None
        }
        // Offline or unavailable: keep what we hold and try again later.
        _ => None,
    }
}

pub async fn signed_in() -> bool {
    access_token().await.is_some()
}

/// Signs out here and on the service. Local data stays where it is (F2).
pub async fn sign_out() {
    if let Some(a) = kc_get(ACCESS).await {
        let _ = post("/api/accounts/logout", &json!({}), Some(&a)).await;
    }
    clear().await;
}

/// Deletes the account on the service. Local tokens are dropped only when the service says it is
/// done, so a failed call leaves the player signed in and able to try again.
pub async fn delete_account() -> Result<(), Problem> {
    let a = access_token().await.ok_or(Problem::Rejected)?;
    let (status, _) = post("/api/accounts/delete", &json!({}), Some(&a)).await;
    if status == 200 {
        clear().await;
        Ok(())
    } else {
        Err(am::problem_for_status(status))
    }
}

#[derive(serde::Deserialize)]
pub struct SyncResp {
    #[serde(default)]
    pub records: Vec<crate::sync_model::SyncRecord>,
    pub cursor: i64,
    #[serde(default)]
    pub more: bool,
}

/// One round trip of `/api/accounts/sync`: push `records`, pull everything after `since`.
/// `Rejected` means there is no session; the caller treats that as "nothing to do", not a failure.
pub async fn sync_call(since: i64, records: &[crate::sync_model::SyncRecord]) -> Result<SyncResp, Problem> {
    let a = access_token().await.ok_or(Problem::Rejected)?;
    let body = json!({ "since": since, "records": records });
    let (status, text) = post("/api/accounts/sync", &body, Some(&a)).await;
    if status != 200 {
        return Err(am::problem_for_status(status));
    }
    serde_json::from_str(&text).map_err(|_| Problem::Rejected)
}
