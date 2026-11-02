//! CC-BOARD-GAME-ONLINE F1: BoardGame on the turn engine. The rules are the
//! app's own, reached through `native_api`, and this file never decides one
//! (D-B6). It only translates: a tap or an answer becomes an `Action`, what
//! the rules did becomes effects, and each seat sees nothing it should not.
//!
//! The server rebuilds a game from seed + config + action log, the signed
//! route for a snapshot: the rules hold their word queues in private.

use serde::Deserialize;
use serde_json::{json, Value};
use spell_wasm::native_api as core;
use spell_wasm::native_api::{
    apply, new_game, set_npc, supply, Action, BoardGameState, Boost, Difficulty, Event, GameConfig, Ledger, Phase, Seat, Trap, Variant as Board,
};

use crate::h2h::audio_key;
use crate::mode::{MakeError, ModeRules};
use crate::protocol::{CreateRoom, ProfileKind, Variant};
use crate::turn::{Abandon, Outcome, Step, TurnEngine, TurnRules, Verdict, AWAITING_CHOICE, AWAITING_ROLL, AWAITING_SPELL, MAX_SEATS};

pub const MODE_VERSION: u32 = 1;

/// F6's room settings, with D-B8's defaults. D-B9: the three feature switches
/// come from the host's build flags.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Options {
    pub seats: u8,
    pub size: String,
    pub npc: String,
    pub stretch: bool,
    pub boosts: bool,
    pub streak: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { seats: 4, size: "sprint".into(), npc: "normal".into(), stretch: false, boosts: false, streak: false }
    }
}

impl Options {
    fn board(&self) -> Option<Board> {
        match self.size.as_str() {
            "full" => Some(Board::Full),
            "sprint" => Some(Board::Sprint),
            "jr" => Some(Board::Jr),
            _ => None,
        }
    }
    fn difficulty(&self) -> Option<Difficulty> {
        match self.npc.as_str() {
            "easy" => Some(Difficulty::Easy),
            "normal" => Some(Difficulty::Normal),
            "tough" => Some(Difficulty::Tough),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub lang: String,
    pub options: Options,
}

impl Default for Config {
    fn default() -> Self {
        Config { lang: "en".into(), options: Options::default() }
    }
}

/// The registry's way in: refuses settings no game could start on.
pub fn make(req: &CreateRoom) -> Result<Box<dyn ModeRules>, MakeError> {
    let options: Options = match &req.options {
        Some(v) => serde_json::from_value(v.clone()).map_err(|_| MakeError::BadOptions)?,
        None => Options::default(),
    };
    let Some(board) = options.board() else { return Err(MakeError::BadOptions) };
    if options.difficulty().is_none() || !(2..=MAX_SEATS).contains(&options.seats) {
        return Err(MakeError::BadOptions);
    }
    if supply(&req.lang, board, &Ledger::default(), 0, 0).is_err() {
        return Err(MakeError::Unavailable);
    }
    Ok(Box::new(TurnEngine::<BoardRules>::with_config(Config { lang: req.lang.clone(), options })))
}

fn grader(lang: &str, kid: bool, typed: &str, entry: &str, tier: core::Tier) -> bool {
    core::grade(lang, kid, typed, entry, tier)
}

/// One step of a game, so it can be replayed.
#[derive(Clone, Debug, serde::Serialize, Deserialize, PartialEq)]
enum Logged {
    Roll,
    Spell(String),
    Stretch(bool),
    Switch(Option<u8>),
    Npc,
    SetNpc(u8, bool),
}

/// Everything a replay needs.
#[derive(Clone, Debug, serde::Serialize, Deserialize)]
struct Record {
    lang: String,
    seats: u8,
    size: String,
    npc: String,
    stretch: bool,
    boosts: bool,
    streak: bool,
    kid: bool,
    seed: u64,
    humans: Vec<u8>,
    log: Vec<Logged>,
}

pub struct BoardRules {
    state: BoardGameState,
    /// Wire seat number of each rules seat, ascending.
    table: Vec<u8>,
    /// How many of the rules' events have gone out.
    seen: usize,
    record: Record,
}

impl std::fmt::Debug for BoardRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoardRules").field("digest", &self.state.digest()).finish()
    }
}

