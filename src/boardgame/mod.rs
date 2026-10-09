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

pub use rules::{Variant, LONG_WORD_FLOOR};

#[cfg(test)]
mod balance;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::spelldoku::rng::Rng;

#[allow(unused_imports)]
pub use engine::{apply, applied, new_game};

// Tile counts, pool minimums, tier weights, trap rules and the NPC delta ladder
// all live in ONE config block per variant: `rules::VariantCfg` (I-P10). Nothing
// outside it matches on `Variant` to pick a number.

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

/// D18 / F5. Standard only; Jr has one fixed delta and shows no setting.
/// Feature 2: the three boost kinds. Hidden until triggered, then marked (D-P2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boost {
    /// Move forward `tailwind_tiles` more tiles.
    Tailwind,
    /// Roll and spell again this turn.
    ExtraRoll,
    /// Hold one charge that cancels the next trap landed on.
    Ward,
}

impl Boost {
    pub const ALL: [Boost; 3] = [Boost::Tailwind, Boost::ExtraRoll, Boost::Ward];

    /// The locale key suffix (`bg.boost.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            Boost::Tailwind => "tailwind",
            Boost::ExtraRoll => "extra",
            Boost::Ward => "ward",
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

    pub fn ix(self) -> usize {
        self as usize
    }
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
    /// Feature 1 / D-P16: Stretch words. Captured here at `new_game` (the screen reads the
    /// `boardStretch` flag once) so the reducer stays pure and a replay is exact. Off, the
    /// engine is move for move what it was before Stretch existed (A-P9).
    pub stretch: bool,
    /// Feature 2 / D-P16: boost tiles. Captured here like `stretch`. Off, no boost is placed, no
    /// RNG is drawn for them, and the game is move for move what it was before (A-P9).
    pub boosts: bool,
    /// Feature 3 / D-P16: hot streak, captured here too.
    pub streak: bool,
}

// A function pointer has no stable identity to compare or print.
impl PartialEq for GameConfig {
    fn eq(&self, o: &Self) -> bool {
        self.variant == o.variant
            && self.difficulty == o.difficulty
            && self.lang == o.lang
            && self.kid == o.kid
            && self.seats == o.seats
            && self.stretch == o.stretch
            && self.boosts == o.boosts
            && self.streak == o.streak
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
        GameConfig { variant, difficulty, lang: lang.to_string(), kid, seats, grader, stretch: false, boosts: false, streak: false }
    }

    /// Turn Stretch on (the flag) for this game. Ignored by variants that have none (Jr).
    pub fn with_stretch(mut self, on: bool) -> Self {
        self.stretch = on;
        self
    }

    /// Turn boost tiles on for this game (ignored by nothing: every variant has boosts).
    pub fn with_boosts(mut self, on: bool) -> Self {
        self.boosts = on;
        self
    }

    /// Turn the hot streak on for this game.
    pub fn with_streak(mut self, on: bool) -> Self {
        self.streak = on;
        self
    }

    /// F7: 2-4 humans on one phone, pieces in seat order.
    pub fn pass_and_play(variant: Variant, humans: u8, lang: &str, kid: bool, grader: Grader) -> Self {
        let seats = (0..humans).map(|piece| Seat { piece, npc: false }).collect();
        GameConfig { variant, difficulty: Difficulty::Normal, lang: lang.to_string(), kid, seats, grader, stretch: false, boosts: false, streak: false }
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
    /// Feature 1: the roll has landed and the player has not yet chosen the normal move or
    /// Stretch. No word has been drawn (I-P6). Appended last so existing discriminants hold.
    AwaitStretch,
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
    /// Feature 1: commit to Stretch (`true`) or the normal move (`false`). Valid only in
    /// `AwaitStretch`; the word is drawn from the committed tier after this (I-P6).
    ChooseStretch(bool),
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
    /// Feature 1: a Stretch landing word. `dest` already includes the bonus (clamped).
    pub stretch: bool,
}

/// Feature 1: what a human sees between the die and the word. Nothing about either word
/// exists yet, only the two tiers and the two destinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offer {
    pub roll: u8,
    pub normal_dest: u32,
    pub normal_tier: Tier,
    pub stretch_dest: u32,
    pub stretch_tier: Tier,
}

