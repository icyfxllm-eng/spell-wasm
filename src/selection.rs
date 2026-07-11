//! Part 2 — orb runtime word selection over the current pools.
//!
//! Layers three things on top of the existing adaptive selector ([`wordstats`])
//! without regressing it (SRS/Misses still run upstream in `game::next_word`;
//! this only shapes the *fresh-word* choice for solo play — head-to-head keeps
//! its plain shared deck):
//!
//!   1. **Rolling per-tier exclusion** — the last N served words for a tier are
//!      held out (N = min(50, pool/4)), persisted with progress, so a tier
//!      doesn't recycle the same handful.
//!   2. **Within-tier sub-bands** — the pool is split into 3 difficulty bands
//!      and consecutive fresh words cycle low→mid→high, so a tier feels
//!      coherent instead of spiking word-to-word.
//!   3. **First-impression rule** — the first 3 words after a session start /
//!      tier switch are mid-band, since a tier's felt identity is set by its
//!      opening words (never open on an edge case).
//!
//! The band split currently uses **word length as the within-island difficulty
//! proxy** (the same proxy `daily.rs` uses), because per-word difficulty scores
//! are computed offline and not shipped at runtime yet. When the scoring
//! pipeline emits per-word scores into the pool data, swap `sorted_by_difficulty`
//! to read them — nothing else changes. The band/cycle/exclusion logic below is
//! pure and unit-tested; the storage- and RNG-backed orchestration is not.

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{storage, wordstats};

// ---- pure logic (host-testable; no js_sys / storage) --------------------------

/// Rolling-exclusion size for a tier: the last N served words are held out.
pub fn exclusion_cap(pool_len: usize) -> usize {
    (pool_len / 4).min(50)
}

/// Pool ordered easiest→hardest. Length is the runtime proxy for within-island
/// difficulty percentile until the pipeline ships per-word scores; the stable
/// secondary key keeps banding deterministic.
fn sorted_by_difficulty(pool: &[String]) -> Vec<&String> {
    let mut v: Vec<&String> = pool.iter().collect();
    v.sort_by(|a, b| a.chars().count().cmp(&b.chars().count()).then_with(|| a.as_str().cmp(b.as_str())));
    v
}

/// The words in sub-band `band` (0 = low/easy third, 1 = mid, 2 = high/hard
/// third). Empty only for an empty pool; for pools < 3 words some bands are
/// empty and the caller relaxes.
pub fn band_members(pool: &[String], band: usize) -> Vec<String> {
    let sorted = sorted_by_difficulty(pool);
    let n = sorted.len();
    if n == 0 {
        return Vec::new();
    }
    let lo = band * n / 3;
    let hi = if band >= 2 { n } else { (band + 1) * n / 3 };
    sorted[lo..hi].iter().map(|s| s.to_string()).collect()
}

/// Which sub-band to serve given how many fresh words have been served since the
/// last tier switch. First 3 are mid-band (first-impression rule); after that,
/// cycle low→mid→high.
pub fn target_band(served_since_switch: u32) -> usize {
    if served_since_switch < 3 {
        1 // mid — first impression
    } else {
        ((served_since_switch - 3) % 3) as usize // 0,1,2 = low,mid,high
    }
}

/// `band_pool` minus the rolling-excluded words, relaxing to the full band if
/// exclusion would empty it (tiny pools).
pub fn available(band_pool: &[String], excluded: &[String]) -> Vec<String> {
    let avail: Vec<String> = band_pool
        .iter()
        .filter(|w| !excluded.iter().any(|e| e.eq_ignore_ascii_case(w)))
        .cloned()
        .collect();
    if avail.is_empty() {
        band_pool.to_vec()
    } else {
        avail
    }
}

// ---- persisted rolling exclusion ----------------------------------------------

const SERVED_KEY: &str = "spell_served_v1";

fn load_served() -> HashMap<String, Vec<String>> {
    storage::get_json(SERVED_KEY).unwrap_or_default()
}

fn recent_for(tier_key: &str) -> Vec<String> {
    load_served().remove(tier_key).unwrap_or_default()
}

/// Append `word` to the tier's rolling history, trimming to `cap` (FIFO).
fn push_served(tier_key: &str, word: &str, cap: usize) {
    let mut all = load_served();
    let q = all.entry(tier_key.to_string()).or_default();
    q.retain(|w| !w.eq_ignore_ascii_case(word));
    q.push(word.to_string());
    while q.len() > cap {
        q.remove(0);
    }
    storage::set_json(SERVED_KEY, &all);
}

// ---- session state (ephemeral: reset each app open) ---------------------------

