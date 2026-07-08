//! Shuffled "deck" word selection, replacing pick-a-random-word-each-turn
//! (which can repeat a word back-to-back or in tight clusters). Each pool
//! (a language+difficulty combo, or the misses-review pool) gets its own
//! deck that's drawn all the way down before reshuffling.

use std::collections::VecDeque;

const RECENT_CAP: usize = 5;

#[derive(Default)]
pub struct Deck {
    queue: Vec<String>,
    recent: VecDeque<String>,
    pool_len: usize,
}

fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (js_sys::Math::random() * len as f64).floor() as usize % len
}

fn shuffled(pool: &[String]) -> Vec<String> {
    let mut v: Vec<String> = pool.to_vec();
    for i in (1..v.len()).rev() {
        let j = rand_index(i + 1);
        v.swap(i, j);
    }
    v
}

impl Deck {
    /// Pops the next word, reshuffling from `pool` first if the deck is
    /// empty or the pool's size changed since the last draw (e.g. custom
    /// words were edited).
    pub fn next(&mut self, pool: &[String]) -> String {
        if pool.is_empty() {
            return "word".to_string();
        }
        if self.queue.is_empty() || self.pool_len != pool.len() {
            self.rebuild(pool);
        }
        let word = self.queue.pop().unwrap_or_else(|| pool[rand_index(pool.len())].clone());
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

    fn rebuild(&mut self, pool: &[String]) {
        let mut deck = shuffled(pool);
        // Avoid the reshuffle handing back something from the tail of what
        // was just played, so a reshuffle boundary doesn't itself create a
        // near-repeat.
        if deck.len() > 1 {
            let mut guard = 0;
            while self.recent.contains(deck.last().unwrap()) && guard < deck.len() {
                deck.rotate_left(1);
                guard += 1;
            }
        }
        self.queue = deck;
        self.pool_len = pool.len();
    }
}
