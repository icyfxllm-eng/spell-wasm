//! CC-WORDGRID v1 Phase A — Spell Search: a word search where every target is
//! heard, never shown. App only, like SpellDoku: this module never compiles
//! into the site build.
//!
//! Everything here is a pure function of its inputs, so it tests on the host and
//! builds the same grid on every platform (I8). The only randomness is
//! `spelldoku::rng::Rng`, which is integer arithmetic.
//!
//! - `lexicon`: which languages play, their word pools, filler letters, audio (I6).
//! - `confusion`: the hand-authored confusion list and the shipped decoys (F-S2, I2).
//! - `gen`: the grid itself (F-S1, F-S3, F-S5; I1, I3, I9).
//! - `ledger`: the no-repeat engine and the Daily schedule (F-X2, F-X3).
//! - `hint`: the give-away filter every hint passes (F-X4, I10).
//! - `serve`: a bank, Daily or My Words puzzle, start to finish (F-X1).

pub mod confusion;
pub mod gen;
pub mod hint;
pub mod ledger;
pub mod lexicon;
pub mod serve;

#[cfg(test)]
mod tests;

use crate::learner::{attempt, Attempt, Channel};

/// F-S4 / F-X6: a Lock It In answer is recorded exactly as the base game
/// records a typed answer.
pub fn lock_in_attempt(lang: &str, word: &str, typed: &str, correct: bool, day: u32) -> Attempt {
    attempt(lang, word, correct, Channel::Typed, Some(typed), day)
}

/// F-S2 / F-X6: TRAP_MISS, a miss on the target with the decoy as what was
/// "typed", in the base game's own log. No parallel stats.
pub fn trap_attempt(lang: &str, target: &str, decoy: &str, day: u32) -> Attempt {
    attempt(lang, target, false, Channel::Trap, Some(decoy), day)
}
