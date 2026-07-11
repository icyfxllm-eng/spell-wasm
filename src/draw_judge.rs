//! Drawing "judge" — turns a handwriting recognizer's output plus the KNOWN
//! target word into a verdict, coaching feedback, and a per-letter breakdown.
//!
//! The core insight: the app already knows the word the child is spelling, so
//! this is *verification*, not open transcription. That makes it far more
//! robust than generic OCR (or an LLM reading a photo), because:
//!
//!   1. **Constrained match** — even a low-ranked candidate that equals the
//!      target is a pass. We don't need the recognizer to rank it #1, only to
//!      have *seen* it. (`judge` scans every candidate, not just the top one.)
//!   2. **Letter-aligned near-miss** — otherwise we align the best candidate to
//!      the target letter-by-letter and check the mismatches against known
//!      child letter-confusions (b/d, p/q mirror flips, u/v, m/n…), so the UI
//!      can coach ("your 'b' is facing the wrong way") instead of a flat wrong.
//!
//! Pure logic: no DOM, no wasm, no I/O — fully unit-testable, and reused as-is
//! by the offline eval harness (see the `eval` tests + tools/draw-eval/cases.json).
//! It is not wired into the live drawing pad yet; the ML Kit stroke recognizer
//! (which yields the ranked `Candidate` list this consumes) lands first.
#![allow(dead_code)]

use crate::norm::fold_lenient;

/// One recognizer hypothesis: the text it thinks was written + a 0.0..=1.0
/// confidence. ML Kit Digital Ink returns a ranked list of these; the current
/// Tesseract path yields exactly one (wrap it via [`judge_single`]).
#[derive(Clone, Debug)]
pub struct Candidate {
    pub text: String,
    pub score: f32,
}

impl Candidate {
    pub fn new(text: impl Into<String>, score: f32) -> Self {
        Candidate { text: text.into(), score }
    }
}

/// What to do with the drawing.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Matches the target — accept as the answer.
    Correct,
    /// One or two letters off, close enough to coach and let them retry.
    Almost,
    /// Not the word — encourage another attempt.
    TryAgain,
    /// Nothing legible was drawn / recognized.
    Empty,
}

/// The single most useful thing to tell the child, resolved to an i18n key so
/// the game layer localizes it. `letter`/`position` let the UI highlight the
/// exact spot on the pad or in the target word.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub key: &'static str,
    pub letter: Option<char>,
    pub position: Option<usize>,
}

