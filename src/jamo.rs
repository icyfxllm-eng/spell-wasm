//! Korean jamo-level grading (Phase 3). Instead of a binary right/wrong, grade a
//! typed Korean answer against the expected word at the level of the individual
//! jamo (초성 initial / 중성 medial / 종성 final) inside each syllable block, so
//! the game can give partial credit and tell the player *which* jamo was wrong
//! (e.g. typed 핝 for 한 → the final consonant).
//!
//! Everything is NFC-normalized first (the Phase 0.1 chokepoint), so decomposed
//! conjoining-jamo input and precomposed syllables grade identically. Compound
//! vowels (ㅢ) and double finals (ㅄ) are each one grading unit — a single
//! position to highlight — matching how a learner perceives the block.
//!
//! Pure logic, fully unit-tested. Exposes `grade` (score + per-syllable diff) and
//! `spell_jamo` (the Kid-Mode "spell it out" hint). Wiring partial credit into
//! the scoring/SRS path and the highlight into the answer UI is the integration
//! step; this is the grading core those consume.
#![allow(dead_code)]

use unicode_normalization::UnicodeNormalization;

use crate::hangul;

/// Which position in a syllable block a mistake is at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Initial,
    Medial,
    Final,
}

/// One expected syllable's result.
#[derive(Clone, Debug)]
pub struct SyllableGrade {
    pub expected: char,
    /// The syllable the player typed at this position, if any (None = they typed
    /// fewer syllables than the word has).
    pub typed: Option<char>,
    /// Positions that differ from expected (empty = this block is perfect).
    pub wrong: Vec<Part>,
}

/// A whole-word jamo grade.
#[derive(Clone, Debug)]
pub struct Grade {
    /// Correct jamo units / total jamo units, in `0.0..=1.0` — the standard word
    /// score scale (1.0 == perfect), so it drops straight into the existing
    /// scoring/SRS path.
    pub score: f32,
    pub syllables: Vec<SyllableGrade>,
    /// True only on an exact, full-length match.
    pub correct: bool,
}

fn nfc(s: &str) -> Vec<char> {
    s.nfc().filter(|c| !c.is_whitespace()).collect()
}

/// Grade `typed` against `expected` at jamo granularity.
pub fn grade(typed: &str, expected: &str) -> Grade {
    let exp = nfc(expected);
    let got = nfc(typed);

    let (mut total, mut correct_units) = (0u32, 0u32);
    let mut syllables = Vec::with_capacity(exp.len());

    for (i, &e) in exp.iter().enumerate() {
        let t = got.get(i).copied();
        let mut wrong = Vec::new();

        match (hangul::parts(e), t.and_then(hangul::parts)) {
            (Some((ei, em, ef)), typed_parts) => {
                // A final is a grading unit if either side has one (so a spurious
                // final — 각 for 가 — is penalized, and a missing one too).
                let tf_present = typed_parts.map(|(_, _, tf)| tf != '\0').unwrap_or(false);
                let has_final = ef != '\0' || tf_present;
                total += 2 + has_final as u32;

                if let Some((ti, tm, tf)) = typed_parts {
                    if ei == ti { correct_units += 1 } else { wrong.push(Part::Initial) }
                    if em == tm { correct_units += 1 } else { wrong.push(Part::Medial) }
                    if has_final {
                        if ef == tf { correct_units += 1 } else { wrong.push(Part::Final) }
                    }
                } else {
                    // typed nothing (or a non-syllable) here → every unit wrong.
                    wrong.push(Part::Initial);
                    wrong.push(Part::Medial);
                    if has_final { wrong.push(Part::Final) }
                }
            }
            // Expected char isn't a Hangul syllable (rare): grade as one unit.
            (None, _) => {
                total += 1;
                if t == Some(e) { correct_units += 1 } else { wrong.push(Part::Initial) }
            }
        }
        syllables.push(SyllableGrade { expected: e, typed: t, wrong });
    }

    let score = if total == 0 { 1.0 } else { correct_units as f32 / total as f32 };
    let correct = score == 1.0 && got.len() == exp.len();
    Grade { score, syllables, correct }
}

/// Spell a Korean word out as its compatibility jamo, one run of jamo per
/// syllable (한 → "ㅎㅏㄴ", 의 → "ㅇㅢ", 값 → "ㄱㅏㅄ"). The Kid-Mode hint.
pub fn spell_jamo(word: &str) -> String {
    let mut out = String::new();
    for c in nfc(word) {
        match hangul::parts(c) {
            Some((i, m, f)) => {
                out.push(i);
                out.push(m);
                if f != '\0' {
                    out.push(f);
                }
            }
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_match_scores_one() {
        let g = grade("한국", "한국");
        assert_eq!(g.score, 1.0);
        assert!(g.correct);
        assert!(g.syllables.iter().all(|s| s.wrong.is_empty()));
    }

    #[test]
    fn decomposed_input_matches_precomposed() {
        // Phase 0.1: conjoining-jamo NFD input grades identical to NFC.
        let decomposed: String = "한국".nfd().collect();
        assert_ne!(decomposed, "한국");
        let g = grade(&decomposed, "한국");
        assert!(g.correct);
        assert_eq!(g.score, 1.0);
    }

    #[test]
    fn wrong_final_is_localized_and_partial() {
        // 핝 for 한: ㅎ ok, ㅏ ok, final ㅌ vs ㄴ wrong → 2/3.
        let g = grade("핝", "한");
        assert!(!g.correct);
        assert_eq!(g.syllables[0].wrong, vec![Part::Final]);
        assert!((g.score - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn double_final_word_grades() {
        assert!(grade("값", "값").correct); // ㅄ compound final, one unit
        // 갑 for 값: final ㅂ vs ㅄ wrong → 2/3, localized to Final.
        let g = grade("갑", "값");
        assert_eq!(g.syllables[0].wrong, vec![Part::Final]);
        assert!((g.score - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn compound_vowel_word_grades() {
        assert!(grade("의", "의").correct); // ㅢ, one medial unit (2 total)
        // 이 for 의: medial ㅣ vs ㅢ wrong → 1/2.
        let g = grade("이", "의");
        assert_eq!(g.syllables[0].wrong, vec![Part::Medial]);
        assert!((g.score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn spurious_final_is_penalized() {
        // 각 for 가: expected has no final, typed added ㄱ → Final wrong, 2/3.
        let g = grade("각", "가");
        assert_eq!(g.syllables[0].wrong, vec![Part::Final]);
        assert!((g.score - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn missing_syllable_penalized() {
        // 한 for 한국: 국 missing → its 3 units all wrong → 3/6 = 0.5.
        let g = grade("한", "한국");
        assert!(!g.correct);
        assert_eq!(g.syllables[1].typed, None);
        assert_eq!(g.syllables[1].wrong.len(), 3);
        assert!((g.score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn spell_jamo_hint() {
        assert_eq!(spell_jamo("한"), "ㅎㅏㄴ");
        assert_eq!(spell_jamo("의"), "ㅇㅢ");
        assert_eq!(spell_jamo("값"), "ㄱㅏㅄ");
        assert_eq!(spell_jamo("한국"), "ㅎㅏㄴㄱㅜㄱ");
    }
}
