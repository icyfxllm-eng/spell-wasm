//! CC-ONLINE-TURN Phase A: the turn engine, a `ModeRules` for 2-4 seats that
//! owns seat order, the active seat and the phase, so a game mode is only its
//! rules (§5.3). A rules module never touches a socket, a timer or a seat.
//!
//! Phase B: every phase has a hard server timer (D-O6, D-O16). A dropped seat
//! keeps its place and its deadline; a timeout resolves the phase without it,
//! and two absent turns in a row hand the seat to the mode (F5, D-O5).

use std::time::Duration;

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

/// F5: how long a dropped seat is expected back.
pub const GRACE: Duration = Duration::from_secs(60);
/// D-O5: turns in a row without the player before `on_abandon`.
pub const AUTOS_TO_ABANDON: u32 = 2;
/// D-O16: no phase may run longer, whatever a mode asks.
pub const MAX_PHASE: Duration = Duration::from_secs(60);
const ROLL_LIMIT: Duration = Duration::from_secs(20);
const CHOICE_LIMIT: Duration = Duration::from_secs(15);
/// A mode gives `awaiting_spell` its solo timer + 5 s; this is for one that does not.
const SPELL_LIMIT: Duration = Duration::from_secs(35);

/// What `on_abandon` turns a seat into (D-O4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abandon {
    Npc,
    Eliminate,
}

/// What a seat is asked to do next. `payload` is public to every seat, so it
/// must never hold a target spelling (I4).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub phase: &'static str,
    pub payload: Value,
    /// The word to voice for an `awaiting_spell` phase.
    pub audio: Option<AudioKey>,
    /// The mode's own limit for this phase. `None` takes the D-O16 default.
    pub limit: Option<Duration>,
}

impl Step {
    pub fn new(phase: &'static str, payload: Value) -> Step {
        Step { phase, payload, audio: None, limit: None }
    }

    fn limit(&self) -> Duration {
        let default = match self.phase {
            AWAITING_ROLL => ROLL_LIMIT,
            AWAITING_CHOICE => CHOICE_LIMIT,
            AWAITING_SPELL => SPELL_LIMIT,
            _ => MAX_PHASE,
        };
        self.limit.unwrap_or(default).min(MAX_PHASE)
    }
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
    const ABANDON: Abandon;
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
    /// A whole turn for a seat the mode now plays itself.
    fn on_npc_turn(&mut self, seat: u8) -> Vec<Value>;
    fn is_over(&self) -> bool;
    fn ranking(&self) -> Vec<u8>;
    /// The whole state, secrets included. Server side only.
    fn snapshot(&self) -> Vec<u8>;
    fn restore(bytes: &[u8]) -> Option<Self>;
    /// What every seat may see. Never a secret (F7, I4).
    fn view(&self) -> Value;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Connected,
    Grace,
    Npc,
    Eliminated,
}

impl Status {
    fn wire(self) -> &'static str {
        match self {
            Status::Connected => "connected",
            Status::Grace => "grace",
            Status::Npc => "npc",
            Status::Eliminated => "eliminated",
        }
    }
    fn is_human(self) -> bool {
        matches!(self, Status::Connected | Status::Grace)
    }
}

struct SeatState {
    seat: u8,
    status: Status,
    /// When the socket dropped. An NPC seat keeps this while its player is away.
    dropped_at: Option<Instant>,
    /// Turns in a row resolved without the player (F5).
    autos: u32,
    /// True for a player who left for good, who cannot come back (O3).
    left: bool,
}

pub struct TurnEngine<R: TurnRules> {
    rules: Option<R>,
    fixed_seed: Option<u64>,
    game_id: String,
    seats: Vec<SeatState>,
    active: usize,
    phase: &'static str,
    /// The live word's ticket, and whether its audio is ready.
    ticket: Option<(String, bool)>,
    /// Server clock is truth (D-O17): `None` only outside a running game.
    deadline: Option<Instant>,
    /// This turn already counted toward `autos`: a roll and a spell that both
    /// time out are one absent turn.
    auto_this_turn: bool,
    running: bool,
}

impl<R: TurnRules> Default for TurnEngine<R> {
    fn default() -> Self {
        TurnEngine {
            rules: None,
            fixed_seed: None,
            game_id: String::new(),
            seats: Vec::new(),
            active: 0,
            phase: ENDED,
            ticket: None,
            deadline: None,
            auto_this_turn: false,
            running: false,
        }
    }
}

