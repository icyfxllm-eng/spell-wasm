//! Pinyin answer normalization for Mandarin. The player types pinyin with tone
//! numbers (苹果 → `ping2guo3`); this canonicalizes both the typed answer and the
//! stored answer before comparison so the same reading always matches.
//!
//! Rules (spec §Mandarin normalization):
//! * NFC + lowercase.
//! * `v` → `ü` (standard pinyin input convention for lü/nü).
//! * Neutral tone `5` is optional: it's dropped, so `de` and `de5` are equal.
//! * Separators (spaces, apostrophes, syllable breaks) are removed — comparison
//!   is on the continuous reading. Tone digits 1–4 are significant and kept.
//!
//! Tone-*marked* input (ā á ǎ à) is out of scope: the custom keyboard can't
//! produce it, so answers are always tone-numbered.
//!
//! Ships ahead of the Mandarin keyboard/word integration, so the public surface
//! is allowed to be unused for now.
#![allow(dead_code)]

use unicode_normalization::UnicodeNormalization;

pub fn normalize(input: &str) -> String {
    input
        .nfc()
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            'v' => Some('ü'),
            '5' => None,
            ' ' | '\'' | '-' => None,
            c if c.is_whitespace() => None,
            c => Some(c),
        })
        .collect()
}

/// Compare a typed pinyin answer to the canonical stored pinyin.
pub fn matches(typed: &str, answer: &str) -> bool {
    normalize(typed) == normalize(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_and_nfc() {
        assert_eq!(normalize("Ping2Guo3"), "ping2guo3");
    }

    #[test]
    fn v_maps_to_u_umlaut() {
        assert_eq!(normalize("lv4"), "lü4");
        assert_eq!(normalize("nv3"), "nü3");
    }

    #[test]
    fn neutral_tone_is_optional() {
        assert_eq!(normalize("de5"), normalize("de"));
        assert!(matches("xie4xie5", "xie4xie"));
    }

    #[test]
    fn separators_removed() {
        assert_eq!(normalize("ping2 guo3"), "ping2guo3");
        assert!(matches("ping2 guo3", "ping2guo3"));
    }

    #[test]
    fn tone_numbers_are_significant() {
        // Different tones are different answers (mother vs hemp vs horse vs scold).
        assert!(!matches("ma1", "ma3"));
        assert!(matches("ma1", "ma1"));
    }

    #[test]
    fn u_umlaut_input_equals_v_input() {
        assert!(matches("lü4", "lv4"));
    }
}
