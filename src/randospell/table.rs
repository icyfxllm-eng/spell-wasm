//! CC-RANDOSPELL v1.1 F10 -- the NPC table: the human against 1-3 computer
//! players, Marathon rules, alternating turns.
//!
//! The human is a `Session` (`Mode::Table`, or `Mode::Jr` for the racing NPC)
//! that the caller owns and passes in; this module owns the computer seats and
//! everything they do. NPCs draw no
//! words (I11): an NPC roll uses the table's single dice stream and its outcome
//! comes from the table's own seeded stream, so a replay with the same seed and
//! the same human inputs reproduces the same game (I19).
//!
//! # Calibration (Eric, 2026-10-14)
//!
//! An NPC's spelling accuracy is CC-BOARD-GAME D13's `clamp(human - delta,
//! 0.30, 0.95)` over the human's running accuracy, with Randospell's own deltas
//! 0.15 / 0.10 / 0.00. Two things differ from the board game, both measured:
//!
//! * The board game's 0.30 / 0.20 / 0.10 do not carry over: Marathon-rule play
//!   is decided by accuracy streaks, and a simulation put the human at 66-89%
//!   against them where D18 asks for 21-60% (census C8).
//! * The board game's estimate `(3 + hits) / (2 + tries)` has a prior of 1.5.
//!   Games here last about seven turns, so that prior never washes out and the
//!   NPCs sit at the 0.95 ceiling: the engine measured a 70% human winning 8%
//!   at Normal. The prior is 0.7 instead (`PRIOR_MILLI`).
//!
//! With both, a 70% human wins about 60% / 46% / 24% at Easy / Normal / Tough
//! against three Cautious NPCs. A flat delta cannot hold D18 for 100% and 50%
//! humans in games this short (their spread is ~30 pp), so A27 is asserted for
//! the 70% human and the other two are only required to be monotone.
//!
//! # Personalities
//!
//! * Cautious banks at a pot of 12 or more.
//! * Daring rolls on until the pot is 30 or more.
//! * Mirror banks at the size of the human's last Bank, and plays Cautious
//!   until the human has banked once (D-R16, as ruled).
//!
//! Every NPC also banks the moment a Bank would reach the goal. Jr's NPC is a
//! racer: it rolls once per turn and its points go straight to its score.

use super::payout::{self, PayIn};
use super::{Mode, Session, MARATHON_GOAL, SPRINT_GOAL};
use crate::spelldoku::rng::Rng;

pub const MAX_NPCS: usize = 3;
/// The prior mean of the human's accuracy estimate (weight two words), in
/// thousandths. The board game's (3 + hits) / (2 + tries) is a prior of 1.5;
/// over its 84-tile games that washes out, but a Randospell game is about
/// seven turns, so a 1.5 prior would keep every NPC near the 0.95 ceiling for
/// the whole game. A neutral 0.7 is what the calibration below was run with.
pub const PRIOR_MILLI: i64 = 700;
/// Cautious's bank threshold, and Mirror's until the human has banked.
pub const CAUTIOUS_BANK: u32 = 12;
pub const DARING_BANK: u32 = 30;
/// Spelling-accuracy bounds, in thousandths (board game D13).
const FLOOR_MILLI: i64 = 300;
const CEIL_MILLI: i64 = 950;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Personality {
    Cautious,
    Daring,
    Mirror,
}

impl Personality {
    pub const ALL: [Personality; 3] = [Personality::Cautious, Personality::Daring, Personality::Mirror];

    pub fn id(self) -> &'static str {
        match self {
            Personality::Cautious => "cautious",
            Personality::Daring => "daring",
            Personality::Mirror => "mirror",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Tough,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Tough];

    /// How far the NPC trails the human, in thousandths (the calibrated deltas).
    pub fn delta_milli(self) -> i64 {
        match self {
            Difficulty::Easy => 150,
            Difficulty::Normal => 100,
            Difficulty::Tough => 0,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Tough => "tough",
        }
    }
}

/// One computer seat.
#[derive(Clone, Debug)]
pub struct Npc {
    pub personality: Personality,
    pub score: u32,
    pub pot: u32,
    pub heat: u8,
    streak: u32,
    pub ward: bool,
    pub ward_used: bool,
    pub rolls: u32,
    pub busts: u32,
    pub banks: u32,
    pub largest_banked: u32,
    pub largest_lost: u32,
}

impl Npc {
    fn new(personality: Personality) -> Npc {
        Npc {
            personality,
            score: 0,
            pot: 0,
            heat: 0,
            streak: 0,
            ward: true,
            ward_used: false,
            rolls: 0,
            busts: 0,
            banks: 0,
            largest_banked: 0,
            largest_lost: 0,
        }
    }
}

/// What an NPC did, in order, for the screen to play back in under 7 s (D10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// The die landed: the displayed roll and whether it was doubles.
    Rolled { seat: usize, len: usize, doubles: bool, stretched: bool },
    Hit { seat: usize, points: u32, pot: u32 },
    Banked { seat: usize, amount: u32, score: u32 },
    /// A miss that emptied the pot (`lost` may be 0 when nothing was at risk).
    Bust { seat: usize, lost: u32 },
    /// The Ward absorbed the loss.
    Warded { seat: usize, pot: u32 },
    /// Jr's racer scored points straight to its score, or missed.
    Raced { seat: usize, points: u32, score: u32 },
    Won { seat: usize },
}

