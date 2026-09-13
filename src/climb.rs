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

/// The saved session token, if any — shared with sibling account-gated modules
/// (e.g. the online Spell Off) so they authenticate as the same signed-in user
/// without duplicating token storage.
pub fn bearer() -> Option<String> {
    token()
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
    wire_front_door(app);
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
    // F12: the Settings route to account deletion exists only when there is an
    // account to delete (and never in Spell Jr, which has none).
    dom::toggle_class("setAccountRow", "btn-hide", gated || !is_logged_in());
    // The online Spell Off entry follows the same account + Kid-Mode gate (plus
    // its own feature flag); keep it in sync whenever auth state changes.
    crate::online_spelloff::reflect_gate();
    if gated {
        return;
    }
    // accountBtn is now an icon-only meta affordance (home-regroup F2): keep its
    // 👤 glyph and convey state via aria-label + a `signed-in` accent class,
    // rather than overwriting its text.
    let el = dom::el("accountBtn");
    match username() {
        Some(name) => {
            let _ = el.set_attribute("aria-label", &name);
            dom::toggle_class("accountBtn", "signed-in", true);
        }
        None => {
            let _ = el.set_attribute("aria-label", &crate::i18n::t("top.signIn"));
            dom::toggle_class("accountBtn", "signed-in", false);
        }
    }
}

// ---------- auth actions ----------

/// The account sheet, where a player renames themselves or deletes the account.
/// CC-ONBOARD-JR F12 wants deletion within three taps of Settings, so the
/// Settings row opens this same sheet rather than a second copy of it.
pub fn open_account_sheet() {
    dom::set_text("acctErr", "");
    dom::set_text("acctUsername", &username().unwrap_or_default());
    dom::input("acctNewUsername").set_value("");
    dom::input("acctDeletePassword").set_value("");
    dom::add_class("accountScrim", "show");
}

fn on_auth_success(v: &serde_json::Value) {
    if let Some(t) = v.get("token").and_then(|t| t.as_str()) {
        set_token(t);
    }
    if let Some(u) = v.get("user").and_then(|u| serde_json::from_value::<ClimbUser>(u.clone()).ok()) {
        set_user(Some(u));
    }
    reflect_auth();
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
                dom::set_text("acctErr", &crate::i18n::t("climb.userUpdated"));
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
    // I2: a locked junior reaches no leaderboard surface. climbBtn is hidden in
    // Spell Jr, but ghost.rs clicks it programmatically and a hidden button still
    // dispatches, so the refusal lives here rather than in the button's visibility.
    if dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false) {
        return;
    }
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
    dom::set_html("climbList", &format!("<div class=\"climb-loading\">{}</div>", crate::i18n::t("climb.loading")));
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
        dom::show_toast(&crate::i18n::t("toast.nameReported"));
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
        dom::show_toast(&crate::i18n::t("toast.loginToPost"));
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
                let dn = crate::i18n::t(&format!("level.{difficulty}"));
                dom::show_toast(&crate::i18n::tp("toast.newRecord", &[("rank", &rank.to_string()), ("difficulty", &dn)]));
            }
        }
    });
}

// ---------- wiring ----------

