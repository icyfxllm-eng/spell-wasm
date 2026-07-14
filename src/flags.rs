//! Compile-in feature flags for in-progress features. Each flag is a `pub fn`
//! returning a bool; the default is OFF so an unfinished feature ships dark —
//! flag OFF means zero behavioral difference from before the feature landed.
//!
//! Sibling feature branches add their OWN flag fn to this module — keep each
//! flag to a single self-contained function so parallel branches don't collide.
//!
//! QA can flip a flag ON at runtime (no rebuild) via the localStorage key
//! `spell_flags`: a comma-separated list of enabled flag names, e.g.
//! `"syllable_replay"`. Absent/blank key ⇒ every flag takes its coded default.

/// F7 "Syllable replay on misses": the flag-gated "hear it slowly" affordance on
/// the miss / spaced-repetition reveal surface, which replays the word
/// syllable-by-syllable and highlights the current syllable in the revealed
/// spelling. Default **OFF** (App-Store build ships without it).
pub fn syllable_replay() -> bool {
    enabled("syllable_replay", false)
}

/// Resolve a flag: the localStorage `spell_flags` allow-list turns flags ON for
/// QA; when the key is absent or blank the coded `default` applies. Every flag
/// here defaults OFF, so membership-or-default is all we need.
fn enabled(name: &str, default: bool) -> bool {
    match crate::storage::get_raw("spell_flags") {
        Some(list) if !list.trim().is_empty() => list.split(',').any(|f| f.trim() == name),
        _ => default,
    }
}
