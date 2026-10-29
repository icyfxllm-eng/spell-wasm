//! CC-GAME-SERVER F16 — load, soak and chaos bots for the room server.
//!
//!   botload run --target https://play.spellgame.net --admin http://127.0.0.1:8081 --profile load
//!   botload tts-stub --listen 127.0.0.1:9190
//!
//! Each room is two bots on two real sockets, driven by one task. The task
//! keeps its OWN scoreboard: the app's `versus::Versus`, fed the verdicts the
//! server announces, in order. Every `match_result` is checked against it
//! (A16.1 "zero score mismatches"), every socket's `seq` must be gapless with
//! no duplicate (A16.1 "zero lost or duplicated turns"), and a dropped bot
//! must resume within 60 s or receive a clean `match_ended` (A16.3).
//!
//! RIGHT ANSWERS. A bot cannot know the word: the server never sends it. Runs
//! use `tts-stub` as the server's SPELL_TTS_BASE; the stub puts the word into
//! the clip's audio payload (not a tag, which the server strips), so a bot
//! that fetches its ticket can read it and answer right on purpose. That side
//! channel exists only with the stub; real TTS clips carry no text.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use spell_server::protocol::{
    decode_server, encode_client, ClientMsg, CreateRoom, Hello, ProfileKind, Resume, ServerMsg, TierMode,
};
use spell_wasm::native_api::Versus;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const MARKER: &str = "SPELLBOT:";
const RESUME_LIMIT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------- options

#[derive(Clone, Debug)]
struct Opts {
    target: String,
    admin: Option<String>,
    profile: String,
    rooms: usize,
    minutes: f64,
    lang: String,
    right: f64,
    drop_frac: f64,
    seed: u64,
    origin: String,
    warmup_min: f64,
    /// Where bots fetch ticket audio. The game sockets always use `target`;
    /// clips may come over the host's loopback so one IP's request rate
    /// through Cloudflare doesn't throttle the test (Phase E, first run).
    audio_base: String,
    /// Spread the room starts over this many seconds.
    ramp_secs: f64,
}

fn opts(args: &[String]) -> Opts {
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let profile = get("--profile").unwrap_or_else(|| "smoke".into());
    // A16.1: 400 connections / 200 rooms. A16.2/3: 50% of that for 2 h.
    let (rooms, minutes, drop_frac) = match profile.as_str() {
        "load" => (200, 10.0, 0.0),
        "soak" => (100, 120.0, 0.0),
        "chaos" => (100, 120.0, 0.10),
        _ => (4, 1.5, 0.25),
    };
    let num = |k: &str, d: f64| get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
    Opts {
        target: get("--target").unwrap_or_else(|| "http://127.0.0.1:8080".into()).trim_end_matches('/').into(),
        admin: get("--admin").map(|a| a.trim_end_matches('/').to_string()),
        rooms: num("--rooms", rooms as f64) as usize,
        minutes: num("--minutes", minutes),
        lang: get("--lang").unwrap_or_else(|| "en".into()),
        right: num("--right", 0.8),
        drop_frac: num("--drop-frac", drop_frac),
        seed: num("--seed", 1.0) as u64,
        origin: get("--origin").unwrap_or_else(|| "capacitor://localhost".into()),
        warmup_min: num("--warmup", if profile == "soak" || profile == "chaos" { 10.0 } else { 0.0 }),
        audio_base: get("--audio-base")
            .or_else(|| get("--target"))
            .unwrap_or_else(|| "http://127.0.0.1:8080".into())
            .trim_end_matches('/')
            .into(),
        // 400 sockets from one IP in 4 s drew Cloudflare 429s; real players
        // arrive from many homes. Ramp like a crowd, not a burst.
        ramp_secs: num("--ramp", if profile == "smoke" { 1.0 } else { 90.0 }),
        profile,
    }
}

// ---------------------------------------------------------------- shared counters

#[derive(Default, Debug, serde::Serialize)]
struct Tally {
    rooms_started: usize,
    matches_done: usize,
    words: usize,
    right: usize,
    score_mismatches: usize,
    seq_gaps: usize,
    seq_dups: usize,
    dup_words: usize,
    drops: usize,
    resumed: usize,
    resume_ms: Vec<u64>,
    resume_late: usize,
    ended_clean: usize,
    /// Sockets the server side closed (a tunnel reconnect closes them all)
    /// and how many the bots got back with a resume, as the app does.
    server_closes: usize,
    close_resumed: usize,
    match_ended: BTreeMap<String, usize>,
    errors: BTreeMap<String, usize>,
    notes: Vec<String>,
}

type Shared = Arc<Mutex<Tally>>;

/// A line in the run log at once, stamped so it lines up with the hb lines
/// and the journal.
fn log(s: &str) {
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    println!("note unix={unix} {s}");
}

