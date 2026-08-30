//! CC-RUSSIAN-STRESS v3 I8 — entry identity.
//!
//! Three persisted stores keyed on the bare spelling, each with its own copy of
//! the same line: `format!("{}::{}", lang, word.to_lowercase())` in
//! `misses::miss_key`, `wordstats::norm_key` and `tone_drill::key_of`. Three
//! copies of one idea is how the idea drifts.
//!
//! v3 F6 resolves entry identity to `(languageCode, canonicalForm,
//! senseDiscriminator)`, so a homograph -- за́мок castle and замо́к lock, one
//! spelling, two words -- can hold two definitions. Without a discriminator in
//! the key, those two senses share one miss record, one stats row and one tone
//! drill: get *castle* wrong and *lock* enters your misses.
//!
//! ZERO-MIGRATION BY CONSTRUCTION. Sense 0 emits the EXACT string the three
//! stores already write, byte for byte. Every record in the field keeps
//! working, no migration runs, and nothing can be orphaned by one. Only a real
//! homograph -- sense > 0, which nothing can create yet -- gets a suffix. The
//! cheapest correct migration is the one that does not happen.

/// The identity of a bank entry, as a store key.
///
/// `sense` is v3's `senseDiscriminator`: 0 for every entry that is the only
/// word with its spelling, non-zero only where a genuine collision exists.
pub fn word_id(lang: &str, word: &str, sense: u8) -> String {
    let base = format!("{}::{}", lang, word.to_lowercase());
    if sense == 0 {
        base // byte-identical to what the stores already persist
    } else {
        format!("{base}#{sense}")
    }
}

/// The common case: the only entry with this spelling.
pub fn word_id0(lang: &str, word: &str) -> String {
    word_id(lang, word, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole reason this is safe to land: sense 0 must reproduce the
    /// legacy key exactly, or every miss record in the field is orphaned.
    #[test]
    fn sense_zero_is_byte_identical_to_the_legacy_key() {
        for (lang, word) in [
            ("ru", "замок"), ("ru", "Замок"), ("en", "apple"),
            ("zh", "hai2zi5"), ("en", "PEAR"), ("ja", "すき"),
        ] {
            let legacy = format!("{}::{}", lang, word.to_lowercase());
            assert_eq!(word_id(lang, word, 0), legacy, "{lang}/{word} would orphan its records");
            assert_eq!(word_id0(lang, word), legacy);
        }
    }

    /// A homograph must NOT collide with its own other sense, nor with the
    /// legacy key -- that separation is the entire point of the discriminator.
    #[test]
    fn senses_are_distinct_from_each_other_and_from_legacy() {
        let a = word_id("ru", "замок", 0);
        let b = word_id("ru", "замок", 1);
        let c = word_id("ru", "замок", 2);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
        assert!(b.starts_with(&a), "a sense must extend its base key, not replace it");
    }

    /// Case folding matches the stores it replaces: answers are compared
    /// case-insensitively, so identity must be too.
    #[test]
    fn identity_is_case_insensitive_like_the_grader() {
        assert_eq!(word_id0("ru", "Замок"), word_id0("ru", "замок"));
        assert_eq!(word_id0("en", "Apple"), word_id0("en", "APPLE"));
    }

    /// Language scoping survives: `sol` in Spanish and `sol` in French are
    /// different entries, which is why the stores carried lang in the key.
    #[test]
    fn language_still_scopes_identity() {
        assert_ne!(word_id0("es", "sol"), word_id0("fr", "sol"));
    }
}