impl<R: TurnRules + 'static> TurnEngine<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// A reproducible game (I6, tests).
    pub fn with_seed(seed: u64) -> Self {
        TurnEngine { fixed_seed: Some(seed), ..Self::default() }
    }

    fn active_seat(&self) -> u8 {
        self.seats[self.active].seat
    }

    fn state_of(&mut self, seat: u8) -> Option<&mut SeatState> {
        self.seats.iter_mut().find(|s| s.seat == seat)
    }

    fn rules(&mut self) -> &mut R {
        self.rules.as_mut().expect("rules exist while running")
    }

    /// Refuses a client message without touching the game (A3).
    fn bad_phase(&self, seat: u8) -> Vec<Event> {
        tracing::info!(event = "bad_phase", game_id = %self.game_id, seat, phase = self.phase);
        vec![Event::seat(seat, ServerMsg::error("bad_phase"))]
    }

    /// D-O25. The seed, the event types and the state hash. Never a typed
    /// answer or a word: this log can hold a child's input otherwise.
    fn logged(&self, ev: Vec<Event>) -> Vec<Event> {
        for e in &ev {
            if let Event::Send { msg, .. } = e {
                tracing::info!(event = "turn_event", game_id = %self.game_id, t = %msg.type_name(), hash = ?self.state_hash());
            }
        }
        ev
    }

    fn apply_step(&mut self, step: Step, now: Instant) -> Vec<Event> {
        let seat = self.seats[self.active].seat;
        let limit = step.limit();
        // F5: past grace nobody is coming, so the room does not wait the timer.
        let gone = self.seats[self.active].dropped_at.is_some_and(|t| now.duration_since(t) >= GRACE);
        self.deadline = Some(if gone { now } else { now + limit });
        self.phase = step.phase;
        let mut ev = vec![Event::all(ServerMsg::TurnBegin {
            seat,
            phase: step.phase.to_string(),
            payload: step.payload,
            deadline_ms: Some(limit.as_millis() as u64),
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

    fn finish(&mut self) {
        self.running = false;
        self.phase = ENDED;
        self.deadline = None;
    }

    /// `next` continues this seat's turn; `None` ends it. NPC seats play at
    /// once here; the app paces what it shows (D10).
    fn after(&mut self, next: Option<Step>, now: Instant) -> Vec<Event> {
        if let Some(step) = next {
            return self.apply_step(step, now);
        }
        let mut ev = Vec::new();
        loop {
            if self.rules().is_over() {
                self.finish();
                let ranking = self.rules().ranking();
                let summary = self.rules().view();
                ev.push(Event::all(ServerMsg::GameEnd { ranking, summary }));
                ev.push(Event::Over);
                return ev;
            }
            // D-O18: no human left means no result.
            if !self.seats.iter().any(|s| s.status.is_human()) {
                self.finish();
                ev.push(Event::all(ServerMsg::MatchEnded { reason: "room_empty".into() }));
                ev.push(Event::Over);
                return ev;
            }
            self.active = (self.active + 1) % self.seats.len();
            self.auto_this_turn = false;
            let seat = self.seats[self.active].seat;
            match self.seats[self.active].status {
                Status::Eliminated => {}
                Status::Npc => {
                    let effects = self.rules().on_npc_turn(seat);
                    ev.push(Event::all(ServerMsg::TurnEffects { seat, effects }));
                }
                Status::Connected | Status::Grace => {
                    let step = self.rules().on_turn_begin(seat);
                    ev.extend(self.apply_step(step, now));
                    return ev;
                }
            }
        }
    }

    /// The seat is not coming back to this turn: F5's conversion.
    fn abandon(&mut self, seat: u8) -> Vec<Event> {
        let effects = self.rules().on_abandon(seat);
        let status = match R::ABANDON {
            Abandon::Npc => Status::Npc,
            Abandon::Eliminate => Status::Eliminated,
        };
        if let Some(s) = self.state_of(seat) {
            s.status = status;
        }
        vec![
            Event::all(ServerMsg::TurnEffects { seat, effects }),
            Event::all(ServerMsg::SeatStatus { seat, status: status.wire().into() }),
        ]
    }

    /// Resolves the active phase without its player.
    fn resolve_auto(&mut self, reason: &str, now: Instant) -> Vec<Event> {
        let seat = self.active_seat();
        let phase = self.phase;
        let o = self.rules().on_auto(seat, phase);
        let mut ev = self.expire_ticket();
        ev.push(Event::all(ServerMsg::TurnAuto { seat, reason: reason.into() }));
        if !o.effects.is_empty() {
            ev.push(Event::all(ServerMsg::TurnEffects { seat, effects: o.effects }));
        }
        let first_this_turn = !std::mem::replace(&mut self.auto_this_turn, true);
        let autos = {
            let s = &mut self.seats[self.active];
            if first_this_turn {
                s.autos += 1;
            }
            s.autos
        };
        if autos < AUTOS_TO_ABANDON {
            ev.extend(self.after(o.next, now));
            return ev;
        }
        // Whatever this turn still had resolves too: nobody will play it.
        let mut next = o.next;
        while let Some(step) = next {
            next = self.rules().on_auto(seat, step.phase).next;
        }
        ev.extend(self.expire_ticket());
        ev.extend(self.abandon(seat));
        ev.extend(self.after(None, now));
        ev
    }

    /// A real action from the player ends any run of automatic turns.
    fn acted(&mut self) {
        self.seats[self.active].autos = 0;
    }

    fn public_state(&self) -> Option<Value> {
        let rules = self.rules.as_ref()?;
        let statuses: Vec<Value> = self.seats.iter().map(|s| json!({ "seat": s.seat, "status": s.status.wire() })).collect();
        Some(json!({
            "phase": self.phase,
            "active_seat": self.seats.get(self.active).map(|s| s.seat),
            "audio_ticket": self.ticket.as_ref().filter(|t| t.1).map(|t| t.0.clone()),
            "seats": statuses,
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
    fn keeps_absent_seats(&self) -> bool {
        true
    }

    fn on_start(&mut self, ctx: &Ctx) -> Vec<Event> {
        let seed = self.fixed_seed.unwrap_or_else(|| {
            let mut b = [0u8; 8];
            getrandom::fill(&mut b).expect("OS randomness");
            u64::from_le_bytes(b)
        });
        self.game_id = crate::h2h::ticket();
        tracing::info!(event = "game_started", game_id = %self.game_id, seed);
        self.seats = ctx
            .seats
            .iter()
            .map(|s| SeatState { seat: *s, status: Status::Connected, dropped_at: None, autos: 0, left: false })
            .collect();
        self.active = 0;
        self.ticket = None;
        self.auto_this_turn = false;
        self.running = true;
        let mut rules = R::init(ctx.seats, seed);
        let step = rules.on_turn_begin(ctx.seats[0]);
        self.rules = Some(rules);
        let ev = self.apply_step(step, ctx.now);
        self.logged(ev)
    }

    /// The tap that rolls. The client names no value (D-O1).
    fn on_next(&mut self, seat: u8, ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_ROLL {
            return self.bad_phase(seat);
        }
        self.acted();
        let step = self.rules().on_roll(seat);
        let ev = self.apply_step(step, ctx.now);
        self.logged(ev)
    }

    fn on_submit(&mut self, seat: u8, _typed: &str, _ctx: &Ctx) -> Vec<Event> {
        self.bad_phase(seat)
    }

    fn on_answer(&mut self, seat: u8, answer: &str, ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_SPELL {
            return self.bad_phase(seat);
        }
        self.acted();
        let v = self.rules().on_answer(seat, answer);
        let mut ev = self.expire_ticket();
        ev.push(Event::all(ServerMsg::TurnVerdict {
            seat,
            word: v.word,
            answer: answer.to_string(),
            correct: v.correct,
            effects: v.effects,
        }));
        ev.extend(self.after(v.next, ctx.now));
        self.logged(ev)
    }

    fn on_choice(&mut self, seat: u8, choice: &str, ctx: &Ctx) -> Vec<Event> {
        if !self.running || seat != self.active_seat() || self.phase != AWAITING_CHOICE {
            return self.bad_phase(seat);
        }
        let Some(o) = self.rules().on_choice(seat, choice) else { return self.bad_phase(seat) };
        self.acted();
        let mut ev = vec![Event::all(ServerMsg::TurnEffects { seat, effects: o.effects })];
        ev.extend(self.after(o.next, ctx.now));
        self.logged(ev)
    }

    fn on_timer(&mut self, ctx: &Ctx) -> Vec<Event> {
        if !self.running {
            return Vec::new();
        }
        // G-D17: the seat's network grace, added here so the wire shows the
        // bare deadline.
        let due = self.deadline.map(|d| d + ctx.grace_for(self.active_seat()));
        if due.is_none_or(|d| ctx.now < d) {
            return Vec::new();
        }
        let ev = self.resolve_auto("timeout", ctx.now);
        self.logged(ev)
    }

    fn on_disconnect(&mut self, seat: u8, released: bool, ctx: &Ctx) -> Vec<Event> {
        if !self.running {
            return Vec::new();
        }
        if !released {
            let Some(s) = self.state_of(seat) else { return Vec::new() };
            s.dropped_at = Some(ctx.now);
            if s.status != Status::Connected {
                return Vec::new();
            }
            s.status = Status::Grace;
            let ev = vec![Event::all(ServerMsg::SeatStatus { seat, status: "grace".into() })];
            return self.logged(ev);
        }
        // Left the room for good: there is no grace to wait out.
        let Some(s) = self.state_of(seat) else { return Vec::new() };
        s.left = true;
        s.dropped_at = Some(ctx.now);
        if !s.status.is_human() {
            return Vec::new();
        }
        let mut ev = vec![Event::all(ServerMsg::SeatStatus { seat, status: "left".into() })];
        if seat == self.active_seat() {
            let phase = self.phase;
            let mut next = self.rules().on_auto(seat, phase).next;
            while let Some(step) = next {
                next = self.rules().on_auto(seat, step.phase).next;
            }
            ev.extend(self.expire_ticket());
            ev.push(Event::all(ServerMsg::TurnAuto { seat, reason: "left".into() }));
            ev.extend(self.abandon(seat));
            ev.extend(self.after(None, ctx.now));
        } else {
            ev.extend(self.abandon(seat));
            // The last human may have gone while another seat was active.
            if !self.seats.iter().any(|s| s.status.is_human()) {
                ev.extend(self.expire_ticket());
                ev.extend(self.after(None, ctx.now));
            }
        }
        self.logged(ev)
    }

    fn on_reconnect(&mut self, seat: u8, _ctx: &Ctx) -> Vec<Event> {
        if !self.running {
            return Vec::new();
        }
        let Some(s) = self.state_of(seat) else { return Vec::new() };
        if s.left || s.status == Status::Eliminated {
            return Vec::new();
        }
        // O3: back from NPC too, while the game lasts; its NPC turns stand.
        s.dropped_at = None;
        s.autos = 0;
        s.status = Status::Connected;
        let ev = vec![Event::all(ServerMsg::SeatStatus { seat, status: "connected".into() })];
        self.logged(ev)
    }

    fn on_audio(&mut self, ticket: &str, ok: bool, ctx: &Ctx) -> Vec<Event> {
        if !self.running || self.ticket.as_ref().map(|t| t.0.as_str()) != Some(ticket) {
            return Vec::new();
        }
        let seat = self.active_seat();
        if ok {
            if let Some(t) = self.ticket.as_mut() {
                t.1 = true;
            }
            let ev = vec![Event::all(ServerMsg::TurnAudio { seat, audio_ticket: ticket.to_string() })];
            return self.logged(ev);
        }
        // The server failed, not the seat: this never counts toward abandon.
        let phase = self.phase;
        let o = self.rules().on_auto(seat, phase);
        let mut ev = self.expire_ticket();
        ev.push(Event::all(ServerMsg::TurnAuto { seat, reason: "audio_unavailable".into() }));
        ev.extend(self.after(o.next, ctx.now));
        self.logged(ev)
    }

    fn next_deadline(&self) -> Option<Instant> {
        if self.running {
            self.deadline
        } else {
            None
        }
    }

    fn snapshot(&self) -> Value {
        if !self.running {
            return Value::Null;
        }
        let Some(mut state) = self.public_state() else { return Value::Null };
        let hash = content_hash(&state);
        state["hash"] = hash.into();
        // Outside the hash: it changes every moment without the game changing.
        let left = self.deadline.map(|d| d.saturating_duration_since(Instant::now()).as_millis() as u64);
        state["deadline_ms"] = left.into();
        state
    }

    fn state_hash(&self) -> Option<String> {
        if !self.running {
            return None;
        }
        self.public_state().map(|s| content_hash(&s))
    }

    fn abort(&mut self) {
        self.finish();
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
    /// Long enough for A1's 40 s drop to come back to a live word.
    pub const SPELL_SECS: Duration = Duration::from_secs(60);

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
        const ABANDON: Abandon = Abandon::Npc;

        fn init(seats: &[u8], seed: u64) -> Self {
            NullRules { seed, draws: 0, word: None, scores: seats.iter().map(|s| (*s, 0)).collect(), turns: 0 }
        }
        fn on_turn_begin(&mut self, _seat: u8) -> Step {
            Step::new(AWAITING_ROLL, Value::Null)
        }
        fn on_roll(&mut self, _seat: u8) -> Step {
            let roll = self.draw() % 6 + 1;
            let word = format!("secret{}", self.draw());
            self.word = Some(word.clone());
            Step {
                phase: AWAITING_SPELL,
                payload: json!({ "roll": roll }),
                audio: Some(AudioKey { lang: "en".into(), text: word, py: None, variant: Variant::Normal }),
                limit: Some(SPELL_SECS),
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
                    next: Some(Step::new(AWAITING_CHOICE, json!({ "kind": "again" }))),
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
        fn on_auto(&mut self, seat: u8, phase: &str) -> Outcome {
            // A roll is not a decision: the server rolls anyway.
            if phase == AWAITING_ROLL {
                return Outcome { effects: Vec::new(), next: Some(self.on_roll(seat)) };
            }
            self.word = None;
            self.turns += 1;
            Outcome::default()
        }
        fn on_abandon(&mut self, seat: u8) -> Vec<Value> {
            vec![json!({ "abandoned": seat })]
        }
        fn on_npc_turn(&mut self, seat: u8) -> Vec<Value> {
            self.turns += 1;
            vec![json!({ "npc": seat })]
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

    impl G {
        fn at(&mut self, d: Duration) {
            self.now += d;
            let ev = self.e.on_timer(&ctx!(self));
            self.take(ev);
        }
        fn drop_seat(&mut self, seat: u8) {
            let ev = self.e.on_disconnect(seat, false, &ctx!(self));
            self.take(ev);
        }
        fn back(&mut self, seat: u8) {
            let ev = self.e.on_reconnect(seat, &ctx!(self));
            self.take(ev);
        }
        fn leave(&mut self, seat: u8) {
            let ev = self.e.on_disconnect(seat, true, &ctx!(self));
            self.take(ev);
        }
        fn autos(&self, seat: u8) -> usize {
            self.log.iter().filter(|e| matches!(e, Event::Send { msg: ServerMsg::TurnAuto { seat: s, .. }, .. } if *s == seat)).count()
        }
        fn status(&self, seat: u8) -> Option<String> {
            self.log.iter().rev().find_map(|e| match e {
                Event::Send { msg: ServerMsg::SeatStatus { seat: s, status }, .. } if *s == seat => Some(status.clone()),
                _ => None,
            })
        }
    }

    #[test]
    fn a1_back_after_40_s_finds_the_same_word_and_deadline() {
        let mut g = G::new(1);
        g.roll(0);
        let ticket = g.e.snapshot()["audio_ticket"].clone();
        assert!(ticket.is_string());
        let deadline = g.e.next_deadline();
        g.drop_seat(0);
        assert_eq!(g.status(0).as_deref(), Some("grace"));
        g.at(Duration::from_secs(40));
        g.back(0);
        assert_eq!(g.status(0).as_deref(), Some("connected"));
        let snap = g.e.snapshot();
        assert_eq!(snap["phase"], AWAITING_SPELL);
        assert_eq!(snap["audio_ticket"], ticket);
        assert_eq!(g.e.next_deadline(), deadline, "a drop must not move the deadline");
        assert_eq!(g.autos(0), 0);
    }

    #[test]
    fn a2_never_back_two_autos_then_npc_and_the_game_ends() {
        let mut g = G::new(1);
        g.roll(0);
        g.drop_seat(0);
        g.at(null::SPELL_SECS);
        assert_eq!(g.autos(0), 1, "turn.auto at the deadline");
        assert_eq!(g.status(0).as_deref(), Some("grace"));
        for seat in [1, 2, 3] {
            g.roll(seat);
            g.answer(seat, "nope");
        }
        assert_eq!(g.e.active_seat(), 0);
        // Past grace: its next turn resolves at the next tick, not after 20 s.
        g.at(Duration::ZERO);
        assert_eq!(g.autos(0), 2);
        assert_eq!(g.status(0).as_deref(), Some("npc"));
        g.play_out();
        assert!(!g.e.running());
        assert!(g.log.iter().any(|e| matches!(e, Event::Send { msg: ServerMsg::GameEnd { .. }, .. })));
    }

    #[test]
    fn every_phase_has_a_hard_limit() {
        let mut g = G::new(2);
        g.at(Duration::from_secs(19));
        assert_eq!(g.e.phase, AWAITING_ROLL, "not yet");
        g.at(Duration::from_secs(1));
        assert_eq!(g.e.phase, AWAITING_SPELL, "the server rolled anyway");
        g.at(null::SPELL_SECS);
        assert_eq!(g.e.active_seat(), 1, "the word timed out as a miss");
        g.roll(1);
        let w = g.word();
        g.answer(1, &w);
        assert_eq!(g.e.phase, AWAITING_CHOICE);
        g.at(Duration::from_secs(15));
        assert_eq!(g.e.active_seat(), 2, "the choice took its default");
        let wild = Step { limit: Some(Duration::from_secs(600)), ..Step::new(AWAITING_SPELL, Value::Null) };
        assert_eq!(wild.limit(), MAX_PHASE, "D-O16: never past 60 s");
    }

    #[test]
    fn a_slow_roll_and_word_in_one_turn_is_one_absent_turn() {
        let mut g = G::new(2);
        g.at(Duration::from_secs(20));
        g.at(null::SPELL_SECS);
        assert_eq!(g.autos(0), 2);
        assert_ne!(g.status(0).as_deref(), Some("npc"));
    }

    #[test]
    fn acting_resets_the_run_of_autos() {
        let mut g = G::new(2);
        // Turn 1 times out, turn 2 is played: the seat stays human.
        g.at(Duration::from_secs(20));
        g.at(null::SPELL_SECS);
        for seat in [1, 2, 3] {
            g.roll(seat);
            g.answer(seat, "nope");
        }
        g.roll(0);
        g.answer(0, "nope");
        assert_ne!(g.status(0).as_deref(), Some("npc"));
        assert_eq!(g.e.active_seat(), 1);
    }

    #[test]
    fn o3_an_npc_seat_can_come_back_and_its_turns_stand() {
        let mut g = G::new(1);
        g.roll(0);
        g.drop_seat(0);
        g.at(null::SPELL_SECS);
        for seat in [1, 2, 3] {
            g.roll(seat);
            g.answer(seat, "nope");
        }
        g.at(Duration::ZERO);
        assert_eq!(g.status(0).as_deref(), Some("npc"));
        g.back(0);
        assert_eq!(g.status(0).as_deref(), Some("connected"));
        assert!(g.e.running());
    }

    #[test]
    fn leaving_resolves_the_turn_at_once_and_cannot_come_back() {
        let mut g = G::new(1);
        g.roll(0);
        g.leave(0);
        assert_eq!(g.status(0).as_deref(), Some("npc"));
        assert_eq!(g.e.active_seat(), 1, "the room did not wait");
        g.back(0);
        assert_eq!(g.status(0).as_deref(), Some("npc"));
    }

    #[test]
    fn i3_the_game_ends_with_everyone_gone_and_no_result() {
        let mut g = G::new(1);
        for seat in [1, 2, 3, 0] {
            g.leave(seat);
        }
        assert!(!g.e.running());
        assert!(g.log.iter().any(|e| matches!(e, Event::Send { msg: ServerMsg::MatchEnded { .. }, .. })));
        assert!(!g.log.iter().any(|e| matches!(e, Event::Send { msg: ServerMsg::GameEnd { .. }, .. })));
        assert_eq!(g.log.last(), Some(&Event::Over));
    }

    #[test]
    fn a_seat_is_given_its_network_grace_before_the_timeout() {
        let mut g = G::new(1);
        let d = g.e.next_deadline().unwrap();
        g.now = d;
        let ctx = Ctx { now: g.now, seats: &[0, 1, 2, 3], kid_safe: false, grace: [Duration::from_millis(300); 4], connected: [true; 4] };
        assert!(g.e.on_timer(&ctx).is_empty(), "still inside the seat's grace");
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
