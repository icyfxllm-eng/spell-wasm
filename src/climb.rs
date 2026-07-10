//! The Climb — frontend: player accounts + global leaderboard. Talks to the
//! Flask backend (`/api/auth/*`, `/api/climb/*`) on the same origin
//! (`window.SPELL_API_BASE`). The session token lives in localStorage (persists
//! across launches in the Capacitor webview) and is sent as `Authorization:
//! Bearer`. Accounts and the leaderboard are gated OFF in Kid Mode / on
//! age-locked devices — no signup/login/email/phone UI is reachable there.

use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

use crate::App;
use crate::{api, dom, storage};

const TOKEN_KEY: &str = "byear_climb_token_v1";
const DIFFICULTIES: [&str; 3] = ["medium", "hard", "expert"];

thread_local! {
    static USER: std::cell::RefCell<Option<ClimbUser>> = const { std::cell::RefCell::new(None) };
    static TAB: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

#[derive(Clone, Deserialize)]
pub struct ClimbUser {
    pub id: i64,
    pub username: String,
}

// ---------- token + session state ----------

fn token() -> Option<String> {
    storage::get_raw(TOKEN_KEY).filter(|s| !s.is_empty())
}

fn set_token(t: &str) {
    storage::set_raw(TOKEN_KEY, t);
}

fn clear_token() {
    storage::set_raw(TOKEN_KEY, "");
}

fn set_user(u: Option<ClimbUser>) {
    USER.with(|c| *c.borrow_mut() = u);
}

pub fn is_logged_in() -> bool {
    USER.with(|c| c.borrow().is_some())
}

fn username() -> Option<String> {
    USER.with(|c| c.borrow().as_ref().map(|u| u.username.clone()))
}

fn user_id() -> Option<i64> {
    USER.with(|c| c.borrow().as_ref().map(|u| u.id))
}

// ---------- HTTP ----------

struct ApiErr {
    message: String,
}

async fn call(method: &str, path: &str, body: Option<String>) -> Result<serde_json::Value, ApiErr> {
    let generic = || ApiErr { message: "Couldn't reach The Climb. Check your connection.".to_string() };
    let url = format!("{}{}", api::api_base(), path);
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);
    let headers = Headers::new().map_err(|_| generic())?;
    let _ = headers.set("Content-Type", "application/json");
    if let Some(t) = token() {
        let _ = headers.set("Authorization", &format!("Bearer {t}"));
    }
    opts.set_headers(&headers);
    if let Some(b) = &body {
        opts.set_body(&JsValue::from_str(b));
    }
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|_| generic())?;
    let win = web_sys::window().ok_or_else(generic)?;
    let resp_val = JsFuture::from(win.fetch_with_request(&req)).await.map_err(|_| generic())?;
    let resp: Response = resp_val.dyn_into().map_err(|_| generic())?;
    let text_val = JsFuture::from(resp.text().map_err(|_| generic())?).await.map_err(|_| generic())?;
    let text = text_val.as_string().unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if resp.ok() {
        Ok(json)
    } else {
        let msg = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Something went wrong. Please try again.")
            .to_string();
        Err(ApiErr { message: msg })
    }
}

fn body(pairs: &[(&str, serde_json::Value)]) -> String {
    let map: serde_json::Map<String, serde_json::Value> =
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    serde_json::Value::Object(map).to_string()
}

fn s(v: &str) -> serde_json::Value {
    serde_json::Value::String(v.to_string())
}

// ---------- boot / session restore ----------

/// Wire the account + leaderboard UI and restore a saved session (validates the
/// stored token against /me; drops it if the server rejects it).
pub fn setup(app: &App) {
    wire(app);
    reflect_auth();
    if token().is_some() {
        let app = app.clone();
        spawn_local(async move {
            match call("GET", "/api/auth/me", None).await {
                Ok(v) => {
                    if let Some(u) = v.get("user").and_then(|u| serde_json::from_value::<ClimbUser>(u.clone()).ok()) {
                        set_user(Some(u));
                    } else {
                        clear_token();
                    }
                }
                Err(_) => { /* offline — keep the token, try again next launch */ }
            }
            reflect_auth();
            let _ = &app;
        });
    }
}

/// Reflect login state + Kid-Mode gating into the top-bar controls.
pub fn reflect_auth() {
    // Kid Mode / age-locked: no accounts or leaderboard at all.
    let gated = dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false);
    dom::toggle_class("climbBtn", "btn-hide", gated);
    dom::toggle_class("accountBtn", "btn-hide", gated);
    if gated {
        return;
    }
    match username() {
        Some(name) => dom::set_text("accountBtn", &format!("\u{1F464} {name}")),
        None => dom::set_text("accountBtn", &crate::i18n::t("top.signIn")),
    }
}

// ---------- auth actions ----------

fn set_auth_err(msg: &str) {
    dom::set_text("authErr", msg);
}

