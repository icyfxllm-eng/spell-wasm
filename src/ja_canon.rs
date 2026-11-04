//! CC-ORTHO-SWEEP-DE-JA F6 / D9: Japanese input normalization.
//!
//! Half-width katakana (U+FF61..U+FF9F) becomes full-width, then NFC (which composes the voiced and
//! semi-voiced marks: ｶﾞ -> ガ). Nothing else is folded: not full NFKC, not full-width Latin or digits.
//! The look-alike long-vowel marks (－ — ‐ 一 for ー) are NOT folded, so they grade wrong (D10).
//! This is a match key in the per-language registry (`norm::language_key_matches`), an extra way to match,
//! never a replacement for the strict fold.

use unicode_normalization::UnicodeNormalization;

fn is_halfwidth_katakana(c: char) -> bool {
    ('\u{FF61}'..='\u{FF9F}').contains(&c)
}

/// `s` with half-width katakana made full-width and NFC applied.
pub fn canonical(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_halfwidth_katakana(c) {
            out.extend(std::iter::once(c).nfkc());
        } else {
            out.push(c);
        }
    }
    out.nfc().collect()
}

/// The comparison key: the canonical form under the strict fold.
pub fn match_key(s: &str) -> String {
    crate::norm::fold_strict(&canonical(s))
}

/// Katakana letters to hiragana (U+30A1..U+30F6 shift down 0x60). ー and everything else is unchanged.
pub fn to_hiragana(s: &str) -> String {
    s.chars().map(|c| if ('\u{30A1}'..='\u{30F6}').contains(&c) { char::from_u32(c as u32 - 0x60).unwrap_or(c) } else { c }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_width_katakana_is_full_width() {
        assert_eq!(canonical("\u{FF79}\u{FF70}\u{FF77}"), "ケーキ");
    }

    #[test]
    fn voiced_marks_compose() {
        assert_eq!(canonical("\u{FF76}\u{FF9E}"), "ガ");
        assert_eq!(canonical("\u{FF8A}\u{FF9F}"), "パ");
    }

    #[test]
    fn nothing_else_is_folded() {
        for s in ["ＡＢＣ１２３", "ケ－キ", "ケ—キ", "ケ‐キ", "ケ一キ"] {
            assert_eq!(canonical(s), s, "{s}");
        }
    }
}