/// A problem: logged at once, and kept for the summary. Routine events go
/// through log() alone, so they can never crowd a failure out of the 40 kept
/// (the 2026-10-28 soak lost its one stall note behind 253 close notes).
fn note(t: &Shared, s: String) {
    log(&s);
    let mut t = t.lock().unwrap();
    if t.notes.len() < 40 {
        t.notes.push(s);
    }
}

/// splitmix64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---------------------------------------------------------------- one bot

struct Bot {
    name: &'static str,
    token: String,
    ws: Option<Ws>,
    seat: Option<u8>,
    last_seq: u64,
    /// The last NextWord/Submit not yet answered, kept with its cid so a
    /// resend after a resume is the same message (the server drops a
    /// duplicate Submit cid, F5).
    pending: Option<(Uuid, ClientMsg)>,
    /// When to resend `pending`: shortly after a resume, once the replayed
    /// frames have had the chance to answer it.
    resend_at: Option<Instant>,
    /// The start of the last frame received, for the stall report.
    last_msg: String,
}

/// Resume as the app does after a drop: keep trying, inside the 60 s seat
/// hold, while the path back (the tunnel) comes up again.
async fn reconnect(o: &Opts, token: &str, who: &str, resume: Resume) -> Result<Ws, String> {
    let give_up = Instant::now() + Duration::from_secs(45);
    let mut attempt = 0;
    loop {
        match connect(o, token, who, Some(resume.clone())).await {
            Ok(ws) => return Ok(ws),
            Err(e) if Instant::now() >= give_up => return Err(e),
            Err(_) => {
                tokio::time::sleep(backoff(attempt.min(3))).await;
                attempt += 1;
            }
        }
    }
}

async fn send_tracked(bot: &mut Bot, m: ClientMsg) {
    let cid = Uuid::new_v4();
    send_cid(bot, cid, &m).await;
    bot.pending = Some((cid, m));
}

async fn resend_pending(bot: &mut Bot) {
    bot.resend_at = None;
    if let Some((cid, m)) = bot.pending.clone() {
        RESENDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        send_cid(bot, cid, &m).await;
    }
}

/// How each socket ended: the server's own close frames say why (a code and
/// reason such as "no pong"); a cut elsewhere on the path shows as an error.
static CLOSE_WHY: std::sync::OnceLock<Mutex<BTreeMap<String, usize>>> = std::sync::OnceLock::new();

fn close_why(why: String) {
    *CLOSE_WHY.get_or_init(Default::default).lock().unwrap().entry(why).or_default() += 1;
}

/// StartMatch sent again because no TurnStart followed.
static START_RESENDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const START_RETRY: Duration = Duration::from_secs(5);

/// Actions resent after a resume because no answer came back.
static RESENDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn ws_url(target: &str) -> String {
    let t = target.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
    format!("{t}/v1/ws")
}

/// Frame sends that did not finish within 10 s (the socket stopped draining).
static SEND_TIMEOUTS: std::sync::OnceLock<Mutex<usize>> = std::sync::OnceLock::new();

/// Times Cloudflare (or anything in front) answered 429 or 5xx and a bot retried.
static RETRIES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn backoff(attempt: u32) -> Duration {
    // 0.5 s, 1 s, 2 s ... capped at 8 s, with up to 50% jitter from the clock.
    let base = (500u64 << attempt.min(4)).min(8000);
    let jitter = (Instant::now().elapsed().subsec_nanos() as u64 ^ (attempt as u64 * 7919)) % (base / 2 + 1);
    Duration::from_millis(base + jitter)
}

async fn connect(o: &Opts, token: &str, name: &str, resume: Option<Resume>) -> Result<Ws, String> {
    use tokio_tungstenite::tungstenite::Error as WsError;
    let mut attempt = 0;
    let mut ws = loop {
        let mut req = ws_url(&o.target).into_client_request().map_err(|e| e.to_string())?;
        req.headers_mut().insert("Origin", o.origin.parse().unwrap());
        // A connect that hangs (the first soak sat 10 min) must surface as an error.
        let Ok(got) = tokio::time::timeout(Duration::from_secs(20), tokio_tungstenite::connect_async(req)).await else {
            return Err("connect: timed out after 20 s".into());
        };
        match got {
            Ok((ws, _)) => break ws,
            Err(WsError::Http(r)) if (r.status().as_u16() == 429 || r.status().is_server_error()) && attempt < 8 => {
                RETRIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                tokio::time::sleep(backoff(attempt)).await;
                attempt += 1;
            }
            Err(e) => return Err(format!("connect: {e}")),
        }
    };
    let hello = ClientMsg::Hello(Hello {
        protocol: 1,
        app_build: "botload".into(),
        platform: "bot".into(),
        profile_kind: ProfileKind::Adult,
        ui_lang: "en".into(),
        display_name: Some(name.into()),
        token: token.into(),
        resume,
    });
    ws.send(Message::Text(encode_client(Uuid::new_v4(), &hello).into())).await.map_err(|e| e.to_string())?;
    Ok(ws)
}