fn trap_name(t: Trap) -> &'static str {
    match t {
        Trap::BackToStart => "start",
        Trap::LongWord => "long",
        Trap::LoseRoll => "lose",
        Trap::SwitchTiles => "switch",
        Trap::DoubleExpert => "double",
    }
}

fn boost_name(b: Boost) -> &'static str {
    match b {
        Boost::Tailwind => "tailwind",
        Boost::ExtraRoll => "extra_roll",
        Boost::Ward => "ward",
    }
}

fn build(r: &Record) -> Option<(BoardGameState, Vec<u8>)> {
    let options = Options {
        seats: r.seats,
        size: r.size.clone(),
        npc: r.npc.clone(),
        stretch: r.stretch,
        boosts: r.boosts,
        streak: r.streak,
    };
    let (board, difficulty) = (options.board()?, options.difficulty()?);
    // Seats a human took, then NPCs in the free numbers up to the room's size (D-B2).
    let mut table = r.humans.clone();
    let mut free = 0u8;
    while table.len() < r.seats as usize {
        if !table.contains(&free) {
            table.push(free);
        }
        free += 1;
    }
    table.sort_unstable();
    let seats = table.iter().enumerate().map(|(i, s)| Seat { piece: i as u8, npc: !r.humans.contains(s) }).collect();
    let mut pools = supply(&r.lang, board, &Ledger::default(), 0, r.seed).ok()?.pools;
    // A kid at a Standard table gets words a kid may have, as Head-to-Head does (F8).
    if r.kid {
        for t in pools.tiers.iter_mut() {
            t.retain(|w| core::kid_allowed(&r.lang, w));
        }
        pools.long_word.retain(|w| core::kid_allowed(&r.lang, w));
    }
    let cfg = GameConfig::mixed_table(board, difficulty, seats, &r.lang, r.kid, grader)
        .with_stretch(r.stretch)
        .with_boosts(r.boosts)
        .with_streak(r.streak);
    Some((new_game(r.seed, cfg, pools).ok()?, table))
}

fn replay_one(s: &mut BoardGameState, l: &Logged) -> bool {
    match l {
        Logged::Roll => apply(s, Action::Roll).is_ok(),
        Logged::Spell(w) => apply(s, Action::SubmitSpelling(w.clone())).is_ok(),
        Logged::Stretch(b) => apply(s, Action::ChooseStretch(*b)).is_ok(),
        Logged::Switch(t) => apply(s, Action::ChooseSwitchTarget(*t)).is_ok(),
        Logged::Npc => apply(s, Action::AdvanceNpc).is_ok(),
        Logged::SetNpc(seat, npc) => set_npc(s, *seat, *npc).is_ok(),
    }
}

impl BoardRules {
    fn index(&self, seat: u8) -> Option<u8> {
        self.table.iter().position(|s| *s == seat).map(|i| i as u8)
    }
    fn wire(&self, index: u8) -> u8 {
        self.table.get(index as usize).copied().unwrap_or(index)
    }

    fn act(&mut self, l: Logged) -> bool {
        let ok = replay_one(&mut self.state, &l);
        if ok {
            self.record.log.push(l);
        }
        ok
    }

