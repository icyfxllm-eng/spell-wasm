//! CC-SPELLUZZLE v1 Phase A — the rune cipher puzzle core. No UI.
//!
//! Everything here is a pure function of its inputs, so a board is the same on
//! every platform (I7). A board is the tuple (seed, language, tier, bank
//! version, generator version); the only randomness is `spelldoku::rng::Rng`,
//! which is integer arithmetic.
//!
//! The layout mirrors the spec's trust boundaries:
//! - `view` is the display function (I4: pure in board and committed entries).
//! - `gates` is the independent checker for G1-G11, with its own solver for G8.
//! - `gen` builds candidate boards. It never decides a board is fair; it hands
//!   every candidate to `gates::check_board` and serves only what that accepts.

pub mod bank;
pub mod fresh;
pub mod gates;
pub mod gen;
pub mod lex;
pub mod offer;
pub mod pencil;
pub mod play;
pub mod render;
pub mod seeds;
pub mod store;
pub mod types;
pub mod view;

#[cfg(test)]
mod seedgen;
#[cfg(test)]
mod tests;
