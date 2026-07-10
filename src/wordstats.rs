//! Adaptive, spaced-repetition-weighted word selection for SOLO normal play.
//! Words the player has missed resurface more often; mastered ones fade. All
//! stats live in one localStorage blob (`spell_word_stats_v1`) — offline,
//! anonymous, no network. Everything degrades gracefully: if storage is
//! unavailable or the blob is corrupt/unknown-version, we start fresh and the
//! selector falls back to (near-)uniform picking, never blocking gameplay.
//!
//! Only solo practice uses this. Competitive (head-to-head) keeps the plain
//! shuffled deck so both players face the same distribution, and Misses/review
//! has its own spaced-repetition queue (`misses.rs`) — neither records here.
//! (There is no Daily Challenge in this app; if one is added, it must bypass
//! this selector to stay deterministic.)

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::storage;

const KEY: &str = "spell_word_stats_v1";
const VERSION: u32 = 1;
const CAP: usize = 500;

#[derive(Serialize, Deserialize, Clone, Default)]
struct WordStat {
    #[serde(default)]
    a: u32, // attempts
    #[serde(default)]
    m: u32, // misses
    #[serde(default)]
    s: u32, // current correct streak
    #[serde(default)]
    t: f64, // last shown, unix seconds
}

#[derive(Serialize, Deserialize)]
struct Blob {
    v: u32,
    words: HashMap<String, WordStat>,
}

impl Default for Blob {
    fn default() -> Self {
        Blob { v: VERSION, words: HashMap::new() }
    }
}

fn load() -> Blob {
    // Unknown version / corrupt / missing -> fresh (no migration attempts).
    match storage::get_json::<Blob>(KEY) {
        Some(b) if b.v == VERSION => b,
        _ => Blob::default(),
    }
}

fn store(b: &Blob) {
    storage::set_json(KEY, b);
}

fn now_secs() -> f64 {
    js_sys::Date::now() / 1000.0
}

fn norm_key(lang: &str, word: &str) -> String {
    // Keyed by (locale, word) so mastery in Español never pollutes Français
    // (§4.1) — words like "sol"/"casa"/"banana" exist in several languages.
    // Case-normalized to match how answers are compared.
    format!("{}::{}", lang, word.to_lowercase())
}

thread_local! {
    // Session-only ring of the most recent picks (last 10 used for recency
    // damping; last 3 excluded outright as the no-repeat guard).
    static RECENT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn push_recent(k: &str) {
    RECENT.with(|r| {
        let mut r = r.borrow_mut();
        r.push(k.to_string());
        let n = r.len();
        if n > 10 {
            r.drain(0..n - 10);
        }
    });
}

/// Keep the blob small: at most CAP entries. Over cap, evict mastered (s>=3)
/// stale words first (oldest `t`), then oldest overall if still over.
fn evict(b: &mut Blob) {
    if b.words.len() <= CAP {
        return;
    }
    let mut entries: Vec<(String, f64, u32)> =
        b.words.iter().map(|(k, v)| (k.clone(), v.t, v.s)).collect();
    entries.sort_by(|x, y| {
        let (xm, ym) = (x.2 >= 3, y.2 >= 3);
        // mastered first, then oldest-shown first
        ym.cmp(&xm).then(x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    let remove = b.words.len() - CAP;
    for (k, _, _) in entries.into_iter().take(remove) {
        b.words.remove(&k);
    }
}

/// Record a round's outcome for `word`. Called once per solo round at its final
/// result (a win, or a loss via out-of-tries / timeout / give-up — all misses).
pub fn record(lang: &str, word: &str, correct: bool) {
    let mut b = load();
    let e = b.words.entry(norm_key(lang, word)).or_default();
    e.a = e.a.saturating_add(1);
    if correct {
        e.s = e.s.saturating_add(1);
    } else {
        e.m = e.m.saturating_add(1);
        e.s = 0;
    }
    evict(&mut b);
    store(&b);
}

/// Weighted pick from `pool` (the active tier/mode list, unchanged). Returns
/// None only for an empty pool, so callers can fall back to the deck.
pub fn pick(lang: &str, pool: &[String]) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    let mut b = load();
    let recent = RECENT.with(|r| r.borrow().clone());
    let n = recent.len();
    let last3: Vec<&String> = recent.iter().skip(n.saturating_sub(3)).collect();

    // Candidate pool minus the last 3 picks (unless that would empty it).
    let mut candidates: Vec<&String> = pool
        .iter()
        .filter(|w| !last3.iter().any(|r| r.eq_ignore_ascii_case(w)))
        .collect();
    if candidates.is_empty() {
        candidates = pool.iter().collect();
    }

    let mut cum: Vec<f64> = Vec::with_capacity(candidates.len());
    let mut total = 0.0f64;
    for w in &candidates {
        let (a, m, s, seen) = match b.words.get(&norm_key(lang, w)) {
            Some(x) => (x.a as f64, x.m as f64, x.s as f64, true),
            None => (0.0, 0.0, 0.0, false),
        };
        let base = 1.0;
        let miss_boost = 1.5 * m / a.max(1.0);
        let streak_damp = 1.0 / (1.0 + s);
        let recency_damp = if recent.iter().any(|r| r.eq_ignore_ascii_case(w)) { 0.25 } else { 1.0 };
        let novelty = if seen { 1.0 } else { 1.3 };
        // Clamp above zero so the pool never starves.
        let weight = ((base + miss_boost) * streak_damp * recency_damp * novelty).max(1e-6);
        total += weight;
        cum.push(total);
    }

    let roll = js_sys::Math::random() * total;
    let idx = cum.iter().position(|&c| roll <= c).unwrap_or(cum.len().saturating_sub(1));
    let chosen = candidates[idx].clone();

    // Stamp last-shown, evict, persist once; update the recency ring.
    b.words.entry(norm_key(lang, &chosen)).or_default().t = now_secs();
    evict(&mut b);
    store(&b);
    push_recent(&chosen);
    Some(chosen)
}

/// Clear all adaptive word stats (wired into the existing reset affordance).
pub fn clear() {
    store(&Blob::default());
}
