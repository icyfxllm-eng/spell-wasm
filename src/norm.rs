//! Unicode-aware answer normalization and comparison.
//!
//! Two folds, one comparison:
//! * [`fold_strict`] — NFC + Unicode lowercase, whitespace dropped. Keeps
//!   diacritics, so `café != cafe`. Normal-mode comparison. This is why the
//!   per-locale keyboards exist: to type the correct accents.
//! * [`fold_lenient`] — `fold_strict` plus accent-stripping (é→e, ñ→n) and the
//!   folds for letters with no NFD decomposition (ß→ss, æ→ae, œ→oe, ł→l, ø→o,
//!   ı→i). Accepts `cafe` for `café`. Kid Mode only — spelling confidence first,
//!   diacritic discipline later.
//!
//! Both accept NFD or NFC input (they normalize to NFC first) and use full
//! Unicode case folding, so `Über == über` and German nouns compare
//! case-insensitively (v1: no capitalization mechanics).

use unicode_normalization::UnicodeNormalization;

/// NFC + Unicode lowercase, whitespace removed. Accent-strict.
pub fn fold_strict(s: &str) -> String {
    s.nfc()
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Accent-lenient fold for Kid Mode: strip combining marks and map the letters
/// that have no NFD decomposition down to A–Z-ish equivalents.
pub fn fold_lenient(s: &str) -> String {
    let lowered: String = s.nfc().collect::<String>().to_lowercase();
    let stripped: String = lowered.nfd().filter(|c| !('\u{0300}'..='\u{036f}').contains(c)).collect();
    stripped
        .replace('\u{df}', "ss") // ß
        .replace('\u{e6}', "ae") // æ
        .replace('\u{153}', "oe") // œ
        .chars()
        .map(|c| match c {
            '\u{142}' => 'l', // ł
            '\u{f8}' => 'o',  // ø
            '\u{131}' => 'i', // ı (dotless)
            other => other,
        })
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Compare a typed answer to the target word. Kid Mode is accent-lenient;
/// normal mode is accent-strict.
pub fn answer_matches(typed: &str, word: &str, kid: bool) -> bool {
    if kid {
        fold_lenient(typed) == fold_lenient(word)
    } else {
        fold_strict(typed) == fold_strict(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive_with_diacritics() {
        // Über == über: full case fold, diacritic preserved on both sides.
        assert!(answer_matches("Über", "über", false));
        assert!(answer_matches("ÜBER", "über", false));
    }

    #[test]
    fn nfd_input_matches_nfc_storage() {
        // "café": decomposed input (e + U+0301) vs precomposed storage.
        let nfd = "cafe\u{301}";
        let nfc = "caf\u{e9}";
        assert_ne!(nfd, nfc, "inputs must differ before normalization");
        assert!(answer_matches(nfd, nfc, false));
    }

    #[test]
    fn normal_mode_is_accent_strict() {
        assert!(!answer_matches("cafe", "café", false));
        assert!(answer_matches("café", "café", false));
    }

    #[test]
    fn kid_mode_is_accent_lenient() {
        assert!(answer_matches("cafe", "café", true));
        assert!(answer_matches("nino", "niño", true));
        assert!(answer_matches("uber", "über", true));
    }

    #[test]
    fn special_letter_folds_lenient() {
        assert!(answer_matches("strasse", "straße", true));
        assert!(answer_matches("lodz", "łódz", true));
    }

    #[test]
    fn strict_rejects_special_letter_strip() {
        // In normal mode ß must be typed (or matched) exactly, not as "ss".
        assert!(!answer_matches("strasse", "straße", false));
    }

    #[test]
    fn whitespace_is_ignored() {
        assert!(answer_matches("  café ", "café", false));
    }

    #[test]
    fn english_unaffected_by_mode() {
        // No diacritics -> strict and lenient agree.
        assert!(answer_matches("Rhythm", "rhythm", false));
        assert!(answer_matches("Rhythm", "rhythm", true));
        assert!(!answer_matches("rythm", "rhythm", false));
    }
}
