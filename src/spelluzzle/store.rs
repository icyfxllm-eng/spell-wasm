//! F11: what Spelluzzle keeps on the device, as plain values.
//!
//! Four stores, declared language-scoped except the streak (CC-LANG-SCOPE F5):
//! progress, history and stars are per (profile, language[, tier]); the streak
//! is language-neutral. The screen owns reading and writing them; the rules
//! live here so the host can test them. None of it is learner data, and none
//! of it leaves the device (I9).
//!
//! There is no profile concept in the app yet, so the profile is the audience:
//! `jr` for Spell Jr, `std` for everyone else.

use serde::{Deserialize, Serialize};

use super::play::Snapshot;
use super::types::{Tier, GEN_VERSION};

/// F11: board hashes kept per profile and language.
pub const HISTORY_CAP: usize = 500;

pub fn profile(kid: bool) -> &'static str {
    if kid {
        "jr"
    } else {
        "std"
    }
}

pub fn progress_key(profile: &str, lang: &str, tier: Tier) -> String {
    format!("spell_spz_progress_v1:{profile}:{lang}:{}", tier.name())
}
pub fn history_key(profile: &str, lang: &str) -> String {
    format!("spell_spz_history_v1:{profile}:{lang}")
}
pub fn stars_key(profile: &str, lang: &str, tier: Tier) -> String {
    format!("spell_spz_stars_v1:{profile}:{lang}:{}", tier.name())
}
/// F16: the pencil marks of the board in progress, with its hash so a different board never inherits them.
pub fn pencil_key(profile: &str, lang: &str, tier: Tier) -> String {
    format!("spell_spz_pencil_v1:{profile}:{lang}:{}", tier.name())
}
pub fn streak_key(profile: &str) -> String {
    format!("spell_spz_streak_v1:{profile}")
}

/// Every key the mode writes, for the store-registry and the no-writes test (I8).
pub const KEY_PREFIXES: [&str; 5] = ["spell_spz_progress_v1", "spell_spz_history_v1", "spell_spz_stars_v1", "spell_spz_streak_v1", "spell_spz_pencil_v1"];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct History {
    pub hashes: Vec<u64>,
    /// The words of the last board shown, so the next one shares at most two (F11).
    #[serde(default)]
    pub last_words: Vec<String>,
}

impl History {
    pub fn contains(&self, h: u64) -> bool {
        self.hashes.contains(&h)
    }
    /// Abandoned boards count too, so this is called when a board is shown.
    pub fn push(&mut self, h: u64) {
        self.hashes.retain(|x| *x != h);
        self.hashes.push(h);
        let n = self.hashes.len();
        if n > HISTORY_CAP {
            self.hashes.drain(0..n - HISTORY_CAP);
        }
    }
}

/// The one board in progress for (profile, language, tier).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Progress {
    pub seed: u64,
    /// The previous board's words when this one was drawn: regenerating needs them (F11 overlap rule).
    #[serde(default)]
    pub previous: Vec<String>,
    pub gen_version: u32,
    pub hash: u64,
    pub snapshot: Snapshot,
}

impl Progress {
    /// A saved board resumes only if regenerating its seed gives the same board.
    pub fn matches(&self, regenerated_hash: u64) -> bool {
        self.gen_version == GEN_VERSION && self.hash == regenerated_hash
    }
}

/// Best stars per tier (a count from 0 to 3). No leaderboard.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BestStars {
    pub best: u8,
}

impl BestStars {
    pub fn record(&mut self, stars: u8) {
        self.best = self.best.max(stars.min(3));
    }
}

/// Solving any board counts once per local calendar day (R2).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Streak {
    pub last: String,
    pub count: u32,
}

impl Streak {
    /// `today` and `yesterday` are local YYYY-MM-DD dates. Replays on the same
    /// day change nothing, and there is no "already played today" gate.
    pub fn solved(&mut self, today: &str, yesterday: &str) {
        if self.last == today {
            return;
        }
        self.count = if self.last == yesterday { self.count + 1 } else { 1 };
        self.last = today.to_string();
    }

    /// The streak as it should read now: broken if the last solve was before yesterday.
    pub fn current(&self, today: &str, yesterday: &str) -> u32 {
        if self.last == today || self.last == yesterday {
            self.count
        } else {
            0
        }
    }
}

/// The pencil marks of one board.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PencilSave {
    pub hash: u64,
    pub marks: Vec<(u8, char)>,
}
