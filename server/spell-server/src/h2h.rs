//! F7 — Online Head-to-Head: local Head-to-Head's rules, verbatim.
//!
//! The scoreboard IS the app's `versus::Versus` (start / record_correct /
//! end_turn / winner); this file adds no rule of its own to it. What it adds
//! is only what a second device forces: who may act, when the next word is
//! served, audio tickets, deadlines judged on the server clock, and the two
//! online-only idle rules (spec §F5).
//!
//! | rules-table row | here |
//! |---|---|
//! | 1 two players | `max_seats`/`min_seats` = 2 |
//! | 3, 6-10 turns, chains, winner | `Versus` |
//! | 4 turns each 3, kid-safe 2 | `on_start` (local: `game.rs:4212`) |
//! | 5 one attempt per word | one `submit` resolves the word |
//! | 11 tier | `word_tier` (`tier_for_streak` + `clamp_base_tier`) |
//! | 12 timer | `tier_time`, judged per G-D17 |
//! | 14 no length, no slots | `word_start` carries neither |
//! | 15 correct → next word after 2200 ms, no definition | `AfterCorrect` |
//! | 16 miss → next turn after 1300 ms | `AfterMiss` |
//! | 17 a turn starts on the orb tap | `AwaitNext` + `on_next` (online: 120 s, then the turn is a miss) |

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{json, Value};
use spell_wasm::native_api::{self as core, Side, Versus};
use tokio::time::Instant;

use crate::grader;
use crate::mode::{AudioKey, Ctx, Event, ModeRules};
use crate::protocol::{CreateRoom, ServerMsg, Standing, TierMode, Variant};
use crate::store::random_bytes;

/// §F5: an untimed word with no answer for this long is a miss.
pub const UNTIMED_IDLE: Duration = Duration::from_secs(120);
/// Eric 2026-10-24: a turn whose player never taps for a word is a miss after
/// this long, timed room or not. Online only; local players share one phone.
pub const TURN_START_IDLE: Duration = Duration::from_secs(120);
/// Local: `schedule(app, 1300, ..)` after a miss (game.rs:4349-4351).
pub const MISS_DELAY: Duration = Duration::from_millis(1300);
/// F9: three consecutive paused words end the match.
pub const MAX_PAUSES: u32 = 3;
/// A fetch the audio worker never answers counts as failed after this.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

fn correct_delay() -> Duration {
    Duration::from_millis(core::CORRECT_DELAY_MS as u64)
}

#[derive(Clone, Debug)]
struct Word {
    entry: String,
    tier: String,
    word_no: u32,
    normal: String,
    slow: Option<String>,
}

#[derive(Clone, Debug)]
enum Phase {
    Idle,
    AwaitNext { seat: u8, idle_at: Option<Instant> },
    Fetching { seat: u8, word: Word, since: Instant },
    Live { seat: u8, word: Word, started: Instant, limit: Option<Duration>, idle_at: Option<Instant> },
    AfterCorrect { seat: u8, at: Instant },
    AfterMiss { at: Instant },
}

/// splitmix64, seeded once per match from the OS CSPRNG (F8); replays with
/// the same seed draw the same words (A7.2).
#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

pub struct H2h {
    req: CreateRoom,
    versus: Versus,
    phase: Phase,
    rng: Rng,
    fixed_seed: Option<u64>,
    used: HashSet<String>,
    /// `seats[0]` plays `Side::P1` (the earlier seat, normally the host).
    seats: [u8; 2],
    kid: bool,
    word_no: u32,
    turn_no: u32,
    pauses: u32,
    /// Rules-table rows 15/16. Only the test-only `h2h_fast` mode changes them.
    after_correct: Duration,
    after_miss: Duration,
}

pub fn ticket() -> String {
    hex::encode(random_bytes::<16>())
}

/// The bank entry's typed half (`pinyin|hanzi` → `pinyin`).
pub fn citation(entry: &str) -> &str {
    core::citation(entry)
}

