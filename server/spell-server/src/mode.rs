//! F7's seam: the turn-based `ModeRules` trait every online mode implements.
//!
//! The room layer (`hub.rs`) knows seats, hosts, joins and resume. It never
//! knows a rule of any game: it forwards `next_word`/`submit`/`replay_audio`
//! and timer ticks here and delivers whatever events come back.

use std::time::Duration;

use serde_json::Value;
use tokio::time::Instant;

use crate::protocol::{CreateRoom, ServerMsg, Variant};

/// What the server asks the TTS backend for (F9). Never sent to a client.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioKey {
    pub lang: String,
    /// The text the backend voices (the hanzi for Mandarin).
    pub text: String,
    /// Mandarin's forced reading (`py`), as the app sends it.
    pub py: Option<String>,
    pub variant: Variant,
}

/// Who an event goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    All,
    Seat(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Send { to: Audience, msg: ServerMsg },
    /// Register an opaque ticket for this audio and fetch it now; the result
    /// comes back through `ModeRules::on_audio`.
    FetchAudio { ticket: String, key: AudioKey },
    /// These tickets' word has resolved: they stay valid 60 s more (F9).
    ExpireAudio { tickets: Vec<String> },
    /// The match is over (normally or not); the room returns to its lobby.
    Over,
}

impl Event {
    pub fn all(msg: ServerMsg) -> Event {
        Event::Send { to: Audience::All, msg }
    }
    pub fn seat(seat: u8, msg: ServerMsg) -> Event {
        Event::Send { to: Audience::Seat(seat), msg }
    }
}

/// What a mode may read about the room. Deliberately small.
pub struct Ctx<'a> {
    pub now: Instant,
    /// Occupied seats, ascending.
    pub seats: &'a [u8],
    pub kid_safe: bool,
    /// G-D17: per-seat deadline grace, `min(median_rtt_last5 / 2, 300 ms)`.
    pub grace: [Duration; 4],
    /// Which seats are connected right now.
    pub connected: [bool; 4],
}

impl Ctx<'_> {
    pub fn grace_for(&self, seat: u8) -> Duration {
        self.grace.get(seat as usize).copied().unwrap_or_default()
    }
}

pub trait ModeRules: Send {
    fn max_seats(&self) -> u8;
    fn min_seats(&self) -> u8;
    /// True between `on_start` and the `Over` event.
    fn running(&self) -> bool;
    fn on_start(&mut self, ctx: &Ctx) -> Vec<Event>;
    fn on_next(&mut self, seat: u8, ctx: &Ctx) -> Vec<Event>;
    fn on_submit(&mut self, seat: u8, typed: &str, ctx: &Ctx) -> Vec<Event>;
    fn on_replay(&mut self, _seat: u8, _variant: Variant, _ctx: &Ctx) -> Vec<Event> {
        Vec::new()
    }
    /// Called when `next_deadline` has passed.
    fn on_timer(&mut self, ctx: &Ctx) -> Vec<Event>;
    /// A seat dropped (`released == false`, seat still held) or was released.
    fn on_disconnect(&mut self, seat: u8, released: bool, ctx: &Ctx) -> Vec<Event>;
    /// A held seat came back.
    fn on_reconnect(&mut self, _seat: u8, _ctx: &Ctx) -> Vec<Event> {
        Vec::new()
    }
    /// A `FetchAudio` finished: the clip is cached (`ok`) or the TTS retries
    /// are spent.
    fn on_audio(&mut self, _ticket: &str, _ok: bool, _ctx: &Ctx) -> Vec<Event> {
        Vec::new()
    }
    fn next_deadline(&self) -> Option<Instant>;
    /// The mode's part of a `room_state` snapshot (F5 resume).
    fn snapshot(&self) -> Value;
    /// Forcibly stop a running match (drain, room close, too few seats).
    fn abort(&mut self);
}

#[derive(Debug, PartialEq, Eq)]
pub enum MakeError {
    /// The mode exists in the spec but not in this build yet.
    Unavailable,
    Unknown,
}

/// The mode registry.
pub fn make(req: &CreateRoom) -> Result<Box<dyn ModeRules>, MakeError> {
    match req.mode.as_str() {
        "h2h" => Ok(Box::new(crate::h2h::H2h::new(req))),
        #[cfg(any(test, feature = "test-modes"))]
        "h2h_fast" => Ok(Box::new(crate::h2h::H2h::fast(req))),
        #[cfg(any(test, feature = "test-modes"))]
        "test" => Ok(Box::new(test_mode::TestMode::new(2))),
        #[cfg(any(test, feature = "test-modes"))]
        "test4" => Ok(Box::new(test_mode::TestMode::new(4))),
        _ => Err(MakeError::Unknown),
    }
}

