//! CC-WORD-PICTURE v5 core — calligrams: the words you spell become the picture.
//!
//! REVIEW-GATED. A picture is a manifest of WORD-PATHS (D1): outline guides
//! visible from word 1; each correctly spelled word is typeset along the next
//! path (flow/textPath, vertical stacks, per-unit CJK placement — and, per
//! Eric's D4 ruling, chord-angled straight words for complex-shaping scripts
//! on curves). Words come seeded from the language's OWN tier pool filtered
//! to each path's typing-unit budget (D2) — no semantic mapping (v2 banned),
//! no fill-mask reveal (v3 banned). The words ARE the artwork.
//!
//! Grow-only (D5): a placed word is never removed mid-run; the ONLY
//! persistence is the `spell_wordpic` state (I8 mode-local writes).

use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

/// D6: per-word flow-in, ms (Reduce Motion: instant).
pub const FLOW_MS: u32 = 1000;
/// D8: absolute legibility floor for rendered words, px at reference viewport.
pub const MIN_FONT: f32 = 12.0;
/// D14: deprioritize words placed in the player's last N completed pictures.
pub const RECENCY_PICS: usize = 3;
/// D7: "Surprise me" no-repeat window (last N starts, capped at library-1).
pub const NO_REPEAT: usize = 5;
/// D3 path-count bands.
/// CC-PICKER v3: the script folders, fixed order; display names are
/// i18n `wp.folder.<id>` (15-locale parity enforced by i18n-check).
pub const FOLDERS: [(&str, &str); 8] = [
    ("latin", "ABC"), ("cyrillic", "\u{410}\u{411}"), ("arabic", "\u{627}\u{628}"),
    ("devanagari", "\u{905}"), ("korean", "\u{d55c}"), ("hiragana", "\u{3042}"),
    ("hanzi", "\u{5b57}"), ("numbers", "123"),
];

pub const BANDS: [(&str, u32, u32); 4] =
    [("easy", 1, 8), ("medium", 1, 20), ("hard", 1, 45), ("expert", 1, 200)];

/// CC-PICTURE-COLOR F2 — resolve a path's fill: the palette entry its
/// `paletteRef` names, when the subject is color-live. Returns None for
/// neutral (the shipped monochrome). Pure lookup, never generated (F1);
/// the solver never calls this (proven by `color_never_touches_layout`).
pub fn path_color(p: &Picture, path_idx: usize) -> Option<&str> {
    if !color_enabled(p) {
        return None;
    }
    let q = p.paths.get(path_idx)?;
    if q.palette_ref.is_empty() {
        return None;
    }
    p.palette
        .iter()
        .find(|c| c.id == q.palette_ref)
        .map(|c| c.hex.as_str())
}

/// CC-PICTURE-COLOR F6 — category exclusion AS DATA: the per-subject
/// override wins; otherwise Numbers & Alphabets (the learn shelf) are
/// neutral and everything else is color-enabled. No `if category ==`
/// lives at any call site — this is the one resolver.
pub fn color_enabled(p: &Picture) -> bool {
    if let Some(v) = p.color_enabled {
        return v;
    }
    !p.categories.iter().any(|c| c == "learn")
}

/// CC-PICTURE-COLOR F4 — the canvas a picture renders on, and the neutral
/// tokens that go with it.
///
/// This was a single constant until Done #6 (2026-08-13). Sampling the Mona
/// Lisa against the dark ground showed SEVEN of eight regions failing the
/// 4.5:1 floor — the face included — while flooring them to pass turned
/// Leonardo's near-black shadows lavender. Neither answer was the painting.
/// The variable was the GROUND: the same faithful colours fail only two of
/// eight on a light one, and both of those clear the large-glyph threshold.
///
/// So a dark painting gets a light canvas, and the ink flips with it. Every
/// token here moves together — a ground swap that changed only the rect would
/// leave near-white words on near-white paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ground {
    /// The canvas rect, and what the contrast lint measures against.
    pub bg: &'static str,
    /// Landed words with no palette color of their own.
    pub ink: &'static str,
    /// Pinned strokes and features — the picture's own line work.
    pub stroke: &'static str,
    /// The guide outline, deliberately faint.
    pub outline: &'static str,
}

/// The default: near-white on near-black. 15.57:1.
pub const DARK: Ground = Ground {
    bg: "#0e1420",
    ink: "#e8ecf5",
    stroke: "rgba(232,236,245,.92)",
    outline: "rgba(255,255,255,.22)",
};

/// The gallery ground for paintings. Warm off-white rather than plain white
/// because a painting reads as a painting on paper, not on a screen; #1a1712
/// on it measures 15.58:1, near-exactly mirroring the dark ground's 15.57:1.
/// `#e8e0d0` was the other candidate and is REJECTED — Mona's dress lands at
/// 2.70:1 there and fails even the large-glyph threshold.
pub const LIGHT: Ground = Ground {
    bg: "#f4efe4",
    ink: "#1a1712",
    stroke: "rgba(26,23,18,.92)",
    outline: "rgba(0,0,0,.22)",
};

/// Which ground a picture renders on. Masterpieces are paintings and get the
/// gallery ground; everything else keeps the app's dark canvas.
pub fn ground_for(p: &Picture) -> Ground {
    if p.categories.iter().any(|c| c == "masters") {
        LIGHT
    } else {
        DARK
    }
}

/// The ground for a subject id, for callers holding only a Plan.
pub fn ground_of(subject: &str) -> Ground {
    picture(subject).map(ground_for).unwrap_or(DARK)
}

/// Kept as the DARK ground's canvas so existing callers keep their meaning.
pub const COLOR_CANVAS: &str = DARK.bg;

