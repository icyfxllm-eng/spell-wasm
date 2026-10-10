//! Which languages and tiers a player is offered (F10, F12, I11).
//!
//! Absent means the row or tier is not shown. It is never shown locked.

use super::types::Tier;

/// F12: never offered in v1, whatever any census says. The generator asserts it.
pub const NEVER: [&str; 5] = ["ko", "zh", "ja", "ar", "hi"];

/// Every tier is built. Medium to Expert have silent words, so a tier is shown only
/// while `seeds::usable` says its verified seeds match this bank (the screen asks).
pub const ENABLED: [Tier; 5] = Tier::ALL;

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

/// F22: the board types a start screen can offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardType {
    Classic,
    Par,
}

impl BoardType {
    pub fn name(self) -> &'static str {
        match self {
            BoardType::Classic => "classic",
            BoardType::Par => "par",
        }
    }
    pub fn from_name(s: &str) -> Option<BoardType> {
        [BoardType::Classic, BoardType::Par].into_iter().find(|b| b.name() == s)
    }
}

/// F15 / D48: Par is Hard and Expert, never in Spell Jr, and only for a language that is offered.
/// (Whether verified boards exist for the tier is `seeds::par_records`, which the screen asks.)
pub fn par_offered(lang: &str, kid: bool, tier: Tier) -> bool {
    !kid && language_offered(lang) && super::par::offered_tier(tier)
}
