//! CC-SNAP-CLEAN v1 — the acceptance table and the invariants.
//!
//! Row 19 of the spec's table is absent: it photographs a page whose lines
//! carry different languages, and the census found there is no per-line
//! language to carry. Eric dropped the per-language rule on 2026-09-30, so
//! there is nothing left for that row to assert.

use proptest::prelude::*;
use unicode_normalization::UnicodeNormalization;
use spell_wasm::snap_clean::{clean_ocr_lines, Candidate, OcrLine};

fn line(text: &str, lang: &str) -> OcrLine {
    OcrLine { text: text.to_string(), confidence: 1.0, lang: lang.to_string(), ..Default::default() }
}

fn clean(pairs: &[(&str, &str)]) -> Vec<Candidate> {
    let lines: Vec<OcrLine> = pairs.iter().map(|(t, l)| line(t, l)).collect();
    clean_ocr_lines(&lines)
}

fn texts(c: &[Candidate]) -> Vec<String> {
    c.iter().map(|x| x.text.clone()).collect()
}

// --------------------------------------------------------- acceptance table

/// The table expects `joy` lowercased, which would need the bank to hold it.
/// It does not: `cat`, `dog` and `example` are bank words and come back in
/// the bank's casing, and `joy` is in no English tier, so D2 keeps the casing
/// the camera saw. Asserting `joy` here would mean lowercasing unmatched
/// words, which is exactly the blanket rule D2 rejects for breaking German
/// nouns and proper names. The row's real promise -- a clean list needs zero
/// edits -- is the `checked` assertion, and that holds.
#[test]
fn row1_digit_list() {
    let c = clean(&[("1. Cat", "en"), ("2. Dog", "en"), ("3. Example", "en"), ("4. Joy", "en")]);
    assert_eq!(texts(&c), ["cat", "dog", "example", "Joy"]);
    assert!(c.iter().all(|x| x.checked), "a clean list opens with everything checked");
    assert!(c[3].flags.not_in_bank, "Joy keeps its casing BECAUSE the bank has no opinion");
}

#[test]
fn row2_ocr_misread_one_as_l() {
    let c = clean(&[("l. Cat", "en"), ("2. Dog", "en")]);
    assert_eq!(texts(&c), ["cat", "dog"]);
}

#[test]
fn row3_letter_list() {
    let c = clean(&[("a. apple", "en"), ("b. bee", "en"), ("c. cat", "en")]);
    assert_eq!(texts(&c), ["apple", "bee", "cat"]);
}

#[test]
fn row4_no_scheme_keeps_a_lone_a() {
    let c = clean(&[("a", "en"), ("bee", "en"), ("cat", "en")]);
    assert_eq!(texts(&c), ["a", "bee", "cat"], "no separators, so no scheme, so nothing stripped");
}

#[test]
fn row5_roman_beats_letter_when_a_token_is_two_long() {
    let c = clean(&[("i. run", "en"), ("ii. jump", "en")]);
    assert_eq!(texts(&c), ["run", "jump"]);
}

#[test]
fn row6_inside_word_punctuation_survives() {
    let c = clean(&[("(3) don't.", "en"), ("(4) well-being,", "en")]);
    assert_eq!(texts(&c), ["don't", "well-being"]);
}

#[test]
fn row7_curly_apostrophe_folds_to_straight() {
    let c = clean(&[("1. don't", "en"), ("2. don\u{2019}t", "en")]);
    // The fold is what makes these ONE candidate: D6 dedupes on the folded
    // text, and before the fold they were two different words.
    assert_eq!(texts(&c), ["don't"]);
}

/// Same as row 1: neither `qué` nor `hola` is in the Spanish bank, so the
/// casing stays as photographed. What this row is really testing is that
/// `¿` and `¡` are trimmed from the FRONT, which no ASCII-only edge trim
/// would do -- and that the accented é survives it.
#[test]
fn row8_spanish_inverted_punctuation() {
    let c = clean(&[("\u{BF}Qu\u{E9}?", "es"), ("\u{A1}Hola!", "es")]);
    assert_eq!(texts(&c), ["Qu\u{E9}", "Hola"]);
}

