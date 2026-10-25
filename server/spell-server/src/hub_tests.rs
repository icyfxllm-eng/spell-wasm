//! Room-layer tests (F3, F5, A1.7) against the trivial test mode, under
//! paused tokio time.

use super::*;
use crate::protocol::{decode_server, Outbound, Variant};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

struct P {
    sess: Session,
    rx: UnboundedReceiver<Out>,
}

fn ip() -> IpAddr {
    "203.0.113.5".parse().unwrap()
}

fn connect(hub: &Hub, id: &str, kind: ProfileKind, conn_id: u64) -> P {
    connect_resume(hub, id, kind, conn_id, None)
}

fn connect_resume(hub: &Hub, id: &str, kind: ProfileKind, conn_id: u64, resume: Option<(String, u64)>) -> P {
    let (tx, rx) = unbounded_channel();
    let sess = Session { player: id.into(), conn_id, kind, name: format!("n-{id}"), ip: ip() };
    hub.attach(&sess, tx, resume, Instant::now());
    P { sess, rx }
}

fn send(hub: &Hub, p: &P, msg: ClientMsg) {
    hub.handle(&p.sess, Inbound { cid: Uuid::new_v4(), msg }, Instant::now());
}

fn send_cid(hub: &Hub, p: &P, cid: Uuid, msg: ClientMsg) {
    hub.handle(&p.sess, Inbound { cid, msg }, Instant::now());
}

/// Everything queued for `p` so far.
fn take(p: &mut P) -> Vec<Outbound> {
    let mut out = Vec::new();
    while let Ok(o) = p.rx.try_recv() {
        if let Out::Frame(f) = o {
            out.push(decode_server(&f).expect("server frame decodes"));
        }
    }
    out
}

fn errors(v: &[Outbound]) -> Vec<String> {
    v.iter()
        .filter_map(|o| match &o.msg {
            ServerMsg::Error { code } => Some(code.clone()),
            _ => None,
        })
        .collect()
}

fn room_req(mode: &str) -> CreateRoom {
    CreateRoom {
        mode: mode.into(),
        lang: "en".into(),
        tier_mode: TierMode::Climb,
        tier: None,
        timed: false,
        approve_joins: false,
    }
}

fn create(hub: &Hub, host: &mut P, mode: &str) -> String {
    send(hub, host, ClientMsg::CreateRoom(room_req(mode)));
    take(host)
        .into_iter()
        .find_map(|o| match o.msg {
            ServerMsg::Joined { code, .. } => Some(code),
            _ => None,
        })
        .expect("joined after create")
}

fn join(hub: &Hub, p: &mut P, code: &str) -> Vec<Outbound> {
    send(hub, p, ClientMsg::JoinRoom { code: code.to_lowercase() });
    take(p)
}

fn pending_id(v: &[Outbound]) -> Option<u64> {
    v.iter().find_map(|o| match o.msg {
        ServerMsg::JoinRequest { request_id, .. } => Some(request_id),
        _ => None,
    })
}

async fn advance(hub: &Hub, d: Duration) {
    tokio::time::advance(d).await;
    hub.tick(Instant::now());
}

// ---------------------------------------------------------------- F3

#[test]
fn a3_1_a_million_codes_use_only_the_alphabet() {
    let mut seen = HashSet::new();
    for _ in 0..1_000_000 {
        let c = new_code();
        assert_eq!(c.len(), CODE_LEN);
        assert!(c.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{c}");
        seen.insert(c);
    }
    // 25^6 ≈ 2.4e8, so a million draws collide a few thousand times at most.
    assert!(seen.len() > 990_000, "codes look non-uniform: {} distinct", seen.len());
    for bad in "0OI1LB8S52Z".chars() {
        assert!(!CODE_ALPHABET.contains(&(bad as u8)), "{bad} must not be in the alphabet");
    }
}

#[test]
fn a3_1_codes_are_case_insensitive_and_strict() {
    assert_eq!(normalize_code(" acdefg "), Some("ACDEFG".into()));
    assert_eq!(normalize_code("ACDEF0"), None);
    assert_eq!(normalize_code("ACDEF"), None);
}

#[tokio::test(start_paused = true)]
async fn a3_1_live_room_codes_are_unique() {
    let hub = Hub::new();
    let mut codes = HashSet::new();
    for i in 0..2000 {
        let mut h = connect(&hub, &format!("h{i}"), ProfileKind::Adult, i);
        // Rooms beyond 5/min/player are fine: each host is a different player.
        codes.insert(create(&hub, &mut h, "test"));
    }
    assert_eq!(codes.len(), 2000);
}

#[tokio::test(start_paused = true)]
async fn join_seats_a_second_player_and_both_see_room_state() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    let code = create(&hub, &mut a, "test");
    let got = join(&hub, &mut b, &code);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::Joined { seat: 1, .. })));
    let st = hub.room_state(&code).unwrap();
    assert_eq!(st.seats.len(), 2);
    assert!(!st.kid_safe);
    assert!(take(&mut a).iter().any(|o| matches!(&o.msg, ServerMsg::RoomState(s) if s.seats.len() == 2)));
}

