//! Shuffled "deck" word selection, replacing pick-a-random-word-each-turn
//! (which can repeat a word back-to-back or in tight clusters). Each pool
//! (a language+difficulty combo, or the misses-review pool) gets its own
//! deck that's drawn all the way down before reshuffling.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

const RECENT_CAP: usize = 5;

/// Persisted so the shuffled order + cursor survive an app restart (I4): a word
/// still won't recur until the whole pool is exhausted, even across sessions and
/// days. `pool_len` also detects a word-list change and forces a reshuffle.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Deck {
    queue: Vec<String>,
    recent: VecDeque<String>,
    pool_len: usize,
}

#[cfg(not(test))]
fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (js_sys::Math::random() * len as f64).floor() as usize % len
}

// Test builds compile `next()` (game.rs calls it) but never run it — the unit
// tests drive `next_with` with a seeded PRNG instead. This stub only needs to
// exist so the crate compiles under `cargo test` without js_sys.
#[cfg(test)]
fn rand_index(_len: usize) -> usize {
    0
}

fn shuffled(pool: &[String], rand: &mut dyn FnMut(usize) -> usize) -> Vec<String> {
    let mut v: Vec<String> = pool.to_vec();
    for i in (1..v.len()).rev() {
        let j = rand(i + 1);
        v.swap(i, j);
    }
    v
}

impl Deck {
    /// Pops the next word, reshuffling from `pool` first if the deck is
    /// empty or the pool's size changed since the last draw (e.g. custom
    /// words were edited).
    pub fn next(&mut self, pool: &[String]) -> String {
        self.next_with(pool, &mut |n| rand_index(n))
    }

    /// Core draw, parameterized by the index-picker so it's deterministically
    /// testable (prod passes `Math::random`; tests pass a seeded PRNG).
    pub fn next_with(&mut self, pool: &[String], rand: &mut dyn FnMut(usize) -> usize) -> String {
        if pool.is_empty() {
            return "word".to_string();
        }
        if self.queue.is_empty() || self.pool_len != pool.len() {
            self.rebuild(pool, rand);
        }
        let word = self.queue.pop().unwrap_or_else(|| pool[rand(pool.len())].clone());
        self.recent.push_back(word.clone());
        // Cap below the pool size (never the *whole* pool) so the
        // reshuffle-boundary guard in `rebuild` always has at least one
        // non-recent candidate to fall back on, even for tiny pools.
        let cap = RECENT_CAP.min(pool.len().saturating_sub(1));
        while self.recent.len() > cap {
            self.recent.pop_front();
        }
        word
    }

    /// Peeks the word `next()` would return right now, without consuming
    /// it — used to preload its audio while the current turn is still
    /// being played out. Returns `None` right as a pass is about to finish
    /// (the following `next()` would trigger a reshuffle) rather than
    /// simulating that reshuffle just to preload one word early.
    pub fn peek(&self) -> Option<String> {
        self.queue.last().cloned()
    }

    fn rebuild(&mut self, pool: &[String], rand: &mut dyn FnMut(usize) -> usize) {
        let mut deck = shuffled(pool, rand);
        let n = deck.len();
        // The next `r` draws are popped from the tail. Ensure none repeats one of
        // the last `r` played (I3): swap each violating tail slot with the deepest
        // non-recent word (one deterministic pass). R = min(3, |pool|/2) so the
        // window shrinks on tiny pools (feasible needs pool ≥ 2R) — graceful
        // degradation for Little Speller's small sets.
        let r = 3.min(n / 2).min(self.recent.len());
        let recent_r: std::collections::HashSet<&String> = self.recent.iter().rev().take(r).collect();
        for k in 0..r {
            let tail = n - 1 - k;
            if recent_r.contains(&deck[tail]) {
                if let Some(src) = (0..tail).rev().find(|&i| !recent_r.contains(&deck[i])) {
                    deck.swap(tail, src);
                }
            }
        }
        self.queue = deck;
        self.pool_len = pool.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Deterministic index picker (splitmix64) so the deck is unit-testable
    /// without js_sys::Math::random.
    fn seeded() -> impl FnMut(usize) -> usize {
        let mut s: u64 = 0x1234_5678_9abc_def0;
        move |n| {
            s = s.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            (z as usize) % n.max(1)
        }
    }

    /// I3: no word repeats before the pool is exhausted, and across a reshuffle
    /// no repeat within the last R=min(3,|pool|-1) draws.
    #[test]
    fn deck_no_repeat() {
        for pool_size in [2usize, 3, 10, 41, 200] {
            let pool: Vec<String> = (0..pool_size).map(|i| format!("w{i}")).collect();
            let mut deck = Deck::default();
            let mut rand = seeded();
            let got: Vec<String> = (0..5 * pool_size).map(|_| deck.next_with(&pool, &mut rand)).collect();

            // Each full pass (pool_size consecutive draws, pass-aligned) is distinct.
            for pass in got.chunks(pool_size).filter(|c| c.len() == pool_size) {
                let uniq: HashSet<_> = pass.iter().collect();
                assert_eq!(uniq.len(), pool_size, "pool {pool_size}: repeat within a pass");
            }
            // Across every pass boundary, the new pass's first R exclude the
            // previous pass's last R.
            let r = 3.min(pool_size / 2);
            let mut b = pool_size;
            while b < got.len() {
                let prev: HashSet<_> = got[b - r..b].iter().collect();
                for w in &got[b..(b + r).min(got.len())] {
                    assert!(!prev.contains(w), "pool {pool_size}: '{w}' repeated within R={r} across reshuffle at {b}");
                }
                b += pool_size;
            }
        }
    }

    /// I4-adjacent: a mid-run pool change rebuilds cleanly (no crash/empty draw).
    #[test]
    fn deck_handles_pool_growth() {
        let mut deck = Deck::default();
        let mut rand = seeded();
        let small: Vec<String> = (0..5).map(|i| format!("w{i}")).collect();
        for _ in 0..5 {
            deck.next_with(&small, &mut rand);
        }
        let big: Vec<String> = (0..12).map(|i| format!("w{i}")).collect();
        let w = deck.next_with(&big, &mut rand);
        assert!(big.contains(&w));
    }
}
