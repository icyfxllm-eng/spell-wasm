//! Async online "Spell Off" — 1v1 head-to-head between friends across the world.
//!
//! Like the Daily Challenge, but 1v1 with a SHARED, server-owned SEED and NO
//! real-time link (no WebSockets). One player creates a match; the server mints a
//! crypto-random seed and a short friend code. Both players spell the SAME words
//! — derived deterministically from that seed by `daily::build_words_from_seed`
//! — on their own time, then submit their result. The server compares the two
//! and declares a winner (see `backend/matches.py`).
//!
//! ADULT-GATED BY DESIGN (COPPA): the entry point exists only when the player is
//! signed in (a `climb` account — the app's adult surface) AND not in Kid Mode.
//! There is no anonymous/Kid-Mode online play, no stranger matchmaking, and no
//! chat — you can only face someone you shared a friend code with.
//!
//! Behind the `flags::online_spelloff()` feature flag (OFF by default): while off
//! there is no entry point and none of this runs, so the build is unchanged.

use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

use crate::App;
use crate::{api, climb, dom, game};

/// Words per match — must match the backend's `WORD_COUNT`.
const WORD_COUNT: usize = 10;

#[derive(Clone, Deserialize, Default)]
struct MatchInfo {
    code: String,
    seed: String,
    lang: String,
    tier: String,
    #[serde(default)]
    status: String,
    /// Which side the caller is: "a" | "b" | null.
    #[serde(default)]
    you: Option<String>,
    #[serde(default)]
    winner: Option<String>,
    #[serde(rename = "submittedA", default)]
    submitted_a: bool,
    #[serde(rename = "submittedB", default)]
    submitted_b: bool,
}

thread_local! {
    /// The match currently being played (set on create/join, cleared on exit),
    /// plus the run's start time for the elapsed-ms result.
    static ACTIVE: std::cell::RefCell<Option<MatchInfo>> = const { std::cell::RefCell::new(None) };
    static RUN_START_MS: std::cell::RefCell<f64> = const { std::cell::RefCell::new(0.0) };
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

// ---------- HTTP (mirrors climb.rs, authenticates as the signed-in user) ----------

struct ApiErr {
    message: String,
}

async fn call(method: &str, path: &str, body: Option<String>) -> Result<serde_json::Value, ApiErr> {
    let generic = || ApiErr { message: crate::i18n::t("so.errNetwork") };
    let url = format!("{}{}", api::api_base(), path);
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);
    let headers = Headers::new().map_err(|_| generic())?;
    let _ = headers.set("Content-Type", "application/json");
    if let Some(t) = climb::bearer() {
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
            .unwrap_or(&crate::i18n::t("so.errGeneric"))
            .to_string();
        Err(ApiErr { message: msg })
    }
}

// ---------- gating ----------

/// Online play is reachable only when the flag is on, the player is signed in,
/// and it's not Kid Mode (COPPA — accounts are the adult surface).
fn allowed() -> bool {
    if !crate::flags::online_spelloff() {
        return false;
    }
    let kid = dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false);
    !kid && climb::is_logged_in()
}

/// Show/hide the "Challenge a friend" entry point per the gate above. Called on
/// boot and whenever auth/Kid-Mode state changes.
pub fn reflect_gate() {
    dom::toggle_class("soBtn", "btn-hide", !allowed());
}

// ---------- create / join ----------

fn set_err(msg: &str) {
    dom::set_text("soErr", msg);
}

/// The lang + tier the created match will use: the player's current study
/// language, and their level clamped to a real tier (Climb → medium).
fn lang_and_tier(app: &App) -> (String, String) {
    let s = app.borrow();
    let lang = s.lang.clone();
    let tier = match s.level.as_str() {
        "easy" | "medium" | "hard" | "expert" => s.level.clone(),
        _ => "medium".to_string(), // "climb" (adaptive) or unknown → a fixed tier
    };
    (lang, tier)
}

fn do_create(app: &App) {
    if !allowed() {
        return;
    }
    let (lang, tier) = lang_and_tier(app);
    set_err("");
    dom::set_text("soCreateBtn", &crate::i18n::t("so.creating"));
    spawn_local(async move {
        let b = serde_json::json!({ "lang": lang, "tier": tier }).to_string();
        dom::set_text("soCreateBtn", &crate::i18n::t("so.create"));
        match call("POST", "/api/match", Some(b)).await {
            Ok(v) => match serde_json::from_value::<MatchInfo>(v) {
                Ok(m) => {
                    dom::set_text("soCode", &m.code);
                    dom::remove_class("soCodeBox", "btn-hide");
                    ACTIVE.with(|a| *a.borrow_mut() = Some(m));
                }
                Err(_) => set_err(&crate::i18n::t("so.errGeneric")),
            },
            Err(e) => set_err(&e.message),
        }
    });
}

fn do_join(app: &App) {
    if !allowed() {
        return;
    }
    let code = dom::input("soJoinCode").value().trim().to_uppercase();
    if code.is_empty() {
        return;
    }
    set_err("");
    let app = app.clone();
    spawn_local(async move {
        match call("POST", &format!("/api/match/{code}/join"), None).await {
            Ok(v) => match serde_json::from_value::<MatchInfo>(v) {
                Ok(m) => start_playing(&app, m),
                Err(_) => set_err(&crate::i18n::t("so.errGeneric")),
            },
            Err(e) => set_err(&e.message),
        }
    });
}

/// From the created-match view, start playing the words A generated.
fn play_created(app: &App) {
    let m = ACTIVE.with(|a| a.borrow().clone());
    if let Some(m) = m {
        start_playing(app, m);
    }
}

