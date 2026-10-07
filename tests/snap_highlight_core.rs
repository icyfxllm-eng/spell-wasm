//! CC-SNAP-HIGHLIGHT v1 — the scoring core's invariants.
//!
//! The 20-photo C2 fixture does not exist, so the acceptance table in
//! `docs/CC-SNAP-HIGHLIGHT.md` cannot run. These are the invariants that do
//! not need it: the ones about arithmetic and about failing safely. When the
//! fixture lands, `tests/snap_highlight_fixture.rs` runs the acceptance table
//! beside these; it does not replace them.

use proptest::prelude::*;
use spell_wasm::snap_highlight::{
    hue_bucket, rejoin_hyphens, score_page, BoxStats, HighlightConfig, Joined, NotScored,
    PageStats, Rejected, Source, Word,
};

const N: usize = 32;

/// A calibrated config, so the scoring path is exercised. The baked one is
/// deliberately uncalibrated; see `uncalibrated_scores_nothing`.
fn cfg() -> HighlightConfig {
    let mut c = HighlightConfig::baked();
    c.calibrated = true;
    c.min_paper_px = Some(500);
    c
}

/// A box whose background is `frac` marked, at saturation `sat`, the rest at
/// saturation 0.
fn boxed(frac: f32, sat: f32, hue: Option<u16>) -> BoxStats {
    boxed_v(frac, sat, hue, Some(0.05))
}

/// The same, with an explicit "how far below the paper the marked pixels sit".
/// 0.05 is a real highlighter; 0.53 is the blue heading that started this.
fn boxed_v(frac: f32, sat: f32, hue: Option<u16>, v_drop: Option<f32>) -> BoxStats {
    let total = 1000u32;
    let marked = (total as f32 * frac).round() as u32;
    let mut hist = vec![0u32; N];
    let bucket = ((sat * N as f32) as usize).min(N - 1);
    hist[bucket] += marked;
    hist[0] += total - marked;
    BoxStats { bg_px: total, sat_hist: hist, hue_deg: hue, mark_v_drop: v_drop }
}

fn page(paper_s: f32, source: Source) -> PageStats {
    PageStats { paper_s, paper_v: 1.0, paper_sample_px: 5_000, source }
}

#[test]
fn uncalibrated_scores_nothing_and_says_so() {
    // The baked config has min_paper_px: null until the C2 fixture exists.
    let c = HighlightConfig::baked();
    assert!(!c.calibrated, "the baked config must not claim calibration it has not had");
    assert!(c.min_paper_px.is_none(), "min_paper_px is null until the fixture sets it");
    let s = score_page(&c, &page(0.05, Source::Camera), &[boxed(1.0, 0.9, Some(55))]);
    assert_eq!(s.count(), 0, "an uncalibrated detector must find nothing");
    assert_eq!(s.not_scored, Some(NotScored::Uncalibrated));
}

#[test]
fn a_clearly_marked_word_is_found() {
    let s = score_page(&cfg(), &page(0.05, Source::Camera), &[boxed(0.9, 0.8, Some(55))]);
    assert_eq!(s.count(), 1);
    assert_eq!(s.not_scored, None);
}

/// I-H2 — a uniformly warm or aged page produces N = 0. The cut is relative
/// to the paper, so tinting the whole page lifts the paper estimate by the
/// same amount it lifts every box and the difference does not move.
#[test]
fn ih2_a_uniformly_tinted_page_finds_nothing() {
    for tint in [0.0f32, 0.1, 0.2, 0.3, 0.4] {
        // Every box's background sits AT the paper saturation: that is what
        // "uniformly tinted" means.
        let boxes: Vec<BoxStats> = (0..20).map(|_| boxed(1.0, tint, Some(40))).collect();
        let s = score_page(&cfg(), &page(tint, Source::Camera), &boxes);
        assert_eq!(s.count(), 0, "tint {tint} invented a highlight on plain paper");
    }
}

