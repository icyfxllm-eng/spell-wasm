//! Daily practice reminder via `@capacitor/local-notifications`, reached (no
//! bundler) through `window.Capacitor.Plugins.LocalNotifications`. Purely a
//! local, on-device schedule — no server / FCM. No-op on the web.
//!
//! One notification, fixed id, fired daily at the user's chosen time (see
//! CLAUDE.md Phase 2: one/day, user-scheduled, off by default, and suppressed
//! in Kid Mode — the caller passes `enabled = remind && !kid`).

use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

const NOTIF_ID: i32 = 1;

fn plugin() -> Option<JsValue> {
    let win = web_sys::window()?;
    let cap = Reflect::get(&win, &JsValue::from_str("Capacitor")).ok()?;
    if cap.is_undefined() || cap.is_null() {
        return None;
    }
    let plugins = Reflect::get(&cap, &JsValue::from_str("Plugins")).ok()?;
    let ln = Reflect::get(&plugins, &JsValue::from_str("LocalNotifications")).ok()?;
    if ln.is_undefined() || ln.is_null() {
        None
    } else {
        Some(ln)
    }
}

fn method(obj: &JsValue, name: &str) -> Option<Function> {
    Reflect::get(obj, &JsValue::from_str(name)).ok()?.dyn_into::<Function>().ok()
}

fn set(obj: &Object, key: &str, val: &JsValue) {
    let _ = Reflect::set(obj, &JsValue::from_str(key), val);
}

/// CC-GUARDIAN-REPORT F7 + CC-GUARDIAN-DASH F2: the ONE weekly Guardian
/// notification (I11). DASH's digest id, kept so an old repeating digest is
/// cancelled by the first reschedule.
const GUARDIAN_ID: f64 = 3.0;
const GUARDIAN_ACTIONS: &str = "GUARDIAN_REPORT";
/// The one opt-in, shared by the report screen and the dashboard (off by
/// default). DASH's key, so a parent who opted in there stays opted in.
const DIGEST_KEY: &str = "spell_digest_on";

/// Local notifications can run here (the native app, not the site).
pub fn available() -> bool {
    plugin().is_some()
}

pub fn digest_on() -> bool {
    crate::storage::get_raw(DIGEST_KEY).as_deref() == Some("1")
}

pub fn set_digest_on(on: bool) {
    crate::storage::set_raw(DIGEST_KEY, if on { "1" } else { "0" });
}

/// The OS notification permission: `Some(granted)`, or `None` where there is
/// no plugin. `request` asks (the prompt, if the OS still shows one); only the
/// parent turning a toggle on may pass it (F7: never at launch).
pub fn permission(request: bool, then: impl FnOnce(Option<bool>) + 'static) {
    let Some(ln) = plugin() else { return then(None) };
    let name = if request { "requestPermissions" } else { "checkPermissions" };
    let promise = method(&ln, name).and_then(|f| f.call0(&ln).ok()).and_then(|p| p.dyn_into::<js_sys::Promise>().ok());
    let Some(promise) = promise else { return then(None) };
    wasm_bindgen_futures::spawn_local(async move {
        let display = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .ok()
            .and_then(|v| Reflect::get(&v, &JsValue::from_str("display")).ok())
            .and_then(|v| v.as_string());
        then(display.map(|d| d == "granted"));
    });
}

