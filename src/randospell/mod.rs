//! CC-RANDOSPELL v1 Phase A -- the engine: dice, word supply, scoring, Sprint.
//!
//! A pure reducer. No DOM, no wall clock, no float in anything that decides an
//! outcome; every random choice comes from `spelldoku::rng::Rng`, so a seed, a
//! bank and a list of actions give the same session on every target (I1, I3).
//! The 30 s timer (D19) lives in the screen layer, which calls `timeout`.
//!
//! # Two streams, on purpose
//!
//! The dice and the words draw from separate streams derived from the seed. The
//! dice therefore depend only on how many presses have happened, never on which
//! words a bank holds: a roll is a pure function of the stream state at
//! press-time (I1), and the animation has no way to reach it (A3).
//!
//! # Length, tier and the bank (census, Eric 2026-10-14)
//!
//! Length coverage POOLS all four tier files. The tier the player is on only
//! biases which words are preferred at a rolled length: the player's tier
//! first, then the nearest tier that has an unused word of that length. The
//! stored length unit is derived (grapheme clusters, spaces excluded, D12);
//! there is no stored field in the bank (census finding 1).
//!
//! # Marathon (Phase B)
//!
//! A correct answer adds to the POT, not the score, and the player must then
//! Bank or Roll again (F5). A miss empties the pot unless the one Ward absorbs
//! it (D13); the Ward is spent only by a loss of a non-empty pot. The game ends
//! only on a Bank that reaches 100 (D15), never because the pot would cross it.
//! If a length runs out of words the pot is banked and the game ends: score
//! never falls (I5), and nothing the player earned is thrown away.
//!
//! # Head-to-Head (Phase C)
//!
//! Two players on one phone share one dice stream and one word supply and
//! take turns. A turn is a run of rolls that ends on a Bank or a miss; a miss
//! hands the other player ONE bonus roll at the start of their next turn
//! (a flag, so two misses in a row do not stack). A bonus roll is free: a
//! right answer is banked at once, a wrong one costs nothing and grants
//! nothing. The first Bank that reaches 100 wins (D15). The idle player's
//! numbers live in `other`; `pass_turn` swaps them with the live fields so
//! everything that reads `score`, `pot` or `ward` shows the player on the move.
//!
//! # Not here yet
//!
//! (Phase D, the Jr die, is below.)

#![allow(dead_code)] // wired by the screen in the next Phase A step

pub mod bank;
pub mod daily;
pub mod payout;
pub mod store;
pub mod table;
#[cfg(test)]
mod tests;

use crate::spelldoku::rng::Rng;
pub use bank::Bank;
pub use payout::{Call, Rules};

pub const SPRINT_GOAL: u32 = 50;
pub const MARATHON_GOAL: u32 = 100;
/// D19: a standard-mode spelling times out after this long (a miss).
pub const TIMER_SECS: u32 = 30;
/// D7: three correct in a row switch the multiplier on.
pub const STREAK_FOR_MULTIPLIER: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sprint,
    Marathon,
    /// Pass-and-play, Marathon rules, first to 100 after a Bank (F8).
    HeadToHead,
    /// Spell Jr (F7): Sprint with one die, faces 1-6 playing word lengths 2-7,
    /// no pot, no Ward, no multiplier, and one more try after a miss.
    Jr,
    /// v1.1 F10: the human at a table with 1-3 computer players, Marathon
    /// rules, alternating turns. The NPCs live in `table::Table`; this mode
    /// makes a miss or a Bank hand the turn on, without swapping seats.
    Table,
}

impl Mode {
    /// Pot, Bank and Ward rules apply.
    pub fn pot_rules(self) -> bool {
        matches!(self, Mode::Marathon | Mode::HeadToHead | Mode::Table)
    }

    /// A miss or a Bank ends the player's turn.
    pub fn passes_turns(self) -> bool {
        matches!(self, Mode::HeadToHead | Mode::Table)
    }

    pub fn goal(self) -> u32 {
        match self {
            Mode::Sprint | Mode::Jr => SPRINT_GOAL,
            Mode::Marathon | Mode::HeadToHead | Mode::Table => MARATHON_GOAL,
        }
    }
}

/// A die face is the sum of two dice, 2..=12 (F1).
pub fn is_jackpot(face: u8) -> bool {
    face == 2 || face == 12
}

/// F3: what a correct answer on `face` is worth BEFORE the streak multiplier.
pub fn base_points(face: u8) -> u32 {
    let v = face as u32;
    if is_jackpot(face) {
        v * 2
    } else {
        v
    }
}