#[tokio::test(start_paused = true)]
async fn a3_7_rooms_are_capped_by_max_seats() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test");
    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    join(&hub, &mut b, &code);
    let mut c = connect(&hub, "c", ProfileKind::Adult, 3);
    assert_eq!(errors(&join(&hub, &mut c, &code)), vec!["room_full"]);

    let mut h = connect(&hub, "h4", ProfileKind::Adult, 10);
    let code4 = create(&hub, &mut h, "test4");
    for i in 0..3 {
        let mut p = connect(&hub, &format!("q{i}"), ProfileKind::Adult, 11 + i);
        assert!(errors(&join(&hub, &mut p, &code4)).is_empty());
    }
    let mut fifth = connect(&hub, "q9", ProfileKind::Adult, 20);
    assert_eq!(errors(&join(&hub, &mut fifth, &code4)), vec!["room_full"]);
    assert_eq!(hub.room_state(&code4).unwrap().seats.len(), 4);
}

#[tokio::test(start_paused = true)]
async fn a3_6_a_kid_cannot_create_a_room() {
    let hub = Hub::new();
    let mut k = connect(&hub, "k", ProfileKind::Kid, 1);
    send(&hub, &k, ClientMsg::CreateRoom(room_req("test")));
    assert_eq!(errors(&take(&mut k)), vec!["not_allowed"]);
}

#[tokio::test(start_paused = true)]
async fn a3_4_a_kid_forces_approval_even_with_the_toggle_off() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test4");
    take(&mut a);
    // The kid's own knock needs approval...
    let mut k = connect(&hub, "k", ProfileKind::Kid, 2);
    let got = join(&hub, &mut k, &code);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::JoinPending { .. })));
    assert_eq!(hub.seat_of("k"), None, "no path seats a kid without approve_join");
    let id = pending_id(&take(&mut a)).expect("host sees the knock");
    send(&hub, &a, ClientMsg::ApproveJoin { request_id: id });
    assert!(hub.seat_of("k").is_some());
    assert!(hub.room_state(&code).unwrap().kid_safe);
    // ...and once a kid is seated, an adult's knock does too.
    let mut b = connect(&hub, "b", ProfileKind::Adult, 3);
    let got = join(&hub, &mut b, &code);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::JoinPending { .. })));
    assert_eq!(hub.seat_of("b"), None);
}

#[tokio::test(start_paused = true)]
async fn o8_the_toggle_gates_adult_rooms() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let mut req = room_req("test");
    req.approve_joins = true;
    send(&hub, &a, ClientMsg::CreateRoom(req));
    let code = take(&mut a)
        .into_iter()
        .find_map(|o| match o.msg {
            ServerMsg::Joined { code, .. } => Some(code),
            _ => None,
        })
        .unwrap();
    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    assert!(join(&hub, &mut b, &code).iter().any(|o| matches!(o.msg, ServerMsg::JoinPending { .. })));
}

