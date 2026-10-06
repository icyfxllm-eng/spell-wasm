//! CC-SNAP-HIGHLIGHT v1 F1/F2 — which OCR'd words sit on a highlighter mark.
//!
//! Owns the scoring decision and nothing else: not OCR, not cleanup, not bank
//! matching, not the review screen. A reader marks five words on a 300-word
//! page; this says which five.
//!
//! # What is built here, and what is not
//!
//! The review record on `docs/CC-SNAP-HIGHLIGHT.md` says the 20-photo C2
//! fixture is "the single thing blocking Level 1.3", and it still is. What
//! the fixture blocks is CALIBRATION, not the arithmetic -- so the scoring is
//! here, behind thresholds that are honest about not being calibrated yet.
//! `min_paper_px` is null in the config, and while it is null this module
//! scores nothing highlighted and says why. That is not a stub: it is the
//! Intent's own fall-back ("fail toward showing everything"), reached by
//! construction rather than by remembering to.
//!
//! Nothing here reads a file at runtime, touches the network, or sees a
//! bitmap -- which is how I-H6 is satisfied by construction rather than by
//! audit.
//!
//! # The fill test, and why F2's text mask was not enough
//!
//! F2 masks text by "luminance below an Otsu threshold" and then treats what
//! survives as background. On the first real fixture page that let a blue
//! section heading through: the Otsu cut fell at L=143, mid-tone coloured
//! type is brighter than that, and the heading is strongly saturated. It
//! looked exactly like a highlight. On a technical book, every section title
//! would have imported.
//!
//! A better luminance number does not fix it, for the same reason an absolute
//! saturation threshold could not survive aged paper. The fix is relative: a
//! highlighter is a FILL and keeps most of the paper's brightness; coloured
//! type and pen ink are STROKES and are far darker than the page they sit on.
//!
//! Measured across five real pages -- Apple Books highlight 0.00 below paper,
//! Kindle highlight 0.05, a yellow highlighter photographed on cream book
//! paper 0.08, a cyan marker on a photographed form 0.35, Kindle's blue
//! heading 0.53, a red pen strikethrough 0.59, a deep gutter shadow 0.77.
//! Highlights 0.00-0.35, type and ink and shadow 0.53-0.77. `max_v_drop` is
//! 0.45, between them.
//!
//! The book-paper number is the load-bearing one. A real highlighter under
//! room light, on cream paper whose own V is 0.800, sits 0.08 below its page
//! -- the same place a screen highlight sits. The rule is not an artefact of
//! pixel-perfect screenshots.
//!
//! It also keeps pen strikethrough out without a special case, which D-H6
//! wanted anyway.
//!
//! # Why a histogram crosses the bridge
//!
//! CC-SNAP-BOXES D-B5 settled that the pixels stay in Swift and only numbers
//! cross; a 12-megapixel page as base64 is tens of megabytes per capture.
//! That leaves the question of WHICH numbers, and the obvious answer -- let
//! Swift apply the threshold and send a count -- is the wrong one. D-H2's ΔS
//! is the value the fixture exists to tune, and if Swift applied it, every
//! re-tune would need a new app build and the two copies would drift the
//! first time one shipped without the other.
//!
//! So Swift sends the DISTRIBUTION: background pixels bucketed by saturation.
//! Every threshold then lives on this side, in one file, and re-tuning D-H2
//! after the fixture is a config edit. Swift still mirrors `v_min` and
//! `sat_buckets`, because the V cut and the bucketing happen where the pixels
//! are -- the same mirror-and-check arrangement `config/snap-geometry.json`
//! already uses, and `scripts/snap-highlight-check.mjs` fails the build if
//! they drift.

use serde::Deserialize;

/// Baked in, so there is no runtime file read.
const CONFIG_JSON: &str = include_str!("../config/snap-highlight.json");

/// Which capture a page came from. D-H3 allows the two a different ΔS,
/// because an e-reader's highlight is a flat synthetic colour and a marker
/// stroke is not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Camera,
    Screenshot,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HighlightConfig {
    /// False until the C2 fixture has set the numbers below.
    pub calibrated: bool,
    /// None until the fixture exists. None means: score nothing.
    pub min_paper_px: Option<u32>,
    /// D-H1. Fraction of a box's background pixels that must read as marked.
    pub coverage: f32,
    pub delta_s_camera: f32,
    pub delta_s_screenshot: f32,
    /// How far below the page's own brightness a saturated pixel may sit and
    /// still be a highlighter mark rather than type.
    ///
    /// This replaces F2's absolute `v_min`, and the replacement is the whole
    /// of the coloured-heading fix. See the module header.
    pub max_v_drop: f32,
    pub hue_buckets: u8,
    pub sat_buckets: usize,
}