    /// What the rules did since last asked, in wire seats (I4).
    fn drain(&mut self) -> Vec<Value> {
        let events: Vec<Event> = self.state.events[self.seen..].to_vec();
        self.seen = self.state.events.len();
        events
            .into_iter()
            .map(|e| match e {
                Event::Rolled { seat, roll, dest } => json!({ "e": "rolled", "seat": self.wire(seat), "roll": roll, "dest": dest }),
                Event::StreakEarned { seat } => json!({ "e": "streak_earned", "seat": self.wire(seat) }),
                Event::StreakBonusUsed { seat } => json!({ "e": "streak_bonus_used", "seat": self.wire(seat) }),
                Event::BoostHit { seat, tile, boost } => json!({ "e": "boost", "seat": self.wire(seat), "tile": tile, "boost": boost_name(boost) }),
                Event::WardGained { seat } => json!({ "e": "ward_gained", "seat": self.wire(seat) }),
                Event::TrapBlocked { seat, tile, trap } => json!({ "e": "trap_blocked", "seat": self.wire(seat), "tile": tile, "trap": trap_name(trap) }),
                Event::Stretched { seat } => json!({ "e": "stretched", "seat": self.wire(seat) }),
                Event::Moved { seat, from, to } => json!({ "e": "moved", "seat": self.wire(seat), "from": from, "to": to }),
                // The verdict carries the word, to every seat at once (F3).
                Event::Missed { seat, at, .. } => json!({ "e": "missed", "seat": self.wire(seat), "at": at }),
                Event::TrapHit { seat, tile, trap } => json!({ "e": "trap", "seat": self.wire(seat), "tile": tile, "trap": trap_name(trap) }),
                Event::TrapSpellOk { seat, trap } => json!({ "e": "trap_ok", "seat": self.wire(seat), "trap": trap_name(trap) }),
                Event::Teleported { seat, from, to } => json!({ "e": "teleported", "seat": self.wire(seat), "from": from, "to": to }),
                Event::SkipSet { seat } => json!({ "e": "skip_set", "seat": self.wire(seat) }),
                Event::Skipped { seat } => json!({ "e": "skipped", "seat": self.wire(seat) }),
                Event::Swapped { a, b } => json!({ "e": "swapped", "a": self.wire(a), "b": self.wire(b) }),
                Event::SwitchDeclined { seat } => json!({ "e": "switch_declined", "seat": self.wire(seat) }),
                Event::Finished { seat } => json!({ "e": "finished", "seat": self.wire(seat) }),
            })
            .collect()
    }

    /// The next phase of `seat`'s turn; `None` when the turn is over. An Extra
    /// Roll leaves the turn with the same seat, in `AwaitRoll`.
    fn step_now(&self, seat: u8) -> Option<Step> {
        let s = &self.state;
        match s.phase {
            Phase::Finished => None,
            Phase::AwaitStretch => Some(Step::new(AWAITING_CHOICE, json!({ "kind": "stretch" }))),
            Phase::AwaitSwitchTarget => Some(Step::new(AWAITING_CHOICE, json!({ "kind": "switch" }))),
            Phase::AwaitSpelling => {
                let p = s.pending.as_ref()?;
                let letters = core::citation(&p.word).chars().count();
                Some(Step {
                    phase: AWAITING_SPELL,
                    // Public to every seat: the shape of the word, never the word (I2).
                    payload: json!({ "tier": p.tier.name(), "length": letters, "tile": p.dest, "of": s.board.last(), "stretch": p.stretch, "step": p.step }),
                    audio: Some(audio_key(&s.cfg.lang, &p.word, Variant::Normal)),
                    limit: None,
                })
            }
            Phase::AwaitRoll => {
                (self.index(seat) == Some(s.current_seat())).then(|| Step::new(AWAITING_ROLL, json!({ "again": true })))
            }
        }
    }

    fn settled(&mut self, seat: u8) -> Outcome {
        let effects = self.drain();
        Outcome { effects, next: self.step_now(seat) }
    }
}

impl TurnRules for BoardRules {
    const ABANDON: Abandon = Abandon::Npc;
    type Config = Config;

    fn max_seats(config: &Config) -> u8 {
        config.options.seats
    }