#[test]
fn row9_fullwidth_digits_and_ideographic_stop() {
    let c = clean(&[("\u{FF11}\u{FF0E}ねこ。", "ja"), ("\u{FF12}\u{FF0E}いぬ。", "ja")]);
    assert_eq!(texts(&c), ["ねこ", "いぬ"]);
}

#[test]
fn row10_cjk_numeral_scheme() {
    let c = clean(&[("一、一样", "zh"), ("二、朋友", "zh")]);
    assert_eq!(texts(&c), ["一样", "朋友"], "the marker goes, the homograph first word stays");
}

#[test]
fn row11_devanagari_is_nfc_exact_and_keeps_its_virama() {
    let c = clean(&[("1. नमस्ते।", "hi"), ("2. पानी।", "hi")]);
    assert_eq!(texts(&c), ["नमस्ते", "पानी"]);
    assert!(texts(&c)[0].contains('\u{094D}'), "the virama survived");
}

#[test]
fn row12_vietnamese_marks_survive_and_two_words_is_not_multiword() {
    let c = clean(&[("1. con mèo", "vi"), ("2. bánh mì", "vi")]);
    assert_eq!(texts(&c), ["con mèo", "bánh mì"]);
    assert!(c.iter().all(|x| !x.flags.multi_word), "two tokens is not three");
}

#[test]
fn row13_split_on_delimiters() {
    let c = clean(&[("cat, dog; joy", "en")]);
    assert_eq!(texts(&c), ["cat", "dog", "joy"]);
    assert!(c.iter().all(|x| x.flags.split));
}

#[test]
fn row14_a_header_is_flagged_not_deleted() {
    let c = clean(&[("Week 3 Spelling List", "en")]);
    assert_eq!(texts(&c), ["Week 3 Spelling List"], "flag, never delete");
    assert!(c[0].flags.multi_word && c[0].flags.has_digits);
    assert!(!c[0].checked);
}

#[test]
fn row15_fill_in_blank() {
    let c = clean(&[("Name: ________", "en")]);
    assert_eq!(texts(&c), ["name"]);
    assert!(c[0].flags.blank && !c[0].checked);
}

#[test]
fn row16_a_line_with_no_letters_is_dropped() {
    assert!(clean(&[("12", "en")]).is_empty());
}

#[test]
fn row17_duplicates_collapse_case_insensitively() {
    let c = clean(&[("1. cat", "en"), ("2. Cat", "en")]);
    assert_eq!(texts(&c), ["cat"]);
}

#[test]
fn row18_only_one_marker_is_stripped() {
    let c = clean(&[("1. 3D", "en"), ("2. cube", "en")]);
    assert_eq!(texts(&c), ["3D", "cube"], "the 3 of 3D is not a second marker (I4)");
    assert!(c[0].flags.has_digits && !c[0].checked);
    assert!(c[1].checked);
}

#[test]
fn row20_trailing_parenthetical() {
    let c = clean(&[("cat (n.)", "en")]);
    assert_eq!(texts(&c), ["cat"]);
    assert_eq!(c[0].removed_suffix.as_deref(), Some("(n.)"));
}

// ------------------------------- the rows the table could not express