fn wire(_app: &App) {
    // Open account: sign-in form when logged out, settings when logged in.
    // Signed in: the account sheet. Signed out: the front door, which is now the
    // only sign-in surface (CC-ONBOARD-JR F7) -- the old signup modal posted to a
    // route that answers 410, so it was a dead end inside this very build.
    dom::on_click("accountBtn", || {
        if is_logged_in() {
            open_account_sheet();
        } else {
            open_front_door();
        }
    });
    // The board opens for everyone, guests included. D1 (signed 2026-09-02) keeps
    // the standings exactly as they were, and ghost.rs reaches them through this
    // click. CC-ONBOARD-JR F7 / Done 22a would send a guest to the front door
    // instead; the two decisions conflict, so this stays as D1 has it until Eric
    // picks one -- the spec's own stop-and-ask rule.
    dom::on_click("climbBtn", || open_leaderboard());
    // The leaderboard's entrance since F1 took the home tile out of the row.
    // Closes the account sheet first rather than stacking a second scrim on it
    // -- the same shape as ghost.rs, which closes its screen before routing.
    dom::on_click("acctClimb", || {
        dom::remove_class("accountScrim", "show");
        open_leaderboard();
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

#[cfg(test)]
mod entrance_tests {
    /// F1 suppressed the home tile, which had been the leaderboard's only tap
    /// entrance -- ghost.rs can reach the board, but ghost racing is
    /// status:hidden, so without this the standings became unreachable while
    /// D1 said they stay exactly as they are. Static, because the sheet only
    /// opens for a signed-in player and an e2e cannot get there cheaply.
    #[test]
    fn the_leaderboard_keeps_a_tap_entrance() {
        let html = include_str!("../index.html");
        assert!(html.contains("id=\"acctClimb\""),
                "the account sheet lost the leaderboard entrance");
        let src = include_str!("climb.rs");
        assert!(src.contains("on_click(\"acctClimb\""),
                "the entrance exists in markup but nothing wires it");
        // The suppressed tile still owns the handler ghost.rs clicks.
        assert!(src.contains("on_click(\"climbBtn\""),
                "climbBtn still owns open_leaderboard -- ghost.rs reaches the board through it");
    }
}

// ---------- the front door (CC-ONBOARD-JR F6-F10) ----------
//
// One screen, four steps. The order is F8's: email, then the six-digit code,
// then the password, then the Climb name -- so the account only exists once the
// address is proved, and there is no half-made account to chase. Log in is the
// same screen's first step, because a returning player and a new one arrive at
// the same door.
//
// D1 (signed) is why the guest link is always visible: play never requires an
// account. I8 is why the typed password lives in memory for the two steps
// between typing and sending, and is never stored or logged.

#[derive(Default, Clone)]
struct FrontDoor {
    email: String,
    purpose: String, // "signup" | "reset"
    code: String,
    password: String,
}

thread_local! {
    static FD: std::cell::RefCell<FrontDoor> = std::cell::RefCell::new(FrontDoor::default());
}

fn fd_step(step: &str) {
    let _ = dom::el("frontDoor").set_attribute("data-step", step);
    dom::set_text("fdErr", "");
}

fn fd_err(msg: &str) {
    dom::set_text("fdErr", msg);
}

fn online() -> bool {
    web_sys::window().map(|w| w.navigator().on_line()).unwrap_or(true)
}

/// True when the device cannot reach anything; the front door says so plainly
/// instead of failing into a generic error (F12: offline first launch still
/// plays, it just cannot make an account).
fn fd_offline_guard() -> bool {
    if online() {
        return false;
    }
    fd_err(&crate::i18n::t("fd.offline"));
    true
}

pub fn open_front_door() {
    // I1/I2 — the one place the door opens is the one place a child is refused.
    // Every caller goes through here -- first launch and the account icon -- and a
    // programmatic click fires even on a hidden button, so no caller present or
    // future can put an email or password field in front of a Spell Jr player.
    if dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false) {
        return;
    }
    FD.with(|f| *f.borrow_mut() = FrontDoor::default());
    dom::input("fdEmail").set_value("");
    dom::input("fdPassword").set_value("");
    dom::input("fdCode").set_value("");
    dom::input("fdNewPassword").set_value("");
    dom::input("fdUsername").set_value("");
    fd_step("start");
    dom::add_class("frontDoor", "show");
}

pub fn close_front_door() {
    dom::remove_class("frontDoor", "show");
}

/// F9's live checklist. The three rules the player can see are checked here;
/// the server is still the authority and owns the rest (length ceiling, the
/// common-password blocklist), whose answers arrive as an error on submit.
fn fd_update_rules() {
    let pw = dom::input("fdNewPassword").value();
    let len = pw.chars().count() >= 8;
    let digit = pw.chars().any(|c| c.is_ascii_digit());
    let symbol = pw.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace());
    for (id, met) in [("fdRuleLen", len), ("fdRuleDigit", digit), ("fdRuleSymbol", symbol)] {
        dom::toggle_class(id, "met", met);
    }
}

fn fd_toggle_password(input_id: &str, button_id: &str) {
    let el = dom::input(input_id);
    let showing = el.type_() == "text";
    el.set_type(if showing { "password" } else { "text" });
    let b = dom::el(button_id);
    let _ = b.set_attribute("aria-pressed", if showing { "false" } else { "true" });
    b.set_text_content(Some(&crate::i18n::t(if showing { "fd.show" } else { "fd.hide" })));
}

fn fd_request_code(purpose: &str) {
    let email = dom::input("fdEmail").value().trim().to_string();
    if !email.contains('@') || !email.contains('.') {
        fd_err(&crate::i18n::t("fd.badEmail"));
        return;
    }
    if fd_offline_guard() {
        return;
    }
    FD.with(|f| {
        let mut f = f.borrow_mut();
        f.email = email.clone();
        f.purpose = purpose.to_string();
    });
    let purpose = purpose.to_string();
    spawn_local(async move {
        let b = body(&[("email", s(&email)), ("purpose", s(&purpose))]);
        // The response is deliberately the same whatever the address is, so
        // there is nothing here to branch on -- and nothing to leak.
        let _ = call("POST", "/api/auth/request-code", Some(b)).await;
        dom::set_text(
            "fdCodeHelp",
            &crate::i18n::tp("fd.codeHelp", &[("email", &email)]),
        );
        dom::input("fdCode").set_value("");
        fd_step("code");
    });
}

fn fd_submit_code() {
    let code = dom::input("fdCode").value().trim().to_string();
    let (email, purpose) = FD.with(|f| {
        let f = f.borrow();
        (f.email.clone(), f.purpose.clone())
    });
    if fd_offline_guard() {
        return;
    }
    spawn_local(async move {
        let b = body(&[("email", s(&email)), ("purpose", s(&purpose)), ("code", s(&code))]);
        match call("POST", "/api/auth/verify-code", Some(b)).await {
            Ok(_) => {
                FD.with(|f| f.borrow_mut().code = code.clone());
                dom::set_text(
                    "fdPwTitle",
                    &crate::i18n::t(if purpose == "reset" { "fd.newPwTitle" } else { "fd.pwTitle" }),
                );
                dom::input("fdNewPassword").set_value("");
                fd_update_rules();
                fd_step("password");
            }
            Err(e) => fd_err(&e.message),
        }
    });
}

fn fd_submit_password() {
    let password = dom::input("fdNewPassword").value();
    let (email, purpose, code) = FD.with(|f| {
        let f = f.borrow();
        (f.email.clone(), f.purpose.clone(), f.code.clone())
    });
    if fd_offline_guard() {
        return;
    }
    if purpose == "signup" {
        FD.with(|f| f.borrow_mut().password = password);
        dom::input("fdUsername").set_value("");
        fd_step("name");
        return;
    }
    // Reset: set the password, then sign in with it, so the player lands in the
    // game rather than back at a login form they just proved they own.
    spawn_local(async move {
        let b = body(&[("email", s(&email)), ("code", s(&code)), ("newPassword", s(&password))]);
        match call("POST", "/api/auth/reset-password", Some(b)).await {
            Ok(_) => {
                let lb = body(&[("identifier", s(&email)), ("password", s(&password))]);
                match call("POST", "/api/auth/login", Some(lb)).await {
                    Ok(v) => {
                        on_auth_success(&v);
                        upload_local_bests();
                        close_front_door();
                    }
                    Err(_) => fd_step("start"),
                }
            }
            Err(e) => fd_err(&e.message),
        }
    });
}

fn fd_submit_name() {
    let username = dom::input("fdUsername").value().trim().to_string();
    let (email, code, password) = FD.with(|f| {
        let f = f.borrow();
        (f.email.clone(), f.code.clone(), f.password.clone())
    });
    if fd_offline_guard() {
        return;
    }
    spawn_local(async move {
        let b = body(&[
            ("email", s(&email)),
            ("code", s(&code)),
            ("username", s(&username)),
            ("password", s(&password)),
        ]);
        match call("POST", "/api/auth/complete-signup", Some(b)).await {
            Ok(v) => {
                on_auth_success(&v);
                FD.with(|f| *f.borrow_mut() = FrontDoor::default()); // drop the password
                upload_local_bests();
                close_front_door();
            }
            Err(e) => fd_err(&e.message),
        }
    });
}

/// F12 / Done 22a — a guest's runs follow them onto the account, exactly once.
///
/// Only runs with a REAL recorded duration are sent: the leaderboard's
/// anti-cheat judges a run by its timing, and inventing one to make an old
/// score fit would be lying to it. Runs recorded before this shipped simply
/// stay local, which loses the player nothing they can see.
fn upload_local_bests() {
    let mut board: Vec<crate::model::BoardEntry> =
        crate::storage::get_json(crate::model::LB_KEY).unwrap_or_default();
    let mut changed = false;
    for e in board.iter_mut() {
        if e.uploaded || e.duration_ms <= 0.0 || !DIFFICULTIES.contains(&e.level.as_str()) {
            continue;
        }
        submit_run(&e.level, e.streak, e.duration_ms);
        e.uploaded = true;
        changed = true;
    }
    if changed {
        crate::storage::set_json(crate::model::LB_KEY, &board);
    }
}

pub fn wire_front_door(app: &App) {
    let _ = app;
    dom::on_click("fdLogin", || {
        let email = dom::input("fdEmail").value().trim().to_string();
        let password = dom::input("fdPassword").value();
        if fd_offline_guard() {
            return;
        }
        spawn_local(async move {
            let b = body(&[("identifier", s(&email)), ("password", s(&password))]);
            match call("POST", "/api/auth/login", Some(b)).await {
                Ok(v) => {
                    on_auth_success(&v);
                    upload_local_bests();
                    close_front_door();
                }
                Err(e) => fd_err(&e.message),
            }
        });
    });
    dom::on_click("fdCreate", || fd_request_code("signup"));
    dom::on_click("fdForgot", || fd_request_code("reset"));
    dom::on_click("fdCodeSubmit", || fd_submit_code());
    dom::on_click("fdResend", || {
        let purpose = FD.with(|f| f.borrow().purpose.clone());
        fd_request_code(&purpose);
    });
    dom::on_click("fdBack", || fd_step("start"));
    dom::on_click("fdPwBack", || fd_step("start"));
    dom::on_click("fdPwSubmit", || fd_submit_password());
    dom::on_click("fdNameSubmit", || fd_submit_name());
    dom::on_click("fdGuest", || close_front_door());
    dom::on_click("fdPwToggle", || fd_toggle_password("fdPassword", "fdPwToggle"));
    dom::on_click("fdNewPwToggle", || fd_toggle_password("fdNewPassword", "fdNewPwToggle"));
    dom::on::<web_sys::Event, _>("fdNewPassword", "input", |_| fd_update_rules());
}