/// F3/D7: ceil(base x 1.5), in integers.
pub fn multiplied(base: u32) -> u32 {
    (base * 3).div_ceil(2)
}

/// One press of the orb. `face` is what the dice made; `len` is the roll the
/// player SEES after any D4 remap, the length of `word`, and the number that
/// scores (A9: the die never shows a length with no words behind it). A jackpot
/// is a displayed 2 or 12, so a remapped 12 is an ordinary 11.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roll {
    pub face: u8,
    pub len: usize,
    pub word: String,
    /// The tier file the word came from, 0..=3.
    pub tier: usize,
    /// F14: the two underlying dice matched (Jr: a roll of 6). Derived from the
    /// same draw as the sum (I12); the dice themselves are never shown.
    pub doubles: bool,
    /// F13/F17: the choices made for this word, locked once it is drawn.
    pub stretch: bool,
    pub ear: bool,
    /// F15: this word is a Steal word (Hard + Expert, length 7), not a points word.
    pub steal: bool,
    /// F12: the guess placed before the press.
    pub call: Option<Call>,
}

/// What happened, in order, for the Stretch ordering law (I14, A22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    StretchChosen(bool),
    WordDrawn { tier: usize },
}

/// What happened when the roll was answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub correct: bool,
    pub points: u32,
    /// True when the 1.5 multiplier was applied to this answer.
    pub multiplied: bool,
    pub streak: u32,
    pub score: u32,
    pub finished: bool,
    /// v1.1: what shaped the payout, and the heat after this answer.
    pub doubles: bool,
    pub stretch: bool,
    pub ear: bool,
    pub heat: u8,
    pub call_bonus: u32,
    /// Marathon: the pot after this answer.
    pub pot: u32,
    /// Marathon: the Ward absorbed a lost pot.
    pub warded: bool,
    /// Marathon: the pot a miss emptied (0 when warded or when nothing was at risk).
    pub lost: u32,
    /// Marathon: the player now owes a Bank or Roll again.
    pub deciding: bool,
    /// Head-to-Head: the turn went to the other player.
    pub passed: bool,
    /// Head-to-Head: this was a free bonus roll.
    pub bonus_roll: bool,
    /// Jr: a miss that lets the child try the same word once more. Nothing
    /// was scored and the streak is untouched.
    pub retry: bool,
    /// F15: 0 none, 1 a Steal landed (no points; the caller moves the pot), 2 it failed.
    pub steal: u8,
}

/// What a Bank did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banked {
    pub banked: u32,
    pub score: u32,
    pub finished: bool,
    pub passed: bool,
    /// F16: this Bank reached the goal and opened the Final Round instead of
    /// ending the game. The caller (the table) runs the final turns.
    pub final_round: bool,
}

/// The idle player's numbers in Head-to-Head.
#[derive(Clone, Debug, Default)]
struct PlayerState {
    score: u32,
    pot: u32,
    streak: u32,
    rolls: u32,
    correct: u32,
    ward: bool,
    ward_used: bool,
    largest_banked: u32,
    largest_lost: u32,
    longest: Option<String>,
    bonus: bool,
    heat: u8,
    attempts: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Goal,
    /// A length had no unused word left: the session ends rather than repeat (I3).
    Exhausted,
}

