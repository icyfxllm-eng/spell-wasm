//! Kid Mode "friendly words" filter — the content side of Kid Mode the tier cap
//! doesn't cover. When Kid Mode is on, words on the per-language kid-exclusion
//! list (alcohol / weapons / death / adult-context) are dropped from the served
//! pools. This is the age-appropriateness layer on top of the global profanity
//! filter (`profanity.rs`), which screens everyone.
//!
//! Lists live in `assets/words/kid-exclude/{lang}.txt`, auditor-extensible.
//! Most lists are seeds and the real value is the gate: any future word (e.g.
//! the Layer-2 East-Asian expansion) is age-filtered before it can reach a kid.
//!
//! English is no longer nearly-clean. Eric added the death verbs on
//! 2026-09-28 — dead, death, deaths, die, died, dies, kill, killed, killing —
//! and every one of them IS in the bank, across easy, medium and hard, so
//! these nine actually change what a child is served rather than seeding a
//! list. They stay in the bank for adult play; this filter is serve-time
//! only, the same mechanism `blood` and `grave` already used.
//!
//! Matching is case/accent-insensitive (lenient fold), so list entries catch
//! their diacritic/case variants.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::norm::fold_lenient;

macro_rules! kid_lists {
    ($($code:literal),* $(,)?) => {{
        let mut m: HashMap<&'static str, HashSet<String>> = HashMap::new();
        $(
            m.insert($code, parse(include_str!(concat!("../assets/words/kid-exclude/", $code, ".txt"))));
        )*
        m
    }};
}

fn parse(s: &str) -> HashSet<String> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(fold_lenient)
        .collect()
}

fn lists() -> &'static HashMap<&'static str, HashSet<String>> {
    static L: OnceLock<HashMap<&'static str, HashSet<String>>> = OnceLock::new();
    L.get_or_init(|| {
        // CC-LINEUP-SWAP: it/nl/sv/nb cut (lists archived under
        // `archive/wordlists/kid-exclude/`). ru/ar/fa/ur have no kid-exclusion
        // list yet — their word lists are CC-NEW-LANG-CONTENT's scope, and a
        // language with no list simply has an empty set here (the gate still
        // runs; it just has nothing to drop).
        kid_lists!["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "zh", "fil"]
    })
}

/// The comparison key for a pool word. Mandarin entries are `"pinyin|hanzi"` —
/// match on the hanzi (what the word actually *is*); everything else matches the
/// word itself.
fn key(word: &str) -> String {
    // zh-ok(grading): matches the hanzi half against the kid-safety list; never a verdict
    fold_lenient(word.rsplit('|').next().unwrap_or(word))
}

/// True if `word` may be served to a kid in `lang` (not on the exclusion list).
/// Unknown language or empty list → always allowed.
pub fn kid_allowed(lang: &str, word: &str) -> bool {
    match lists().get(lang) {
        Some(set) if !set.is_empty() => !set.contains(&key(word)),
        _ => true,
    }
}

/// Drop kid-excluded words from `pool` for `lang`. Never returns empty from a
/// non-empty input — if the list somehow excluded everything (a data error),
/// the unfiltered pool is kept, because a word on screen beats a stuck game.
pub fn filter_kid(lang: &str, pool: Vec<String>) -> Vec<String> {
    let filtered: Vec<String> = pool.iter().filter(|w| kid_allowed(lang, w)).cloned().collect();
    if filtered.is_empty() {
        pool
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nine death verbs Eric added on 2026-09-28. Pinned by name because
    /// each one is a real bank word -- `die` and `kill` are in easy.txt, which
    /// is where Spell Jr plays -- so a silent regression here puts them back
    /// in front of a child.
    #[test]
    fn the_death_verbs_never_reach_a_child() {
        for w in ["dead", "death", "deaths", "die", "died", "dies", "kill", "killed", "killing"] {
            assert!(!kid_allowed("en", w), "{w} must not be served in Kid Mode");
            assert!(!kid_allowed("en", &w.to_uppercase()), "{w} must be caught case-insensitively");
        }
        // Words that merely contain them are untouched: this is a whole-word
        // list, not a substring ban, or `diet`, `skilled` and `deadline` would
        // vanish with them.
        for w in ["diet", "skilled", "deadline", "diesel"] {
            assert!(kid_allowed("en", w), "{w} is benign and must stay");
        }
    }

    #[test]
    fn cemetery_is_excluded_for_english() {
        assert!(!kid_allowed("en", "cemetery"));
        assert!(!kid_allowed("en", "Cemetery")); // case-insensitive
        assert!(kid_allowed("en", "rhythm")); // benign word stays
    }

    #[test]
    fn accent_variants_are_caught() {
        // fr list has "cimetière"; a de-accented "cimetiere" must still match.
        assert!(!kid_allowed("fr", "cimetière"));
        assert!(!kid_allowed("fr", "cimetiere"));
    }

    #[test]
    fn empty_list_allows_everything() {
        // A language with no kid list → nothing filtered.
        assert!(kid_allowed("xx", "hello"));
    }

    #[test]
    fn mandarin_matches_on_hanzi() {
        // A "pinyin|hanzi" entry is keyed by its hanzi.
        // (zh list is empty by default, so seed a direct key check instead.)
        assert_eq!(key("lao3shi1|\u{8001}\u{5e08}"), fold_lenient("\u{8001}\u{5e08}"));
    }

    #[test]
    fn filter_never_empties_a_pool() {
        let pool = vec!["cemetery".to_string()]; // the only word, and it's excluded
        assert_eq!(filter_kid("en", pool.clone()), pool); // kept — game must have a word
    }

    #[test]
    fn filter_drops_excluded_keeps_rest() {
        let pool = vec!["cat".to_string(), "cemetery".to_string(), "dog".to_string()];
        assert_eq!(filter_kid("en", pool), vec!["cat".to_string(), "dog".to_string()]);
    }
}
