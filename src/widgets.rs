//! CC-IOS-SURFACES (BD-1) — the app side of the one-way widget snapshot
//! (D2, decided): the app writes on meaningful state change; the widget
//! extension only ever reads. Values are audited-pool endonyms plus
//! numbers — no free-composed text crosses the App Group. A no-op
//! anywhere the native plugin is absent (web, Android, simulator).
//! CC-CALENDAR D5 (signed): goal-ring fields are reserved in the schema
//! now so the calendar wave is a data change, not a schema bump.

use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    schema: u32,
    streak: u32,
    daily_done: bool,
    daily_lang: String,
    shields_earned: u32,
    shields_total: u32,
    entitled_langs: Vec<String>,
    // CC-CALENDAR D5 reservations — zero until the calendar wave writes them.
    goal_progress: u32,
    goal_target: u32,
    planned_today: u32,
}

pub fn write_snapshot(s: &crate::model::AppState) {
    let daily_lang = crate::daily::locale_for(&s.lang);
    let endonym = crate::words::LANGUAGES
        .iter()
        .find(|(c, _)| *c == daily_lang)
        .map(|(_, info)| info.name.to_string())
        .unwrap_or_default();
    let ent = crate::entitlements::resolve_entitlements(false, &[], false);
    let entitled: Vec<String> = crate::consts::BUILTIN_LANGS
        .iter()
        .filter(|(c, _, st, _)| {
            matches!(st, crate::consts::LangStatus::Active)
                && ent.lang_level(c) != crate::entitlements::AccessLevel::None
        })
        .map(|(c, _, _, _)| c.to_string())
        .collect();
    let snap = Snapshot {
        schema: 1,
        streak: s.streak,
        daily_done: crate::daily::is_done_today(),
        daily_lang: endonym,
        shields_earned: s.aids.shields.min(9) as u32,
        shields_total: 5,
        entitled_langs: entitled,
        goal_progress: crate::calendar::active_goal(&s.lang)
            .map(|g| crate::calendar::goal_progress(&s.lang, &g).min(g.target))
            .unwrap_or(0),
        goal_target: crate::calendar::active_goal(&s.lang).map(|g| g.target).unwrap_or(0),
        planned_today: crate::calendar::planned_for(&s.lang, (js_sys::Date::now() / 86_400_000.0) as u32).len() as u32,
    };
    let Ok(json) = serde_json::to_string(&snap) else { return };
    call_native(&json);
}

fn call_native(json: &str) {
    let Some(win) = web_sys::window() else { return };
    let Ok(cap) = js_sys::Reflect::get(&win, &JsValue::from_str("Capacitor")) else { return };
    if cap.is_undefined() {
        return;
    }
    let Ok(plugins) = js_sys::Reflect::get(&cap, &JsValue::from_str("Plugins")) else { return };
    let Ok(kit) = js_sys::Reflect::get(&plugins, &JsValue::from_str("NativeLanguageKit")) else { return };
    if kit.is_undefined() {
        return;
    }
    let Ok(f) = js_sys::Reflect::get(&kit, &JsValue::from_str("writeWidgetSnapshot")) else { return };
    let Ok(f) = f.dyn_into::<js_sys::Function>() else { return };
    let arg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&arg, &JsValue::from_str("json"), &JsValue::from_str(json));
    let Ok(promise) = f.call1(&kit, &arg) else { return };
    // The snapshot is fire-and-forget (D2: state flows one way), but the
    // promise was DROPPED, so a rejection had no handler and became an
    // unhandled rejection. writeWidgetSnapshot rejects wherever the app group
    // container is missing -- the simulator, or a build without the
    // entitlement -- once per snapshot, so a Mandarin session logged five in
    // five rounds. Telemetry counted six before this was found (stack hash
    // 1c43291052885d44, first seen 2026-09-19). Awaiting and discarding the
    // result keeps the failure as harmless as it always was, and silent.
    if let Ok(promise) = promise.dyn_into::<js_sys::Promise>() {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
    }
}
