//! The variants, as ONE config block (CC-BOARD-GAME-POLISH I-P10, Phase B).
//!
//! Every number that differs between Full (84 tiles), Sprint (42) and Spell Jr
//! (40) is a field of `VariantCfg` below, and nothing else in the engine, the
//! pool builder or the screen matches on `Variant` to pick one. Integers only
//! (the engine's source scan bans floats).
//!
//! A variant with `trap_count == 0` (Jr) never reaches the trap table: its board
//! has an empty `traps` vector, so there is nothing to index (I2).

use super::{Boost, Difficulty, Tier, Trap};
use crate::spelldoku::rng::Rng;

/// D25: the Long Word band's floor.
pub const LONG_WORD_FLOOR: usize = 20;

/// D3 / D-P4: Full is the 84-tile game (v1 "Standard"), Sprint the 42-tile solo
/// default, Jr the 40-tile no-trap variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    Full,
    Sprint,
    Jr,
}

pub struct VariantCfg {
    /// Ring length; the finish is tile `tiles - 1`.
    pub tiles: usize,
    /// The canonical ring's width (Full 22x22, Sprint 11x12, Jr 11x11). Presentation
    /// chooses the drawn shape from `boardgame_ring`; the engine only digests this.
    pub grid: u32,
    /// F2: the mode's tier pool and its weights, in thousandths of a whole. Ascending
    /// tier order: the lowest is the variant's one-pip tier.
    pub weights: &'static [(Tier, u64)],
    /// F9 / I8: unique words each pooled tier must supply. Full 150; Sprint is exactly
    /// half (42 tiles against 84, so a game draws about half the words); Jr 60.
    pub min_pool: usize,
    /// F4: hidden traps. 0 means the variant has no trap table and no Long Word pool.
    pub trap_count: usize,
    /// No trap in the first or the last `trap_margin` tiles (Full: [0,6] and [77,83]).
    pub trap_margin: u64,
    /// Any two traps are more than this many tiles apart (I2).
    pub trap_gap: u64,
    /// D18: NPC accuracy trails the human's by this many thousandths, indexed by
    /// `Difficulty` (Easy, Normal, Tough). Jr has one fixed value in all three.
    pub delta_milli: [i64; 3],
    /// D27 / D-P20: the NPC formula's starting counts, in thousandths of a spelling. The human's
    /// running accuracy is `(prior_hits + hits) / (prior_attempts + attempts)`. Full and Jr keep D27's 3 / 2;
    /// Sprint, whose games end before that prior washes out, has its own.
    pub prior_hits_milli: i64,
    pub prior_attempts_milli: i64,
    /// Feature 1 (O-P2): whether this variant can offer Stretch at all. Off for Spell Jr.
    pub stretch: bool,
    /// D-P1: extra tiles a correct Stretch spelling adds.
    pub stretch_bonus: u32,
    /// D-P21: an NPC's chance to take Stretch is `base + slope * (acc - 500) / 1000` thousandths,
    /// clamped to 0..=1000, where `acc` is its current spelling accuracy in thousandths.
    pub npc_stretch_rate_base_milli: i64,
    pub npc_stretch_rate_slope_milli: i64,
    /// D-P10: how far an NPC's accuracy drops on a Stretch word, in thousandths. D-P21: an NPC
    /// whose `acc - penalty` would fall below the accuracy floor does not take Stretch at all.
    pub npc_stretch_penalty_milli: i64,
    /// D-P2 as amended by D-P23/D-P24 (Oct 9 2026): boost tiles on the board. Full 3 (one of each
    /// kind; was 4), Sprint 2 of different kinds, Jr 2 from Tailwind and Extra Roll.
    pub boost_count: usize,
    /// The kinds this variant places. `min(boost_count, len)` distinct kinds come first, the rest
    /// are drawn at random from the same list.
    pub boost_pool: &'static [Boost],
    /// D-P2: tiles a Tailwind adds.
    pub tailwind_tiles: u32,
    /// D-P3 as amended by D-P23/D-P24: correct spellings in a row that earn the bonus (5, every
    /// variant; was 3), and the bonus itself.
    pub streak_length: u32,
    pub streak_bonus: u32,
    /// D16: a soft 30 s ring on the screen while spelling (never an auto-fail).
    pub timed: bool,
}

