//! CC-BOARD-GAME v1.1 F1 -- the engine. A seeded, pure reducer.
//!
//! Everything the game decides lives here: the board, the dice, the traps, the
//! NPCs, the word order. The screen layer (`boardgame_screen.rs`) renders state
//! and forwards actions; nothing else decides anything (spec section 0).
//!
//! # Rules of this module (I12, I14)
//!
//! No DOM, no `js_sys`, no `web_sys`, no wall clock, no `Math.random`, no float
//! in anything that picks an outcome. All randomness is one
//! `spelldoku::rng::Rng` (splitmix64, u64 arithmetic) held in the state, so a
//! seed, a config, a set of pools and a list of actions give a bit-identical
//! game on every target (I3). `tests::the_engine_source_has_no_forbidden_tokens`
//! enforces the first half of that by reading these files.
//!
//! # The grader is injected (D19, D21)
//!
//! The engine does not know how a language compares a typed answer to a word.
//! `GameConfig::grader` is a plain function pointer; the screen layer passes
//! `boardgame_grade::grade`, and the tests pass an exact-match stub. That keeps
//! the reducer pure and the same for local and (later) online play.
//!
//! # Phases
//!
//! The spec lists `AwaitRoll -> AwaitSpelling -> Resolving -> AwaitSwitchTarget?
//! -> NextTurn`. `Resolving` and `NextTurn` have no observable state in a
//! reducer that resolves them inside one `apply`, so they are not variants:
//! what happened is in `events`, and the turn has already moved on when `apply`
//! returns.

pub mod board;
pub mod engine;
pub mod golden;
pub mod rules;

#[cfg(test)]
mod balance;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::spelldoku::rng::Rng;

#[allow(unused_imports)]
pub use engine::{apply, applied, new_game};

/// Standard: 22 x 22 grid perimeter. Jr: 11 x 11. (D3, I1)
pub const STANDARD_TILES: usize = 84;
pub const JR_TILES: usize = 40;

/// F9: the unique words each tier of the mode's pool must supply. The engine
/// enforces it as well as the screen (I8): a pool that cannot serve the game is
/// refused rather than recycled.
pub const MIN_POOL_STANDARD: usize = 150;
pub const MIN_POOL_JR: usize = 60;
/// D25: the Long Word band's floor.
pub const LONG_WORD_FLOOR: usize = 20;

/// The four bank tiers, in the ladder's order. Names come from
/// `consts::TIER_ORDER` and nowhere else (F2; `tier-list-check.mjs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert];

    pub fn ix(self) -> usize {
        self as usize
    }

    /// The bank's own name for this tier (what `words::tier_for` takes).
    pub fn name(self) -> &'static str {
        crate::consts::TIER_ORDER[self as usize]
    }
}

/// F4's five kinds. Standard only; Jr's generator never names this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trap {
    BackToStart,
    LongWord,
    LoseRoll,
    SwitchTiles,
    DoubleExpert,
}

impl Trap {
    /// O1 default: uniform over the five.
    pub const ALL: [Trap; 5] = [Trap::BackToStart, Trap::LongWord, Trap::LoseRoll, Trap::SwitchTiles, Trap::DoubleExpert];

    /// The locale key suffix for the trap's name (`bg.trap.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            Trap::BackToStart => "start",
            Trap::LongWord => "long",
            Trap::LoseRoll => "lose",
            Trap::SwitchTiles => "switch",
            Trap::DoubleExpert => "double",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Standard,
    Jr,
}

/// D18 / F5. Standard only; Jr has one fixed delta and shows no setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Tough,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Tough];
}

/// One piece on the board. `piece` is the icon index (D23); the seat is the
/// index in `GameConfig::seats`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub piece: u8,
    pub npc: bool,
}

pub const MAX_SEATS: usize = 4;

/// `(language, kid, typed, word, tier) -> correct`. Pure and offline (D21).
pub type Grader = fn(&str, bool, &str, &str, Tier) -> bool;

#[derive(Clone)]
pub struct GameConfig {
    pub variant: Variant,
    pub difficulty: Difficulty,
    pub lang: String,
    pub kid: bool,
    pub seats: Vec<Seat>,
    pub grader: Grader,
}

// A function pointer has no stable identity to compare or print.
impl PartialEq for GameConfig {
    fn eq(&self, o: &Self) -> bool {
        self.variant == o.variant
            && self.difficulty == o.difficulty
            && self.lang == o.lang
            && self.kid == o.kid
            && self.seats == o.seats
    }
}

impl std::fmt::Debug for GameConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameConfig")
            .field("variant", &self.variant)
            .field("difficulty", &self.difficulty)
            .field("lang", &self.lang)
            .field("kid", &self.kid)
            .field("seats", &self.seats)
            .finish()
    }
}