/// The audio request for an entry, exactly as the app makes it (`voice_target`).
pub fn audio_key(lang: &str, entry: &str, variant: Variant) -> AudioKey {
    let mut parts = entry.splitn(2, '|');
    let word = parts.next().unwrap_or(entry);
    let spoken = parts.next().unwrap_or("");
    let (text, py) = core::voice_target(lang, word, spoken);
    AudioKey { lang: lang.to_string(), text, py, variant }
}

impl H2h {
    pub fn new(req: &CreateRoom) -> H2h {
        H2h {
            req: req.clone(),
            versus: Versus::default(),
            phase: Phase::Idle,
            rng: Rng(0),
            fixed_seed: None,
            used: HashSet::new(),
            seats: [0, 1],
            kid: false,
            word_no: 0,
            turn_no: 0,
            pauses: 0,
            after_correct: correct_delay(),
            after_miss: MISS_DELAY,
        }
    }

    /// `h2h_fast` (feature `test-modes` only): the same rules with no hand-over
    /// pauses, so socket tests can play hundreds of words in real time.
    #[cfg(any(test, feature = "test-modes"))]
    pub fn fast(req: &CreateRoom) -> H2h {
        let mut h = H2h::new(req);
        h.after_correct = Duration::ZERO;
        h.after_miss = Duration::ZERO;
        h
    }

    /// Tests and replays: draw words from a known seed.
    pub fn with_seed(req: &CreateRoom, seed: u64) -> H2h {
        let mut h = H2h::new(req);
        h.fixed_seed = Some(seed);
        h
    }

    fn side_seat(&self, side: Side) -> u8 {
        match side {
            Side::P1 => self.seats[0],
            Side::P2 => self.seats[1],
        }
    }

    fn active_seat(&self) -> u8 {
        self.side_seat(self.versus.active)
    }

    /// Rules-table row 11.
    fn word_tier(&self) -> String {
        let asked = match self.req.tier_mode {
            TierMode::Climb => core::tier_for_streak(self.versus.active_player().current).to_string(),
            TierMode::Fixed => self.req.tier.clone().unwrap_or_else(|| "easy".into()),
        };
        core::clamp_base_tier(self.kid, asked)
    }

    /// F8: a word from the gated bank, never repeated in a match. Kid-safe
    /// rooms check every word against the Kid list themselves: the app's
    /// `filter_kid` returns the WHOLE pool when nothing survives, and an
    /// online kid room must never get that fallback.
    fn pick(&mut self, tier: &str) -> Option<String> {
        let pool = core::tier_for(&self.req.lang, tier);
        if pool.is_empty() {
            return None;
        }
        let lang = self.req.lang.clone();
        let ok = |used: &HashSet<String>, w: &str| !used.contains(w) && (!self.kid || core::kid_allowed(&lang, w));
        // Rejection sampling: uniform over the allowed words, and cheap while
        // most of the pool is allowed. A crowded pool falls back to a scan.
        for _ in 0..64 {
            let w = pool[(self.rng.next() % pool.len() as u64) as usize];
            if ok(&self.used, w) {
                self.used.insert(w.to_string());
                return Some(w.to_string());
            }
        }
        let candidates: Vec<&str> = pool.iter().copied().filter(|w| ok(&self.used, w)).collect();
        if candidates.is_empty() {
            return None;
        }
        let w = candidates[(self.rng.next() % candidates.len() as u64) as usize].to_string();
        self.used.insert(w.clone());
        Some(w)
    }

    fn start_word(&mut self, seat: u8, now: Instant) -> Vec<Event> {
        let tier = self.word_tier();
        let Some(entry) = self.pick(&tier) else {
            self.phase = Phase::Idle;
            return vec![Event::all(ServerMsg::MatchEnded { reason: "bank_exhausted".into() }), Event::Over];
        };
        self.word_no += 1;
        let normal = ticket();
        let key = audio_key(&self.req.lang, &entry, Variant::Normal);
        let word = Word { entry, tier, word_no: self.word_no, normal: normal.clone(), slow: None };
        self.phase = Phase::Fetching { seat, word, since: now };
        vec![Event::FetchAudio { ticket: normal, key }]
    }

