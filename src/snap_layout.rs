//! CC-SNAP-LAYOUT 2.1 — recognise a worksheet's shape, so only the spelling
//! words import.
//!
//! A two-column `word — definition` sheet should import the left column. A
//! fill-in-the-blank sentence (`The ___ sat on the mat.`) should import
//! nothing. Today both import every word on the line and the parent deletes
//! the rest.
//!
//! # Status: built out of order, inert, and against a roadmap bullet
//!
//! D1's signed order puts 2.1 after Level 3; Eric asked for it now, which
//! is his to do. But CC-SNAP-LAYOUT v1 does not exist as a file, so this is
//! built against the roadmap's four bullets rather than a signed spec, and
//! the Level 2 fixture that would calibrate it (15 pages: 5 two-column, 5
//! bold-word sentence sheets, 5 book pages) does not exist either.
//!
//! So it is inert. `LayoutConfig::baked()` has `enabled: false`, and while
//! that holds `detect` returns `PlainList` and every line comes back
//! `Whole` — which is precisely 2.1's own stated fall-back ("confidence
//! below threshold → fall back to plain list import with every line
//! present"). Turning it on is one bool, after a fixture has shown what the
//! thresholds should be.
//!
//! # The invariant that shapes the API
//!
//! 2.1: *layout detection never removes a line; it only changes which lines
//! start checked.* So nothing here returns a filtered list. `roles` returns
//! exactly one [`LineRole`] per input line, always, and the strongest thing
//! a role can say is "import nothing from this one" — which reaches the
//! player as CLEAN's existing `blank` flag, already in `FlagSet` and already
//! in `blocks_check`, leaving the line visible and unchecked rather than
//! gone.
//!
//! # What it cannot do yet
//!
//! The third bullet — a sentence with the target word bold, underlined or
//! boxed — needs the OCR engine's style hints, and nothing passes them to
//! the core. That is the same shape as CC-SNAP-HIGHLIGHT's pixel statistics
//! and CC-SNAP-BOXES' shim: a platform measurement with no bridge yet. It is
//! deliberately absent rather than guessed at from stroke weight, which the
//! core cannot see at all.

use crate::snap_clean::{geometry_ratio, glyph_width, OcrLine};

/// A column gutter, in multiples of the line's median glyph width. Lives in
/// `config/snap-geometry.json` with every other gap threshold in this
/// pipeline, so there is one source and `scripts/snap-geometry-check.mjs`
/// guards it.
fn column_gap() -> f32 {
    geometry_ratio("column_gap")
}

/// Below this many usable lines, a page has not shown enough of a pattern to
/// call it one. Not a gap, so not in the geometry config.
const MIN_LINES_FOR_COLUMNS: usize = 4;

/// The share of usable lines that must agree before a gutter is a column
/// boundary rather than one wide sentence. PROVISIONAL: the Level 2 fixture
/// sets it.
const COLUMN_AGREEMENT: f32 = 0.6;

/// How far apart two lines' gutters may sit and still be the same boundary,
/// in multiples of the page's median glyph width.
///
/// Measured in glyphs, not in a share of page width, and the difference is
/// not cosmetic. A share of the page is generous exactly where it should be
/// strict: on a 380-wide page, 12% is 45 pixels, which happily merged four
/// gutters at 80, 100, 145 and 175 into one "column" on a page of ordinary
/// sentences. A real gutter is cut by a typesetter and lands in the same
/// place every line, to within a character or two; the unit that says so is
/// the glyph, which is also the unit every other threshold in this pipeline
/// already uses. PROVISIONAL, as above.
const BOUNDARY_SPREAD_GLYPHS: f32 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutConfig {
    /// False until a fixture has calibrated the thresholds above.
    pub enabled: bool,
}