/// A deliberately trivial turn game for exercising the room layer: seats take
/// turns in order; the active seat's `submit` of "right" scores a point; after
/// `ROUNDS` submits per seat the match ends. It has no relation to any real
/// rule, which is the point: the room tests must pass without one.
#[cfg(any(test, feature = "test-modes"))]
pub mod test_mode {
    use super::*;
    use crate::protocol::Standing;
    use std::time::Duration;

    pub const ROUNDS: u32 = 3;
    pub const TURN_SECS: u64 = 30;

    pub struct TestMode {
        max: u8,
        running: bool,
        order: Vec<u8>,
        active: usize,
        submits: u32,
        score: Vec<(u8, u32)>,
        word_live: bool,
        deadline: Option<Instant>,
        pub graded: u32,
    }

    impl TestMode {
        pub fn new(max: u8) -> TestMode {
            TestMode {
                max,
                running: false,
                order: Vec::new(),
                active: 0,
                submits: 0,
                score: Vec::new(),
                word_live: false,
                deadline: None,
                graded: 0,
            }
        }

        fn active_seat(&self) -> u8 {
            self.order[self.active]
        }

        fn advance(&mut self, now: Instant) -> Vec<Event> {
            let seat = self.active_seat();
            let mut ev = vec![Event::all(ServerMsg::TurnEnd { seat })];
            self.submits += 1;
            self.word_live = false;
            if self.submits >= ROUNDS * self.order.len() as u32 {
                self.running = false;
                self.deadline = None;
                let standings: Vec<Standing> =
                    self.score.iter().map(|(s, b)| Standing { seat: *s, best: *b }).collect();
                ev.push(Event::all(ServerMsg::MatchResult { standings, winner_seat: None }));
                ev.push(Event::Over);
                return ev;
            }
            self.active = (self.active + 1) % self.order.len();
            self.deadline = Some(now + Duration::from_secs(TURN_SECS));
            ev.push(Event::all(ServerMsg::TurnStart {
                seat: self.active_seat(),
                turn_no: self.submits + 1,
                turns_left: 0,
            }));
            ev
        }
    }

    impl ModeRules for TestMode {
        fn max_seats(&self) -> u8 {
            self.max
        }
        fn min_seats(&self) -> u8 {
            2
        }
        fn running(&self) -> bool {
            self.running
        }
        fn on_start(&mut self, ctx: &Ctx) -> Vec<Event> {
            self.running = true;
            self.order = ctx.seats.to_vec();
            self.score = self.order.iter().map(|s| (*s, 0)).collect();
            self.active = 0;
            self.submits = 0;
            self.deadline = Some(ctx.now + Duration::from_secs(TURN_SECS));
            vec![Event::all(ServerMsg::TurnStart { seat: self.active_seat(), turn_no: 1, turns_left: 0 })]
        }
        fn on_next(&mut self, seat: u8, _ctx: &Ctx) -> Vec<Event> {
            if !self.running || seat != self.active_seat() {
                return vec![Event::seat(seat, ServerMsg::error("not_your_turn"))];
            }
            self.word_live = true;
            vec![Event::all(ServerMsg::WordStart {
                seat,
                word_no: self.submits + 1,
                tier: "easy".into(),
                audio_ticket: "test".into(),
                deadline_ms: None,
            })]
        }
        fn on_submit(&mut self, seat: u8, typed: &str, ctx: &Ctx) -> Vec<Event> {
            if !self.running || seat != self.active_seat() || !self.word_live {
                return vec![Event::seat(seat, ServerMsg::error("not_your_turn"))];
            }
            self.graded += 1;
            let correct = typed == "right";
            let entry = self.score.iter_mut().find(|(s, _)| *s == seat).unwrap();
            if correct {
                entry.1 += 1;
            }
            let best = entry.1;
            let mut ev = vec![Event::all(ServerMsg::WordResult { seat, correct, chain: best, best, word: "w".into() })];
            ev.extend(self.advance(ctx.now));
            ev
        }
        fn on_timer(&mut self, ctx: &Ctx) -> Vec<Event> {
            if !self.running {
                return Vec::new();
            }
            self.advance(ctx.now)
        }
        fn on_disconnect(&mut self, seat: u8, released: bool, _ctx: &Ctx) -> Vec<Event> {
            // The room layer ends the match if too few seats remain; a
            // released seat simply leaves the rotation otherwise.
            if released {
                if let Some(pos) = self.order.iter().position(|s| *s == seat) {
                    self.order.remove(pos);
                    if pos < self.active {
                        self.active -= 1;
                    }
                    if self.active >= self.order.len() {
                        self.active = 0;
                    }
                }
            }
            Vec::new()
        }
        fn next_deadline(&self) -> Option<Instant> {
            if self.running {
                self.deadline
            } else {
                None
            }
        }
        fn snapshot(&self) -> Value {
            serde_json::json!({
                "running": self.running,
                "active": self.order.get(self.active),
                "submits": self.submits,
                "score": self.score,
            })
        }
        fn abort(&mut self) {
            self.running = false;
            self.deadline = None;
        }
    }
}
