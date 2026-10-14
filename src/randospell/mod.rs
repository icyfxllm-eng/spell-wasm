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
//! # Marathon (Phase B)
//!
//! A correct answer adds to the POT, not the score, and the player must then
//! Bank or Roll again (F5). A miss empties the pot unless the one Ward absorbs
//! it (D13); the Ward is spent only by a loss of a non-empty pot. The game ends
//! only on a Bank that reaches 100 (D15), never because the pot would cross it.
//! If a length runs out of words the pot is banked and the game ends: score
//! never falls (I5), and nothing the player earned is thrown away.
//!
//! # Head-to-Head (Phase C)
//!
//! Two players on one phone share one dice stream and one word supply and
//! take turns. A turn is a run of rolls that ends on a Bank or a miss; a miss
//! hands the other player ONE bonus roll at the start of their next turn
//! (a flag, so two misses in a row do not stack). A bonus roll is free: a
//! right answer is banked at once, a wrong one costs nothing and grants
//! nothing. The first Bank that reaches 100 wins (D15). The idle player's
//! numbers live in `other`; `pass_turn` swaps them with the live fields so
//! everything that reads `score`, `pot` or `ward` shows the player on the move.
//!
//! # Not here yet
//!
//! The Jr die (D).

#![allow(dead_code)] // wired by the screen in the next Phase A step

pub mod bank;
pub mod daily;
pub mod store;
#[cfg(test)]
mod tests;

use crate::spelldoku::rng::Rng;
pub use bank::Bank;

pub const SPRINT_GOAL: u32 = 50;
pub const MARATHON_GOAL: u32 = 100;
/// D19: a standard-mode spelling times out after this long (a miss).
pub const TIMER_SECS: u32 = 30;
/// D7: three correct in a row switch the multiplier on.
pub const STREAK_FOR_MULTIPLIER: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sprint,
    Marathon,
    /// Pass-and-play, Marathon rules, first to 100 after a Bank (F8).
    HeadToHead,
}

impl Mode {
    /// Pot, Bank and Ward rules apply.
    pub fn pot_rules(self) -> bool {
        matches!(self, Mode::Marathon | Mode::HeadToHead)
    }