/// I-H3 — the coverage threshold is D-H1's 0.50, in frac_sat.
#[test]
fn ih3_partial_coverage_counts_at_half() {
    let c = cfg();
    let p = page(0.05, Source::Camera);
    assert_eq!(score_page(&c, &p, &[boxed(0.60, 0.8, None)]).count(), 1, "60% must count");
    assert_eq!(score_page(&c, &p, &[boxed(0.30, 0.8, None)]).count(), 0, "30% must not");
    assert_eq!(score_page(&c, &p, &[boxed(0.51, 0.8, None)]).count(), 1, "just over half counts");
    assert_eq!(score_page(&c, &p, &[boxed(0.49, 0.8, None)]).count(), 0, "just under does not");
}

/// Too little paper to estimate from is a reason to show everything, not a
/// reason to guess. This is F1's weak spot on a dense page or a cropped
/// screenshot, and the review record raises it.
#[test]
fn too_little_paper_scores_nothing() {
    let mut p = page(0.05, Source::Camera);
    p.paper_sample_px = 499; // below the configured 500
    let s = score_page(&cfg(), &p, &[boxed(1.0, 0.9, Some(55))]);
    assert_eq!(s.count(), 0);
    assert_eq!(s.not_scored, Some(NotScored::PaperSampleTooSmall));
}

#[test]
fn an_empty_box_is_not_highlighted() {
    let b = BoxStats { bg_px: 0, sat_hist: vec![0; N], hue_deg: Some(55), mark_v_drop: Some(0.05) };
    let s = score_page(&cfg(), &page(0.05, Source::Camera), &[b]);
    assert_eq!(s.count(), 0, "a box with no background pixels cannot be marked");
    assert_eq!(s.words[0].frac_sat, 0.0);
}

/// D-H3 — the two sources carry their own ΔS. They are equal today; the
/// fixture (C4) may separate them, and this pins that the path exists.
#[test]
fn dh3_each_source_uses_its_own_delta() {
    let mut c = cfg();
    c.delta_s_camera = 0.25;
    c.delta_s_screenshot = 0.70;
    let b = [boxed(1.0, 0.5, None)];
    assert_eq!(score_page(&c, &page(0.05, Source::Camera), &b).count(), 1);
    assert_eq!(
        score_page(&c, &page(0.05, Source::Screenshot), &b).count(),
        0,
        "the screenshot threshold must be the one applied to a screenshot"
    );
}

#[test]
fn hue_buckets_wrap_and_stay_in_range() {
    for deg in 0..=720u16 {
        let b = hue_bucket(deg, 12);
        assert!(b < 12, "hue {deg} fell outside the bucket range");
    }
    assert_eq!(hue_bucket(0, 12), hue_bucket(360, 12), "360 wraps to 0");
}

#[test]
fn hue_is_only_reported_for_a_highlighted_word() {
    let s = score_page(&cfg(), &page(0.05, Source::Camera), &[boxed(0.1, 0.8, Some(55))]);
    assert!(!s.words[0].highlighted);
    assert_eq!(s.words[0].hue_bucket, None, "an unmarked word has no colour chip");
}

/// I-H5 — same input, same output. No clock, no randomness.
#[test]
fn ih5_scoring_is_deterministic() {
    let boxes: Vec<BoxStats> = (0..50)
        .map(|i| boxed(i as f32 / 50.0, 0.7, Some((i * 7) as u16)))
        .collect();
    let p = page(0.05, Source::Camera);
    let first = score_page(&cfg(), &p, &boxes);
    for _ in 0..20 {
        let again = score_page(&cfg(), &p, &boxes);
        assert_eq!(first.words, again.words, "scoring is not deterministic");
    }
}

/// The coloured-heading false positive, which a real fixture page carried.
/// Saturation alone says yes; the pixels are far darker than the paper, so it
/// is type, not a mark.
#[test]
fn a_coloured_heading_is_not_a_highlight() {
    let heading = boxed_v(1.0, 0.8, Some(200), Some(0.53));
    let s = score_page(&cfg(), &page(0.0, Source::Screenshot), &[heading]);
    assert_eq!(s.count(), 0, "a blue section heading must not import as a highlight");
    assert_eq!(s.words[0].rejected, Some(Rejected::TooDarkForAMark));
    assert!(s.words[0].frac_sat >= 0.5, "and it was rejected DESPITE the coverage, not for lack of it");
}

