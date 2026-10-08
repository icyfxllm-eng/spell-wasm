//! The two rulesets, as types (F4: "compile-time variant, not a runtime if").
//!
//! `Jr` places no traps and its implementation never touches the trap table;
//! `new_game` picks one ruleset by `match` ONCE and everything after is
//! monomorphised. A Jr board therefore has no trap code path to take (I2).

use super::{Tier, Trap};
use crate::spelldoku::rng::Rng;

pub trait Ruleset {
    const TILES: usize;
    /// Cells per side of the square whose perimeter is the ring.
    const GRID: u32;
    /// F9 / I8: unique words each pooled tier must supply.
    const MIN_POOL: usize;
    /// Whether this ruleset has a trap table (and so needs a Long Word pool).
    const HAS_TRAPS: bool;
    /// F2: the mode's pool and its weights, in thousandths of a whole.
    const WEIGHTS: &'static [(Tier, u64)];
    /// F4: hidden traps, as a `tiles`-long table. Empty when there are none.
    fn place_traps(rng: &mut Rng) -> Vec<Option<Trap>>;
}

pub struct Standard;
pub struct Jr;

impl Ruleset for Standard {
    const TILES: usize = super::STANDARD_TILES;
    const GRID: u32 = 22;
    const MIN_POOL: usize = super::MIN_POOL_STANDARD;
    const HAS_TRAPS: bool = true;
    // Medium 40 / Hard 40 / Expert 20.
    const WEIGHTS: &'static [(Tier, u64)] = &[(Tier::Medium, 400), (Tier::Hard, 400), (Tier::Expert, 200)];

    fn place_traps(rng: &mut Rng) -> Vec<Option<Trap>> {
        const COUNT: usize = 6;
        // index not in [0,6] and not in [77,83]; any two more than 3 apart (I2).
        const LO: u64 = 7;
        let span = Self::TILES as u64 - 14;
        let mut placed: Vec<u64> = Vec::new();
        while placed.len() < COUNT {
            let t = LO + rng.next_u64() % span;
            if placed.iter().all(|&p| t.abs_diff(p) > 3) {
                placed.push(t);
            }
        }
        let mut table = vec![None; Self::TILES];
        for t in placed {
            // O1: uniform over the five kinds, with replacement.
            table[t as usize] = Some(Trap::ALL[(rng.next_u64() % Trap::ALL.len() as u64) as usize]);
        }
        table
    }
}

impl Ruleset for Jr {
    const TILES: usize = super::JR_TILES;
    const GRID: u32 = 11;
    const MIN_POOL: usize = super::MIN_POOL_JR;
    const HAS_TRAPS: bool = false;
    // Easy 50 / Medium 50.
    const WEIGHTS: &'static [(Tier, u64)] = &[(Tier::Easy, 500), (Tier::Medium, 500)];

    fn place_traps(_rng: &mut Rng) -> Vec<Option<super::Trap>> {
        Vec::new()
    }
}