    pub fn goal(self) -> u32 {
        match self {
            Mode::Sprint => SPRINT_GOAL,
            Mode::Marathon | Mode::HeadToHead => MARATHON_GOAL,
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

/// One press of the orb. `face` is what the dice made; `len` is the roll the
/// player SEES after any D4 remap, the length of `word`, and the number that
/// scores (A9: the die never shows a length with no words behind it). A jackpot
/// is a displayed 2 or 12, so a remapped 12 is an ordinary 11.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roll {
    pub face: u8,
    pub len: usize,
    pub word: String,
    /// The tier file the word came from, 0..=3.
    pub tier: usize,
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
    /// Marathon: the pot after this answer.
    pub pot: u32,
    /// Marathon: the Ward absorbed a lost pot.
    pub warded: bool,
    /// Marathon: the pot a miss emptied (0 when warded or when nothing was at risk).
    pub lost: u32,
    /// Marathon: the player now owes a Bank or Roll again.
    pub deciding: bool,
    /// Head-to-Head: the turn went to the other player.
    pub passed: bool,
    /// Head-to-Head: this was a free bonus roll.
    pub bonus_roll: bool,
}

/// What a Bank did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banked {
    pub banked: u32,
    pub score: u32,
    pub finished: bool,
    pub passed: bool,
}

/// The idle player's numbers in Head-to-Head.
#[derive(Clone, Debug, Default)]
struct PlayerState {
    score: u32,
    pot: u32,
    streak: u32,
    rolls: u32,
    correct: u32,
    ward: bool,
    ward_used: bool,
    largest_banked: u32,
    largest_lost: u32,
    longest: Option<String>,
    bonus: bool,
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
    /// Rolls as displayed, index 0 is a roll of 2 (D20).
    pub histogram: [u32; 11],
    pub longest: Option<String>,
    pub ended: Option<End>,
    /// Marathon: points earned and not yet banked.
    pub pot: u32,
    /// Marathon: one Ward per game (D13), true while it is still held.
    pub ward: bool,
    pub ward_used: bool,
    pub largest_banked: u32,
    pub largest_lost: u32,
    /// Marathon: a correct answer is waiting for Bank or Roll again.
    pub deciding: bool,
    /// Head-to-Head: 0 or 1, the player on the move.
    pub turn: usize,
    /// Head-to-Head: who banked past the goal.
    pub winner: Option<usize>,
    /// Head-to-Head: the player on the move holds a bonus roll.
    pub bonus: bool,
    /// The pending roll is that bonus roll.
    bonus_roll: bool,
    other: PlayerState,
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
            pot: 0,
            ward: mode.pot_rules(),
            ward_used: false,
            largest_banked: 0,
            largest_lost: 0,
            deciding: false,
            turn: 0,
            winner: None,
            bonus: false,
            bonus_roll: false,
            other: PlayerState { ward: mode == Mode::HeadToHead, ..PlayerState::default() },
            tier: tier.min(3),
            dice: Rng::new(seed ^ 0xD1CE_D1CE_5EED_0001),
            words: Rng::new(seed ^ 0x0057_0AD5_0000_0002),
            bank,
            used: Vec::new(),
            avoid,
            pending: None,
        }
    }

    /// Test seam only: put the score where a test needs it (A7). Never called
    /// by the game.
    #[cfg(feature = "testseam")]
    pub fn set_score_for_test(&mut self, n: u32) {
        self.score = n;
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
        if self.pending.is_some() || self.ended.is_some() || self.deciding {
            return None;
        }
        let bonus_roll = self.mode == Mode::HeadToHead && self.bonus;
        let face = roll_two_dice(&mut self.dice);
        self.rolls += 1;
        let len = self.bank.cov.length_for(face);
        let word = len.and_then(|l| self.pick(l));
        match (len, word) {
            (Some(len), Some((tier, word))) => {
                self.histogram[len - 2] += 1;
                if bonus_roll {
                    self.bonus = false;
                    self.bonus_roll = true;
                }
                self.used.push(word.clone());
                let r = Roll { face, len, word, tier };
                self.pending = Some(r.clone());
                Some(r)
            }
            _ => {
                // Nothing left to draw: bank whatever the pot holds so no
                // earned point is lost, then end (see the module docs).
                if self.pot > 0 {
                    self.largest_banked = self.largest_banked.max(self.pot);
                    self.score += self.pot;
                    self.pot = 0;
                }
                self.ended = Some(End::Exhausted);
                if self.mode == Mode::HeadToHead {
                    self.winner = match self.score.cmp(&self.other.score) {
                        std::cmp::Ordering::Greater => Some(self.turn),
                        std::cmp::Ordering::Less => Some(1 - self.turn),
                        std::cmp::Ordering::Equal => None,
                    };
                }
                None
            }
        }
    }

    /// F4/F5: score the pending roll. A miss is 0 and resets the streak (D5).
    /// Sprint adds straight to the score; Marathon adds to the pot and then
    /// waits for `bank` or `roll_again`.
    pub fn answer(&mut self, correct: bool) -> Option<Outcome> {
        let r = self.pending.take()?;
        let marathon = self.mode.pot_rules();
        let h2h = self.mode == Mode::HeadToHead;
        let bonus_roll = std::mem::take(&mut self.bonus_roll);
        let mut points = 0;
        let mut mult = false;
        let (mut warded, mut lost) = (false, 0);
        if correct {
            let mut p = base_points(r.len as u8);
            if self.streak >= STREAK_FOR_MULTIPLIER {
                p = multiplied(p);
                mult = true;
            }
            points = p;
            self.streak += 1;
            self.correct += 1;
            if bonus_roll {
                // A bonus roll is banked at once and never put at risk.
                self.score += p;
                self.largest_banked = self.largest_banked.max(p);
            } else if marathon {
                self.pot += p;
                self.deciding = true;
            } else {
                self.score += p; // I5: score only ever rises.
            }
            let longer = self.longest.as_ref().map_or(true, |w| r.len > bank::length_of(w));
            if longer {
                self.longest = Some(r.word.clone());
            }
        } else {
            self.streak = 0;
            if marathon && !bonus_roll && self.pot > 0 {
                if self.ward {
                    // D13: the pot survives, the Ward is spent, the streak still resets.
                    self.ward = false;
                    self.ward_used = true;
                    warded = true;
                } else {
                    lost = self.pot;
                    self.largest_lost = self.largest_lost.max(lost);
                    self.pot = 0;
                }
            }
        }
        // Sprint ends on the score. Marathon ends only on a Bank (D15); a
        // Head-to-Head bonus roll IS a Bank.
        let finished = (!marathon || (bonus_roll && correct)) && self.score >= self.mode.goal();
        if finished {
            self.ended = Some(End::Goal);
            if h2h {
                self.winner = Some(self.turn);
            }
        }
        let mut out = Outcome {
            correct,
            points,
            multiplied: mult,
            streak: self.streak,
            score: self.score,
            finished,
            pot: self.pot,
            warded,
            lost,
            deciding: self.deciding,
            passed: false,
            bonus_roll,
        };
        // Head-to-Head: a real miss hands over the turn and grants the other
        // player a bonus roll (a flag: it does not stack).
        if h2h && !correct && !bonus_roll && !finished {
            self.other.bonus = true;
            self.pass_turn();
            out.passed = true;
        }
        Some(out)
    }

    /// Swap the live player's numbers with the idle player's.
    fn pass_turn(&mut self) {
        let o = &mut self.other;
        std::mem::swap(&mut self.score, &mut o.score);
        std::mem::swap(&mut self.pot, &mut o.pot);
        std::mem::swap(&mut self.streak, &mut o.streak);
        std::mem::swap(&mut self.rolls, &mut o.rolls);
        std::mem::swap(&mut self.correct, &mut o.correct);
        std::mem::swap(&mut self.ward, &mut o.ward);
        std::mem::swap(&mut self.ward_used, &mut o.ward_used);
        std::mem::swap(&mut self.largest_banked, &mut o.largest_banked);
        std::mem::swap(&mut self.largest_lost, &mut o.largest_lost);
        std::mem::swap(&mut self.longest, &mut o.longest);
        std::mem::swap(&mut self.bonus, &mut o.bonus);
        self.turn = 1 - self.turn;
        self.deciding = false;
    }

    /// Head-to-Head: a player's banked score, whoever is on the move.
    pub fn score_of(&self, player: usize) -> u32 {
        if player == self.turn { self.score } else { self.other.score }
    }

    /// Head-to-Head: does this player hold a bonus roll?
    pub fn bonus_of(&self, player: usize) -> bool {
        if player == self.turn { self.bonus } else { self.other.bonus }
    }

    /// Head-to-Head: a player's Ward, whoever is on the move.
    pub fn ward_of(&self, player: usize) -> bool {
        if player == self.turn { self.ward } else { self.other.ward }
    }

    /// F5: move the pot into the score. The only way a Marathon ends (D15).
    pub fn bank(&mut self) -> Option<Banked> {
        if !self.deciding || !self.mode.pot_rules() {
            return None;
        }
        let banked = self.pot;
        self.score += banked;
        self.largest_banked = self.largest_banked.max(banked);
        self.pot = 0;
        self.deciding = false;
        let finished = self.score >= self.mode.goal();
        let score = self.score;
        let mut passed = false;
        if finished {
            self.ended = Some(End::Goal);
            if self.mode == Mode::HeadToHead {
                self.winner = Some(self.turn);
            }
        } else if self.mode == Mode::HeadToHead {
            self.pass_turn();
            passed = true;
        }
        Some(Banked { banked, score, finished, passed })
    }

    /// F5: keep the pot at risk and roll again.
    pub fn roll_again(&mut self) -> bool {
        if !self.deciding {
            return false;
        }
        self.deciding = false;
        true
    }

    /// D19: the 30 s soft timer ran out. A miss.
    pub fn timeout(&mut self) -> Option<Outcome> {
        self.answer(false)
    }

    fn pick(&mut self, len: usize) -> Option<(usize, String)> {
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
        Some(cands.swap_remove(i))
    }
}

/// F1: two dice, 2..=12. Two draws from the stream, in a fixed order.
pub fn roll_two_dice(rng: &mut Rng) -> u8 {
    let a = rng.below(6) as u8 + 1;
    let b = rng.below(6) as u8 + 1;
    a + b
}
