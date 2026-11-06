//! CC-ORTHO-SWEEP-DE-JA F3/F5: accepted spelling variants for German and Japanese, from data files only.
//!
//! `data/<lang>/accepted-variants.tsv` is `stored<TAB>accepted<TAB>source[<TAB>kind]`. The grader reads accepted
//! variants from these files and nowhere else; no pair is generated from a pattern. The English list
//! (`variants.rs`) predates this and will migrate onto the same format (CC-EN-VARIANTS Phase B).
//! English is the one symmetric list (either spelling grades for the other), and its pairs also drive the
//! Settings display switch: see [`pairs`].
//! `data/ja/dual-script.tsv` (`row<TAB>dual_script`) lists the rows that also accept the other kana script (D8): hiragana for katakana
//! and the reverse, and only for those rows.
//!
//! A variant is a spelling of the SAME word. It is never a near-sound, so every row needs a dictionary entry
//! that lists both forms.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::norm::fold_strict;

const DE: &str = include_str!("../data/de/accepted-variants.tsv");
const DE_SPLIT: &str = include_str!("../data/de/together-apart.tsv");
const EN: &str = include_str!("../data/en/accepted-variants.tsv");
const JA: &str = include_str!("../data/ja/accepted-variants.tsv");
const JA_DUAL: &str = include_str!("../data/ja/dual-script.tsv");
// CC-ORTHO-SWEEP-ALL (Eric, 2026-11-05): the one-row-per-word cleanups. Same one-way format as de and ja.
const ES: &str = include_str!("../data/es/accepted-variants.tsv");
const FR: &str = include_str!("../data/fr/accepted-variants.tsv");
const PT: &str = include_str!("../data/pt/accepted-variants.tsv");
const VI: &str = include_str!("../data/vi/accepted-variants.tsv");
const FIL: &str = include_str!("../data/fil/accepted-variants.tsv");
const HI: &str = include_str!("../data/hi/accepted-variants.tsv");

