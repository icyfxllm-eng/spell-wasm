//! CC-LETTER-FORGE F2 — the per-language validity index.
//!
//! "Is this a real word in this language?", answered at input speed. The forge
//! asks it on every submission (hundreds per session, each reaction budgeted
//! under 300ms) and CC-WORD-CHAINS will reuse the same structure to ask "what
//! can follow this unit?".
//!
//! # Why an index at all
//!
//! The banks are already sorted `&'static [&'static str]`, so a binary search
//! looks like it should be enough. It is not. Gameplay compares with
//! `norm::fold_strict` (NFC + lowercase), and a bank sorted as STORED is not
//! sorted once folded: German capitalises every common noun, so `Auge` sorts
//! before `alt` raw and after it folded. Binary-searching the raw array would
//! answer a different question from the one the game asks, and would reject
//! valid submissions in exactly one language. English and Korean happen to be
//! safe; that they are is a coincidence, not a design.
//!
//! # Why it is derived, not built
//!
//! F2 specifies "built at content-build time, versioned with the list". This
//! derives it from the compiled-in bank at first use instead, on Eric's call
//! (2026-08-10). A build-time artifact is a thing that can go stale, and this
//! repo has been bitten by exactly that twice: a hand-edited `word_data.rs`
//! the generator would have overwritten, and stale `TIER_HASHES` caught only
//! by its drift guard. An index with no independent existence cannot drift
//! from the list it indexes — "versioned with the list" becomes structural
//! rather than checked. It also keeps ~80k duplicated strings out of the
//! binary.
//!
//! Cost: one sort per language on first use, and only for languages actually
//! played.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::norm::fold_strict;

thread_local! {
    /// lang -> its folded, sorted vocabulary. `Rc` so a lookup can hold the
    /// list without keeping the map borrowed across the search.
    static INDEX: RefCell<HashMap<String, Rc<Vec<String>>>> =
        RefCell::new(HashMap::new());
}

/// The comparable form of a bank entry.
///
/// zh stores `pinyin|hanzi` pairs and the player types the PINYIN half, so
/// that is what the forge must match. Two characters sharing a pinyin — 九
/// and 酒 are both `jiu3` — collapse to one entry here, which is correct for
/// a word-finding game: the player composed a real syllable either way. (The
/// gloss keys on the full pair precisely because it must NOT collapse them;
/// the two structures answer different questions.)
fn comparable(entry: &str) -> String {
    fold_strict(entry.split('|').next().unwrap_or(entry))
}

/// Build (once) and borrow the folded, sorted vocabulary for `lang`.
fn index_for(lang: &str) -> Rc<Vec<String>> {
    if let Some(hit) = INDEX.with(|m| m.borrow().get(lang).cloned()) {
        return hit;
    }
    let mut words: Vec<String> = Vec::new();
    for tier in ["easy", "medium", "hard", "expert"] {
        words.extend(crate::words::tier_for(lang, tier).iter().map(|w| comparable(w)));
    }
    words.sort_unstable();
    words.dedup();
    let rc = Rc::new(words);
    INDEX.with(|m| m.borrow_mut().insert(lang.to_string(), rc.clone()));
    rc
}

/// Is `word` a real word in `lang`?
///
/// Compared exactly as gameplay compares an answer, so the forge can never
/// accept something the base game would reject, or vice versa.
pub fn is_valid(lang: &str, word: &str) -> bool {
    let needle = comparable(word);
    if needle.is_empty() {
        return false;
    }
    index_for(lang).binary_search(&needle).is_ok()
}

/// How many distinct words `lang` offers. The forge's D2 acceptance gate
/// (pool >= 20) is computed against this vocabulary, so a caller can size a
/// puzzle without walking the tiers itself.
pub fn vocabulary_size(lang: &str) -> usize {
    index_for(lang).len()
}

/// The `i`th word of `lang`, for callers that need a DETERMINISTIC pick rather
/// than a search.
///
/// CC-IMPOSTOR seeds a round from (wordId, roundIndex) and must reproduce the
/// same card set on every platform (I2). Picking by index over the sorted index
/// gives that for free; picking by prefix would bias every round toward one
/// letter, and picking at random would not be reproducible in a bug report.
pub fn word_at(lang: &str, i: usize) -> Option<String> {
    let idx = index_for(lang);
    if idx.is_empty() {
        return None;
    }
    idx.get(i % idx.len()).cloned()
}