/// A deep gutter shadow in a photo of an open book is saturated and very
/// dark; it must not read as a mark either.
#[test]
fn a_gutter_shadow_is_not_a_highlight() {
    let shadow = boxed_v(1.0, 0.8, Some(200), Some(0.77));
    assert_eq!(score_page(&cfg(), &page(0.09, Source::Camera), &[shadow]).count(), 0);
}

/// delta_s HAS NO VALID VALUE, and not because the window is narrow -- the
/// ordering is INVERTED. Every number below is Display P3 (the fixture is
/// normalised to it), and every one is the same statistic: the MEDIAN S of
/// the mark box's brightest 40%, minus the MODE S of a paper box's.
///
///     0.125  FIND    row 21  pale pink mark        screenshot
///     0.129  FIND    row 25  cyan marker           photo
///     0.153  REFUSE  row 24  yellow callout        photo
///     0.157  REFUSE  row 20  yellow callout        screenshot
///     0.188  FIND    row 10  pink on cream         photo
///     0.341  FIND    row 22  vivid pink            photo
///     0.400  FIND    row 23  neon pink             photo
///     0.471  FIND    row 11  Kindle yellow         screenshot
///     0.698  FIND    row 12  Apple Books           screenshot
///
/// Two real marks sit BELOW two things that must be refused, which sit below
/// five more real marks. No threshold separates those sets at any value, and
/// it is not an artefact of mixing sources -- it holds within each:
///
///     camera:      0.129 find < 0.153 refuse < 0.188 find
///     screenshot:  0.125 find < 0.157 refuse < 0.471 find
///
/// The current 0.25 refuses both boxes and loses four real marks, and the
/// Intent says losing a mark is the wrong way to fail.
///
/// What separates them is size, not colour -- a highlight is a word wide and
/// a line tall, a callout box is a rectangle over many lines. Unbuilt, and
/// it is new spec for F2.
///
/// HISTORY, because these numbers moved once. Before the fixture was
/// normalised to one colour space these read 0.196 for row 20 and 0.27 for
/// row 11, and rows in sRGB were being compared with rows in P3. The shape
/// of the finding did not change; the figures did. Row 12 is the one that
/// gained information: in sRGB it was CLIPPED at S 1.000 and is 0.698 here.
#[test]
fn delta_s_cannot_work_at_all_the_ordering_is_inverted() {
    let c = cfg();
    let shot = |d: f32, hue: u16| score_page(&c, &page(0.0, Source::Screenshot), &[boxed(1.0, d, Some(hue))]).count();
    let cam = |paper: f32, d: f32, hue: u16| score_page(&c, &page(paper, Source::Camera), &[boxed(1.0, paper + d, Some(hue))]).count();

    // The two things that MUST be refused. Both are, today.
    assert_eq!(shot(0.157, 55), 0, "row 20's callout imported -- delta_s fell below it");
    assert_eq!(cam(0.0, 0.153, 50), 0, "row 24's photographed callout imported");

    // Four real marks BELOW or NEAR them that must be found, and are not.
    assert_eq!(shot(0.125, 350), 0, "row 21's pale pink is found -- saturation alone cannot have done it");
    assert_eq!(cam(0.157, 0.129, 172), 0, "row 25's cyan is found -- the inversion would be solved");
    assert_eq!(cam(0.145, 0.188, 352), 0, "row 10's pink on cream is found -- say how");

    // And the vivid end, which is found. Colour is not the problem.
    assert_eq!(shot(0.471, 53), 1, "row 11's Kindle yellow was lost -- a regression, not a known gap");
    assert_eq!(cam(0.118, 0.341, 345), 1, "row 22's vivid pink was lost -- a regression");
}

/// Fixture row 3: a printed sheet under a warm lamp, no highlighter on it.
///
/// Saturation survives the lamp easily. The page's paper sits at S 0.075 and
/// the most saturated BRIGHT pixel anywhere on it reaches 0.20 -- six pixels
/// of them -- so even a box entirely filled at the page's own worst colour is
/// 0.125 above paper, half of delta_s. Warm light does not manufacture marks.
///
/// This is the MEASURED version of `ih2_a_uniformly_tinted_page_finds_nothing`
/// above, which idealises the tint as perfectly uniform. A real lamp is not
/// uniform, and the gap between the paper and the page's warmest pixel is the
/// part that test assumes away.
#[test]
fn a_warm_lamp_does_not_manufacture_a_highlight() {
    let c = cfg();
    let lamp = page(0.075, Source::Camera);
    assert_eq!(score_page(&c, &lamp, &[boxed(1.0, 0.20, Some(45))]).count(), 0,
        "the warmest pixel on an unmarked lamp-lit page scored as a mark");
}

