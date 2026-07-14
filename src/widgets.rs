//! F3 — Home-screen widget state bridge.
//!
//! Reads the ALREADY-COMPUTED daily streak + daily-challenge record (this module
//! NEVER recomputes streak rules — it only READS `crate::daily`) and pushes a
//! small JSON snapshot to the native iOS App Group container, where the
//! WidgetKit extension — and, later, the F4 App Intents — read it. Off iOS every
//! call no-ops through the `window.SpellNativeLang` bridge. Gated by
//! `crate::flags::widgets()` (D1: default ON); when OFF nothing is written, so
//! the feature is a zero-diff no-op.
//!
//! # App Group key schema (CONTRACT — F4 READS THE SAME BLOB)
//!
//! - Suite / App Group: `group.net.spellgame.app`
//! - Key: `widget_state_v1`
//! - Value: a JSON string encoding [`WidgetState`] (fields are **camelCase**):
//!
//! ```json
//! {
//!   "schemaVersion": 1,
//!   "streak": 5,                       // consecutive-DAY daily streak (Record.streak)
//!   "bestStreak": 12,                  // Record.best_streak
//!   "lastCompletedDate": "2026-07-13", // YYYY-MM-DD of the last finished daily ("" if never)
//!   "dailyStatus": "done",             // "not_started" | "in_progress" | "done", as of dailyDate
//!   "dailyDate": "2026-07-13",         // the local date dailyStatus refers to
//!   "language": "en",                  // app language code
//!   "updatedAtMs": 1720000000000.0     // wall-clock ms when written
//! }
//! ```
//!
//! Readers derive "practiced today" as `lastCompletedDate == <today>` and treat
//! `dailyStatus` as authoritative ONLY when `dailyDate == <today>` (else the day
//! rolled over and today's daily is "not started"). This keeps the widget/app
//! correct across a midnight rollover without any extra write.

use serde::{Deserialize, Serialize};

/// Schema version stamped into every snapshot (bump on any breaking field change).
pub const SCHEMA_VERSION: u32 = 1;

/// Daily-challenge status values (the `dailyStatus` field).
pub const STATUS_NOT_STARTED: &str = "not_started";
pub const STATUS_IN_PROGRESS: &str = "in_progress";
pub const STATUS_DONE: &str = "done";

/// The snapshot mirrored to the App Group container. See the module docs for the
/// on-disk key/schema contract that F4 also reads.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WidgetState {
    pub schema_version: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub last_completed_date: String,
    pub daily_status: String,
    pub daily_date: String,
    pub language: String,
    pub updated_at_ms: f64,
}

/// Build a snapshot from an already-computed daily [`Record`](crate::daily::Record).
/// Pure (no clock, no bridge) so it is unit-testable; the caller supplies today's
/// local date and the wall-clock stamp.
pub fn build(
    rec: &crate::daily::Record,
    today: &str,
    lang: &str,
    status: &str,
    now_ms: f64,
) -> WidgetState {
    WidgetState {
        schema_version: SCHEMA_VERSION,
        streak: rec.streak,
        best_streak: rec.best_streak,
        last_completed_date: rec.last_completed.clone(),
        daily_status: status.to_string(),
        daily_date: today.to_string(),
        language: lang.to_string(),
        updated_at_ms: now_ms,
    }
}

/// Push `state` to the native container IF `flags::widgets()` is on. Returns
/// whether a sync was emitted: `false` when the flag is OFF (the zero-diff path),
/// `true` otherwise. The underlying bridge write is itself a no-op off iOS.
pub fn sync(state: &WidgetState) -> bool {
    if !crate::flags::widgets() {
        return false;
    }
    // The bridge reaches `window.SpellNativeLang`, which only exists on the wasm
    // build; gating keeps native `cargo test` off the wasm-only imports.
    #[cfg(target_arch = "wasm32")]
    if let Ok(json) = serde_json::to_string(state) {
        crate::native_lang::sync_widget_state(&json);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = state;
    true
}

/// Convenience for the single call site (`game::finish_daily`): snapshot the
/// freshly-recorded streak + today's DONE status and push it. Reads the persisted
/// daily record — call this AFTER `daily::record_result` has saved.
///
/// `js_sys::Date::now()` is wasm-only at runtime but compiles everywhere; this
/// function is the wasm glue and is intentionally NOT exercised by unit tests
/// (the pure `build` + `sync` above are).
pub fn sync_after_daily(lang: &str) {
    let rec = crate::daily::load();
    let today = crate::daily::today();
    let state = build(&rec, &today, lang, STATUS_DONE, js_sys::Date::now());
    sync(&state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daily::Record;

    fn sample_record() -> Record {
        let mut r = Record {
            last_completed: "2026-07-13".to_string(),
            streak: 5,
            best_streak: 12,
            ..Default::default()
        };
        r.history.insert("2026-07-13".to_string(), 8);
        r
    }

    #[test]
    fn build_snapshots_the_record() {
        let s = build(&sample_record(), "2026-07-13", "en", STATUS_DONE, 42.0);
        assert_eq!(s.schema_version, SCHEMA_VERSION);
        assert_eq!(s.streak, 5);
        assert_eq!(s.best_streak, 12);
        assert_eq!(s.last_completed_date, "2026-07-13");
        assert_eq!(s.daily_status, "done");
        assert_eq!(s.daily_date, "2026-07-13");
        assert_eq!(s.language, "en");
        assert_eq!(s.updated_at_ms, 42.0);
    }

    #[test]
    fn serializes_to_the_camelcase_contract() {
        let s = build(&sample_record(), "2026-07-13", "es", STATUS_DONE, 1720000000000.0);
        let json = serde_json::to_string(&s).unwrap();
        // The exact keys the Swift `SpellWidgetState` Codable + F4 decode.
        for key in [
            "\"schemaVersion\"",
            "\"streak\"",
            "\"bestStreak\"",
            "\"lastCompletedDate\"",
            "\"dailyStatus\"",
            "\"dailyDate\"",
            "\"language\"",
            "\"updatedAtMs\"",
        ] {
            assert!(json.contains(key), "missing contract key {key} in {json}");
        }
        // Round-trips cleanly (guards the F4 decode side).
        let back: WidgetState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn sync_off_flag_emits_nothing() {
        crate::flags::set_test_override(Some("off"));
        let s = build(&sample_record(), "2026-07-13", "en", STATUS_DONE, 0.0);
        assert!(!sync(&s), "flag OFF must be a no-op (no sync emitted)");
        crate::flags::set_test_override(None);
    }

    #[test]
    fn sync_on_flag_emits() {
        // Default (None) is ON per D1. `sync` returns true; the bridge write
        // itself is a no-op off iOS / in tests (no window), which is fine.
        crate::flags::set_test_override(Some("on"));
        let s = build(&sample_record(), "2026-07-13", "en", STATUS_DONE, 0.0);
        assert!(sync(&s), "flag ON must emit a sync");
        crate::flags::set_test_override(None);
    }
}
