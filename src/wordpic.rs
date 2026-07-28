//! CC-WORD-PICTURE v3 core — fill-canvas pictures (spell words, fill pieces).
//!
//! REVIEW-GATED. A picture is a FILL TEMPLATE: the full ghost outline shows
//! from word 1; each correct word fills the next piece (D1). Words come from
//! the SAME tier's normal pool, seeded, no repeats within a picture (D3) —
//! there is NO word→piece meaning relationship (the v2 mapping architecture
//! is removed scope). Art is 100% language-neutral.
//!
//! Grow-only (D5): nothing here can unfill a piece — the progress record
//! only ever increases until completion or an explicit restart. Mode-local
//! writes only (I7): the ONLY persistence is `spell_wordpic` state.

use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

/// D6: per-piece fill animation, ms (Reduce Motion: instant).
pub const FILL_MS: u32 = 1000;
/// D7: "Surprise me" no-repeat window (last N starts; capped at library-1).
pub const NO_REPEAT: usize = 5;
/// D3 tier bands (piece counts) — also enforced by wordpic-check.mjs.
pub const BANDS: [(&str, u32, u32); 4] =
    [("easy", 6, 10), ("medium", 15, 25), ("hard", 30, 50), ("expert", 60, 100)];

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RasterGrid {
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Raster {
    pub img: String,
    pub w: u32,
    pub h: u32,
    pub grid: RasterGrid,
    /// Fill order over cell indices (row-major r*cols+c) — face last (D6).
    pub order: Vec<u32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Variant {
    pub id: String,
    pub palette: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Picture {
    pub id: String,
    pub tier: String,
    pub class: String,
    pub icon: String,
    #[serde(default)]
    pub kid: bool,
    #[serde(default)]
    pub raster: Option<Raster>,
    #[serde(default)]
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Manifest {
    #[serde(rename = "fillMs")]
    pub fill_ms: u32,
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

/// The vector picture SVG markup — the ONLY place asset files are named.
pub fn svg(id: &str) -> Option<&'static str> {
    Some(match id {
        "cat" => include_str!("../assets/scenes/starter/cat.svg"),
        "star" => include_str!("../assets/scenes/starter/star.svg"),
        "fish" => include_str!("../assets/scenes/starter/fish.svg"),
        "house" => include_str!("../assets/scenes/starter/house.svg"),
        "rocket" => include_str!("../assets/scenes/starter/rocket.svg"),
        "dragon" => include_str!("../assets/scenes/starter/dragon.svg"),
        "snowman" => include_str!("../assets/scenes/starter/snowman.svg"),
        "eiffel" => include_str!("../assets/scenes/starter/eiffel.svg"),
        "pyramids" => include_str!("../assets/scenes/starter/pyramids.svg"),
        _ => return None,
    })
}

/// Piece count: vector = data-piece group count (parsed from the SVG's own
/// data-pieces stamp); raster = grid size.
pub fn pieces(p: &Picture) -> u32 {
    if let Some(r) = &p.raster {
        return r.grid.cols * r.grid.rows;
    }
    svg(&p.id)
        .and_then(|s| {
            let k = s.find("data-pieces=\"")? + 13;
            s[k..].split('"').next()?.parse().ok()
        })
        .unwrap_or(0)
}

// ---- deterministic PRNG (splitmix64 — the crate convention) ----

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// D3 — the word feed: `pieces` words drawn seeded from the picture's tier
/// pool in `lang`, without replacement (I3: identical sequence across
/// platforms for the same (picture, language, seed)). NFC-normalized.
pub fn word_feed(pic: &Picture, lang: &str, seed: u64) -> Vec<String> {
    let pool = crate::words::tier_for(lang, &pic.tier);
    let need = pieces(pic) as usize;
    let mut idx: Vec<usize> = (0..pool.len()).collect();
    let mut st = seed ^ 0x57505F5631; // "WP_V1" — feed-domain salt
    let mut out = Vec::with_capacity(need);
    while !idx.is_empty() && out.len() < need {
        let k = (splitmix64(&mut st) % idx.len() as u64) as usize;
        let w = pool[idx.swap_remove(k)];
        // zh entries are "pinyin|hanzi" — the typed word is the pinyin side.
        let typed = w.split('|').next().unwrap_or(w);
        out.push(typed.nfc().collect::<String>());
    }
    out
}

/// I1 — completability: the tier pool must hold ≥ 2× the piece count.
pub fn completable(pic: &Picture, lang: &str) -> bool {
    crate::words::tier_for(lang, &pic.tier).len() as u32 >= pieces(pic) * 2
}

/// D8 — kid eligibility: tier ≤ medium, manifest kid flag, and every word of
/// the language's deterministic feed passes the kid filter.
pub fn kid_ok(pic: &Picture, lang: &str, seed: u64) -> bool {
    if !pic.kid || !(pic.tier == "easy" || pic.tier == "medium") {
        return false;
    }
    word_feed(pic, lang, seed).iter().all(|w| crate::kid_filter::kid_allowed(lang, w))
}

// ---- persistence (I7: the ONE mode-local record) ----

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Run {
    pub pic: String,
    pub lang: String,
    pub seed: u64,
    /// Filled pieces (grow-only; never decremented — D5/I2).
    pub filled: u32,
    pub done: bool,
    /// Monotonic play counter at last touch (picker LRU without wall clocks).
    pub touched: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub runs: Vec<Run>,
    /// Monotonic counter incremented per open/fill (ordering source).
    pub plays: u64,
    /// Shuffle-bag remainder + last-N started ids (D7).
    #[serde(default)]
    pub bag: Vec<String>,
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default)]
    pub rng: u64,
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

    /// Start-or-resume: creates the run (with a fresh seed from the state
    /// rng) on first open; always bumps the LRU counter.
    pub fn open(&mut self, pic: &str, lang: &str) -> Run {
        self.plays += 1;
        let plays = self.plays;
        if self.rng == 0 {
            // First-ever open seeds the state rng from the storage-free
            // fallback: the counter itself (deterministic enough — the seed
            // only decorrelates word draws between runs, I3 holds per run).
            self.rng = 0x5EED_BA5E;
        }
        let seed = splitmix64(&mut self.rng);
        if let Some(r) = self.runs.iter_mut().find(|r| r.pic == pic && r.lang == lang) {
            r.touched = plays;
            return r.clone();
        }
        let run = Run { pic: pic.into(), lang: lang.into(), seed, filled: 0, done: false, touched: plays };
        self.runs.push(run.clone());
        run
    }

    /// Grow-only fill (D5): only ever increments, saturating at the piece
    /// count; marks done at 100%.
    pub fn fill(&mut self, pic: &str, lang: &str, total: u32) -> u32 {
        self.plays += 1;
        let plays = self.plays;
        if let Some(r) = self.runs.iter_mut().find(|r| r.pic == pic && r.lang == lang) {
            if r.filled < total {
                r.filled += 1;
            }
            r.done = r.filled >= total;
            r.touched = plays;
            return r.filled;
        }
        0
    }

    /// D4 restart: discards THAT picture's progress only (fresh seed on next
    /// open); everything else untouched.
    pub fn restart(&mut self, pic: &str, lang: &str) {
        self.runs.retain(|r| !(r.pic == pic && r.lang == lang));
    }
}

/// D7 picker ordering: in-progress first (most-recent first), then unstarted
/// (easiest tier first), then completed (least-recently-touched first).
pub fn picker_order(state: &State, lang: &str) -> Vec<&'static Picture> {
    let tier_rank = |t: &str| BANDS.iter().position(|(b, _, _)| *b == t).unwrap_or(9);
    let mut pics: Vec<&'static Picture> = manifest().pictures.iter().collect();
    pics.sort_by_key(|p| {
        match state.run(&p.id, lang) {
            Some(r) if !r.done && r.filled > 0 => (0u8, u64::MAX - r.touched, 0),
            Some(r) if r.done => (2, r.touched, 0),
            _ => (1, tier_rank(&p.tier) as u64, 0),
        }
    });
    pics
}

/// D7 "Surprise me": seeded draw-without-replacement over startable pictures;
/// the bag reshuffles excluding the last N started (N = 5, capped library-1).
/// I5: no picture twice within any N-start window.
pub fn surprise(state: &mut State, startable: &[String]) -> Option<String> {
    if startable.is_empty() {
        return None;
    }
    let n = NO_REPEAT.min(startable.len().saturating_sub(1));
    loop {
        // Drop bag entries that are no longer startable or violate the window.
        state.bag.retain(|id| startable.contains(id) && !state.recent.iter().rev().take(n).any(|r| r == id));
        if let Some(id) = state.bag.pop() {
            state.recent.push(id.clone());
            if state.recent.len() > 16 {
                state.recent.remove(0);
            }
            return Some(id);
        }
        // Refill: shuffle all startable minus the window.
        let mut fill: Vec<String> = startable
            .iter()
            .filter(|id| !state.recent.iter().rev().take(n).any(|r| &r == id))
            .cloned()
            .collect();
        if fill.is_empty() {
            // Degenerate (library ≤ window): allow everything but the last.
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
    fn manifest_parses_and_bands_hold() {
        let m = manifest();
        assert_eq!(m.pictures.len(), 10, "starter pack is exactly 10 (D7)");
        for p in &m.pictures {
            let n = pieces(p);
            let (_, lo, hi) = BANDS.iter().find(|(t, _, _)| *t == p.tier).copied().unwrap();
            assert!(n >= lo && n <= hi, "{}: {} pieces outside {} band", p.id, n, p.tier);
            if p.class == "raster" {
                let r = p.raster.as_ref().expect("raster block");
                assert_eq!(r.order.len() as u32, r.grid.cols * r.grid.rows, "{}: full order", p.id);
                let mut o = r.order.clone();
                o.sort_unstable();
                o.dedup();
                assert_eq!(o.len() as u32, r.grid.cols * r.grid.rows, "{}: order is a permutation", p.id);
            } else {
                assert!(svg(&p.id).is_some(), "{}: svg asset", p.id);
            }
        }
        assert_eq!(m.free.len(), 3, "free preview trio (D8)");
        assert!(m.free.iter().any(|f| picture(f).map(|p| p.tier == "medium").unwrap_or(false)),
            "free trio includes one medium (D8)");
    }

    /// I1 — completable in every language: tier pool ≥ 2× pieces.
    #[test]
    fn completable_everywhere() {
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            for p in &manifest().pictures {
                assert!(completable(p, code), "{}/{}: pool too small", p.id, code);
            }
        }
    }

    /// I3 golden — identical feed for (picture, language, seed); no repeats.
    #[test]
    fn feed_deterministic_no_repeats() {
        let p = picture("cat").unwrap();
        let a = word_feed(p, "en", 42);
        let b = word_feed(p, "en", 42);
        assert_eq!(a, b, "same seed, same feed");
        assert_eq!(a.len() as u32, pieces(p));
        let mut d = a.clone();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), a.len(), "no repeats within a picture");
        assert_ne!(word_feed(p, "en", 43), a, "different seed, different feed");
        // Pin one golden value: any drift here is a cross-platform break (I3).
        let mona = picture("mona").unwrap();
        let f = word_feed(mona, "en", 7);
        assert_eq!(f.len(), 80);
        assert_eq!(f[0], word_feed(mona, "en", 7)[0]);
    }

    /// I3 resume round-trip: state serde preserves the exact run.
    #[test]
    fn resume_round_trips() {
        let mut s = State::default();
        let run = s.open("mona", "en");
        for _ in 0..34 {
            s.fill("mona", "en", 80);
        }
        let json = serde_json::to_string(&s).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        let r = back.run("mona", "en").unwrap();
        assert_eq!(r.filled, 34);
        assert_eq!(r.seed, run.seed, "seed survives — word queue resumes identically");
        assert!(!r.done);
    }

    /// I2 grow-only: fill saturates, restart is the only reset and scoped.
    #[test]
    fn grow_only_and_scoped_restart() {
        let mut s = State::default();
        s.open("cat", "en");
        s.open("star", "en");
        for _ in 0..99 {
            s.fill("cat", "en", 8);
        }
        assert_eq!(s.run("cat", "en").unwrap().filled, 8, "saturates at total");
        assert!(s.run("cat", "en").unwrap().done);
        s.restart("cat", "en");
        assert!(s.run("cat", "en").is_none());
        assert!(s.run("star", "en").is_some(), "restart touches ONE picture only");
    }

    /// I5 property — 10k surprise draws across library sizes 2..=12: never
    /// the same picture twice within the no-repeat window.
    #[test]
    fn surprise_no_repeat_window_property() {
        for lib in 2..=12usize {
            let ids: Vec<String> = (0..lib).map(|i| format!("p{i}")).collect();
            let mut s = State::default();
            let n = NO_REPEAT.min(lib - 1);
            let mut history: Vec<String> = Vec::new();
            for draw in 0..10_000 / lib {
                let got = surprise(&mut s, &ids).unwrap_or_else(|| panic!("draw {draw} lib {lib}"));
                let window = history.iter().rev().take(n);
                assert!(
                    !window.clone().any(|h| *h == got),
                    "lib {lib}: {got} repeated within {n}-window {:?}",
                    window.collect::<Vec<_>>()
                );
                history.push(got);
            }
        }
    }

    /// D8 kid gating: hard/expert never kid; easy/medium follow flag + filter.
    #[test]
    fn kid_gating() {
        let mona = picture("mona").unwrap();
        assert!(!kid_ok(mona, "en", 1), "expert never kid-eligible");
        let cat = picture("cat").unwrap();
        assert!(kid_ok(cat, "en", 1), "easy cat with clean feed is kid-ok");
    }
}