/// What the lamp DOES move is hue, and that is why hue cannot be promoted to
/// the primary test now that saturation has been shown not to work.
///
///   fixture row 3, BLANK paper under a lamp:   43-50 deg
///   fixture row 12, Apple Books highlight:        50 deg
///   fixture row 11, Kindle highlight:             53 deg
///
/// Blank paper is nearer Apple Books' highlight hue than Apple Books is to
/// Kindle's, and at 12 buckets all three are the same chip. Hue is for
/// NAMING a mark under F5, never for finding one.
#[test]
fn blank_warm_paper_lands_in_the_same_hue_chip_as_two_real_highlights() {
    let paper_lo = hue_bucket(43, 12);
    let paper_hi = hue_bucket(50, 12);
    let apple = hue_bucket(50, 12);
    let kindle = hue_bucket(53, 12);
    assert_eq!(paper_lo, paper_hi, "row 3's own tint spans two chips");
    assert_eq!(paper_hi, apple, "blank lamp-lit paper is not Apple Books' chip -- re-measure");
    assert_eq!(apple, kindle, "the two e-reader highlights are not one chip -- re-measure");
}

/// Rows 20 and 24 are the same card, screen-captured and then photographed.
/// The camera costs saturation, and not by a constant:
///
///   yellow callout   screenshot 0.157 -> photo 0.153   ( 5% off)
///   blue callout     screenshot 0.090 -> photo 0.016   (83% off)
///
/// Those were 22% and 86% before the fixture was normalised to one colour
/// space, with row 20 in a MONITOR profile and row 24 in P3. Most of the
/// yellow "loss" was the colour space. The hue dependence is what survives,
/// and as a ratio it is starker: 5% against 83%.
///
/// D-H3's per-source delta_s was kept "available rather than deleted as
/// unused". This is why it is needed -- and also why a per-source SCALAR is
/// not enough, because one number cannot be 22% off yellow and 86% off blue.
///
/// Row 25 later narrowed what this means: the 86% loss is specific to
/// photographing a GLOWING SCREEN through room light. A real cyan marker
/// photographed on paper keeps its colour (delta_s 0.125-0.137). So this
/// pair measures the screen-photo path, not cameras in general.
#[test]
fn the_camera_path_costs_saturation_and_not_by_a_constant() {
    let c = cfg();
    // Both sources, both callouts, all four correctly refused at 0.25 today.
    for (src, delta) in [(Source::Screenshot, 0.157), (Source::Camera, 0.153)] {
        assert_eq!(score_page(&c, &page(0.0, src), &[boxed(1.0, delta, Some(50))]).count(), 0,
            "the yellow callout imported at delta_s {delta}");
    }
    // The blue callout photographed is indistinguishable from paper. If a real
    // blue marker loses saturation the same way, nothing finds it. Row 17 --
    // a cyan marker PHOTOGRAPHED -- is the unfiled shot that would say.
    assert_eq!(score_page(&c, &page(0.0, Source::Camera), &[boxed(1.0, 0.016, Some(192))]).count(), 0,
        "a box 0.016 above paper scored as a mark");
}

