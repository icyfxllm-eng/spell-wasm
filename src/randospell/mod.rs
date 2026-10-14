//! CC-RANDOSPELL v1 Phase A -- the engine: dice, word supply, scoring, Sprint.
//!
//! A pure reducer. No DOM, no wall clock, no float in anything that decides an
//! outcome; every random choice comes from `spelldoku::rng::Rng`, so a seed, a
//! bank and a list of actions give the same session on every target (I1, I3).
//! The 30 s timer (D19) lives in the screen layer, which calls `timeout`.
//!
//! # Two streams, on purpose
//!
//! The dice and the words draw from separate streams derived from the seed. The
//! dice therefore depend only on how many presses have happened, never on which
//! words a bank holds: a roll is a pure function of the stream state at
//! press-time (I1), and the animation has no way to reach it (A3).
//!
//! # Length, tier and the bank (census, Eric 2026-10-14)
//!
//! Length coverage POOLS all four tier files. The tier the player is on only
//! biases which words are preferred at a rolled length: the player's tier
//! first, then the nearest tier that has an unused word of that length. The
//! stored length unit is derived (grapheme clusters, spaces excluded, D12);
//! there is no stored field in the bank (census finding 1).
//!
//! # Not here yet
//!
//! Marathon pot/Bank/Ward (B), Daily and Head-to-Head (C), the Jr die (D).

#![allow(dead_code)] // wired by the screen in the next Phase A step

pub mod bank;
#[cfg(test)]
mod tests;

use crate::spelldoku::rng::Rng;
pub use bank::Bank;

pub const SPRINT_GOAL: u32 = 50;
/// D19: a standard-mode spelling times out after this long (a miss).
pub const TIMER_SECS: u32 = 30;
/// D7: three correct in a row switch the multiplier on.
pub const STREAK_FOR_MULTIPLIER: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sprint,
}

impl Mode {
    pub fn goal(self) -> u32 {
        match self {
            Mode::Sprint => SPRINT_GOAL,
        }
    }
}

/// A die face is the sum of two dice, 2..=12 (F1).
pub fn is_jackpot(face: u8) -> bool {
    face == 2 || face == 12
}

/// F3: what a correct answer on `face` is worth BEFORE the streak multiplier.
pub fn base_points(face: u8) -> u32 {
    let v = face as u32;
    if is_jackpot(face) {
        v * 2
    } else {
        v
    }
}

/// F3/D7: ceil(base x 1.5), in integers.
pub fn multiplied(base: u32) -> u32 {
    (base * 3).div_ceil(2)
}

/// One press of the orb: the face, the length after any D4 remap, the word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roll {
    pub face: u8,
    pub len: usize,
    pub word: String,
}

/// What happened when the roll was answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub correct: bool,
    pub points: u32,
    /// True when the 1.5 multiplier was applied to this answer.
    pub multiplied: bool,
    pub streak: u32,
    pub score: u32,
    pub finished: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Goal,
    /// A length had no unused word left: the session ends rather than repeat (I3).
    Exhausted,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub mode: Mode,
    pub score: u32,
    pub streak: u32,
    pub rolls: u32,
    pub correct: u32,
    /// Faces rolled, index 0 is face 2 (D20).
    pub histogram: [u32; 11],
    pub longest: Option<String>,
    pub ended: Option<End>,
    tier: usize,
    dice: Rng,
    words: Rng,
    bank: Bank,
    used: Vec<String>,
    avoid: std::collections::BTreeSet<String>,
    pending: Option<Roll>,
}

impl Session {
    /// `tier` is 0..=3 (easy..expert). `avoid` is the cross-session history
    /// (D9): words held there are used only after the fresh ones run out.
    pub fn new(seed: u64, mode: Mode, tier: usize, bank: Bank, avoid: std::collections::BTreeSet<String>) -> Session {
        Session {
            mode,
            score: 0,
            streak: 0,
            rolls: 0,
            correct: 0,
            histogram: [0; 11],
            longest: None,
            ended: None,
            tier: tier.min(3),
            dice: Rng::new(seed ^ 0xD1CE_D1CE_5EED_0001),
            words: Rng::new(seed ^ 0x0057_0AD5_0000_0002),
            bank,
            used: Vec::new(),
            avoid,
            pending: None,
        }
    }

    pub fn pending(&self) -> Option<&Roll> {
        self.pending.as_ref()
    }

    /// Every word served this session, in order: what the screen layer writes
    /// to the shared no-repeat history once (D9).
    pub fn served(&self) -> &[String] {
        &self.used
    }

    /// F1: the player pressed the orb. The face comes from the dice stream
    /// right here; nothing the animation does can change it (I1, A3).
    /// Returns None while a roll is unanswered or after the game has ended.
    pub fn press(&mut self) -> Option<Roll> {
        if self.pending.is_some() || self.ended.is_some() {
            return None;
        }
        let face = roll_two_dice(&mut self.dice);
        self.histogram[face as usize - 2] += 1;
        self.rolls += 1;
        let len = self.bank.cov.length_for(face);
        let word = len.and_then(|l| self.pick(l));
        match (len, word) {
            (Some(len), Some(word)) => {
                self.used.push(word.clone());
                let r = Roll { face, len, word };
                self.pending = Some(r.clone());
                Some(r)
            }
            _ => {
                self.ended = Some(End::Exhausted);
                None
            }
        }
    }

    /// F4: score the pending roll. A miss is 0 and resets the streak (D5).
    pub fn answer(&mut self, correct: bool) -> Option<Outcome> {
        let r = self.pending.take()?;
        let mut points = 0;
        let mut mult = false;
        if correct {
            let mut p = base_points(r.face);
            if self.streak >= STREAK_FOR_MULTIPLIER {
                p = multiplied(p);
                mult = true;
            }
            points = p;
            self.streak += 1;
            self.correct += 1;
            self.score += p; // I5: score only ever rises.
            let longer = self.longest.as_ref().map_or(true, |w| r.len > bank::length_of(w));
            if longer {
                self.longest = Some(r.word.clone());
            }
        } else {
            self.streak = 0;
        }
        let finished = self.score >= self.mode.goal();
        if finished {
            self.ended = Some(End::Goal);
        }
        Some(Outcome { correct, points, multiplied: mult, streak: self.streak, score: self.score, finished })
    }

    /// D19: the 30 s soft timer ran out. A miss.
    pub fn timeout(&mut self) -> Option<Outcome> {
        self.answer(false)
    }

    fn pick(&mut self, len: usize) -> Option<String> {
        let mut cands: Vec<(usize, String)> = self
            .bank
            .of_len(len)
            .iter()
            .filter(|e| !self.used.contains(&e.word))
            .map(|e| (e.tier, e.word.clone()))
            .collect();
        if cands.is_empty() {
            return None;
        }
        // Fresh (not in the history) first, then the player's tier, then the
        // nearest tier: tier biases difficulty and nothing else.
        if cands.iter().any(|(_, w)| !self.avoid.contains(w)) {
            cands.retain(|(_, w)| !self.avoid.contains(w));
        }
        let best = cands.iter().map(|(t, _)| t.abs_diff(self.tier)).min()?;
        cands.retain(|(t, _)| t.abs_diff(self.tier) == best);
        let i = self.words.below(cands.len());
        Some(cands.swap_remove(i).1)
    }
}

/// F1: two dice, 2..=12. Two draws from the stream, in a fixed order.
pub fn roll_two_dice(rng: &mut Rng) -> u8 {
    let a = rng.below(6) as u8 + 1;
    let b = rng.below(6) as u8 + 1;
    a + b
}
