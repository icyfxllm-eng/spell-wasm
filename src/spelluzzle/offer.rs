//! Which languages and tiers a player is offered (F10, F12, I11).
//!
//! Absent means the row or tier is not shown. It is never shown locked.

use super::types::Tier;

/// F12: never offered in v1, whatever any census says. The generator asserts it.
pub const NEVER: [&str; 5] = ["ko", "zh", "ja", "ar", "hi"];

/// The tiers that are built and verified. Phase B ships Spell Jr and Easy; Medium
/// to Expert wait for the build-time seed verification (they have silent words,
/// which need the validity list the device does not carry).
pub const ENABLED: [Tier; 2] = [Tier::Jr, Tier::Easy];

/// Census outcome (Eric, 2026-10-09): English only.
pub fn language_offered(lang: &str) -> bool {
    !NEVER.contains(&lang) && lang == "en"
}

/// F10: a Spell Jr profile gets Spell Jr boards only; everyone else picks from
/// the standard tiers. Unknown age is Jr, which the caller resolves.
pub fn tiers_for(lang: &str, kid: bool) -> Vec<Tier> {
    if !language_offered(lang) {
        return Vec::new();
    }
    if kid {
        return vec![Tier::Jr];
    }
    ENABLED.iter().copied().filter(|t| *t != Tier::Jr).collect()
}