async fn send(bot: &mut Bot, m: &ClientMsg) {
    send_cid(bot, Uuid::new_v4(), m).await
}

async fn send_cid(bot: &mut Bot, cid: Uuid, m: &ClientMsg) {
    if let Some(ws) = bot.ws.as_mut() {
        let frame = Message::Text(encode_client(cid, m).into());
        if tokio::time::timeout(Duration::from_secs(10), ws.send(frame)).await.is_err() {
            *SEND_TIMEOUTS.get_or_init(Default::default).lock().unwrap() += 1;
        }
    }
}

enum Got {
    Msg(u64, ServerMsg),
    Closed,
    Idle,
}

/// The next text frame within `wait`. Control frames (the server pings every
/// socket as soon as it opens) are skipped, not mistaken for silence.
async fn next(bot: &mut Bot, wait: Duration) -> Got {
    let Some(ws) = bot.ws.as_mut() else { return Got::Closed };
    let until = tokio::time::Instant::now() + wait;
    loop {
        match tokio::time::timeout_at(until, ws.next()).await {
            Err(_) => return Got::Idle,
            Ok(Some(Ok(Message::Text(t)))) => {
                return match decode_server(&t) {
                    Some(o) => Got::Msg(o.seq, o.msg),
                    None => Got::Idle,
                }
            }
            Ok(Some(Ok(Message::Close(f)))) => {
                close_why(f.map_or("close frame, no code".into(), |f| format!("close {} {}", u16::from(f.code), f.reason)));
                return Got::Closed;
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => {
                close_why(format!("error: {}", e.to_string().chars().take(60).collect::<String>()));
                return Got::Closed;
            }
            Ok(None) => {
                close_why("stream ended, no close frame".into());
                return Got::Closed;
            }
        }
    }
}

/// The word, read from the stub's clip, if the clip is the stub's.
async fn word_of(o: &Opts, http: &reqwest::Client, token: &str, ticket: &str) -> Option<String> {
    let mut attempt = 0;
    let r = loop {
        let r = http
            .get(format!("{}/v1/audio/{ticket}", o.audio_base))
            .header("Origin", &o.origin)
            .header("Authorization", format!("Bearer {token}"))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .ok()?;
        if r.status().as_u16() == 429 && attempt < 5 {
            RETRIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tokio::time::sleep(backoff(attempt)).await;
            attempt += 1;
            continue;
        }
        break r;
    };
    if !r.status().is_success() {
        return None;
    }
    let body = r.bytes().await.ok()?;
    let text = String::from_utf8_lossy(&body);
    let at = text.find(MARKER)? + MARKER.len();
    Some(text[at..].split('\n').next()?.to_string())
}

// ---------------------------------------------------------------- one room

struct RoomRun {
    o: Opts,
    t: Shared,
    http: reqwest::Client,
    rng: Rng,
    code: String,
    /// The bot-side scoreboard: the app's own Versus, fed the server's verdicts.
    local: Versus,
    seen_words: HashSet<(u8, u32)>,
    drop_at: Option<Instant>,
    deadline: Instant,
    /// The local scoreboard already ended the current turn.
    turn_closed: bool,
    /// StartMatch is sent and no TurnStart has come yet: when to send it
    /// again. A StartMatch lost in a drop, or refused while the guest is
    /// still resuming (seat_disconnected), left rooms in the lobby for good
    /// in the 2026-10-28 chaos run.
    start_retry_at: Option<Instant>,
}

impl RoomRun {
    fn track_seq(&self, bot: &mut Bot, seq: u64) {
        if seq == 0 {
            return;
        }
        if seq <= bot.last_seq {
            self.t.lock().unwrap().seq_dups += 1;
        } else if seq != bot.last_seq + 1 {
            self.t.lock().unwrap().seq_gaps += 1;
            note(&self.t, format!("{} {}: seq {} after {}", self.code, bot.name, seq, bot.last_seq));
        }
        bot.last_seq = bot.last_seq.max(seq);
    }

    /// Handles one frame for one bot. `scorer` is the bot whose stream feeds
    /// the local scoreboard (each event reaches both; count it once).
    async fn on_msg(&mut self, bot: &mut Bot, scorer: bool, seq: u64, msg: ServerMsg) -> bool {
        self.track_seq(bot, seq);
        match msg {
            ServerMsg::TurnStart { seat, .. } => {
                self.start_retry_at = None;
                if scorer {
                    self.turn_closed = false;
                }
                if Some(seat) == bot.seat {
                    send_tracked(bot, ClientMsg::NextWord {}).await;
                }
            }
            ServerMsg::WordPaused { .. } => {
                // No audio: the same player asks for a fresh word (the app's
                // "can't hear it" round).
                if matches!(bot.pending, Some((_, ClientMsg::NextWord {}))) {
                    bot.resend_at = Some(Instant::now() + Duration::from_secs(1));
                }
            }
            ServerMsg::WordStart { seat, word_no, audio_ticket, .. } => {
                if scorer && !self.seen_words.insert((seat, word_no)) {
                    self.t.lock().unwrap().dup_words += 1;
                }
                if Some(seat) == bot.seat {
                    bot.pending = None;
                    let word = word_of(&self.o, &self.http, &bot.token, &audio_ticket).await;
                    let right = self.rng.unit() < self.o.right;
                    let typed = match (&word, right) {
                        (Some(w), true) => w.clone(),
                        _ => "zzqzzq".to_string(),
                    };
                    if word.is_none() {
                        *self.t.lock().unwrap().errors.entry("no_word_in_clip".into()).or_default() += 1;
                    }
                    send_tracked(bot, ClientMsg::Submit { typed }).await;
                }
            }
            ServerMsg::WordResult { seat, correct, .. } => {
                if Some(seat) == bot.seat {
                    bot.pending = None;
                }
                if !scorer {
                    return false;
                }
                let mut t = self.t.lock().unwrap();
                t.words += 1;
                if correct {
                    t.right += 1;
                }
                drop(t);
                if correct {
                    self.local.record_correct();
                } else {
                    self.local.end_turn();
                    self.turn_closed = true;
                }
            }
            ServerMsg::TurnEnd { .. } if scorer => {
                // A turn can also end with no WordResult: the 120 s turn-start
                // idle rule counts it as a miss (h2h idle_turn).
                if !self.turn_closed {
                    self.local.end_turn();
                    self.turn_closed = true;
                }
            }
            ServerMsg::MatchResult { standings, winner_seat } if scorer => {
                let best = |s: u8| standings.iter().find(|x| x.seat == s).map(|x| x.best).unwrap_or(u32::MAX);
                let want_winner = self.local.winner().map(|w| w as u8);
                let ok = best(0) == self.local.p1.best && best(1) == self.local.p2.best && winner_seat == want_winner;
                {
                    let mut t = self.t.lock().unwrap();
                    t.matches_done += 1;
                    if !ok {
                        t.score_mismatches += 1;
                    }
                }
                if !ok {
                    note(
                        &self.t,
                        format!(
                            "{}: server {:?}/{:?} vs local {}/{} {:?}",
                            self.code, standings, winner_seat, self.local.p1.best, self.local.p2.best, want_winner
                        ),
                    );
                }
                self.new_match();
                return Instant::now() < self.deadline; // rematch while time remains
            }
            ServerMsg::MatchEnded { reason } if scorer => {
                *self.t.lock().unwrap().match_ended.entry(reason.clone()).or_default() += 1;
                self.new_match();
                return reason == "player_left" && Instant::now() < self.deadline;
            }
            // While StartMatch is being retried these answers are expected:
            // the guest is still resuming, or an earlier copy got through.
            ServerMsg::Error { code }
                if self.start_retry_at.is_some() && (code == "seat_disconnected" || code == "match_in_progress") => {}
            ServerMsg::Error { code } if code != "not_your_turn" => {
                *self.t.lock().unwrap().errors.entry(code).or_default() += 1;
            }
            _ => {}
        }
        false
    }

    fn new_match(&mut self) {
        self.turn_closed = false;
        self.local = Versus::start("P1".into(), "P2".into(), 3);
        self.seen_words.clear();
    }
}

async fn run_room(o: Opts, t: Shared, tokens: (String, String), seed: u64, deadline: Instant) {
    let http = reqwest::Client::new();
    let mut a = Bot { name: "A", token: tokens.0, ws: None, seat: None, last_seq: 0, pending: None, resend_at: None, last_msg: String::new() };
    let mut b = Bot { name: "B", token: tokens.1, ws: None, seat: None, last_seq: 0, pending: None, resend_at: None, last_msg: String::new() };
    let fail = |t: &Shared, what: &str| *t.lock().unwrap().errors.entry(what.to_string()).or_default() += 1;
    match connect(&o, &a.token, "BotA", None).await {
        Ok(ws) => a.ws = Some(ws),
        Err(e) => return note(&t, format!("A {e}")),
    }
    match connect(&o, &b.token, "BotB", None).await {
        Ok(ws) => b.ws = Some(ws),
        Err(e) => return note(&t, format!("B {e}")),
    }
    let req = CreateRoom {
        mode: "h2h".into(),
        lang: o.lang.clone(),
        tier_mode: TierMode::Climb,
        tier: None,
        timed: false,
        approve_joins: false,
    };
    send(&mut a, &ClientMsg::CreateRoom(req)).await;
    let mut code = String::new();
    while code.is_empty() {
        match next(&mut a, Duration::from_secs(20)).await {
            Got::Msg(seq, ServerMsg::Joined { code: c, seat }) => {
                a.seat = Some(seat);
                a.last_seq = seq;
                code = c;
            }
            Got::Msg(seq, other) => {
                if let ServerMsg::Error { code: c } = &other {
                    note(&t, format!("create_room answered error {c}"));
                }
                a.last_seq = a.last_seq.max(seq);
            }
            Got::Closed => return fail(&t, "create_closed"),
            Got::Idle => return fail(&t, "create_timeout"),
        }
    }
    send(&mut b, &ClientMsg::JoinRoom { code: code.clone() }).await;
    loop {
        match next(&mut b, Duration::from_secs(20)).await {
            Got::Msg(seq, ServerMsg::Joined { seat, .. }) => {
                b.seat = Some(seat);
                b.last_seq = seq;
                break;
            }
            Got::Msg(seq, _) => b.last_seq = b.last_seq.max(seq),
            _ => return fail(&t, "join_timeout"),
        }
    }
    let mut rng = Rng(seed);
    // Chaos: this room's guest drops once, at a random moment (A16.3).
    let drop_at = (rng.unit() < o.drop_frac).then(|| {
        let span = deadline.saturating_duration_since(Instant::now()).as_secs_f64();
        Instant::now() + Duration::from_secs_f64(span * (0.2 + 0.6 * rng.unit()))
    });
    let mut run = RoomRun {
        o: o.clone(),
        t: t.clone(),
        http,
        rng,
        code: code.clone(),
        local: Versus::start("P1".into(), "P2".into(), 3),
        seen_words: HashSet::new(),
        drop_at,
        deadline,
        turn_closed: false,
        start_retry_at: None,
    };
    t.lock().unwrap().rooms_started += 1;
    send(&mut a, &ClientMsg::StartMatch {}).await;
    run.start_retry_at = Some(Instant::now() + START_RETRY);

    let mut dropped_since: Option<Instant> = None;
    let mut last_progress = Instant::now();
    while Instant::now() < deadline + Duration::from_secs(90) {
        // Chaos drop and resume of the guest.
        if run.drop_at.is_some_and(|d| Instant::now() >= d) {
            run.drop_at = None;
            if let Some(mut ws) = b.ws.take() {
                let _ = ws.close(None).await;
            }
            t.lock().unwrap().drops += 1;
            dropped_since = Some(Instant::now());
            tokio::time::sleep(Duration::from_millis(500 + (run.rng.next() % 4000))).await;
            let resume = Resume { room: code.clone(), last_seq: b.last_seq };
            match reconnect(&o, &b.token, "BotB", resume).await {
                Ok(ws) => {
                    b.ws = Some(ws);
                    b.resend_at = Some(Instant::now() + Duration::from_secs(3));
                }
                Err(e) => note(&t, format!("{code} resume connect: {e}")),
            }
        }
        let mut rematch = false;
        let mut progressed = false;
        for (which, bot) in [(0, &mut a), (1, &mut b)] {
            match next(bot, Duration::from_millis(50)).await {
                Got::Msg(seq, msg) => {
                    progressed = true;
                    bot.last_msg = format!("{msg:?}").chars().take(400).collect();
                    if which == 1 {
                        if let (Some(since), ServerMsg::Welcome { .. }) = (dropped_since, &msg) {
                            let ms = since.elapsed().as_millis() as u64;
                            let mut tt = t.lock().unwrap();
                            tt.resumed += 1;
                            tt.resume_ms.push(ms);
                            if since.elapsed() > RESUME_LIMIT {
                                tt.resume_late += 1;
                            }
                            dropped_since = None;
                        }
                    }
                    if run.on_msg(bot, which == 0, seq, msg).await {
                        rematch = true;
                    }
                }
                Got::Closed if bot.ws.is_some() => {
                    bot.ws = None;
                    t.lock().unwrap().server_closes += 1;
                    log(&format!("{code} bot {} closed by server", bot.name));
                    // The app resumes after a drop; so do the bots. The 2026-10-27
                    // soak lost every socket to a tunnel reconnect, and bots
                    // that never came back spun this loop and starved the run.
                    let resume = Resume { room: code.clone(), last_seq: bot.last_seq };
                    let who = if which == 0 { "BotA" } else { "BotB" };
                    tokio::time::sleep(Duration::from_millis(250 + (run.rng.next() % 2000))).await;
                    match reconnect(&o, &bot.token, who, resume).await {
                        Ok(ws) => {
                            bot.ws = Some(ws);
                            bot.resend_at = Some(Instant::now() + Duration::from_secs(3));
                            t.lock().unwrap().close_resumed += 1;
                        }
                        Err(e) => note(&t, format!("{code} bot {} resume after close: {e}", bot.name)),
                    }
                }
                _ => {}
            }
            if bot.resend_at.is_some_and(|at| Instant::now() >= at) {
                resend_pending(bot).await;
            }
        }
        if progressed {
            last_progress = Instant::now();
        }
        if a.ws.is_none() || b.ws.is_none() {
            // A lost bot answers next() at once; never spin on it.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if rematch {
            tokio::time::sleep(Duration::from_millis(200)).await;
            send(&mut a, &ClientMsg::StartMatch {}).await;
            run.start_retry_at = Some(Instant::now() + START_RETRY);
        } else if a.ws.is_some() && run.start_retry_at.is_some_and(|at| Instant::now() >= at) {
            START_RESENDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            send(&mut a, &ClientMsg::StartMatch {}).await;
            run.start_retry_at = Some(Instant::now() + START_RETRY);
        }
        if Instant::now() >= deadline && run.local.p1.turns_left == 3 && run.local.p2.turns_left == 3 && !rematch {
            break; // between matches at the end of the run
        }
        if last_progress.elapsed() > Duration::from_secs(150) {
            for bot in [&a, &b] {
                note(
                    &t,
                    format!(
                        "{code}: stalled 150 s; bot {} connected={} seat={:?} last_seq={} pending={:?} last frame: {}",
                        bot.name,
                        bot.ws.is_some(),
                        bot.seat,
                        bot.last_seq,
                        bot.pending.as_ref().map(|(_, m)| m),
                        bot.last_msg
                    ),
                );
            }
            note(&t, format!("{code}: local p1 turns_left={} p2 turns_left={} turn_closed={} resuming={}", run.local.p1.turns_left, run.local.p2.turns_left, run.turn_closed, dropped_since.is_some()));
            fail(&t, "stalled");
            break;
        }
    }
    if dropped_since.is_some() {
        // Never resumed: only acceptable if the room ended cleanly.
        let mut tt = t.lock().unwrap();
        if tt.match_ended.contains_key("player_left") {
            tt.ended_clean += 1;
        } else {
            tt.resume_late += 1;
        }
    }
    for bot in [&mut a, &mut b] {
        if let Some(mut ws) = bot.ws.take() {
            let _ = ws.close(None).await;
        }
    }
}

// ---------------------------------------------------------------- admin

async fn admin_stats(o: &Opts) -> Option<Value> {
    let a = o.admin.as_ref()?;
    let r = reqwest::Client::new().get(format!("{a}/admin/stats")).timeout(Duration::from_secs(5)).send().await.ok()?;
    serde_json::from_str(&r.text().await.ok()?).ok()
}

async fn tokens(o: &Opts, n: usize) -> Result<Vec<String>, String> {
    let Some(a) = &o.admin else {
        return Err("--admin is required: bots mint through /admin/test-sessions (one IP may mint only 5 per hour)".into());
    };
    let mut out = Vec::new();
    while out.len() < n {
        let k = (n - out.len()).min(1000);
        let r = reqwest::Client::new()
            .post(format!("{a}/admin/test-sessions?n={k}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        for s in v["sessions"].as_array().ok_or("no sessions")? {
            out.push(s["token"].as_str().ok_or("no token")?.to_string());
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- the run

async fn run(o: Opts) -> i32 {
    println!(
        "botload: profile={} rooms={} ({} connections) minutes={} target={} audio={} ramp={}s",
        o.profile,
        o.rooms,
        o.rooms * 2,
        o.minutes,
        o.target,
        o.audio_base,
        o.ramp_secs
    );
    let toks = match tokens(&o, o.rooms * 2).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("botload: {e}");
            return 2;
        }
    };
    let t: Shared = Arc::new(Mutex::new(Tally::default()));
    let start = Instant::now();
    let deadline = start + Duration::from_secs_f64(o.minutes * 60.0);
    let samples: Arc<Mutex<Vec<(f64, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let (o2, s2) = (o.clone(), samples.clone());
        tokio::spawn(async move {
            loop {
                if let Some(v) = admin_stats(&o2).await {
                    s2.lock().unwrap().push((start.elapsed().as_secs_f64() / 60.0, v));
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
    {
        // Heartbeat: one wall-clock line a minute, so a stall can be dated
        // against the host's journal and told apart: the line itself late
        // means this process (or the box) stopped; loopback fine but tunnel
        // failing means the network.
        let (o2, t2) = (o.clone(), t.clone());
        tokio::spawn(async move {
            let probe = |url: String| async move {
                let at = Instant::now();
                match reqwest::Client::new().get(url).timeout(Duration::from_secs(10)).send().await {
                    Ok(r) if r.status().is_success() => format!("{}ms", at.elapsed().as_millis()),
                    Ok(r) => format!("http{}", r.status().as_u16()),
                    Err(e) if e.is_timeout() => "timeout".into(),
                    Err(_) => "fail".into(),
                }
            };
            let mut planned = Instant::now();
            loop {
                let late = planned.elapsed().as_secs();
                let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let local = probe(format!("{}/healthz", o2.audio_base)).await;
                let tunnel = probe(format!("{}/healthz", o2.target)).await;
                let conns = admin_stats(&o2).await.and_then(|v| v["connections"].as_u64()).map_or("?".into(), |c| c.to_string());
                let (words, rooms, closes) = {
                    let t = t2.lock().unwrap();
                    (t.words, t.rooms_started, t.server_closes)
                };
                let sends = *SEND_TIMEOUTS.get_or_init(Default::default).lock().unwrap();
                println!(
                    "hb unix={unix} min={:.1} late={late}s loopback={local} tunnel={tunnel} server_conns={conns} rooms={rooms} words={words} send_timeouts={sends} closes={closes}",
                    start.elapsed().as_secs_f64() / 60.0
                );
                planned += Duration::from_secs(60);
                tokio::time::sleep_until(planned.into()).await;
            }
        });
    }
    let mut handles = Vec::new();
    for i in 0..o.rooms {
        let (o2, t2) = (o.clone(), t.clone());
        let pair = (toks[2 * i].clone(), toks[2 * i + 1].clone());
        handles.push(tokio::spawn(run_room(o2, t2, pair, o.seed.wrapping_mul(1_000_003).wrapping_add(i as u64), deadline)));
        // Ramp: spread the starts over --ramp seconds, like players arriving.
        tokio::time::sleep(Duration::from_secs_f64((o.ramp_secs / o.rooms.max(1) as f64).max(0.02))).await;
    }
    for h in handles {
        let _ = h.await;
    }
    let end_stats = admin_stats(&o).await;
    if let Some(v) = &end_stats {
        samples.lock().unwrap().push((start.elapsed().as_secs_f64() / 60.0, v.clone()));
    }
    let tally = t.lock().unwrap();
    let samples = samples.lock().unwrap();
    report(&o, &tally, &samples)
}

fn report(o: &Opts, t: &Tally, samples: &[(f64, Value)]) -> i32 {
    let p95_max = samples.iter().filter_map(|(_, v)| v["handling_us_p95"].as_u64()).max().unwrap_or(0);
    let peak_conns = samples.iter().filter_map(|(_, v)| v["connections"].as_u64()).max().unwrap_or(0);
    // Live samples only: the last one is taken after every bot has closed,
    // when the server rightly holds no sockets (the first passing soak read
    // that as "fds 259 → 19").
    let warm: Vec<&(f64, Value)> = samples.iter().filter(|(m, _)| *m >= o.warmup_min && *m <= o.minutes).collect();
    let rss = |s: &&(f64, Value)| s.1["rss_bytes"].as_u64();
    // Descriptors beyond one per open socket: rooms that end early close
    // their sockets, and a fall in fds is not a leak (the 2026-10-28 chaos
    // run read 38 stalled rooms as "fds 299 → 200").
    let fds = |s: &&(f64, Value)| Some(s.1["open_fds"].as_u64()?.saturating_sub(s.1["connections"].as_u64()?));
    let (rss0, rss1) = (warm.first().and_then(rss), warm.last().and_then(rss));
    let (fd0, fd1) = (warm.first().and_then(fds), warm.last().and_then(fds));
    let rss_growth = match (rss0, rss1) {
        (Some(a), Some(b)) if a > 0 => Some((b as f64 - a as f64) / a as f64 * 100.0),
        _ => None,
    };
    let mut resume = t.resume_ms.clone();
    resume.sort_unstable();
    let resume_max = resume.last().copied().unwrap_or(0);

    let mut checks: Vec<(&str, bool, String)> = vec![
        ("rooms started", t.rooms_started == o.rooms, format!("{}/{}", t.rooms_started, o.rooms)),
        ("matches completed", t.matches_done > 0, t.matches_done.to_string()),
        ("score mismatches = 0", t.score_mismatches == 0, t.score_mismatches.to_string()),
        ("seq gaps = 0", t.seq_gaps == 0, t.seq_gaps.to_string()),
        ("seq duplicates = 0", t.seq_dups == 0, t.seq_dups.to_string()),
        ("duplicated words = 0", t.dup_words == 0, t.dup_words.to_string()),
        ("stalled rooms = 0", !t.errors.contains_key("stalled"), t.errors.get("stalled").copied().unwrap_or(0).to_string()),
        ("right answers were possible (stub clips)", t.right > 0, format!("{} of {} words", t.right, t.words)),
    ];
    if o.admin.is_some() {
        checks.push(("handling p95 < 50 ms", p95_max < 50_000, format!("{:.2} ms", p95_max as f64 / 1000.0)));
    }
    if o.profile == "load" {
        checks.push(("peak connections ≥ 2×rooms", peak_conns as usize >= o.rooms * 2, peak_conns.to_string()));
    }
    if o.profile == "soak" || o.profile == "chaos" {
        if let Some(g) = rss_growth {
            checks.push(("memory growth after warm-up < 5%", g < 5.0, format!("{g:.2}% ({:?} → {:?})", rss0, rss1)));
        } else {
            checks.push(("memory growth after warm-up < 5%", false, "no /proc samples (run on the Linux host)".into()));
        }
        if let (Some(a), Some(b)) = (fd0, fd1) {
            let flat = (b as i64 - a as i64).abs() <= (a as i64 / 50).max(8);
            checks.push(("file descriptors flat (beyond one per socket)", flat, format!("{a} → {b}")));
        }
    }
    if o.drop_frac > 0.0 {
        checks.push((
            "every dropped bot resumed ≤ 60 s or ended cleanly",
            t.resume_late == 0 && t.resumed + t.ended_clean >= t.drops,
            format!("drops {} resumed {} clean-ended {} late {} slowest {} ms", t.drops, t.resumed, t.ended_clean, t.resume_late, resume_max),
        ));
    }
    println!("\n== botload {} ==", o.profile);
    let mut failed = false;
    for (name, ok, detail) in &checks {
        println!("  {} {name}: {detail}", if *ok { "PASS" } else { "FAIL" });
        failed |= !ok;
    }
    println!("  retries after 429/5xx: {}", RETRIES.load(std::sync::atomic::Ordering::Relaxed));
    println!(
        "  server closes: {} (resumed {}), actions resent after a resume: {}",
        t.server_closes,
        t.close_resumed,
        RESENDS.load(std::sync::atomic::Ordering::Relaxed)
    );
    println!("  StartMatch sent again (no TurnStart within 5 s): {}", START_RESENDS.load(std::sync::atomic::Ordering::Relaxed));
    println!("  how sockets closed: {:?}", CLOSE_WHY.get_or_init(Default::default).lock().unwrap());
    println!("  sends that timed out: {}", *SEND_TIMEOUTS.get_or_init(Default::default).lock().unwrap());
    println!("  matches_ended: {:?}", t.match_ended);
    println!("  errors: {:?}", t.errors);
    for n in &t.notes {
        println!("  note: {n}");
    }
    let summary = json!({ "profile": o.profile, "tally": t, "samples": samples.iter().map(|(m, v)| json!({"minute": m, "stats": v})).collect::<Vec<_>>() });
    println!("\nJSON {}", summary);
    if failed {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------- the TTS stub

/// The canned-clip TTS stand-in for load runs (F16): answers `/api/speak`
/// with an MP3 frame header and the requested word in the payload, so bots
/// can answer right on purpose, and `/api/health` with 200.
async fn tts_stub(listen: &str) -> i32 {
    use axum::extract::Query;
    use axum::routing::get;
    async fn speak(Query(q): Query<std::collections::HashMap<String, String>>) -> axum::response::Response {
        use axum::response::IntoResponse;
        // Mandarin is answered by its reading; everything else by its word.
        let w = q.get("py").or_else(|| q.get("word")).cloned().unwrap_or_default();
        let mut body = vec![0xFF, 0xFB, 0x90, 0x00];
        body.extend(format!("{MARKER}{w}\n").as_bytes());
        body.extend(std::iter::repeat_n(0u8, 256));
        ([("content-type", "audio/mpeg")], body).into_response()
    }
    let app = axum::Router::new().route("/api/health", get(|| async { "ok" })).route("/api/speak", get(speak));
    let l = match tokio::net::TcpListener::bind(listen).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("botload tts-stub: cannot bind {listen}: {e}");
            return 2;
        }
    };
    println!("botload tts-stub: listening on {listen}. The server in its container reaches it as SPELL_TTS_BASE=http://host.containers.internal:{port}", port = listen.rsplit(':').next().unwrap_or("9190"));
    let _ = axum::serve(l, app).await;
    0
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(String::as_str) {
        Some("run") => run(opts(&args)).await,
        Some("tts-stub") => {
            let listen = args.iter().position(|a| a == "--listen").and_then(|i| args.get(i + 1)).cloned();
            tts_stub(listen.as_deref().unwrap_or("127.0.0.1:9190")).await
        }
        _ => {
            eprintln!("usage: botload run --target URL --admin URL --profile smoke|load|soak|chaos [--rooms N --minutes M --lang en --right 0.8 --drop-frac 0.1]\n       botload tts-stub --listen 127.0.0.1:9190");
            2
        }
    };
    std::process::exit(code);
}