#[derive(Default)]
struct Session {
    tier_key: String,
    served_since_switch: u32,
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

// ---- served-word telemetry (local, anonymous) ---------------------------------

const TRACE_KEY: &str = "spell_seltrace_v1";
const TRACE_CAP: usize = 200;

#[derive(Serialize, Deserialize, Default)]
struct Trace {
    w: String,
    tier: String,
    band: u8,
    correct: Option<bool>,
}

fn trace_serve(word: &str, tier: &str, band: usize) {
    let mut log: Vec<Trace> = storage::get_json(TRACE_KEY).unwrap_or_default();
    log.push(Trace { w: word.to_string(), tier: tier.to_string(), band: band as u8, correct: None });
    let n = log.len();
    if n > TRACE_CAP {
        log.drain(0..n - TRACE_CAP);
    }
    storage::set_json(TRACE_KEY, &log);
}

/// Stamp the outcome onto the most recent serve of `word` — this is the local
/// evidence stream the future evidence-based re-tiering consumes. Anonymous,
/// offline; degrades to a no-op if storage is unavailable.
pub fn note_outcome(_lang: &str, word: &str, correct: bool) {
    let mut log: Vec<Trace> = match storage::get_json(TRACE_KEY) {
        Some(l) => l,
        None => return,
    };
    if let Some(t) = log.iter_mut().rev().find(|t| t.w.eq_ignore_ascii_case(word) && t.correct.is_none()) {
        t.correct = Some(correct);
        storage::set_json(TRACE_KEY, &log);
    }
}

// ---- orchestration ------------------------------------------------------------

/// Pick a fresh solo-play word from `pool` for `(lang, tier)`, applying the
/// rolling exclusion + sub-band cycling + first-impression rule, then delegating
/// the final choice to the adaptive [`wordstats`] selector *within* the chosen
/// band so miss-weighting is preserved. `None` only for an empty pool (caller
/// falls back to the deck).
pub fn pick_solo(lang: &str, tier: &str, pool: &[String]) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    let key = format!("{lang}:{tier}");

    let served = SESSION.with(|s| {
        let mut st = s.borrow_mut();
        if st.tier_key != key {
            st.tier_key = key.clone();
            st.served_since_switch = 0;
        }
        st.served_since_switch
    });

    let band = target_band(served);
    let mut band_pool = band_members(pool, band);
    if band_pool.is_empty() {
        band_pool = pool.to_vec(); // pool smaller than 3 bands
    }
    let avail = available(&band_pool, &recent_for(&key));

    // Adaptive (miss-weighted) choice within the band; fall back to any word.
    let chosen = wordstats::pick(lang, &avail).or_else(|| avail.first().cloned())?;

    push_served(&key, &chosen, exclusion_cap(pool.len()));
    trace_serve(&chosen, tier, band);
    SESSION.with(|s| s.borrow_mut().served_since_switch += 1);
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exclusion_cap_is_quarter_capped_at_50() {
        assert_eq!(exclusion_cap(40), 10);
        assert_eq!(exclusion_cap(8), 2);
        assert_eq!(exclusion_cap(1000), 50); // capped
        assert_eq!(exclusion_cap(0), 0);
    }

    #[test]
    fn bands_split_by_length_low_to_high() {
        // 9 words, lengths 1..=9 → terciles of 3.
        let p = pool(&["a", "bb", "ccc", "dddd", "eeeee", "ffffff", "ggggggg", "hhhhhhhh", "iiiiiiiii"]);
        let low = band_members(&p, 0);
        let high = band_members(&p, 2);
        assert_eq!(low, pool(&["a", "bb", "ccc"]));
        assert_eq!(high, pool(&["ggggggg", "hhhhhhhh", "iiiiiiiii"]));
    }

    #[test]
    fn bands_cover_the_whole_pool_without_overlap() {
        let p = pool(&["a", "bb", "ccc", "dddd", "eeeee", "ff", "ggg", "hhhh"]);
        let mut all: Vec<String> = (0..3).flat_map(|b| band_members(&p, b)).collect();
        all.sort();
        let mut expect = p.clone();
        expect.sort();
        assert_eq!(all, expect); // partition: every word in exactly one band
    }

    #[test]
    fn first_three_are_mid_then_cycle() {
        assert_eq!(target_band(0), 1); // first-impression: mid
        assert_eq!(target_band(1), 1);
        assert_eq!(target_band(2), 1);
        assert_eq!(target_band(3), 0); // then cycle low→mid→high
        assert_eq!(target_band(4), 1);
        assert_eq!(target_band(5), 2);
        assert_eq!(target_band(6), 0);
    }

    #[test]
    fn available_holds_out_excluded_but_relaxes_when_empty() {
        let band = pool(&["cat", "dog", "fox"]);
        assert_eq!(available(&band, &pool(&["dog"])), pool(&["cat", "fox"]));
        // excluding everything relaxes back to the full band (never starve)
        assert_eq!(available(&band, &pool(&["cat", "dog", "fox"])), band);
        // case-insensitive exclusion
        assert_eq!(available(&band, &pool(&["DOG"])), pool(&["cat", "fox"]));
    }
}
