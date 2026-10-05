//! CC-SNAP-LAYOUT 2.1 — the detection core's invariants.
//!
//! The 15-page Level 2 fixture does not exist, so the acceptance numbers
//! ("≤1 manual correction per page averaged") cannot run. These are the
//! claims that do not need it: the invariant, the discriminators, and the
//! fact that the whole thing is off.

use proptest::prelude::*;
use spell_wasm::snap_clean::{OcrLine, WordBox};
use spell_wasm::snap_layout::{detect, left_of, roles, LayoutConfig, LineRole, PageLayout};

fn b(text: &str, x0: f32, x1: f32) -> WordBox {
    WordBox { text: text.into(), x0, x1, confidence: 1.0, gaps: Vec::new() }
}

fn line(text: &str, boxes: Vec<WordBox>) -> OcrLine {
    OcrLine { text: text.into(), confidence: 1.0, lang: "en".into(), boxes, glyph: Some(10.0), ..Default::default() }
}

fn on() -> LayoutConfig {
    LayoutConfig { enabled: true }
}

/// `word   —   a definition of it`, the gutter always at the same x.
fn two_column_page() -> Vec<OcrLine> {
    ["cat", "dog", "fox", "owl", "bee"]
        .iter()
        .map(|w| line(&format!("{w} a small animal"), vec![
            b(w, 10.0, 40.0),
            b("a", 200.0, 210.0),
            b("small", 215.0, 265.0),
            b("animal", 270.0, 330.0),
        ]))
        .collect()
}

/// Sentences that each happen to contain one wide gap, in a DIFFERENT place.
fn ragged_sentence_page() -> Vec<OcrLine> {
    [60.0f32, 150.0, 240.0, 300.0, 110.0]
        .iter()
        .enumerate()
        .map(|(i, x)| line(&format!("sentence number {i}"), vec![
            b("sentence", 10.0, 50.0),
            b("number", *x, x + 40.0),
            b("here", x + 45.0, x + 80.0),
        ]))
        .collect()
}

#[test]
fn it_is_off_until_a_fixture_calibrates_it() {
    let cfg = LayoutConfig::baked();
    assert!(!cfg.enabled, "the baked config must not claim calibration it has not had");
    let page = two_column_page();
    let d = detect(&cfg, &page);
    assert_eq!(d.layout, PageLayout::PlainList, "a disabled detector must find nothing");
    assert_eq!(d.confidence, 0.0);
    assert!(roles(&cfg, &page, &d).iter().all(|r| *r == LineRole::Whole),
        "disabled, every line imports whole — 2.1's own fall-back");
}

#[test]
fn a_two_column_sheet_is_recognised() {
    let page = two_column_page();
    let d = detect(&on(), &page);
    match d.layout {
        PageLayout::TwoColumn { boundary_x } => {
            assert!((boundary_x - 120.0).abs() < 10.0, "gutter at {boundary_x}, expected ~120");
            assert!(d.confidence >= 0.6, "confidence {}", d.confidence);
        }
        other => panic!("expected two columns, got {other:?}"),
    }
}

/// The discriminator that matters: one wide gap per line is not a column
/// unless the gaps line up. Without this, every sentence list becomes a
/// two-column sheet and the right-hand words silently stop importing.
#[test]
fn ragged_wide_gaps_are_sentences_not_columns() {
    let d = detect(&on(), &ragged_sentence_page());
    assert_eq!(d.layout, PageLayout::PlainList, "ragged gaps must not read as columns");
}

#[test]
fn too_few_lines_is_not_a_pattern() {
    let page: Vec<OcrLine> = two_column_page().into_iter().take(3).collect();
    assert_eq!(detect(&on(), &page).layout, PageLayout::PlainList);
}

#[test]
fn a_line_with_no_boxes_cannot_vote() {
    let page = vec![line("cat", vec![]), line("dog", vec![]), line("fox", vec![])];
    let d = detect(&on(), &page);
    assert_eq!(d.layout, PageLayout::PlainList);
    assert_eq!(roles(&on(), &page, &d).len(), 3);
}

#[test]
fn the_left_column_is_what_survives_the_cut() {
    let page = two_column_page();
    let d = detect(&on(), &page);
    let PageLayout::TwoColumn { boundary_x } = d.layout else { panic!("not columns") };
    assert_eq!(left_of(&page[0], boundary_x), vec!["cat".to_string()]);
}

// ----------------------------------------------------------- fill-in-blank

#[test]
fn a_mid_line_blank_imports_nothing() {
    let page = vec![line("The ___ sat on the mat.", vec![b("The", 0.0, 20.0), b("sat", 60.0, 80.0)])];
    let r = roles(&on(), &page, &detect(&on(), &page));
    assert_eq!(r[0], LineRole::Blank, "a sentence built around a blank has no word to import");
}

/// A TRAILING blank is CLEAN F3's job, not this file's: it trims the blank
/// and keeps the word. Claiming it here would import nothing from a line
/// that has a perfectly good word on it.
#[test]
fn a_trailing_blank_is_left_to_clean() {
    let page = vec![line("Name: ______", vec![b("Name:", 0.0, 40.0)])];
    let r = roles(&on(), &page, &detect(&on(), &page));
    assert_eq!(r[0], LineRole::Whole);
}

#[test]
fn a_single_underscore_is_not_a_blank() {
    let page = vec![line("snake_case word", vec![b("snake_case", 0.0, 60.0)])];
    let r = roles(&on(), &page, &detect(&on(), &page));
    assert_eq!(r[0], LineRole::Whole);
}

#[test]
fn detection_is_deterministic() {
    let page = two_column_page();
    let first = detect(&on(), &page);
    for _ in 0..20 {
        assert_eq!(detect(&on(), &page), first);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3_000))]

    /// 2.1's invariant: detection never removes a line. One role per line,
    /// in order, whatever the page looks like and whether or not it is on.
    #[test]
    fn il1_never_removes_a_line(
        texts in prop::collection::vec("[a-z_ ]{0,24}", 0..14),
        enabled in any::<bool>(),
        xs in prop::collection::vec(0.0f32..400.0, 0..14),
    ) {
        let page: Vec<OcrLine> = texts.iter().enumerate().map(|(i, t)| {
            let x = *xs.get(i).unwrap_or(&50.0);
            line(t, vec![b("a", 0.0, 20.0), b("b", x + 30.0, x + 50.0)])
        }).collect();
        let cfg = LayoutConfig { enabled };
        let d = detect(&cfg, &page);
        let r = roles(&cfg, &page, &d);
        prop_assert_eq!(r.len(), page.len());
    }

    /// The cut never invents a token, and never reorders one.
    #[test]
    fn left_of_is_a_prefix_of_the_line(boundary in 0.0f32..400.0) {
        let l = line("cat a small animal", vec![
            b("cat", 10.0, 40.0), b("a", 200.0, 210.0),
            b("small", 215.0, 265.0), b("animal", 270.0, 330.0),
        ]);
        let kept = left_of(&l, boundary);
        let all: Vec<String> = l.boxes.iter().map(|x| x.text.clone()).collect();
        prop_assert!(kept.len() <= all.len());
        prop_assert_eq!(kept.clone(), all.iter().filter(|t| kept.contains(t)).cloned().collect::<Vec<_>>());
    }
}
