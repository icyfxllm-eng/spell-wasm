//! CC-ONLINE-TURN F8 / A9: the load run for 3+ seat modes. Each room is four
//! bots on four sockets playing `turn_null` on the real engine, one task per
//! bot, each hearing every word as a phone at the table would.
//!
//! The server must be built with `--features test-modes` and run with
//! SPELL_ONLINE_TURN=1, since `turn_null` exists nowhere else.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// `turn_null`'s rule version. A mismatch shows as a `mode_version` error.
pub const NULL_MODE_VERSION: u32 = 1;
pub const SEATS: usize = 4;
const MODE: &str = "turn_null";

/// What each seat heard, game by game. All four must match.
type Heard = Arc<Mutex<BTreeMap<u32, Vec<u64>>>>;

fn fnv(h: u64, s: &str) -> u64 {
    s.bytes().fold(h, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3))
}

fn track_seq(t: &Shared, bot: &mut Bot, seq: u64) {
    if seq == 0 {
        return;
    }
    if seq <= bot.last_seq {
        t.lock().unwrap().seq_dups += 1;
    } else if seq != bot.last_seq + 1 {
        t.lock().unwrap().seq_gaps += 1;
        note(t, format!("{} seq {} after {}", bot.name, seq, bot.last_seq));
    }
    bot.last_seq = bot.last_seq.max(seq);
}

fn error_count(t: &Shared, what: &str) {
    *t.lock().unwrap().errors.entry(what.to_string()).or_default() += 1;
}

async fn think(rng: &mut Rng, from_ms: u64, to_ms: u64) {
    tokio::time::sleep(Duration::from_millis(from_ms + rng.next() % (to_ms - from_ms))).await;
}

fn new_bot(name: &'static str, token: String) -> Bot {
    Bot { name, token, ws: None, seat: None, last_seq: 0, pending: None, resend_at: None, last_msg: String::new() }
}

/// Plays until the deadline, then to the end of the game in hand.
#[allow(clippy::too_many_arguments)]
async fn play(o: Opts, t: Shared, mut bot: Bot, host: bool, mut rng: Rng, deadline: Instant, heard: Heard, done: Arc<AtomicBool>) {
    let http = reqwest::Client::new();
    let me = bot.seat.expect("seated before play");
    let mut scores: BTreeMap<u8, u32> = BTreeMap::new();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut game = 0u32;
    let mut in_game = false;
    let mut start_retry_at = host.then(|| Instant::now() + START_RETRY);
    let mut last_progress = Instant::now();
    if host {
        send(&mut bot, &ClientMsg::StartMatch {}).await;
    }
    loop {
        // The host decides when the room stops. A guest deciding by the clock
        // can leave as the host starts one more game (first 50-room run).
        if done.load(Ordering::Relaxed) && !in_game && start_retry_at.is_none() {
            break;
        }
        if last_progress.elapsed() > Duration::from_secs(150) {
            note(&t, format!("{} stalled 150 s; last frame: {}", bot.name, bot.last_msg));
            error_count(&t, "stalled");
            break;
        }
        if start_retry_at.is_some_and(|at| Instant::now() >= at) {
            START_RESENDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            send(&mut bot, &ClientMsg::StartMatch {}).await;
            start_retry_at = Some(Instant::now() + START_RETRY);
        }
        let (seq, msg) = match next(&mut bot, Duration::from_secs(1)).await {
            Got::Msg(seq, msg) => (seq, msg),
            Got::Idle => continue,
            Got::Closed => {
                t.lock().unwrap().server_closes += 1;
                error_count(&t, "socket_closed");
                break;
            }
        };
        last_progress = Instant::now();
        track_seq(&t, &mut bot, seq);
        bot.last_msg = format!("{msg:?}").chars().take(400).collect();
        if matches!(
            msg,
            ServerMsg::TurnBegin { .. } | ServerMsg::TurnAudio { .. } | ServerMsg::TurnVerdict { .. } | ServerMsg::TurnEffects { .. } | ServerMsg::TurnAuto { .. } | ServerMsg::GameEnd { .. }
        ) {
            hash = fnv(hash, &format!("{msg:?}"));
        }
        match msg {
            ServerMsg::TurnBegin { seat, phase, .. } => {
                if host {
                    start_retry_at = None;
                }
                in_game = true;
                if seat == me && phase == "awaiting_roll" {
                    think(&mut rng, 200, 1200).await;
                    send(&mut bot, &ClientMsg::NextWord {}).await;
                } else if seat == me && phase == "awaiting_choice" {
                    think(&mut rng, 200, 1200).await;
                    send(&mut bot, &ClientMsg::TurnChoice { choice: "stop".into() }).await;
                }
            }
            ServerMsg::TurnAudio { seat, audio_ticket } => {
                // The whole table hears the word (CC-BOARD-GAME-ONLINE O1).
                let word = word_of(&o, &http, &bot.token, &audio_ticket).await;
                if seat != me {
                    continue;
                }
                let Some(word) = word else {
                    error_count(&t, "no_clip");
                    send(&mut bot, &ClientMsg::TurnAnswer { answer: "zzz".into() }).await;
                    continue;
                };
                think(&mut rng, 500, 2500).await;
                let right = rng.unit() < o.right;
                {
                    let mut tt = t.lock().unwrap();
                    tt.words += 1;
                    if right {
                        tt.right += 1;
                    }
                }
                let answer = if right { word } else { "zzz".to_string() };
                send(&mut bot, &ClientMsg::TurnAnswer { answer }).await;
            }
            ServerMsg::TurnVerdict { seat, correct, .. } => {
                if correct {
                    *scores.entry(seat).or_default() += 1;
                }
            }
            ServerMsg::TurnAuto { reason, .. } => error_count(&t, &format!("auto_{reason}")),
            ServerMsg::GameEnd { summary, .. } => {
                for row in summary["scores"].as_array().map_or(&[][..], |a| a.as_slice()) {
                    let (seat, got) = (row[0].as_u64().unwrap_or(9) as u8, row[1].as_u64().unwrap_or(0) as u32);
                    if scores.get(&seat).copied().unwrap_or(0) != got {
                        t.lock().unwrap().score_mismatches += 1;
                        note(&t, format!("{}: seat {seat} scored {got} on the server", bot.name));
                    }
                }
                heard.lock().unwrap().entry(game).or_default().push(hash);
                game += 1;
                hash = 0xcbf2_9ce4_8422_2325;
                scores.clear();
                in_game = false;
                if host {
                    t.lock().unwrap().matches_done += 1;
                    if Instant::now() < deadline {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                        send(&mut bot, &ClientMsg::StartMatch {}).await;
                        start_retry_at = Some(Instant::now() + START_RETRY);
                    } else {
                        done.store(true, Ordering::Relaxed);
                    }
                }
            }
            ServerMsg::MatchEnded { reason } => {
                *t.lock().unwrap().match_ended.entry(reason).or_default() += 1;
                in_game = false;
            }
            // A guest may still be arriving when the first StartMatch lands.
            ServerMsg::Error { code } if start_retry_at.is_some() && code == "seat_disconnected" => {}
            ServerMsg::Error { code } => error_count(&t, &code),
            _ => {}
        }
    }
    if let Some(mut ws) = bot.ws.take() {
        let _ = ws.close(None).await;
    }
}