fn rows(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

/// language -> folded stored form -> the accepted spellings, as written.
fn table() -> &'static HashMap<&'static str, HashMap<String, Vec<&'static str>>> {
    static T: OnceLock<HashMap<&'static str, HashMap<String, Vec<&'static str>>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut all = HashMap::new();
        for (lang, text) in [
            (crate::consts::DE, DE), (crate::consts::JA, JA), (crate::consts::EN, EN),
            (crate::consts::ES, ES), (crate::consts::FR, FR), (crate::consts::PT, PT), (crate::consts::VI, VI), (crate::consts::FIL, FIL), (crate::consts::HI, HI),
        ] {
            let mut m: HashMap<String, Vec<&'static str>> = HashMap::new();
            for line in rows(text) {
                let mut it = line.split('\t');
                let (Some(stored), Some(accepted)) = (it.next(), it.next()) else { continue };
                if fold_strict(stored) != fold_strict(accepted) {
                    m.entry(fold_strict(stored)).or_default().push(accepted);
                    // English is graded both ways (Eric, 2026-10-19); de and ja list the reverse row when they want it.
                    if lang == crate::consts::EN {
                        m.entry(fold_strict(accepted)).or_default().push(stored);
                    }
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

/// The English pairs as `(stored US form, accepted British form)`, for the display switch.
pub fn pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    rows(EN).filter_map(|l| {
        let mut it = l.split('\t');
        Some((it.next()?, it.next()?))
    })
}

/// The listed accepted spellings of `word`, as written (case kept, for the case-strict German grader).
pub fn alternates(lang: &str, word: &str) -> &'static [&'static str] {
    table().get(lang).and_then(|m| m.get(&fold_strict(word))).map(|v| v.as_slice()).unwrap_or(&[])
}

/// CC-ORTHO-SWEEP-DE-JA F9: is `typed` (which has a space inside it) a together/apart spelling the Regelwerk LISTS for the stored `word`?
/// Whitespace runs collapse to one space and case is folded; nothing else is loosened and no split is inferred from a rule.
pub fn split_listed(word: &str, typed: &str) -> bool {
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let t = squash(typed);
    rows(DE_SPLIT).any(|l| {
        let mut it = l.split('\t');
        matches!((it.next(), it.next()), (Some(a), Some(b)) if fold_strict(a) == fold_strict(word) && squash(b) == t)
    })
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
    fn de_listed_splits_are_accepted_and_unlisted_splits_are_not() {
        use crate::norm::same_answer;
        for (w, t) in [("sodass", "so dass"), ("weitgehend", "weit gehend"), ("dahingehend", "dahin gehend"), ("hierzulande", "hier zu Lande"), ("sodass", "  So   dass "), ("kennenlernen", "kennen lernen"), ("sitzenbleiben", "sitzen bleiben")] {
            assert!(same_answer("de", t, w), "T15 {t} for {w}");
        }
        for (w, t) in [("sodass", "s o dass"), ("wieder", "wie der"), ("weitgehend", "weitgeh end"), ("stattdessen", "statt dessen"), ("nachdem", "nach dem")] {
            assert!(!same_answer("de", t, w), "T14 an unlisted split is wrong: {t} for {w}");
        }
        assert!(same_answer("de", "sodass ", "sodass") && same_answer("de", " sodass", "sodass"), "edge whitespace is trimmed, not a split");
    }

    #[test]
    fn de_split_rule_is_normal_mode_and_german_only() {
        use crate::norm::{answer_matches_in, same_answer};
        assert!(answer_matches_in("de", "wie der", "wieder", true), "T16 Spell Jr is untouched");
        assert!(same_answer("en", "ice cream", "icecream"), "T16 other languages keep the whitespace-free fold");
        assert!(!answer_matches_in("de", "wie der", "wieder", false));
    }

    #[test]
    fn de_stand_ins_are_close_at_medium_and_up_and_accepted_at_easy_and_spell_jr() {
        use crate::norm::{answer_matches_in, de_near_miss, same_answer, DeNearMiss};
        for (w, t, fix) in [("Straße", "Strasse", "ss → ß"), ("Mädchen", "Maedchen", "ae → ä"), ("Tür", "Tuer", "ue → ü"), ("schön", "schoen", "oe → ö")] {
            assert!(!same_answer("de", t, w), "{t} is not correct at Medium+");
            assert_eq!(de_near_miss(t, w), Some(DeNearMiss::Letters { fix: fix.into() }), "{t} is close");
            assert!(answer_matches_in("de", t, w, true), "{t} is accepted in Spell Jr");
            assert!(crate::norm::de_stand_in_accepted(t, w), "{t} is accepted at Easy");
        }
        assert_eq!(de_near_miss("kennen lernen", "kennenlernen"), Some(DeNearMiss::Split), "an unlisted split is close; this one is listed so same_answer accepts it first");
        for (w, t) in [("Mädchen", "Madchen"), ("Straße", "Strase"), ("Bär", "Baer ")] {
            let near = de_near_miss(t, w);
            assert!(matches!(near, None | Some(DeNearMiss::Letters { .. })), "{t}");
        }
        assert_eq!(de_near_miss("Madchen", "Mädchen"), None, "dropping the dots is not a stand-in");
        assert_eq!(de_near_miss("Mädchen", "Mädchen"), None, "correct is not close");
        assert!(!answer_matches_in("en", "Maedchen", "Mädchen", true), "German only");
    }

    #[test]
    fn en_pairs_grade_both_ways_and_migrated_whole() {
        assert_eq!(pairs().count(), 209, "the 209 pairs Eric signed on 2026-10-19");
        for (us, gb) in [("color", "colour"), ("theater", "theatre"), ("organize", "organise"), ("traveled", "travelled"), ("gray", "grey"), ("learned", "learnt"), ("judgment", "judgement"), ("anemia", "anaemia")] {
            assert!(accepts("en", us, gb) && accepts("en", gb, us), "{us} / {gb}");
        }
        assert!(!accepts("en", "size", "sise") && !accepts("en", "prize", "prise"), "T3: no pattern-made pair");
        assert!(!accepts("en", "color", "color"), "the same spelling is the normal comparison");
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

    #[test]
    fn ortho_sweep_all_cleanup_variants_grade_both_spellings_of_one_row() {
        // fr: the 1990 form is stored, the traditional form is accepted
        assert!(accepts("fr", "évènement", "événement"));
        // pt: the stored form is Brazilian AO90; the pre-reform and European forms are accepted
        assert!(accepts("pt", "para", "pára"));
        assert!(accepts("pt", "deem", "dêem"));
        assert!(accepts("pt", "fato", "facto"));
        assert!(accepts("pt", "seção", "secção"));
        // vi, fil
        assert!(accepts("vi", "hí", "hý"));
        assert!(accepts("fil", "lalaki", "lalake"));
        // hi: the nukta row is stored, the no-nukta spelling is accepted
        assert!(crate::norm::same_answer("hi", "जरूर", "जरूर"));
        let nuk = alternates("hi", "बर्फ़");
        assert_eq!(nuk.len(), 1, "one accepted spelling for the nukta row");
        // a variant never leaks across languages or onto another word
        assert!(!accepts("es", "para", "pára"));
        assert!(!accepts("fr", "évènement", "evenement"));
    }

    #[test]
    fn every_new_variant_row_cites_a_rule_and_resolves_to_one_stored_row() {
        for (name, text) in [("es", ES), ("fr", FR), ("pt", PT), ("vi", VI), ("fil", FIL), ("hi", HI)] {
            let mut seen = HashSet::new();
            for line in rows(text) {
                let f: Vec<&str> = line.split('\t').collect();
                assert!(f.len() >= 4 && !f[2].trim().is_empty(), "{name}: row without a source: {line}");
                assert!(seen.insert(fold_strict(f[1])), "{name}: accepted spelling {} resolves to two rows", f[1]);
            }
        }
    }
}