/// WCAG relative luminance of a #RRGGBB hex.
pub fn hex_luminance(hex: &str) -> f32 {
    let c = |i: usize| {
        let v = u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * c(1) + 0.7152 * c(3) + 0.0722 * c(5)
}

/// WCAG contrast ratio between two hex colors.
pub fn hex_contrast(a: &str, b: &str) -> f32 {
    let (la, lb) = (hex_luminance(a), hex_luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// CC-PICTURE-COLOR F4 — the floor a path's color must clear against
/// the canvas: band 1 renders large glyphs (22-40), everything else is
/// small text. Deterministic; no runtime measurement.
pub fn contrast_floor_for_band(band: u8) -> f32 {
    if band <= 1 { 3.0 } else { 4.5 }
}

/// CC-PICTURE-COLOR F1 — one curated color: authored at content time,
/// looked up at render time, NEVER generated. 8-12 per subject max.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaletteColor {
    pub id: String,
    pub hex: String,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WordPath {
    pub mode: String, // "flow" | "stack"
    #[serde(default)]
    pub d: Option<String>,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub size: f32,
    pub order: u32,
    /// Typing-unit budget [min, max] the feed filters to (D2).
    pub budget: (u32, u32),
    pub band: u8,
    pub arch: String,
    /// v7 F2 — the nameable feature this stroke draws (lint-enforced).
    #[serde(default)]
    pub feature: String,
    /// Long flow paths host `segs` one-word segments instead of stretching.
    #[serde(default)]
    pub segs: Option<u32>,
    /// CC-PICTURE-COLOR F2 — which palette entry this path's LANDED
    /// words fill with. Empty = neutral. Resolution is a lint, not a
    /// runtime branch; the solver never reads this.
    #[serde(default, rename = "paletteRef")]
    pub palette_ref: String,
    /// Stacks: intended letter count — v6 fixes the COLUMN height at
    /// column × size and solves glyph size per word (fill-the-column).
    #[serde(default)]
    pub column: u32,
    /// POLISH F4: the focal stroke (the smile). Set only by the tonal
    /// export; carries max export order (CI-enforced) and is the
    /// streak-feed target (D2).
    #[serde(default)]
    pub focal: bool,
}

impl WordPath {
    pub fn slots(&self) -> u32 {
        self.segs.unwrap_or(1).max(1)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Picture {
    pub id: String,
    pub tier: String,
    pub subject: String,
    pub icon: String,
    /// CC-PICKER v3 hub (Eric: "NO more sliding menu options" +
    /// "greenlight 1 and 2 ship 134" + "3 and 4"): letter tiles live in
    /// script FOLDERS — membership is registry data.
    #[serde(default)]
    pub folder: String,
    /// CC-PICKER v2 (Eric, 2026-08-05: "call the pictures by their
    /// name"): the display name on every tile, curated en v1; the
    /// `wp.name.<id>` i18n key overrides per locale when present.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kid: bool,
    #[serde(default)]
    pub wash: bool,
    #[serde(default)]
    pub pack: String,
    /// CC-PICKER-SEARCH: category membership IS registry data — the picker
    /// renders these and nothing else (Feature 7). >=1 required (CI).
    #[serde(default)]
    pub categories: Vec<String>,
    /// The v5 same-tier-uniqueness home (must be a member of categories).
    #[serde(default, rename = "canonicalCategory")]
    pub canonical_category: String,
    /// Match-only strings (D6): never displayed, so never display-audited —
    /// but profanity-checked at CI. Curated at pack-creation time.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// CC-MASTERPIECE-TONAL: the pipeline fork is explicit data, never a
    /// heuristic. LINE = v7.5 ink extraction; TONAL = the posterize/flow
    /// pipeline. Missing = lint failure (the test below).
    #[serde(default, rename = "extractionClass")]
    pub extraction_class: String,
    /// TONAL subjects declare their checkable features (portraits: the
    /// twelve Eric named). Export blocks when one is missing.
    #[serde(default, rename = "requiredFeatures")]
    pub required_features: Vec<String>,
    /// CC-PICTURE-COLOR F1 — the subject's curated palette (D1 flat
    /// fill v1; index-0 named-palette reserved for later). Data only:
    /// the layout solver is proven blind to it by
    /// `color_never_touches_layout`.
    #[serde(default)]
    pub palette: Vec<PaletteColor>,
    /// CC-PICTURE-COLOR F6 — category exclusion as data: None inherits
    /// the shelf default (numbers/alphabets false, elsewhere true).
    #[serde(default, rename = "colorEnabled")]
    pub color_enabled: Option<bool>,
    /// v7.5 Option 2 (Eric): always-visible guide art — the traced ink he
    /// graded. Renders as outline strokes; never hosts words, never
    /// collides, never counts as a word path.
    #[serde(default)]
    pub guide: Vec<String>,
    pub paths: Vec<WordPath>,
    #[serde(default)]
    pub provenance: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Manifest {
    #[serde(rename = "flowMs")]
    pub flow_ms: u32,
    #[serde(rename = "minFont")]
    pub min_font: f32,
    #[serde(rename = "recencyBias")]
    pub recency_bias: u32,
    pub free: Vec<String>,
    pub pictures: Vec<Picture>,
}

pub fn manifest() -> &'static Manifest {
    static M: OnceLock<Manifest> = OnceLock::new();
    M.get_or_init(|| {
        serde_json::from_str(include_str!("../config/wordpic/pictures.json"))
            .expect("pictures.json parses")
    })
}

pub fn picture(id: &str) -> Option<&'static Picture> {
    manifest().pictures.iter().find(|p| p.id == id)
}

/// Total word slots (paths expanded by their segments) — the campaign length.
pub fn slots(p: &Picture) -> u32 {
    p.paths.iter().map(|q| q.slots()).sum()
}

/// The slot list in fill order: (path index, budget) per hosted word.
pub fn slot_budgets(p: &Picture) -> Vec<(usize, (u32, u32))> {
    let mut out = Vec::new();
    for (i, q) in p.paths.iter().enumerate() {
        for _ in 0..q.slots() {
            out.push((i, q.budget));
        }
    }
    out
}

// ---- deterministic PRNG (splitmix64 — the crate convention) ----

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Typing-unit length (registry units, not bytes): jamo for ko, NFC chars
/// elsewhere — the same unit the rest of the game types in.
pub fn unit_len(lang: &str, word: &str) -> u32 {
    crate::practice::units(lang, word).len() as u32
}

/// The typed form of a pool entry (zh stores "pinyin|hanzi").
fn typed(w: &str) -> String {
    w.split('|').next().unwrap_or(w).nfc().collect()
}

/// D2 + D14 — the word feed: one word per slot, seeded, budget-filtered, no
/// repeats within the picture, recent-picture words deprioritized (they stay
/// LEGAL — pools are finite — but only fill a slot when nothing fresh fits).
/// Deterministic for (picture, lang, seed, recent) — `recent` is part of the
/// persisted state so resume replays identically (I3).
pub fn word_feed(p: &Picture, lang: &str, seed: u64, recent: &[String]) -> Vec<String> {
    let pool = crate::words::tier_for(lang, &p.tier);
    let mut st = seed ^ 0x57505F5635; // "WP_V5" feed-domain salt
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (_, (lo, hi)) in slot_budgets(p) {
        let fits = |w: &&str| {
            let t = typed(w);
            let n = unit_len(lang, &t);
            n >= lo && n <= hi && !used.contains(&t)
        };
        let fresh: Vec<&str> = pool
            .iter()
            .filter(|w| fits(w) && !recent.contains(&typed(w)))
            .copied()
            .collect();
        let candidates: Vec<&str> = if fresh.is_empty() {
            pool.iter().filter(|w| fits(w)).copied().collect()
        } else {
            fresh
        };
        if candidates.is_empty() {
            // I1 shortfall — the checker hides this (picture, lang) pair; the
            // runtime guard just stops short (screen never opens such pairs).
            break;
        }
        let k = (splitmix64(&mut st) % candidates.len() as u64) as usize;
        let w = typed(candidates[k]);
        used.push(w.clone());
        out.push(w);
    }
    out
}

/// I1 — every slot's budget matches ≥3 candidates in the language's pool.
pub fn playable(p: &Picture, lang: &str) -> bool {
    let pool = crate::words::tier_for(lang, &p.tier);
    slot_budgets(p).iter().all(|(_, (lo, hi))| {
        pool.iter()
            .filter(|w| {
                let n = unit_len(lang, &typed(w));
                n >= *lo && n <= *hi
            })
            .take(3)
            .count()
            >= 3
    })
}

/// D16 kid eligibility: tier ≤ medium, manifest flag, and the language's
/// deterministic feed passes the kid filter end-to-end.
pub fn kid_ok(p: &Picture, lang: &str, seed: u64) -> bool {
    if !p.kid || !(p.tier == "easy" || p.tier == "medium") {
        return false;
    }
    word_feed(p, lang, seed, &[]).iter().all(|w| crate::kid_filter::kid_allowed(lang, w))
}

// ---- persistence (I8: the ONE mode-local record) ----

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Run {
    pub pic: String,
    pub lang: String,
    pub seed: u64,
    /// Words placed so far, in path order (grow-only; gallery replay data).
    #[serde(default)]
    pub words: Vec<String>,
    pub done: bool,
    /// Latest "Spell it again" replay (canonical first completion kept above).
    #[serde(default)]
    pub replay: Vec<String>,
    #[serde(default)]
    pub replay_seed: u64,
    /// Monotonic play counter at last touch (picker LRU without wall clocks).
    pub touched: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub runs: Vec<Run>,
    pub plays: u64,
    #[serde(default)]
    pub bag: Vec<String>,
    #[serde(default)]
    pub recent: Vec<String>,
    /// D14: words placed in the last RECENCY_PICS completed pictures, per
    /// language (flattened, most recent last).
    #[serde(default)]
    pub recent_words: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub rng: u64,
    #[serde(default)]
    pub how_shown: bool,
}

const KEY: &str = "spell_wordpic";

pub fn load() -> State {
    crate::storage::get_json(KEY).unwrap_or_default()
}

pub fn save(s: &State) {
    crate::storage::set_json(KEY, s);
}

impl State {
    pub fn run(&self, pic: &str, lang: &str) -> Option<&Run> {
        self.runs.iter().find(|r| r.pic == pic && r.lang == lang)
    }

    pub fn open(&mut self, pic: &str, lang: &str) -> Run {
        self.plays += 1;
        let plays = self.plays;
        if self.rng == 0 {
            self.rng = 0x5EED_BA5E;
        }
        let seed = splitmix64(&mut self.rng);
        if let Some(r) = self.runs.iter_mut().find(|r| r.pic == pic && r.lang == lang) {
            r.touched = plays;
            return r.clone();
        }
        let run = Run {
            pic: pic.into(),
            lang: lang.into(),
            seed,
            words: Vec::new(),
            done: false,
            replay: Vec::new(),
            replay_seed: 0,
            touched: plays,
        };
        self.runs.push(run.clone());
        run
    }

    /// Grow-only placement (D5): records the word, marks done at the last
    /// slot, and feeds D14's recency list on completion.
    pub fn place(&mut self, pic: &str, lang: &str, word: &str, total: u32) -> u32 {
        self.plays += 1;
        let plays = self.plays;
        let mut completed: Option<Vec<String>> = None;
        let mut placed = 0;
        if let Some(r) = self.runs.iter_mut().find(|r| r.pic == pic && r.lang == lang) {
            if (r.words.len() as u32) < total {
                r.words.push(word.to_string());
            }
            placed = r.words.len() as u32;
            if placed >= total && !r.done {
                r.done = true;
                completed = Some(r.words.clone());
            }
            r.touched = plays;
        }
        if let Some(words) = completed {
            let per_pic = words.len();
            let list = self.recent_words.entry(lang.to_string()).or_default();
            list.extend(words);
            let cap = per_pic.max(8) * RECENCY_PICS;
            if list.len() > cap {
                let drop = list.len() - cap;
                list.drain(0..drop);
            }
        }
        placed
    }

    /// D15c "Spell it again": fresh seed over the same paths; the canonical
    /// first completion stays, the latest replay is stored alongside.
    pub fn start_replay(&mut self, pic: &str, lang: &str) -> u64 {
        self.plays += 1;
        if self.rng == 0 {
            self.rng = 0x5EED_BA5E;
        }
        let seed = splitmix64(&mut self.rng);
        let plays = self.plays;
        if let Some(r) = self.runs.iter_mut().find(|r| r.pic == pic && r.lang == lang) {
            r.replay_seed = seed;
            r.replay = Vec::new();
            r.touched = plays;
        }
        seed
    }

    pub fn restart(&mut self, pic: &str, lang: &str) {
        self.runs.retain(|r| !(r.pic == pic && r.lang == lang));
    }
}

/// D7 picker ordering: in-progress first (most-recent), then unstarted
/// (easiest tier first), then completed (least-recently-touched first).
// ---------------- CC-PICKER-SEARCH pure core ----------------
// The picker is a pure function of the registry: these functions take
// registry + run state and return decisions. The synthetic-pack CI test
// below exercises them with a fabricated pack — if a new pack ever needs
// more than registry data, the architecture has failed (the spec's words).

/// Search match (Feature 1): case- and diacritic-insensitive contains
/// over the corpus, via the EXISTING normalization layer
/// (norm::fold_lenient — no new normalizer). No fuzzy match, no stemming.
pub fn search_matches(corpus: &[String], query: &str) -> bool {
    let q = crate::norm::fold_lenient(query);
    if q.is_empty() {
        return false;
    }
    corpus.iter().any(|h| crate::norm::fold_lenient(h).contains(&q))
}

/// Deterministic sort key (Feature 5): In Progress (recent first), then
/// Not Started (tier ascending, then registry order), then Completed
/// (most recent first — `touched` is the app's monotonic recency counter;
/// completion DATES were never stored, and the counter is order-isomorphic
/// to them; recorded in the ledger).
pub fn picker_sort_key(p: &Picture, run: Option<&Run>, registry_idx: usize) -> (u8, i64, i64) {
    match run {
        Some(r) if r.done => (2, -(r.touched as i64), registry_idx as i64),
        Some(r) if !r.words.is_empty() => (0, -(r.touched as i64), registry_idx as i64),
        _ => (1, tier_rank(&p.tier) as i64, registry_idx as i64),
    }
}

pub fn tier_rank(tier: &str) -> u8 {
    match tier {
        "easy" => 0,
        "medium" => 1,
        "hard" => 2,
        "expert" => 3,
        _ => 4,
    }
}

/// The shelf list, registry-driven (Feature 7): (id, nameKey) in shipped
/// order. Zero hardcoded categories anywhere in picker code.
pub fn category_list() -> Vec<(String, String)> {
    #[derive(serde::Deserialize)]
    struct Cat {
        id: String,
        #[serde(rename = "nameKey")]
        name_key: String,
    }
    #[derive(serde::Deserialize)]
    struct Reg {
        #[serde(rename = "categoryList", default)]
        category_list: Vec<Cat>,
    }
    let reg: Reg = serde_json::from_str(include_str!("../config/wordpic/pictures.json"))
        .unwrap_or(Reg { category_list: Vec::new() });
    reg.category_list.into_iter().map(|c| (c.id, c.name_key)).collect()
}

#[cfg(test)]
mod picker_search_ci {
    use super::*;

    fn registry() -> Vec<Picture> {
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../config/wordpic/pictures.json")).unwrap();
        serde_json::from_value(d["pictures"].clone()).unwrap()
    }

    fn category_ids() -> Vec<String> {
        category_list().into_iter().map(|(id, _)| id).collect()
    }

    /// Feature 7 lint: categories nonempty, canonical is a member, every
    /// referenced category exists. Old-format (empty categories) entries
    /// are a build failure — the one-shot migration has landed.
    #[test]
    fn every_subject_is_categorized_and_canonical() {
        let cats = category_ids();
        assert!(!cats.is_empty(), "registry categoryList missing");
        for p in registry() {
            assert!(!p.categories.is_empty(), "{}: categories empty (old format?)", p.id);
            assert!(
                p.categories.contains(&p.canonical_category),
                "{}: canonicalCategory not a member",
                p.id
            );
            for c in &p.categories {
                assert!(cats.contains(c), "{}: unknown category {c}", p.id);
            }
        }
    }

    /// CC-PICTURE-COLOR Done #3 — contrast lint, both thresholds. Every
    /// path that names a palette color must clear its band's floor
    /// against the shipped canvas. Runs over the whole registry (vacuous
    /// until palettes ship) and over an inline fixture that proves BOTH
    /// thresholds bite.
    #[test]
    fn palette_contrast_floor() {
        let check = |p: &Picture| -> Vec<String> {
            let mut bad = Vec::new();
            for q in &p.paths {
                if q.palette_ref.is_empty() {
                    continue;
                }
                let Some(c) = p.palette.iter().find(|c| c.id == q.palette_ref) else {
                    continue; // Done #2's resolution lint owns missing refs
                };
                let floor = contrast_floor_for_band(q.band);
                // Against the picture's OWN ground. Measuring every palette
                // against the dark canvas would pass masterpiece colours that
                // are illegible on the light one, and fail ones that are fine
                // — the lint has to follow the ground, not assume it.
                let got = hex_contrast(&c.hex, ground_for(p).bg);
                if got < floor {
                    bad.push(format!(
                        "{}: path {} ({}) color {} = {:.2}:1 < {:.1}:1 (band {})",
                        p.id, q.order, q.palette_ref, c.hex, got, floor, q.band
                    ));
                }
            }
            bad
        };
        for p in registry() {
            let bad = check(&p);
            assert!(bad.is_empty(), "contrast floor violations:\n{}", bad.join("\n"));
        }
        // Fixture: #8A6A50 is 3.9:1 on the canvas — LEGAL on a band-1
        // (large) path, ILLEGAL on a band-3 (small) path. Both
        // thresholds must bite or the lint is decoration.
        let mut fx = synthetic("colorfx", &["things"]);
        let donor = registry().into_iter().find(|p| p.id == "sun").unwrap();
        fx.paths = donor.paths[..2].to_vec();
        fx.palette = vec![PaletteColor {
            id: "mid".into(), hex: "#8A6A50".into(), role: "test".into(),
        }];
        for q in &mut fx.paths {
            q.palette_ref = "mid".into();
        }
        fx.paths[0].band = 1;
        fx.paths[1].band = 3;
        let bad = check(&fx);
        assert_eq!(bad.len(), 1, "exactly the small-glyph path must trip: {bad:?}");
        assert!(bad[0].contains("band 3"), "the band-3 path is the violation");
    }

    /// CC-PICTURE-COLOR Done #2 — resolution lint. Unresolvable color
    /// is a LINT failure, never a runtime fallback (no default-gray
    /// escape hatch). Registry-wide, plus the deliberately-unresolvable
    /// fixture the spec demands — the lint must be seen to bite.
    #[test]
    fn palette_resolution() {
        let check = |p: &Picture| -> Vec<String> {
            let mut bad = Vec::new();
            let ids: Vec<&str> = p.palette.iter().map(|c| c.id.as_str()).collect();
            for (i, c) in p.palette.iter().enumerate() {
                if ids[..i].contains(&c.id.as_str()) {
                    bad.push(format!("{}: duplicate palette id '{}'", p.id, c.id));
                }
                let hex_ok = c.hex.len() == 7
                    && c.hex.starts_with('#')
                    && c.hex[1..].chars().all(|ch| ch.is_ascii_hexdigit());
                if !hex_ok {
                    bad.push(format!("{}: palette '{}' bad hex {:?}", p.id, c.id, c.hex));
                }
            }
            if p.palette.len() > 12 {
                bad.push(format!("{}: {} palette entries (max 12)", p.id, p.palette.len()));
            }
            let live = p.color_enabled == Some(true) || !p.palette.is_empty();
            for q in &p.paths {
                if q.palette_ref.is_empty() {
                    if live {
                        bad.push(format!(
                            "{}: path {} has NO paletteRef on a color-live subject                              (no default-gray escape hatch)",
                            p.id, q.order
                        ));
                    }
                    continue;
                }
                if !ids.contains(&q.palette_ref.as_str()) {
                    bad.push(format!(
                        "{}: path {} names unresolvable color '{}'",
                        p.id, q.order, q.palette_ref
                    ));
                }
            }
            bad
        };
        for p in registry() {
            let bad = check(&p);
            assert!(bad.is_empty(), "resolution violations:\n{}", bad.join("\n"));
        }
        // The deliberately-unresolvable entry (Done #2's own fixture):
        // a ref to a ghost id AND a bare path on a live subject — the
        // lint must flag exactly these two, by name.
        let mut fx = synthetic("colorfx", &["things"]);
        let donor = registry().into_iter().find(|p| p.id == "sun").unwrap();
        fx.paths = donor.paths[..2].to_vec();
        fx.palette = vec![PaletteColor {
            id: "real".into(), hex: "#C8955C".into(), role: "test".into(),
        }];
        fx.paths[0].palette_ref = "ghost".into();
        fx.paths[1].palette_ref = String::new();
        let bad = check(&fx);
        assert!(
            bad.iter().any(|b| b.contains("unresolvable color 'ghost'")),
            "ghost ref must be flagged: {bad:?}"
        );
        assert!(
            bad.iter().any(|b| b.contains("no default-gray") || b.contains("NO paletteRef")),
            "bare path on live subject must be flagged: {bad:?}"
        );
    }

    /// CC-PICKER v3: folder ids are a closed set, and every Learn-shelf
    /// picture is foldered — the 221-tile avalanche can never return.
    #[test]
    fn folders_are_closed_and_learn_is_foldered() {
        let ids: Vec<&str> = FOLDERS.iter().map(|(id, _)| *id).collect();
        for p in registry() {
            if !p.folder.is_empty() {
                assert!(ids.contains(&p.folder.as_str()), "{}: unknown folder {}", p.id, p.folder);
            }
            if p.categories.iter().any(|c| c == "learn") {
                assert!(!p.folder.is_empty(), "{}: learn picture without a folder", p.id);
            }
        }
    }

    /// CC-MASTERPIECE-TONAL registry lint: every subject declares its
    /// extraction class; every TONAL subject carries requiredFeatures.
    #[test]
    fn extraction_class_is_explicit_everywhere() {
        for p in registry() {
            assert!(
                p.extraction_class == "LINE" || p.extraction_class == "TONAL",
                "{}: extractionClass missing or invalid ({:?})", p.id, p.extraction_class
            );
            if p.extraction_class == "TONAL" {
                assert!(!p.required_features.is_empty(), "{}: TONAL without requiredFeatures", p.id);
            }
        }
    }

    /// Feature 3 CI: every Masterpieces subject carries title + artist in
    /// EVERY shipped locale. Emoji-only masterpiece tiles are impossible.
    /// The char budget IS the +40% pseudo-locale sweep in deterministic
    /// form: no ellipsis path exists in the renderer, so oversize fails
    /// HERE, not truncates there.
    #[test]
    fn masterpieces_carry_captions_in_all_locales() {
        const LOCALES: [(&str, &str); 15] = [
            ("en", include_str!("i18n/locales/en.json")),
            ("es", include_str!("i18n/locales/es.json")),
            ("fr", include_str!("i18n/locales/fr.json")),
            ("de", include_str!("i18n/locales/de.json")),
            ("pt", include_str!("i18n/locales/pt.json")),
            ("pl", include_str!("i18n/locales/pl.json")),
            ("ru", include_str!("i18n/locales/ru.json")),
            ("ar", include_str!("i18n/locales/ar.json")),
            ("hi", include_str!("i18n/locales/hi.json")),
            ("zh", include_str!("i18n/locales/zh.json")),
            ("ja", include_str!("i18n/locales/ja.json")),
            ("ko", include_str!("i18n/locales/ko.json")),
            ("sw", include_str!("i18n/locales/sw.json")),
            ("vi", include_str!("i18n/locales/vi.json")),
            ("fil", include_str!("i18n/locales/fil.json")),
        ];
        for p in registry().iter().filter(|p| p.categories.iter().any(|c| c == "masters")) {
            for (code, raw) in LOCALES {
                let d: serde_json::Value = serde_json::from_str(raw).unwrap();
                for suffix in ["title", "artist"] {
                    let key = format!("mp.{}.{suffix}", p.id);
                    let v = d[&key].as_str().unwrap_or("");
                    assert!(!v.is_empty(), "{}: missing {key} in {code}", p.id);
                    assert!(
                        (v.chars().count() as f64 * 1.4) <= 32.0,
                        "{key} in {code} would overflow the 2-line cap at +40%"
                    );
                }
            }
        }
    }

    /// D6: aliases are match-only but profanity-screened at CI; alias
    /// collisions across different subjects surface as warnings (v1).
    #[test]
    fn aliases_pass_the_profanity_screen() {
        let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for p in registry() {
            for a in &p.aliases {
                assert!(
                    !crate::profanity::is_blocked(a),
                    "{}: alias {a:?} fails the profanity screen",
                    p.id
                );
                if let Some(other) = seen.insert(crate::norm::fold_lenient(a), p.id.clone()) {
                    if other != p.id {
                        eprintln!("ALIAS COLLISION (triage, v1 warning): {a:?} -> {other} and {}", p.id);
                    }
                }
            }
        }
    }

    /// Feature 1 acceptance: durer with and without the umlaut, any case,
    /// one result set; empty query matches nothing.
    #[test]
    fn diacritic_insensitive_matching() {
        let corpus = vec!["Albrecht D\u{fc}rer".to_string(), "The Rhinoceros".to_string()];
        for q in ["d\u{fc}rer", "durer", "DURER", "Durer"] {
            assert!(search_matches(&corpus, q), "{q} must match");
        }
        assert!(!search_matches(&corpus, "hokusai"));
        assert!(!search_matches(&corpus, ""));
    }

    /// Done #3's unit face: every shipped subject is reachable by typing
    /// <=3 characters of its own corpus head.
    #[test]
    fn every_subject_reachable_in_three_characters() {
        for p in registry() {
            let mut corpus: Vec<String> = vec![p.id.clone(), p.subject.clone()];
            corpus.extend(p.aliases.iter().cloned());
            let head: String = crate::norm::fold_lenient(&p.id).chars().take(3).collect();
            assert!(
                search_matches(&corpus, &head),
                "{}: not reachable via its 3-char head {head:?}",
                p.id
            );
        }
    }

    /// Feature 5: deterministic order — In Progress, Not Started (tier
    /// then registry), Completed.
    #[test]
    fn sort_orders_progress_then_fresh_then_done() {
        let p_easy = synthetic("aaa", &["synthcat"]);
        let p_hard = Picture { tier: "hard".into(), ..synthetic("bbb", &["synthcat"]) };
        let going = Run { pic: "c".into(), lang: "en".into(), seed: 1, words: vec!["x".into()], done: false, replay: vec![], replay_seed: 0, touched: 9 };
        let done = Run { pic: "d".into(), lang: "en".into(), seed: 1, words: vec!["x".into()], done: true, replay: vec![], replay_seed: 0, touched: 5 };
        let mut keys = vec![
            ("done", picker_sort_key(&p_easy, Some(&done), 0)),
            ("fresh_easy", picker_sort_key(&p_easy, None, 1)),
            ("going", picker_sort_key(&p_easy, Some(&going), 2)),
            ("fresh_hard", picker_sort_key(&p_hard, None, 3)),
        ];
        keys.sort_by_key(|(_, k)| *k);
        let order: Vec<&str> = keys.iter().map(|(n, _)| *n).collect();
        assert_eq!(order, vec!["going", "fresh_easy", "fresh_hard", "done"]);
    }

    fn synthetic(id: &str, cats: &[&str]) -> Picture {
        Picture {
            id: id.into(),
            tier: "easy".into(),
            subject: id.into(),
            name: id.into(),
            folder: String::new(),
            icon: "\u{2b50}".into(),
            kid: true,
            wash: false,
            pack: "synthpack".into(),
            categories: cats.iter().map(|c| c.to_string()).collect(),
            canonical_category: cats[0].into(),
            aliases: vec![format!("{id}-alias")],
            extraction_class: "LINE".into(),
            required_features: vec![],
            palette: vec![],
            color_enabled: None,
            guide: vec![],
            paths: vec![],
            provenance: None,
        }
    }

    /// Feature 7's synthetic-pack test: a fabricated pack (fake category,
    /// three fake subjects, one cross-listed, one masterpiece-style)
    /// renders, searches, and sorts through the SAME pure functions the
    /// picker uses — zero picker-code modification by construction: this
    /// module imports registry-level functions only.
    #[test]
    fn synthetic_pack_needs_no_picker_code() {
        let pack = vec![
            synthetic("synthone", &["synthcat"]),
            synthetic("synthtwo", &["synthcat", "animals"]),
            Picture { tier: "expert".into(), ..synthetic("synthmaster", &["masters", "synthcat"]) },
        ];
        let mut shelf: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for p in &pack {
            for c in &p.categories {
                *shelf.entry(c.as_str()).or_default() += 1;
            }
        }
        assert_eq!(shelf["synthcat"], 3);
        assert_eq!(shelf["animals"], 1, "cross-listing renders in both");
        assert_eq!(shelf["masters"], 1);
        for p in &pack {
            let corpus: Vec<String> =
                std::iter::once(p.id.clone()).chain(p.aliases.iter().cloned()).collect();
            assert!(search_matches(&corpus, &p.id[..3]));
            assert!(search_matches(&corpus, &format!("{}-ALIAS", p.id)));
        }
        let keys: Vec<_> = pack.iter().enumerate().map(|(i, p)| picker_sort_key(p, None, i)).collect();
        assert!(keys[0] < keys[1] && keys[1] < keys[2]);
    }
}

/// The registry, in registry order — CC-PICKER-SEARCH's sole render source.
pub fn pictures() -> &'static [Picture] {
    &manifest().pictures
}

pub fn picker_order(state: &State, lang: &str) -> Vec<&'static Picture> {
    let tier_rank = |t: &str| BANDS.iter().position(|(b, _, _)| *b == t).unwrap_or(9);
    let mut pics: Vec<&'static Picture> = manifest().pictures.iter().collect();
    pics.sort_by_key(|p| match state.run(&p.id, lang) {
        Some(r) if !r.done && !r.words.is_empty() => (0u8, u64::MAX - r.touched),
        Some(r) if r.done => (2, r.touched),
        _ => (1, tier_rank(&p.tier) as u64),
    });
    pics
}

/// D7 "Surprise me": seeded draw-without-replacement; no picture twice within
/// the last-N window (N = 5 capped at library-1). I7 property-tested.
pub fn surprise(state: &mut State, startable: &[String]) -> Option<String> {
    if startable.is_empty() {
        return None;
    }
    let n = NO_REPEAT.min(startable.len().saturating_sub(1));
    loop {
        state
            .bag
            .retain(|id| startable.contains(id) && !state.recent.iter().rev().take(n).any(|r| r == id));
        if let Some(id) = state.bag.pop() {
            state.recent.push(id.clone());
            if state.recent.len() > 16 {
                state.recent.remove(0);
            }
            return Some(id);
        }
        let mut fill: Vec<String> = startable
            .iter()
            .filter(|id| !state.recent.iter().rev().take(n).any(|r| &r == id))
            .cloned()
            .collect();
        if fill.is_empty() {
            fill = startable
                .iter()
                .filter(|id| state.recent.last() != Some(*id))
                .cloned()
                .collect();
        }
        if state.rng == 0 {
            state.rng = 0x5EED_BA5E;
        }
        for i in (1..fill.len()).rev() {
            let j = (splitmix64(&mut state.rng) % (i as u64 + 1)) as usize;
            fill.swap(i, j);
        }
        state.bag = fill;
        if state.bag.is_empty() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_bands_and_choreography_hold() {
        let m = manifest();
        let starter = m.pictures.iter().filter(|p| p.pack == "starter").count();
        assert_eq!(starter, 10, "starter pack is exactly 10 (D16)");
        for pk in ["starter", "animals"] {
            let n = m.pictures.iter().filter(|p| p.pack == pk).count();
            assert!(n <= 10, "{pk}: pack cap is 10 (D13), got {n}");
        }
        for p in &m.pictures {
            let n = p.paths.len() as u32;
            let (_, lo, hi) = BANDS.iter().find(|(t, _, _)| *t == p.tier).copied().unwrap();
            assert!(n >= lo && n <= hi, "{}: {} paths outside {} band", p.id, n, p.tier);
            let orders: Vec<u32> = p.paths.iter().map(|q| q.order).collect();
            let mut sorted = orders.clone();
            sorted.sort_unstable();
            assert_eq!(orders, sorted, "{}: paths in fill order", p.id);
            for q in &p.paths {
                assert!(q.budget.0 >= 2 && q.budget.1 >= q.budget.0, "{}: sane budget", p.id);
                match q.mode.as_str() {
                    "flow" => assert!(q.d.is_some(), "{}: flow path has geometry", p.id),
                    "stack" => assert!(q.size > 0.0, "{}: stack has size", p.id),
                    other => panic!("{}: unknown mode {other}", p.id),
                }
            }
            if p.tier == "expert" {
                assert!(p.provenance.is_some(), "{}: expert carries provenance (I4)", p.id);
            }
        }
        assert_eq!(m.free.len(), 3, "free trio (D16)");
        assert!(
            m.free.iter().any(|f| picture(f).map(|p| p.tier == "medium").unwrap_or(false)),
            "free trio includes one medium"
        );
        for a in &m.pictures {
            for b in &m.pictures {
                assert!(
                    a.id == b.id || !(a.tier == b.tier && a.subject == b.subject),
                    "{}/{}: same subject at same tier (D11)",
                    a.id,
                    b.id
                );
            }
        }
    }

    /// I1 — every (picture, language): every slot budget matches ≥3 words.
    #[test]
    fn playable_everywhere() {
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            for p in &manifest().pictures {
                assert!(playable(p, code), "{}/{}: a slot budget lacks 3 candidates", p.id, code);
            }
        }
    }

    /// I3 golden — deterministic feed, full length, budget-true, no repeats;
    /// D14 bias steers clear of recent words when alternatives exist.
    #[test]
    fn feed_deterministic_budgeted_no_repeats() {
        let p = picture("smiley").unwrap();
        let a = word_feed(p, "en", 42, &[]);
        assert_eq!(a, word_feed(p, "en", 42, &[]), "same seed, same feed");
        assert_eq!(a.len() as u32, slots(p), "every slot filled");
        let mut d = a.clone();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), a.len(), "no repeats within a picture");
        for (w, (_, (lo, hi))) in a.iter().zip(slot_budgets(p)) {
            let n = unit_len("en", w);
            assert!(n >= lo && n <= hi, "{w}: {n} outside [{lo},{hi}]");
        }
        let biased = word_feed(p, "en", 42, &a);
        assert!(biased.iter().all(|w| !a.contains(w)), "recency bias avoids recent words");
        let mona = picture("mona").unwrap();
        assert_eq!(word_feed(mona, "en", 7, &[]).len() as u32, slots(mona));
    }

    /// I3 — resume round-trips exact state; replay disjoint when pool allows.
    #[test]
    fn resume_and_replay_round_trip() {
        let mut s = State::default();
        let run = s.open("mona", "en");
        let mona = picture("mona").unwrap();
        let feed = word_feed(mona, "en", run.seed, &[]);
        // v7.5 Option 2: fine art rides the guide layer; the word feed is
        // the hostable-stroke set (smaller than the old dense map).
        let take = feed.len().min(12).max(4);
        for w in feed.iter().take(take) {
            s.place("mona", "en", w, slots(mona));
        }
        let back: State = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let r = back.run("mona", "en").unwrap();
        assert_eq!(r.words.len(), take);
        assert_eq!(r.seed, run.seed);
        assert_eq!(r.words, feed[..take].to_vec(), "resume continues the exact queue");
        let p = picture("smiley").unwrap();
        let first = word_feed(p, "en", 1, &[]);
        let again = word_feed(p, "en", 2, &first);
        let overlap = again.iter().filter(|w| first.contains(w)).count();
        assert_eq!(overlap, 0, "fresh-words replay disjoint when the pool permits (D15c)");
    }

    /// I2 grow-only + scoped restart; completion feeds the D14 list.
    #[test]
    fn grow_only_and_scoped_restart() {
        let mut s = State::default();
        s.open("smiley", "en");
        s.open("star", "en");
        let total = slots(picture("smiley").unwrap());
        for i in 0..99 {
            s.place("smiley", "en", &format!("w{i}"), total);
        }
        assert_eq!(s.run("smiley", "en").unwrap().words.len() as u32, total, "saturates");
        assert!(s.run("smiley", "en").unwrap().done);
        assert!(!s.recent_words.get("en").unwrap().is_empty(), "completion feeds D14");
        s.restart("smiley", "en");
        assert!(s.run("smiley", "en").is_none());
        assert!(s.run("star", "en").is_some(), "restart is scoped to one picture");
    }

    /// I7 property — 10k surprise draws, libraries 2..=12, zero window hits.
    #[test]
    fn surprise_no_repeat_window_property() {
        for lib in 2..=12usize {
            let ids: Vec<String> = (0..lib).map(|i| format!("p{i}")).collect();
            let mut s = State::default();
            let n = NO_REPEAT.min(lib - 1);
            let mut history: Vec<String> = Vec::new();
            for _ in 0..10_000 / lib {
                let got = surprise(&mut s, &ids).unwrap();
                assert!(
                    !history.iter().rev().take(n).any(|h| *h == got),
                    "lib {lib}: {got} inside the {n}-window"
                );
                history.push(got);
            }
        }
    }

    /// D16 kid gating.
    #[test]
    fn kid_gating() {
        assert!(!kid_ok(picture("mona").unwrap(), "en", 1), "expert never kid");
        assert!(!kid_ok(picture("eiffel").unwrap(), "en", 1), "hard never kid");
        assert!(kid_ok(picture("smiley").unwrap(), "en", 1), "easy smiley kid-ok");
    }
}

