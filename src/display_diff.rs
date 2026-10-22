//! CC-VISUAL-ECLIPSE F4 / V-D14 -- the letter comparison under a wrong answer.
//!
//! `display_diff(target, attempt)` lines the player's attempt up against the
//! correct spelling so the reveal can mark which letters were extra, missing,
//! wrong or swapped. It is optimal string alignment (Levenshtein plus adjacent
//! transposition, each unit used once) over extended grapheme clusters after
//! NFC, so a Hangul syllable block, a Devanagari conjunct and a Vietnamese
//! tone-marked vowel each count as one letter.
//!
//! DISPLAY ONLY. It never decides whether an answer is right: grading never
//! imports it, and `tests::grading_never_imports_the_diff` (AT-12) enforces
//! that by scanning the tree. It compares exactly -- no case folding, no
//! accent folding -- because it draws what the player typed against what the
//! mode already revealed; a German `hund` against `Hund` is a real substitution
//! on screen even though it adds nothing the reveal did not show.
//!
//! Pure: no DOM, no flags, no allocation beyond the result.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// What happened to one letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// The same letter in both.
    Same,
    /// A different letter in the same place (highlighted on both rows).
    Sub,
    /// In the attempt only (struck on the "You wrote" row).
    Extra,
    /// In the target only (underlined on the "Correct" row).
    Missing,
    /// Two neighbouring letters typed in the wrong order. A swap is emitted as
    /// two consecutive `Swap` cells, each carrying its own target and attempt.
    Swap,
}

/// One column of the alignment. `target` is `None` for `Extra`, `attempt` is
/// `None` for `Missing`; every other op carries both.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Cell {
    pub op: Op,
    pub target: Option<String>,
    pub attempt: Option<String>,
}

/// The units the diff counts: NFC, then extended grapheme clusters.
pub fn clusters(s: &str) -> Vec<String> {
    let nfc: String = s.nfc().collect();
    nfc.graphemes(true).map(str::to_string).collect()
}

/// Align `attempt` against `target`. Empty in, empty out; identical in, all
/// `Same`. The result's targets, read in order, spell `target` (NFC) and its
/// attempts spell `attempt` (NFC) -- nothing is dropped or invented.
pub fn display_diff(target: &str, attempt: &str) -> Vec<Cell> {
    let t = clusters(target);
    let a = clusters(attempt);
    let (n, m) = (t.len(), a.len());
    // d[i][j] = cost of aligning t[..i] with a[..j].
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let sub = if t[i - 1] == a[j - 1] { 0 } else { 1 };
            let mut best = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + sub);
            if i > 1 && j > 1 && t[i - 1] == a[j - 2] && t[i - 2] == a[j - 1] && t[i - 1] != t[i - 2] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    // Walk back. Preference on ties: keep a match, then a swap, then a
    // substitution, then a missing letter, then an extra one. The order only
    // matters when two alignments cost the same, and it is fixed so the same
    // pair always draws the same way.
    let mut out = Vec::with_capacity(n.max(m));
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && t[i - 1] == a[j - 1] && d[i][j] == d[i - 1][j - 1] {
            out.push(cell(Op::Same, Some(&t[i - 1]), Some(&a[j - 1])));
            i -= 1;
            j -= 1;
        } else if i > 1
            && j > 1
            && t[i - 1] == a[j - 2]
            && t[i - 2] == a[j - 1]
            && t[i - 1] != t[i - 2]
            && d[i][j] == d[i - 2][j - 2] + 1
        {
            // Pushed in reverse, like everything else here.
            out.push(cell(Op::Swap, Some(&t[i - 1]), Some(&a[j - 1])));
            out.push(cell(Op::Swap, Some(&t[i - 2]), Some(&a[j - 2])));
            i -= 2;
            j -= 2;
        } else if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + 1 {
            out.push(cell(Op::Sub, Some(&t[i - 1]), Some(&a[j - 1])));
            i -= 1;
            j -= 1;
        } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            out.push(cell(Op::Missing, Some(&t[i - 1]), None));
            i -= 1;
        } else {
            out.push(cell(Op::Extra, None, Some(&a[j - 1])));
            j -= 1;
        }
    }
    out.reverse();
    out
}

