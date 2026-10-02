//! CC-SNAP-BOXES v1 Phase A — the bridge acceptance table and invariants.
//!
//! Rows 11 and 12 and invariants I-B6 and I-B8 are NOT here, and that is not an
//! omission:
//!
//! - I-B8 (probe budget) and row 11 (a 400-character line) are properties of
//!   the Swift probing loop. They belong in the Swift unit test the spec's
//!   Phase A also names, and they are not asserted by this file.
//! - I-B6 (one gap threshold) and row 12 (healSplitWords must not re-split)
//!   are Phase B. The threshold has not been hoisted yet, so there are still
//!   two constants and the invariant would fail honestly.
//!
//! What is here: every rule the core is responsible for, which is all of the
//! validation. The core's job is to be unfoolable by a bad payload, because
//! the payload comes from a platform this test cannot run.

use spell_wasm::snap_clean::{
    clean_ocr_lines, geometry_ok, split_on_box_gaps, Boundary, CharGap, Candidate, OcrLine,
    WordBox,
};

fn text_only(text: &str) -> OcrLine {
    OcrLine { text: text.into(), confidence: 1.0, lang: "en".into(), ..Default::default() }
}

/// A box with no interior probing.
fn b(text: &str, x0: f32, x1: f32) -> WordBox {
    WordBox { text: text.into(), x0, x1, confidence: 1.0, gaps: Vec::new() }
}

/// A box with interior gaps, as F1 would report them.
fn bg(text: &str, x0: f32, x1: f32, gaps: &[(usize, f32)]) -> WordBox {
    WordBox {
        text: text.into(),
        x0,
        x1,
        confidence: 1.0,
        gaps: gaps.iter().map(|(at, w)| CharGap { at: *at, w: *w }).collect(),
    }
}

fn line(text: &str, boxes: Vec<WordBox>, glyph: Option<f32>) -> OcrLine {
    OcrLine { text: text.into(), confidence: 1.0, lang: "en".into(), boxes, glyph, ..Default::default() }
}

fn texts(c: &[Candidate]) -> Vec<String> {
    c.iter().map(|x| x.text.clone()).collect()
}

// ------------------------------------------------------------------ the table

#[test]
fn row1_no_geometry_is_the_identity() {
    // I-B1. The shape every build before CC-SNAP-BOXES sent.
    for t in ["cat", "1. horse", "big red dog", "Name: ______", "3D"] {
        let with = clean_ocr_lines(&[text_only(t)]);
        let bare = clean_ocr_lines(&[OcrLine {
            text: t.into(),
            confidence: 1.0,
            lang: "en".into(),
            ..Default::default()
        }]);
        assert_eq!(texts(&with), texts(&bare), "{t}");
    }
}

#[test]
fn row2_three_boxes_with_real_gaps() {
    // Already-spaced text: the boxes agree with the string, nothing changes.
    let l = line(
        "big red dog",
        vec![b("big", 0.10, 0.20), b("red", 0.26, 0.36), b("dog", 0.42, 0.52)],
        Some(0.033),
    );
    assert_eq!(split_on_box_gaps(&l), ["big", "red", "dog"]);
    let out = clean_ocr_lines(&[l]);
    assert_eq!(texts(&out), ["big", "red", "dog"]);
    assert!(out.iter().all(|c| c.unbunch.is_none()), "box evidence needs no suggestion");
}

#[test]
fn row3_interior_gaps_split_a_merged_token() {
    // THE point of the file. One box, one token, no space in the string --
    // and the per-character probe found the boundaries anyway.
    let l = line("bigreddog", vec![bg("bigreddog", 0.10, 0.40, &[(3, 0.02), (6, 0.02)])], Some(0.033));
    assert_eq!(split_on_box_gaps(&l), ["big", "red", "dog"]);
    assert_eq!(texts(&clean_ocr_lines(&[l])), ["big", "red", "dog"]);
}

#[test]
fn row4_no_interior_profile_leaves_the_token_whole() {
    // Budget spent, or token too short to probe. One candidate, and the
    // dictionary path may offer a split -- which is where it is currently
    // switched off (CC-SNAP-CLEAN v1.1 AUTO_SPLIT_ENABLED).
    let l = line("bigreddog", vec![b("bigreddog", 0.10, 0.40)], Some(0.033));
    assert_eq!(split_on_box_gaps(&l), ["bigreddog"]);
    let out = clean_ocr_lines(&[l]);
    assert_eq!(texts(&out), ["bigreddog"]);
}

