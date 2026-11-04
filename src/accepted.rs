//! CC-ORTHO-SWEEP-DE-JA F3/F5: accepted spelling variants for German and Japanese, from data files only.
//!
//! `data/<lang>/accepted-variants.tsv` is `stored<TAB>accepted<TAB>source[<TAB>kind]`. The grader reads accepted
//! variants from these files and nowhere else; no pair is generated from a pattern. The English list
//! (`variants.rs`) predates this and will migrate onto the same format (CC-EN-VARIANTS Phase B).
//! `data/ja/dual-script.tsv` (`row<TAB>dual_script`) lists the rows that also accept the other kana script (D8): hiragana for katakana
//! and the reverse, and only for those rows.
//!
//! A variant is a spelling of the SAME word. It is never a near-sound, so every row needs a dictionary entry
//! that lists both forms.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::norm::fold_strict;

const DE: &str = include_str!("../data/de/accepted-variants.tsv");
const JA: &str = include_str!("../data/ja/accepted-variants.tsv");
const JA_DUAL: &str = include_str!("../data/ja/dual-script.tsv");

fn rows(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

/// language -> folded stored form -> the accepted spellings, as written.
fn table() -> &'static HashMap<&'static str, HashMap<String, Vec<&'static str>>> {
    static T: OnceLock<HashMap<&'static str, HashMap<String, Vec<&'static str>>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut all = HashMap::new();
        for (lang, text) in [(crate::consts::DE, DE), (crate::consts::JA, JA)] {
            let mut m: HashMap<String, Vec<&'static str>> = HashMap::new();
            for line in rows(text) {
                let mut it = line.split('\t');
                let (Some(stored), Some(accepted)) = (it.next(), it.next()) else { continue };
                if fold_strict(stored) != fold_strict(accepted) {
                    m.entry(fold_strict(stored)).or_default().push(accepted);
                }
            }
            all.insert(lang, m);
        }
        all
    })
}

fn dual_script() -> &'static HashSet<String> {
    static S: OnceLock<HashSet<String>> = OnceLock::new();
    S.get_or_init(|| rows(JA_DUAL).filter_map(|l| l.split('\t').next()).map(fold_strict).collect())
}

/// The listed accepted spellings of `word`, as written (case kept, for the case-strict German grader).
pub fn alternates(lang: &str, word: &str) -> &'static [&'static str] {
    table().get(lang).and_then(|m| m.get(&fold_strict(word))).map(|v| v.as_slice()).unwrap_or(&[])
}

/// Is `typed` a listed variant of the stored `word`, or its other-script spelling for a `dual_script` row?
pub fn accepts(lang: &str, word: &str, typed: &str) -> bool {
    let t = fold_strict(typed);
    if alternates(lang, word).iter().any(|a| fold_strict(a) == t) {
        return true;
    }
    lang == crate::consts::JA
        && dual_script().contains(&fold_strict(&crate::ja_canon::to_hiragana(word)))
        && crate::ja_canon::to_hiragana(&crate::ja_canon::match_key(typed)) == crate::ja_canon::to_hiragana(&crate::ja_canon::match_key(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_official_double_spelling_is_accepted_and_umlaut_stand_ins_are_not() {
        assert!(accepts("de", "Ghetto", "Getto"));
        assert!(accepts("de", "Xylophon", "xylofon"), "case is folded outside Impossible");
        assert!(!accepts("de", "Getto", "Ghetto"), "one direction: the stored form's variants only");
        for (w, t) in [("Straße", "Strasse"), ("Mädchen", "Maedchen"), ("Mädchen", "Madchen")] {
            assert!(!accepts("de", w, t) && !crate::norm::same_answer("de", t, w), "{t} for {w}");
        }
    }

    #[test]
    fn ja_zu_forms_are_listed_both_ways_and_only_for_the_same_entry() {
        assert!(accepts("ja", "づつ", "ずつ") && accepts("ja", "ずつ", "づつ"));
        assert!(!accepts("ja", "づき", "ずき"), "different words, not a variant");
    }

    #[test]
    fn dual_script_rows_take_the_other_script_and_no_others_do() {
        assert!(dual_script().contains("おおかみ"));
        assert!(accepts("ja", "おおかみ", "オオカミ"));
        assert!(!accepts("ja", "ありがとう", "アリガトウ"), "not a dual_script row");
        assert!(!accepts("en", "cat", "CAT"));
    }
}