const FULL: VariantCfg = VariantCfg {
    tiles: 84,
    grid: 22,
    // Medium 40 / Hard 40 / Expert 20.
    weights: &[(Tier::Medium, 400), (Tier::Hard, 400), (Tier::Expert, 200)],
    min_pool: 150,
    trap_count: 6,
    trap_margin: 7,
    trap_gap: 3,
    delta_milli: [300, 200, 100],
    prior_hits_milli: 3000,
    prior_attempts_milli: 2000,
    stretch: true,
    stretch_bonus: 2,
    npc_stretch_rate_base_milli: 75,
    npc_stretch_rate_slope_milli: 1200,
    npc_stretch_penalty_milli: 250,
    boost_count: 3,
    boost_pool: &[Boost::Tailwind, Boost::ExtraRoll, Boost::Ward],
    tailwind_tiles: 3,
    streak_length: 5,
    streak_bonus: 1,
    timed: true,
};

/// Sprint: Full's tiers, ladder and trap rules on half the ring with half the traps
/// (D-P12; its 2 boosts arrive with the boost feature in Phase D). D-P20: its NPC
/// formula starts from the same 1.5 as Full but weighs it as half an attempt (0.75 / 0.5
/// instead of 3 / 2), so a 12-round game is not spent waiting for the prior to wash out.
const SPRINT: VariantCfg = VariantCfg {
    tiles: 42,
    grid: 11,
    weights: &[(Tier::Medium, 400), (Tier::Hard, 400), (Tier::Expert, 200)],
    min_pool: 75,
    trap_count: 3,
    trap_margin: 7,
    trap_gap: 3,
    delta_milli: [300, 200, 100],
    prior_hits_milli: 750,
    prior_attempts_milli: 500,
    stretch: true,
    stretch_bonus: 2,
    npc_stretch_rate_base_milli: 75,
    npc_stretch_rate_slope_milli: 1200,
    npc_stretch_penalty_milli: 250,
    boost_count: 2,
    boost_pool: &[Boost::Tailwind, Boost::ExtraRoll, Boost::Ward],
    tailwind_tiles: 3,
    streak_length: 5,
    streak_bonus: 1,
    timed: true,
};

const JR: VariantCfg = VariantCfg {
    tiles: 40,
    grid: 11,
    // Easy 50 / Medium 50.
    weights: &[(Tier::Easy, 500), (Tier::Medium, 500)],
    min_pool: 60,
    trap_count: 0,
    trap_margin: 0,
    trap_gap: 0,
    delta_milli: [250, 250, 250],
    prior_hits_milli: 3000,
    prior_attempts_milli: 2000,
    stretch: false,
    stretch_bonus: 0,
    npc_stretch_rate_base_milli: 0,
    npc_stretch_rate_slope_milli: 0,
    npc_stretch_penalty_milli: 0,
    boost_count: 2,
    boost_pool: &[Boost::Tailwind, Boost::ExtraRoll],
    tailwind_tiles: 3,
    streak_length: 5,
    streak_bonus: 1,
    timed: false,
};

impl Variant {
    pub const ALL: [Variant; 3] = [Variant::Full, Variant::Sprint, Variant::Jr];

    pub fn cfg(self) -> &'static VariantCfg {
        match self {
            Variant::Full => &FULL,
            Variant::Sprint => &SPRINT,
            Variant::Jr => &JR,
        }
    }
}

impl VariantCfg {
    pub fn has_traps(&self) -> bool {
        self.trap_count > 0
    }

