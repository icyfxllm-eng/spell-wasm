//! F16: pencil marks, and F17's strip, F18's ripple (the three Phase E layers).
//!
//! All three are drawn OVER `view::board_view`, which they never change (I14):
//! a mark is a note on a rune, not an entry, so it cannot commit, decode, clash,
//! enter the full-board check, or touch stars, the Listen count or par.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::view::RuneState;

/// One mark per rune, on any rune (marks on decoded or contested runes are kept
/// and simply not drawn: D35).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pencil {
    marks: BTreeMap<u8, char>,
}

impl Pencil {
    pub fn set(&mut self, rune: u8, unit: char) {
        self.marks.insert(rune, unit);
    }
    pub fn clear(&mut self, rune: u8) {
        self.marks.remove(&rune);
    }
    pub fn clear_all(&mut self) {
        self.marks.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
    pub fn get(&self, rune: u8) -> Option<char> {
        self.marks.get(&rune).copied()
    }
    pub fn all(&self) -> Vec<(u8, char)> {
        self.marks.iter().map(|(r, c)| (*r, *c)).collect()
    }
    pub fn from_all(v: &[(u8, char)]) -> Pencil {
        Pencil { marks: v.iter().copied().collect() }
    }
    /// The marks to draw: only runes that are undecoded. A mark on a rune that is
    /// decoded or contested is hidden, not deleted; it comes back if the rune
    /// becomes undecoded again.
    pub fn visible(&self, view: &[RuneState]) -> BTreeMap<u8, char> {
        self.marks.iter().filter(|(r, _)| view.get(**r as usize) == Some(&RuneState::Unknown)).map(|(r, c)| (*r, *c)).collect()
    }
}

/// One entry of the rune strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StripEntry {
    pub rune: u8,
    pub cells: usize,
    pub state: RuneState,
    pub mark: Option<char>,
}

/// F17 / D36: runes by cell count, most first, ties by first appearance in
/// reading order. The order is a function of the board's rune pattern only, so it
/// never changes during play. Takes the slots' rune arrays, not the board: no
/// answer is reachable from here (I15).
pub fn strip(slots: &[&[u8]], view: &[RuneState], marks: &BTreeMap<u8, char>) -> Vec<StripEntry> {
    let n = view.len();
    let mut cells = vec![0usize; n];
    let mut first = vec![usize::MAX; n];
    let mut idx = 0;
    for s in slots {
        for &r in *s {
            let r = r as usize;
            if r < n {
                cells[r] += 1;
                if first[r] == usize::MAX {
                    first[r] = idx;
                }
            }
            idx += 1;
        }
    }
    let mut order: Vec<usize> = (0..n).filter(|&r| cells[r] > 0).collect();
    order.sort_by_key(|&r| (std::cmp::Reverse(cells[r]), first[r]));
    order.into_iter().map(|r| StripEntry { rune: r as u8, cells: cells[r], state: view[r], mark: marks.get(&(r as u8)).copied() }).collect()
}

/// The runes in strip order, for the ripple.
pub fn strip_order(slots: &[&[u8]], n_runes: usize) -> Vec<u8> {
    strip(slots, &vec![RuneState::Unknown; n_runes], &BTreeMap::new()).into_iter().map(|e| e.rune).collect()
}

/// F18: the ripple is a function of two views and nothing else (I16). It cannot
/// tell a right commit from a wrong one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ripple {
    /// Runes that went from not-decoded to decoded, in strip order.
    pub runes: Vec<u8>,
    /// Cells OUTSIDE the committed slot that newly show a letter.
    pub cells: usize,
}

pub fn ripple(slots: &[&[u8]], order: &[u8], before: &[RuneState], after: &[RuneState], committed: usize) -> Ripple {
    let newly = |r: u8| {
        let (b, a) = (before.get(r as usize), after.get(r as usize));
        matches!(a, Some(RuneState::Decoded(_))) && !matches!(b, Some(RuneState::Decoded(_)))
    };
    let runes: Vec<u8> = order.iter().copied().filter(|&r| newly(r)).collect();
    let cells = slots.iter().enumerate().filter(|(i, _)| *i != committed).map(|(_, s)| s.iter().filter(|&&r| newly(r)).count()).sum();
    Ripple { runes, cells }
}

/// The whole ripple lasts at most this long, however many runes (F18).
pub const MAX_MS: u32 = 1200;
/// Reduce Motion: one short cross-fade.
pub const REDUCED_MS: u32 = 200;
/// The longest a single rune is held.
const STEP_CAP_MS: u32 = 120;

/// Milliseconds between runes. `n * step_ms(n)` never exceeds `MAX_MS`.
pub fn step_ms(n_runes: usize) -> u32 {
    if n_runes == 0 {
        0
    } else {
        (MAX_MS / n_runes as u32).min(STEP_CAP_MS)
    }
}