/// The edit distance the alignment pays: a swap costs one.
#[cfg(test)]
pub fn distance(cells: &[Cell]) -> usize {
    let swaps = cells.iter().filter(|c| c.op == Op::Swap).count();
    cells.iter().filter(|c| matches!(c.op, Op::Sub | Op::Extra | Op::Missing)).count() + swaps / 2
}

fn cell(op: Op, target: Option<&String>, attempt: Option<&String>) -> Cell {
    Cell { op, target: target.cloned(), attempt: attempt.cloned() }
}

/// The WASM export (V-D14): the alignment as JSON, for the web layer and the
/// Playwright specs. Same function, same answer.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = displayDiff)]
pub fn display_diff_json(target: &str, attempt: &str) -> String {
    serde_json::to_string(&display_diff(target, attempt)).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Op::*;

    /// The ops only, for compact assertions.
    fn ops(t: &str, a: &str) -> Vec<Op> {
        display_diff(t, a).iter().map(|c| c.op).collect()
    }

    /// Both rows rebuild their inputs exactly (after NFC).
    fn rebuilds(t: &str, a: &str) {
        let cells = display_diff(t, a);
        let tt: String = cells.iter().filter_map(|c| c.target.clone()).collect();
        let aa: String = cells.iter().filter_map(|c| c.attempt.clone()).collect();
        assert_eq!(tt, t.nfc().collect::<String>(), "target row for {t:?}/{a:?}");
        assert_eq!(aa, a.nfc().collect::<String>(), "attempt row for {t:?}/{a:?}");
    }

    // --- the basic operations ---

    #[test]
    fn identical_is_all_same() {
        assert_eq!(ops("cat", "cat"), vec![Same, Same, Same]);
    }

    #[test]
    fn empty_both() {
        assert!(display_diff("", "").is_empty());
    }

    #[test]
    fn empty_attempt_is_all_missing() {
        assert_eq!(ops("dog", ""), vec![Missing, Missing, Missing]);
    }

    #[test]
    fn empty_target_is_all_extra() {
        assert_eq!(ops("", "xy"), vec![Extra, Extra]);
    }

    #[test]
    fn insertion_in_the_middle() {
        assert_eq!(ops("cat", "cart"), vec![Same, Same, Extra, Same]);
    }

    #[test]
    fn insertion_at_the_end() {
        assert_eq!(ops("cat", "cats"), vec![Same, Same, Same, Extra]);
    }

    #[test]
    fn insertion_at_the_start() {
        assert_eq!(ops("rain", "brain"), vec![Extra, Same, Same, Same, Same]);
    }

    #[test]
    fn deletion_in_the_middle() {
        assert_eq!(ops("necessary", "necesary"), {
            let mut v = vec![Same; 9];
            v[4] = Missing; // n e c e [s] s a r y -- the walk-back keeps the later s
            v
        });
    }

    #[test]
    fn deletion_of_a_doubled_letter_marks_one() {
        let c = display_diff("little", "litle");
        assert_eq!(c.iter().filter(|c| c.op == Missing).count(), 1);
        assert_eq!(distance(&c), 1);
    }

    #[test]
    fn substitution() {
        assert_eq!(ops("cat", "cot"), vec![Same, Sub, Same]);
        let c = display_diff("cat", "cot");
        assert_eq!(c[1].target.as_deref(), Some("a"));
        assert_eq!(c[1].attempt.as_deref(), Some("o"));
    }

    #[test]
    fn transposition_is_one_swap_pair() {
        assert_eq!(ops("receive", "recieve"), vec![Same, Same, Same, Swap, Swap, Same, Same]);
        assert_eq!(distance(&display_diff("receive", "recieve")), 1);
    }

    #[test]
    fn swap_cells_carry_their_own_letters() {
        let c = display_diff("ab", "ba");
        assert_eq!(c[0], Cell { op: Swap, target: Some("a".into()), attempt: Some("b".into()) });
        assert_eq!(c[1], Cell { op: Swap, target: Some("b".into()), attempt: Some("a".into()) });
    }

    #[test]
    fn a_doubled_letter_is_never_a_swap() {
        // "aa" against "aa" is not a swap of identical letters.
        assert_eq!(ops("aa", "aa"), vec![Same, Same]);
    }

    #[test]
    fn osa_uses_each_letter_once() {
        // True Damerau would say 2 (ca -> ac -> abc); OSA may not edit a
        // transposed pair again, so it pays 3. That is the specified metric.
        assert_eq!(distance(&display_diff("abc", "ca")), 3);
    }

    #[test]
    fn mixed_errors() {
        // definitely / definately: one substitution.
        assert_eq!(distance(&display_diff("definitely", "definately")), 1);
        // separate / seperate: one substitution.
        assert_eq!(distance(&display_diff("separate", "seperate")), 1);
        // rhythm / rythem: one missing h, one extra e.
        let c = display_diff("rhythm", "rythem");
        assert_eq!(distance(&c), 2);
        rebuilds("rhythm", "rythem");
    }

    #[test]
    fn completely_different() {
        assert_eq!(ops("abc", "xyz"), vec![Sub, Sub, Sub]);
    }

    #[test]
    fn the_rows_always_rebuild_the_inputs() {
        for (t, a) in [
            ("necessary", "neccesary"),
            ("accommodate", "acomodate"),
            ("receive", "recieve"),
            ("", "abc"),
            ("abc", ""),
            ("한국어", "한국"),
            ("việt", "viet"),
            ("كتاب", "كتب"),
            ("Straße", "strasse"),
        ] {
            rebuilds(t, a);
        }
    }

    // --- scripts: the unit is the grapheme cluster after NFC ---

    #[test]
    fn nfc_makes_decomposed_and_composed_equal() {
        // e + combining acute against precomposed é.
        assert_eq!(ops("caf\u{e9}", "cafe\u{301}"), vec![Same, Same, Same, Same]);
    }

    #[test]
    fn a_missing_accent_is_one_substitution_not_an_extra_mark() {
        assert_eq!(ops("caf\u{e9}", "cafe"), vec![Same, Same, Same, Sub]);
    }

    #[test]
    fn hangul_block_is_one_unit() {
        assert_eq!(clusters("한국어").len(), 3);
        assert_eq!(ops("한국어", "한국어"), vec![Same, Same, Same]);
    }

    #[test]
    fn hangul_wrong_block_is_one_substitution() {
        // 국 (guk) against 극 (geuk): one wrong vowel jamo, still one unit.
        assert_eq!(ops("한국어", "한극어"), vec![Same, Sub, Same]);
    }

    #[test]
    fn hangul_conjoining_jamo_compose_to_the_block() {
        // ᄒ ᅡ ᆫ (conjoining jamo) NFC-compose to 한.
        assert_eq!(ops("한", "\u{1112}\u{1161}\u{11ab}"), vec![Same]);
    }

    #[test]
    fn hangul_missing_block() {
        assert_eq!(ops("사랑해", "사해"), vec![Same, Missing, Same]);
    }

    #[test]
    fn devanagari_matra_stays_with_its_consonant() {
        // नमस्ते: न, म, स्ते (conjunct + matra). The matra never splits off.
        assert!(clusters("की").len() == 1, "ki is one cluster");
        assert_eq!(ops("की", "कि"), vec![Sub], "a wrong vowel sign is one substitution");
    }

    #[test]
    fn devanagari_conjunct_is_one_unit() {
        // क्ष (k + virama + ss) is one akshara (GB9c, Unicode 15.1).
        assert_eq!(clusters("क्ष").len(), 1);
        assert_eq!(clusters("नमस्ते").len(), 3);
    }

    #[test]
    fn devanagari_wrong_conjunct_is_one_substitution() {
        // The conjunct स्ते against plain स + ते: the conjunct is one unit, so
        // the attempt's two units align as one substitution plus one extra.
        let c = display_diff("नमस्ते", "नमसते");
        assert_eq!(distance(&c), 2);
        assert_eq!(c[0].op, Same);
        assert_eq!(c[1].op, Same);
        rebuilds("नमस्ते", "नमसते");
    }

    #[test]
    fn vietnamese_tone_marked_vowel_is_one_unit() {
        // ệ (e + circumflex + dot below), decomposed or not, is one letter.
        assert_eq!(clusters("vie\u{302}\u{323}t").len(), 4);
        assert_eq!(ops("việt", "vie\u{323}\u{302}t"), vec![Same, Same, Same, Same], "mark order is canonical after NFC");
    }

    #[test]
    fn vietnamese_wrong_tone_is_one_substitution() {
        // việt against viết: same base vowel, different tone, one unit.
        assert_eq!(ops("việt", "viết"), vec![Same, Same, Sub, Same]);
    }

    #[test]
    fn vietnamese_missing_tone_is_one_substitution() {
        assert_eq!(ops("tiếng", "tieng"), vec![Same, Same, Sub, Same, Same]);
    }

    #[test]
    fn arabic_logical_order_missing_letter() {
        // كتاب (kitaab) against كتب: the alef is missing. The diff works in
        // logical order; right-to-left is the renderer's job.
        let c = display_diff("كتاب", "كتب");
        assert_eq!(c.iter().map(|c| c.op).collect::<Vec<_>>(), vec![Same, Same, Missing, Same]);
        assert_eq!(c[2].target.as_deref(), Some("ا"));
    }

    #[test]
    fn arabic_harakat_stay_on_their_letter() {
        // A diacritic (fatha) is part of its letter's cluster.
        assert_eq!(clusters("كَتَبَ").len(), 3);
        assert_eq!(ops("كَتَبَ", "كَتَبِ"), vec![Same, Same, Sub]);
    }

    #[test]
    fn arabic_transposition() {
        assert_eq!(ops("كتاب", "تكاب"), vec![Swap, Swap, Same, Same]);
    }

    #[test]
    fn german_case_is_a_substitution() {
        // The diff draws what was typed; it does not fold case.
        assert_eq!(ops("Hund", "hund"), vec![Sub, Same, Same, Same]);
    }

    #[test]
    fn german_eszett_against_double_s() {
        let c = display_diff("Straße", "Strasse");
        assert_eq!(distance(&c), 2, "ß -> s is a substitution, the second s an extra");
        rebuilds("Straße", "Strasse");
    }

    #[test]
    fn german_umlaut_against_base_letter() {
        assert_eq!(ops("Mädchen", "Madchen"), vec![Same, Sub, Same, Same, Same, Same, Same]);
    }

    #[test]
    fn emoji_and_zwj_never_split() {
        // Not a spelling input, but a cluster is a cluster.
        assert_eq!(clusters("👩\u{200d}💻").len(), 1);
    }

    #[test]
    fn deterministic() {
        for _ in 0..3 {
            assert_eq!(display_diff("accommodate", "acommodatte"), display_diff("accommodate", "acommodatte"));
        }
    }

    #[test]
    fn distance_matches_the_dp_minimum() {
        // Classic pairs with known OSA distances.
        for (t, a, want) in [("kitten", "sitting", 3), ("flaw", "lawn", 2), ("ca", "ac", 1), ("abcdef", "abcdef", 0), ("a", "", 1)] {
            assert_eq!(distance(&display_diff(t, a)), want, "{t}/{a}");
        }
    }

    // --- AT-12: grading never imports the diff ---

    /// Every file that may reach into the diff. Presentation only: the module
    /// itself and the F4 reveal renderer. (lib.rs declares the module with a
    /// bare `mod`, which the scan does not count as a use.) Anything
    /// that grades -- norm, api, the mode engines, boardgame_grade, the
    /// impossible engine -- must never appear here, and a new caller has to be
    /// added by hand, which is the point.
    const ALLOWED: &[&str] = &["src/display_diff.rs", "src/answer_reveal.rs"];

    #[test]
    fn grading_never_imports_the_diff() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("src"), &mut files);
        assert!(files.len() > 50, "the walk found only {} files", files.len());
        let mut offenders = Vec::new();
        for f in files {
            let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if ALLOWED.contains(&rel.as_str()) {
                continue;
            }
            let src = std::fs::read_to_string(&f).unwrap();
            if src.contains("display_diff::") || src.contains("displayDiff") {
                offenders.push(rel);
            }
        }
        assert!(offenders.is_empty(), "display_diff is display-only; these files name it: {offenders:?}");
    }
}
