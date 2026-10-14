//! CC-RANDOSPELL F9 -- the mode-local store: best Sprint rolls-to-50, best
//! Marathon pot banked, Daily results and dice histograms. It is its own key
//! and never touches Climb, Daily Challenge or shield state (I9).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Mode, Session};

pub const STORE_KEY: &str = "spell_randospell_v1";
/// Daily results kept on the device; the oldest dates fall off.
const KEEP_DAILY: usize = 400;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyEntry {
    /// The game was played to its end. An entry that is not done is an attempt
    /// that was abandoned, and it still spends the day's one attempt.
    pub done: bool,
    pub score: u32,
    pub rolls: u32,
    pub correct: u32,
    pub best_pot: u32,
    pub ward_used: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Store {
    pub best_sprint_rolls: Option<u32>,
    pub best_marathon_pot: u32,
    pub daily: BTreeMap<String, DailyEntry>,
    /// Every roll ever displayed, index 0 is a roll of 2 (D20).
    pub histogram: [u32; 11],
}

impl Store {
    /// F6: one attempt per day per language per tier. Returns false when the
    /// attempt has already been spent. The attempt is spent at the START, so a
    /// player cannot replay a day whose words they have already heard.
    pub fn try_start_daily(&mut self, key: &str) -> bool {
        if self.daily.contains_key(key) {
            return false;
        }
        self.daily.insert(key.to_string(), DailyEntry::default());
        while self.daily.len() > KEEP_DAILY {
            let oldest = self.daily.keys().next().cloned();
            match oldest {
                Some(k) => self.daily.remove(&k),
                None => break,
            };
        }
        true
    }

    pub fn daily_entry(&self, key: &str) -> Option<&DailyEntry> {
        self.daily.get(key)
    }

    /// Record a finished Daily (or an abandoned one, with `done` false).
    pub fn finish_daily(&mut self, key: &str, s: &Session, done: bool) {
        if let Some(e) = self.daily.get_mut(key) {
            if e.done {
                return; // a result is written once
            }
            *e = DailyEntry { done, score: s.score, rolls: s.rolls, correct: s.correct, best_pot: s.largest_banked, ward_used: s.ward_used };
        }
    }

    /// Fold a finished game into the bests and the histogram.
    pub fn record_game(&mut self, s: &Session) {
        for (a, b) in self.histogram.iter_mut().zip(s.histogram.iter()) {
            *a += *b;
        }
        match s.mode {
            Mode::Sprint if s.ended == Some(super::End::Goal) => {
                self.best_sprint_rolls = Some(self.best_sprint_rolls.map_or(s.rolls, |b| b.min(s.rolls)));
            }
            Mode::Marathon => self.best_marathon_pot = self.best_marathon_pot.max(s.largest_banked),
            _ => {}
        }
    }
}