pub async fn run_room(o: Opts, t: Shared, tokens: Vec<String>, seed: u64, deadline: Instant) {
    let names = ["BotA", "BotB", "BotC", "BotD"];
    let mut bots = Vec::new();
    for (name, token) in names.into_iter().zip(tokens) {
        let mut bot = new_bot(name, token);
        match connect(&o, &bot.token, name, None).await {
            Ok(ws) => bot.ws = Some(ws),
            Err(e) => return note(&t, format!("{name} {e}")),
        }
        bots.push(bot);
    }
    let req = CreateRoom { mode: MODE.into(), lang: o.lang.clone(), tier_mode: TierMode::Climb, tier: None, timed: false, approve_joins: false, options: None};
    send(&mut bots[0], &ClientMsg::CreateRoom(req)).await;
    let mut code = String::new();
    while code.is_empty() {
        match next(&mut bots[0], Duration::from_secs(20)).await {
            Got::Msg(seq, ServerMsg::Joined { code: c, seat }) => {
                bots[0].seat = Some(seat);
                bots[0].last_seq = seq;
                code = c;
            }
            Got::Msg(_, ServerMsg::Error { code: c }) => return note(&t, format!("create_room answered error {c}")),
            Got::Msg(seq, _) => bots[0].last_seq = bots[0].last_seq.max(seq),
            _ => return error_count(&t, "create_failed"),
        }
    }
    for bot in bots.iter_mut().skip(1) {
        send(bot, &ClientMsg::JoinRoom { code: code.clone() }).await;
        loop {
            match next(bot, Duration::from_secs(20)).await {
                Got::Msg(seq, ServerMsg::Joined { seat, .. }) => {
                    bot.seat = Some(seat);
                    bot.last_seq = seq;
                    break;
                }
                Got::Msg(_, ServerMsg::Error { code: c }) => return note(&t, format!("join answered error {c}")),
                Got::Msg(seq, _) => bot.last_seq = bot.last_seq.max(seq),
                _ => return error_count(&t, "join_failed"),
            }
        }
    }
    t.lock().unwrap().rooms_started += 1;
    let heard: Heard = Arc::default();
    let done = Arc::new(AtomicBool::new(false));
    let mut rng = Rng(seed);
    let mut tasks = Vec::new();
    for (i, bot) in bots.into_iter().enumerate() {
        let r = Rng(rng.next());
        tasks.push(tokio::spawn(play(o.clone(), t.clone(), bot, i == 0, r, deadline, heard.clone(), done.clone())));
    }
    for task in tasks {
        let _ = task.await;
    }
    // I2: every seat heard the same game, event for event.
    for (game, hashes) in heard.lock().unwrap().iter() {
        if hashes.len() != SEATS || hashes.iter().any(|h| *h != hashes[0]) {
            t.lock().unwrap().score_mismatches += 1;
            note(&t, format!("{code} game {game}: seats heard different games ({} reported)", hashes.len()));
        }
    }
}
