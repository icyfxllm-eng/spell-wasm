//! CC-BOARD-GAME v1.1 F3 / D21 -- the answer check.
//!
//! One function, `grade`, composed from primitives that already exist and
//! adding no comparison rule of its own. It mirrors the main game's local
//! path (`game.rs`, the block after the English backend round trip) so a word
//! is right here exactly when it is right there: Mandarin through the pinyin
//! canonicaliser (tone-blind for Spell Jr, surface tones accepted at the first
//! two tiers), Korean through `jamo::grade`, everything else through
//! `norm::answer_matches` (accent-lenient for Spell Jr) plus the data-driven
//! homophone layer. Pure and offline: no server, no DOM, no clock.
//!
//! The `word` argument is the bank ENTRY, which for Mandarin is
//! `pinyin|hanzi`. The engine treats it as opaque; only this file and the
//! screen split it.

use crate::boardgame::Tier;
use crate::consts;

/// The part of a bank entry the player types (and the screen compares).
pub fn citation(entry: &str) -> &str {
    entry.split('|').next().unwrap_or(entry)
}

/// `boardgame::Grader`.
pub fn grade(lang: &str, kid: bool, typed: &str, entry: &str, tier: Tier) -> bool {
    let word = citation(entry);
    if lang == consts::ZH {
        let surface = crate::zh_sandhi::lookup(entry).map(|(s, _)| s);
        let (v, _) = crate::pinyin::grade_sandhi_aware(
            typed,
            word,
            surface,
            crate::pinyin::ToneMode::for_kid(kid),
            crate::pinyin::tier_accepts_surface(tier.name()),
        );
        return v.is_correct();
    }
    if lang == consts::KO {
        return crate::jamo::grade(typed, word).correct;
    }
    crate::norm::answer_matches(typed, word, kid) || crate::homophones::accepts(lang, word, typed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A13 -- per-language fixtures. No network: nothing here can reach one,
    /// and the module imports none.
    #[test]
    fn a13_english_exact_case_and_homophone() {
        assert!(grade("en", false, "Elephant", "elephant", Tier::Medium));
        assert!(!grade("en", false, "elefant", "elephant", Tier::Medium));
        // A real homophone of the prompt (CC-SENSE-CUE I1), and not a typo of it.
        assert!(grade("en", false, "pear", "pair", Tier::Medium));
        assert!(grade("en", false, "there", "their", Tier::Hard));
        assert!(!grade("en", false, "pare ", "pail", Tier::Hard));
    }

    #[test]
    fn a13_accent_folding_is_spell_jr_only() {
        assert!(!grade("fr", false, "cafe", "café", Tier::Medium));
        assert!(grade("fr", true, "cafe", "café", Tier::Medium));
        assert!(grade("fr", false, "café", "café", Tier::Medium));
        assert!(grade("de", true, "strasse", "straße", Tier::Easy));
        assert!(!grade("de", false, "strasse", "straße", Tier::Easy));
    }

    #[test]
    fn a13_zh_tone_sandhi_and_kid() {
        // Tone is graded for everyone but Spell Jr.
        assert!(grade("zh", false, "ba1", "ba1|八", Tier::Easy));
        assert!(!grade("zh", false, "ba2", "ba1|八", Tier::Easy));
        assert!(grade("zh", true, "ba2", "ba1|八", Tier::Easy), "Spell Jr is tone-blind");
        assert!(!grade("zh", true, "bo1", "ba1|八", Tier::Easy), "but not segment-blind");
        // Sandhi: bu4 + 4th tone is spoken bu2. The surface form passes at
        // tiers 1-2 and not at tier 3+; the citation passes everywhere.
        let entry = "bu4bian4|不便";
        assert!(grade("zh", false, "bu4bian4", entry, Tier::Hard));
        assert!(grade("zh", false, "bu2bian4", entry, Tier::Easy));
        assert!(grade("zh", false, "bu2bian4", entry, Tier::Medium));
        assert!(!grade("zh", false, "bu2bian4", entry, Tier::Hard));
        assert!(!grade("zh", false, "bu2bian4", entry, Tier::Expert));
    }

    #[test]
    fn a13_korean_grades_at_jamo_granularity() {
        assert!(grade("ko", false, "가게", "가게", Tier::Easy));
        assert!(!grade("ko", false, "가께", "가게", Tier::Easy));
        assert!(!grade("ko", false, "가", "가게", Tier::Easy));
        assert!(!grade("ko", false, "각게", "가게", Tier::Easy), "a spurious final is wrong");
    }

    #[test]
    fn a13_arabic_tashkeel_is_silent_but_hamza_is_not() {
        assert!(grade("ar", false, "أَب", "أب", Tier::Easy));
        assert!(grade("ar", false, "أب", "أَب", Tier::Easy));
        assert!(!grade("ar", false, "اب", "أب", Tier::Easy), "the hamza seat is spelling");
        assert!(!grade("ar", true, "اب", "أب", Tier::Easy), "and Spell Jr does not forgive it");
    }

    #[test]
    fn a13_the_cited_part_of_a_zh_entry_is_what_is_typed() {
        assert_eq!(citation("ba1|八"), "ba1");
        assert_eq!(citation("cat"), "cat");
    }

    /// The module composes existing primitives only: nothing in it reaches a
    /// network or the DOM.
    #[test]
    fn a13_the_grader_source_names_no_network_or_dom() {
        let src = include_str!("boardgame_grade.rs");
        let code: String = src.split("#[cfg(test)]").next().unwrap().lines().filter(|l| !l.trim_start().starts_with("//")).collect();
        for banned in ["fetch", "spawn_local", "web_sys", "js_sys", "crate::api", "crate::dom"] {
            assert!(!code.contains(banned), "grader uses {banned}");
        }
    }
}