#[test]
fn row5_boxes_out_of_order_are_rejected_whole() {
    let l = line("big red", vec![b("big", 0.30, 0.40), b("red", 0.10, 0.20)], Some(0.033));
    assert!(!geometry_ok(&l));
    assert_eq!(split_on_box_gaps(&l), ["big red"], "falls back to text");
}

#[test]
fn row6_inverted_box_is_rejected() {
    let l = line("big", vec![b("big", 0.40, 0.10)], Some(0.033));
    assert!(!geometry_ok(&l));
    assert_eq!(split_on_box_gaps(&l), ["big"]);
}

#[test]
fn row7_out_of_range_coordinate_is_rejected() {
    let l = line("big", vec![b("big", 0.10, 1.4)], Some(0.033));
    assert!(!geometry_ok(&l));
}

#[test]
fn row8_nan_is_rejected() {
    // Written so that a range check phrased the other way round would let it
    // through: NaN fails every comparison, including against itself.
    let l = line("big", vec![b("big", f32::NAN, 0.4)], Some(0.033));
    assert!(!geometry_ok(&l));
    let g = line("big", vec![bg("bigred", 0.1, 0.4, &[(3, f32::NAN)])], Some(0.033));
    assert!(!geometry_ok(&g));
    let y = line("big", vec![b("big", 0.1, 0.4)], Some(f32::NAN));
    assert!(!geometry_ok(&y));
}

#[test]
fn row9_boxes_that_rejoin_to_the_line_are_accepted() {
    // I-B5, the passing direction.
    let l = line("big red", vec![b("big", 0.10, 0.20), b("red", 0.26, 0.36)], Some(0.033));
    assert!(geometry_ok(&l));
}

#[test]
fn row10_boxes_that_disagree_with_the_line_are_rejected() {
    // I-B5, the catching direction: a tokenizer that drifts from the geometry
    // it describes. This is why the Swift side shares ONE tokenizer.
    let l = line("big red", vec![b("big", 0.10, 0.20), b("blue", 0.26, 0.36)], Some(0.033));
    assert!(!geometry_ok(&l));
    assert_eq!(split_on_box_gaps(&l), ["big red"]);
}

// ------------------------------------------------------------- the invariants

#[test]
fn i_b2_geometry_never_removes_a_line() {
    // It may change where a line divides; the page keeps all its content.
    let l = line("bigreddog", vec![bg("bigreddog", 0.1, 0.4, &[(3, 0.02), (6, 0.02)])], Some(0.033));
    let split = clean_ocr_lines(&[l]);
    let whole = clean_ocr_lines(&[text_only("bigreddog")]);
    assert!(split.len() >= whole.len());
    assert_eq!(split.concat_text(), "bigreddog");
}

trait ConcatText {
    fn concat_text(&self) -> String;
}
impl ConcatText for Vec<Candidate> {
    fn concat_text(&self) -> String {
        self.iter().map(|c| c.text.as_str()).collect()
    }
}

#[test]
fn i_b3_gaps_must_sit_inside_their_token() {
    // at == 0 would mean "before the first character", which is not a
    // boundary; at >= len would point past the end.
    for bad in [0usize, 9, 12] {
        let l = line("bigreddog", vec![bg("bigreddog", 0.1, 0.4, &[(bad, 0.02)])], Some(0.033));
        assert!(!geometry_ok(&l), "gap at {bad} must be rejected");
    }
}

#[test]
fn i_b3_gaps_must_ascend() {
    let l = line("bigreddog", vec![bg("bigreddog", 0.1, 0.4, &[(6, 0.02), (3, 0.02)])], Some(0.033));
    assert!(!geometry_ok(&l), "unordered gaps make the split order payload-dependent");
    let dup = line("bigreddog", vec![bg("bigreddog", 0.1, 0.4, &[(3, 0.02), (3, 0.02)])], Some(0.033));
    assert!(!geometry_ok(&dup));
}

#[test]
fn i_b4_empty_geometry_is_not_geometry() {
    assert!(!geometry_ok(&text_only("cat")));
}

#[test]
fn gap_below_the_threshold_does_not_split() {
    // 0.35 x glyph is the bar (D-U4). A hair under it is a solid word.
    let glyph = 0.033_f32;
    let under = glyph * 0.34;
    let l = line("bigred", vec![bg("bigred", 0.1, 0.3, &[(3, under)])], Some(glyph));
    assert_eq!(split_on_box_gaps(&l), ["bigred"]);
    let over = glyph * 0.36;
    let m = line("bigred", vec![bg("bigred", 0.1, 0.3, &[(3, over)])], Some(glyph));
    assert_eq!(split_on_box_gaps(&m), ["big", "red"]);
}