#[tokio::test(start_paused = true)]
async fn a3_5_deny_and_expiry() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test4");
    let mut k1 = connect(&hub, "k1", ProfileKind::Kid, 2);
    join(&hub, &mut k1, &code);
    let id = pending_id(&take(&mut a)).unwrap();
    send(&hub, &a, ClientMsg::DenyJoin { request_id: id });
    assert!(take(&mut k1).iter().any(|o| matches!(&o.msg, ServerMsg::Denied { reason } if reason == "host")));

    let mut k2 = connect(&hub, "k2", ProfileKind::Kid, 3);
    join(&hub, &mut k2, &code);
    advance(&hub, Duration::from_secs(119)).await;
    assert!(take(&mut k2).is_empty(), "still pending at 119 s");
    advance(&hub, Duration::from_secs(2)).await;
    assert!(take(&mut k2).iter().any(|o| matches!(&o.msg, ServerMsg::Denied { reason } if reason == "expired")));
    assert_eq!(hub.seat_of("k2"), None);
}

#[tokio::test(start_paused = true)]
async fn a3_3_host_migration_skips_kids_and_no_adult_closes() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test4");
    let mut k = connect(&hub, "k", ProfileKind::Kid, 2);
    join(&hub, &mut k, &code);
    let id = pending_id(&take(&mut a)).unwrap();
    send(&hub, &a, ClientMsg::ApproveJoin { request_id: id });
    let mut b = connect(&hub, "b", ProfileKind::Adult, 3);
    join(&hub, &mut b, &code);
    let id = pending_id(&take(&mut a)).unwrap();
    send(&hub, &a, ClientMsg::ApproveJoin { request_id: id });
    // The kid joined before b, but host skips the kid.
    send(&hub, &a, ClientMsg::Leave {});
    assert_eq!(hub.host_of(&code).as_deref(), Some("b"));
    // b leaves: only the kid remains, so the room closes.
    send(&hub, &b, ClientMsg::Leave {});
    assert!(!hub.room_exists(&code));
    assert!(take(&mut k).iter().any(|o| matches!(&o.msg, ServerMsg::MatchEnded { reason } if reason == "host_left")));
    assert_eq!(hub.seat_of("k"), None);
}

#[tokio::test(start_paused = true)]
async fn a3_2_room_lifetimes_under_paused_time() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test");
    // The host drops. The seat is held 60 s; when it is released no adult is
    // left, so the room closes at once (A3.3) -- which is why the 10-minute
    // empty-room rule is only a safety net: an empty room has no host.
    hub.detach("a", 1, Instant::now());
    advance(&hub, Duration::from_secs(59)).await;
    assert!(hub.room_exists(&code), "the seat hold keeps the room");
    advance(&hub, Duration::from_secs(2)).await;
    assert!(!hub.room_exists(&code));

    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    let code = create(&hub, &mut b, "test");
    advance(&hub, ROOM_LIFETIME - Duration::from_secs(1)).await;
    assert!(hub.room_exists(&code));
    advance(&hub, Duration::from_secs(1)).await;
    assert!(!hub.room_exists(&code));
    assert!(take(&mut b).iter().any(|o| matches!(&o.msg, ServerMsg::MatchEnded { reason } if reason == "room_expired")));
}

#[tokio::test(start_paused = true)]
async fn one_live_room_per_player() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    create(&hub, &mut a, "test");
    send(&hub, &a, ClientMsg::CreateRoom(room_req("test")));
    assert_eq!(errors(&take(&mut a)), vec!["already_in_room"]);
}

#[tokio::test(start_paused = true)]
async fn only_the_host_starts_and_needs_two_players() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let code = create(&hub, &mut a, "test");
    send(&hub, &a, ClientMsg::StartMatch {});
    assert_eq!(errors(&take(&mut a)), vec!["not_enough_players"]);
    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    join(&hub, &mut b, &code);
    send(&hub, &b, ClientMsg::StartMatch {});
    assert_eq!(errors(&take(&mut b)), vec!["not_host"]);
    send(&hub, &a, ClientMsg::StartMatch {});
    assert!(take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::TurnStart { seat: 0, .. })));
    assert_eq!(hub.room_state(&code).unwrap().phase, "playing");
}