/// Rows 1 and 8 want a bank match to supply the casing, but the words they
/// chose are not in the banks: `joy` is in no English tier, and neither
/// `qué` nor `hola` is in the Spanish one. Those rows therefore test the
/// trimming and nothing else. THIS is the casing rule, with words the banks
/// actually hold.
#[test]
fn d2_the_bank_supplies_the_casing_when_it_knows_the_word() {
    // English: a worksheet shouts, the bank does not.
    let c = clean(&[("1. CAT", "en"), ("2. Dog", "en")]);
    assert_eq!(texts(&c), ["cat", "dog"]);
    assert!(c.iter().all(|x| !x.flags.not_in_bank && x.checked));

    // Spanish: inverted punctuation off the front, the accent intact, and
    // the bank's lowercase applied.
    let c = clean(&[("\u{BF}Aqu\u{ED}?", "es"), ("\u{A1}Avi\u{F3}n!", "es")]);
    assert_eq!(texts(&c), ["aqu\u{ED}", "avi\u{F3}n"]);
    assert!(c.iter().all(|x| !x.flags.not_in_bank));

    // German: the reason D2 is not "lowercase everything". Nouns keep their
    // capital because the BANK keeps it, not because of a special case.
    let c = clean(&[("1. baum", "de"), ("2. BERG", "de")]);
    assert_eq!(texts(&c), ["Baum", "Berg"]);
}

/// The six inputs the I7 proptest found, one at a time, each of which broke
/// idempotence in a different way. They are named here so that a future
/// change to the pipeline order cannot quietly reintroduce any of them --
/// the proptest would find them again eventually, but a named row says what
/// went wrong and why the fixed point exists.
#[test]
fn i7_the_counterexamples_that_built_the_fixed_point() {
    let cases: [(&[&str], &[&str]); 6] = [
        // Leading whitespace hid a marker from detection.
        (&["1.cat", "  1.dog"], &["cat", "dog"]),
        // So did leading punctuation, which F3 then trimmed away.
        (&["1.cat", ",1.dog"], &["cat", "dog"]),
        // A bracket that never closes is junk, not a marker opener.
        (&["a)cat", "(n.)dog"], &["cat", "dog"]),
        // A bullet followed by a digit is junk, not a bullet marker.
        (&["\u{2022}1.cat", "1.dog"], &["cat", "dog"]),
        // Two junk characters at once needed a run, not a single skip.
        (&["a)cat", "\u{2022}(n.)dog"], &["cat", "dog"]),
        // A doubled marker: one strip per round, two rounds to settle (I4).
        (&["1.1.cat", "1.1.dog"], &["cat", "dog"]),
    ];
    for (input, want) in cases {
        let pairs: Vec<(&str, &str)> = input.iter().map(|t| (*t, "en")).collect();
        let once = clean(&pairs);
        assert_eq!(texts(&once), want, "first pass on {input:?}");
        // ...and cleaning the output again changes nothing, which is I7.
        let again: Vec<(&str, &str)> =
            once.iter().map(|c| (c.text.as_str(), "en")).collect();
        assert_eq!(texts(&clean(&again)), want, "second pass on {input:?}");
    }
}

/// I4, literally: one marker per round, and `3D` is not a marker.
#[test]
fn i4_a_round_strips_exactly_one_marker() {
    // "1." goes, the "3" of "3D" stays, because "3D" has no separator after
    // the digit and so was never a marker to begin with.
    let c = clean(&[("1. 3D", "en"), ("2. cube", "en")]);
    assert_eq!(texts(&c), ["3D", "cube"]);
    // A doubled marker loses both halves, across two rounds of the fixed
    // point rather than two strips in one.
    let c = clean(&[("1.1. cat", "en"), ("1.2. dog", "en")]);
    assert_eq!(texts(&c), ["cat", "dog"]);
}

// ------------------------------------------------------------- invariants

/// Lines drawn from real script material rather than random bytes: the point
/// is to exercise marks and letters that actually co-occur.
fn line_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            "cat", "don't", "well-being", "1.", "a)", "•", "(3)", "  ", ",", ";", "…", "¿", "?",
            "нет", "ねこ", "नमस्ते", "mèo", "l·l", "3D", "____", "(n.)", "一、", "Ω", "’",
        ]),
        0..8,
    )
    .prop_map(|v| v.join(""))
}

