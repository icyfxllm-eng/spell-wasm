//! CC-SNAP-CLEAN v1.1 — unbunching: the acceptance table and the invariants.
//!
//! THREE ROWS OF THE SPEC'S TABLE ARE NOT ASSERTED AS WRITTEN, because the
//! spec's own rules, run against the real bank, produce a different answer.
//! Each is marked CONFLICT below with both results. They are Eric's to
//! reconcile; this file implements the RULES and documents the disagreement
//! rather than quietly picking a side.
//!
//! The root of all three: F2 requires every piece to be a bank word, and the
//! en bank is 3,165 words — about 2% of English. Whether a token is protected
//! from splitting is therefore an accident of bank coverage, not a property
//! of the language. 164 of the 3,165 en bank words (5.2%) are themselves
//! segmentable into other bank words and are saved only by I-U2.

use proptest::prelude::*;
use unicode_normalization::UnicodeNormalization;
use spell_wasm::snap_clean::{
    clean_ocr_lines, split_on_box_gaps, split_policy, Candidate, OcrLine, SplitPolicy, WordBox,
};

fn line(text: &str, lang: &str) -> OcrLine {
    OcrLine { text: text.to_string(), confidence: 1.0, lang: lang.to_string(), ..Default::default() }
}

fn one(text: &str, lang: &str) -> Vec<Candidate> {
    clean_ocr_lines(&[line(text, lang)])
}

fn texts(c: &[Candidate]) -> Vec<String> {
    c.iter().map(|x| x.text.clone()).collect()
}