/// F19: what the player chose and what came of it, for the result card and the
/// mode-local store. Counts only; nothing leaves the device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BetStats {
    pub stretch_taken: u32,
    pub stretch_won: u32,
    pub ear_taken: u32,
    pub ear_won: u32,
    pub call_taken: u32,
    pub call_won: u32,
    pub steals_tried: u32,
    pub steals_landed: u32,
    pub max_heat: u8,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub mode: Mode,
    pub score: u32,
    pub streak: u32,
    pub rolls: u32,
    pub correct: u32,
    /// Rolls as displayed, index 0 is a roll of 2 (D20).
    pub histogram: [u32; 11],
    pub longest: Option<String>,
    pub ended: Option<End>,
    /// Marathon: points earned and not yet banked.
    pub pot: u32,
    /// Marathon: one Ward per game (D13), true while it is still held.
    pub ward: bool,
    pub ward_used: bool,
    pub largest_banked: u32,
    pub largest_lost: u32,
    /// Marathon: a correct answer is waiting for Bank or Roll again.
    pub deciding: bool,
    /// Answered words (a Jr retry is not one): the denominator of the human's
    /// running accuracy that NPCs track (D13).
    pub attempts: u32,
    /// The pot of the human's most recent Bank, for the Mirror personality.
    pub last_bank: Option<u32>,
    /// The longest run of right answers, tracked for the result card. Jr shows
    /// no multiplier but keeps this silently (F7).
    pub best_streak: u32,
    /// Jr: how many tries the pending word has had.
    attempt: u8,
    /// v1.1: which mechanics are on (all off = v1), set by the screen from flags.
    pub rules: Rules,
    pub bets: BetStats,
    /// F11: unbanked correct words in a row, 0..=7.
    pub heat: u8,
    /// F12: the guess for the NEXT press; locked on press (D-R15).
    call: Option<Call>,
    /// Rolled but the word is not drawn yet (Stretch and Ear are chosen here).
    rolled: Option<Roll>,
    /// Audio requests made for the pending word (I15).
    plays: u8,
    pub events: Vec<Ev>,
    /// F16 (Head-to-Head): the player who banked to the goal; the other has
    /// exactly one final turn.
    pub final_leader: Option<usize>,
    /// Head-to-Head: 0 or 1, the player on the move.
    pub turn: usize,
    /// Head-to-Head: who banked past the goal.
    pub winner: Option<usize>,
    /// Head-to-Head: the player on the move holds a bonus roll.
    pub bonus: bool,
    /// The pending roll is that bonus roll.
    bonus_roll: bool,
    other: PlayerState,
    tier: usize,
    dice: Rng,
    words: Rng,
    bank: Bank,
    /// Words actually served, in order (the no-repeat law, I3, and the history).
    used: Vec<String>,
    /// Words the NORMAL stream has drawn, served or not: a Stretch swaps the
    /// served word but the normal stream still advances and the word it drew
    /// still counts, so a Daily's ordinary words do not depend on who Stretched.
    drawn: Vec<String>,
    /// Stretch words, drawn from their own stream.
    stretch_used: Vec<String>,
    words_s: Rng,
    avoid: std::collections::BTreeSet<String>,
    pending: Option<Roll>,
}

impl Session {
    /// `tier` is 0..=3 (easy..expert). `avoid` is the cross-session history
    /// (D9): words held there are used only after the fresh ones run out.
    pub fn new(seed: u64, mode: Mode, tier: usize, bank: Bank, avoid: std::collections::BTreeSet<String>) -> Session {
        Session {
            mode,
            score: 0,
            streak: 0,
            rolls: 0,
            correct: 0,
            histogram: [0; 11],
            longest: None,
            ended: None,
            pot: 0,
            ward: mode.pot_rules(),
            ward_used: false,
            largest_banked: 0,
            largest_lost: 0,
            deciding: false,
            attempts: 0,
            last_bank: None,
            rules: Rules::default(),
            bets: BetStats::default(),
            heat: 0,
            call: None,
            rolled: None,
            plays: 0,
            events: Vec::new(),
            best_streak: 0,
            attempt: 0,
            final_leader: None,
            turn: 0,
            winner: None,
            bonus: false,
            bonus_roll: false,
            other: PlayerState { ward: mode == Mode::HeadToHead, ..PlayerState::default() },
            tier: tier.min(3),
            dice: Rng::new(seed ^ 0xD1CE_D1CE_5EED_0001),
            words: Rng::new(seed ^ 0x0057_0AD5_0000_0002),
            bank,
            used: Vec::new(),
            drawn: Vec::new(),
            stretch_used: Vec::new(),
            words_s: Rng::new(seed ^ 0x57BE_7C4D_0000_0003),
            avoid,
            pending: None,
        }
    }

    /// Test seam only: put the score where a test needs it (A7). Never called
    /// by the game.
    #[cfg(feature = "testseam")]
    pub fn set_score_for_test(&mut self, n: u32) {
        self.score = n;
    }

    pub fn pending(&self) -> Option<&Roll> {
        self.pending.as_ref()
    }

    /// The die has landed but the word is not drawn yet (Stretch, Ear and
    /// Steal are chosen here).
    pub fn rolled(&self) -> Option<&Roll> {
        self.rolled.as_ref()
    }

    /// Every word served this session, in order: what the screen layer writes
    /// to the shared no-repeat history once (D9).
    pub fn served(&self) -> &[String] {
        &self.used
    }

    /// F12 / D-R15: the guess for the next roll. It must be placed before the
    /// orb is pressed and locks on the press. `None` clears it. Not in Jr, and
    /// only when the Call rule is on.
    pub fn set_call(&mut self, call: Option<Call>) -> bool {
        if !self.rules.call || self.mode == Mode::Jr || self.pending.is_some() || self.rolled.is_some() || self.ended.is_some() {
            return false;
        }
        self.call = call;
        true
    }

    pub fn call(&self) -> Option<Call> {
        self.call
    }

