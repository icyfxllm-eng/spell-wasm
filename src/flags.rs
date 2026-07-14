//! FP2 feature flags (Decision D1). Every FP2 feature ships behind its OWN flag
//! in this one module; sibling FP2 branches add their own `pub fn <feature>()`
//! here rather than scattering flag reads across the codebase. A flag is read
//! from localStorage ("spell_flag_<name>": "on"/"off"); when absent it falls
//! back to the compiled-in default. Under `cargo test` the storage read is
//! replaced by a settable in-process override so the gate is unit-testable
//! without a browser.

#[cfg(not(test))]
use crate::storage;

/// Resolve a flag from an optional stored override string against a default.
/// Pure — the unit-tested core. "on"/"1"/"true" → true, "off"/"0"/"false" →
/// false, anything else (including absent) → `default`.
fn resolve(stored: Option<&str>, default: bool) -> bool {
    match stored {
        Some("on") | Some("1") | Some("true") => true,
        Some("off") | Some("0") | Some("false") => false,
        _ => default,
    }
}

#[cfg(not(test))]
fn stored(name: &str) -> Option<String> {
    storage::get_raw(&format!("spell_flag_{name}"))
}

#[cfg(test)]
thread_local! {
    static TEST_OVERRIDE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn stored(_name: &str) -> Option<String> {
    TEST_OVERRIDE.with(|c| c.borrow().clone())
}

/// Test-only: force the next flag read (`Some("on")`/`Some("off")`) or clear
/// (`None`) so both branches of a flag-gated path are reachable in unit tests.
#[cfg(test)]
pub fn set_test_override(v: Option<&str>) {
    TEST_OVERRIDE.with(|c| *c.borrow_mut() = v.map(|s| s.to_string()));
}

/// F3 — Home-screen widgets (WidgetKit). Default ON per Decision D1 (the
/// low-risk retention pair with the ghost). When OFF, `crate::widgets::sync`
/// short-circuits and NOTHING is written to the App Group container — the
/// feature has ZERO behavioral effect.
pub fn widgets() -> bool {
    resolve(stored("widgets").as_deref(), true)
}

/// F4 — Siri / App Intents. Default OFF per Decision D1.
///
/// HONEST SEMANTICS (see the PR): this flag does NOT gate the deep-link router
/// (`crate::deeplink`), which is always live and harmless. Nor can it stop iOS
/// from registering the app's `AppShortcuts` — those are compiled into the app
/// binary and iOS indexes them at install time regardless of any runtime flag.
/// So OFF means the feature is "dark": we don't ADVERTISE the intents (no in-app
/// promotion / tip surfacing them), even though a user who already knows the
/// phrase could still invoke them. Flip ON when we're ready to promote Siri entry.
pub fn app_intents() -> bool {
    resolve(stored("app_intents").as_deref(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_semantics() {
        assert!(resolve(Some("on"), false));
        assert!(resolve(Some("1"), false));
        assert!(resolve(Some("true"), false));
        assert!(!resolve(Some("off"), true));
        assert!(!resolve(Some("0"), true));
        assert!(!resolve(Some("false"), true));
        // Absent or unrecognized → the compiled-in default.
        assert!(resolve(None, true));
        assert!(!resolve(None, false));
        assert!(resolve(Some("garbage"), true));
        assert!(!resolve(Some("garbage"), false));
    }

    #[test]
    fn widgets_default_on_and_overridable() {
        set_test_override(None);
        assert!(widgets(), "D1: widgets defaults ON");
        set_test_override(Some("off"));
        assert!(!widgets());
        set_test_override(Some("on"));
        assert!(widgets());
        set_test_override(None);
    }

    #[test]
    fn app_intents_default_off_and_overridable() {
        set_test_override(None);
        assert!(!app_intents(), "D1: app_intents defaults OFF (dark)");
        set_test_override(Some("on"));
        assert!(app_intents());
        set_test_override(Some("off"));
        assert!(!app_intents());
        set_test_override(None);
    }
}
