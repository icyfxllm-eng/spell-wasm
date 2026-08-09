//! CC-PERSIAN-FOUNDATION F1 — the Persian canonicalizer.
//!
//! One word, one byte representation, everywhere. Persian text arrives from
//! keyboards, OCR, web paste and dictionaries carrying Arabic-script variants
//! that LOOK identical and compare unequal: Arabic yeh/kaf where Persian uses
//! its own, Eastern-Arabic digits where Persian uses its own, decorative
//! tatweel, optional harakat, and legacy presentation forms. Every one of
//! those is a false mismatch waiting to mark a correct answer wrong.
//!
//! This is the SINGLE write path. D1 fixes the table below as law; extending
//! it needs a new decision, so this module deliberately does not "improve" on
//! it (see the open questions at the bottom).
//!
//! Consumers per the spec: bank ingestion, player-input acceptance, TTS
//! request text, cache keys, search, My Words import, photo/OCR import.
//! Nothing else may hand-roll an `if lang == "fa"` fixup.

// Nothing calls this yet: `fa` is not in the registry, and putting it there
// reverses CC-MASTER-PARITY Phase A, which is Eric's decision and not a side
// effect of landing a canonicalizer. The module is exercised by its own tests
// meanwhile. Delete this allow the moment a consumer is wired.
#![allow(dead_code)]

use unicode_normalization::UnicodeNormalization;

/// Zero-width non-joiner — orthography, not spelling (F2). Legal in a bank
/// word; never a tile, never a required keystroke.
pub const ZWNJ: char = '\u{200C}';

/// The 32 letters of the Persian alphabet, in alphabetical order.
const FA_LETTERS: &str = "\u{627}\u{628}\u{67e}\u{62a}\u{62b}\u{62c}\u{686}\u{62d}\u{62e}\u{62f}\
\u{630}\u{631}\u{632}\u{698}\u{633}\u{634}\u{635}\u{636}\u{637}\u{638}\u{639}\u{63a}\u{641}\u{642}\
\u{6a9}\u{6af}\u{644}\u{645}\u{646}\u{648}\u{647}\u{6cc}";

/// Canonicalize `s` per the D1 table, then NFC.
///
/// # Why presentation forms are folded FIRST
///
/// The table lists them last, but applying it in that literal order is NOT
/// idempotent, and the pass's own acceptance criterion is
/// `canon(canon(x)) == canon(x)`. A presentation-form kaf (U+FEDB) folds to
/// Arabic kaf U+0643 — which the kaf row would then need to map to U+06A9,
/// except that row already ran. First pass yields U+0643, second yields
/// U+06A9: not a fixed point.
///
/// Folding first makes every later row see base letters, so one pass reaches
/// the fixed point. This is an ordering change, not a table change — no row is
/// added, removed or altered — but it is a deliberate deviation from the
/// literal reading and is flagged for Eric.
/// # Why NFC runs before the rows, not only after
///
/// The harakat row strips U+064B–065F, which contains MADDAH ABOVE (U+0653)
/// and HAMZA ABOVE (U+0654). A DECOMPOSED آ is ا + U+0653, so stripping first
/// would quietly turn آب into اب — losing a distinction Eric ruled meaningful.
/// Composing first turns those sequences into أ and آ, which the rows then
/// handle deliberately: أ folds to ا, آ is kept.
pub fn canon(s: &str) -> String {
    let folded = fold_presentation_forms(s);
    let composed: String = folded.nfc().collect();
    let mapped: String = composed
        .chars()
        .filter_map(|c| match c {
            // Arabic yeh / alef maksura -> Persian yeh
            '\u{64a}' | '\u{649}' => Some('\u{6cc}'),
            // Arabic kaf -> Persian kaf
            '\u{643}' => Some('\u{6a9}'),
            // D1 EXTENSION (Eric 2026-08-09, "whatever doesn't limit the
            // player or frustrate them"): alef with hamza folds to plain alef
            // and teh marbuta to heh. These are not Persian letters — standard
            // Persian orthography already writes ا and ه — so a bank word
            // carrying them is mis-sourced, and normalising is what lets the
            // player type the word they actually see. آ is NOT here: it is a
            // real distinction and is kept (leniency for it lives in
            // `match_key`, not in the stored form).
            '\u{623}' | '\u{625}' => Some('\u{627}'),
            '\u{629}' => Some('\u{647}'),
            // Eastern-Arabic digits -> Persian digits
            '\u{660}'..='\u{669}' => {
                char::from_u32(c as u32 - 0x0660 + 0x06F0)
            }
            // tatweel: decorative elongation, never meaning
            '\u{640}' => None,
            // harakat and other combining vowel marks
            '\u{64b}'..='\u{65f}' | '\u{670}' => None,
            other => Some(other),
        })
        .collect();
    mapped.nfc().collect()
}