    fn tickets(word: &Word) -> Vec<String> {
        let mut t = vec![word.normal.clone()];
        t.extend(word.slow.clone());
        t
    }

    fn resolve(&mut self, seat: u8, word: &Word, correct: bool, now: Instant) -> Vec<Event> {
        let mut ev = vec![Event::ExpireAudio { tickets: Self::tickets(word) }];
        if correct {
            self.versus.record_correct();
            let p = self.versus.active_player();
            ev.push(Event::all(ServerMsg::WordResult {
                seat,
                correct: true,
                chain: p.current,
                best: p.best,
                word: word.entry.clone(),
            }));
            self.phase = Phase::AfterCorrect { seat, at: now + self.after_correct };
        } else {
            let best = self.versus.active_player().best;
            self.versus.end_turn();
            ev.push(Event::all(ServerMsg::WordResult { seat, correct: false, chain: 0, best, word: word.entry.clone() }));
            ev.push(Event::all(ServerMsg::TurnEnd { seat }));
            self.phase = Phase::AfterMiss { at: now + self.after_miss };
        }
        ev
    }

    fn begin_turn(&mut self, now: Instant) -> Vec<Event> {
        self.turn_no += 1;
        let seat = self.active_seat();
        self.phase = Phase::AwaitNext { seat, idle_at: Some(now + TURN_START_IDLE) };
        vec![Event::all(ServerMsg::TurnStart {
            seat,
            turn_no: self.turn_no,
            turns_left: self.versus.active_player().turns_left,
        })]
    }

    fn finish(&mut self) -> Vec<Event> {
        self.phase = Phase::Idle;
        let standings = vec![
            Standing { seat: self.seats[0], best: self.versus.p1.best },
            Standing { seat: self.seats[1], best: self.versus.p2.best },
        ];
        let winner_seat = self.versus.winner().map(|s| self.side_seat(s));
        vec![Event::all(ServerMsg::MatchResult { standings, winner_seat }), Event::Over]
    }

    fn not_your_turn(seat: u8) -> Vec<Event> {
        vec![Event::seat(seat, ServerMsg::error("not_your_turn"))]
    }

    /// The turn-start idle limit ran out: the turn ends as a miss, with no
    /// word served (so there is no `word_result`, only `turn_end`).
    fn idle_turn(&mut self, seat: u8, now: Instant) -> Vec<Event> {
        self.versus.end_turn();
        self.phase = Phase::AfterMiss { at: now + self.after_miss };
        vec![Event::all(ServerMsg::TurnEnd { seat })]
    }

    fn audio_failed(&mut self, seat: u8, word: Word, now: Instant) -> Vec<Event> {
        self.pauses += 1;
        let mut ev = vec![
            Event::ExpireAudio { tickets: Self::tickets(&word) },
            Event::all(ServerMsg::WordPaused { reason: "audio_unavailable".into() }),
        ];
        if self.pauses >= MAX_PAUSES {
            self.phase = Phase::Idle;
            ev.push(Event::all(ServerMsg::MatchEnded { reason: "audio_unavailable".into() }));
            ev.push(Event::Over);
        } else {
            // The word is void, as the app's "can't hear it" round: no score
            // moves, and the same player taps for a fresh word.
            self.phase = Phase::AwaitNext { seat, idle_at: Some(now + TURN_START_IDLE) };
        }
        ev
    }
}