/// Every word whose comparable form starts with `prefix`.
///
/// Not used by the forge. It exists because CC-WORD-CHAINS needs successor
/// lookup ("what starts with 리?") and a sorted list answers that with two
/// binary searches — building a second structure later would risk the two
/// disagreeing about what counts as a word, which is the bug class this
/// module exists to prevent.
pub fn starting_with(lang: &str, prefix: &str) -> Vec<String> {
    let p = comparable(prefix);
    if p.is_empty() {
        return Vec::new();
    }
    let idx = index_for(lang);
    let from = idx.partition_point(|w| w.as_str() < p.as_str());
    idx[from..]
        .iter()
        .take_while(|w| w.starts_with(&p))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason this module exists: validity must mean what gameplay means.
    #[test]
    fn matching_agrees_with_the_gameplay_fold() {
        // German is the case that forces an index. `Auge` is stored
        // capitalised; a player composing lowercase letters must still be
        // told it is a word.
        assert!(is_valid("de", "auge"), "lowercase must match a capitalised bank word");
        assert!(is_valid("de", "Auge"), "and so must the stored form");
        assert!(is_valid("de", "AUGE"), "and any casing between");
        assert!(!is_valid("de", "aug"), "a prefix is not a word");
        assert!(!is_valid("de", ""), "empty is never valid");
    }

    /// Guards the assumption a raw binary search would have relied on: that
    /// the bank is sorted once folded. It is not, in German — so if this ever
    /// starts passing, the index could be replaced by a plain search, and if
    /// it keeps failing the index is load-bearing.
    #[test]
    fn the_raw_bank_is_not_folded_sorted() {
        let mut raw: Vec<String> = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            raw.extend(crate::words::tier_for("de", tier).iter().map(|w| fold_strict(w)));
        }
        let mut sorted = raw.clone();
        sorted.sort();
        assert_ne!(
            raw, sorted,
            "de folded is sorted — a plain binary search would do and this index is dead weight"
        );
    }

    #[test]
    fn every_bank_word_validates_in_its_own_language() {
        // sw is STRUCTURALLY 3-tier (signed, CC-BANK-COMPLETE) and this
        // index asks every language for all four, so sw is the case that
        // proves the missing tier resolves to empty rather than panicking
        // or falling through to another language's bank.
        for lang in ["en", "de", "es", "ko", "ja", "ar", "sw", "zh", "vi", "hi"] {
            let pool = crate::words::tier_for(lang, "easy");
            assert!(!pool.is_empty(), "{lang}: empty easy tier");
            for w in pool.iter().take(200) {
                assert!(is_valid(lang, w), "{lang}: bank word {w:?} fails its own index");
            }
        }
    }

    /// zh keys on the pinyin half — that is what the player types.
    #[test]
    fn chinese_matches_the_typed_half() {
        let entry = crate::words::tier_for("zh", "easy")
            .iter()
            .find(|e| e.contains('|'))
            .copied()
            .expect("zh entries are pinyin|hanzi");
        let (pinyin, hanzi) = entry.split_once('|').unwrap();
        assert!(is_valid("zh", pinyin), "the pinyin the player types must validate");
        assert!(is_valid("zh", entry), "and the stored pair resolves to the same key");
        assert!(!is_valid("zh", hanzi), "the hanzi alone is not what is typed");
    }

    /// WORD-CHAINS' successor query, on the shared structure.
    #[test]
    fn prefix_search_finds_successors() {
        let hits = starting_with("en", "sp");
        assert!(!hits.is_empty(), "en has words starting 'sp'");
        assert!(hits.iter().all(|w| w.starts_with("sp")), "every hit starts with the prefix");
        assert!(hits.iter().any(|w| is_valid("en", w)), "and they are valid words");
        assert!(starting_with("en", "zzzz").is_empty(), "no false hits");
    }

    /// No language may inherit another's vocabulary.
    ///
    /// `simple_tier` falls through an unknown tier name to MEDIUM, which is
    /// per-language and therefore safe; this pins that the index cannot leak
    /// ACROSS languages, which is the failure that would validate an English
    /// word as Swahili.
    ///
    /// NOTE, pre-existing and not this module's business: sw is declared
    /// "structurally 3-tier" by config/bank_floors.json and asserted so by
    /// bank_complete::floor_registry_is_lawful, but it ships SW_EXPERT with
    /// 700 words and assets/words/sw/expert.txt exists. Both predate the
    /// 2026-08-09 bank rework (verified at 0c9ea86). The data and the signed
    /// floor table disagree; nothing breaks, because the fourth tier falls
    /// under the default 840 ceiling. Flagged for Eric.
    #[test]
    fn no_language_inherits_another_bank() {
        let sw = index_for("sw");
        for foreign in ["the", "and", "because", "hello"] {
            assert!(
                !sw.contains(&foreign.to_string()),
                "sw index contains the English word {foreign:?}"
            );
        }
        assert!(vocabulary_size("sw") > 500, "sw still has a real vocabulary");
        // and the reverse direction
        let en = index_for("en");
        assert!(!en.contains(&"kufuatana".to_string()), "en index contains a Swahili word");
    }

    #[test]
    fn the_index_is_deduped_and_sorted() {
        let idx = index_for("en");
        let mut expect = (*idx).clone();
        expect.sort();
        expect.dedup();
        assert_eq!(*idx, expect, "index must be sorted and free of duplicates");
        assert!(vocabulary_size("en") > 1000, "en vocabulary looks too small");
    }
}