impl Hint {
    fn new(key: &'static str) -> Self {
        Hint { key, letter: None, position: None }
    }
    fn at(key: &'static str, letter: char, position: usize) -> Self {
        Hint { key, letter: Some(letter), position: Some(position) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LetterStatus {
    Ok,
    /// Wrong letter, and it's a known child confusion with the expected one.
    Confusion,
    /// Wrong letter (not a recognized confusion).
    Wrong,
    /// The expected letter is absent from the drawing.
    Missing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerLetter {
    pub expected: char,
    pub got: Option<char>,
    pub status: LetterStatus,
}

/// The judge's full result: a verdict, the localizable hint, the letter-by-
/// letter breakdown (aligned to the target), and the candidate text it judged
/// against (for display / confirmation UI).
#[derive(Clone, Debug)]
pub struct Outcome {
    pub verdict: Verdict,
    pub hint: Hint,
    pub per_letter: Vec<PerLetter>,
    pub best: String,
}

// ---- tuning knobs (pinned by the eval harness) --------------------------------

/// A candidate whose text folds to the target is accepted even at low rank, but
/// not from pure noise — it must clear this floor. Exact-text match is strong
/// evidence, so the floor is permissive.
const ACCEPT_FLOOR: f32 = 0.20;
/// Max edit distance (and at most half the word) still treated as "Almost".
const ALMOST_MAX_DIST: usize = 2;

// ---- child letter-confusion tables (Latin script) -----------------------------

/// True mirror-image pairs — the classic reversal a young writer makes. These
/// get the most specific "you flipped it" coaching.
const MIRROR: &[(char, char)] = &[('b', 'd'), ('p', 'q'), ('b', 'p'), ('d', 'q')];

/// Broader look-alike / sound-alike confusions kids make by hand.
const CONFUSABLE: &[(char, char)] = &[
    ('b', 'd'), ('p', 'q'), ('b', 'p'), ('d', 'q'),
    ('m', 'n'), ('n', 'h'), ('u', 'v'), ('v', 'w'), ('i', 'j'),
    ('i', 'l'), ('a', 'o'), ('g', 'q'), ('c', 'e'), ('f', 't'),
    ('g', 'y'), ('k', 'x'), ('r', 'n'),
];

fn pair_in(table: &[(char, char)], a: char, b: char) -> bool {
    table.iter().any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

fn is_mirror(a: char, b: char) -> bool {
    pair_in(MIRROR, a, b)
}
fn is_confusable(a: char, b: char) -> bool {
    pair_in(CONFUSABLE, a, b)
}

// ---- alignment ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Match(char),
    Sub(char, char), // (expected, got)
    Del(char),       // expected letter missing from the drawing
    Ins(char),       // an extra letter drawn that the target doesn't have
}

/// Needleman–Wunsch / Levenshtein alignment of the target (`exp`) against the
/// recognized word (`got`). Substitutions are preferred over indels on ties so
/// the letter-position feedback stays tight.
fn align(exp: &[char], got: &[char]) -> Vec<Op> {
    let (n, m) = (exp.len(), got.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        d[i][0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if exp[i - 1] == got[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j - 1] + cost)
                .min(d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1);
        }
    }
    // Backtrace, biasing toward the diagonal (sub/match).
    let (mut i, mut j) = (n, m);
    let mut ops = Vec::new();
    while i > 0 || j > 0 {
        let cost = if i > 0 && j > 0 && exp[i - 1] == got[j - 1] { 0 } else { 1 };
        if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + cost {
            ops.push(if cost == 0 { Op::Match(exp[i - 1]) } else { Op::Sub(exp[i - 1], got[j - 1]) });
            i -= 1;
            j -= 1;
        } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            ops.push(Op::Del(exp[i - 1]));
            i -= 1;
        } else {
            ops.push(Op::Ins(got[j - 1]));
            j -= 1;
        }
    }
    ops.reverse();
    ops
}

fn per_letter_from(ops: &[Op]) -> Vec<PerLetter> {
    let mut out = Vec::new();
    for op in ops {
        match *op {
            Op::Match(c) => out.push(PerLetter { expected: c, got: Some(c), status: LetterStatus::Ok }),
            Op::Sub(e, g) => {
                let status = if is_confusable(e, g) { LetterStatus::Confusion } else { LetterStatus::Wrong };
                out.push(PerLetter { expected: e, got: Some(g), status });
            }
            Op::Del(e) => out.push(PerLetter { expected: e, got: None, status: LetterStatus::Missing }),
            Op::Ins(_) => {} // extra stroke-letter: counts toward distance, no target slot
        }
    }
    out
}

fn distance(ops: &[Op]) -> usize {
    ops.iter().filter(|o| !matches!(o, Op::Match(_))).count()
}

// ---- public API ---------------------------------------------------------------

/// Judge a recognizer's ranked candidates against the target word.
pub fn judge(target: &str, candidates: &[Candidate], _locale: &str) -> Outcome {
    let tgt = fold_lenient(target);
    if tgt.is_empty() {
        return Outcome { verdict: Verdict::Empty, hint: Hint::new("draw.empty"), per_letter: vec![], best: String::new() };
    }
    // Keep only candidates that recognized *something*.
    let cands: Vec<(&Candidate, String)> = candidates
        .iter()
        .map(|c| (c, fold_lenient(&c.text)))
        .filter(|(_, f)| !f.is_empty())
        .collect();
    if cands.is_empty() {
        return Outcome { verdict: Verdict::Empty, hint: Hint::new("draw.empty"), per_letter: vec![], best: String::new() };
    }

    // 1) Constrained match: the target appearing at ANY rank (above the noise
    //    floor) wins — this is the whole advantage of knowing the answer.
    if let Some((c, _)) = cands.iter().filter(|(_, f)| *f == tgt).max_by(|a, b| a.0.score.total_cmp(&b.0.score)) {
        if c.score >= ACCEPT_FLOOR {
            let per = tgt.chars().map(|c| PerLetter { expected: c, got: Some(c), status: LetterStatus::Ok }).collect();
            return Outcome { verdict: Verdict::Correct, hint: Hint::new("draw.correct"), per_letter: per, best: c.text.clone() };
        }
    }

    // 2) Best near-miss: fewest edits to the target, then highest score.
    let tgt_chars: Vec<char> = tgt.chars().collect();
    let (best_c, best_ops) = cands
        .iter()
        .map(|(c, f)| {
            let ops = align(&tgt_chars, &f.chars().collect::<Vec<_>>());
            (c, ops)
        })
        .min_by(|a, b| distance(&a.1).cmp(&distance(&b.1)).then(b.0.score.total_cmp(&a.0.score)))
        .unwrap();

    let per_letter = per_letter_from(&best_ops);
    let dist = distance(&best_ops);
    let hint = choose_hint(&best_ops);

    // "Almost" only when it's genuinely close: few edits AND not more than half
    // the word wrong (so "cat" vs "dog" is a TryAgain, not an Almost).
    let verdict = if dist <= ALMOST_MAX_DIST && dist * 2 <= tgt_chars.len().max(1) {
        Verdict::Almost
    } else {
        Verdict::TryAgain
    };
    let hint = if verdict == Verdict::TryAgain { Hint::new("draw.tryAgain") } else { hint };

    Outcome { verdict, hint, per_letter, best: best_c.text.clone() }
}

/// Pick the single most actionable hint from the alignment: a mirror flip is
/// the most specific ("you flipped your b/d"), then any single off letter, else
/// a generic nudge.
fn choose_hint(ops: &[Op]) -> Hint {
    let mut pos = 0usize; // position in the TARGET
    let mut first_off: Option<(char, usize)> = None;
    for op in ops {
        match *op {
            Op::Match(_) => pos += 1,
            Op::Sub(e, g) => {
                if is_mirror(e, g) {
                    return Hint::at("draw.almost.mirror", e, pos);
                }
                first_off.get_or_insert((e, pos));
                pos += 1;
            }
            Op::Del(e) => {
                first_off.get_or_insert((e, pos));
                pos += 1;
            }
            Op::Ins(_) => {}
        }
    }
    match first_off {
        Some((e, p)) => Hint::at("draw.almost.oneLetter", e, p),
        None => Hint::new("draw.almost.generic"),
    }
}

/// Adapter for the current single-result OCR path (Tesseract returns one string
/// + a 0–100 confidence). Wraps it as a one-candidate list so today's pad can
/// use the same judge the ML Kit path will.
pub fn judge_single(target: &str, recognized: &str, confidence_0_1: f32, locale: &str) -> Outcome {
    judge(target, &[Candidate::new(recognized, confidence_0_1)], locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(t: &str, s: f32) -> Candidate {
        Candidate::new(t, s)
    }

    #[test]
    fn exact_top_rank_is_correct() {
        let o = judge("dog", &[c("dog", 0.98)], "en");
        assert_eq!(o.verdict, Verdict::Correct);
        assert!(o.per_letter.iter().all(|p| p.status == LetterStatus::Ok));
    }

    #[test]
    fn target_at_low_rank_still_accepted() {
        // Recognizer's #1 guess is wrong, but it saw "dog" lower down — the
        // known-answer constraint accepts it.
        let o = judge("dog", &[c("bog", 0.71), c("dog", 0.24)], "en");
        assert_eq!(o.verdict, Verdict::Correct);
    }

    #[test]
    fn exact_but_pure_noise_is_not_trusted() {
        let o = judge("dog", &[c("dog", 0.05)], "en");
        assert_ne!(o.verdict, Verdict::Correct); // below ACCEPT_FLOOR
    }

    #[test]
    fn mirror_flip_gets_specific_hint() {
        // wrote "bog" for "dog": d↔b is a mirror pair.
        let o = judge("dog", &[c("bog", 0.9)], "en");
        assert_eq!(o.verdict, Verdict::Almost);
        assert_eq!(o.hint.key, "draw.almost.mirror");
        assert_eq!(o.hint.letter, Some('d'));
        assert_eq!(o.hint.position, Some(0));
        assert_eq!(o.per_letter[0].status, LetterStatus::Confusion);
    }

    #[test]
    fn one_letter_off_is_almost() {
        let o = judge("cat", &[c("cot", 0.9)], "en");
        assert_eq!(o.verdict, Verdict::Almost);
        assert_eq!(o.hint.key, "draw.almost.oneLetter");
        assert_eq!(o.hint.position, Some(1));
    }

    #[test]
    fn missing_letter_is_almost_with_position() {
        let o = judge("frog", &[c("fog", 0.9)], "en"); // dropped the 'r'
        assert_eq!(o.verdict, Verdict::Almost);
        assert_eq!(o.per_letter.iter().filter(|p| p.status == LetterStatus::Missing).count(), 1);
    }

    #[test]
    fn totally_different_word_is_try_again() {
        let o = judge("cat", &[c("dog", 0.95)], "en");
        assert_eq!(o.verdict, Verdict::TryAgain);
        assert_eq!(o.hint.key, "draw.tryAgain");
    }

    #[test]
    fn nothing_recognized_is_empty() {
        assert_eq!(judge("cat", &[], "en").verdict, Verdict::Empty);
        assert_eq!(judge("cat", &[c("   ", 0.9)], "en").verdict, Verdict::Empty);
    }

    #[test]
    fn accent_lenient_match_for_kid_mode() {
        // fold_lenient strips the accent, so a plain "cafe" matches "café".
        let o = judge("café", &[c("cafe", 0.9)], "fr");
        assert_eq!(o.verdict, Verdict::Correct);
    }

    #[test]
    fn best_of_several_candidates_is_chosen() {
        let o = judge("ship", &[c("slip", 0.6), c("shan", 0.5), c("shsp", 0.4)], "en");
        // "slip" and "shsp" are both edit-distance 1 from "ship"; the tie breaks
        // to the higher-scored candidate ("slip", 0.6). "shan" is distance 2.
        assert_eq!(o.verdict, Verdict::Almost);
        assert_eq!(o.best, "slip");
    }
}

/// Offline accuracy harness. Runs the judge over the labeled corpus in
/// tools/draw-eval/cases.json and asserts it reaches the human-labeled verdict
/// on every case, so any regression (or a tuning-knob change) fails CI. The two
/// metrics that matter for a kids' game are reported explicitly:
///   * false-accept — judged Correct when the child was actually wrong (never
///     teaches the right spelling); target = 0.
///   * false-reject — judged not-Correct when the child was actually right
///     (punishes a correct answer); target = 0.
#[cfg(test)]
mod eval {
    use super::*;

    const CASES: &str = include_str!("../tools/draw-eval/cases.json");

    fn class(v: &Verdict) -> &'static str {
        match v {
            Verdict::Correct => "correct",
            Verdict::Almost => "almost",
            Verdict::TryAgain => "tryagain",
            Verdict::Empty => "empty",
        }
    }

    #[test]
    fn judge_hits_labeled_truth_on_every_case() {
        let v: serde_json::Value = serde_json::from_str(CASES).expect("cases.json parses");
        let cases = v["cases"].as_array().expect("cases array");

        let (mut pass, mut false_accept, mut false_reject) = (0usize, 0usize, 0usize);
        let mut failures: Vec<String> = Vec::new();

        for case in cases {
            let id = case["id"].as_str().unwrap_or("?");
            let locale = case["locale"].as_str().unwrap_or("en");
            let target = case["target"].as_str().unwrap_or("");
            let truth = case["truth"].as_str().unwrap_or("");
            let cands: Vec<Candidate> = case["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    let a = c.as_array().unwrap();
                    Candidate::new(a[0].as_str().unwrap_or(""), a[1].as_f64().unwrap_or(0.0) as f32)
                })
                .collect();

            let got = class(&judge(target, &cands, locale).verdict);
            if got == truth {
                pass += 1;
            } else {
                failures.push(format!("  [{id}] target={target:?} truth={truth} got={got}"));
                if truth != "correct" && got == "correct" {
                    false_accept += 1;
                }
                if truth == "correct" && got != "correct" {
                    false_reject += 1;
                }
            }
        }

        eprintln!(
            "\ndraw-eval: {pass}/{} cases correct | false-accept={false_accept} false-reject={false_reject}",
            cases.len()
        );
        if !failures.is_empty() {
            eprintln!("mismatches:\n{}", failures.join("\n"));
        }
        assert_eq!(false_accept, 0, "a wrong drawing was judged Correct (never teaches spelling)");
        assert_eq!(false_reject, 0, "a correct drawing was judged wrong (punishes the child)");
        assert!(failures.is_empty(), "{} case(s) missed their labeled verdict", failures.len());
    }
}
