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

/// Marks that are never part of a stored answer, and so must never decide a
/// verdict.
///
/// Arabic tashkeel and tatweel: the banks hold the unvocalized standard
/// (CC-LINEUP-SWAP D5), and the ar bank carries zero harakat at any tier today.
/// But writing the marks is correct Arabic and the Arabic keyboard reaches them
/// in one long-press, so a player who spelled the word and then vocalized it was
/// being told the word was wrong. Dropping these from BOTH sides cannot flip a
/// verdict that exists today -- no canonical answer contains one, which
/// `no_shipping_answer_depends_on_a_silent_mark` checks rather than assumes --
/// so the only reachable change is a correct spelling that used to be refused
/// now being accepted.
///
/// ZWNJ is orthography, not a keystroke. `fa_canon` settled this for Persian
/// (F2, Eric 2026-08-09): the stored form keeps its ZWNJ, the comparison
/// ignores it, because a player cannot be held to an invisible character.
/// Devanagari half-forms are the same question.
///
/// ZWJ (U+200D) is deliberately absent: My Words takes arbitrary text and emoji
/// sequences are joined with it, so folding it away would merge distinct
/// entries in the one place a player supplies their own words.
///
/// NFC runs before this, so أ إ آ ؤ ئ have already composed out of their
/// letter + hamza pairs. Hamza seats are spelling and survive; ة/ه and ى/ي are
/// letters and were never in reach of this filter.
fn is_silent_mark(c: char) -> bool {
    matches!(c,
        '\u{064B}'..='\u{065F}'  // tashkeel: fathatan .. wavy hamza below
        | '\u{0670}'             // superscript (dagger) alef
        | '\u{0640}'             // tatweel: decorative elongation
        | '\u{200C}'             // ZWNJ
    )
}

/// NFC + Unicode lowercase, whitespace and silent marks removed. Accent-strict.
pub fn fold_strict(s: &str) -> String {
    s.nfc()
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && !is_silent_mark(*c))
        .collect()
}