#[test]
fn measured_glyph_beats_the_estimate() {
    // D-B3. Same boxes, same gap; only the carried glyph width differs, and
    // it decides. Without the measurement the core divides box width by
    // character count, which a proportional font makes a guess.
    let gaps = &[(3usize, 0.010_f32)];
    let tight = line("bigred", vec![bg("bigred", 0.1, 0.3, gaps)], Some(0.100));
    assert_eq!(split_on_box_gaps(&tight), ["bigred"], "huge glyph: 0.010 is not a space");
    let loose = line("bigred", vec![bg("bigred", 0.1, 0.3, gaps)], Some(0.010));
    assert_eq!(split_on_box_gaps(&loose), ["big", "red"], "small glyph: 0.010 is a space");
}

#[test]
fn zh_splits_on_box_evidence() {
    // Row 14. D-U1's `never` governs the DICTIONARY path only; a gap on the
    // page is a gap in any script.
    let l = OcrLine {
        text: "一样朋友".into(),
        confidence: 1.0,
        lang: "zh".into(),
        boxes: vec![bg("一样朋友", 0.1, 0.5, &[(2, 0.05)])],
        glyph: Some(0.100),
        ..Default::default()
    };
    assert_eq!(split_on_box_gaps(&l), ["一样", "朋友"]);
}


#[test]
fn row12_a_tight_gap_merges_and_is_not_re_split() {
    // Phase B. "soft ware" is the case healSplitWords exists for: Vision
    // splits a solid word on a small handwriting gap. The core joins boxes
    // below the threshold, so the same comparison that splits a wide gap
    // closes a narrow one -- one rule, not two that can both fire.
    let glyph = 0.030_f32;
    let gap = glyph * 0.20; // well under split_gap (0.35)
    let l = line(
        "soft ware",
        vec![b("soft", 0.10, 0.22), b("ware", 0.22 + gap, 0.34 + gap)],
        Some(glyph),
    );
    assert_eq!(split_on_box_gaps(&l), ["software"], "a tight gap is not a space");
    // And it stays merged: nothing downstream re-divides it.
    assert_eq!(texts(&clean_ocr_lines(&[l])), ["software"], "and nothing downstream re-divides it");
}

#[test]
fn the_threshold_comes_from_the_one_config_file() {
    // I-B6 from the Rust side: the value the splitter uses must be the value
    // config/snap-geometry.json carries. scripts/snap-geometry-check.mjs
    // covers the Swift half, which this test cannot reach.
    let cfg = std::fs::read_to_string("config/snap-geometry.json").expect("the one source");
    let want: f32 = cfg
        .split("\"split_gap\"")
        .nth(1)
        .and_then(|r| r.trim_start().trim_start_matches(':').trim().split(',').next())
        .and_then(|v| v.trim().parse().ok())
        .expect("split_gap is a number");
    // Drive the splitter either side of the configured ratio.
    let glyph = 0.030_f32;
    let under = line("ab", vec![bg("ab", 0.1, 0.2, &[(1, glyph * (want - 0.01))])], Some(glyph));
    let over = line("ab", vec![bg("ab", 0.1, 0.2, &[(1, glyph * (want + 0.01))])], Some(glyph));
    assert_eq!(split_on_box_gaps(&under), ["ab"], "just under the configured ratio");
    assert_eq!(split_on_box_gaps(&over), ["a", "b"], "just over it");
}


// ------------------------------------------------- Phase C: the merge, moved

/// Two tokens separated by `gap`, with the platform's three verdicts.
fn pair(left: &str, right: &str, gap: f32, avg: f32, v: (bool, bool, bool)) -> OcrLine {
    let w = 0.12_f32;
    OcrLine {
        text: format!("{left} {right}"),
        confidence: 1.0,
        lang: "en".into(),
        boxes: vec![b(left, 0.10, 0.10 + w), b(right, 0.10 + w + gap, 0.10 + 2.0 * w + gap)],
        glyph: Some(avg),
        avg_char: Some(avg),
        has_dict: true,
        bounds: vec![Boundary { at: 0, left: v.0, right: v.1, joined: v.2 }],
        ..Default::default()
    }
}

#[test]
fn phase_c_software_still_heals() {
    // The case healSplitWords was written for: Vision splits a solid word on
    // a small handwriting gap. Moderate gap, and the join is a real word.
    let avg = 0.030_f32;
    let l = pair("soft", "ware", avg * 0.9, avg, (false, true, true));
    assert_eq!(split_on_box_gaps(&l), ["software"]);
}