fn on_auth_success(v: &serde_json::Value) {
    if let Some(t) = v.get("token").and_then(|t| t.as_str()) {
        set_token(t);
    }
    if let Some(u) = v.get("user").and_then(|u| serde_json::from_value::<ClimbUser>(u.clone()).ok()) {
        set_user(Some(u));
    }
    dom::remove_class("authScrim", "show");
    reflect_auth();
}

fn do_signup(_app: &App) {
    let username = dom::input("authUsername").value();
    let email = dom::input("authEmail").value();
    let password = dom::input("authPassword").value();
    set_auth_err("");
    spawn_local(async move {
        let b = body(&[("username", s(&username)), ("email", s(&email)), ("password", s(&password))]);
        match call("POST", "/api/auth/signup", Some(b)).await {
            Ok(v) => on_auth_success(&v),
            Err(e) => set_auth_err(&e.message),
        }
    });
}

fn do_login(_app: &App) {
    let identifier = dom::input("authIdentifier").value();
    let password = dom::input("authLoginPassword").value();
    set_auth_err("");
    spawn_local(async move {
        let b = body(&[("identifier", s(&identifier)), ("password", s(&password))]);
        match call("POST", "/api/auth/login", Some(b)).await {
            Ok(v) => on_auth_success(&v),
            Err(e) => set_auth_err(&e.message),
        }
    });
}

fn do_logout() {
    spawn_local(async move {
        let _ = call("POST", "/api/auth/logout", None).await;
        clear_token();
        set_user(None);
        dom::remove_class("accountScrim", "show");
        reflect_auth();
    });
}

fn do_change_username() {
    let new = dom::input("acctNewUsername").value();
    dom::set_text("acctErr", "");
    spawn_local(async move {
        match call("POST", "/api/auth/change-username", Some(body(&[("username", s(&new))]))).await {
            Ok(v) => {
                if let Some(name) = v.get("username").and_then(|n| n.as_str()) {
                    USER.with(|c| {
                        if let Some(u) = c.borrow_mut().as_mut() {
                            u.username = name.to_string();
                        }
                    });
                }
                dom::set_text("acctErr", "Username updated.");
                reflect_auth();
            }
            Err(e) => dom::set_text("acctErr", &e.message),
        }
    });
}

fn do_delete_account() {
    let password = dom::input("acctDeletePassword").value();
    dom::set_text("acctErr", "");
    spawn_local(async move {
        match call("POST", "/api/auth/delete-account", Some(body(&[("password", s(&password))]))).await {
            Ok(_) => {
                clear_token();
                set_user(None);
                dom::remove_class("accountScrim", "show");
                reflect_auth();
            }
            Err(e) => dom::set_text("acctErr", &e.message),
        }
    });
}

// ---------- leaderboard ----------

fn open_leaderboard() {
    let tab = TAB.with(|t| {
        let mut t = t.borrow_mut();
        if t.is_empty() {
            *t = "medium".to_string();
        }
        t.clone()
    });
    dom::add_class("climbScrim", "show");
    render_tab(&tab);
}

fn render_tab(difficulty: &str) {
    TAB.with(|t| *t.borrow_mut() = difficulty.to_string());
    for d in DIFFICULTIES {
        dom::toggle_class(&format!("climbTab-{d}"), "on", d == difficulty);
    }
    dom::set_html("climbList", "<div class=\"climb-loading\">Loading\u{2026}</div>");
    let difficulty = difficulty.to_string();
    // Default the board to the player's current language (§4.4).
    let locale = crate::i18n::current();
    spawn_local(async move {
        match call("GET", &format!("/api/climb/leaderboard?difficulty={difficulty}&locale={locale}"), None).await {
            Ok(v) => dom::set_html("climbList", &render_rows(&v)),
            Err(e) => dom::set_html("climbList", &format!("<div class=\"climb-empty\">{}</div>", dom::escape_html(&e.message))),
        }
    });
}

fn row_html(rank: i64, name: &str, chain: i64, uid: i64, me: bool) -> String {
    let cls = if me { "climb-row me" } else { "climb-row" };
    format!(
        "<div class=\"{cls}\"><span class=\"c-rank\">{rank}</span>\
         <span class=\"c-name\">{name}</span>\
         <span class=\"c-chain\">{chain}</span>\
         <button class=\"c-report\" data-uid=\"{uid}\" title=\"Report name\">\u{2691}</button></div>",
        name = dom::escape_html(name)
    )
}

fn render_rows(v: &serde_json::Value) -> String {
    let mine = user_id();
    let empty = v.get("top").and_then(|t| t.as_array()).map(|a| a.is_empty()).unwrap_or(true);
    if empty {
        return "<div class=\"climb-empty\">No chains posted yet \u{2014} be the first.</div>".to_string();
    }
    let mut html = String::new();
    if let Some(top) = v.get("top").and_then(|t| t.as_array()) {
        for e in top {
            let rank = e.get("rank").and_then(|r| r.as_i64()).unwrap_or(0);
            let name = e.get("username").and_then(|n| n.as_str()).unwrap_or("");
            let chain = e.get("chain").and_then(|c| c.as_i64()).unwrap_or(0);
            let uid = e.get("userId").and_then(|u| u.as_i64()).unwrap_or(0);
            html.push_str(&row_html(rank, name, chain, uid, Some(uid) == mine));
        }
    }
    if let Some(me) = v.get("me").filter(|m| !m.is_null()) {
        html.push_str("<div class=\"climb-sep\">\u{22ef}</div>");
        let rank = me.get("rank").and_then(|r| r.as_i64()).unwrap_or(0);
        let name = me.get("username").and_then(|n| n.as_str()).unwrap_or("");
        let chain = me.get("chain").and_then(|c| c.as_i64()).unwrap_or(0);
        let uid = me.get("userId").and_then(|u| u.as_i64()).unwrap_or(0);
        html.push_str(&row_html(rank, name, chain, uid, true));
    }
    html
}