/// F7 scheduling: cancel the pending Guardian notification, then schedule
/// `ping` (one, non-repeating) if there is one. Local notifications fix their
/// content when scheduled, so callers reschedule on every foreground and
/// session end. The payload is counts only (I7).
#[cfg(not(feature = "web"))]
pub fn guardian_ping(ping: Option<&crate::guardian_report::Ping>) {
    let Some(ln) = plugin() else { return };
    let ids = Array::new();
    let id_obj = Object::new();
    set(&id_obj, "id", &JsValue::from_f64(GUARDIAN_ID));
    ids.push(&id_obj);
    let cancel_arg = Object::new();
    set(&cancel_arg, "notifications", &ids);
    if let Some(f) = method(&ln, "cancel") {
        let _ = f.call1(&ln, &cancel_arg);
    }
    let Some(p) = ping else { return };
    let (y, m, d) = p.date;
    // A concrete local date, so `at` fires once (an `on` clause repeats).
    let when = js_sys::Date::new_with_year_month_day_hr(y as u32, m as i32 - 1, d as i32, crate::guardian_report::PING_HOUR as i32);
    let schedule = Object::new();
    set(&schedule, "at", &when);
    let notif = Object::new();
    set(&notif, "id", &JsValue::from_f64(GUARDIAN_ID));
    set(&notif, "title", &JsValue::from_str(&p.title));
    set(&notif, "body", &JsValue::from_str(&p.body));
    set(&notif, "schedule", &schedule);
    if let Some(route) = &p.route {
        set(&notif, "actionTypeId", &JsValue::from_str(GUARDIAN_ACTIONS));
        let extra = Object::new();
        set(&extra, "route", &JsValue::from_str(route));
        set(&notif, "extra", &extra);
    }
    let arr = Array::new();
    arr.push(&notif);
    let arg = Object::new();
    set(&arg, "notifications", &arr);
    if let Some(f) = method(&ln, "schedule") {
        let _ = f.call1(&ln, &arg);
    }
}

/// F7: the one action, "Review & add to My Words", foreground only (G-D9: no
/// background action writes anything). Tapping the notification or the
/// action hands its route to `open`, which goes through the router and so
/// through the adult gate. Registered once at boot.
#[cfg(not(feature = "web"))]
pub fn register_guardian_actions(open: impl Fn(String) + 'static) {
    let Some(ln) = plugin() else { return };
    let action = Object::new();
    set(&action, "id", &JsValue::from_str("review"));
    set(&action, "title", &JsValue::from_str(&crate::i18n::t("greport.pingAction")));
    set(&action, "foreground", &JsValue::TRUE);
    let actions = Array::new();
    actions.push(&action);
    let kind = Object::new();
    set(&kind, "id", &JsValue::from_str(GUARDIAN_ACTIONS));
    set(&kind, "actions", &actions);
    let types = Array::new();
    types.push(&kind);
    let arg = Object::new();
    set(&arg, "types", &types);
    if let Some(f) = method(&ln, "registerActionTypes") {
        let _ = f.call1(&ln, &arg);
    }
    let cb = wasm_bindgen::closure::Closure::<dyn Fn(JsValue)>::new(move |ev: JsValue| {
        let route = Reflect::get(&ev, &JsValue::from_str("notification"))
            .and_then(|n| Reflect::get(&n, &JsValue::from_str("extra")))
            .and_then(|e| Reflect::get(&e, &JsValue::from_str("route")))
            .ok()
            .and_then(|r| r.as_string());
        // Only our own route, whatever else an event carries.
        if let Some(r) = route.filter(|r| r.starts_with("#guardian-report")) {
            open(r);
        }
    });
    if let Some(f) = method(&ln, "addListener") {
        let _ = f.call2(&ln, &JsValue::from_str("localNotificationActionPerformed"), cb.as_ref());
    }
    cb.forget();
}

