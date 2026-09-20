//! CC-SPELLDOKU v1 — Sudoku where every cell is committed by spelling a number
//! word. App only (D9): this module never compiles into the site build.
//!
//! Everything here is a pure function of its inputs so it can be tested on the
//! host and produces the same board on every platform (I5): a board is a
//! function of `(seed, config)`, and the only randomness is `rng::Rng`, which is
//! integer arithmetic with no platform dependence.

pub mod bind;
pub mod canon;
pub mod geo;
pub mod pack;
pub mod gen;
pub mod play;
pub mod rng;
pub mod solve;
pub mod symbols;
pub mod table;
pub mod tier; // CC-SPELLDOKU v1.3 — Tier Mode ladder (I-T9)
pub mod wordmode;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod wordmode_tests;
#[cfg(test)]
mod pack_tests;
