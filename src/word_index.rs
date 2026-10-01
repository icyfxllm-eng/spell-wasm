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
    /// lang -> its vocabulary as `(folded, bank form)`, sorted by the folded
    /// half. `Rc` so a lookup can hold the list without keeping the map
    /// borrowed across the search.
    ///
    /// CC-SNAP-CLEAN C4 added the second half. The index used to store the
    /// folded form alone, which answers "is this a word?" but throws away the
    /// bank's own casing at build time — so nothing could tell a photographed
    /// "CAT" that the bank writes it "cat", or that German "Hund" keeps its
    /// capital. One index still, per Eric's choice (a) of three: the
    /// alternative was a scan, and a second structure is the bug class this
    /// module exists to prevent.
    static INDEX: RefCell<HashMap<String, Rc<Vec<(String, String)>>>> =
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
    // zh-ok(grading): builds the sorted bank index for Forge; never sees typed input
    fold_strict(entry.split('|').next().unwrap_or(entry))
}

/// The form a player reads and types: the bank entry, minus zh's `|hanzi`
/// half, with its own casing intact.
fn bank_form(entry: &str) -> String {
    // zh-ok(grading): the same left-hand split comparable() makes, unfolded
    entry.split('|').next().unwrap_or(entry).to_string()
}

/// Build (once) and borrow the sorted vocabulary for `lang`.
fn index_for(lang: &str) -> Rc<Vec<(String, String)>> {
    if let Some(hit) = INDEX.with(|m| m.borrow().get(lang).cloned()) {
        return hit;
    }
    let mut words: Vec<(String, String)> = Vec::new();
    for tier in ["easy", "medium", "hard", "expert"] {
        words.extend(
            crate::words::tier_for(lang, tier).iter().map(|w| (comparable(w), bank_form(w))),
        );
    }
    words.sort_unstable();
    // Dedupe on the FOLDED half only: two bank entries that fold together are
    // one word to every caller here, and keeping both would make `word_at`
    // return the same word twice under different casings.
    words.dedup_by(|a, b| a.0 == b.0);
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
    index_for(lang).binary_search_by(|e| e.0.as_str().cmp(needle.as_str())).is_ok()
}

/// The bank's own form of `word`, or `None` when the bank does not have it.
///
/// CC-SNAP-CLEAN F5/D2: a photographed word that the bank knows is rewritten
/// in the bank's casing, because worksheet title-casing is layout rather than
/// spelling. A word the bank does not know keeps whatever the camera saw —
/// which is why this returns an Option and not a lossy String.
pub fn canonical(lang: &str, word: &str) -> Option<String> {
    let needle = comparable(word);
    if needle.is_empty() {
        return None;
    }
    let idx = index_for(lang);
    idx.binary_search_by(|e| e.0.as_str().cmp(needle.as_str())).ok().map(|i| idx[i].1.clone())
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
    idx.get(i % idx.len()).map(|e| e.0.clone())
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
    let from = idx.partition_point(|e| e.0.as_str() < p.as_str());
    idx[from..]
        .iter()
        .take_while(|e| e.0.starts_with(&p))
        .map(|e| e.0.clone())
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
                !sw.iter().any(|e| e.0 == *foreign),
                "sw index contains the English word {foreign:?}"
            );
        }
        assert!(vocabulary_size("sw") > 500, "sw still has a real vocabulary");
        // and the reverse direction
        let en = index_for("en");
        assert!(!en.iter().any(|e| e.0 == "kufuatana"), "en index contains a Swahili word");
    }

    /// C4/F5: the bank's own form comes back, not the folded key. The whole
    /// point of storing the pair.
    #[test]
    fn canonical_returns_the_banks_casing() {
        // A word the bank holds, asked for in the casing a worksheet uses.
        let Some(form) = canonical("en", "CAT") else {
            panic!("`cat` should be in the en bank");
        };
        assert_eq!(form, "cat", "the bank's casing wins over the photograph's");
        assert_eq!(canonical("en", "cat").as_deref(), Some("cat"));
        // ...and a word it does not hold gets no opinion at all, which is what
        // lets F5 keep the scanned casing instead of inventing one.
        assert_eq!(canonical("en", "zzzznotaword"), None);
        assert_eq!(canonical("en", ""), None);
    }

    /// The pair must not change what every existing caller sees: word_at and
    /// starting_with still speak in FOLDED forms, and is_valid still answers
    /// exactly what it did.
    #[test]
    fn the_pair_did_not_change_the_old_answers() {
        assert!(is_valid("en", "cat") && is_valid("en", "CAT"));
        assert!(!is_valid("en", "zzzznotaword"));
        let w = word_at("en", 0).expect("en has words");
        assert_eq!(w, w.to_lowercase(), "word_at still returns the folded form");
        for s in starting_with("en", "ca").iter().take(5) {
            assert!(s.starts_with("ca"), "starting_with still matches folded prefixes");
        }
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