/// CC-YEARBOOK D5 (signed): ONE opt-in local ping when the calendar-year
/// period closes — scheduled on-device for Jan 1, 09:00, id 2, cancelled
/// whenever the parent opts back out. The only time-based prompt in the
/// BD batch.
pub fn yearbook_ping(enabled: bool) {
    let Some(ln) = plugin() else { return };
    if !enabled {
        let ids = Array::new();
        let id_obj = Object::new();
        set(&id_obj, "id", &JsValue::from_f64(2.0));
        ids.push(&id_obj);
        let arg = Object::new();
        set(&arg, "notifications", &ids);
        if let Some(f) = method(&ln, "cancel") {
            let _ = f.call1(&ln, &arg);
        }
        return;
    }
    // One-shot => `at` with a concrete Date (an `on` clause would repeat
    // every year — a nag, not a ping). Jan 1 next year, 09:00 local.
    let year = js_sys::Date::new_0().get_full_year();
    let when = js_sys::Date::new_with_year_month_day_hr(year + 1, 0, 1, 9);
    let schedule = Object::new();
    set(&schedule, "at", &when);
    let notif = Object::new();
    set(&notif, "id", &JsValue::from_f64(2.0));
    set(&notif, "title", &JsValue::from_str(&crate::i18n::t("yb.title")));
    set(&notif, "body", &JsValue::from_str(&crate::i18n::t("yb.ping")));
    set(&notif, "schedule", &schedule);
    let arr = Array::new();
    arr.push(&notif);
    let arg = Object::new();
    set(&arg, "notifications", &arr);
    if let Some(f) = method(&ln, "schedule") {
        let _ = f.call1(&ln, &arg);
    }
}

/// Apply the reminder setting: schedule the daily notification when `enabled`,
/// otherwise cancel any existing one. `time` is "HH:MM" (24h). Best-effort —
/// silently does nothing on the web or if the plugin rejects.
pub fn apply(enabled: bool, time: &str) {
    let Some(ln) = plugin() else { return };

    if !enabled {
        cancel(&ln);
        return;
    }

    let (hour, minute) = parse_hhmm(time);

    // { notifications: [{ id, title, body, schedule: { on: { hour, minute },
    //   allowWhileIdle: true } }] }  — `on` with only hour/minute repeats daily.
    let on = Object::new();
    set(&on, "hour", &JsValue::from_f64(hour as f64));
    set(&on, "minute", &JsValue::from_f64(minute as f64));
    let schedule = Object::new();
    set(&schedule, "on", &on);
    set(&schedule, "allowWhileIdle", &JsValue::TRUE);

    let notif = Object::new();
    set(&notif, "id", &JsValue::from_f64(NOTIF_ID as f64));
    // icon-exempt: a system notification title, drawn by the OS, not the app.
    set(&notif, "title", &JsValue::from_str("Time to spell \u{2728}"));
    set(&notif, "body", &JsValue::from_str("Your daily words are waiting \u{2014} keep the chain going."));
    set(&notif, "schedule", &schedule);

    let arr = Array::new();
    arr.push(&notif);
    let opts = Object::new();
    set(&opts, "notifications", &arr);

    // Ask for permission first (needed on Android 13+ / iOS), then schedule.
    // We schedule regardless of the answer — if denied, the OS just won't
    // display it, and the user can grant it later in system settings.
    let schedule_now = {
        let ln = ln.clone();
        let opts = opts.clone();
        move || {
            if let Some(f) = method(&ln, "schedule") {
                let _ = f.call1(&ln, &opts);
            }
        }
    };

    if let Some(req) = method(&ln, "requestPermissions") {
        if let Ok(p) = req.call0(&ln) {
            if let Ok(promise) = p.dyn_into::<js_sys::Promise>() {
                let fut = wasm_bindgen_futures::JsFuture::from(promise);
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = fut.await;
                    schedule_now();
                });
                return;
            }
        }
    }
    // No permission API available (shouldn't happen on native) — just try.
    schedule_now();
}

fn cancel(ln: &JsValue) {
    let ids = Array::new();
    let one = Object::new();
    set(&one, "id", &JsValue::from_f64(NOTIF_ID as f64));
    ids.push(&one);
    let opts = Object::new();
    set(&opts, "notifications", &ids);
    if let Some(f) = method(ln, "cancel") {
        let _ = f.call1(ln, &opts);
    }
}

/// Parse "HH:MM" into (hour, minute), clamped to valid ranges; defaults to
/// 17:00 on anything malformed.
fn parse_hhmm(time: &str) -> (i32, i32) {
    let mut parts = time.split(':');
    let h = parts.next().and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(17);
    let m = parts.next().and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(0);
    (h.clamp(0, 23), m.clamp(0, 59))
}
