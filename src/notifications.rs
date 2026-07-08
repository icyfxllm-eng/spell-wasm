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
