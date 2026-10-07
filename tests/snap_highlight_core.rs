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

/// delta_s is boxed in from BOTH sides by measured pages, and the window is
/// narrow. Pinned so a later tuning pass cannot widen one margin without
/// seeing what it costs on the other.
///
///   fixture row 20, a pale callout box behind dark text:  S 0.196
///   fixture row 11, the palest real highlight (Kindle):   S 0.27
///
/// Below 0.196 an infographic imports its callout boxes; above 0.27 Kindle
/// highlights stop being found. 0.25 sits between, 0.054 above the floor and
/// 0.02 under the ceiling.
#[test]
fn delta_s_sits_in_the_window_its_two_fixtures_leave() {
    let c = cfg();
    let p = page(0.0, Source::Screenshot);
    // A pale callout fill must NOT read as a mark.
    assert_eq!(score_page(&c, &p, &[boxed(1.0, 0.196, Some(55))]).count(), 0,
        "a pale callout box imported -- delta_s has fallen below row 20's floor");
    // The palest real highlight must still be found.
    assert_eq!(score_page(&c, &p, &[boxed(1.0, 0.27, Some(55))]).count(), 1,
        "the palest Kindle highlight was missed -- delta_s has risen above row 11's ceiling");
}

/// KNOWN GAP, pinned so it cannot be forgotten or silently "fixed".
///
/// Fixture row 16 is a Kindle page whose blue headings drop only 0.29 below
/// paper. That is UNDER max_v_drop, so the fill rule passes them, and they
/// cover 4.3% of the page in large type, so coverage will not save it the
/// way it saves thin ink. This page imports its section titles today.
///
/// No threshold fixes it. Row 11's heading measured 0.53 and row 16's is
/// 0.29, so coloured headings span 0.29-0.53 while real marks span
/// 0.00-0.35 (the dimmest being a cyan marker at 0.34). They OVERLAP.
/// Lowering max_v_drop to catch 0.29 would reject that marker.
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
    let heading = boxed_v(1.0, 0.8, Some(210), Some(0.29));
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