/// Accent-lenient fold for Kid Mode: strip combining marks and map the letters
/// that have no NFD decomposition down to A–Z-ish equivalents.
pub fn fold_lenient(s: &str) -> String {
    let lowered: String = s.nfc().collect::<String>().to_lowercase();
    // Silent marks go BEFORE the NFD pass, not after: NFD would decompose أ into
    // ا + U+0654, which sits inside the tashkeel range, and stripping it there
    // would quietly make Kid Mode hamza-blind -- gutting Arabic's main trap
    // class for exactly the players least able to notice.
    let lowered: String = lowered.chars().filter(|c| !is_silent_mark(*c)).collect();
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

/// Say-It mode match rule (Feature F2, Decision D2): the target word is
/// considered *said* iff, after NFC + Unicode case folding, it appears as one of
/// the tokens in the recognizer's transcription. This is an **exact token match
/// after folding** — deliberately NOT fuzzy, phonetic, or confidence-scored.
///
/// Tokenization only splits the transcription on whitespace and trims edge
/// punctuation the recognizer attaches (leading/trailing `.,!?;:"'…` and quotes),
/// which is orthographic hygiene, not fuzzing — the *comparison* is still exact.
/// Accent-strict like normal typed play (`café` ≠ `cafe`); Say-It is never
/// offered in Kid Mode, so there is no lenient variant.
pub fn spoken_matches(transcript: &str, word: &str) -> bool {
    let target = fold_strict(word);
    if target.is_empty() {
        return false;
    }
    transcript
        .split_whitespace()
        .any(|tok| fold_strict(trim_edge_punct(tok)) == target)
}

/// Strip leading/trailing punctuation a speech recognizer commonly appends to a
/// word ("elephant." / "¿casa?"), keeping any internal marks (apostrophes,
/// hyphens) intact so the fold can compare them.
fn trim_edge_punct(tok: &str) -> &str {
    tok.trim_matches(|c: char| {
        !c.is_alphanumeric() && c != '\'' && c != '\u{2019}' && c != '-'
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

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
    fn filipino_charset_and_folding() {
        // ñ composed (U+00F1) vs decomposed (n + U+0303) fold to the same thing.
        assert_eq!(fold_strict("piña"), fold_strict("pin\u{303}a"));
        assert!(answer_matches("piña", "pin\u{303}a", false));
        // Hyphen is kept (orthographic); case folds.
        assert_eq!(fold_strict("Mag-Aral"), "mag-aral");
        assert!(answer_matches("Mag-Aral", "mag-aral", false));
        // Normal mode is ñ-strict: "pina" must not match "piña".
        assert!(!answer_matches("pina", "piña", false));
    }

    #[test]
    fn korean_precomposed_matches_decomposed_jamo() {
        // Phase 0.1: the NFC chokepoint must make conjoining-jamo input match a
        // precomposed syllable. 한 (simple), 값 (double final consonant ㅄ), 의
        // (compound vowel) — decomposed via NFD, must recompose and match.
        for w in ["\u{d55c}", "\u{ac12}", "\u{c758}"] {
            let decomposed: String = w.nfd().collect();
            assert_ne!(w, decomposed.as_str(), "{w} must actually decompose");
            assert!(answer_matches(&decomposed, w, false), "{w}: decomposed jamo must match NFC");
        }
    }

    #[test]
    fn vietnamese_precomposed_matches_decomposed() {
        // Phase 0.1: ế / ệ (base + circumflex + tone) decomposed must match NFC.
        for w in ["\u{1ebf}", "\u{1ec7}"] {
            let decomposed: String = w.nfd().collect();
            assert_ne!(w, decomposed.as_str(), "{w} must actually decompose");
            assert!(answer_matches(&decomposed, w, false), "{w}: decomposed must match NFC");
        }
    }

    #[test]
    fn english_unaffected_by_mode() {
        // No diacritics -> strict and lenient agree.
        assert!(answer_matches("Rhythm", "rhythm", false));
        assert!(answer_matches("Rhythm", "rhythm", true));
        assert!(!answer_matches("rythm", "rhythm", false));
    }

    // ---- Say-It (spoken) match rule: exact token match after NFC + case fold ----

    #[test]
    fn spoken_word_present_as_a_token_matches() {
        // The recognizer heard a sentence; the target token is in it.
        assert!(spoken_matches("the elephant is big", "elephant"));
        assert!(spoken_matches("elephant", "elephant"));
    }

    #[test]
    fn spoken_is_case_insensitive_via_fold() {
        assert!(spoken_matches("ELEPHANT", "elephant"));
        assert!(spoken_matches("Elephant", "ELEPHANT"));
    }

    #[test]
    fn spoken_trims_recognizer_edge_punctuation() {
        // SFSpeechRecognizer commonly appends/prepends punctuation.
        assert!(spoken_matches("Elephant.", "elephant"));
        assert!(spoken_matches("is it a casa?", "casa"));
        assert!(spoken_matches("\u{201c}Casa,\u{201d}", "casa"));
        assert!(spoken_matches("\u{a1}Hola!", "hola"));
    }

    #[test]
    fn spoken_is_accent_strict_like_typed_play() {
        // café != cafe: Say-It uses the strict fold (never offered in Kid Mode).
        assert!(spoken_matches("un caf\u{e9} por favor", "caf\u{e9}"));
        assert!(!spoken_matches("un cafe por favor", "caf\u{e9}"));
    }

    #[test]
    fn spoken_nfd_transcript_matches_nfc_target() {
        // Recognizer emits decomposed (n + combining tilde); target stored NFC.
        let nfd_sentence = "la aran\u{0303}a";
        assert!(spoken_matches(nfd_sentence, "ara\u{f1}a"));
    }

    #[test]
    fn spoken_no_partial_or_substring_match() {
        // Exact token only — "elephants" is a different token than "elephant".
        assert!(!spoken_matches("two elephants here", "elephant"));
        // No substring credit: target inside a bigger token doesn't count.
        assert!(!spoken_matches("elephantine", "elephant"));
    }

    #[test]
    fn spoken_empty_never_matches() {
        assert!(!spoken_matches("", "elephant"));
        assert!(!spoken_matches("anything at all", ""));
        assert!(!spoken_matches("   ", "elephant"));
    }

    #[test]
    fn spoken_keeps_internal_apostrophe_and_hyphen() {
        assert!(spoken_matches("say don't now", "don't"));
        assert!(spoken_matches("it is mag-aral", "mag-aral"));
    }

    /// CC-AR-HI-PREAUDIT acceptance test 5, first half. Arabic is written
    /// without short-vowel marks and the bank stores the unvocalized standard
    /// (CC-LINEUP-SWAP D5), but typing the marks is correct Arabic and the
    /// iOS Arabic keyboard reaches them in one long-press. A player who spells
    /// the word and decorates it was being told the word was wrong.
    #[test]
    fn typed_harakat_are_accepted_never_penalized() {
        // مدرسة, then the same word fully vocalized: مَدْرَسَة
        let bare = "\u{645}\u{62f}\u{631}\u{633}\u{629}";
        let marked = "\u{645}\u{64e}\u{62f}\u{652}\u{631}\u{64e}\u{633}\u{64e}\u{629}";
        assert_ne!(bare, marked, "inputs must differ before normalization");
        assert!(answer_matches(marked, bare, false), "vocalized spelling is the same spelling");
        assert!(answer_matches(marked, bare, true), "and in Kid Mode too");
    }

    /// The other half of acceptance test 5: the marks are noise, the letters
    /// are not. ة and ه are different letters (trap class taa-marbuta-vs-haa),
    /// and stripping marks must not start folding them together.
    #[test]
    fn stripping_marks_does_not_fold_letters() {
        let madrasa_ta = "\u{645}\u{62f}\u{631}\u{633}\u{629}"; // مدرسة
        let madrasa_ha = "\u{645}\u{62f}\u{631}\u{633}\u{647}"; // مدرسه
        assert!(!answer_matches(madrasa_ha, madrasa_ta, false), "ة vs ه stays a miss");

        // Hamza seats are spelling (F2/I1) and survive the fold in both modes.
        let with_hamza = "\u{623}\u{646}"; // أن
        let bare_alif = "\u{627}\u{646}"; // ان
        assert!(!answer_matches(bare_alif, with_hamza, false), "hamza seat stays graded");
        assert!(!answer_matches(bare_alif, with_hamza, true), "including in Kid Mode");

        // ى vs ي likewise (trap class alif-maqsura-vs-yaa).
        assert!(!answer_matches("\u{639}\u{644}\u{64a}", "\u{639}\u{644}\u{649}", false));
    }

    /// Tatweel is decorative elongation with no meaning -- fa_canon says so for
    /// Persian and it is the same character in Arabic.
    #[test]
    fn tatweel_is_decoration_not_spelling() {
        let plain = "\u{628}\u{628}";
        let stretched = "\u{628}\u{640}\u{640}\u{628}";
        assert!(answer_matches(stretched, plain, false));
    }

    /// ZWNJ is orthography, not a keystroke a player can be held to. fa_canon
    /// F2 (Eric, 2026-08-09) settled this for Persian: the stored form keeps
    /// its ZWNJ, the comparison ignores it. Devanagari half-forms are the same
    /// question, and the hi bank stores no ZWNJ at any tier.
    #[test]
    fn zwnj_never_decides_a_verdict() {
        let hindi = "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}"; // हिन्दी
        let with_zwnj = "\u{939}\u{93f}\u{928}\u{94d}\u{200c}\u{926}\u{940}";
        assert_ne!(hindi, with_zwnj);
        assert!(answer_matches(with_zwnj, hindi, false));
    }

    /// ZWJ is deliberately NOT stripped. My Words takes arbitrary text and
    /// emoji sequences are joined with U+200D; folding it away would merge
    /// distinct entries in the one place a player supplies their own words.
    #[test]
    fn zwj_survives_because_my_words_takes_emoji() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";
        let three = "\u{1f468}\u{1f469}\u{1f467}";
        assert!(!answer_matches(three, family, false), "ZWJ still distinguishes emoji sequences");
    }

    /// The whole change is only safe because no canonical answer carries one of
    /// these marks: removing them from both sides can then never flip a verdict
    /// that exists today, it can only accept a correct spelling that used to be
    /// refused. This test is that premise, checked against the shipping banks
    /// rather than assumed.
    #[test]
    fn no_shipping_answer_depends_on_a_silent_mark() {
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS {
            for tier in ["easy", "medium", "hard", "expert"] {
                for w in crate::words::tier_for(code, tier) {
                    // The whole entry, Mandarin's "pinyin|hanzi" included: no
                    // split, because none is needed. Nothing here grades -- it
                    // counts characters -- and checking both halves is the
                    // stronger claim anyway.
                    assert_eq!(
                        fold_strict(w).chars().count(),
                        w.nfc().collect::<String>().to_lowercase()
                            .chars().filter(|c| !c.is_whitespace()).count(),
                        "locale {code}: {w:?} loses a character to the silent-mark fold",
                    );
                }
            }
        }
    }
}
