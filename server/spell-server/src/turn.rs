//! CC-ONLINE-TURN Phase A: the turn engine, a `ModeRules` for 2-4 seats that
//! owns seat order, the active seat and the phase, so a game mode is only its
//! rules (§5.3). A rules module never touches a socket, a timer or a seat.
//!
//! Hard timers, disconnect grace and NPC conversion arrive in Phase B; until
//! then `deadline_ms` is absent and a dropped seat simply holds up its turn.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::time::Instant;

use crate::mode::{AudioKey, Ctx, Event, ModeRules};
use crate::protocol::ServerMsg;

/// D-O15: the one place the seat limit lives.
pub const MAX_SEATS: u8 = 4;
/// D-O19: fewer humans than this cannot start.
pub const MIN_SEATS: u8 = 2;

pub const AWAITING_ROLL: &str = "awaiting_roll";
pub const AWAITING_SPELL: &str = "awaiting_spell";
pub const AWAITING_CHOICE: &str = "awaiting_choice";
pub const ENDED: &str = "ended";

/// What a seat is asked to do next. `payload` is public to every seat, so it
/// must never hold a target spelling (I4).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub phase: &'static str,
    pub payload: Value,
    /// The word to voice for an `awaiting_spell` phase.
    pub audio: Option<AudioKey>,
}

/// The result of a spelling.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub word: String,
    pub correct: bool,
    pub effects: Vec<Value>,
    /// Another phase for this seat; `None` ends the turn.
    pub next: Option<Step>,
}

/// The result of a choice or an automatic resolution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub effects: Vec<Value>,
    pub next: Option<Step>,
}

/// §5.3: a mode's rules. `Self` is the game state.
pub trait TurnRules: Sized + Send {
    fn init(seats: &[u8], seed: u64) -> Self;
    fn on_turn_begin(&mut self, seat: u8) -> Step;
    /// The seat tapped to roll, in `awaiting_roll`. The server does the rolling.
    fn on_roll(&mut self, seat: u8) -> Step;
    fn on_answer(&mut self, seat: u8, answer: &str) -> Verdict;
    /// `None` refuses a choice that is not legal right now.
    fn on_choice(&mut self, seat: u8, choice: &str) -> Option<Outcome>;
    /// A timeout or disconnect resolved this phase.
    fn on_auto(&mut self, seat: u8, phase: &str) -> Outcome;
    /// Two automatic resolutions in a row.
    fn on_abandon(&mut self, seat: u8) -> Vec<Value>;
    fn is_over(&self) -> bool;
    fn ranking(&self) -> Vec<u8>;
    /// The whole state, secrets included. Server side only.
    fn snapshot(&self) -> Vec<u8>;
    fn restore(bytes: &[u8]) -> Option<Self>;
    /// What every seat may see. Never a secret (F7, I4).
    fn view(&self) -> Value;
}

pub struct TurnEngine<R: TurnRules> {
    rules: Option<R>,
    fixed_seed: Option<u64>,
    seats: Vec<u8>,
    active: usize,
    phase: &'static str,
    /// The live word's ticket, and whether its audio is ready.
    ticket: Option<(String, bool)>,
    running: bool,
}

impl<R: TurnRules> Default for TurnEngine<R> {
    fn default() -> Self {
        TurnEngine { rules: None, fixed_seed: None, seats: Vec::new(), active: 0, phase: ENDED, ticket: None, running: false }
    }
}