    /// The tiers this variant draws tile words from, lowest first.
    pub fn tiers(&self) -> impl Iterator<Item = Tier> + '_ {
        self.weights.iter().map(|w| w.0)
    }

    /// The starting counts, with a test-only override so the search can sweep them.
    pub fn prior(&self) -> (i64, i64) {
        #[cfg(test)]
        if let Some(p) = PRIOR_OVERRIDE.with(|c| c.get()) {
            return p;
        }
        (self.prior_hits_milli, self.prior_attempts_milli)
    }

    /// The tier one above `t` IN THIS VARIANT'S POOL (not `Tier::ALL`), if any.
    pub fn harder(&self, t: Tier) -> Option<Tier> {
        let mut ts: Vec<Tier> = self.tiers().collect();
        ts.sort();
        ts.iter().position(|&x| x == t).and_then(|i| ts.get(i + 1).copied())
    }

    /// (rate base, rate slope, penalty) in thousandths, with a test-only override for the A-P14 search.
    pub fn npc_stretch(&self) -> (i64, i64, i64) {
        #[cfg(test)]
        if let Some(p) = STRETCH_OVERRIDE.with(|c| c.get()) {
            return p;
        }
        (self.npc_stretch_rate_base_milli, self.npc_stretch_rate_slope_milli, self.npc_stretch_penalty_milli)
    }

    /// F2: the hidden boosts, as a `tiles`-long table (empty when the game has none). Called only
    /// when boosts are on, after the traps, so a game without them draws nothing here. The first
    /// `min(count, pool)` boosts are distinct kinds in random order, the rest random; each goes on a
    /// free tile that is not tile 0, the finish or a trap tile (I-P5).
    pub fn place_boosts(&self, rng: &mut Rng, traps: &[Option<Trap>]) -> Vec<Option<Boost>> {
        let mut kinds: Vec<Boost> = self.boost_pool.to_vec();
        let distinct = self.boost_count.min(kinds.len());
        let mut picked: Vec<Boost> = Vec::new();
        for _ in 0..distinct {
            let k = rng.below(kinds.len());
            picked.push(kinds.remove(k));
        }
        while picked.len() < self.boost_count {
            picked.push(self.boost_pool[rng.below(self.boost_pool.len())]);
        }
        let mut table = vec![None; self.tiles];
        for b in picked {
            loop {
                let t = 1 + (rng.next_u64() % (self.tiles as u64 - 2)) as usize;
                let trap = traps.get(t).copied().flatten().is_some();
                if table[t].is_none() && !trap {
                    table[t] = Some(b);
                    break;
                }
            }
        }
        table
    }

    pub fn delta(&self, d: Difficulty) -> i64 {
        self.delta_milli[d.ix()]
    }

    /// F4: hidden traps, as a `tiles`-long table. Empty when there are none.
    /// The window is `[trap_margin, tiles - trap_margin)`; draws and rejection are
    /// the same stream the 84-tile game always used.
    pub fn place_traps(&self, rng: &mut Rng) -> Vec<Option<Trap>> {
        if !self.has_traps() {
            return Vec::new();
        }
        let span = self.tiles as u64 - 2 * self.trap_margin;
        let mut placed: Vec<u64> = Vec::new();
        while placed.len() < self.trap_count {
            let t = self.trap_margin + rng.next_u64() % span;
            if placed.iter().all(|&p| t.abs_diff(p) > self.trap_gap) {
                placed.push(t);
            }
        }
        let mut table = vec![None; self.tiles];
        for t in placed {
            // O1: uniform over the five kinds, with replacement.
            table[t as usize] = Some(Trap::ALL[(rng.next_u64() % Trap::ALL.len() as u64) as usize]);
        }
        table
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static STRETCH_OVERRIDE: std::cell::Cell<Option<(i64, i64, i64)>> = const { std::cell::Cell::new(None) };
    pub(crate) static PRIOR_OVERRIDE: std::cell::Cell<Option<(i64, i64)>> = const { std::cell::Cell::new(None) };
}
