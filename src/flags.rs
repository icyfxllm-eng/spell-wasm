//! Runtime feature flags — the one place a shipped-dark feature is switched on.
//!
//! Each flag is a `pub fn -> bool`, read from localStorage (`spell_flag_<name>`
//! = "on"/"off"/…); when absent it falls back to the compiled-in default. A flag
//! being OFF must be a true no-op: the feature registers nothing, wires nothing,
//! renders nothing — zero observable diff. Under `cargo test` the storage read is
//! replaced by a settable in-process override so gates are unit-testable.
//!
//! This is the UNIFIED Feature Pack 2 module (the sibling branches each authored
//! their own copy; this reconciles them, keeping F6's testable design). Defaults
//! below are set for the FP2 **integration / TestFlight build**: F1, F2, F6, F7
//! are ON so testers exercise them; F5 word-stories stays OFF (its CC BY-SA
//! attribution gate is unresolved). For the App Store these revert to OFF per PR.

#[cfg(not(test))]
use crate::storage;

/// Pure flag resolver (unit-tested). "on"/"1"/"true"/"yes" → true,
/// "off"/"0"/"false"/"no" → false, anything else (incl. absent) → `default`.
fn resolve(stored: Option<&str>, default: bool) -> bool {
    match stored {
        Some("on") | Some("1") | Some("true") | Some("yes") => true,
        Some("off") | Some("0") | Some("false") | Some("no") => false,
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

/// F6 "Ghost racing in The Climb" — race your best local run. Cross-platform.
pub fn ghost_racing() -> bool {
    resolve(stored("ghost_racing").as_deref(), true)
}

/// F7 "Syllable replay on misses" — the "hear it slowly" affordance on the
/// reveal / spaced-repetition surface (es-only).
pub fn syllable_replay() -> bool {
    resolve(stored("syllable_replay").as_deref(), true)
}

/// F2 "Say It" — on-device pronunciation practice; iOS-only at runtime, and
/// hard-disabled in Kid Mode regardless of this flag (COPPA).
pub fn say_it() -> bool {
    resolve(stored("say_it").as_deref(), true)
}

/// F1 "Photo-to-word-list" — VisionKit OCR of a handout; iOS-only, hidden in Kid Mode.
pub fn photo_list() -> bool {
    resolve(stored("photo_list").as_deref(), true)
}

/// F5 "Word stories" — etymology cards. **Default OFF**: dark until the CC BY-SA
/// attribution approach is approved.
pub fn word_stories() -> bool {
    resolve(stored("word_stories").as_deref(), false)
}

/// Online "Spell Off" (async 1v1, server-owned seed — see online_spelloff.rs +
/// backend/matches.py). **Default OFF**: the /api/match backend must be deployed
/// to the Pi before this can function, so it ships dark until then.
pub fn online_spelloff() -> bool {
    resolve(stored("online_spelloff").as_deref(), false)
}

/// Feature "Spell It Out Loud" — voice spelling INPUT (a mic beside the answer
/// field; the player speaks letter names "C… A… T" and the parser produces the
/// same string a keyboard would). **Default ON for the v2 TestFlight QA pass**
/// (Eric, 2026-07-15) so the on-device mic is testable without a localStorage
/// flag. Still iOS-only at runtime and hidden until the per-language voiceSpell
/// flag + on-device speech availability both hold, so on the web/Android it stays
/// a no-op. Override off with `localStorage['spell_flag_spell_aloud'] = '0'`.
pub fn spell_aloud() -> bool {
    resolve(stored("spell_aloud").as_deref(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_semantics() {
        assert!(resolve(Some("on"), false));
        assert!(resolve(Some("1"), false));
        assert!(!resolve(Some("off"), true));
        assert!(!resolve(Some("0"), true));
        assert!(resolve(None, true));
        assert!(!resolve(None, false));
        assert!(resolve(Some("garbage"), true));
    }

    #[test]
    fn defaults_match_the_integration_build() {
        set_test_override(None);
        assert!(ghost_racing() && syllable_replay() && say_it() && photo_list());
        assert!(!word_stories(), "F5 ships dark");
        set_test_override(Some("off"));
        assert!(!ghost_racing());
        set_test_override(None);
    }
}