impl ModeRules for H2h {
    fn max_seats(&self) -> u8 {
        2
    }
    fn min_seats(&self) -> u8 {
        2
    }
    fn running(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    fn on_start(&mut self, ctx: &Ctx) -> Vec<Event> {
        self.seats = [ctx.seats[0], ctx.seats[1]];
        self.kid = ctx.kid_safe;
        let seed = self.fixed_seed.unwrap_or_else(|| u64::from_le_bytes(random_bytes::<8>()));
        self.rng = Rng(seed);
        self.used.clear();
        self.word_no = 0;
        self.turn_no = 0;
        self.pauses = 0;
        // Rules-table row 4: Kid Mode plays 2 turns each, else 3.
        let turns = if self.kid { 2 } else { 3 };
        self.versus = Versus::start("P1".into(), "P2".into(), turns);
        self.begin_turn(ctx.now)
    }

    fn on_next(&mut self, seat: u8, ctx: &Ctx) -> Vec<Event> {
        match self.phase {
            Phase::AwaitNext { seat: s, .. } if s == seat => self.start_word(seat, ctx.now),
            _ if seat != self.active_seat() => Self::not_your_turn(seat),
            _ => vec![Event::seat(seat, ServerMsg::error("word_live"))],
        }
    }

    fn on_submit(&mut self, seat: u8, typed: &str, ctx: &Ctx) -> Vec<Event> {
        let Phase::Live { seat: s, ref word, started, limit, .. } = self.phase else {
            return if seat == self.active_seat() {
                vec![Event::seat(seat, ServerMsg::error("no_word"))]
            } else {
                Self::not_your_turn(seat)
            };
        };
        if s != seat {
            return Self::not_your_turn(seat);
        }
        let word = word.clone();
        // G-D17: on time iff server_recv ≤ deadline + min(median_rtt/2, 300 ms).
        let on_time = limit.is_none_or(|l| ctx.now <= started + l + ctx.grace_for(seat));
        let tier = grader::tier(&word.tier).expect("bank tier");
        let correct = on_time && grader::grade(&self.req.lang, self.kid, typed, &word.entry, tier);
        self.resolve(seat, &word, correct, ctx.now)
    }

    fn on_replay(&mut self, seat: u8, variant: Variant, _ctx: &Ctx) -> Vec<Event> {
        let Phase::Live { word, .. } = &mut self.phase else {
            return vec![Event::seat(seat, ServerMsg::error("no_word"))];
        };
        match variant {
            Variant::Normal => {
                vec![Event::all(ServerMsg::Audio { variant, audio_ticket: word.normal.clone() })]
            }
            Variant::Slow => {
                if let Some(t) = &word.slow {
                    return vec![Event::all(ServerMsg::Audio { variant, audio_ticket: t.clone() })];
                }
                let t = ticket();
                word.slow = Some(t.clone());
                let key = audio_key(&self.req.lang, &word.entry, Variant::Slow);
                vec![Event::FetchAudio { ticket: t.clone(), key }, Event::all(ServerMsg::Audio { variant, audio_ticket: t })]
            }
        }
    }

    fn on_audio(&mut self, ticket: &str, ok: bool, ctx: &Ctx) -> Vec<Event> {
        let Phase::Fetching { seat, ref word, .. } = self.phase else {
            return Vec::new(); // a slow replay's fetch: the clip is served on request
        };
        if word.normal != ticket {
            return Vec::new();
        }
        let word = word.clone();
        if !ok {
            return self.audio_failed(seat, word, ctx.now);
        }
        self.pauses = 0;
        let limit = self.req.timed.then(|| Duration::from_secs(core::tier_time(&word.tier) as u64));
        let idle_at = (!self.req.timed).then(|| ctx.now + UNTIMED_IDLE);
        let msg = ServerMsg::WordStart {
            seat,
            word_no: word.word_no,
            tier: word.tier.clone(),
            audio_ticket: word.normal.clone(),
            deadline_ms: limit.map(|l| l.as_millis() as u64),
        };
        self.phase = Phase::Live { seat, word, started: ctx.now, limit, idle_at };
        vec![Event::all(msg)]
    }

    fn on_timer(&mut self, ctx: &Ctx) -> Vec<Event> {
        match self.phase.clone() {
            Phase::Live { seat, word, started, limit, idle_at } => {
                let timed_out = limit.is_some_and(|l| ctx.now > started + l + ctx.grace_for(seat));
                let idle = idle_at.is_some_and(|t| ctx.now >= t);
                if timed_out || idle {
                    self.resolve(seat, &word, false, ctx.now)
                } else {
                    Vec::new()
                }
            }
            Phase::Fetching { seat, word, since } if ctx.now.duration_since(since) >= FETCH_TIMEOUT => {
                self.audio_failed(seat, word, ctx.now)
            }
            Phase::AwaitNext { seat, idle_at: Some(t) } if ctx.now >= t => self.idle_turn(seat, ctx.now),
            Phase::AfterCorrect { seat, at } if ctx.now >= at => self.start_word(seat, ctx.now),
            Phase::AfterMiss { at } if ctx.now >= at => {
                if self.versus.over {
                    self.finish()
                } else {
                    self.begin_turn(ctx.now)
                }
            }
            _ => Vec::new(),
        }
    }

    fn on_disconnect(&mut self, seat: u8, released: bool, ctx: &Ctx) -> Vec<Event> {
        // Waiting at turn start: a dropped player waits on the seat hold, not
        // the idle limit (the room ends the match if the seat is released).
        if let Phase::AwaitNext { seat: s, ref mut idle_at } = self.phase {
            if s == seat && !released {
                *idle_at = None;
            }
            return Vec::new();
        }
        let Phase::Live { seat: s, ref word, ref mut idle_at, .. } = self.phase else { return Vec::new() };
        if s != seat {
            return Vec::new();
        }
        if released {
            // §F5: not back within the seat hold, so the live word is a miss.
            let word = word.clone();
            return self.resolve(seat, &word, false, ctx.now);
        }
        // An untimed turn waits for the seat hold instead of the idle rule.
        if idle_at.is_some() {
            *idle_at = None;
        }
        Vec::new()
    }

    fn on_reconnect(&mut self, seat: u8, ctx: &Ctx) -> Vec<Event> {
        if let Phase::AwaitNext { seat: s, ref mut idle_at } = self.phase {
            if s == seat {
                *idle_at = Some(ctx.now + TURN_START_IDLE);
            }
        }
        if let Phase::Live { seat: s, ref mut idle_at, .. } = self.phase {
            if s == seat && !self.req.timed {
                *idle_at = Some(ctx.now + UNTIMED_IDLE);
            }
        }
        Vec::new()
    }

    fn next_deadline(&self) -> Option<Instant> {
        match &self.phase {
            Phase::Live { started, limit, idle_at, .. } => {
                // Wake at the bare deadline; on_timer adds the seat's grace
                // and simply waits again if that grace has not run out.
                limit.map(|l| *started + l).or(*idle_at)
            }
            Phase::Fetching { since, .. } => Some(*since + FETCH_TIMEOUT),
            Phase::AfterCorrect { at, .. } | Phase::AfterMiss { at } => Some(*at),
            Phase::AwaitNext { idle_at, .. } => *idle_at,
            _ => None,
        }
    }

    fn snapshot(&self) -> Value {
        // Never the word: only what every player may already see.
        let live = match &self.phase {
            Phase::Live { word, .. } => json!({
                "word_no": word.word_no,
                "tier": word.tier,
                "audio_ticket": word.normal,
            }),
            _ => Value::Null,
        };
        let phase = match self.phase {
            Phase::Idle => "idle",
            Phase::AwaitNext { .. } => "await_next",
            Phase::Fetching { .. } => "fetching",
            Phase::Live { .. } => "live",
            Phase::AfterCorrect { .. } => "after_correct",
            Phase::AfterMiss { .. } => "after_miss",
        };
        json!({
            "phase": phase,
            "active_seat": self.active_seat(),
            "turn_no": self.turn_no,
            "word_no": self.word_no,
            "players": [
                { "seat": self.seats[0], "current": self.versus.p1.current, "best": self.versus.p1.best, "turns_left": self.versus.p1.turns_left },
                { "seat": self.seats[1], "current": self.versus.p2.current, "best": self.versus.p2.best, "turns_left": self.versus.p2.turns_left },
            ],
            "live": live,
        })
    }

    fn abort(&mut self) {
        self.phase = Phase::Idle;
    }
}

#[cfg(test)]
#[path = "h2h_tests.rs"]
mod tests;
