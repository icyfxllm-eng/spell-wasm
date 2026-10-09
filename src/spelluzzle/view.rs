//! F3 and F4: what the board shows, as one pure function.
//!
//! `board_view` depends only on the board and the committed entries. Commit
//! order never matters (I4), and nothing here knows whether an entry was right:
//! a clash is a disagreement between entries, not a verdict (I6).

use std::collections::{BTreeMap, BTreeSet};

use super::types::Board;

/// What the board knows about one rune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneState {
    Unknown,
    Decoded(char),
    Contested,
}

/// Committed entries: slot index to the typed units, one per cell. A slot with
/// any cell empty is a draft and is not passed in at all.
pub type Entries = BTreeMap<usize, Vec<char>>;

/// For each rune: unknown, decoded, or contested.
///
/// A rune is contested when committed entries give it two different units, or
/// when its unit is also given to a different rune. A rune whose cells are
/// contested never shows a letter anywhere, including in uncommitted slots.
pub fn board_view(board: &Board, entries: &Entries) -> Vec<RuneState> {
    let n = board.n_runes();
    let mut units_of: Vec<BTreeSet<char>> = vec![BTreeSet::new(); n];
    let mut runes_of: BTreeMap<char, BTreeSet<u8>> = BTreeMap::new();
    for (&i, typed) in entries {
        let Some(slot) = board.slots.get(i) else { continue };
        if typed.len() != slot.runes.len() {
            continue; // not a commit
        }
        for (&r, &u) in slot.runes.iter().zip(typed) {
            units_of[r as usize].insert(u);
            runes_of.entry(u).or_default().insert(r);
        }
    }
    (0..n)
        .map(|r| {
            let us = &units_of[r];
            match us.len() {
                0 => RuneState::Unknown,
                1 => {
                    let u = *us.iter().next().expect("one unit");
                    if runes_of.get(&u).map_or(0, |s| s.len()) > 1 {
                        RuneState::Contested
                    } else {
                        RuneState::Decoded(u)
                    }
                }
                _ => RuneState::Contested,
            }
        })
        .collect()
}

/// One cell as the screen shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub rune: u8,
    pub state: RuneState,
}

/// The cells of one slot under a view.
pub fn slot_cells(board: &Board, view: &[RuneState], slot: usize) -> Vec<Cell> {
    board.slots[slot].runes.iter().map(|&r| Cell { rune: r, state: view[r as usize] }).collect()
}

/// F7: every slot committed, nothing contested, and every entry equals its
/// answer. Entries are compared as given: the caller has already applied the
/// language's match key (F12).
pub fn is_solved(board: &Board, entries: &Entries) -> bool {
    all_committed(board, entries)
        && !board_view(board, entries).contains(&RuneState::Contested)
        && board.slots.iter().enumerate().all(|(i, s)| entries.get(&i) == Some(&s.answer))
}

pub fn all_committed(board: &Board, entries: &Entries) -> bool {
    board.slots.iter().enumerate().all(|(i, s)| entries.get(&i).is_some_and(|e| e.len() == s.answer.len()))
}

/// F7: the committed slots whose entry differs from the answer. Meaningful only
/// when every slot is committed and nothing is contested.
pub fn wrong_slots(board: &Board, entries: &Entries) -> Vec<usize> {
    board.slots.iter().enumerate().filter(|(i, s)| entries.get(i) != Some(&s.answer)).map(|(i, _)| i).collect()
}