#[derive(Debug)]
pub struct Table {
    pub npcs: Vec<Npc>,
    pub difficulty: Difficulty,
    /// The prior mean of the human's accuracy estimate, in thousandths, with a
    /// weight of two words. 1500 is the board game's (3 + hits) / (2 + tries).
    pub prior_milli: i64,
    /// Calibration only: use this delta (thousandths) instead of the difficulty's.
    pub delta_override: Option<i64>,
    /// 0 is the human, 1.. the NPCs in seat order.
    pub turn: usize,
    pub winner: Option<usize>,
    rng: Rng,
    jr: bool,
    started: bool,
}

impl Table {
    /// `human` must be `Mode::Table` (Marathon rules) or `Mode::Jr` (a racer).
    /// A Jr table has exactly one NPC, Cautious in name, and no banking.
    pub fn new(seed: u64, human: &Session, mut personalities: Vec<Personality>, difficulty: Difficulty) -> Table {
        let jr = human.mode == Mode::Jr;
        assert!(jr || human.mode == Mode::Table, "a table needs Mode::Table or Mode::Jr");
        if jr {
            personalities = vec![Personality::Cautious];
        }
        personalities.truncate(MAX_NPCS);
        if personalities.is_empty() {
            personalities.push(Personality::Cautious);
        }
        let mut rng = Rng::new(seed ^ 0x7AB1_E0_00_00_00_0004);
        let seats = personalities.len() + 1;
        let turn = rng.below(seats);
        Table {
            npcs: personalities.into_iter().map(Npc::new).collect(),
            difficulty,
            prior_milli: PRIOR_MILLI,
            delta_override: None,
            turn,
            winner: None,
            rng,
            jr,
            started: false,
        }
    }

    pub fn is_jr(&self) -> bool {
        self.jr
    }

    pub fn goal(&self) -> u32 {
        if self.jr { SPRINT_GOAL } else { MARATHON_GOAL }
    }

    /// The human's running accuracy as the board game keeps it, in thousandths.
    pub fn human_acc_milli(&self, h: &Session) -> i64 {
        (2 * self.prior_milli + 1000 * h.correct as i64) / (2 + h.attempts as i64)
    }

    /// D13: the NPCs' spelling accuracy right now, in thousandths.
    pub fn npc_acc_milli(&self, h: &Session) -> i64 {
        (self.human_acc_milli(h) - self.delta_override.unwrap_or(self.difficulty.delta_milli())).clamp(FLOOR_MILLI, CEIL_MILLI)
    }

    pub fn seats(&self) -> usize {
        self.npcs.len() + 1
    }

    /// Banked score of seat `i` (0 is the human).
    pub fn score_of(&self, h: &Session, i: usize) -> u32 {
        if i == 0 { h.score } else { self.npcs[i - 1].score }
    }

    fn chance(&mut self, milli: i64) -> bool {
        (self.rng.next_u64() % 1000) < milli.clamp(0, 1000) as u64
    }

    fn bank_at(&self, h: &Session, p: Personality) -> u32 {
        match p {
            Personality::Cautious => CAUTIOUS_BANK,
            Personality::Daring => DARING_BANK,
            Personality::Mirror => h.last_bank.unwrap_or(CAUTIOUS_BANK),
        }
    }

    /// Run the table until it is the human's turn again, or the game is over.
    /// Call it once at the start (an NPC may be first) and after every human
    /// turn that passed. Each call returns the steps to play back.
    pub fn run_npcs(&mut self, h: &mut Session) -> Vec<Step> {
        let mut steps = Vec::new();
        self.started = true;
        let n = self.seats();
        // Guard: a table has at most 4 seats and a turn is bounded, so this
        // cannot spin; the cap is a belt-and-braces stop, not a rule.
        for _ in 0..(n * 4) {
            if self.winner.is_some() || h.ended.is_some() {
                break;
            }
            if self.turn == 0 {
                break;
            }
            self.npc_turn(h, self.turn, &mut steps);
            self.turn = (self.turn + 1) % n;
        }
        if self.winner.is_none() {
            if let Some(end) = h.ended {
                // The human's own game ended: a goal Bank wins, exhaustion goes to the leader.
                self.winner = Some(match end {
                    super::End::Goal => 0,
                    super::End::Exhausted => (0..n).max_by_key(|i| self.score_of(h, *i)).unwrap_or(0),
                });
            }
        }
        steps
    }

    /// The human's turn is over (a miss or a Bank): hand it to the next seat.
    pub fn human_turn_over(&mut self, h: &Session) {
        if self.winner.is_none() && h.ended.is_none() {
            self.turn = 1 % self.seats();
        }
    }

    /// The human's game ended on their own Bank or roll: record the winner.
    pub fn settle(&mut self, h: &Session) {
        if self.winner.is_none() {
            if let Some(super::End::Goal) = h.ended {
                self.winner = Some(0);
            }
        }
    }