/// What happened, in order. The screen turns these into chips and animation;
/// nothing in the engine reads them back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Rolled { seat: u8, roll: u8, dest: u32 },
    /// Feature 3: the third correct spelling in a row; the next roll taken gets +1.
    StreakEarned { seat: u8 },
    /// Feature 3: the pending bonus went onto the roll that follows.
    StreakBonusUsed { seat: u8 },
    /// Feature 2: a hidden boost was triggered (it stays marked from now on).
    BoostHit { seat: u8, tile: u32, boost: Boost },
    /// Feature 2: Ward collected (a second pickup while holding one changes nothing).
    WardGained { seat: u8 },
    /// Feature 2: a held Ward cancelled the trap on `tile`; the charge is spent.
    TrapBlocked { seat: u8, tile: u32, trap: Trap },
    /// Feature 1: the move that follows (Moved, Missed, trap events) was a Stretch.
    Stretched { seat: u8 },
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
    /// Feature 1 / D-P19: the on-device counters behind the podium line. Stretch spellings are
    /// kept OUT of `attempts`/`hits` (D-P11); these three are their own.
    pub stretch_offered: u32,
    pub stretch_taken: u32,
    pub stretch_hits: u32,
    /// Feature 2: holds a Ward charge (at most one).
    pub ward: bool,
    /// Feature 3: correct spellings in a row, 0..streak_length-1.
    pub streak: u32,
    /// Feature 3: the next roll actually taken gets the streak bonus.
    pub bonus: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// Index 0 (Start) and the last tile (Finish) are `None`.
    pub tiers: Vec<Option<Tier>>,
    /// Empty when the variant has no traps (Jr): there is no trap table to index (I2).
    pub traps: Vec<Option<Trap>>,
    /// Feature 2: empty unless the game has boosts on. Never on tile 0, the finish or a trap tile.
    pub boosts: Vec<Option<Boost>>,
    /// The canonical ring's width (Full 22 of 22x22, Sprint 11 of 11x12, Jr 11 of 11x11).
    /// Presentation picks the drawn shape (`boardgame_ring`); this only feeds the digest.
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

    pub fn boost_at(&self, tile: u32) -> Option<Boost> {
        self.boosts.get(tile as usize).copied().flatten()
    }

    pub fn boost_count(&self) -> usize {
        self.boosts.iter().flatten().count()
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
    /// Feature 1: set only while `phase == AwaitStretch`.
    pub offer: Option<Offer>,
    /// I-P4: a trap or boost has already resolved this turn. Reset when the turn passes; an Extra
    /// Roll keeps the turn, so its second roll resolves no further tile effect.
    pub effect_used: bool,
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

    /// D13/D27/D-P20: `(prior_hits + hits) / (prior_attempts + attempts)` (3 and 2 except Sprint) of the human, in thousandths. It
    /// starts at 1.5 and the NPC clamp holds it to 0.95 until attempts accumulate. In
    /// solo there is exactly one human; in pass-and-play nothing reads it.
    pub fn human_acc_milli(&self) -> i64 {
        let (h, a) = self
            .players
            .iter()
            .find(|p| !p.seat.npc)
            .map(|p| (p.hits as i64, p.attempts as i64))
            .unwrap_or((0, 0));
        let (ph, pa) = self.cfg.variant.cfg().prior();
        (ph + h * 1000) * 1000 / (pa + a * 1000)
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
        // Stretch state joins the digest only in a game that has Stretch on, so a game with the
        // flag off digests exactly as it did before the feature existed (A-P9).
        if self.cfg.stretch {
            push(&mut b, 0x5742_C7);
            if let Some(o) = &self.offer {
                push(&mut b, o.roll as u64 | (o.normal_tier as u64) << 8 | (o.stretch_tier as u64) << 16);
                push(&mut b, o.normal_dest as u64 | (o.stretch_dest as u64) << 32);
            }
            for p in &self.players {
                push(&mut b, p.stretch_offered as u64 | (p.stretch_taken as u64) << 20 | (p.stretch_hits as u64) << 40);
            }
        }
        // Boost and streak state joins the digest only in a game that has either on (A-P9).
        if self.cfg.boosts || self.cfg.streak {
            push(&mut b, 0xB005_7);
            push(&mut b, self.effect_used as u64);
            for (i, bo) in self.board.boosts.iter().enumerate() {
                push(&mut b, bo.map(|x| x as u64 + 1).unwrap_or(0) | (i as u64) << 8);
            }
            for p in &self.players {
                push(&mut b, p.ward as u64 | (p.streak as u64) << 1 | (p.bonus as u64) << 8);
            }
        }
        if let Some(sp) = &self.pending {
            push(&mut b, sp.kind as u64 | (sp.tier as u64) << 8 | (sp.step as u64) << 16 | (sp.stretch as u64) << 24);
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
