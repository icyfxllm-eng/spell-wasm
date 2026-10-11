//! F19: Expeditions. A run is three boards that share one cipher: one unit is one rune on all of
//! them, and what you decode on a board you solved shows decoded when the next one loads.
//!
//! The device never searches for a run. The build step generates and proves runs offline
//! (`rungen`, against the large word list) and ships each as a seed and sixteen indexes into the
//! tier's pool; this module builds the three boards from such a record.

use crate::spelldoku::rng::{fnv, mix, Rng};

use super::lex::{Lexicon, Word};
use super::types::{Board, RunInfo, Slot, SlotKind, Tier, GEN_VERSION};

/// Words per run: 5 + 6 + 5.
pub const RUN_WORDS: usize = 16;
/// Words on each board.
pub const BOARD_SIZES: [usize; 3] = [5, 6, 5];

pub fn offered_tier(tier: Tier) -> bool {
    matches!(tier, Tier::Medium | Tier::Hard | Tier::Expert)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRecord {
    pub seed: u64,
    pub words: [u16; RUN_WORDS],
}

/// The three boards a record names, or None if the pool no longer has those indexes.
pub fn boards_from_record(lex: &Lexicon, tier: Tier, rec: &RunRecord) -> Option<Vec<Board>> {
    let pool: Vec<&Word> = lex.pool(tier);
    let words: Vec<&Vec<char>> = rec.words.iter().map(|&i| pool.get(i as usize).map(|w| &w.units)).collect::<Option<_>>()?;
    // One order of the run's units, for glyphs.
    let mut units: Vec<char> = words.iter().flat_map(|w| w.iter().copied()).collect();
    units.sort_unstable();
    units.dedup();
    let mut boards = Vec::new();
    let mut carried: Vec<char> = Vec::new();
    let mut at = 0;
    for (k, &n) in BOARD_SIZES.iter().enumerate() {
        let ws = &words[at..at + n];
        at += n;
        // Kinds: board 1 is 4 spoken + secret, board 2 is 3 spoken + 2 silent + secret, the Vault is
        // 4 silent + secret.
        let spoken = [4usize, 3, 0][k];
        let mut bu: Vec<char> = ws.iter().flat_map(|w| w.iter().copied()).collect();
        bu.sort_unstable();
        bu.dedup();
        // This board's own rune numbering: a seeded permutation, never alphabet order.
        Rng::new(mix(rec.seed ^ fnv(format!("rune:{k}").as_bytes()), 0x52_55_4e_45)).shuffle(&mut bu);
        let rune_of = |c: char| bu.iter().position(|&u| u == c).expect("unit has a rune") as u8;
        let slots: Vec<Slot> = ws
            .iter()
            .enumerate()
            .map(|(i, w)| Slot {
                kind: if i + 1 == n {
                    SlotKind::Secret
                } else if i < spoken {
                    SlotKind::Spoken
                } else {
                    SlotKind::Silent
                },
                answer: (*w).clone(),
                runes: w.iter().map(|&c| rune_of(c)).collect(),
            })
            .collect();
        boards.push(Board {
            lang: lex.lang.clone(),
            tier,
            seed: rec.seed,
            gen_version: GEN_VERSION,
            slots,
            rune_unit: bu.clone(),
            par: None,
            list: None,
            run: Some(RunInfo { seed: rec.seed, board: k as u8, units: units.clone(), carried: carried.clone() }),
        });
        for u in bu {
            if !carried.contains(&u) {
                carried.push(u);
            }
        }
    }
    Some(boards)
}

/// A run's identity for the no-repeat history: the hash of its three boards.
pub fn run_hash(boards: &[Board]) -> u64 {
    fnv(boards.iter().map(|b| b.hash().to_string()).collect::<Vec<_>>().join(",").as_bytes())
}