/// Fold Arabic presentation forms (U+FB50–FDFF, U+FE70–FEFF) to base letters.
///
/// NFKC is exactly this mapping for the Arabic blocks — an isolated/initial/
/// medial/final glyph is a *compatibility* variant of its base letter, and a
/// ligature like U+FEFA decomposes to its two letters. It is applied ONLY to
/// characters in those ranges so that NFKC's unrelated behaviour elsewhere
/// (width folding, ligature expansion in other scripts) cannot touch the rest
/// of the string.
fn fold_presentation_forms(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            let pf = matches!(c, '\u{fb50}'..='\u{fdff}' | '\u{fe70}'..='\u{feff}');
            let one = [c];
            let src: String = one.iter().collect();
            if pf {
                src.nfkc().collect::<Vec<char>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

/// Beyond the 32: legal in a canonical fa word, and reachable on the fa
/// keyboard (آ ء on long-press of ا, ؤ on و, ئ on ی).
///
/// D1 EXTENSION (Eric 2026-08-09). The specified set of "32 letters + ZWNJ +
/// digits" rejects آب — water — because آ is counted as a form of ا rather
/// than a letter. A lint that fails on ordinary vocabulary is the definition
/// of limiting the player. ء ؤ ئ join it: they are genuinely Persian, in
/// جزء, مؤسسه, مسئول, and unlike أ إ ة they have no plain-letter equivalent
/// that standard orthography would substitute.
const FA_EXTRA: &str = "\u{622}\u{621}\u{624}\u{626}";

/// The closed set of codepoints a canonical fa bank word may contain:
/// the 32 letters, the four above, ZWNJ, and Persian digits.
pub fn is_fa_legal_char(c: char) -> bool {
    FA_LETTERS.contains(c)
        || FA_EXTRA.contains(c)
        || c == ZWNJ
        || ('\u{6f0}'..='\u{6f9}').contains(&c)
}

/// What an answer is COMPARED against — canonical, minus the distinctions a
/// player should never lose a round over.
///
/// F2 states the principle for ZWNJ: the player spells letters, the game owns
/// orthography. The same reasoning covers آ. The bank stores آب because that
/// is correct Persian and correct Persian is what gets displayed — but typing
/// اب is a keyboard reflex, not a spelling error, so the match folds آ to ا.
/// The keyboard still offers آ on long-press, so a player who wants the exact
/// form can produce it; accepting the plain form is generosity, not a
/// workaround for a missing key.
///
/// Display always uses `canon`. This is comparison only.
pub fn match_key(s: &str) -> String {
    canon(s)
        .chars()
        .filter(|c| *c != ZWNJ)
        .map(|c| if c == '\u{622}' { '\u{627}' } else { c })
        .collect()
}

/// Characters in `s` outside the legal set, in order, deduplicated.
///
/// Ingestion gate: any bank word with a non-empty result fails at CI. Reported
/// rather than silently stripped — a word carrying an illegal codepoint is a
/// sourcing problem, and quietly rewriting it hides the source.
pub fn illegal_chars(s: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in s.chars() {
        if !is_fa_legal_char(c) && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

// ── open questions for Eric (D1 says the table is law; these EXTEND it) ─────
//
// 1. آ (U+0622, alef with madda) is not one of the 32 letters, but Persian
//    cannot be written without it — آب (water) starts with it. As specified,
//    the legal set rejects it and the ingestion lint would fail on ordinary
//    vocabulary. `alef_madda_is_currently_illegal` below pins the CURRENT
//    behaviour so the gap is visible rather than theoretical.
// 2. Likewise ء أ إ ؤ ئ ة, which appear in Persian text of Arabic origin.
//    Standard Persian orthography often normalizes ة->ه and أ/إ->ا, which
//    would be canonicalizer ROWS, not legal-set entries.
//
// Both are additions to a table D1 froze, so neither is made here.

#[cfg(test)]
mod tests {
    use super::*;

    /// Done-when #1: idempotent on a deliberately polluted corpus.
    #[test]
    fn canon_is_a_fixed_point() {
        let polluted = [
            "\u{643}\u{62a}\u{627}\u{628}",           // Arabic kaf   -> Persian kaf
            "\u{628}\u{644}\u{64a}",                   // Arabic yeh   -> Persian yeh
            "\u{628}\u{644}\u{649}",                   // alef maksura -> Persian yeh
            "\u{628}\u{640}\u{640}\u{644}\u{6cc}",     // tatweel
            "\u{645}\u{64e}\u{62f}\u{631}\u{633}\u{647}", // harakat
            "\u{666}\u{667}\u{668}",                   // Eastern-Arabic digits
            "\u{fedb}\u{fe98}\u{fe8e}\u{fe91}",        // presentation forms
            "\u{6a9}\u{62a}\u{627}\u{628}\u{200c}\u{647}\u{627}", // already canonical, with ZWNJ
        ];
        for p in polluted {
            let once = canon(p);
            assert_eq!(canon(&once), once, "not a fixed point: {p:?} -> {once:?}");
        }
    }

    #[test]
    fn every_row_of_the_table_applies() {
        assert_eq!(canon("\u{643}"), "\u{6a9}", "Arabic kaf -> Persian kaf");
        assert_eq!(canon("\u{64a}"), "\u{6cc}", "Arabic yeh -> Persian yeh");
        assert_eq!(canon("\u{649}"), "\u{6cc}", "alef maksura -> Persian yeh");
        assert_eq!(canon("\u{660}\u{669}"), "\u{6f0}\u{6f9}", "digits");
        assert_eq!(canon("\u{628}\u{640}\u{628}"), "\u{628}\u{628}", "tatweel removed");
        assert_eq!(canon("\u{628}\u{64e}"), "\u{628}", "harakat removed");
        assert_eq!(canon("\u{628}\u{670}"), "\u{628}", "superscript alef removed");
    }

    /// The ordering deviation, made concrete: a presentation-form kaf must
    /// land on PERSIAN kaf in ONE pass, not Arabic kaf awaiting a second.
    #[test]
    fn presentation_forms_fold_before_the_letter_rows_run() {
        assert_eq!(canon("\u{fedb}"), "\u{6a9}", "presentation kaf -> Persian kaf");
        assert_eq!(canon("\u{fef3}"), "\u{6cc}", "presentation yeh -> Persian yeh");
        // and the ligature lam-alef expands rather than surviving as one glyph
        assert_eq!(canon("\u{fefb}"), "\u{644}\u{627}");
    }

    /// Done-when #1's second half: no two variants collide post-canon — they
    /// CONVERGE, which is the point.
    #[test]
    fn variants_converge_on_one_representation() {
        let same = ["\u{643}\u{62a}\u{627}\u{628}", "\u{6a9}\u{62a}\u{627}\u{628}",
                    "\u{643}\u{640}\u{62a}\u{627}\u{628}", "\u{fedb}\u{62a}\u{627}\u{628}"];
        let canonical: std::collections::HashSet<String> = same.iter().map(|s| canon(s)).collect();
        assert_eq!(canonical.len(), 1, "four spellings of ketab must converge: {canonical:?}");
    }

    #[test]
    fn legal_set_accepts_canonical_persian_and_rejects_arabic() {
        assert!(illegal_chars("\u{6a9}\u{62a}\u{627}\u{628}").is_empty(), "ketab is legal");
        assert!(illegal_chars("\u{6a9}\u{62a}\u{627}\u{628}\u{200c}\u{647}\u{627}").is_empty(),
                "ZWNJ is legal (F2)");
        assert!(illegal_chars("\u{6f1}\u{6f2}").is_empty(), "Persian digits are legal");
        assert_eq!(illegal_chars("\u{643}\u{62a}\u{627}\u{628}"), vec!['\u{643}'],
                   "a planted Arabic kaf fails ingestion");
        assert_eq!(illegal_chars("\u{628}\u{644}\u{64a}"), vec!['\u{64a}'],
                   "a planted Arabic yeh fails ingestion");
    }

    /// D1 EXTENSION (Eric 2026-08-09): آ is legal, because آب is water.
    #[test]
    fn ordinary_persian_vocabulary_is_legal() {
        for w in ["\u{622}\u{628}", "\u{62c}\u{632}\u{621}",
                  "\u{645}\u{624}\u{633}\u{633}\u{647}", "\u{645}\u{633}\u{626}\u{648}\u{644}"] {
            assert!(illegal_chars(w).is_empty(), "{w} must be legal: {:?}", illegal_chars(w));
        }
    }

    /// The rows that fold: not Persian letters, so a bank word carrying them
    /// is mis-sourced. Standard Persian already writes ا and ه.
    #[test]
    fn arabic_only_forms_normalise_away() {
        assert_eq!(canon("\u{623}"), "\u{627}", "alef+hamza above -> alef");
        assert_eq!(canon("\u{625}"), "\u{627}", "alef+hamza below -> alef");
        assert_eq!(canon("\u{629}"), "\u{647}", "teh marbuta -> heh");
        // ...and they are consequently illegal in a canonical word
        assert_eq!(illegal_chars("\u{623}"), vec!['\u{623}']);
    }

    /// The madda survives composition. A DECOMPOSED آ is ا + U+0653, and
    /// U+0653 sits inside the harakat range the table strips — so without
    /// composing first, آب would silently become اب.
    #[test]
    fn decomposed_alef_madda_keeps_its_madda() {
        assert_eq!(canon("\u{627}\u{653}\u{628}"), "\u{622}\u{628}", "composed, not stripped");
        assert_eq!(canon("\u{627}\u{654}"), "\u{627}", "hamza above still folds to plain alef");
    }

    /// F2's principle, applied to آ per the ruling: correct orthography is
    /// stored and displayed; the comparison forgives the keyboard.
    #[test]
    fn match_key_forgives_what_display_preserves() {
        let stored = canon("\u{622}\u{628}");                 // آب
        assert_eq!(stored, "\u{622}\u{628}", "display keeps the madda");
        assert_eq!(match_key("\u{627}\u{628}"), match_key(&stored),
                   "typing اب must be accepted for آب");
        // ZWNJ likewise: with or without, same verdict (F2).
        let ketabha = "\u{6a9}\u{62a}\u{627}\u{628}\u{200c}\u{647}\u{627}";
        assert_eq!(match_key(ketabha), match_key("\u{6a9}\u{62a}\u{627}\u{628}\u{647}\u{627}"));
        assert!(canon(ketabha).contains(ZWNJ), "but the stored form KEEPS its ZWNJ");
        // and it stays a fixed point
        assert_eq!(match_key(&match_key(ketabha)), match_key(ketabha));
    }
}
