//! CC-GAME-SERVER F16 — load, soak and chaos bots for the room server.
//!
//!   botload run --target https://play.spellgame.net --admin http://127.0.0.1:8081 --profile load
//!   botload tts-stub --listen 127.0.0.1:9090
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
    match_ended: BTreeMap<String, usize>,
    errors: BTreeMap<String, usize>,
    notes: Vec<String>,
}

type Shared = Arc<Mutex<Tally>>;

fn note(t: &Shared, s: String) {
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
}

fn ws_url(target: &str) -> String {
    let t = target.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
    format!("{t}/v1/ws")
}

async fn connect(o: &Opts, token: &str, name: &str, resume: Option<Resume>) -> Result<Ws, String> {
    let mut req = ws_url(&o.target).into_client_request().map_err(|e| e.to_string())?;
    req.headers_mut().insert("Origin", o.origin.parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::connect_async(req).await.map_err(|e| format!("connect: {e}"))?;
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
    if let Some(ws) = bot.ws.as_mut() {
        let _ = ws.send(Message::Text(encode_client(Uuid::new_v4(), m).into())).await;
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
            Ok(Some(Ok(_))) => continue,
            _ => return Got::Closed,
        }
    }
}

/// The word, read from the stub's clip, if the clip is the stub's.
async fn word_of(o: &Opts, http: &reqwest::Client, token: &str, ticket: &str) -> Option<String> {
    let r = http
        .get(format!("{}/v1/audio/{ticket}", o.target))
        .header("Origin", &o.origin)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .ok()?;
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
            ServerMsg::TurnStart { seat, .. } if Some(seat) == bot.seat => {
                send(bot, &ClientMsg::NextWord {}).await;
            }
            ServerMsg::WordStart { seat, word_no, audio_ticket, .. } => {
                if scorer && !self.seen_words.insert((seat, word_no)) {
                    self.t.lock().unwrap().dup_words += 1;
                }
                if Some(seat) == bot.seat {
                    let word = word_of(&self.o, &self.http, &bot.token, &audio_ticket).await;
                    let right = self.rng.unit() < self.o.right;
                    let typed = match (&word, right) {
                        (Some(w), true) => w.clone(),
                        _ => "zzqzzq".to_string(),
                    };
                    if word.is_none() {
                        *self.t.lock().unwrap().errors.entry("no_word_in_clip".into()).or_default() += 1;
                    }
                    send(bot, &ClientMsg::Submit { typed }).await;
                }
            }
            ServerMsg::WordResult { correct, .. } if scorer => {
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
                }
            }
            ServerMsg::TurnEnd { .. } => {}
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
            ServerMsg::Error { code } if code != "not_your_turn" => {
                *self.t.lock().unwrap().errors.entry(code).or_default() += 1;
            }
            _ => {}
        }
        false
    }

    fn new_match(&mut self) {
        self.local = Versus::start("P1".into(), "P2".into(), 3);
        self.seen_words.clear();
    }
}

async fn run_room(o: Opts, t: Shared, tokens: (String, String), seed: u64, deadline: Instant) {
    let http = reqwest::Client::new();
    let mut a = Bot { name: "A", token: tokens.0, ws: None, seat: None, last_seq: 0 };
    let mut b = Bot { name: "B", token: tokens.1, ws: None, seat: None, last_seq: 0 };
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
    };
    t.lock().unwrap().rooms_started += 1;
    send(&mut a, &ClientMsg::StartMatch {}).await;

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
            match connect(&o, &b.token, "BotB", Some(resume)).await {
                Ok(ws) => b.ws = Some(ws),
                Err(e) => note(&t, format!("{code} resume connect: {e}")),
            }
        }
        let mut rematch = false;
        let mut progressed = false;
        for (which, bot) in [(0, &mut a), (1, &mut b)] {
            match next(bot, Duration::from_millis(50)).await {
                Got::Msg(seq, msg) => {
                    progressed = true;
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
                    note(&t, format!("{code} bot {} closed by server", bot.name));
                }
                _ => {}
            }
        }
        if progressed {
            last_progress = Instant::now();
        }
        if rematch {
            tokio::time::sleep(Duration::from_millis(200)).await;
            send(&mut a, &ClientMsg::StartMatch {}).await;
        }
        if Instant::now() >= deadline && run.local.p1.turns_left == 3 && run.local.p2.turns_left == 3 && !rematch {
            break; // between matches at the end of the run
        }
        if last_progress.elapsed() > Duration::from_secs(150) {
            note(&t, format!("{code}: stalled 150 s"));
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
    let r = reqwest::get(format!("{a}/admin/stats")).await.ok()?;
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
    println!("botload: profile={} rooms={} ({} connections) minutes={} target={}", o.profile, o.rooms, o.rooms * 2, o.minutes, o.target);
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
    let mut handles = Vec::new();
    for i in 0..o.rooms {
        let (o2, t2) = (o.clone(), t.clone());
        let pair = (toks[2 * i].clone(), toks[2 * i + 1].clone());
        handles.push(tokio::spawn(run_room(o2, t2, pair, o.seed.wrapping_mul(1_000_003).wrapping_add(i as u64), deadline)));
        // Ramp: don't open every socket in the same millisecond.
        tokio::time::sleep(Duration::from_millis(20)).await;
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
    let warm: Vec<&(f64, Value)> = samples.iter().filter(|(m, _)| *m >= o.warmup_min).collect();
    let rss = |s: &&(f64, Value)| s.1["rss_bytes"].as_u64();
    let fds = |s: &&(f64, Value)| s.1["open_fds"].as_u64();
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
            checks.push(("file descriptors flat", flat, format!("{a} → {b}")));
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
    println!("botload tts-stub: listening on {listen} (point SPELL_TTS_BASE at http://{listen})");
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
            tts_stub(listen.as_deref().unwrap_or("127.0.0.1:9090")).await
        }
        _ => {
            eprintln!("usage: botload run --target URL --admin URL --profile smoke|load|soak|chaos [--rooms N --minutes M --lang en --right 0.8 --drop-frac 0.1]\n       botload tts-stub --listen 127.0.0.1:9090");
            2
        }
    };
    std::process::exit(code);
}
