//! "Notify Me" capture for coming-soon languages (language-availability
//! registry). Anonymous per-language interest — a tap count is all the backend
//! needs to rank demand. No email, no account, no PII. Offline-tolerant: taps
//! are queued locally and retried until the backend accepts them.

use std::collections::HashSet;

use crate::storage;

const SET_KEY: &str = "spell_notify_v1"; // langs this device has tapped Notify Me for
const QUEUE_KEY: &str = "spell_notify_queue_v1"; // pending POSTs (delivered on reconnect)
const ID_KEY: &str = "spell_install_id_v1"; // anonymous per-install id (not PII)

fn notified() -> HashSet<String> {
    storage::get_json(SET_KEY).unwrap_or_default()
}

/// True if this device already registered interest in `lang` (persists — drives
/// the confirmed "You're on the list" button state across sessions).
pub fn has(lang: &str) -> bool {
    notified().contains(lang)
}

/// A stable anonymous install id (random, no PII), created once on first use.
fn install_id() -> String {
    if let Some(id) = storage::get_raw(ID_KEY) {
        return id;
    }
    let id = format!("{:016x}", (js_sys::Math::random() * 1e18) as u64);
    storage::set_raw(ID_KEY, &id);
    id
}

/// Record interest in `lang`: mark it confirmed locally, queue a backend POST,
/// and try to flush now. Idempotent per language.
pub fn record(lang: &str) {
    let mut set = notified();
    if set.insert(lang.to_string()) {
        storage::set_json(SET_KEY, &set);
        let mut q: Vec<serde_json::Value> = storage::get_json(QUEUE_KEY).unwrap_or_default();
        q.push(serde_json::json!({ "language": lang, "id": install_id() }));
        storage::set_json(QUEUE_KEY, &q);
    }
    flush();
}

/// Deliver any queued taps to the backend; keep the ones that fail for a later
/// retry. Safe to call on every launch (no-op when the queue is empty).
pub fn flush() {
    let q: Vec<serde_json::Value> = storage::get_json(QUEUE_KEY).unwrap_or_default();
    if q.is_empty() {
        return;
    }
    let url = format!("{}/api/notify", crate::api::api_base());
    wasm_bindgen_futures::spawn_local(async move {
        let mut remaining = Vec::new();
        for item in q {
            if storage::fetch_post_json(&url, &item.to_string()).await.is_err() {
                remaining.push(item); // backend unreachable — keep for next flush
            }
        }
        storage::set_json(QUEUE_KEY, &remaining);
    });
}