impl<R: TurnRules> TurnEngine<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// A reproducible game (I6, tests).
    pub fn with_seed(seed: u64) -> Self {
        TurnEngine { fixed_seed: Some(seed), ..Self::default() }
    }

    fn active_seat(&self) -> u8 {
        self.seats[self.active]
    }

    fn rules(&mut self) -> &mut R {
        self.rules.as_mut().expect("rules exist while running")
    }

    /// Refuses a client message without touching the game (A3).
    fn bad_phase(&self, seat: u8) -> Vec<Event> {
        tracing::info!(event = "bad_phase", seat, phase = self.phase);
        vec![Event::seat(seat, ServerMsg::error("bad_phase"))]
    }

    fn apply_step(&mut self, step: Step) -> Vec<Event> {
        let seat = self.active_seat();
        self.phase = step.phase;
        let mut ev = vec![Event::all(ServerMsg::TurnBegin {
            seat,
            phase: step.phase.to_string(),
            payload: step.payload,
            deadline_ms: None,
        })];
        if let Some(key) = step.audio {
            let ticket = crate::h2h::ticket();
            self.ticket = Some((ticket.clone(), false));
            ev.push(Event::FetchAudio { ticket, key });
        }
        ev
    }

    fn expire_ticket(&mut self) -> Vec<Event> {
        match self.ticket.take() {
            Some((t, _)) => vec![Event::ExpireAudio { tickets: vec![t] }],
            None => Vec::new(),
        }
    }

    /// `next` continues this seat's turn; `None` ends it.
    fn after(&mut self, next: Option<Step>) -> Vec<Event> {
        if let Some(step) = next {
            return self.apply_step(step);
        }
        if self.rules().is_over() {
            self.running = false;
            self.phase = ENDED;
            let ranking = self.rules().ranking();
            let summary = self.rules().view();
            return vec![Event::all(ServerMsg::GameEnd { ranking, summary }), Event::Over];
        }
        self.active = (self.active + 1) % self.seats.len();
        let seat = self.active_seat();
        let step = self.rules().on_turn_begin(seat);
        self.apply_step(step)
    }

    fn public_state(&self) -> Option<Value> {
        let rules = self.rules.as_ref()?;
        Some(json!({
            "phase": self.phase,
            "active_seat": self.seats.get(self.active),
            "audio_ticket": self.ticket.as_ref().filter(|t| t.1).map(|t| t.0.clone()),
            "view": rules.view(),
        }))
    }
}

/// F4: the content hash a client echoes to confirm it is in sync.
pub fn content_hash(state: &Value) -> String {
    hex::encode(Sha256::digest(state.to_string().as_bytes()))
}

impl<R: TurnRules + 'static> ModeRules for TurnEngine<R> {
    fn max_seats(&self) -> u8 {
        MAX_SEATS
    }
    fn min_seats(&self) -> u8 {
        MIN_SEATS
    }
    fn running(&self) -> bool {
        self.running
    }

    fn on_start(&mut self, ctx: &Ctx) -> Vec<Event> {
        let seed = self.fixed_seed.unwrap_or_else(|| {
            let mut b = [0u8; 8];
            getrandom::fill(&mut b).expect("OS randomness");
            u64::from_le_bytes(b)
        });
        tracing::info!(event = "game_started", seed);
        self.seats = ctx.seats.to_vec();
        self.active = 0;
        self.ticket = None;
        self.running = true;
        let mut rules = R::init(&self.seats, seed);
        let step = rules.on_turn_begin(self.seats[0]);
        self.rules = Some(rules);
        self.apply_step(step)
    }

    /// The tap that rolls. The client names no value (D-O1).
    fn on_next(&mut self, seat: u8, _ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_ROLL {
            return self.bad_phase(seat);
        }
        let step = self.rules().on_roll(seat);
        self.apply_step(step)
    }

    fn on_submit(&mut self, seat: u8, _typed: &str, _ctx: &Ctx) -> Vec<Event> {
        self.bad_phase(seat)
    }

    fn on_answer(&mut self, seat: u8, answer: &str, _ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_SPELL {
            return self.bad_phase(seat);
        }
        let v = self.rules().on_answer(seat, answer);
        let mut ev = self.expire_ticket();
        ev.push(Event::all(ServerMsg::TurnVerdict {
            seat,
            word: v.word,
            answer: answer.to_string(),
            correct: v.correct,
            effects: v.effects,
        }));
        ev.extend(self.after(v.next));
        ev
    }

    fn on_choice(&mut self, seat: u8, choice: &str, _ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_CHOICE {
            return self.bad_phase(seat);
        }
        let Some(o) = self.rules().on_choice(seat, choice) else { return self.bad_phase(seat) };
        let mut ev = vec![Event::all(ServerMsg::TurnEffects { seat, effects: o.effects })];
        ev.extend(self.after(o.next));
        ev
    }

    fn on_timer(&mut self, _ctx: &Ctx) -> Vec<Event> {
        Vec::new()
    }

    fn on_disconnect(&mut self, _seat: u8, _released: bool, _ctx: &Ctx) -> Vec<Event> {
        Vec::new()
    }

    fn on_audio(&mut self, ticket: &str, ok: bool, _ctx: &Ctx) -> Vec<Event> {
        if !self.running || self.ticket.as_ref().map(|t| t.0.as_str()) != Some(ticket) {
            return Vec::new();
        }
        let seat = self.active_seat();
        if ok {
            if let Some(t) = self.ticket.as_mut() {
                t.1 = true;
            }
            return vec![Event::all(ServerMsg::TurnAudio { seat, audio_ticket: ticket.to_string() })];
        }
        // The word cannot be heard, so it cannot count against the seat as a
        // spelling. The mode's own default decides.
        let phase = self.phase;
        let o = self.rules().on_auto(seat, phase);
        let mut ev = self.expire_ticket();
        ev.push(Event::all(ServerMsg::TurnAuto { seat, reason: "audio_unavailable".into() }));
        ev.extend(self.after(o.next));
        ev
    }

    fn next_deadline(&self) -> Option<Instant> {
        None
    }

    fn snapshot(&self) -> Value {
        if !self.running {
            return Value::Null;
        }
        let Some(mut state) = self.public_state() else { return Value::Null };
        let hash = content_hash(&state);
        state["hash"] = hash.into();
        state
    }

    fn state_hash(&self) -> Option<String> {
        if !self.running {
            return None;
        }
        self.public_state().map(|s| content_hash(&s))
    }

    fn abort(&mut self) {
        self.running = false;
        self.phase = ENDED;
        self.ticket = None;
    }
}