    /// D-B11: a Jr room seats kids and its adult host, who needs no admitting.
    fn admits(config: &Config, kind: ProfileKind) -> bool {
        config.options.size != "jr" || kind == ProfileKind::Kid
    }

    fn init(config: &Config, seats: &[u8], kid_safe: bool, seed: u64) -> Option<Self> {
        let o = &config.options;
        let record = Record {
            lang: config.lang.clone(),
            seats: o.seats.max(seats.len() as u8),
            size: o.size.clone(),
            npc: o.npc.clone(),
            stretch: o.stretch,
            boosts: o.boosts,
            streak: o.streak,
            kid: kid_safe,
            seed,
            humans: seats.to_vec(),
            log: Vec::new(),
        };
        let (state, table) = build(&record)?;
        let seen = state.events.len();
        Some(BoardRules { state, table, seen, record })
    }

    fn table(&self, _seats: &[u8]) -> Vec<(u8, bool)> {
        self.table.iter().enumerate().map(|(i, s)| (*s, self.state.players[i].seat.npc)).collect()
    }

    fn current_seat(&self) -> Option<u8> {
        Some(self.wire(self.state.current_seat()))
    }

    fn on_turn_begin(&mut self, _seat: u8) -> Step {
        Step::new(AWAITING_ROLL, Value::Null)
    }

    fn on_roll(&mut self, seat: u8) -> Outcome {
        self.act(Logged::Roll);
        self.settled(seat)
    }

    fn on_answer(&mut self, seat: u8, answer: &str) -> Verdict {
        let s = &self.state;
        let (word, correct) = match &s.pending {
            Some(p) => (core::citation(&p.word).to_string(), core::grade(&s.cfg.lang, s.cfg.kid, answer, &p.word, p.tier)),
            None => (String::new(), false),
        };
        self.act(Logged::Spell(answer.to_string()));
        let o = self.settled(seat);
        Verdict { word, correct, effects: o.effects, next: o.next }
    }

    fn on_choice(&mut self, seat: u8, choice: &str) -> Option<Outcome> {
        let done = match self.state.phase {
            Phase::AwaitStretch => match choice {
                "yes" => self.act(Logged::Stretch(true)),
                "no" => self.act(Logged::Stretch(false)),
                _ => false,
            },
            Phase::AwaitSwitchTarget => {
                if choice == "nobody" {
                    self.act(Logged::Switch(None))
                } else {
                    let target = choice.parse().ok().and_then(|t| self.index(t));
                    target.is_some_and(|t| self.act(Logged::Switch(Some(t))))
                }
            }
            _ => false,
        };
        done.then(|| self.settled(seat))
    }

    /// F3 and D-B5: what a silent seat's phase means. Nothing here is a rule
    /// of its own; each is an answer a player could have given.
    fn on_auto(&mut self, seat: u8, phase: &str) -> Outcome {
        match (phase, self.state.phase) {
            (AWAITING_ROLL, _) => self.act(Logged::Roll),
            (_, Phase::AwaitStretch) => self.act(Logged::Stretch(false)),
            (_, Phase::AwaitSwitchTarget) => self.act(Logged::Switch(None)),
            (_, Phase::AwaitSpelling) => self.act(Logged::Spell(String::new())),
            _ => false,
        };
        self.settled(seat)
    }

    fn on_abandon(&mut self, seat: u8) -> Vec<Value> {
        if let Some(i) = self.index(seat) {
            self.act(Logged::SetNpc(i, true));
        }
        self.drain()
    }

    fn on_return(&mut self, seat: u8) {
        if let Some(i) = self.index(seat) {
            self.act(Logged::SetNpc(i, false));
        }
    }

    fn on_npc_turn(&mut self, _seat: u8) -> Vec<Value> {
        self.act(Logged::Npc);
        self.drain()
    }

    fn is_over(&self) -> bool {
        self.state.phase == Phase::Finished
    }

    fn ranking(&self) -> Vec<u8> {
        self.state.standings().into_iter().map(|i| self.wire(i)).collect()
    }

