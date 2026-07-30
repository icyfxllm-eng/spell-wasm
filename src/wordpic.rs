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
pub const BANDS: [(&str, u32, u32); 4] =
    [("easy", 1, 8), ("medium", 1, 20), ("hard", 1, 45), ("expert", 1, 200)];

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
    /// Stacks: intended letter count — v6 fixes the COLUMN height at
    /// column × size and solves glyph size per word (fill-the-column).
    #[serde(default)]
    pub column: u32,
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
    #[serde(default)]
    pub kid: bool,
    #[serde(default)]
    pub wash: bool,
    #[serde(default)]
    pub pack: String,
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
        for w in feed.iter().take(12) {
            s.place("mona", "en", w, slots(mona));
        }
        let back: State = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let r = back.run("mona", "en").unwrap();
        assert_eq!(r.words.len(), 12);
        assert_eq!(r.seed, run.seed);
        assert_eq!(r.words, feed[..12].to_vec(), "resume continues the exact queue");
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
