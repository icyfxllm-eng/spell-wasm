//! Kid Mode "friendly words" filter — the content side of Kid Mode the tier cap
//! doesn't cover. When Kid Mode is on, words on the per-language kid-exclusion
//! list (alcohol / weapons / death / adult-context) are dropped from the served
//! pools. This is the age-appropriateness layer on top of the global profanity
//! filter (`profanity.rs`), which screens everyone.
//!
//! Lists live in `assets/words/kid-exclude/{lang}.txt`, auditor-extensible; the
//! current curated pools are already largely clean (English has only `cemetery`),
//! so most lists are seeds — the real value is the gate: any future word (e.g.
//! the Layer-2 East-Asian expansion) is age-filtered before it can reach a kid.
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
        kid_lists!["en", "es", "fr", "de", "pt", "pl", "tr", "vi", "ko", "ja", "zh", "th", "fil"]
    })
}

/// The comparison key for a pool word. Mandarin entries are `"pinyin|hanzi"` —
/// match on the hanzi (what the word actually *is*); everything else matches the
/// word itself.
fn key(word: &str) -> String {
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
        // th has no entries yet → nothing filtered.
        assert!(kid_allowed("th", "\u{0e01}\u{0e1a}")); // กบ
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