impl GameConfig {
    /// D8: the human plus 1-3 NPCs. NPCs take the pieces the human did not pick.
    pub fn solo(variant: Variant, difficulty: Difficulty, npcs: u8, human_piece: u8, lang: &str, kid: bool, grader: Grader) -> Self {
        let mut seats = vec![Seat { piece: human_piece, npc: false }];
        let mut p = 0u8;
        while seats.len() < 1 + npcs as usize {
            if p != human_piece {
                seats.push(Seat { piece: p, npc: true });
            }
            p += 1;
        }
        GameConfig { variant, difficulty, lang: lang.to_string(), kid, seats, grader }
    }

    /// F7: 2-4 humans on one phone, pieces in seat order.
    pub fn pass_and_play(variant: Variant, humans: u8, lang: &str, kid: bool, grader: Grader) -> Self {
        let seats = (0..humans).map(|piece| Seat { piece, npc: false }).collect();
        GameConfig { variant, difficulty: Difficulty::Normal, lang: lang.to_string(), kid, seats, grader }
    }
}

/// D19 / D25: what the screen layer hands the engine. The engine never calls a
/// bank. Each list is already filtered (ledger, kid) and in the screen's order;
/// the engine shuffles it from the seed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TierPools {
    pub tiers: [Vec<String>; 4],
    /// The Long Word band (D25), computed once by the pool builder. The engine
    /// does no length math.
    pub long_word: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Fewer than 2 or more than 4 seats.
    SeatCount,
    /// Solo is one human plus NPCs; pass-and-play has no NPC (D8, F7).
    MixedSeats,
    /// Two seats on one piece, or a piece outside 0..4.
    BadPiece,
    /// I8: this tier cannot serve the required unique-word count.
    PoolTooSmall(Tier),
    /// D25's floor: fewer than 20 Long Words.
    LongPoolTooSmall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    AwaitRoll,
    AwaitSpelling,
    AwaitSwitchTarget,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Roll,
    SubmitSpelling(String),
    /// A seat to swap with, or `None` for nobody (O2).
    ChooseSwitchTarget(Option<u8>),
    /// No state effect, ever. Present so a client can send it without the
    /// engine having to know what an animation is.
    SkipAnimation,
    /// Play the current NPC seat's whole turn.
    AdvanceNpc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    GameOver,
    /// The action does not belong to the current phase.
    WrongPhase,
    /// A human action on an NPC seat's turn.
    NotHumanTurn,
    /// `AdvanceNpc` on a human seat's turn.
    NotNpcTurn,
    /// A switch target that is not another seat.
    BadTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellKind {
    /// The word that takes the move.
    Landing,
    LongWord,
    /// `step` 0 then 1; both required.
    DoubleExpert,
}

/// The word the current player owes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spell {
    pub kind: SpellKind,
    pub tier: Tier,
    pub word: String,
    /// Landing: the tile a correct answer moves to. Traps: the trap's tile.
    pub dest: u32,
    pub step: u8,
}

/// What happened, in order. The screen turns these into chips and animation;
/// nothing in the engine reads them back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Rolled { seat: u8, roll: u8, dest: u32 },
    Moved { seat: u8, from: u32, to: u32 },
    /// `word` is `Some` for a human (shown only to that player, F7) and `None`
    /// for an NPC, which never has a word (I11).
    Missed { seat: u8, at: u32, word: Option<String> },
    TrapHit { seat: u8, tile: u32, trap: Trap },
    TrapSpellOk { seat: u8, trap: Trap },
    Teleported { seat: u8, from: u32, to: u32 },
    SkipSet { seat: u8 },
    Skipped { seat: u8 },
    Swapped { a: u8, b: u8 },
    SwitchDeclined { seat: u8 },
    Finished { seat: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub seat: Seat,
    pub pos: u32,
    pub skip: bool,
    pub rolls: u32,
    /// Landing spells only (this is what D13's running accuracy counts).
    pub attempts: u32,
    pub hits: u32,
    pub spelled: Vec<String>,
    pub missed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// Index 0 (Start) and the last tile (Finish) are `None`.
    pub tiers: Vec<Option<Tier>>,
    /// Empty for Jr: there is no trap table to index (I2).
    pub traps: Vec<Option<Trap>>,
    /// Cells per side of the square the ring is the perimeter of.
    pub grid: u32,
}

impl Board {
    pub fn len(&self) -> usize {
        self.tiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiers.is_empty()
    }

    pub fn last(&self) -> u32 {
        self.tiers.len() as u32 - 1
    }

    pub fn trap_at(&self, tile: u32) -> Option<Trap> {
        self.traps.get(tile as usize).copied().flatten()
    }