/// The camera path HAS inverted too. Row 25 refuted this test's predecessor.
///
/// An earlier version of this test asserted that a camera-only delta_s of
/// 0.18 separated everything measured: refuse row 24's callout at 0.153,
/// find row 10's pink. Row 25 -- a real cyan marker, photographed -- lands
/// at 0.129, BELOW the thing that must be refused. All Display P3, which
/// the fixture is now normalised to:
///
///   0.129  row 25, cyan marker            FIND
///   0.153  row 24, yellow callout         REFUSE
///   0.188  row 10, pink on cream          FIND
///
/// So the inversion is not a property of screenshots. It holds within each
/// source separately, and no per-source scalar fixes it. This test exists to
/// stop the window being proposed a third time.
#[test]
fn no_camera_only_threshold_can_work_either() {
    let c = cfg();
    let camera = |d: f32| score_page(&c, &page(0.0, Source::Camera), &[boxed(1.0, d, Some(170))]).count();
    // Whatever the threshold, a cut that finds row 25 also imports row 24.
    for cut in [0.10f32, 0.13, 0.15, 0.18, 0.20, 0.25] {
        let mut c2 = cfg();
        c2.delta_s_camera = cut;
        let finds_cyan = score_page(&c2, &page(0.0, Source::Camera), &[boxed(1.0, 0.129, Some(170))]).count() == 1;
        let refuses_callout = score_page(&c2, &page(0.0, Source::Camera), &[boxed(1.0, 0.153, Some(50))]).count() == 0;
        assert!(!(finds_cyan && refuses_callout),
            "a camera delta_s of {cut} both found row 25 and refused row 24 -- \
             the inversion is solved and this test should be rewritten");
    }
    // And as shipped, at 0.25, the cyan marker is simply missed.
    assert_eq!(camera(0.129), 0, "row 25's cyan is found at the shipped camera delta_s");
}

/// KNOWN GAP, pinned so it cannot be forgotten or silently "fixed".
///
/// Fixture row 16 is a Kindle page whose blue headings drop only 0.32 below
/// paper. That is UNDER max_v_drop, so the fill rule passes them, and they
/// cover 4.3% of the page in large type, so coverage will not save it the
/// way it saves thin ink. This page imports its section titles today.
///
/// No threshold fixes it. Row 11's heading measured 0.53 and row 16's is
/// 0.32, so coloured headings span 0.32-0.53 while real marks span
/// 0.00-0.35 (the dimmest being a cyan marker at 0.34). They OVERLAP.
/// Lowering max_v_drop to catch 0.32 would reject that marker.
///
/// The real difference is structural: a highlight's saturated pixels sit in
/// the gaps BETWEEN glyphs, while coloured type IS the glyphs. A stroke-aware
/// text mask removes the heading and leaves the highlight; a brightness
/// threshold cannot tell them apart. That is platform image work.
///
/// This test asserts the WRONG answer on purpose. When the mask is fixed it
/// will fail, and whoever fixes it should flip it then.
#[test]
fn known_gap_a_light_coloured_heading_still_reads_as_a_highlight() {
    let heading = boxed_v(1.0, 0.8, Some(210), Some(0.32));
    let s = score_page(&cfg(), &page(0.0, Source::Screenshot), &[heading]);
    assert_eq!(s.count(), 1,
        "if this now passes, the mask has been fixed -- flip this test and delete the gap note");
}

/// Thin ink in a DIM photo is the case the fill rule does NOT catch, and it
/// is coverage that saves the page. Measured on fixture row 19: blue biro on
/// ruled paper, paper V 0.580, ink 0.38 below it -- under max_v_drop, so the
/// fill test passes it -- but 0.03% of the page, so no box comes near D-H1.
///
/// Pinned because the temptation on reading row 19 is to tighten
/// max_v_drop, and that would reject real markers: the dimmest real mark
/// measured is a cyan marker at 0.34, four hundredths away.
#[test]
fn thin_ink_in_a_dim_photo_fails_on_coverage_not_on_the_fill_test() {
    let ink = boxed_v(0.04, 0.8, Some(220), Some(0.38));
    let s = score_page(&cfg(), &page(0.17, Source::Camera), &[ink]);
    assert_eq!(s.count(), 0, "thin ink must not import");
    assert_eq!(s.words[0].rejected, None,
        "and it must be coverage that refused it, not the fill test");
}

/// Pen strikethrough falls out of the same rule, which is what D-H6 wanted.
#[test]
fn pen_ink_is_not_a_highlight_either() {
    let ink = boxed_v(1.0, 0.8, Some(5), Some(0.59));
    assert_eq!(score_page(&cfg(), &page(0.15, Source::Camera), &[ink]).count(), 0);
}