/// Derive the shared words from the match seed and hand off to the Daily-style
/// run machinery (see `game::start_spelloff_run`).
fn start_playing(app: &App, m: MatchInfo) {
    let seed = u64::from_str_radix(m.seed.trim(), 16).unwrap_or(0);
    let words = crate::daily::build_words_from_seed(&m.lang, &m.tier, seed, WORD_COUNT);
    if words.is_empty() {
        set_err(&crate::i18n::t("so.errGeneric"));
        return;
    }
    let locale = crate::daily::locale_for(&m.lang);
    ACTIVE.with(|a| *a.borrow_mut() = Some(m));
    RUN_START_MS.with(|r| *r.borrow_mut() = now_ms());
    dom::remove_class("soScrim", "show");
    dom::remove_class("soCodeBox", "btn-hide");
    game::start_spelloff_run(app, locale, words);
}

// ---------- finish / result ----------

/// Called by `game::finish_daily` when a Spell Off run completes. Submits this
/// side's result to the server, then polls the outcome.
pub fn finish_run(app: &App, correct: u32, total: u32) {
    let m = ACTIVE.with(|a| a.borrow().clone());
    let Some(m) = m else { return };
    let elapsed = (now_ms() - RUN_START_MS.with(|r| *r.borrow())).max(0.0) as i64;
    // Scaffold score: correct answers scaled — the server decides by `correct`
    // (tiebreak elapsed), so score is informational for now.
    let score = correct as i64 * 10;
    let code = m.code.clone();
    let app = app.clone();
    spawn_local(async move {
        let b = serde_json::json!({
            "score": score,
            "correct": correct,
            "total": total,
            "elapsed_ms": elapsed,
        })
        .to_string();
        match call("POST", &format!("/api/match/{code}/result"), Some(b)).await {
            Ok(v) => {
                if let Ok(res) = serde_json::from_value::<MatchInfo>(v) {
                    show_outcome(&res);
                } else {
                    show_outcome(&m);
                }
            }
            // Even if submit fails, show the local score so the run isn't a dead end.
            Err(e) => {
                dom::set_text("soResultTitle", &crate::i18n::t("so.doneTitle"));
                dom::set_text("soResultMsg", &e.message);
                dom::add_class("soResultScrim", "show");
            }
        }
        let _ = &app;
    });
}

/// Poll the current match and refresh the outcome view (the "waiting for friend"
/// → "you won/lost" transition, since there's no push channel).
fn refresh_outcome() {
    let m = ACTIVE.with(|a| a.borrow().clone());
    let Some(m) = m else { return };
    let code = m.code.clone();
    spawn_local(async move {
        if let Ok(v) = call("GET", &format!("/api/match/{code}"), None).await {
            if let Ok(res) = serde_json::from_value::<MatchInfo>(v) {
                show_outcome(&res);
            }
        }
    });
}

/// Render the head-to-head outcome: your win/loss/tie once both are in, else a
/// "waiting for your friend" holding state.
fn show_outcome(m: &MatchInfo) {
    ACTIVE.with(|a| *a.borrow_mut() = Some(m.clone()));
    let you = m.you.clone().unwrap_or_default();
    let both_in = m.submitted_a && m.submitted_b;
    let (title, msg) = if m.status == "complete" || both_in {
        match m.winner.as_deref() {
            Some("tie") => (crate::i18n::t("so.tieTitle"), crate::i18n::t("so.tieMsg")),
            Some(side) if side == you => (crate::i18n::t("so.wonTitle"), crate::i18n::t("so.wonMsg")),
            Some(_) => (crate::i18n::t("so.lostTitle"), crate::i18n::t("so.lostMsg")),
            None => (crate::i18n::t("so.doneTitle"), crate::i18n::t("so.waitingMsg")),
        }
    } else {
        (crate::i18n::t("so.doneTitle"), crate::i18n::t("so.waitingMsg"))
    };
    dom::set_text("soResultTitle", &title);
    dom::set_text("soResultMsg", &msg);
    // Only offer "check again" while still waiting on the other side.
    dom::toggle_class("soResultRefresh", "btn-hide", m.status == "complete" || both_in);
    dom::add_class("soResultScrim", "show");
}

// ---------- boot / wiring ----------

pub fn setup(app: &App) {
    reflect_gate();

    // Entry point → open the create/join modal (guarded).
    {
        let a = app.clone();
        dom::on_click("soBtn", move || {
            if !allowed() {
                return;
            }
            set_err("");
            dom::add_class("soCodeBox", "btn-hide");
            dom::input("soJoinCode").set_value("");
            dom::add_class("soScrim", "show");
            let _ = &a;
        });
    }
    {
        let a = app.clone();
        dom::on_click("soCreateBtn", move || do_create(&a));
    }
    {
        let a = app.clone();
        dom::on_click("soJoinBtn", move || do_join(&a));
    }
    {
        let a = app.clone();
        dom::on_click("soPlay", move || play_created(&a));
    }
    dom::on_click("soClose", || dom::remove_class("soScrim", "show"));
    dom::on::<web_sys::Event, _>("soScrim", "click", |e| {
        if dom::is_self_target(&e, "soScrim") {
            dom::remove_class("soScrim", "show");
        }
    });

    // Result view: poll again, or close.
    dom::on_click("soResultRefresh", || refresh_outcome());
    dom::on_click("soResultClose", || {
        dom::remove_class("soResultScrim", "show");
        ACTIVE.with(|a| *a.borrow_mut() = None);
    });
    dom::on::<web_sys::Event, _>("soResultScrim", "click", |e| {
        if dom::is_self_target(&e, "soResultScrim") {
            dom::remove_class("soResultScrim", "show");
        }
    });
}
