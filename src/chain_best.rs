//! The player's best chain: the longest run of consecutive correct words in one
//! solo run, at any difficulty. A max, never reset, stored on this device only.
//!
//! The local board is not a record of every run: an entry exists only when the
//! player saves a name and the run makes the top ten. A dismissed save prompt
//! would lose the run, so this counter is written where the run ends, before
//! that prompt opens. Review runs are not chains. Nothing here touches Daily,
//! the streak, The Climb or shields. Great Wall reads `best()`.

use crate::model::{BoardEntry, LB_KEY};
use crate::storage;

pub const KEY: &str = "spell_best_chain_v1";

/// The chain the Great Wall unlock asks for (CC-THEMES-COSMOS O2: no data
/// exists, so this is a judgement call; retune it after real play).
#[allow(dead_code)] // read by the Great Wall unlock once that spec is built
pub const GREAT_WALL_CHAIN: u32 = 25;

/// The new best after a run of `reached`. Review runs and zero never count.
pub fn fold(prev: u32, reached: u32, review: bool) -> u32 {
    if review { prev } else { prev.max(reached) }
}

/// The best chain among saved board entries (the one-time seed). The board
/// holds `review` entries too; they are not chains.
pub fn seed(board: &[BoardEntry]) -> u32 {
    board.iter().filter(|e| e.level != "review").map(|e| e.streak).max().unwrap_or(0)
}

/// The best chain so far. First read on a device seeds from the existing board
/// so current players keep what they have.
pub fn best() -> u32 {
    if let Some(v) = storage::get_raw(KEY).and_then(|s| s.parse::<u32>().ok()) {
        return v;
    }
    let board: Vec<BoardEntry> = storage::get_json(LB_KEY).unwrap_or_default();
    let v = seed(&board);
    storage::set_raw(KEY, &v.to_string());
    v
}

/// A solo run just ended with `reached` correct words in a row.
pub fn note_run(reached: u32, review: bool) {
    let prev = best();
    let next = fold(prev, reached, review);
    if next != prev {
        storage::set_raw(KEY, &next.to_string());
    }
}

/// Whether the Great Wall chain has been reached.
#[allow(dead_code)]
pub fn great_wall_earned() -> bool {
    best() >= GREAT_WALL_CHAIN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: &str, streak: u32) -> BoardEntry {
        serde_json::from_value(serde_json::json!({
            "name": "a", "streak": streak, "lang": "en", "level": level, "timed": false, "ts": 0.0
        }))
        .unwrap()
    }

    #[test]
    fn a_run_raises_the_best_and_never_lowers_it() {
        assert_eq!(fold(0, 12, false), 12);
        assert_eq!(fold(30, 12, false), 30);
        assert_eq!(fold(30, 0, false), 30);
    }

    #[test]
    fn review_runs_are_not_chains() {
        assert_eq!(fold(5, 40, true), 5);
    }

    #[test]
    fn the_seed_is_the_board_max_without_review_entries() {
        let board = vec![entry("easy", 9), entry("review", 99), entry("hard", 14)];
        assert_eq!(seed(&board), 14);
        assert_eq!(seed(&[]), 0);
        assert_eq!(seed(&[entry("review", 50)]), 0);
    }

    #[test]
    fn the_great_wall_chain_is_twenty_five() {
        assert_eq!(GREAT_WALL_CHAIN, 25);
        assert!(fold(0, GREAT_WALL_CHAIN, false) >= GREAT_WALL_CHAIN);
        assert!(fold(0, GREAT_WALL_CHAIN - 1, false) < GREAT_WALL_CHAIN);
    }
}