/// A rules module with no game in it: every seat rolls and spells a fixed
/// number of words, and a roll of the seed decides the word. CI only.
#[cfg(any(test, feature = "test-modes"))]
pub mod null {
    use super::*;
    use crate::protocol::Variant;
    use serde::{Deserialize, Serialize};

    pub const MODE_VERSION: u32 = 1;
    pub const ROUNDS: u32 = 2;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct NullRules {
        seed: u64,
        draws: u64,
        word: Option<String>,
        scores: Vec<(u8, u32)>,
        turns: u32,
    }

    impl NullRules {
        fn draw(&mut self) -> u64 {
            self.draws += 1;
            self.seed.wrapping_add(self.draws).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40
        }
    }

    impl TurnRules for NullRules {
        fn init(seats: &[u8], seed: u64) -> Self {
            NullRules { seed, draws: 0, word: None, scores: seats.iter().map(|s| (*s, 0)).collect(), turns: 0 }
        }
        fn on_turn_begin(&mut self, _seat: u8) -> Step {
            Step { phase: AWAITING_ROLL, payload: Value::Null, audio: None }
        }
        fn on_roll(&mut self, _seat: u8) -> Step {
            let roll = self.draw() % 6 + 1;
            let word = format!("secret{}", self.draw());
            self.word = Some(word.clone());
            Step {
                phase: AWAITING_SPELL,
                payload: json!({ "roll": roll }),
                audio: Some(AudioKey { lang: "en".into(), text: word, py: None, variant: Variant::Normal }),
            }
        }
        fn on_answer(&mut self, seat: u8, answer: &str) -> Verdict {
            let word = self.word.take().unwrap_or_default();
            let correct = answer == word;
            if correct {
                if let Some(e) = self.scores.iter_mut().find(|e| e.0 == seat) {
                    e.1 += 1;
                }
            }
            if correct {
                return Verdict {
                    word,
                    correct,
                    effects: vec![json!({ "scored": seat })],
                    next: Some(Step { phase: AWAITING_CHOICE, payload: json!({ "kind": "again" }), audio: None }),
                };
            }
            self.turns += 1;
            Verdict { word, correct, effects: Vec::new(), next: None }
        }
        fn on_choice(&mut self, _seat: u8, choice: &str) -> Option<Outcome> {
            match choice {
                "stop" => {
                    self.turns += 1;
                    Some(Outcome { effects: vec![json!({ "stopped": true })], next: None })
                }
                _ => None,
            }
        }
        fn on_auto(&mut self, _seat: u8, _phase: &str) -> Outcome {
            self.word = None;
            self.turns += 1;
            Outcome::default()
        }
        fn on_abandon(&mut self, seat: u8) -> Vec<Value> {
            vec![json!({ "abandoned": seat })]
        }
        fn is_over(&self) -> bool {
            self.turns >= ROUNDS * self.scores.len() as u32
        }
        fn ranking(&self) -> Vec<u8> {
            let mut s = self.scores.clone();
            s.sort_by_key(|e| std::cmp::Reverse(e.1));
            s.into_iter().map(|e| e.0).collect()
        }
        fn snapshot(&self) -> Vec<u8> {
            serde_json::to_vec(self).expect("state serializes")
        }
        fn restore(bytes: &[u8]) -> Option<Self> {
            serde_json::from_slice(bytes).ok()
        }
        fn view(&self) -> Value {
            json!({ "scores": self.scores, "turns": self.turns })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::null::NullRules;
    use super::*;
    use crate::protocol::encode_server;
    use std::time::Duration;

    struct G {
        e: TurnEngine<NullRules>,
        seats: Vec<u8>,
        now: Instant,
        log: Vec<Event>,
    }

    macro_rules! ctx {
        ($g:expr) => {
            Ctx {
                now: $g.now,
                seats: &$g.seats,
                kid_safe: false,
                grace: [Duration::ZERO; 4],
                connected: [true; 4],
            }
        };
    }

    impl G {
        fn new(seed: u64) -> G {
            let mut g = G { e: TurnEngine::with_seed(seed), seats: vec![0, 1, 2, 3], now: Instant::now(), log: Vec::new() };
            let ev = g.e.on_start(&ctx!(g));
            g.take(ev);
            g
        }
        /// Audio is ready as soon as it is asked for.
        fn take(&mut self, ev: Vec<Event>) {
            let mut queue = ev;
            while !queue.is_empty() {
                let mut next = Vec::new();
                for e in queue {
                    if let Event::FetchAudio { ticket, .. } = &e {
                        let t = ticket.clone();
                        next.extend(self.e.on_audio(&t, true, &ctx!(self)));
                    }
                    self.log.push(e);
                }
                queue = next;
            }
        }
        fn roll(&mut self, seat: u8) {
            let ev = self.e.on_next(seat, &ctx!(self));
            self.take(ev);
        }
        fn answer(&mut self, seat: u8, a: &str) {
            let ev = self.e.on_answer(seat, a, &ctx!(self));
            self.take(ev);
        }
        fn choice(&mut self, seat: u8, c: &str) {
            let ev = self.e.on_choice(seat, c, &ctx!(self));
            self.take(ev);
        }
        fn word(&self) -> String {
            self.log
                .iter()
                .rev()
                .find_map(|e| match e {
                    Event::FetchAudio { key, .. } => Some(key.text.clone()),
                    _ => None,
                })
                .unwrap()
        }
        fn wire(&self) -> Vec<String> {
            self.log
                .iter()
                .filter_map(|e| match e {
                    Event::Send { msg, .. } => Some(encode_server(0, msg)),
                    _ => None,
                })
                .collect()
        }
        /// One turn each: seats 0 and 2 score then stop, the others miss.
        fn play_out(&mut self) {
            while self.e.running() {
                let seat = self.e.active_seat();
                self.roll(seat);
                if seat.is_multiple_of(2) {
                    let w = self.word();
                    self.answer(seat, &w);
                    self.choice(seat, "stop");
                } else {
                    self.answer(seat, "nope");
                }
            }
        }
    }

    /// Tickets are random by design; everything else must repeat.
    fn without_tickets(wire: Vec<String>) -> Vec<String> {
        wire.into_iter().filter(|f| !f.contains("audio_ticket")).collect()
    }

    #[test]
    fn four_seats_play_to_the_end_and_are_ranked() {
        let mut g = G::new(1);
        g.play_out();
        assert!(!g.e.running());
        assert_eq!(g.log.last(), Some(&Event::Over));
        let ranking = g.log.iter().find_map(|e| match e {
            Event::Send { msg: ServerMsg::GameEnd { ranking, .. }, .. } => Some(ranking.clone()),
            _ => None,
        });
        assert_eq!(ranking, Some(vec![0, 2, 1, 3]));
    }

    #[test]
    fn the_same_seed_replays_the_same_game() {
        let (mut a, mut b) = (G::new(42), G::new(42));
        a.play_out();
        b.play_out();
        assert_eq!(without_tickets(a.wire()), without_tickets(b.wire()));
        let mut c = G::new(43);
        c.play_out();
        assert_ne!(without_tickets(a.wire()), without_tickets(c.wire()));
    }

    #[test]
    fn a_tampered_client_changes_nothing() {
        let mut g = G::new(1);
        let before = g.e.state_hash();
        // Not its turn, and the wrong phase for everyone.
        g.answer(1, "anything");
        g.answer(0, "anything");
        g.choice(0, "stop");
        g.roll(2);
        assert_eq!(g.e.state_hash(), before, "refused messages moved the game");
        let refused = g.log.iter().filter(|e| matches!(e, Event::Send { msg: ServerMsg::Error { code }, .. } if code == "bad_phase"));
        assert_eq!(refused.count(), 4);
    }

    #[test]
    fn an_illegal_choice_is_refused() {
        let mut g = G::new(1);
        g.roll(0);
        let w = g.word();
        g.answer(0, &w);
        let before = g.e.state_hash();
        g.choice(0, "steal-everything");
        assert_eq!(g.e.state_hash(), before);
        assert_eq!(g.e.phase, AWAITING_CHOICE);
    }

    #[test]
    fn no_frame_holds_the_word_before_its_verdict() {
        let mut g = G::new(9);
        while g.e.running() {
            let seat = g.e.active_seat();
            let mark = g.log.len();
            g.roll(seat);
            let w = g.word();
            for e in &g.log[mark..] {
                if let Event::Send { msg, .. } = e {
                    assert!(!encode_server(0, msg).contains(&w), "{msg:?} leaks {w}");
                }
            }
            assert!(!g.e.snapshot().to_string().contains(&w), "the snapshot leaks {w}");
            g.answer(seat, "nope");
            let told = g.log[mark..].iter().any(|e| {
                matches!(e, Event::Send { msg: ServerMsg::TurnVerdict { word, .. }, .. } if *word == w)
            });
            assert!(told, "the verdict did not reveal the word");
        }
    }

    #[test]
    fn the_snapshot_carries_a_hash_of_what_seats_see() {
        let mut g = G::new(3);
        g.roll(0);
        let snap = g.e.snapshot();
        assert_eq!(snap["phase"], AWAITING_SPELL);
        assert_eq!(snap["active_seat"], 0);
        assert!(snap["audio_ticket"].is_string(), "a rejoining seat needs the same ticket");
        assert_eq!(snap["hash"].as_str().map(str::to_string), g.e.state_hash());
        let w = g.word();
        g.answer(0, &w);
        assert_ne!(g.e.snapshot()["hash"], snap["hash"]);
    }

    #[test]
    fn rules_restore_from_their_snapshot() {
        let mut g = G::new(5);
        g.roll(0);
        let rules = g.e.rules.as_ref().unwrap();
        assert_eq!(NullRules::restore(&rules.snapshot()).as_ref(), Some(rules));
        assert_eq!(NullRules::restore(b"not a state"), None);
    }

    #[test]
    fn audio_that_never_loads_moves_the_game_on() {
        let mut g = G::new(6);
        let ev = g.e.on_next(0, &ctx!(g));
        let t = ev.iter().find_map(|e| match e {
            Event::FetchAudio { ticket, .. } => Some(ticket.clone()),
            _ => None,
        });
        let ev = g.e.on_audio(&t.unwrap(), false, &ctx!(g));
        assert!(ev.iter().any(|e| matches!(e, Event::Send { msg: ServerMsg::TurnAuto { seat: 0, .. }, .. })));
        assert_eq!(g.e.active_seat(), 1);
    }
}