#[test]
fn phase_c_cat_dog_still_stays_two_words() {
    // The OVER-MERGE GUARD, from a device report: five words written close
    // together imported as ONE entry. Both sides real, join is not a word --
    // no gap, however cramped, may merge them.
    let avg = 0.030_f32;
    for ratio in [0.9_f32, 0.4, 0.1, 0.01] {
        let l = pair("cat", "dog", avg * ratio, avg, (true, true, false));
        assert_eq!(split_on_box_gaps(&l), ["cat", "dog"], "gap ratio {ratio}");
    }
}

#[test]
fn phase_c_a_fragment_side_heals_on_a_tight_gap() {
    // "sof tware": one side is not a word, so a TIGHT gap merges even though
    // the join was not looked up favourably.
    let avg = 0.030_f32;
    let tight = pair("sof", "tware", avg * 0.4, avg, (false, false, false));
    assert_eq!(split_on_box_gaps(&tight), ["software"]);
    // ... but a merely moderate gap does not.
    let moderate = pair("sof", "tware", avg * 0.9, avg, (false, false, false));
    assert_eq!(split_on_box_gaps(&moderate), ["sof", "tware"]);
}

#[test]
fn phase_c_without_a_dictionary_geometry_stands_alone() {
    let avg = 0.030_f32;
    let mut l = pair("一样", "朋友", avg * 0.2, avg, (false, false, false));
    l.lang = "zh".into();
    l.has_dict = false;
    l.bounds.clear();
    assert_eq!(split_on_box_gaps(&l), ["一样朋友"], "0.2 is under merge_nodict (0.25)");
    let mut wide = pair("一样", "朋友", avg * 0.3, avg, (false, false, false));
    wide.lang = "zh".into();
    wide.has_dict = false;
    wide.bounds.clear();
    assert_eq!(split_on_box_gaps(&wide), ["一样", "朋友"], "0.3 is over it");
}

#[test]
fn phase_c_a_healed_line_keeps_probing_its_other_tokens() {
    // THE point of Phase C. Before it, a line needing one merge was healed on
    // the platform, its text no longer matched the raw candidate, and the core
    // dropped the whole line's geometry -- so the merged word was fixed and
    // every other token on the line went unprobed.
    let avg = 0.030_f32;
    let w = 0.12_f32;
    let l = OcrLine {
        text: "soft ware bigreddog".into(),
        confidence: 1.0,
        lang: "en".into(),
        boxes: vec![
            b("soft", 0.10, 0.10 + w),
            b("ware", 0.10 + w + avg * 0.9, 0.10 + 2.0 * w + avg * 0.9),
            bg("bigreddog", 0.50, 0.80, &[(3, avg * 0.5), (6, avg * 0.5)]),
        ],
        glyph: Some(avg),
        avg_char: Some(avg),
        has_dict: true,
        bounds: vec![
            Boundary { at: 0, left: false, right: true, joined: true },
            Boundary { at: 1, left: true, right: false, joined: false },
        ],
        ..Default::default()
    };
    // The merge happened AND the third token was still probed.
    assert_eq!(split_on_box_gaps(&l), ["software", "big", "red", "dog"]);
}

#[test]
fn phase_c_merge_uses_avg_char_not_the_median_glyph() {
    // The two denominators are different quantities and the merge keeps the
    // one it has always used. Same gap, same boxes; only which measure the
    // line carries changes the verdict.
    let gap = 0.030_f32 * 0.8;
    let mut narrow = pair("soft", "ware", gap, 0.030, (false, true, true));
    narrow.glyph = Some(0.001); // a median that would make the gap enormous
    assert_eq!(split_on_box_gaps(&narrow), ["software"], "avg_char decides the merge");
}

#[test]
fn i_b7_no_pixels_in_the_bridge_payload() {
    // The payload schema must admit no image. Asserted against the Swift
    // source because that is where the payload is built; a Rust test cannot
    // run the plugin, but it can read it.
    let src = std::fs::read_to_string("ios/App/App/NativeLanguageKitPlugin+PhotoList.swift")
        .expect("the plugin source is part of this repo");
    let start = src.find("func finish(resolve").expect("the resolve path exists");
    // Stop at the next declaration, or the window runs into neighbours that
    // legitimately mention image types (cgOrientation takes a CGImage).
    let rest = &src[start..];
    let end = rest[1..].find("    fileprivate func").map(|i| i + 1).unwrap_or(rest.len());
    let body = &rest[..end];
    for banned in ["base64", "jpegData", "pngData", "data:image", "CGImage", "UIImage"] {
        assert!(!body.contains(banned), "the resolve payload must not carry pixels: {banned}");
    }
}