    pub fn trap_count(&self) -> usize {
        self.traps.iter().flatten().count()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoardGameState {
    pub cfg: GameConfig,
    pub board: Board,
    pub players: Vec<Player>,
    /// Seat ids in turn order. Immutable after turn 1 (I7).
    pub order: Vec<u8>,
    /// Index into `order`.
    pub turn: usize,
    pub phase: Phase,
    pub pending: Option<Spell>,
    pub(crate) rng: Rng,
    pub(crate) pools: Arc<TierPools>,
    /// Indices into `pools`, popped from the back.
    pub(crate) queues: [Vec<u32>; 4],
    pub(crate) long_queue: Vec<u32>,
    /// I5: every word drawn this game.
    pub used: BTreeSet<String>,
    /// Draws that had to reuse a word because a queue ran dry. Zero in every
    /// game the bank gate lets start; a test holds it there.
    pub recycled: u32,
    /// Every word drawn, in order, with the tier it was drawn for (a Long Word
    /// counts as Expert). What the ledger records (D19).
    pub drawn: Vec<(Tier, String)>,
    /// Tiles whose trap has been triggered (F4: shown on the map from then on).
    pub revealed: Vec<u32>,
    pub events: Vec<Event>,
    pub winner: Option<u8>,
    /// Turns taken including skipped ones, as the calibration sim counts them.
    pub turns: u32,
}

impl BoardGameState {
    pub fn current_seat(&self) -> u8 {
        self.order[self.turn]
    }

    pub fn is_npc_turn(&self) -> bool {
        self.phase != Phase::Finished && self.players[self.current_seat() as usize].seat.npc
    }

    /// D13/D27: `(3 + hits) / (2 + attempts)` of the human, in thousandths. It
    /// starts at 1.5 and the NPC clamp holds it to 0.95 until attempts accumulate. In
    /// solo there is exactly one human; in pass-and-play nothing reads it.
    pub fn human_acc_milli(&self) -> i64 {
        let (h, a) = self
            .players
            .iter()
            .find(|p| !p.seat.npc)
            .map(|p| (p.hits as i64, p.attempts as i64))
            .unwrap_or((0, 0));
        (3 + h) * 1000 / (2 + a)
    }

    /// The seats from first to last: a finisher first, then by position
    /// (descending), ties by seat.
    pub fn standings(&self) -> Vec<u8> {
        let mut v: Vec<u8> = (0..self.players.len() as u8).collect();
        v.sort_by(|&a, &b| {
            let (pa, pb) = (&self.players[a as usize], &self.players[b as usize]);
            let (fa, fb) = (self.winner == Some(a), self.winner == Some(b));
            fb.cmp(&fa).then(pb.pos.cmp(&pa.pos)).then(a.cmp(&b))
        });
        v
    }

    /// A stable 64-bit digest of the whole state, for the cross-target golden
    /// (I3, A15). FNV-1a over a canonical byte string; integers little-endian.
    pub fn digest(&self) -> u64 {
        let mut b: Vec<u8> = Vec::new();
        let push = |b: &mut Vec<u8>, v: u64| b.extend_from_slice(&v.to_le_bytes());
        let pstr = |b: &mut Vec<u8>, s: &str| {
            b.extend_from_slice(&(s.len() as u64).to_le_bytes());
            b.extend_from_slice(s.as_bytes());
        };
        push(&mut b, self.rng_state());
        push(&mut b, self.board.grid as u64);
        for (i, t) in self.board.tiers.iter().enumerate() {
            push(&mut b, t.map(|t| t as u64 + 1).unwrap_or(0));
            push(&mut b, self.board.traps.get(i).copied().flatten().map(|t| t as u64 + 1).unwrap_or(0));
        }
        for p in &self.players {
            push(&mut b, p.seat.piece as u64 | (p.seat.npc as u64) << 8 | (p.skip as u64) << 9);
            push(&mut b, p.pos as u64);
            push(&mut b, p.rolls as u64);
            push(&mut b, p.attempts as u64);
            push(&mut b, p.hits as u64);
            for w in p.spelled.iter().chain(p.missed.iter()) {
                pstr(&mut b, w);
            }
        }
        for s in &self.order {
            push(&mut b, *s as u64);
        }
        push(&mut b, self.turn as u64);
        push(&mut b, self.phase as u64);
        if let Some(sp) = &self.pending {
            push(&mut b, sp.kind as u64 | (sp.tier as u64) << 8 | (sp.step as u64) << 16);
            push(&mut b, sp.dest as u64);
            pstr(&mut b, &sp.word);
        }
        for q in self.queues.iter() {
            push(&mut b, q.len() as u64);
        }
        push(&mut b, self.long_queue.len() as u64);
        for w in &self.used {
            pstr(&mut b, w);
        }
        push(&mut b, self.recycled as u64);
        for r in &self.revealed {
            push(&mut b, *r as u64);
        }
        push(&mut b, self.events.len() as u64);
        push(&mut b, format!("{:?}", self.events).len() as u64);
        for byte in format!("{:?}", self.events).bytes() {
            b.push(byte);
        }
        push(&mut b, self.winner.map(|w| w as u64 + 1).unwrap_or(0));
        push(&mut b, self.turns as u64);
        crate::spelldoku::rng::fnv(&b)
    }

    /// The stream's position, read by cloning it and drawing once. `Rng` keeps
    /// its counter private and this module does not widen it.
    fn rng_state(&self) -> u64 {
        self.rng.clone().next_u64()
    }
}