#[tokio::test(start_paused = true)]
async fn unknown_modes_languages_and_tiers_refuse() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    send(&hub, &a, ClientMsg::CreateRoom(room_req("chess")));
    assert_eq!(errors(&take(&mut a)), vec!["unknown_mode"]);
    let mut req = room_req("test");
    req.lang = "xx".into();
    send(&hub, &a, ClientMsg::CreateRoom(req));
    assert_eq!(errors(&take(&mut a)), vec!["lang_unavailable"]);
    let mut req = room_req("test");
    req.tier_mode = TierMode::Fixed;
    send(&hub, &a, ClientMsg::CreateRoom(req));
    assert_eq!(errors(&take(&mut a)), vec!["bad_tier"]);
}

#[tokio::test(start_paused = true)]
async fn a4_1_join_failures_rate_limit_through_the_hub() {
    let hub = Hub::new();
    let mut p = connect(&hub, "p", ProfileKind::Adult, 1);
    for _ in 0..10 {
        assert_eq!(errors(&join(&hub, &mut p, "ACDEFG")), vec!["room_not_found"]);
    }
    assert_eq!(errors(&join(&hub, &mut p, "ACDEFG")), vec!["rate_limited"]);
    assert_eq!(errors(&join(&hub, &mut p, "ACDEFH")), vec!["rate_limited"], "blocked for 60 s");
}

#[tokio::test(start_paused = true)]
async fn draining_refuses_new_rooms() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    hub.start_drain(123);
    assert!(take(&mut a).iter().any(|o| matches!(o.msg, ServerMsg::ServerDraining { deadline_unix_ms: 123 })));
    send(&hub, &a, ClientMsg::CreateRoom(room_req("test")));
    assert_eq!(errors(&take(&mut a)), vec!["draining"]);
}

#[tokio::test(start_paused = true)]
async fn emotes_are_preset_ids_only() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    let mut b = connect(&hub, "b", ProfileKind::Adult, 2);
    let code = create(&hub, &mut a, "test");
    join(&hub, &mut b, &code);
    take(&mut a);
    send(&hub, &b, ClientMsg::Emote { id: 7 });
    assert!(take(&mut a).iter().any(|o| matches!(o.msg, ServerMsg::Emote { seat: 1, id: 7 })));
    send(&hub, &b, ClientMsg::Emote { id: 8 });
    assert_eq!(errors(&take(&mut b)), vec!["bad_emote"]);
}

// ---------------------------------------------------------------- F5 + A1.7

async fn started(hub: &Hub) -> (P, P, String) {
    let mut a = connect(hub, "a", ProfileKind::Adult, 1);
    let mut b = connect(hub, "b", ProfileKind::Adult, 2);
    let code = create(hub, &mut a, "test");
    join(hub, &mut b, &code);
    send(hub, &a, ClientMsg::StartMatch {});
    take(&mut a);
    take(&mut b);
    (a, b, code)
}

#[tokio::test(start_paused = true)]
async fn a1_7_out_of_turn_messages_never_change_state() {
    let hub = Hub::new();
    let (_a, mut b, code) = started(&hub).await;
    let before = serde_json::to_string(&hub.room_state(&code).unwrap()).unwrap();
    send(&hub, &b, ClientMsg::NextWord {});
    send(&hub, &b, ClientMsg::Submit { typed: "right".into() });
    send(&hub, &b, ClientMsg::ReplayAudio { variant: Variant::Normal });
    assert_eq!(errors(&take(&mut b)), vec!["not_your_turn", "not_your_turn"]);
    let after = serde_json::to_string(&hub.room_state(&code).unwrap()).unwrap();
    assert_eq!(before, after);
}

#[tokio::test(start_paused = true)]
async fn a5_3_the_same_cid_three_times_is_graded_once() {
    let hub = Hub::new();
    let (a, mut b, _code) = started(&hub).await;
    send(&hub, &a, ClientMsg::NextWord {});
    let cid = Uuid::new_v4();
    for _ in 0..3 {
        send_cid(&hub, &a, cid, ClientMsg::Submit { typed: "right".into() });
    }
    let results = take(&mut b).into_iter().filter(|o| matches!(o.msg, ServerMsg::WordResult { .. })).count();
    assert_eq!(results, 1);
}

fn seqs(v: &[Outbound]) -> Vec<u64> {
    v.iter().map(|o| o.seq).filter(|s| *s > 0).collect()
}

