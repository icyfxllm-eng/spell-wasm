//! CC-SPELL-RACING G-B — stable, content-derived word IDs + per-tier list hash.
//!
//! A racing ghost references words by `(languageCode, wordListHash, wordID)`, never
//! by raw string (spec F1). This module is the foundation that makes that possible,
//! and it is **purely additive**: the word arrays in `word_data.rs` are unchanged.
//!
//! - `word_id(word)` = FNV-1a-64 of `NFC(word)`. Content-derived, so a word's ID
//!   never changes when other words are added/removed by an audit. A fixed, seedless
//!   hash so it is byte-identical to `scripts/build-wordlists.py` (Rust's default
//!   `Hash` is seeded and must NOT be used for a cross-device stable ID).
//! - `list_hash(lang, tier)` = FNV-1a-64 over the tier's ordered word IDs. Matching
//!   hashes ⇒ two devices have the identical word set ⇒ a seeded track reproduces
//!   exactly. The build emits golden values (`word_data::TIER_HASHES`); a test here
//!   pins the Rust recompute against them, catching any Python/Rust drift.
//! - `resolve(lang, tier, id)` returns the word or `None`. The race loader turns
//!   `None` into an abort (spec F1: never substitute a word silently).
//!
//! The Spell Racing mode that consumes this is REVIEW-GATED and not built; this is
//! only Phase 0 (the foundation). Nothing here references Climb/shields (D2).

use std::cell::RefCell;
use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable content-derived ID for a word: FNV-1a-64 of its NFC UTF-8 bytes.
/// Identical to `word_id` in `scripts/build-wordlists.py`.
pub fn word_id(word: &str) -> u64 {
    let mut h = FNV64_OFFSET;
    // word_data words are already NFC, but normalize defensively so an ID computed
    // from arbitrary input (an imported ghost, a decomposed keystroke) still matches.
    let nfc: String = word.nfc().collect();
    for b in nfc.as_bytes() {
        h = (h ^ *b as u64).wrapping_mul(FNV64_PRIME);
    }
    h
}

/// FNV-1a-64 over each word's 8-byte big-endian ID, in slice order.
fn compute_list_hash(words: &[&str]) -> u64 {
    let mut h = FNV64_OFFSET;
    for w in words {
        for b in word_id(w).to_be_bytes() {
            h = (h ^ b as u64).wrapping_mul(FNV64_PRIME);
        }
    }
    h
}

/// The pinned build-time hash for `(lang, tier)`, if that tier has words.
fn pinned(lang: &str, tier: &str) -> Option<u64> {
    crate::word_data::TIER_HASHES
        .iter()
        .find(|(l, t, _)| *l == lang && *t == tier)
        .map(|(_, _, h)| *h)
}

/// The `wordListHash` for `(lang, tier)`. Returns the pinned build-time value (a
/// test proves it equals a fresh Rust recompute) or recomputes for any tier not in
/// the table (e.g. an audit-only bank).
pub fn list_hash(lang: &str, tier: &str) -> u64 {
    pinned(lang, tier).unwrap_or_else(|| compute_list_hash(crate::words::tier_for(lang, tier)))
}

thread_local! {
    // Lazy id -> word map per (lang, tier). Built once, then O(1) resolves.
    static RESOLVE_CACHE: RefCell<HashMap<(String, String), HashMap<u64, &'static str>>> =
        RefCell::new(HashMap::new());
}

/// The word in `(lang, tier)` with this ID, or `None`. `None` is the signal for the
/// caller to abort the race — never to substitute a different word (spec F1).
pub fn resolve(lang: &str, tier: &str, id: u64) -> Option<&'static str> {
    RESOLVE_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        let map = cache
            .entry((lang.to_string(), tier.to_string()))
            .or_insert_with(|| {
                crate::words::tier_for(lang, tier)
                    .iter()
                    .map(|w| (word_id(w), *w))
                    .collect()
            });
        map.get(&id).copied()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_id_deterministic_and_nfc_stable() {
        assert_eq!(word_id("cat"), word_id("cat"));
        assert_ne!(word_id("cat"), word_id("cot"));
        // café composed (NFC) and decomposed (NFD) resolve to the same ID.
        assert_eq!(word_id("caf\u{e9}"), word_id("cafe\u{301}"));
    }

    #[test]
    fn matches_the_pinned_hash_algorithm() {
        // The exact FNV-1a-64 spec, pinned to a known vector so a refactor can't
        // silently change the algorithm (which would break every stored ghost).
        // "" hashes to the offset basis; "a" to the documented FNV-1a value.
        assert_eq!(compute_list_hash(&[]), FNV64_OFFSET);
        assert_eq!(word_id(""), FNV64_OFFSET);
        assert_eq!(word_id("a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn golden_hashes_match_runtime_recompute() {
        // THE drift guard: the build (Python) emitted TIER_HASHES; recompute each in
        // Rust and require equality. A mismatch means the two hash implementations
        // diverged — which would silently break ghost determinism across devices.
        for (lang, tier, golden) in crate::word_data::TIER_HASHES {
            let got = compute_list_hash(crate::words::tier_for(lang, tier));
            assert_eq!(got, *golden, "list_hash drift for {lang}/{tier}");
            assert_eq!(list_hash(lang, tier), *golden);
        }
    }

    #[test]
    fn no_id_collisions_in_any_registered_language() {
        use crate::consts::{BUILTIN_LANGS, TIER_ORDER};
        for (lang, _, _, _) in BUILTIN_LANGS {
            let mut seen: HashMap<u64, &str> = HashMap::new();
            for tier in TIER_ORDER {
                for w in crate::words::tier_for(lang, tier) {
                    if let Some(prev) = seen.insert(word_id(w), w) {
                        assert_eq!(prev, *w, "word-ID collision in {lang}: {prev} vs {w}");
                    }
                }
            }
        }
    }

    #[test]
    fn resolve_round_trips_and_rejects_unknown() {
        let words = crate::words::tier_for("en", "easy");
        assert!(!words.is_empty());
        for w in words {
            assert_eq!(resolve("en", "easy", word_id(w)), Some(*w));
        }
        // An ID not in the list resolves to None — the caller aborts, never
        // substitutes. (0 is not a real word ID here.)
        assert_eq!(resolve("en", "easy", 0), None);
    }
}