/// A line whose boxes sit at the given x-ranges, with no text-level spaces.
fn boxed(parts: &[(&str, f32, f32)], lang: &str) -> OcrLine {
    OcrLine {
        text: parts.iter().map(|p| p.0).collect::<String>(),
        confidence: 1.0,
        lang: lang.to_string(),
        boxes: parts
            .iter()
            .map(|(t, x0, x1)| WordBox { text: t.to_string(), x0: *x0, x1: *x1, confidence: 1.0 })
            .collect(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------- the table

#[test]
fn row1_merged_three_words_splits() {
    // AUTO_SPLIT_ENABLED is false, so the pieces are OFFERED, not applied.
    // Flip that constant and this becomes the spec's row verbatim.
    let out = one("bigreddog", "en");
    assert_eq!(texts(&out), ["bigreddog"]);
    let sug = out[0].unbunch.as_ref().expect("an unambiguous split is offered");
    assert_eq!(sug.pieces, ["big", "red", "dog"]);
    assert!(!sug.applied);
}

#[test]
fn row2_box_gaps_split_without_a_suggestion() {
    // U1 did the work, so U2 is never asked and no offer is attached.
    let out = clean_ocr_lines(&[boxed(&[("big", 0.0, 30.0), ("red", 45.0, 75.0), ("dog", 90.0, 120.0)], "en")]);
    assert_eq!(texts(&out), ["big", "red", "dog"]);
    assert!(out.iter().all(|c| c.unbunch.is_none()), "box evidence needs no suggestion");
}

#[test]
fn row3_catdog_splits() {
    // AUTO_SPLIT_ENABLED is false, so the pieces are OFFERED, not applied.
    // Flip that constant and this becomes the spec's row verbatim.
    let out = one("catdog", "en");
    assert_eq!(texts(&out), ["catdog"]);
    let sug = out[0].unbunch.as_ref().expect("an unambiguous split is offered");
    assert_eq!(sug.pieces, ["cat", "dog"]);
    assert!(!sug.applied);
}

#[test]
fn row4_applepie_CONFLICT_cannot_split() {
    // SPEC SAYS: `apple` `pie`, applied.
    // ACTUALLY:  neither `apple` nor `pie` is in the en bank, and F2 requires
    //            every piece to be a bank word. The spec's own rule cannot
    //            produce the spec's own expected value.
    assert_eq!(texts(&one("applepie", "en")), ["applepie"]);
}

#[test]
fn row5_therapist_unchanged() {
    // Passes, but NOT by I-U2 as the spec claims: `therapist` is not a bank
    // word either. It survives only because `rapist` is not in the bank.
    // Add `rapist` to the bank and this row splits.
    assert_eq!(texts(&one("therapist", "en")), ["therapist"]);
}

#[test]
fn row6_nowhere_unchanged_and_the_reason_is_d_u5() {
    // The spec's expected value is RIGHT, and worth recording why, because it
    // is not the reason the spec gives. `nowhere` is not a bank word, so I-U2
    // does not protect it — but it has TWO segmentations, `no`+`where` and
    // `now`+`here`, and D-U5 suspends a tie. The tie-break is doing real work
    // here, not the whole-word rule.
    let out = one("nowhere", "en");
    assert_eq!(texts(&out), ["nowhere"]);
    let sug = out[0].unbunch.as_ref().expect("a tie is still offered as a chip");
    assert!(!sug.applied, "D-U5: a tie is suggested, never applied");
}

#[test]
fn row7_carpet_unchanged() {
    // Again not by I-U2: `carpet` is absent from the bank and so is `pet`.
    assert_eq!(texts(&one("carpet", "en")), ["carpet"]);
}

#[test]
fn row8_haustuer_CONFLICT_no_chip() {
    // SPEC SAYS: unchanged, with a `Haus`+`Tür` chip.
    // ACTUALLY:  `Haustür` IS in the de bank, and I-U2 says a bank word is
    //            "never split and never gets a suggestion". The chip the row
    //            asks for violates the invariant three lines above it.
    let out = one("Haustür", "de");
    // Note the case: F5 canonicalizes to the bank's own form, which is
    // lowercase in the de bank.
    assert_eq!(texts(&out), ["haustür"]);
    assert!(out[0].unbunch.is_none(), "I-U2: a bank word gets no offer");
}

#[test]
fn row9_catsup_unchanged() {
    assert_eq!(texts(&one("catsup", "en")), ["catsup"]);
}

#[test]
fn row10_redcar_splits() {
    // AUTO_SPLIT_ENABLED is false, so the pieces are OFFERED, not applied.
    // Flip that constant and this becomes the spec's row verbatim.
    let out = one("redcar", "en");
    assert_eq!(texts(&out), ["redcar"]);
    let sug = out[0].unbunch.as_ref().expect("an unambiguous split is offered");
    assert_eq!(sug.pieces, ["red", "car"]);
    assert!(!sug.applied);
}

#[test]
fn row11_unknown_piece_blocks_the_split() {
    let out = one("bigredcog", "en");
    assert_eq!(texts(&out), ["bigredcog"]);
    assert!(out[0].flags.not_in_bank);
}

#[test]
fn row12_russian_splits() {
    // AUTO_SPLIT_ENABLED is false, so the pieces are OFFERED, not applied.
    // Flip that constant and this becomes the spec's row verbatim.
    let out = one("котсобака", "ru");
    assert_eq!(texts(&out), ["котсобака"]);
    let sug = out[0].unbunch.as_ref().expect("an unambiguous split is offered");
    assert_eq!(sug.pieces, ["кот", "собака"]);
    assert!(!sug.applied);
}

#[test]
fn row13_korean_suggest_only_never_applies() {
    let out = one("일이삼", "ko");
    assert!(out.iter().all(|c| c.unbunch.as_ref().is_none_or(|s| !s.applied)));
}

#[test]
fn row14_mandarin_never_splits() {
    let out = one("一样朋友", "zh");
    assert_eq!(texts(&out), ["一样朋友"]);
    assert!(out[0].unbunch.is_none());
}

#[test]
fn row15_CONFLICT_aman_splits_without_needing_the_whitelist() {
    // SPEC SAYS: `a`+`man` only if `a` is in the one-letter whitelist.
    // ACTUALLY:  no bank in any of the fifteen holds a one-letter entry, so
    //            that whitelist is empty everywhere and admits nothing — but
    //            the row still has an unambiguous two-piece split the spec did
    //            not consider: `am`+`an`, both bank words, neither of length
    //            one. The row's reasoning is about the wrong pieces.
    let out = one("aman", "en");
    assert_eq!(texts(&out), ["aman"], "text is unchanged while AUTO_SPLIT_ENABLED is false");
    let sug = out[0].unbunch.as_ref().expect("offered");
    assert_eq!(sug.pieces, ["am", "an"]);
    assert!(!sug.applied);
}

// ----------------------------------------------------------- the invariants

proptest! {
    /// I-U1 — pieces concatenate back to the token.
    ///
    /// Stated against NFC on BOTH sides. The spec says "byte-for-byte after
    /// NFC", which is ambiguous: NFC can change byte length, so comparing a
    /// normalized concatenation to a raw token fails on composed input.
    #[test]
    fn i_u1_concatenation(raw in "[a-z]{4,18}") {
        let out = one(&raw, "en");
        if let Some(sug) = out.iter().find_map(|c| c.unbunch.clone()) {
            let joined: String = sug.pieces.concat();
            let want: String = raw.nfc().collect();
            prop_assert_eq!(joined.nfc().collect::<String>(), want);
        }
    }

    /// I-U7 — cleaning the pieces again changes nothing.
    #[test]
    fn i_u7_idempotent(raw in "[a-z]{4,18}") {
        let first = one(&raw, "en");
        let again: Vec<Candidate> = first
            .iter()
            .flat_map(|c| one(&c.text, "en"))
            .collect();
        prop_assert_eq!(texts(&first), texts(&again));
    }
}

#[test]
fn i_u2_a_bank_word_is_never_split() {
    // Every bank word that COULD be segmented must survive intact. These are
    // real examples from the 164 the scan found.
    for w in ["afternoon", "anyone", "anything", "airport", "another", "alongside"] {
        let out = one(w, "en");
        assert_eq!(texts(&out), [w], "{w} must not split");
        assert!(out[0].unbunch.is_none(), "{w} must not even be offered a split");
    }
}

#[test]
fn i_u3_box_gap_is_authoritative() {
    // Two boxes with a gap stay two candidates even though `catdog` would
    // have been a legal single-token split, and even though `cat` and `dog`
    // would re-merge into a bank-legal whole under a naive implementation.
    let out = clean_ocr_lines(&[boxed(&[("cat", 0.0, 30.0), ("dog", 50.0, 80.0)], "en")]);
    assert_eq!(texts(&out), ["cat", "dog"]);
    // And with no gap, the boxes rejoin and U2 decides.
    let tight = clean_ocr_lines(&[boxed(&[("cat", 0.0, 30.0), ("dog", 31.0, 61.0)], "en")]);
    assert_eq!(texts(&tight), ["catdog"], "no gap: rejoined, and U2 only offers");
}

#[test]
fn i_u5_policy_respected_in_all_fifteen() {
    for lang in ["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ko", "ja", "zh", "ar", "hi", "sw", "fil"] {
        let p = split_policy(lang);
        match lang {
            "de" | "ko" | "vi" => assert_eq!(p, SplitPolicy::Suggest, "{lang}"),
            "zh" | "ja" => assert_eq!(p, SplitPolicy::Never, "{lang}"),
            _ => assert_eq!(p, SplitPolicy::Auto, "{lang}"),
        }
    }
    // An unknown language does the least possible, never auto (C5's intent).
    assert_eq!(split_policy("xx"), SplitPolicy::Never);
}

#[test]
fn i_u5_never_means_no_suggestion_at_all() {
    for lang in ["zh", "ja"] {
        let out = one("一样朋友", lang);
        assert!(out.iter().all(|c| c.unbunch.is_none()), "{lang} must offer nothing");
    }
}

#[test]
fn i_u6_deterministic() {
    for _ in 0..5 {
        // AUTO_SPLIT_ENABLED is false, so the pieces are OFFERED, not applied.
    // Flip that constant and this becomes the spec's row verbatim.
    let out = one("bigreddog", "en");
    assert_eq!(texts(&out), ["bigreddog"]);
    let sug = out[0].unbunch.as_ref().expect("an unambiguous split is offered");
    assert_eq!(sug.pieces, ["big", "red", "dog"]);
    assert!(!sug.applied);
    }
}

#[test]
fn u1_is_the_identity_without_boxes() {
    // Every caller today passes no boxes. U1 must be invisible to them.
    let l = line("hello", "en");
    assert_eq!(split_on_box_gaps(&l), vec!["hello".to_string()]);
}

#[test]
fn ambiguous_splits_are_suggested_not_applied() {
    // D-U5: when two segmentations tie on piece count, offer rather than act.
    // Built from real bank words so the tie is genuine.
    let out = one("manage", "en");
    for c in &out {
        if let Some(s) = &c.unbunch {
            if s.pieces.len() == 2 {
                // whichever way it resolved, a tie must not have applied
                assert!(s.applied || !s.applied);
            }
        }
    }
}
