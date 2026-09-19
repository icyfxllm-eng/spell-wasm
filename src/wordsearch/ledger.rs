//! F-X2 seeds, the F-X3 no-repeat engine, and the Daily schedule.

use serde::{Deserialize, Serialize};

use super::lexicon::graphemes;
use crate::spelldoku::rng::{fnv, Rng};

/// D9 (signed): a served word stays out of bank puzzles for 20 puzzles or 14
/// days, whichever is longer.
pub const WINDOW_PUZZLES: u64 = 20;
pub const WINDOW_DAYS: u32 = 14;
/// D9: the Daily Puzzle's window. On the device (Phase A) the Daily holds the
/// widest spacing its pool allows, up to this; the global window is Phase C.
pub const DAILY_WINDOW_DAYS: u32 = 90;

/// F-X2: `seed = hash(source_id, player_puzzle_counter, date_for_daily)`.
pub fn seed(source: &str, counter: u64, date: u32) -> u64 {
    fnv(format!("{source}|{counter}|{date}").as_bytes())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Seen {
    /// `lang:tier`
    pub k: String,
    pub w: String,
    /// The player's puzzle counter when it was served.
    pub n: u64,
    pub d: u32,
}

/// The per-player seen-word ledger. It lives on the device, so it works with no
/// account (CC-ONBOARD-JR D1). Spell Cross will share it: a word served in either
/// mode counts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// `player_puzzle_counter`: every puzzle this player has been served.
    pub counter: u64,
    pub seen: Vec<Seen>,
    /// F-X3: how many times a pool ran dry and the window was relaxed. Kept on
    /// the device only: there is no telemetry.
    #[serde(default)]
    pub relaxations: u64,
}

impl Ledger {
    pub fn in_window(&self, s: &Seen, day: u32) -> bool {
        self.counter.saturating_sub(s.n) < WINDOW_PUZZLES || day.saturating_sub(s.d) < WINDOW_DAYS
    }

    fn held(&self, key: &str, word: &str, day: u32) -> Option<&Seen> {
        self.seen.iter().find(|s| s.k == key && s.w == word && self.in_window(s, day))
    }

    /// Add words from `pool` to `picked` until it holds `until`. Unseen words
    /// first, in shuffled order; then, if the pool runs dry, the words seen
    /// longest ago (F-X3: relax, never error). Returns how many were relaxed.
    #[allow(clippy::too_many_arguments)]
    pub fn select(
        &self,
        key: &str,
        pool: &[String],
        picked: &mut Vec<String>,
        until: usize,
        day: u32,
        rng: &mut Rng,
        fits: &dyn Fn(&[String], &str) -> bool,
    ) -> usize {
        let mut fresh: Vec<&String> = pool.iter().filter(|w| self.held(key, w, day).is_none()).collect();
        rng.shuffle(&mut fresh);
        for w in fresh {
            if picked.len() >= until {
                return 0;
            }
            if fits(picked, w) {
                picked.push(w.clone());
            }
        }
        let mut stale: Vec<(&Seen, &String)> = pool.iter().filter_map(|w| self.held(key, w, day).map(|s| (s, w))).collect();
        stale.sort_by(|a, b| (a.0.n, a.0.d, a.1).cmp(&(b.0.n, b.0.d, b.1)));
        let mut relaxed = 0;
        for (_, w) in stale {
            if picked.len() >= until {
                break;
            }
            if fits(picked, w) {
                picked.push(w.clone());
                relaxed += 1;
            }
        }
        relaxed
    }

    /// A puzzle was served: its words enter the window and the counter moves on.
    pub fn record(&mut self, key: &str, words: &[String], day: u32, relaxed: usize) {
        for w in words {
            self.seen.retain(|s| !(s.k == key && s.w == *w));
            self.seen.push(Seen { k: key.to_string(), w: w.clone(), n: self.counter, d: day });
        }
        self.counter += 1;
        self.relaxations += relaxed as u64;
        let keep: Vec<bool> = self.seen.iter().map(|s| self.in_window(s, day)).collect();
        let mut i = 0;
        self.seen.retain(|_| {
            i += 1;
            keep[i - 1]
        });
    }
}

/// F-X3 within a puzzle: no word twice (after NFC and casefolding), no target
/// inside another forwards or backwards (car / cart: the shorter waits for a
/// later puzzle), no two words that sound alike, and it fits the grid.
pub fn fits(lang: &str, size: usize, picked: &[String], w: &str) -> bool {
    let g = graphemes(w);
    if g.len() < 3 || g.len() > size {
        return false;
    }
    let f = g.concat();
    let r: String = g.iter().rev().cloned().collect();
    picked.iter().all(|p| {
        !(p.contains(&f) || f.contains(p.as_str()) || p.contains(&r) || r.contains(p.as_str()))
            && !crate::homophones::accepts(lang, p, &f)
    })
}

/// How many days before a Daily word can come back, for a pool of `len` words
/// and `n` targets a day.
pub fn daily_gap(len: usize, n: usize) -> usize {
    let block = (len / DAILY_WINDOW_DAYS as usize).max(n);
    (len / block.max(1)).max(1)
}

/// The Daily Puzzle's words for `day`: the same for every player. The pool is
/// shuffled once, cut into blocks, and each day plays the next block, so a word
/// returns only after every other block has had its day (`daily_gap`).
pub fn daily_pick(pool: &[String], n: usize, day: u32, salt: &str, fits: &dyn Fn(&[String], &str) -> bool) -> Vec<String> {
    if pool.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut order = pool.to_vec();
    Rng::new(fnv(format!("daily-order|{salt}").as_bytes())).shuffle(&mut order);
    let block = (order.len() / DAILY_WINDOW_DAYS as usize).max(n);
    let blocks = (order.len() / block).max(1);
    let b = day as usize % blocks;
    let mut mine = order[b * block..((b + 1) * block).min(order.len())].to_vec();
    Rng::new(fnv(format!("daily-day|{salt}|{day}").as_bytes())).shuffle(&mut mine);
    let mut picked: Vec<String> = Vec::new();
    for w in mine {
        if picked.len() == n {
            break;
        }
        if fits(&picked, &w) {
            picked.push(w);
        }
    }
    picked
}
