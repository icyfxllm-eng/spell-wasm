//! Playing a board: verdicts (I4), unlock chips (F1/D1), error timing (D2),
//! Jr gating (F7), the Daily seed (F8) and the repeat window (I6).

use super::gen::Tier;
use super::rng::fnv;
use super::table::Table;

/// I4: three separate outcomes. A spelling error never counts as a logic error
/// or the reverse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Correct,
    /// Not a number word this table accepts.
    Misspelled,
    /// A correctly spelled number word, but not this cell's value.
    WrongValue,
    /// Phase B (F5): right value, wrong counting system.
    WrongSystem,
}

pub fn judge(table: &Table, typed: &str, expected: u8) -> Verdict {
    verdict(&table.values_for(typed), expected)
}

/// I4 on any board: `values` are the symbols whose spelling matches what was
/// typed (numbers in Number Mode, this board's words in Word Mode).
pub fn verdict(values: &[u8], expected: u8) -> Verdict {
    if values.contains(&expected) {
        Verdict::Correct
    } else if values.is_empty() {
        Verdict::Misspelled
    } else {
        Verdict::WrongValue
    }
}

/// D1: correct full spellings before a value becomes a tap chip. Never on Expert.
pub fn unlock_after(tier: Tier) -> Option<u32> {
    match tier {
        Tier::Easy => Some(1),
        Tier::Medium => Some(2),
        Tier::Hard => Some(3),
        Tier::Expert => None,
    }
}

/// D1 with v1.2 D17: a 12×12 Expert board unlocks at the Hard threshold, 3.
/// This exception is scoped to that size; a 9×9 Expert board always spells.
pub fn unlock_after_on(n: usize, tier: Tier) -> Option<u32> {
    if n == 12 && tier == Tier::Expert {
        Some(3)
    } else {
        unlock_after(tier)
    }
}

/// Per board, per value: how many CELLS have been spelled correctly for it.
///
/// Cells, not attempts. Until 2026-09-20 this counted every correct spelling,
/// and nothing stops a player re-selecting a cell that is already right and
/// spelling it again — so one easy word, spelled three times, unlocked its chip
/// on Hard (found by the CC-SPELLDOKU v1.3 census, C5). A (cell, value) is
/// credited once; a different value in the same cell, or the same value in
/// another cell, still counts. The set lives and dies with the board.
#[derive(Clone, Debug)]
pub struct Unlocks {
    need: Option<u32>,
    counts: [u32; 16],
    credited: Vec<(usize, u8)>,
}

impl Unlocks {
    pub fn new(tier: Tier) -> Self {
        Unlocks { need: unlock_after(tier), counts: [0; 16], credited: Vec::new() }
    }

    pub fn for_board(n: usize, tier: Tier) -> Self {
        Unlocks { need: unlock_after_on(n, tier), counts: [0; 16], credited: Vec::new() }
    }

    /// Credit one correctly spelled CELL toward `v`'s chip. Re-spelling a cell
    /// that already earned its credit does nothing.
    pub fn record_correct(&mut self, cell: usize, v: u8) {
        if (v as usize) >= self.counts.len() || self.credited.contains(&(cell, v)) {
            return;
        }
        self.credited.push((cell, v));
        self.counts[v as usize] += 1;
    }

    pub fn chip(&self, v: u8) -> bool {
        self.need.is_some_and(|n| (v as usize) < self.counts.len() && self.counts[v as usize] >= n)
    }
}

/// D2: logic errors show at once on Easy and Medium; on Hard and Expert only
/// through Check Board.
pub fn logic_errors_shown_at_once(tier: Tier) -> bool {
    matches!(tier, Tier::Easy | Tier::Medium)
}

/// D2: Check Board uses per board, Hard and Expert.
pub const CHECK_BOARD_USES: u32 = 3;

/// I6: the repeat window, one config value.
pub const REPEAT_WINDOW_DAYS: u32 = 365;

/// D11 (OPEN, default applied): Spell Jr players also need tone marks. One
/// value, so it can be reversed without code changes.
pub const JR_TONE_MARKS_REQUIRED: bool = true;

/// F8: the Daily seed -- the same date and language give everyone the same board.
pub fn daily_seed(ymd: u32, lang: &str) -> u64 {
    fnv(format!("spelldoku-daily|{ymd}|{lang}").as_bytes())
}

/// F7: the configurations a player may be served.
pub fn allowed(kid: bool, n: usize, tier: Tier) -> bool {
    if kid {
        super::gen::jr_allowed(n, tier)
    } else {
        super::gen::standard_allowed(n, tier)
    }
}