fn report_name(uid: i64) {
    spawn_local(async move {
        let _ = call("POST", "/api/climb/report-name", Some(body(&[("userId", serde_json::json!(uid))]))).await;
        dom::show_toast("Thanks \u{2014} that name was reported.");
    });
}

// ---------- run submission ----------

/// Called when a solo run ends. Submits the chain to The Climb only for a fixed
/// ranked difficulty (medium/hard/expert) when logged in; otherwise prompts.
/// Never called from Kid Mode / head-to-head (guarded by the caller).
pub fn submit_run(difficulty: &str, chain: u32, duration_ms: f64) {
    if !DIFFICULTIES.contains(&difficulty) || chain == 0 {
        return;
    }
    if !is_logged_in() {
        dom::show_toast("Log in to post your chain to The Climb.");
        return;
    }
    let difficulty = difficulty.to_string();
    // The run's word language segments the leaderboard (§4.4).
    let locale = crate::i18n::current();
    spawn_local(async move {
        let meta = serde_json::json!({"wordCount": chain, "durationMs": duration_ms});
        let b = body(&[
            ("difficulty", s(&difficulty)),
            ("locale", s(&locale)),
            ("chain", serde_json::json!(chain)),
            ("meta", meta),
        ]);
        if let Ok(v) = call("POST", "/api/climb/submit-chain", Some(b)).await {
            if v.get("record").and_then(|r| r.as_bool()).unwrap_or(false) {
                let rank = v.get("rank").and_then(|r| r.as_i64()).unwrap_or(0);
                dom::show_toast(&format!("New record! Posted to The Climb \u{2014} #{rank} on {difficulty}."));
            }
        }
    });
}

// ---------- wiring ----------

fn wire(app: &App) {
    // Open account: sign-in form when logged out, settings when logged in.
    dom::on_click("accountBtn", || {
        if is_logged_in() {
            dom::set_text("acctErr", "");
            dom::set_text("acctUsername", &username().unwrap_or_default());
            dom::input("acctNewUsername").set_value("");
            dom::input("acctDeletePassword").set_value("");
            dom::add_class("accountScrim", "show");
        } else {
            set_auth_err("");
            dom::add_class("authScrim", "show");
        }
    });
    dom::on_click("climbBtn", || open_leaderboard());

    // Auth modal: toggle login/signup, submit, close.
    dom::on_click("authToLogin", || dom::toggle_class("authScrim", "login-mode", true));
    dom::on_click("authToSignup", || dom::toggle_class("authScrim", "login-mode", false));
    {
        let a = app.clone();
        dom::on_click("authSignupBtn", move || do_signup(&a));
    }
    {
        let a = app.clone();
        dom::on_click("authLoginBtn", move || do_login(&a));
    }
    dom::on_click("authClose", || dom::remove_class("authScrim", "show"));
    dom::on::<web_sys::Event, _>("authScrim", "click", |e| {
        if dom::is_self_target(&e, "authScrim") {
            dom::remove_class("authScrim", "show");
        }
    });

    // Account settings modal.
    dom::on_click("acctChangeName", || do_change_username());
    dom::on_click("acctDelete", || do_delete_account());
    dom::on_click("acctLogout", || do_logout());
    dom::on_click("acctClose", || dom::remove_class("accountScrim", "show"));
    dom::on::<web_sys::Event, _>("accountScrim", "click", |e| {
        if dom::is_self_target(&e, "accountScrim") {
            dom::remove_class("accountScrim", "show");
        }
    });

    // Leaderboard modal: tabs, report-name (delegated), close.
    for d in DIFFICULTIES {
        dom::on_click(&format!("climbTab-{d}"), move || render_tab(d));
    }
    dom::on_click("climbClose", || dom::remove_class("climbScrim", "show"));
    dom::on::<web_sys::Event, _>("climbScrim", "click", |e| {
        if dom::is_self_target(&e, "climbScrim") {
            dom::remove_class("climbScrim", "show");
        }
    });
    // Report buttons are rebuilt on each render — delegate from the list.
    dom::on::<web_sys::MouseEvent, _>("climbList", "click", |e| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            if let Some(btn) = t.closest(".c-report").ok().flatten() {
                if let Some(uid) = btn.get_attribute("data-uid").and_then(|u| u.parse::<i64>().ok()) {
                    report_name(uid);
                }
            }
        }
    });
}