#[tokio::test(start_paused = true)]
async fn a5_1_a_thirty_second_drop_resumes_gapless_in_the_same_seat() {
    let hub = Hub::new();
    let (a, b, code) = started(&hub).await;
    // b has seen everything up to here.
    let seen = hub_last_seq(&hub, &code, "b");
    let before_seat = hub.seat_of("b");
    hub.detach("b", b.sess.conn_id, Instant::now());
    drop(b.rx);
    // The match goes on while b is away.
    send(&hub, &a, ClientMsg::NextWord {});
    send(&hub, &a, ClientMsg::Submit { typed: "right".into() });
    send(&hub, &a, ClientMsg::Emote { id: 1 });
    advance(&hub, Duration::from_secs(30)).await;
    assert_eq!(hub.seat_of("b"), before_seat, "the seat is held");
    let mut b2 = connect_resume(&hub, "b", ProfileKind::Adult, 9, Some((code.clone(), seen)));
    let got = take(&mut b2);
    let s = seqs(&got);
    assert!(!s.is_empty());
    assert_eq!(s[0], seen + 1, "replay starts right after last_seq");
    assert!(s.windows(2).all(|w| w[1] == w[0] + 1), "gapless, no duplicates: {s:?}");
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::WordResult { seat: 0, .. })), "missed events arrive");
    assert!(!got.iter().any(|o| matches!(o.msg, ServerMsg::RoomState(_)) && o.seq <= seen));
    assert_eq!(hub.seat_of("b"), before_seat);
}

/// The last seq queued for a member: read back from a fresh room_state
/// subscription would perturb it, so look at the ring directly.
fn hub_last_seq(hub: &Hub, code: &str, player: &str) -> u64 {
    let s = hub.state.lock().unwrap();
    let room = &s.rooms[code];
    let seat = room.seat_of(player).unwrap();
    room.seats[&seat].outbox.next_seq - 1
}

#[tokio::test(start_paused = true)]
async fn a5_2_a_sixty_one_second_drop_releases_the_seat_and_tells_the_room() {
    let hub = Hub::new();
    let (mut a, b, code) = started(&hub).await;
    hub.detach("b", b.sess.conn_id, Instant::now());
    take(&mut a);
    advance(&hub, Duration::from_secs(59)).await;
    assert!(hub.seat_of("b").is_some());
    advance(&hub, Duration::from_secs(2)).await;
    assert_eq!(hub.seat_of("b"), None, "seat released after 60 s");
    let got = take(&mut a);
    assert!(got.iter().any(|o| matches!(&o.msg, ServerMsg::MatchEnded { reason } if reason == "player_left")));
    assert!(got.iter().any(|o| matches!(&o.msg, ServerMsg::RoomState(s) if s.seats.len() == 1)));
    assert_eq!(hub.room_state(&code).unwrap().phase, "lobby");
}

#[tokio::test(start_paused = true)]
async fn a5_4_a_six_hundred_event_gap_takes_the_snapshot_path() {
    let hub = Hub::new();
    let (a, b, code) = started(&hub).await;
    let seen = hub_last_seq(&hub, &code, "b");
    hub.detach("b", b.sess.conn_id, Instant::now());
    for _ in 0..600 {
        send(&hub, &a, ClientMsg::Emote { id: 2 });
    }
    let mut b2 = connect_resume(&hub, "b", ProfileKind::Adult, 9, Some((code.clone(), seen)));
    let got = take(&mut b2);
    let snaps: Vec<&RoomState> = got
        .iter()
        .filter_map(|o| match &o.msg {
            ServerMsg::RoomState(s) => Some(s),
            _ => None,
        })
        .collect();
    assert!(!got.iter().any(|o| matches!(o.msg, ServerMsg::Emote { .. })), "no partial replay");
    assert!(!snaps.is_empty(), "a full snapshot instead");
    assert_eq!(snaps[0], &hub.room_state(&code).unwrap(), "the snapshot is the live state");
}