#[cfg(test)]
mod ground_tests {
    use super::*;

    /// Done #6 — a painting gets the gallery ground, everything else keeps the
    /// app's canvas.
    #[test]
    fn masterpieces_render_light_and_nothing_else_does() {
        let mut masters = 0;
        for p in pictures() {
            let g = ground_for(p);
            if p.categories.iter().any(|c| c == "masters") {
                masters += 1;
                assert_eq!(g, LIGHT, "{} is a master and should be on the gallery ground", p.id);
            } else {
                assert_eq!(g, DARK, "{} is not a master and should stay dark", p.id);
            }
        }
        assert!(masters >= 6, "only {masters} masters found — the split proves nothing");
        assert_eq!(ground_of("nosuchpicture"), DARK, "an unknown id must not go light");
    }

    /// Every ground's own ink has to be readable ON that ground. This is the
    /// check the first draft of the light ground would have failed: swapping
    /// the background alone leaves near-white words on near-white paper.
    #[test]
    fn each_ground_can_read_its_own_ink() {
        for (name, g) in [("DARK", DARK), ("LIGHT", LIGHT)] {
            let r = hex_contrast(g.ink, g.bg);
            assert!(r >= 4.5, "{name}: ink {} on {} is {r:.2}:1", g.ink, g.bg);
        }
        // The two are near-mirrors, which is the point — a masterpiece should
        // read as well as anything else, not merely legibly.
        let (d, l) = (hex_contrast(DARK.ink, DARK.bg), hex_contrast(LIGHT.ink, LIGHT.bg));
        assert!((d - l).abs() < 1.0, "grounds differ in legibility: {d:.2} vs {l:.2}");
    }

    /// The rejected candidate, pinned so nobody reinstates it. Mona's dress
    /// lands at 2.70:1 on #e8e0d0 and fails even the large-glyph threshold.
    #[test]
    fn the_rejected_ground_is_recorded() {
        let dress = "#a28446";
        assert!(hex_contrast(dress, "#e8e0d0") < 3.0, "gallery linen was rejected for this");
        assert!(hex_contrast(dress, LIGHT.bg) >= 3.0, "the chosen ground clears large-glyph");
    }
}
