//! D18 — the 12×12 Expert seed pack.
//!
//! Finding a 12×12 grid that grades exactly Expert takes about twenty digs, far
//! too long to do while a player waits. The dig never looks at the symbols, so
//! the search runs ahead of time, once for every language: the pack lists the
//! `(seed, attempt)` pairs whose grid grades Expert, and a device rebuilds its
//! board from one pair with a single dig (I5 makes it byte-identical), then
//! binds its own words and fragment. There is no server generation path
//! (census item 12), so the pack ships inside the app, built by
//! `pack_tests::build_pack12` and checked by the gate.

use serde::{Deserialize, Serialize};

use super::gen::{grid_at, Clue, Config, Puzzle, Tier};
use super::geo::S12;

const PACK12: &str = include_str!("../../assets/spelldoku/pack12.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seed: u64,
    pub attempt: u64,
    /// Canonical hash of the dug grid (I6): the same grid, however its words
    /// and fragment are bound, is the same board.
    pub hash: u64,
}

#[derive(Serialize, Deserialize)]
struct PackFile {
    #[serde(rename = "$comment")]
    comment: String,
    entries: Vec<Entry>,
}

pub fn config12() -> Config {
    Config { size: S12, tier: Tier::Expert }
}

pub fn entries() -> &'static [Entry] {
    static CACHE: std::sync::OnceLock<Vec<Entry>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| serde_json::from_str::<PackFile>(PACK12).map(|p| p.entries).unwrap_or_default())
}

/// The dug grid of a pack entry as a clue-only puzzle, for its canonical hash.
pub fn grid_puzzle(seed: u64, fixed: &[u8]) -> Puzzle {
    Puzzle {
        n: 12,
        tier: Tier::Expert,
        clues: fixed.iter().map(|&v| if v > 0 { Clue::Given(v) } else { Clue::Empty }).collect(),
        solution: fixed.to_vec(),
        seed,
    }
}

/// D17 / Done #21: every symbol keeps at least three empty cells, so a board
/// asks for exactly three full spellings of each before its chip unlocks.
pub fn three_empties_each(fixed: &[u8]) -> bool {
    (1..=12u8).all(|v| {
        let placed = fixed.iter().filter(|&&x| x == v).count();
        12 - placed >= 3
    })
}

/// I5 / Done #20: a digest over rebuilt pack grids. The host test pins it and
/// the browser test asks the WebAssembly build for the same value.
pub fn pack_digest() -> u64 {
    let mut bytes = Vec::new();
    for e in entries().iter().take(8) {
        if let Some(f) = grid_at(e.seed, e.attempt, &config12()) {
            bytes.extend(f);
        }
    }
    super::rng::fnv(&bytes)
}

/// Build an entry, or None: the attempt must grade Expert and leave every
/// symbol three empty cells.
pub fn entry_for(seed: u64, attempt: u64) -> Option<Entry> {
    let fixed = grid_at(seed, attempt, &config12())?;
    if !three_empties_each(&fixed) {
        return None;
    }
    Some(Entry { seed, attempt, hash: super::canon::hash(&grid_puzzle(seed, &fixed)) })
}