    /// F1: the player pressed the orb. Dice and word at once, with no Stretch or
    /// Ear choice (v1 behaviour). Use `press_roll` then `reveal` to offer them.
    pub fn press(&mut self) -> Option<Roll> {
        self.press_roll()?;
        self.reveal(false, false)
    }

    /// F1, first half: the dice. The face comes from the dice stream right here;
    /// nothing the animation does can change it (I1, A3). Returns the roll with
    /// no word yet, so the screen can offer Stretch and Ear before anything is
    /// fetched (D-R9). None while a roll is unanswered or after the game ended.
    pub fn press_roll(&mut self) -> Option<Roll> {
        if self.pending.is_some() || self.rolled.is_some() || self.ended.is_some() || self.deciding {
            return None;
        }
        let bonus_roll = self.mode == Mode::HeadToHead && self.bonus;
        let jr = self.mode == Mode::Jr;
        // F7: Jr rolls ONE die (1..=6) for a word of face + 1 letters, kept
        // inside 2..=7 by the remap; everyone else rolls two dice. F14: doubles
        // come from the same draw as the sum (I12); Jr's lucky face is a 6.
        let (face, doubles) = if jr {
            let f = self.dice.below(6) as u8 + 1;
            (f, f == 6)
        } else {
            let (a, b) = roll_two_dice_pair(&mut self.dice);
            (a + b, a == b)
        };
        self.rolls += 1;
        let len = if jr {
            self.bank.cov.length_within(face as usize + 1, bank::JR_LO, bank::JR_HI)
        } else {
            self.bank.cov.length_for(face)
        };
        let Some(len) = len else {
            self.exhaust();
            return None;
        };
        self.histogram[len - 2] += 1;
        if bonus_roll {
            self.bonus = false;
            self.bonus_roll = true;
        }
        let r = Roll { face, len, doubles, call: self.call.take(), ..Roll::default() };
        self.rolled = Some(r.clone());
        self.plays = 0;
        Some(r)
    }

    /// F13: could this roll be Stretched? A harder word has to exist at the
    /// rolled length (census C6), the rule has to be on, and not in Jr or at Expert.
    pub fn stretch_offered(&self) -> bool {
        self.rolled.as_ref().is_some_and(|r| self.stretch_available_at(r.len))
    }

    /// F13 for a given length (the table's NPCs ask this without a roll in hand).
    pub fn stretch_available_at(&self, len: usize) -> bool {
        self.rules.stretch
            && self.mode != Mode::Jr
            && self.tier < 3
            && self.bank.of_len(len).iter().filter(|e| e.tier == self.tier + 1).count() >= bank::CELL_FLOOR
    }

    /// F10: an NPC's roll. The same dice stream and coverage as a human roll, so
    /// the table's dice are one fair sequence, but no word is drawn (I11: NPCs
    /// never show or play one). Returns (displayed length, doubles).
    pub fn npc_roll(&mut self) -> Option<(usize, bool)> {
        if self.ended.is_some() {
            return None;
        }
        let jr = self.mode == Mode::Jr;
        let (face, doubles) = if jr {
            let f = self.dice.below(6) as u8 + 1;
            (f, f == 6)
        } else {
            let (a, b) = roll_two_dice_pair(&mut self.dice);
            (a + b, a == b)
        };
        let len = if jr {
            self.bank.cov.length_within(face as usize + 1, bank::JR_LO, bank::JR_HI)
        } else {
            self.bank.cov.length_for(face)
        }?;
        self.histogram[len - 2] += 1;
        Some((len, doubles))
    }

    /// End the game from outside (the table, when an NPC wins).
    pub fn force_end(&mut self, why: End) {
        if self.ended.is_none() {
            self.ended = Some(why);
        }
    }

    /// F17: is Ear on the table, given the Stretch choice? D-R3's ceiling.
    pub fn ear_offered(&self, stretch: bool) -> bool {
        let Some(r) = &self.rolled else { return false };
        self.rules.ear && self.mode != Mode::Jr && payout::ear_offered(r.len, r.doubles && self.rules.doubles, stretch)
    }

