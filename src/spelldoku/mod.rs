//! CC-SPELLDOKU v1 — Sudoku where every cell is committed by spelling a number
//! word. App only (D9): this module never compiles into the site build.
//!
//! Everything here is a pure function of its inputs so it can be tested on the
//! host and produces the same board on every platform (I5): a board is a
//! function of `(seed, config)`, and the only randomness is `rng::Rng`, which is
//! integer arithmetic with no platform dependence.

pub mod canon;
pub mod geo;
pub mod gen;
pub mod play;
pub mod rng;
pub mod solve;
pub mod table;

#[cfg(test)]
mod tests;