    /// Seed + config + every action. Server side only: it holds what was typed.
    fn snapshot(&self) -> Vec<u8> {
        serde_json::to_vec(&self.record).expect("record serializes")
    }

    fn restore(bytes: &[u8]) -> Option<Self> {
        let record: Record = serde_json::from_slice(bytes).ok()?;
        let (mut state, table) = build(&record)?;
        for l in &record.log {
            if !replay_one(&mut state, l) {
                return None;
            }
        }
        let seen = state.events.len();
        Some(BoardRules { state, table, seen, record })
    }

    /// F7, minus anything hidden: tile kinds only once revealed, no word.
    fn view(&self) -> Value {
        let s = &self.state;
        let seats: Vec<Value> = s
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                json!({
                    "seat": self.wire(i as u8), "piece": p.seat.piece, "npc": p.seat.npc, "tile": p.pos,
                    "skip": p.skip, "ward": p.ward, "streak": p.streak, "bonus": p.bonus,
                })
            })
            .collect();
        let revealed: Vec<Value> = s
            .revealed
            .iter()
            .map(|t| {
                let trap = s.board.traps.get(*t as usize).copied().flatten().map(trap_name);
                let boost = s.board.boosts.get(*t as usize).copied().flatten().map(boost_name);
                json!({ "tile": t, "trap": trap, "boost": boost })
            })
            .collect();
        let tiers: Vec<Value> = s.board.tiers.iter().map(|t| json!(t.map(|t| t.name()))).collect();
        json!({
            "size": self.record.size,
            "tiers": tiers,
            "revealed": revealed,
            "seats": seats,
            "winner": s.winner.map(|w| self.wire(w)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::{Audience, Ctx};
    use crate::protocol::{encode_server, ServerMsg};
    use std::time::Duration;
    use tokio::time::Instant;
    use crate::mode::Event as Ev;

    fn cfg(seats: u8, stretch: bool, boosts: bool) -> Config {
        Config { lang: "en".into(), options: Options { seats, stretch, boosts, ..Options::default() } }
    }

    struct G {
        e: TurnEngine<BoardRules>,
        humans: Vec<u8>,
        now: Instant,
        log: Vec<Ev>,
    }

    macro_rules! ctx {
        ($g:expr) => {
            Ctx { now: $g.now, seats: &$g.humans, kid_safe: false, grace: [Duration::ZERO; 4], connected: [true; 4] }
        };
    }

    impl G {
        fn new(config: Config, humans: &[u8], seed: u64) -> G {
            let mut e = TurnEngine::<BoardRules>::with_config(config);
            e.set_seed(seed);
            let mut g = G { e, humans: humans.to_vec(), now: Instant::now(), log: Vec::new() };
            let ev = g.e.on_start(&ctx!(g));
            g.take(ev);
            g
        }
        fn take(&mut self, ev: Vec<Ev>) {
            let mut queue = ev;
            while !queue.is_empty() {
                let mut next = Vec::new();
                for e in queue {
                    if let Ev::FetchAudio { ticket, .. } = &e {
                        let t = ticket.clone();
                        next.extend(self.e.on_audio(&t, true, &ctx!(self)));
                    }
                    self.log.push(e);
                }
                queue = next;
            }
        }
        fn rules(&self) -> &BoardRules {
            self.e.rules_ref().unwrap()
        }
        fn phase(&self) -> &'static str {
            self.e.phase_name()
        }
        fn active(&self) -> u8 {
            self.e.active()
        }
        /// One legal move for the active seat. Every third word is wrong.
        fn step(&mut self, k: &mut u32) {
            let seat = self.active();
            let ev = match self.phase() {
                AWAITING_ROLL => self.e.on_next(seat, &ctx!(self)),
                AWAITING_SPELL => {
                    *k += 1;
                    let w = core::citation(&self.rules().state.pending.as_ref().unwrap().word).to_string();
                    let a = if (*k).is_multiple_of(3) { "zzz".to_string() } else { w };
                    self.e.on_answer(seat, &a, &ctx!(self))
                }
                _ => {
                    let c = if self.rules().state.phase == Phase::AwaitStretch { "no" } else { "nobody" };
                    self.e.on_choice(seat, c, &ctx!(self))
                }
            };
            self.take(ev);
        }
        fn play_out(&mut self) {
            let mut k = 0;
            for _ in 0..20_000 {
                if !self.e.running() {
                    return;
                }
                self.step(&mut k);
            }
            panic!("the game did not end");
        }
        fn at(&mut self, d: Duration) {
            self.now += d;
            let ev = self.e.on_timer(&ctx!(self));
            self.take(ev);
        }
        fn sends(&self) -> Vec<String> {
            self.log
                .iter()
                .filter_map(|e| match e {
                    Ev::Send { msg, .. } => Some(encode_server(0, msg)),
                    _ => None,
                })
                .collect()
        }
    }

    fn ticketless(v: Vec<String>) -> Vec<String> {
        v.into_iter().filter(|f| !f.contains("audio_ticket")).collect()
    }

    #[test]
    fn empty_seats_fill_with_npcs_and_the_game_ends_with_a_winner() {
        let mut g = G::new(cfg(4, false, false), &[0, 2], 11);
        let npcs: Vec<bool> = g.rules().state.players.iter().map(|p| p.seat.npc).collect();
        assert_eq!(npcs, vec![false, true, false, true]);
        g.play_out();
        assert!(g.rules().state.winner.is_some());
        assert!(g.log.iter().any(|e| matches!(e, Ev::Send { msg: ServerMsg::GameEnd { .. }, .. })));
    }

    #[test]
    fn a4_replay_from_the_record_is_the_same_game() {
        for seed in [1, 2, 3, 4, 5] {
            let mut g = G::new(cfg(4, true, true), &[0, 1, 3], seed);
            let mut k = 0;
            for _ in 0..40 {
                if g.e.running() {
                    g.step(&mut k);
                }
            }
            let live = g.rules();
            let back = BoardRules::restore(&live.snapshot()).expect("the record replays");
            assert_eq!(back.state.digest(), live.state.digest(), "seed {seed}");
            let mut a = G::new(cfg(4, true, true), &[0, 1, 3], seed);
            let mut b = G::new(cfg(4, true, true), &[0, 1, 3], seed);
            a.play_out();
            b.play_out();
            assert_eq!(ticketless(a.sends()), ticketless(b.sends()), "I1: seed {seed}");
        }
        assert!(BoardRules::restore(b"junk").is_none());
    }

    #[test]
    fn a9_no_frame_or_snapshot_holds_the_word_before_its_verdict() {
        let mut g = G::new(cfg(3, true, true), &[0, 1, 2], 21);
        let mut k = 0;
        for _ in 0..400 {
            if !g.e.running() {
                break;
            }
            if g.phase() == AWAITING_SPELL {
                let w = core::citation(&g.rules().state.pending.as_ref().unwrap().word).to_string();
                let told = g.log.iter().filter_map(|e| match e {
                    Ev::Send { msg, .. } if !matches!(msg, ServerMsg::TurnVerdict { .. }) => Some(encode_server(0, msg)),
                    _ => None,
                });
                for f in told {
                    assert!(!f.contains(&format!("\"{w}\"")), "{f} leaks {w}");
                }
                assert!(!g.e.snapshot().to_string().contains(&format!("\"{w}\"")), "the snapshot leaks {w}");
            }
            g.step(&mut k);
        }
    }

    #[test]
    fn a6_stretch_is_chosen_before_any_word_audio() {
        let mut offers = 0;
        for seed in 0..40 {
            let mut g = G::new(cfg(2, true, false), &[0, 1], seed);
            let mut k = 0;
            while g.e.running() {
                let before = g.log.len();
                let stretch = g.rules().state.phase == Phase::AwaitStretch;
                if stretch {
                    offers += 1;
                    let fetched = g.log.iter().rev().take_while(|e| !matches!(e, Ev::Send { msg: ServerMsg::TurnBegin { phase, .. }, .. } if phase == AWAITING_ROLL)).any(|e| matches!(e, Ev::FetchAudio { .. }));
                    assert!(!fetched, "audio came before the Stretch choice");
                }
                g.step(&mut k);
                assert!(g.log.len() >= before);
            }
        }
        assert!(offers > 0, "no game offered Stretch");
        assert!(saw_word_after_offer(), "Stretch never led to a word");
    }

    fn saw_word_after_offer() -> bool {
        (0..40).any(|seed| {
            let mut g = G::new(cfg(2, true, false), &[0, 1], seed);
            let mut k = 0;
            while g.e.running() {
                if g.rules().state.phase == Phase::AwaitStretch {
                    let seat = g.active();
                    let ev = g.e.on_choice(seat, "yes", &ctx!(g));
                    return ev.iter().any(|e| matches!(e, Ev::FetchAudio { .. }));
                }
                g.step(&mut k);
            }
            false
        })
    }

    /// The first seed and seat that reach a Switch Tiles choice.
    fn at_a_switch() -> G {
        for seed in 0..400 {
            let mut g = G::new(cfg(2, false, false), &[0, 1], seed);
            let mut k = 0;
            while g.e.running() {
                if g.rules().state.phase == Phase::AwaitSwitchTarget {
                    return g;
                }
                g.step(&mut k);
            }
        }
        panic!("no game hit Switch Tiles");
    }

    #[test]
    fn a5_a_silent_switch_resolves_to_nobody_at_15_s() {
        let mut g = at_a_switch();
        let seat = g.active();
        let pos = g.rules().state.players.iter().map(|p| p.pos).collect::<Vec<_>>();
        g.at(Duration::from_secs(14));
        assert_eq!(g.rules().state.phase, Phase::AwaitSwitchTarget);
        g.at(Duration::from_secs(1));
        assert_ne!(g.rules().state.phase, Phase::AwaitSwitchTarget);
        assert_eq!(g.rules().state.players.iter().map(|p| p.pos).collect::<Vec<_>>(), pos, "nobody moved");
        assert!(g.log.iter().any(|e| matches!(e, Ev::Send { msg: ServerMsg::TurnAuto { seat: s, .. }, .. } if *s == seat)));
    }

    #[test]
    fn a_word_times_out_at_35_s_as_a_miss_in_place() {
        let mut g = G::new(cfg(2, false, false), &[0, 1], 3);
        let seat = g.active();
        let ev = g.e.on_next(seat, &ctx!(g));
        g.take(ev);
        assert_eq!(g.phase(), AWAITING_SPELL, "a first roll never reaches the finish");
        let i = g.rules().index(seat).unwrap() as usize;
        let tile = g.rules().state.players[i].pos;
        let ticket = g.e.snapshot()["audio_ticket"].clone();
        // Back within the timer: the same word and the same deadline (A1).
        let deadline = g.e.next_deadline();
        let ev = g.e.on_disconnect(seat, false, &ctx!(g));
        g.take(ev);
        g.at(Duration::from_secs(20));
        let ev = g.e.on_reconnect(seat, &ctx!(g));
        g.take(ev);
        assert_eq!(g.e.snapshot()["audio_ticket"], ticket);
        assert_eq!(g.e.next_deadline(), deadline);
        // Past it: a miss, on the same tile.
        g.at(Duration::from_secs(15));
        assert_ne!(g.active(), seat);
        assert_eq!(g.rules().state.players[i].pos, tile);
        assert_eq!(g.rules().state.players[i].streak, 0);
    }

    #[test]
    fn a2_an_absent_seat_becomes_an_npc_keeping_its_state() {
        let mut g = G::new(cfg(3, false, true), &[0, 1, 2], 8);
        // Give the seat something to keep: a few turns' worth of tile and boosts.
        let mut k = 0;
        for _ in 0..30 {
            g.step(&mut k);
        }
        while g.phase() != AWAITING_ROLL {
            g.step(&mut k);
        }
        let gone = g.active();
        let i = g.rules().index(gone).unwrap() as usize;
        let ev = g.e.on_disconnect(gone, false, &ctx!(g));
        g.take(ev);
        let mut k = 0;
        for _ in 0..2000 {
            if !g.e.running() || g.rules().state.players[i].seat.npc {
                break;
            }
            if g.active() == gone {
                let phase = g.phase();
                let limit = match phase {
                    AWAITING_ROLL => 20,
                    AWAITING_CHOICE => 15,
                    _ => 35,
                };
                g.at(Duration::from_secs(limit));
            } else {
                g.step(&mut k);
            }
        }
        assert!(g.rules().state.players[i].seat.npc, "two absent turns make an NPC");
        assert!(g.log.iter().any(|e| matches!(e, Ev::Send { msg: ServerMsg::SeatStatus { seat, status }, .. } if *seat == gone && status == "npc")));
        // I3: handing the seat over moved nothing. Replaying up to the hand-over gives the same seat.
        let rec = &g.rules().record;
        let cut = rec.log.iter().position(|l| matches!(l, Logged::SetNpc(s, true) if *s == i as u8)).unwrap();
        let mut before = build(rec).unwrap().0;
        for l in &rec.log[..cut] {
            assert!(replay_one(&mut before, l));
        }
        let mut after = before.clone();
        assert!(replay_one(&mut after, &rec.log[cut]));
        let (a, b) = (&before.players[i], &after.players[i]);
        assert_eq!((a.pos, a.skip, a.ward, a.streak, a.bonus), (b.pos, b.skip, b.ward, b.streak, b.bonus));
        g.play_out();
        assert!(!g.e.running(), "the game reached its end");
    }

    #[test]
    fn a3_nothing_a_client_can_send_moves_the_game_out_of_turn() {
        let mut g = G::new(cfg(2, false, false), &[0, 1], 5);
        let cur = g.active();
        let other = if cur == 0 { 1 } else { 0 };
        let before = g.rules().state.digest();
        for ev in [
            g.e.on_next(other, &ctx!(g)),
            g.e.on_answer(cur, "x", &ctx!(g)),
            g.e.on_answer(other, "x", &ctx!(g)),
            g.e.on_choice(cur, "6", &ctx!(g)),
        ] {
            assert!(matches!(&ev[0], Ev::Send { to: Audience::Seat(_), msg: ServerMsg::Error { .. } }));
        }
        assert_eq!(g.rules().state.digest(), before);
    }

    #[test]
    fn bad_room_settings_are_refused_and_jr_takes_only_kids() {
        let req = |options: Value| CreateRoom {
            mode: "boardgame".into(),
            lang: "en".into(),
            tier_mode: crate::protocol::TierMode::Climb,
            tier: None,
            timed: false,
            approve_joins: false,
            options: Some(options),
        };
        for bad in [json!({ "size": "huge" }), json!({ "seats": 9 }), json!({ "npc": "brutal" }), json!({ "x": 1 })] {
            assert_eq!(make(&req(bad.clone())).err(), Some(MakeError::BadOptions), "{bad}");
        }
        let jr = make(&req(json!({ "size": "jr", "seats": 2 }))).unwrap();
        assert!(jr.admits(ProfileKind::Kid));
        assert!(!jr.admits(ProfileKind::Adult), "A8");
        assert_eq!(jr.max_seats(), 2);
        assert!(make(&req(json!({}))).unwrap().admits(ProfileKind::Adult));
    }
}