/// The four real marks measured on 2026-10-06 must all still read as marks.
/// Apple Books 0.00, Kindle 0.05, a cyan marker photographed on a form 0.35.
#[test]
fn every_measured_real_mark_survives_the_fill_test() {
    for (who, drop) in [("apple books", 0.00f32), ("kindle", 0.05),
                        ("yellow highlighter on cream book paper", 0.08),
                        ("cyan marker on a photo", 0.35)] {
        let m = boxed_v(0.9, 0.8, Some(53), Some(drop));
        assert_eq!(score_page(&cfg(), &page(0.04, Source::Camera), &[m]).count(), 1,
            "{who} (V drop {drop}) stopped being a highlight");
    }
}

/// A shim that does not report the drop must not silently lose every mark.
#[test]
fn an_unmeasured_box_is_judged_on_saturation_alone() {
    let b = boxed_v(0.9, 0.8, Some(53), None);
    assert_eq!(score_page(&cfg(), &page(0.0, Source::Camera), &[b]).count(), 1);
}

// ------------------------------------------------------------------- F3

fn w(text: &str, line: usize, hl: bool) -> Word {
    Word { text: text.to_string(), line, highlighted: hl }
}

#[test]
fn f3_joins_only_highlighted_to_highlighted() {
    let joined = rejoin_hyphens(&[w("remem-", 0, true), w("ber", 1, true)]);
    assert_eq!(joined.len(), 1);
    match &joined[0] {
        Joined::Hyphen { text, left, right, .. } => {
            assert_eq!(text, "remember");
            assert_eq!(left, "remem-");
            assert_eq!(right, "ber");
        }
        other => panic!("expected a join, got {other:?}"),
    }

    // One half marked: two entries, the reader decides (F3).
    let split = rejoin_hyphens(&[w("remem-", 0, true), w("ber", 1, false)]);
    assert_eq!(split.len(), 2, "a half-marked hyphenation must not join");
}

#[test]
fn f3_does_not_join_across_a_gap_or_within_a_line() {
    assert_eq!(rejoin_hyphens(&[w("remem-", 0, true), w("ber", 2, true)]).len(), 2,
        "lines must be adjacent");
    assert_eq!(rejoin_hyphens(&[w("remem-", 0, true), w("ber", 0, true)]).len(), 2,
        "a hyphen inside one line is not a line break");
    assert_eq!(rejoin_hyphens(&[w("-", 0, true), w("ber", 1, true)]).len(), 2,
        "a bare hyphen is not a word stem");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5_000))]

    /// I-H4 — a join is exact: the pieces put back together, minus the
    /// hyphen, are the joined text. Nothing is invented and nothing is lost.
    #[test]
    fn ih4_rejoin_is_exact(
        stems in prop::collection::vec("[a-z]{1,8}", 1..6),
        tails in prop::collection::vec("[a-z]{1,8}", 1..6),
        flags in prop::collection::vec(any::<bool>(), 1..12),
    ) {
        let mut words = Vec::new();
        let mut line = 0usize;
        for (i, (s, t)) in stems.iter().zip(tails.iter()).enumerate() {
            let hl_a = *flags.get(i * 2).unwrap_or(&true);
            let hl_b = *flags.get(i * 2 + 1).unwrap_or(&true);
            words.push(w(&format!("{s}-"), line, hl_a));
            words.push(w(t, line + 1, hl_b));
            line += 2;
        }
        for j in rejoin_hyphens(&words) {
            if let Joined::Hyphen { text, left, right, .. } = j {
                let rebuilt = format!("{}{}", left.trim_end_matches('-'), right);
                prop_assert_eq!(rebuilt, text);
            }
        }
    }

    /// Every input word survives: a join consumes exactly two, everything
    /// else passes through. Nothing is silently dropped.
    #[test]
    fn f3_conserves_words(flags in prop::collection::vec(any::<bool>(), 1..20)) {
        let words: Vec<Word> = flags.iter().enumerate()
            .map(|(i, f)| w(if i % 2 == 0 { "remem-" } else { "ber" }, i, *f))
            .collect();
        let out = rejoin_hyphens(&words);
        let accounted: usize = out.iter().map(|j| match j {
            Joined::Single(_) => 1,
            Joined::Hyphen { .. } => 2,
        }).sum();
        prop_assert_eq!(accounted, words.len());
    }
}
