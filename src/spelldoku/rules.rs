//! CC-SPELLDOKU-RULES v1 — the one place a placement is judged legal (F1, F2).
//!
//! Symbol-blind (I-R7): this sees value ids, never words, so it compiles and
//! tests without a language. It reads only the VISIBLE board — givens and the
//! player's own entries, which the screen keeps in one array — and never the
//! solution (I-R5). Check board and hints are the only things allowed to look
//! at the answer.
//!
//! D-R1 (Eric, 2026-09-22) amends v1.0 D2: a duplicate in a row, column or box
//! is refused at every tier, immediately. D2's surviving half is unchanged —
//! an entry that breaks no rule but is not the solution still waits for Check
//! board on Hard and Expert.

use super::geo::Geo;

/// Which unit holds the duplicate. Reported row before column before box,
/// the order F4's message names them in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Row,
    Column,
    Box,
}

impl Unit {
    /// The i18n key fragment for F4's "{symbol} is already in this …".
    pub fn key(self) -> &'static str {
        match self {
            Unit::Row => "sd.conflictRow",
            Unit::Column => "sd.conflictCol",
            Unit::Box => "sd.conflictBox",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The first unit, in row → column → box order, that already holds it.
    pub unit: Unit,
    /// Every cell holding the value in ANY of the three units, so the screen
    /// can flash all of them, sorted and without repeats.
    pub cells: Vec<usize>,
}

/// May `value` go into `cell`? `entries` is the visible board: a given or a
/// committed entry is a positive value, an empty cell is 0.
///
/// Value 0 means "erase", which never conflicts.
pub fn validate(geo: &Geo, entries: &[u8], cell: usize, value: u8) -> Result<(), Conflict> {
    if value == 0 || cell >= entries.len() {
        return Ok(());
    }
    let mut cells: Vec<usize> = Vec::new();
    let mut first: Option<Unit> = None;
    for (k, unit) in [Unit::Row, Unit::Column, Unit::Box].into_iter().enumerate() {
        let mut hit = false;
        for &j in &geo.units[geo.cell_units[cell][k]] {
            if j != cell && entries.get(j).copied() == Some(value) {
                hit = true;
                if !cells.contains(&j) {
                    cells.push(j);
                }
            }
        }
        if hit && first.is_none() {
            first = Some(unit);
        }
    }
    match first {
        None => Ok(()),
        Some(unit) => {
            cells.sort_unstable();
            Err(Conflict { unit, cells })
        }
    }
}

/// F2: how many of `value` are still to be placed. Derived from the board every
/// time it is asked for, never stored (I-R4). Never negative.
pub fn remaining(n: usize, entries: &[u8], value: u8) -> usize {
    n.saturating_sub(entries.iter().filter(|&&v| v == value).count())
}

/// F3: the values already ruled out for `cell` by its row, column and box.
/// A bit per value, `1 << v`. Reads only the visible board (I-R5).
pub fn ruled_out(geo: &Geo, entries: &[u8], cell: usize) -> u16 {
    let mut mask = 0u16;
    if cell >= entries.len() {
        return mask;
    }
    for &j in &geo.peers[cell] {
        if let Some(&v) = entries.get(j) {
            if v > 0 {
                mask |= super::geo::bit(v);
            }
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spelldoku::geo::{size_of, Geo};

    fn geo9() -> Geo {
        Geo::new(size_of(9).unwrap())
    }

    #[test]
    fn a_duplicate_is_refused_in_each_unit_and_named_in_order() {
        let g = geo9();
        let mut e = vec![0u8; 81];
        e[0] = 3; // r1c1
        // Same row, and the same box: the row is named first.
        let c = validate(&g, &e, 2, 3).unwrap_err();
        assert_eq!(c.unit, Unit::Row);
        assert_eq!(c.cells, vec![0]);
        // Same column only.
        let c = validate(&g, &e, 36, 3).unwrap_err();
        assert_eq!(c.unit, Unit::Column);
        // Same box only: r2c2 shares neither row nor column with r1c1.
        let c = validate(&g, &e, 10, 3).unwrap_err();
        assert_eq!(c.unit, Unit::Box);
        // A cell that shares nothing is free.
        assert!(validate(&g, &e, 40, 3).is_ok());
        // Another value in the same row is free.
        assert!(validate(&g, &e, 2, 4).is_ok());
    }

    #[test]
    fn every_conflicting_cell_is_reported_not_just_the_first() {
        let g = geo9();
        let mut e = vec![0u8; 81];
        e[1] = 7; // same row
        e[9] = 7; // same column
        e[11] = 7; // same box
        let c = validate(&g, &e, 0, 7).unwrap_err();
        assert_eq!(c.unit, Unit::Row, "row is named first");
        assert_eq!(c.cells, vec![1, 9, 11], "all three flash");
    }

    #[test]
    fn erasing_never_conflicts() {
        let g = geo9();
        let mut e = vec![0u8; 81];
        e[0] = 5;
        assert!(validate(&g, &e, 1, 0).is_ok());
    }

    #[test]
    fn remaining_counts_down_and_floors_at_zero() {
        let mut e = vec![0u8; 81];
        assert_eq!(remaining(9, &e, 2), 9);
        for i in 0..7 {
            e[i * 9] = 2;
        }
        assert_eq!(remaining(9, &e, 2), 2);
        for i in 7..9 {
            e[i * 9] = 2;
        }
        assert_eq!(remaining(9, &e, 2), 0);
        // A board that somehow holds more than n never reports a negative.
        e[1] = 2;
        assert_eq!(remaining(9, &e, 2), 0);
    }

    #[test]
    fn ruled_out_names_exactly_the_peers_values() {
        let g = geo9();
        let mut e = vec![0u8; 81];
        e[1] = 4; // row peer
        e[9] = 6; // column peer
        e[10] = 8; // box peer
        e[80] = 9; // shares nothing with cell 0
        let m = ruled_out(&g, &e, 0);
        for v in [4u8, 6, 8] {
            assert!(m & crate::spelldoku::geo::bit(v) != 0, "{v} is ruled out");
        }
        assert!(m & crate::spelldoku::geo::bit(9) == 0, "a far cell rules nothing out");
    }

    /// I-R5: the validator is handed the visible board only. A generated board's
    /// solution never reaches it, and this test is the reminder — `validate`
    /// takes no solution argument at all, so a future change cannot sneak one in
    /// without editing this signature.
    #[test]
    fn a_full_valid_board_accepts_nothing_new_and_no_valid_board_conflicts() {
        let g = geo9();
        // A trivially valid Latin-square-with-boxes board.
        let mut e = vec![0u8; 81];
        for r in 0..9 {
            for c in 0..9 {
                let v = ((r * 3 + r / 3 + c) % 9 + 1) as u8;
                e[r * 9 + c] = v;
            }
        }
        for i in 0..81 {
            let v = e[i];
            e[i] = 0;
            assert!(validate(&g, &e, i, v).is_ok(), "cell {i} should accept its own value back");
            for other in 1..=9u8 {
                if other != v {
                    assert!(validate(&g, &e, i, other).is_err(), "cell {i} must refuse {other}");
                }
            }
            e[i] = v;
        }
    }

    /// Done #2 (`prop_no_conflict_reachable`), adapted: the spec asks for
    /// 4x4, 9x9 and 12x12 across numbers and letters. 12x12 was cut on
    /// 2026-09-21, so the sizes are 4, 6 and 9 — and the symbol kind cannot
    /// matter here, because this module never sees a word (I-R7). What it does
    /// prove is I-R1: a board built only from accepted placements never holds
    /// a conflict, whatever order they arrive in.
    #[test]
    fn no_sequence_of_accepted_placements_can_reach_a_conflict() {
        use crate::spelldoku::rng::Rng;
        let mut rng = Rng::new(0xD0_1234);
        let mut steps = 0u32;
        for round in 0..10_000u32 {
            let n = [4usize, 6, 9][(round % 3) as usize];
            let g = Geo::new(size_of(n).unwrap());
            let mut e = vec![0u8; n * n];
            for _ in 0..(n * 3) {
                let cell = rng.below(n * n);
                let value = (rng.below(n) + 1) as u8;
                if validate(&g, &e, cell, value).is_ok() {
                    e[cell] = value;
                }
                steps += 1;
                // I-R1 after EVERY step, not just at the end.
                for i in 0..e.len() {
                    if e[i] > 0 {
                        let v = e[i];
                        e[i] = 0;
                        let ok = validate(&g, &e, i, v).is_ok();
                        e[i] = v;
                        assert!(ok, "round {round}: cell {i} holds a duplicate {v}");
                    }
                }
            }
        }
        assert!(steps >= 10_000, "the sweep really ran ({steps} placements)");
    }

    /// F2 and F1 agree: when a value has no copies left to place, every cell
    /// that could still take it is already refused by the conflict rule too.
    /// (A full board of one value is impossible, so this is about the count
    /// never contradicting the validator.)
    #[test]
    fn a_spent_value_has_no_legal_home_left() {
        let g = geo9();
        let mut e = vec![0u8; 81];
        for r in 0..9 {
            e[r * 9 + (r * 3 + r / 3) % 9] = 1; // nine 1s, one per row/column/box
        }
        assert_eq!(remaining(9, &e, 1), 0);
        for i in 0..81 {
            if e[i] == 0 {
                assert!(validate(&g, &e, i, 1).is_err(), "cell {i} must refuse a tenth 1");
            }
        }
    }
}
