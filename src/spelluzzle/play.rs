//! One play of one board: what the player has typed, what is committed, and the
//! stars (F2, F5, F6, F7, F8). Pure state, no DOM and no storage, so every rule
//! is testable on the host.
//!
//! Nothing here reveals whether an individual commit was right while the board
//! is open (I6). `stars()` answers only once the board is solved.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Board, SlotKind, Tier};
use super::view::{all_committed, board_view, is_solved, wrong_slots, Entries, RuneState};

/// What a commit did, for the feedback mapping in F4: no clash is `neutral`, a
/// new clash is `close`, a solved board is `success`. Never a miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Neutral,
    Close,
    Success,
}

/// F7's full-board check, shown only when every slot is committed, nothing is
/// contested and at least one entry is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// Medium and above: the count only.
    Count(usize),
    /// Spell Jr and Easy: the wrong slots wobble amber (D12).
    Slots(Vec<usize>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stars {
    pub solved: bool,
    pub sharp_ear: bool,
    pub codebreaker: bool,
}

impl Stars {
    pub fn count(&self) -> u8 {
        self.solved as u8 + self.sharp_ear as u8 + self.codebreaker as u8
    }
}

#[derive(Clone, Debug)]
pub struct Play {
    pub board: Board,
    pub entries: Entries,
    pub drafts: BTreeMap<usize, Vec<char>>,
    pub selected: Option<usize>,
    /// Silent slots the player paid to hear (F5). Each stops being silent.
    pub listened: BTreeSet<usize>,
    first_commit: BTreeMap<usize, Vec<char>>,
    secret_undecoded_at_first: Option<bool>,
}

impl Play {
    pub fn new(board: Board) -> Play {
        Play {
            board,
            entries: Entries::new(),
            drafts: BTreeMap::new(),
            selected: None,
            listened: BTreeSet::new(),
            first_commit: BTreeMap::new(),
            secret_undecoded_at_first: None,
        }
    }

    pub fn is_jr(&self) -> bool {
        self.board.tier == Tier::Jr
    }

    pub fn len_of(&self, slot: usize) -> usize {
        self.board.slots[slot].answer.len()
    }

    pub fn solved(&self) -> bool {
        is_solved(&self.board, &self.entries)
    }

    /// May the orb say this slot's word on a tap? Spoken slots and bought
    /// Listens, and every slot once the board is solved. The secret never before.
    pub fn can_hear(&self, slot: usize) -> bool {
        self.solved() || self.board.slots[slot].kind == SlotKind::Spoken || self.listened.contains(&slot)
    }

    /// F5: a silent slot that has not been bought.
    pub fn is_silent(&self, slot: usize) -> bool {
        self.board.slots[slot].kind == SlotKind::Silent && !self.listened.contains(&slot)
    }

    /// Selecting a slot returns the word to play, if it may be heard.
    pub fn select(&mut self, slot: usize) -> Option<String> {
        if slot >= self.board.slots.len() {
            return None;
        }
        self.selected = Some(slot);
        self.can_hear(slot).then(|| self.word(slot))
    }

    pub fn word(&self, slot: usize) -> String {
        self.board.slots[slot].answer.iter().collect()
    }

    /// F5: buy a Listen. Costs the Codebreaker star; returns the word to play.
    pub fn listen(&mut self, slot: usize) -> Option<String> {
        if slot < self.board.slots.len() && self.board.slots[slot].kind == SlotKind::Silent {
            self.listened.insert(slot);
            return Some(self.word(slot));
        }
        None
    }

    fn contested(&self) -> usize {
        board_view(&self.board, &self.entries).iter().filter(|s| **s == RuneState::Contested).count()
    }

    /// Type one unit into the selected slot. Filling the last cell commits.
    pub fn type_unit(&mut self, unit: char) -> Option<Outcome> {
        let slot = self.selected?;
        let want = self.len_of(slot);
        // Typing into a committed slot reopens it: the old commit stops counting.
        if self.entries.remove(&slot).is_some() {
            self.drafts.remove(&slot);
        }
        let draft = self.drafts.entry(slot).or_default();
        if draft.len() >= want {
            return None;
        }
        draft.push(unit);
        if draft.len() < want {
            return None;
        }
        let typed = self.drafts.remove(&slot).unwrap_or_default();
        Some(self.commit(slot, typed))
    }

    fn commit(&mut self, slot: usize, typed: Vec<char>) -> Outcome {
        let before = self.contested();
        if !self.first_commit.contains_key(&slot) {
            if self.board.slots[slot].kind == SlotKind::Secret {
                let view = board_view(&self.board, &self.entries);
                let undecoded = self.board.slots[slot].runes.iter().any(|&r| view[r as usize] != RuneState::Decoded(self.board.rune_unit[r as usize]));
                self.secret_undecoded_at_first = Some(undecoded);
            }
            self.first_commit.insert(slot, typed.clone());
        }
        self.entries.insert(slot, typed);
        if self.solved() {
            Outcome::Success
        } else if self.contested() > before {
            Outcome::Close
        } else {
            Outcome::Neutral
        }
    }

    /// Delete the last unit of the selected slot, reopening it if it was committed.
    pub fn backspace(&mut self) {
        let Some(slot) = self.selected else { return };
        if let Some(mut e) = self.entries.remove(&slot) {
            e.pop();
            if !e.is_empty() {
                self.drafts.insert(slot, e);
            }
            return;
        }
        if let Some(d) = self.drafts.get_mut(&slot) {
            d.pop();
            if d.is_empty() {
                self.drafts.remove(&slot);
            }
        }
    }

    /// "Clear word": the slot returns to empty.
    pub fn clear_word(&mut self, slot: usize) {
        self.entries.remove(&slot);
        self.drafts.remove(&slot);
    }

    /// F7: the full-board check, or None when it does not apply.
    pub fn check(&self) -> Option<Check> {
        if !all_committed(&self.board, &self.entries) || self.solved() || board_view(&self.board, &self.entries).contains(&RuneState::Contested) {
            return None;
        }
        let wrong = wrong_slots(&self.board, &self.entries);
        if wrong.is_empty() {
            return None;
        }
        Some(match self.board.tier {
            Tier::Jr | Tier::Easy => Check::Slots(wrong),
            _ => Check::Count(wrong.len()),
        })
    }

    /// F8: worked out when the board is solved, and only then.
    pub fn stars(&self) -> Option<Stars> {
        if !self.solved() {
            return None;
        }
        let first_right = |slot: usize| self.first_commit.get(&slot) == Some(&self.board.slots[slot].answer);
        let sharp_ear = (0..self.board.slots.len()).filter(|&i| self.board.slots[i].kind == SlotKind::Spoken).all(first_right);
        let secret = self.board.slots.len() - 1;
        let earned_secret = first_right(secret) && (self.is_jr() || self.secret_undecoded_at_first == Some(true));
        Some(Stars { solved: true, sharp_ear, codebreaker: self.listened.is_empty() && earned_secret })
    }

    /// The word the orb says for the first time when the board is solved (F6).
    pub fn finale_word(&self) -> Option<String> {
        self.solved().then(|| self.word(self.board.slots.len() - 1))
    }
}