impl HighlightConfig {
    /// The one source, parsed. Panics only if the baked config is malformed,
    /// which is a build-time fact and cannot depend on a player's device.
    pub fn baked() -> Self {
        serde_json::from_str(CONFIG_JSON)
            .unwrap_or_else(|e| panic!("config/snap-highlight.json is not a valid config: {e}"))
    }

    fn delta_s(&self, source: Source) -> f32 {
        match source {
            Source::Camera => self.delta_s_camera,
            Source::Screenshot => self.delta_s_screenshot,
        }
    }
}

/// The page-level paper estimate (F1), measured in the shim.
#[derive(Clone, Copy, Debug)]
pub struct PageStats {
    /// Median saturation of non-text pixels outside every dilated word box.
    pub paper_s: f32,
    /// The same pixels' median VALUE. F1 always specified this ("paper_V
    /// similarly"); the first cut of this module dropped it, which is what
    /// left nothing to measure a coloured heading against.
    pub paper_v: f32,
    /// How many pixels that median was taken from. On a dense book page this
    /// is margins only, and on a cropped e-reader screenshot it can be close
    /// to nothing -- which is the whole reason `min_paper_px` exists.
    pub paper_sample_px: u32,
    pub source: Source,
}

/// What the shim measures for one word box.
#[derive(Clone, Debug)]
pub struct BoxStats {
    /// Background pixels left after masking text and applying the V cut.
    pub bg_px: u32,
    /// Those pixels bucketed by saturation: bucket `i` holds the pixels whose
    /// S falls in `[i/n, (i+1)/n)`, where `n == sat_buckets`.
    pub sat_hist: Vec<u32>,
    /// Dominant hue of the saturated pixels, in degrees. None when there are
    /// none.
    pub hue_deg: Option<u16>,
    /// How far below `paper_v` those saturated pixels sit, as a fraction.
    ///
    /// One number per box, and it is what tells a highlighter from a heading.
    /// The shim measures it; the THRESHOLD stays here, so the fixture can
    /// retune it without an app build -- the same reason ΔS lives on this
    /// side. `None` means the shim did not report it, and an unmeasured box
    /// is judged on saturation alone, as it was before.
    pub mark_v_drop: Option<f32>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WordScore {
    pub highlighted: bool,
    /// Set when saturation alone would have said yes and something else said
    /// no. Worth surfacing: a silent rejection is the hardest kind to debug.
    pub rejected: Option<Rejected>,
    /// Fraction of background pixels reading as marked. DEV_PREVIEW only in
    /// the UI (D-H8), but always computed -- it is what a tuning run reads.
    pub frac_sat: f32,
    /// Dominant hue quantized to `hue_buckets`, for F5's colour chips.
    pub hue_bucket: Option<u8>,
}

/// Why a page scored nothing, when it scored nothing.
/// Why a box with plenty of saturated background was not a highlight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rejected {
    /// Its saturated pixels are far darker than the page: type or ink, not a
    /// fill laid over the paper.
    TooDarkForAMark,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotScored {
    /// `min_paper_px` is still null: the C2 fixture has not been made.
    Uncalibrated,
    /// There was too little paper to estimate from, so any verdict would be
    /// a guess. Showing every word is the better failure.
    PaperSampleTooSmall,
}

#[derive(Clone, Debug)]
pub struct PageScore {
    pub words: Vec<WordScore>,
    /// `None` when the page was scored normally.
    pub not_scored: Option<NotScored>,
}

impl PageScore {
    pub fn count(&self) -> usize {
        self.words.iter().filter(|w| w.highlighted).count()
    }
}

/// Score one page. Pure: same input, same output, no clock, no randomness
/// (I-H5).
pub fn score_page(cfg: &HighlightConfig, page: &PageStats, boxes: &[BoxStats]) -> PageScore {
    let unscored = |why: NotScored| PageScore {
        words: boxes
            .iter()
            .map(|_| WordScore { highlighted: false, rejected: None, frac_sat: 0.0, hue_bucket: None })
            .collect(),
        not_scored: Some(why),
    };

    let Some(min_paper) = cfg.min_paper_px else {
        return unscored(NotScored::Uncalibrated);
    };
    if page.paper_sample_px < min_paper {
        return unscored(NotScored::PaperSampleTooSmall);
    }

    // The cut is RELATIVE to the paper, which is what makes a uniformly warm
    // or aged page score zero (I-H2): tinting the page lifts paper_s by the
    // same amount it lifts every box, and the difference is unmoved.
    let cut = page.paper_s + cfg.delta_s(page.source);
    let words = boxes
        .iter()
        .map(|b| score_box(cfg, b, cut))
        .collect();
    PageScore { words, not_scored: None }
}

fn score_box(cfg: &HighlightConfig, b: &BoxStats, cut: f32) -> WordScore {
    if b.bg_px == 0 {
        return WordScore { highlighted: false, rejected: None, frac_sat: 0.0, hue_bucket: None };
    }
    let n = cfg.sat_buckets.max(1);
    // A bucket counts when ANY of its saturations clears the cut -- that is,
    // when its upper edge does. The Intent prefers one extra word shown over
    // one marked word hidden, so the boundary rounds that way on purpose.
    let sat_px: u32 = b
        .sat_hist
        .iter()
        .take(n)
        .enumerate()
        .filter(|(i, _)| (*i as f32 + 1.0) / n as f32 > cut)
        .map(|(_, c)| *c)
        .sum();
    let frac_sat = sat_px as f32 / b.bg_px as f32;
    let enough = frac_sat >= cfg.coverage;

    // The fill test. A highlighter sits ON the paper and keeps most of its
    // brightness; coloured type and pen ink are strokes and are far darker.
    // Without this, a blue section heading is indistinguishable from a yellow
    // highlight -- both are "saturated background" once an absolute luminance
    // mask has let the heading through.
    let too_dark = b.mark_v_drop.is_some_and(|d| d > cfg.max_v_drop);
    let highlighted = enough && !too_dark;
    WordScore {
        highlighted,
        rejected: if enough && too_dark { Some(Rejected::TooDarkForAMark) } else { None },
        frac_sat,
        hue_bucket: if highlighted { b.hue_deg.map(|h| hue_bucket(h, cfg.hue_buckets)) } else { None },
    }
}

/// Hue in degrees to one of `buckets` bins. 360 wraps to 0 rather than
/// falling off the end.
pub fn hue_bucket(hue_deg: u16, buckets: u8) -> u8 {
    let buckets = buckets.max(1) as u32;
    ((hue_deg as u32 % 360) * buckets / 360) as u8
}

// ------------------------------------------------------------------ F3 rejoin

/// One OCR'd word, with the line it sat on and whether it scored highlighted.
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub line: usize,
    pub highlighted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Joined {
    Single(Word),
    /// `remem-` + `ber` -> `remember`, with both originals kept so the review
    /// screen can show the seam.
    Hyphen { text: String, left: String, right: String, line: usize },
}

/// F3. Join a highlighted word ending in `-` at a line end to a highlighted
/// word starting the next line.
///
/// Only highlighted-to-highlighted, per F3: a hyphenated word with one half
/// marked stays two entries and the reader decides. The join happens BEFORE
/// cleanup, which is why `docs/CC-SNAP-HIGHLIGHT.md`'s I-H1 ("All words is
/// byte-identical to v1") is not true on a page that joins -- a finding the
/// review record already raises and Eric has not yet ruled on. Nothing here
/// depends on which way he rules.
pub fn rejoin_hyphens(words: &[Word]) -> Vec<Joined> {
    let mut out = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let joinable = w.highlighted
            && w.text.ends_with('-')
            && w.text.chars().count() > 1
            && words
                .get(i + 1)
                .is_some_and(|n| n.highlighted && n.line == w.line + 1 && !n.text.is_empty());
        if joinable {
            let next = &words[i + 1];
            let stem = &w.text[..w.text.len() - '-'.len_utf8()];
            out.push(Joined::Hyphen {
                text: format!("{stem}{}", next.text),
                left: w.text.clone(),
                right: next.text.clone(),
                line: w.line,
            });
            i += 2;
        } else {
            out.push(Joined::Single(w.clone()));
            i += 1;
        }
    }
    out
}
