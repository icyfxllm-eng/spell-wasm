//! Feature flags. A flag resolves as: test override (set by unit/E2E harnesses)
//! -> stored value (localStorage) -> compiled default. Flags ship DARK — the
//! default is OFF and stays OFF in production until a stored value flips it, so
//! adding a flag is a zero-diff change to the shipped experience.
//!
//! `attempts_shields` gates CC-ATTEMPTS-SHIELDS (the extra-attempts toggle in
//! normal mode + shields in The Climb). While it is OFF: the settings toggle is
//! hidden, no shield/attempt logic runs, no HUD renders — byte-for-byte the
//! current build.

use std::cell::RefCell;

thread_local! {
    /// Per-flag test override. `Some(v)` forces the flag to `v` regardless of
    /// stored/default; `None` falls through. Only tests and the E2E seam set
    /// this — production never does.
    static ATTEMPTS_SHIELDS_OVERRIDE: RefCell<Option<bool>> = const { RefCell::new(None) };
}

const ATTEMPTS_SHIELDS_KEY: &str = "spell_flag_attempts_shields_v1";

/// Resolve the CC-ATTEMPTS-SHIELDS flag. Test override wins; otherwise a stored
/// "1" enables it; otherwise the compiled default (OFF).
pub fn attempts_shields() -> bool {
    if let Some(v) = ATTEMPTS_SHIELDS_OVERRIDE.with(|c| *c.borrow()) {
        return v;
    }
    stored_attempts_shields()
}

/// The persisted value only (ignores any test override). Absent/anything but
/// "1" reads as OFF.
pub fn stored_attempts_shields() -> bool {
    crate::storage::get_raw(ATTEMPTS_SHIELDS_KEY).as_deref() == Some("1")
}

/// Persist the flag (owner/QA switch; there is no player-facing control). Kept
/// so the dark flag can be flipped for a build without a recompile.
#[allow(dead_code)]
pub fn set_attempts_shields(on: bool) {
    crate::storage::set_raw(ATTEMPTS_SHIELDS_KEY, if on { "1" } else { "0" });
}

/// Force the flag for the duration of a test (or E2E run). `None` clears it.
pub fn set_test_override(v: Option<bool>) {
    ATTEMPTS_SHIELDS_OVERRIDE.with(|c| *c.borrow_mut() = v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off_and_override_wins() {
        set_test_override(None);
        // No window in native tests -> stored is None -> default OFF.
        assert!(!attempts_shields());
        set_test_override(Some(true));
        assert!(attempts_shields());
        set_test_override(Some(false));
        assert!(!attempts_shields());
        set_test_override(None);
    }
}