impl LayoutConfig {
    pub fn baked() -> Self {
        LayoutConfig { enabled: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageLayout {
    /// Import every line, as CC-SNAP-LIST does today.
    PlainList,
    /// Two columns with a gutter at `boundary_x`; the left one is the words.
    TwoColumn { boundary_x: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub layout: PageLayout,
    /// Share of usable lines that agreed. 0.0 when nothing was detected.
    pub confidence: f32,
    /// Why this verdict, for the review screen and for a bug report.
    pub why: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineRole {
    /// Import the line as CLEAN produced it.
    Whole,
    /// Import only the tokens ending left of this x.
    LeftOfBoundary(f32),
    /// Import nothing from this line, and say why with CLEAN's `blank` flag.
    /// The line still appears on the review screen, unchecked.
    Blank,
}

/// Underscore runs a worksheet uses for a blank. Not a general "gap in the
/// text": a wide gap is how a two-column sheet looks, and conflating the two
/// is how a definition column becomes a blank.
fn blank_run(text: &str) -> Option<(usize, usize)> {
    let b: Vec<char> = text.chars().collect();
    let is_u = |c: char| c == '_' || c == '\u{FF3F}' || c == '\u{2017}';
    let mut i = 0;
    while i < b.len() {
        if is_u(b[i]) {
            let start = i;
            while i < b.len() && is_u(b[i]) {
                i += 1;
            }
            if i - start >= 2 {
                return Some((start, i));
            }
        } else {
            i += 1;
        }
    }
    None
}

/// A fill-in-the-blank SENTENCE: a blank with real text after it.
///
/// The trailing case (`Name: ______`) is already CLEAN F3's, which trims it
/// and sets `blank`. What CLEAN cannot see is that `The ___ sat on the mat.`
/// has nothing worth importing at all, and that is only visible from the
/// text that FOLLOWS the blank.
fn is_fill_in_blank(text: &str) -> bool {
    let Some((_, end)) = blank_run(text) else { return false };
    text.chars().skip(end).any(|c| c.is_alphabetic())
}

/// The widest inter-token gap on a line, as a multiple of its glyph width,
/// with the x it sits at. `None` when the line has no usable geometry.
fn widest_gap(line: &OcrLine) -> Option<(f32, f32)> {
    let glyph = glyph_width(line)?;
    if glyph <= 0.0 || line.boxes.len() < 2 {
        return None;
    }
    let mut best: Option<(f32, f32)> = None;
    for w in line.boxes.windows(2) {
        let gap = w[1].x0 - w[0].x1;
        if gap <= 0.0 {
            continue;
        }
        let ratio = gap / glyph;
        if best.is_none_or(|(b, _)| ratio > b) {
            best = Some((ratio, (w[0].x1 + w[1].x0) / 2.0));
        }
    }
    best
}

/// Per page, from box geometry alone. Pure; no clock, no randomness.
pub fn detect(cfg: &LayoutConfig, lines: &[OcrLine]) -> Detection {
    let plain = |why| Detection { layout: PageLayout::PlainList, confidence: 0.0, why };
    if !cfg.enabled {
        return plain("layout detection is off until a fixture calibrates it");
    }

    let usable: Vec<&OcrLine> = lines.iter().filter(|l| l.boxes.len() >= 2).collect();
    if usable.len() < MIN_LINES_FOR_COLUMNS {
        return plain("too few lines with word boxes to see a pattern");
    }

    let cut = column_gap();
    let mut gutters: Vec<f32> =
        usable.iter().filter_map(|l| widest_gap(l)).filter(|(r, _)| *r >= cut).map(|(_, x)| x).collect();
    let agreement = gutters.len() as f32 / usable.len() as f32;
    if agreement < COLUMN_AGREEMENT {
        return plain("no gutter on enough lines to be a column");
    }

    // They must agree on WHERE, not just that there is one. A sentence list
    // with one wide gap per line, each in a different place, is not columns.
    gutters.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let boundary = gutters[gutters.len() / 2];
    let mut glyphs: Vec<f32> = usable.iter().filter_map(|l| glyph_width(l)).collect();
    if glyphs.is_empty() {
        return plain("no glyph width to judge the gutter against");
    }
    glyphs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let tol = BOUNDARY_SPREAD_GLYPHS * glyphs[glyphs.len() / 2];
    let agreeing = gutters.iter().filter(|x| (**x - boundary).abs() <= tol).count();
    let tight = agreeing as f32 / usable.len() as f32;
    if tight < COLUMN_AGREEMENT {
        return plain("the wide gaps are not at a consistent x, so they are sentences");
    }
    Detection {
        layout: PageLayout::TwoColumn { boundary_x: boundary },
        confidence: tight,
        why: "a gutter at a consistent x on most lines",
    }
}

/// One role per input line, always and in order.
///
/// 2.1's invariant is that detection never removes a line, so this never
/// returns fewer than it was given. `debug_assert` is not the guard here;
/// the shape of the return type is.
pub fn roles(cfg: &LayoutConfig, lines: &[OcrLine], d: &Detection) -> Vec<LineRole> {
    lines
        .iter()
        .map(|line| {
            if cfg.enabled && is_fill_in_blank(&line.text) {
                return LineRole::Blank;
            }
            match d.layout {
                PageLayout::PlainList => LineRole::Whole,
                PageLayout::TwoColumn { boundary_x } => {
                    // A line that never crosses the gutter is already only
                    // the left column; saying LeftOfBoundary would be true
                    // but would invite a caller to re-filter it for nothing.
                    if line.boxes.iter().any(|b| b.x0 >= boundary_x) {
                        LineRole::LeftOfBoundary(boundary_x)
                    } else {
                        LineRole::Whole
                    }
                }
            }
        })
        .collect()
}

/// The tokens a `LeftOfBoundary` role keeps. Separate from `roles` so the
/// decision and the cut are testable apart.
pub fn left_of(line: &OcrLine, boundary_x: f32) -> Vec<String> {
    line.boxes.iter().filter(|b| b.x1 <= boundary_x).map(|b| b.text.clone()).collect()
}