    /// F1, second half: draw the word. `stretch` and `ear` are the player's
    /// choices; one that is not on offer is ignored. The choice is logged BEFORE
    /// the draw (I14), and the draw uses tier + 1 when Stretch is set.
    pub fn reveal(&mut self, stretch: bool, ear: bool) -> Option<Roll> {
        let mut r = self.rolled.take()?;
        self.rolled = Some(r.clone());
        let stretch = stretch && self.stretch_offered();
        let ear = ear && self.ear_offered(stretch);
        self.rolled = None;
        self.events.push(Ev::StretchChosen(stretch));
        // The normal stream ALWAYS advances, so the ordinary words of a game are
        // the same whoever Stretched; Stretch then swaps the served word for one
        // a tier up, drawn from its own stream.
        let normal = self.pick(r.len, None);
        let Some((ntier, nword)) = normal else {
            self.exhaust();
            return None;
        };
        let mut served = (ntier, nword);
        let mut stretched = false;
        if stretch {
            if let Some(sw) = self.pick(r.len, Some(self.tier + 1)) {
                served = sw;
                stretched = true;
            }
        }
        // The choice is logged before the draw it governs (I14).
        self.events.push(Ev::WordDrawn { tier: served.0 });
        self.used.push(served.1.clone());
        r.word = served.1;
        r.tier = served.0;
        r.stretch = stretched;
        r.ear = ear;
        if stretched {
            self.bets.stretch_taken += 1;
        }
        if ear {
            self.bets.ear_taken += 1;
        }
        if r.call.is_some() {
            self.bets.call_taken += 1;
        }
        self.pending = Some(r.clone());
        Some(r)
    }

    /// I15: may the word be played again? The first request is always allowed;
    /// a word chosen with Ear gets no second one.
    pub fn request_audio(&mut self) -> bool {
        let ear = self.pending.as_ref().is_some_and(|r| r.ear);
        if self.plays >= 1 && ear {
            return false;
        }
        self.plays = self.plays.saturating_add(1);
        true
    }

    fn exhaust(&mut self) {
        // Nothing left to draw: bank whatever the pot holds so no earned point
        // is lost, then end (see the module docs).
        if self.pot > 0 {
            self.largest_banked = self.largest_banked.max(self.pot);
            self.score += self.pot;
            self.pot = 0;
        }
        self.heat = 0;
        self.ended = Some(End::Exhausted);
        if self.mode == Mode::HeadToHead {
            self.winner = match self.score.cmp(&self.other.score) {
                std::cmp::Ordering::Greater => Some(self.turn),
                std::cmp::Ordering::Less => Some(1 - self.turn),
                std::cmp::Ordering::Equal => None,
            };
        }
    }