    fn npc_turn(&mut self, h: &mut Session, seat: usize, steps: &mut Vec<Step>) {
        let i = seat - 1;
        let acc = self.npc_acc_milli(h);
        if self.jr {
            self.jr_turn(h, seat, i, acc, steps);
            return;
        }
        let (base, slope, penalty) = crate::boardgame::Variant::Full.cfg().npc_stretch();
        loop {
            let Some((len, doubles)) = h.npc_roll() else {
                // The bank ran out of words at this length: the game ends rather than repeat.
                h.force_end(super::End::Exhausted);
                let best = (0..self.seats()).max_by_key(|s| self.score_of(h, *s)).unwrap_or(0);
                self.winner = Some(best);
                return;
            };
            self.npcs[i].rolls += 1;
            // F13: NPCs Stretch at the board-game NPC rate, only where a harder
            // word exists and the accuracy penalty cannot hit the floor.
            let mut stretched = false;
            let mut p = acc;
            if h.stretch_available_at(len) && acc - penalty >= FLOOR_MILLI {
                let rate = (base + slope * (acc - 500) / 1000).clamp(0, 1000);
                if self.chance(rate) {
                    stretched = true;
                    p = acc - penalty;
                }
            }
            steps.push(Step::Rolled { seat, len, doubles, stretched });
            let ok = self.chance(p);
            let rules = h.rules;
            let npc = &mut self.npcs[i];
            if ok {
                let factor = if rules.heat {
                    payout::heat_factor(npc.heat)
                } else if npc.streak >= super::STREAK_FOR_MULTIPLIER {
                    (3, 2)
                } else {
                    (1, 1)
                };
                let pts = payout::payout(PayIn {
                    roll: len,
                    jr: false,
                    doubles: doubles && rules.doubles,
                    stretch: stretched,
                    ear: false,
                    factor,
                    call_bonus: 0,
                });
                npc.pot += pts;
                npc.streak += 1;
                npc.heat = (npc.heat + 1).min(payout::MAX_HEAT);
                steps.push(Step::Hit { seat, points: pts, pot: npc.pot });
                let mine = match npc.personality {
                    Personality::Cautious => CAUTIOUS_BANK,
                    Personality::Daring => DARING_BANK,
                    Personality::Mirror => h.last_bank.unwrap_or(CAUTIOUS_BANK),
                };
                let reaches = npc.score + npc.pot >= MARATHON_GOAL;
                if npc.pot >= mine || reaches {
                    let amount = npc.pot;
                    npc.score += amount;
                    npc.pot = 0;
                    npc.heat = 0;
                    npc.banks += 1;
                    npc.largest_banked = npc.largest_banked.max(amount);
                    steps.push(Step::Banked { seat, amount, score: npc.score });
                    if npc.score >= MARATHON_GOAL {
                        self.winner = Some(seat);
                        h.force_end(super::End::Goal);
                        steps.push(Step::Won { seat });
                    }
                    return;
                }
            } else {
                npc.streak = 0;
                npc.heat = 0;
                if npc.pot > 0 {
                    if npc.ward {
                        npc.ward = false;
                        npc.ward_used = true;
                        steps.push(Step::Warded { seat, pot: npc.pot });
                    } else {
                        let lost = npc.pot;
                        npc.pot = 0;
                        npc.busts += 1;
                        npc.largest_lost = npc.largest_lost.max(lost);
                        steps.push(Step::Bust { seat, lost });
                    }
                } else {
                    steps.push(Step::Bust { seat, lost: 0 });
                }
                return; // a miss ends the turn
            }
        }
    }

    /// Jr's racer: one roll, points straight to its score, first to 50 wins.
    fn jr_turn(&mut self, h: &mut Session, seat: usize, i: usize, acc: i64, steps: &mut Vec<Step>) {
        let Some((len, doubles)) = h.npc_roll() else {
            h.force_end(super::End::Exhausted);
            let best = (0..self.seats()).max_by_key(|s| self.score_of(h, *s)).unwrap_or(0);
            self.winner = Some(best);
            return;
        };
        self.npcs[i].rolls += 1;
        steps.push(Step::Rolled { seat, len, doubles, stretched: false });
        let ok = self.chance(acc);
        let pts = if ok {
            payout::payout(PayIn {
                roll: len,
                jr: true,
                doubles: doubles && h.rules.doubles,
                stretch: false,
                ear: false,
                factor: (1, 1),
                call_bonus: 0,
            })
        } else {
            0
        };
        let npc = &mut self.npcs[i];
        npc.score += pts;
        steps.push(Step::Raced { seat, points: pts, score: npc.score });
        if npc.score >= SPRINT_GOAL {
            self.winner = Some(seat);
            h.force_end(super::End::Goal);
            steps.push(Step::Won { seat });
        }
    }

    /// Mirror's threshold, for the screen's "banks at" caption.
    pub fn threshold_of(&self, h: &Session, npc_index: usize) -> u32 {
        self.bank_at(h, self.npcs[npc_index].personality)
    }
}
