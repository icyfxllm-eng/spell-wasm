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

fn rows(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

/// language -> folded stored form -> the accepted spellings, as written.
fn table() -> &'static HashMap<&'static str, HashMap<String, Vec<&'static str>>> {
    static T: OnceLock<HashMap<&'static str, HashMap<String, Vec<&'static str>>>> = OnceLock::new();
    T.get_or_init(|| {
        let mut all = HashMap::new();
        for (lang, text) in [(crate::consts::DE, DE), (crate::consts::JA, JA), (crate::consts::EN, EN)] {
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

/// The English national-spelling pairs as `(stored US form, accepted British form)`, for the display switch.
/// Accent (`diacritic`) and `compound` rows are accepted answers, never a form the player is SHOWN, so they are left out.
pub fn pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    rows(EN).filter_map(|l| {
        let mut it = l.split('\t');
        let (us, gb, _source, kind) = (it.next()?, it.next()?, it.next()?, it.next()?);
        (!matches!(kind.trim(), "diacritic" | "compound")).then_some((us, gb))
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
        assert_eq!(pairs().count(), 327, "the 209 pairs Eric signed on 2026-10-19, neurone, annexe and donut (2026-11-04), and 115 SCOWL-confirmed pairs approved 2026-11-05");
        for (us, gb) in [("color", "colour"), ("theater", "theatre"), ("organize", "organise"), ("traveled", "travelled"), ("gray", "grey"), ("learned", "learnt"), ("judgment", "judgement"), ("anemia", "anaemia")] {
            assert!(accepts("en", us, gb) && accepts("en", gb, us), "{us} / {gb}");
        }
        assert!(!accepts("en", "size", "sise") && !accepts("en", "prize", "prise"), "T3: no pattern-made pair");
        assert!(!accepts("en", "color", "color"), "the same spelling is the normal comparison");
    }

    /// CC-EN-VARIANTS F4: the plain form is stored, the accented one is accepted, never required, and never shown.
    #[test]
    fn en_accents_are_accepted_not_required_and_not_displayed() {
        assert!(accepts("en", "naive", "na\u{ef}ve") && accepts("en", "fete", "f\u{ea}te"));
        assert!(crate::norm::fold_strict("naive") != crate::norm::fold_strict("na\u{ef}ve"), "the fold itself stays accent-strict");
        assert!(!pairs().any(|(_, gb)| gb.chars().any(|c| !c.is_ascii())), "no accented form reaches the display switch");
        assert!(accepts("en", "neuron", "neurone") && accepts("en", "doughnut", "donut") && accepts("en", "annex", "annexe"));
        assert!(!accepts("en", "fjord", "fiord"), "fiord has no commercial-licence source; staged");
    }

    /// CC-EN-VARIANTS B2 (Eric, 2026-11-05): the approved SCOWL batch is live; the pairs he pulled or rejected are not.
    #[test]
    fn en_approved_batch_is_live_and_the_pulled_pairs_are_not() {
        assert!(accepts("en", "organization", "organisation") && accepts("en", "tumor", "tumour") && accepts("en", "yogurt", "yoghurt"));
        assert!(accepts("en", "visualization", "visualisation"));
        let kind = |us: &str| EN.lines().find(|l| l.starts_with(&format!("{us}\t"))).and_then(|l| l.split('\t').nth(3)).unwrap_or("");
        assert_eq!(kind("visualization"), "ise", "relabelled from other");
        assert!(!accepts("en", "bogey", "bogie") && !accepts("en", "bogie", "bogey"), "separate words, not a variant");
        assert!(!accepts("en", "pedophilia", "paedophilia") && !accepts("en", "paedophilia", "pedophilia"), "the word is quarantined");
        for (w, v) in [("country", "counterie"), ("sole", "soal"), ("strength", "strenth"), ("humorous", "humourous"), ("warp", "wharp"), ("pedagogical", "paedagogical"), ("defensible", "defencible"), ("bagel", "beigel")] {
            assert!(!accepts("en", w, v), "{w}/{v} was rejected");
        }
        for (w, v) in [("bylaw", "byelaw"), ("amoxicillin", "amoxycillin"), ("fjord", "fiord")] {
            assert!(!accepts("en", w, v), "{w}/{v} waits for a commercial-licence source");
        }
        for (a, b) in [("vice", "vise"), ("castor", "caster"), ("drier", "dryer")] {
            assert!(!accepts("en", a, b) && !accepts("en", b, a), "{a}/{b} are different words");
        }
    }

    /// A staged file is documentation, never read: it holds exactly the three pairs still waiting on a licensed source.
    #[test]
    fn en_staged_file_holds_only_the_waiting_pairs() {
        let staged = include_str!("../data/en/accepted-variants.staged.tsv");
        let got: Vec<&str> = rows(staged).filter_map(|l| l.split('\t').next()).collect();
        assert_eq!(got, ["bylaw", "amoxicillin", "fjord"]);
    }

    /// Eric, 2026-11-05: pedophilia is quarantined (both spellings) and can never reach the growth stage again.
    #[test]
    fn pedophilia_is_quarantined_and_not_staged() {
        let q = include_str!("../assets/words-quarantine/en/expert.txt");
        assert!(q.lines().any(|l| l == "pedophilia") && q.lines().any(|l| l == "paedophilia"));
        let staged = include_str!("../assets/words/en/staged.tsv");
        assert!(!staged.lines().any(|l| l.starts_with("pedophilia\t") || l.starts_with("paedophilia\t")));
        for t in [include_str!("../assets/words/en/easy.txt"), include_str!("../assets/words/en/medium.txt"), include_str!("../assets/words/en/hard.txt"), include_str!("../assets/words/en/expert.txt")] {
            assert!(!t.lines().any(|l| l.trim() == "pedophilia" || l.trim() == "paedophilia"));
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