    /// F4/F5: score the pending roll. A miss is 0 and resets the streak (D5).
    /// Sprint adds straight to the score; Marathon adds to the pot and then
    /// waits for `bank` or `roll_again`.
    pub fn answer(&mut self, correct: bool) -> Option<Outcome> {
        if self.pending.is_none() {
            return None;
        }
        // F7/D18: only Jr gets a second try, and only once per word. The
        // roll stays pending, nothing is scored and the streak is untouched.
        if self.mode == Mode::Jr && !correct && self.attempt == 0 {
            self.attempt = 1;
            return Some(Outcome {
                correct: false,
                points: 0,
                multiplied: false,
                streak: self.streak,
                score: self.score,
                finished: false,
                doubles: false,
                stretch: false,
                ear: false,
                heat: self.heat,
                call_bonus: 0,
                steal: 0,
                pot: 0,
                warded: false,
                lost: 0,
                deciding: false,
                passed: false,
                bonus_roll: false,
                retry: true,
            });
        }
        self.attempt = 0;
        self.attempts += 1;
        let r = self.pending.take()?;
        let marathon = self.mode.pot_rules();
        let h2h = self.mode == Mode::HeadToHead;
        let bonus_roll = std::mem::take(&mut self.bonus_roll);
        let mut points = 0;
        let mut mult = false;
        let (mut warded, mut lost) = (false, 0);
        let (mut doubles_hit, mut bonus_paid) = (false, 0);
        if r.steal && correct {
            // F15: a landed steal pays no points; the table moves the pot. Heat
            // is unchanged, the streak and the longest word count as a right word.
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);
            self.correct += 1;
            self.deciding = true;
            self.bets.steals_landed += 1;
            if self.longest.as_ref().map_or(true, |w| r.len > bank::length_of(w)) {
                self.longest = Some(r.word.clone());
            }
            return Some(Outcome {
                correct: true,
                points: 0,
                multiplied: false,
                streak: self.streak,
                score: self.score,
                finished: false,
                doubles: false,
                stretch: false,
                ear: false,
                heat: self.heat,
                call_bonus: 0,
                steal: 1,
                pot: self.pot,
                warded: false,
                lost: 0,
                deciding: true,
                passed: false,
                bonus_roll: false,
                retry: false,
            });
        }
        if correct {
            // D-R3: one pure payout. Jr scores the die (length - 1) with no jackpot.
            // With heat on, a pot mode uses the heat ladder in place of v1's streak
            // multiplier (D-R2); Sprint and Jr keep the streak.
            let jr = self.mode == Mode::Jr;
            let factor = if self.rules.heat && marathon {
                payout::heat_factor(self.heat)
            } else if self.streak >= STREAK_FOR_MULTIPLIER && !jr {
                (3, 2)
            } else {
                (1, 1)
            };
            let call_bonus = r.call.filter(|c| c.hits(r.len)).map_or(0, |c| c.bonus());
            let p = payout::payout(payout::PayIn {
                roll: r.len,
                jr,
                doubles: r.doubles && self.rules.doubles,
                stretch: r.stretch,
                ear: r.ear,
                factor,
                call_bonus,
            });
            mult = factor != (1, 1);
            doubles_hit = r.doubles && self.rules.doubles;
            bonus_paid = call_bonus;
            if r.stretch {
                self.bets.stretch_won += 1;
            }
            if r.ear {
                self.bets.ear_won += 1;
            }
            if call_bonus > 0 {
                self.bets.call_won += 1;
            }
            points = p;
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);
            self.correct += 1;
            if bonus_roll {
                // A bonus roll is banked at once and never put at risk.
                self.score += p;
                self.largest_banked = self.largest_banked.max(p);
                self.heat = 0; // banked, so there is nothing unbanked to heat
            } else if marathon {
                self.pot += p;
                self.deciding = true;
                if self.rules.heat {
                    self.heat = (self.heat + 1).min(payout::MAX_HEAT);
                    self.bets.max_heat = self.bets.max_heat.max(self.heat);
                }
            } else {
                self.score += p; // I5: score only ever rises.
            }
            let longer = self.longest.as_ref().map_or(true, |w| r.len > bank::length_of(w));
            if longer {
                self.longest = Some(r.word.clone());
            }
        } else {
            self.streak = 0;
            // I13: any miss, a Ward-absorbed one included, takes the heat to 0 (D-R14).
            self.heat = 0;
            if marathon && !bonus_roll && self.pot > 0 {
                if self.ward {
                    // D13: the pot survives, the Ward is spent, the streak still resets.
                    self.ward = false;
                    self.ward_used = true;
                    warded = true;
                } else {
                    lost = self.pot;
                    self.largest_lost = self.largest_lost.max(lost);
                    self.pot = 0;
                }
            }
        }
        // Sprint ends on the score. Marathon ends only on a Bank (D15); a
        // Head-to-Head bonus roll IS a Bank.
        let mut finished = (!marathon || (bonus_roll && correct)) && self.score >= self.mode.goal();
        if finished && h2h && self.rules.lastcall && self.final_leader.is_none() {
            // F16: reaching the goal on a bonus roll opens the Final Round too.
            finished = false;
            self.final_leader = Some(self.turn);
            self.pass_turn();
        } else if finished {
            self.ended = Some(End::Goal);
            if h2h {
                if self.final_leader.is_some() {
                    self.conclude_h2h();
                } else {
                    self.winner = Some(self.turn);
                }
            }
        }
        let mut out = Outcome {
            correct,
            points,
            multiplied: mult,
            streak: self.streak,
            score: self.score,
            finished,
            doubles: doubles_hit,
            stretch: r.stretch,
            ear: r.ear,
            heat: self.heat,
            call_bonus: bonus_paid,
            steal: if r.steal && !correct { 2 } else { 0 },
            pot: self.pot,
            warded,
            lost,
            deciding: self.deciding,
            passed: false,
            bonus_roll,
            retry: false,
        };
        // Head-to-Head: a real miss hands over the turn and grants the other
        // player a bonus roll (a flag: it does not stack).
        if self.mode.passes_turns() && !correct && !bonus_roll && !finished {
            if h2h && self.final_leader.is_some() && self.ended.is_none() {
                // F16: the final turn is over; no bonus roll is worth granting now.
                self.conclude_h2h();
                out.finished = true;
            } else {
                if h2h {
                    self.other.bonus = true;
                    self.pass_turn();
                }
                out.passed = true;
            }
        }
        Some(out)
    }

    /// Swap the live player's numbers with the idle player's.
    fn pass_turn(&mut self) {
        let o = &mut self.other;
        std::mem::swap(&mut self.score, &mut o.score);
        std::mem::swap(&mut self.pot, &mut o.pot);
        std::mem::swap(&mut self.streak, &mut o.streak);
        std::mem::swap(&mut self.rolls, &mut o.rolls);
        std::mem::swap(&mut self.correct, &mut o.correct);
        std::mem::swap(&mut self.ward, &mut o.ward);
        std::mem::swap(&mut self.ward_used, &mut o.ward_used);
        std::mem::swap(&mut self.largest_banked, &mut o.largest_banked);
        std::mem::swap(&mut self.largest_lost, &mut o.largest_lost);
        std::mem::swap(&mut self.longest, &mut o.longest);
        std::mem::swap(&mut self.bonus, &mut o.bonus);
        std::mem::swap(&mut self.heat, &mut o.heat);
        std::mem::swap(&mut self.attempts, &mut o.attempts);
        self.turn = 1 - self.turn;
        self.deciding = false;
    }

    /// Head-to-Head: a player's banked score, whoever is on the move.
    pub fn score_of(&self, player: usize) -> u32 {
        if player == self.turn { self.score } else { self.other.score }
    }

    /// Head-to-Head: does this player hold a bonus roll?
    pub fn bonus_of(&self, player: usize) -> bool {
        if player == self.turn { self.bonus } else { self.other.bonus }
    }

    /// Head-to-Head: a player's Ward, whoever is on the move.
    pub fn ward_of(&self, player: usize) -> bool {
        if player == self.turn { self.ward } else { self.other.ward }
    }

    /// F5: move the pot into the score. The only way a Marathon ends (D15).
    pub fn bank(&mut self) -> Option<Banked> {
        if !self.deciding || !self.mode.pot_rules() {
            return None;
        }
        let banked = self.pot;
        self.last_bank = Some(banked);
        self.heat = 0; // I13: a Bank takes the heat to 0
        self.score += banked;
        self.largest_banked = self.largest_banked.max(banked);
        self.pot = 0;
        self.deciding = false;
        let reaches = self.score >= self.mode.goal();
        let score = self.score;
        let lastcall = self.rules.lastcall && matches!(self.mode, Mode::Table | Mode::HeadToHead);
        let (mut finished, mut passed, mut final_round) = (reaches, false, false);
        if reaches && lastcall && self.mode == Mode::Table {
            // F16: the table opens the Final Round; the session does not end.
            finished = false;
            passed = true;
            final_round = true;
        } else if lastcall && self.mode == Mode::HeadToHead {
            if self.final_leader.is_some() {
                // The other player's final turn is over: settle it.
                finished = true;
                self.conclude_h2h();
            } else if reaches {
                finished = false;
                final_round = true;
                self.final_leader = Some(self.turn);
                self.pass_turn();
                passed = true;
            } else {
                self.pass_turn();
                passed = true;
            }
        } else if finished {
            self.ended = Some(End::Goal);
            if self.mode == Mode::HeadToHead {
                self.winner = Some(self.turn);
            }
        } else if self.mode == Mode::HeadToHead {
            self.pass_turn();
            passed = true;
        } else if self.mode == Mode::Table {
            passed = true;
        }
        Some(Banked { banked, score, finished, passed, final_round })
    }

    /// F5: keep the pot at risk and roll again.
    pub fn roll_again(&mut self) -> bool {
        if !self.deciding {
            return false;
        }
        self.deciding = false;
        true
    }

    /// F16 / D-R18: settle a Head-to-Head Final Round. Highest banked score
    /// wins; a tie goes to the longest word, then the fewest misses, then the
    /// leader who opened the round.
    fn conclude_h2h(&mut self) {
        let me = self.turn;
        let you = 1 - me;
        let (ms, ys) = (self.score, self.other.score);
        let len = |w: &Option<String>| w.as_deref().map_or(0, bank::length_of);
        let (ml, yl) = (len(&self.longest), len(&self.other.longest));
        let misses = |attempts: u32, correct: u32| attempts.saturating_sub(correct);
        let (mm, ym) = (misses(self.attempts, self.correct), misses(self.other.attempts, self.other.correct));
        let leader = self.final_leader.unwrap_or(you);
        self.winner = Some(if ms != ys {
            if ms > ys { me } else { you }
        } else if ml != yl {
            if ml > yl { me } else { you }
        } else if mm != ym {
            if mm < ym { me } else { you }
        } else {
            leader
        });
        self.ended = Some(End::Goal);
        self.deciding = false;
    }

    /// F15: could the player on the move Steal on this roll? Only in the NPC
    /// table and Head-to-Head (I18), only on a 7, only with a pot to take and a
    /// Hard-or-Expert word of that length to serve (no padding).
    pub fn steal_offered(&self, opponent_best_pot: u32) -> bool {
        let Some(r) = &self.rolled else { return false };
        self.rules.steal
            && matches!(self.mode, Mode::Table | Mode::HeadToHead)
            && r.len == 7
            && opponent_best_pot >= 1
            && self.steal_pool_ok()
    }

    /// Is there a Steal word to serve at length 7 (>= the coverage floor in the
    /// Hard and Expert tiers the player owns)?
    pub fn steal_pool_ok(&self) -> bool {
        self.bank.of_len(7).iter().filter(|e| e.tier >= 2).count() >= bank::CELL_FLOOR
    }

    /// F15: draw the Steal word instead of a points word. The ordinary word
    /// stream still advances (D-R9 / Daily purity), as for Stretch.
    pub fn reveal_steal(&mut self) -> Option<Roll> {
        let mut r = self.rolled.take()?;
        let ok = self.rules.steal && matches!(self.mode, Mode::Table | Mode::HeadToHead) && r.len == 7 && self.steal_pool_ok();
        if !ok {
            self.rolled = Some(r);
            return None;
        }
        let _ = self.pick(7, None); // the normal stream advances either way
        let mut cands: Vec<(usize, String)> = self
            .bank
            .of_len(7)
            .iter()
            .filter(|e| e.tier >= 2 && !self.used.contains(&e.word) && !self.stretch_used.contains(&e.word))
            .map(|e| (e.tier, e.word.clone()))
            .collect();
        if cands.is_empty() {
            self.exhaust();
            return None;
        }
        let i = self.words_s.below(cands.len());
        let (tier, word) = cands.swap_remove(i);
        self.stretch_used.push(word.clone());
        self.events.push(Ev::WordDrawn { tier });
        self.used.push(word.clone());
        r.word = word;
        r.tier = tier;
        r.steal = true;
        self.bets.steals_tried += 1;
        if r.call.is_some() {
            self.bets.call_taken += 1;
        }
        self.pending = Some(r.clone());
        Some(r)
    }

    /// F15: the pot a successful steal adds. Heat is untouched.
    pub fn receive_steal(&mut self, n: u32) {
        self.pot += n;
    }

    /// F15: an opponent took `n` of this player's pot.
    pub fn take_pot(&mut self, n: u32) -> u32 {
        let t = n.min(self.pot);
        self.pot -= t;
        t
    }

    /// Head-to-Head: the idle player's unbanked pot.
    pub fn h2h_opponent_pot(&self) -> u32 {
        self.other.pot
    }

    /// Head-to-Head F15, success: floor(half) of the opponent's pot moves to the
    /// stealer; the opponent keeps the ceiling half. Returns the amount moved.
    pub fn resolve_steal_h2h(&mut self) -> u32 {
        let n = self.other.pot / 2;
        self.other.pot -= n;
        self.pot += n;
        n
    }

    /// D19: the 30 s soft timer ran out. A miss.
    pub fn timeout(&mut self) -> Option<Outcome> {
        self.answer(false)
    }

    fn pick(&mut self, len: usize, only_tier: Option<usize>) -> Option<(usize, String)> {
        let mut cands: Vec<(usize, String)> = self
            .bank
            .of_len(len)
            .iter()
            .filter(|e| {
                !self.used.contains(&e.word)
                    && only_tier.map_or(!self.drawn.contains(&e.word), |t| e.tier == t && !self.stretch_used.contains(&e.word))
            })
            .map(|e| (e.tier, e.word.clone()))
            .collect();
        if cands.is_empty() {
            return None;
        }
        // Fresh (not in the history) first, then the player's tier, then the
        // nearest tier: tier biases difficulty and nothing else.
        if cands.iter().any(|(_, w)| !self.avoid.contains(w)) {
            cands.retain(|(_, w)| !self.avoid.contains(w));
        }
        let best = cands.iter().map(|(t, _)| t.abs_diff(self.tier)).min()?;
        cands.retain(|(t, _)| t.abs_diff(self.tier) == best);
        let i = if only_tier.is_some() { self.words_s.below(cands.len()) } else { self.words.below(cands.len()) };
        let picked = cands.swap_remove(i);
        if only_tier.is_some() {
            self.stretch_used.push(picked.1.clone());
        } else {
            self.drawn.push(picked.1.clone());
        }
        Some(picked)
    }
}

/// F1: two dice, 2..=12. Two draws from the stream, in a fixed order.
pub fn roll_two_dice(rng: &mut Rng) -> u8 {
    let (a, b) = roll_two_dice_pair(rng);
    a + b
}

/// The two dice themselves, in draw order. Only the engine sees them: the
/// screen shows the sum and, for a pair, a glow (F14, v1 ban on visible dice).
pub fn roll_two_dice_pair(rng: &mut Rng) -> (u8, u8) {
    let a = rng.below(6) as u8 + 1;
    let b = rng.below(6) as u8 + 1;
    (a, b)
}