#[tokio::test(start_paused = true)]
async fn a_newer_connection_replaces_the_old_one() {
    let hub = Hub::new();
    let mut a1 = connect(&hub, "a", ProfileKind::Adult, 1);
    let _a2 = connect(&hub, "a", ProfileKind::Adult, 2);
    let closed = std::iter::from_fn(|| a1.rx.try_recv().ok())
        .any(|o| o == Out::Close(crate::protocol::close::REPLACED, "replaced"));
    assert!(closed);
    hub.detach("a", 1, Instant::now());
    assert!(hub.is_connected("a"), "the stale connection's detach is ignored");
}

// ---------------------------------------------------------------- Online Head-to-Head in the room (F5, F10)

fn h2h_req(timed: bool) -> CreateRoom {
    CreateRoom {
        mode: "h2h".into(),
        lang: "en".into(),
        tier_mode: if timed { TierMode::Fixed } else { TierMode::Climb },
        tier: timed.then(|| "easy".into()),
        timed,
        approve_joins: false,
    }
}

async fn h2h_started(hub: &Hub, timed: bool) -> (P, P, String) {
    let mut a = connect(hub, "a", ProfileKind::Adult, 1);
    let mut b = connect(hub, "b", ProfileKind::Adult, 2);
    send(hub, &a, ClientMsg::CreateRoom(h2h_req(timed)));
    let code = take(&mut a)
        .into_iter()
        .find_map(|o| match o.msg {
            ServerMsg::Joined { code, .. } => Some(code),
            _ => None,
        })
        .unwrap();
    join(hub, &mut b, &code);
    send(hub, &a, ClientMsg::StartMatch {});
    take(&mut a);
    take(&mut b);
    (a, b, code)
}

/// Advances in 250 ms steps, ticking like the background task does.
async fn run_for(hub: &Hub, d: Duration) {
    let steps = d.as_millis().div_ceil(250) as u32;
    for _ in 0..steps {
        tokio::time::advance(Duration::from_millis(250)).await;
        hub.tick(Instant::now());
    }
}

#[tokio::test(start_paused = true)]
async fn a5_5_an_idle_untimed_word_is_a_miss_after_120_s_and_the_turn_passes() {
    let hub = Hub::new();
    let (a, mut b, _code) = h2h_started(&hub, false).await;
    send(&hub, &a, ClientMsg::NextWord {});
    assert!(take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::WordStart { seat: 0, .. })));
    run_for(&hub, Duration::from_secs(119)).await;
    assert!(!take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::WordResult { .. })));
    run_for(&hub, Duration::from_secs(2)).await;
    run_for(&hub, Duration::from_millis(1500)).await;
    let got = take(&mut b);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::WordResult { seat: 0, correct: false, .. })));
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::TurnStart { seat: 1, .. })), "the turn passes");
}

#[tokio::test(start_paused = true)]
async fn a5_2_an_active_seat_gone_61_s_in_an_untimed_room_misses_and_releases() {
    let hub = Hub::new();
    let (a, mut b, code) = h2h_started(&hub, false).await;
    send(&hub, &a, ClientMsg::NextWord {});
    hub.detach("a", a.sess.conn_id, Instant::now());
    take(&mut b);
    // The idle rule waits for the seat hold instead: still nothing at 59 s,
    // and nothing even past 120 s of the word if the seat were still held.
    run_for(&hub, Duration::from_secs(59)).await;
    assert!(!take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::WordResult { .. })));
    run_for(&hub, Duration::from_secs(2)).await;
    let got = take(&mut b);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::WordResult { seat: 0, correct: false, .. })));
    assert!(got.iter().any(|o| matches!(&o.msg, ServerMsg::MatchEnded { reason } if reason == "player_left")));
    assert_eq!(hub.seat_of("a"), None);
    assert_eq!(hub.room_state(&code).unwrap().phase, "lobby");
}

#[tokio::test(start_paused = true)]
async fn a_timed_deadline_keeps_running_while_the_active_seat_is_away() {
    let hub = Hub::new();
    let (a, mut b, _code) = h2h_started(&hub, true).await;
    send(&hub, &a, ClientMsg::NextWord {});
    hub.detach("a", a.sess.conn_id, Instant::now());
    take(&mut b);
    run_for(&hub, Duration::from_millis(12_500)).await;
    let got = take(&mut b);
    assert!(got.iter().any(|o| matches!(o.msg, ServerMsg::WordResult { seat: 0, correct: false, .. })), "timed out while away");
    assert!(hub.seat_of("a").is_some(), "the seat is still held");
}