fn as_lines(v: &[String], lang: &str) -> Vec<OcrLine> {
    v.iter()
        .map(|t| OcrLine { text: t.clone(), confidence: 1.0, lang: lang.to_string(), ..Default::default() })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// I2 — no combining mark from the input is ever removed.
    #[test]
    fn i2_marks_survive(v in prop::collection::vec(line_strategy(), 1..4)) {
        let out = clean_ocr_lines(&as_lines(&v, "en"));
        for (line, _) in v.iter().zip(0..) {
            let marks: Vec<char> = line.chars().filter(|c| spell_wasm::snap_clean::is_mark(*c)).collect();
            if marks.is_empty() { continue; }
            // Every mark that came in on a line with letters is still somewhere
            // in that line's output.
            if !line.chars().any(spell_wasm::snap_clean::is_letter) { continue; }
            let joined: String = out.iter().map(|c| c.text.as_str()).collect();
            for m in marks {
                prop_assert!(joined.contains(m), "mark {m:?} was dropped from {line:?}");
            }
        }
    }

    /// I1 — no invented characters. Every candidate is a substring of the
    /// NFC input, allowing for the two substitutions F5 is permitted: the
    /// apostrophe fold and the bank's casing.
    #[test]
    fn i1_no_invented_characters(t in line_strategy()) {
        let out = clean_ocr_lines(&as_lines(&[t.clone()], "en"));
        let nfc: String = t.nfc().collect();
        let folded = nfc.replace('\u{2019}', "'").replace('\u{02BC}', "'");
        for c in &out {
            // Either it is literally there, or the bank supplied the casing —
            // in which case it is there case-insensitively.
            let plain = folded.contains(c.text.as_str());
            let cased = folded.to_lowercase().contains(&c.text.to_lowercase());
            prop_assert!(plain || cased, "{:?} is not a substring of {folded:?}", c.text);
        }
    }

    /// I3 — a character with a letter on each side is never removed. This is
    /// the promise that keeps don't, l'eau, well-being and Catalan l·l whole.
    #[test]
    fn i3_inside_word_characters_survive(
        a in "[a-z]{1,4}", mid in prop::sample::select(vec!["'", "-", "\u{00B7}", "\u{2019}"]), b in "[a-z]{1,4}"
    ) {
        let word = format!("{a}{mid}{b}");
        let out = clean_ocr_lines(&as_lines(&[word.clone()], "en"));
        prop_assert_eq!(out.len(), 1, "{:?} should yield exactly one candidate", word);
        // The apostrophe fold is the one permitted rewrite.
        let want = word.replace('\u{2019}', "'");
        prop_assert_eq!(&out[0].text, &want, "an inside-word character was removed");
    }

    /// I6 — flag, never delete. A line with any letter yields a candidate.
    #[test]
    fn i6_a_line_with_a_letter_always_produces_a_candidate(t in line_strategy()) {
        if !t.chars().any(spell_wasm::snap_clean::is_letter) { return Ok(()); }
        let out = clean_ocr_lines(&as_lines(&[t.clone()], "en"));
        prop_assert!(!out.is_empty(), "{t:?} produced nothing");
    }

    /// I5 — deterministic. The same input gives byte-identical output.
    #[test]
    fn i5_deterministic(v in prop::collection::vec(line_strategy(), 1..4)) {
        let a = clean_ocr_lines(&as_lines(&v, "en"));
        let b = clean_ocr_lines(&as_lines(&v, "en"));
        prop_assert_eq!(
            a.iter().map(|c| c.text.clone()).collect::<Vec<_>>(),
            b.iter().map(|c| c.text.clone()).collect::<Vec<_>>()
        );
    }

    /// I7 — idempotent. Re-running on its own output texts changes nothing.
    #[test]
    fn i7_idempotent(v in prop::collection::vec(line_strategy(), 1..4)) {
        let once = clean_ocr_lines(&as_lines(&v, "en"));
        let texts: Vec<String> = once.iter().map(|c| c.text.clone()).collect();
        let twice = clean_ocr_lines(&as_lines(&texts, "en"));
        prop_assert_eq!(
            texts,
            twice.iter().map(|c| c.text.clone()).collect::<Vec<_>>()
        );
    }
}
