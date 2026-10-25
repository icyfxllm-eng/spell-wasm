//! F6 — the one online grader: `boardgame_grade::grade` from the app's own
//! core, compiled natively. The server never compares answers itself.

use spell_wasm::native_api::{self, Tier};

/// The bank's tier name (`easy`..`expert`) as the grader's tier.
pub fn tier(name: &str) -> Option<Tier> {
    Tier::ALL.into_iter().find(|t| t.name() == name)
}

/// Is `typed` a right answer for bank `entry`? `kid` is true for every word in
/// a kid-safe room (accent-lenient, tone-blind for Mandarin, as Spell Jr).
pub fn grade(lang: &str, kid: bool, typed: &str, entry: &str, tier: Tier) -> bool {
    native_api::grade(lang, kid, typed, entry, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_through_to_the_app_grader() {
        let t = tier("easy").unwrap();
        assert!(grade("en", false, "cat", "cat", t));
        assert!(grade("en", false, " CAT ", "cat", t), "case and padding fold as in the app");
        assert!(!grade("en", false, "cta", "cat", t));
        assert!(grade("en", false, "kat", "cat", t), "the app's sound-alike set accepts kat (CC-SENSE-CUE)");
        assert!(grade("fr", true, "cafe", "café", t), "Kid Mode is accent-lenient");
        assert!(!grade("fr", false, "cafe", "café", t), "normal mode is accent-strict");
        assert_eq!(tier("bogus"), None);
    }
}