#[tokio::test(start_paused = true)]
async fn coming_back_restarts_the_idle_clock() {
    let hub = Hub::new();
    let (a, mut b, code) = h2h_started(&hub, false).await;
    send(&hub, &a, ClientMsg::NextWord {});
    hub.detach("a", a.sess.conn_id, Instant::now());
    run_for(&hub, Duration::from_secs(50)).await;
    let _a2 = connect_resume(&hub, "a", ProfileKind::Adult, 7, Some((code, 0)));
    take(&mut b);
    run_for(&hub, Duration::from_secs(110)).await;
    assert!(!take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::WordResult { .. })), "a fresh 120 s after return");
    run_for(&hub, Duration::from_secs(11)).await;
    assert!(take(&mut b).iter().any(|o| matches!(o.msg, ServerMsg::WordResult { correct: false, .. })));
}

#[tokio::test(start_paused = true)]
async fn tickets_are_room_bound_and_expire_60_s_after_the_word_resolves() {
    let hub = Hub::new();
    let (a, mut b, _code) = h2h_started(&hub, false).await;
    let _outsider = connect(&hub, "z", ProfileKind::Adult, 9);
    send(&hub, &a, ClientMsg::NextWord {});
    let ticket = take(&mut b)
        .into_iter()
        .find_map(|o| match o.msg {
            ServerMsg::WordStart { audio_ticket, .. } => Some(audio_ticket),
            _ => None,
        })
        .unwrap();
    assert_eq!(ticket.len(), 32, "128-bit hex");
    let now = Instant::now();
    assert!(hub.audio_lookup(&ticket, "a", now).is_some());
    assert!(hub.audio_lookup(&ticket, "b", now).is_some(), "the watcher may hear it too");
    assert!(hub.audio_lookup(&ticket, "z", now).is_none(), "not a member");
    assert!(hub.audio_lookup("00000000000000000000000000000000", "a", now).is_none());
    send(&hub, &a, ClientMsg::Submit { typed: "zzzzqqq".into() });
    run_for(&hub, Duration::from_secs(59)).await;
    assert!(hub.audio_lookup(&ticket, "a", Instant::now()).is_some(), "valid until resolve + 60 s");
    run_for(&hub, Duration::from_secs(2)).await;
    assert!(hub.audio_lookup(&ticket, "a", Instant::now()).is_none());
}

#[tokio::test(start_paused = true)]
async fn a_kid_safe_h2h_room_plays_two_turns_each() {
    let hub = Hub::new();
    let mut a = connect(&hub, "a", ProfileKind::Adult, 1);
    send(&hub, &a, ClientMsg::CreateRoom(h2h_req(false)));
    let code = take(&mut a)
        .into_iter()
        .find_map(|o| match o.msg {
            ServerMsg::Joined { code, .. } => Some(code),
            _ => None,
        })
        .unwrap();
    let mut k = connect(&hub, "k", ProfileKind::Kid, 2);
    join(&hub, &mut k, &code);
    let id = pending_id(&take(&mut a)).unwrap();
    send(&hub, &a, ClientMsg::ApproveJoin { request_id: id });
    send(&hub, &a, ClientMsg::StartMatch {});
    assert!(take(&mut k).iter().any(|o| matches!(o.msg, ServerMsg::TurnStart { turns_left: 2, .. })));
}

#[tokio::test(start_paused = true)]
async fn rtt_samples_become_capped_grace() {
    let hub = Hub::new();
    let (_a, _b, code) = h2h_started(&hub, true).await;
    for r in [40, 50, 900, 60, 45] {
        hub.note_rtt("a", r);
    }
    {
        let s = hub.state.lock().unwrap();
        let room = &s.rooms[&code];
        assert_eq!(room.seats[&0].grace(), Duration::from_millis(25), "median 50 / 2");
    }
    for r in [900, 900, 900, 900, 900] {
        hub.note_rtt("a", r);
    }
    let s = hub.state.lock().unwrap();
    assert_eq!(s.rooms[&code].seats[&0].grace(), GRACE_CAP, "capped at 300 ms");
}
