//! Phase A acceptance over real sockets: F1, F2, F4, F5 (end to end), F11.
//! Needs the `test-modes` feature (`cargo test -p spell-server --all-features`).
#![cfg(feature = "test-modes")]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use spell_server::app::AppState;
use spell_server::config::Config;
use spell_server::protocol::{
    close, decode_server, encode_client, ClientMsg, CreateRoom, Hello, Outbound, ProfileKind, Resume, ServerMsg,
    TierMode,
};
use spell_server::store::Store;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

const ORIGIN: &str = "capacitor://localhost";

// ---------------------------------------------------------------- log capture (A11.2, A2.1)

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn logs() -> &'static Capture {
    static CAP: OnceLock<Capture> = OnceLock::new();
    CAP.get_or_init(|| {
        let cap = Capture::default();
        let w = cap.clone();
        tracing_subscriber::fmt()
            .json()
            .with_env_filter("info")
            .with_writer(move || w.clone())
            .init();
        cap
    })
}

fn log_text() -> String {
    String::from_utf8_lossy(&logs().0.lock().unwrap()).into_owned()
}

// ---------------------------------------------------------------- server harness

struct Srv {
    app: Arc<AppState>,
    public: SocketAddr,
    admin: SocketAddr,
    db: String,
    done: tokio::task::JoinHandle<std::io::Result<()>>,
}

fn temp_db() -> String {
    let dir = std::env::temp_dir().join(format!("spell-server-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("spell.db").to_string_lossy().into_owned()
}

async fn start_with(edit: impl FnOnce(&mut Config)) -> Srv {
    logs();
    let db = temp_db();
    let mut cfg = Config::for_tests(&db);
    edit(&mut cfg);
    let store = Store::open(&db, Default::default()).unwrap();
    let app = AppState::new(cfg, store);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let a = app.clone();
    let done = tokio::spawn(async move { spell_server::serve(a, Some(tx)).await });
    let (public, admin) = rx.await.unwrap();
    Srv { app, public, admin, db, done }
}

async fn start() -> Srv {
    start_with(|_| {}).await
}

fn http() -> reqwest::Client {
    reqwest::Client::new()
}

async fn mint(srv: &Srv, ip: &str) -> (String, String) {
    let r = http()
        .post(format!("http://{}/v1/session", srv.public))
        .header("Origin", ORIGIN)
        .header("CF-Connecting-IP", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    (v["player_id"].as_str().unwrap().into(), v["token"].as_str().unwrap().into())
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn ws_raw(srv: &Srv, path: &str, origin: Option<&str>) -> Result<Ws, tokio_tungstenite::tungstenite::Error> {
    let mut req = format!("ws://{}{}", srv.public, path).into_client_request().unwrap();
    if let Some(o) = origin {
        req.headers_mut().insert("Origin", o.parse().unwrap());
    }
    req.headers_mut().insert("CF-Connecting-IP", "203.0.113.77".parse().unwrap());
    tokio_tungstenite::connect_async(req).await.map(|(ws, _)| ws)
}

fn hello(token: &str, kind: ProfileKind, name: Option<&str>, resume: Option<Resume>) -> ClientMsg {
    ClientMsg::Hello(Hello {
        protocol: 1,
        app_build: "300".into(),
        platform: "ios".into(),
        profile_kind: kind,
        ui_lang: "en".into(),
        display_name: name.map(str::to_string),
        token: token.into(),
        resume,
    })
}

async fn tx(ws: &mut Ws, msg: &ClientMsg) {
    ws.send(Message::Text(encode_client(Uuid::new_v4(), msg).into())).await.unwrap();
}

/// Next server frame, or the close code if the server closed.
async fn rx(ws: &mut Ws) -> Result<Outbound, Option<u16>> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => return Ok(decode_server(&t).expect("server frame")),
            Ok(Some(Ok(Message::Close(f)))) => return Err(f.map(|f| u16::from(f.code))),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(_))) | Ok(None) => return Err(None),
            Err(_) => panic!("timed out waiting for a frame"),
        }
    }
}

async fn until(ws: &mut Ws, pred: impl Fn(&ServerMsg) -> bool) -> Outbound {
    loop {
        let o = rx(ws).await.unwrap_or_else(|c| panic!("closed ({c:?}) before the expected frame"));
        if pred(&o.msg) {
            return o;
        }
    }
}

async fn closed_with(ws: &mut Ws) -> Option<u16> {
    loop {
        match rx(ws).await {
            Ok(_) => continue,
            Err(code) => return code,
        }
    }
}

/// Mint + connect + hello + welcome.
async fn player(srv: &Srv, ip: &str, kind: ProfileKind, name: &str) -> (Ws, String, String) {
    let (id, token) = mint(srv, ip).await;
    let mut ws = ws_raw(srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    tx(&mut ws, &hello(&token, kind, Some(name), None)).await;
    let w = rx(&mut ws).await.unwrap();
    assert!(matches!(w.msg, ServerMsg::Welcome { .. }), "{w:?}");
    (ws, id, token)
}

fn create_msg(mode: &str) -> ClientMsg {
    ClientMsg::CreateRoom(CreateRoom {
        mode: mode.into(),
        lang: "en".into(),
        tier_mode: TierMode::Climb,
        tier: None,
        timed: false,
        approve_joins: false,
    })
}

async fn create(ws: &mut Ws) -> String {
    tx(ws, &create_msg("test")).await;
    match until(ws, |m| matches!(m, ServerMsg::Joined { .. })).await.msg {
        ServerMsg::Joined { code, .. } => code,
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------- F1

#[tokio::test]
async fn a1_2_an_old_protocol_gets_update_required_then_4001() {
    let srv = start().await;
    let (_, token) = mint(&srv, "198.51.100.10").await;
    let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    let mut h = hello(&token, ProfileKind::Adult, Some("Mom"), None);
    if let ClientMsg::Hello(ref mut x) = h {
        x.protocol = 0;
    }
    tx(&mut ws, &h).await;
    assert!(matches!(rx(&mut ws).await.unwrap().msg, ServerMsg::UpdateRequired { min: 1, current: 1 }));
    assert_eq!(closed_with(&mut ws).await, Some(close::UPDATE_REQUIRED));
}

#[tokio::test]
async fn a1_3_a_4097_byte_frame_closes_1009() {
    let srv = start().await;
    let (mut ws, _, _) = player(&srv, "198.51.100.11", ProfileKind::Adult, "Mom").await;
    ws.send(Message::Text("x".repeat(4097).into())).await.unwrap();
    assert_eq!(closed_with(&mut ws).await, Some(close::TOO_LARGE));
    // ...and exactly 4096 is still a (bad) frame, not a close.
    let (mut ws, _, _) = player(&srv, "198.51.100.12", ProfileKind::Adult, "Mom").await;
    ws.send(Message::Text("x".repeat(4096).into())).await.unwrap();
    assert!(matches!(rx(&mut ws).await.unwrap().msg, ServerMsg::Error { .. }));
}

#[tokio::test]
async fn a1_4_any_frame_before_hello_closes_4002() {
    let srv = start().await;
    let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    tx(&mut ws, &ClientMsg::Leave {}).await;
    assert_eq!(closed_with(&mut ws).await, Some(close::NO_HELLO));
    let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    ws.send(Message::Text("not json".into())).await.unwrap();
    assert_eq!(closed_with(&mut ws).await, Some(close::NO_HELLO));
}

#[tokio::test]
async fn a1_5_unknown_types_error_then_five_close_4003() {
    let srv = start().await;
    let (mut ws, _, _) = player(&srv, "198.51.100.13", ProfileKind::Adult, "Mom").await;
    let frame = |t: &str| format!(r#"{{"v":1,"t":"{t}","cid":"{}","body":{{}}}}"#, Uuid::new_v4());
    for i in 0..4 {
        ws.send(Message::Text(frame("chat").into())).await.unwrap();
        let o = rx(&mut ws).await.unwrap();
        assert_eq!(o.msg, ServerMsg::error("unknown_type"), "frame {i}");
    }
    ws.send(Message::Text(frame("chat").into())).await.unwrap();
    assert_eq!(closed_with(&mut ws).await, Some(close::TOO_MANY_BAD));
}

// ---------------------------------------------------------------- F2

#[tokio::test]
async fn a2_1_no_token_bytes_in_the_database_or_logs() {
    let srv = start().await;
    let mut tokens = Vec::new();
    for i in 0..100 {
        let ip = format!("10.1.{}.{}", i / 5, i % 5); // stays under 5 mints per IP per hour
        let (_, t) = mint(&srv, &ip).await;
        tokens.push(t);
    }
    // Use a few, so authenticate paths log too.
    for t in tokens.iter().take(5) {
        let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
        tx(&mut ws, &hello(t, ProfileKind::Adult, Some("Mom"), None)).await;
        rx(&mut ws).await.unwrap();
    }
    srv.app.store.snapshot(&format!("{}.copy", srv.db)).unwrap();
    let mut bytes = Vec::new();
    for f in [srv.db.clone(), format!("{}-wal", srv.db), format!("{}.copy", srv.db)] {
        if let Ok(b) = std::fs::read(&f) {
            bytes.extend(b);
        }
    }
    let logs = log_text();
    assert!(!bytes.is_empty());
    for t in &tokens {
        assert!(!bytes.windows(t.len()).any(|w| w == t.as_bytes()), "token found in the database");
        assert!(!logs.contains(t.as_str()), "token found in logs");
    }
    assert_eq!(srv.app.store.player_count(), 100);
}

#[tokio::test]
async fn a2_2_a_token_in_the_url_is_400() {
    let srv = start().await;
    let r = http()
        .get(format!("http://{}/v1/ws?token=abc", srv.public))
        .header("Origin", ORIGIN)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}

#[tokio::test]
async fn a2_3_expired_is_session_expired_and_revoked_is_4010() {
    let srv = start().await;
    let (_, token) = mint(&srv, "198.51.100.20").await;
    srv.app.store.clock.advance_days(91);
    let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    tx(&mut ws, &hello(&token, ProfileKind::Adult, Some("Mom"), None)).await;
    assert_eq!(rx(&mut ws).await.unwrap().msg, ServerMsg::error("session_expired"));
    assert_eq!(closed_with(&mut ws).await, Some(close::SESSION));

    // Revoked while connected: closed at once with 4010; and refused after.
    let (mut ws, id, token) = player(&srv, "198.51.100.21", ProfileKind::Adult, "Mom").await;
    let r = http().post(format!("http://{}/admin/revoke/{id}", srv.admin)).send().await.unwrap();
    assert_eq!(r.status(), 204);
    assert_eq!(closed_with(&mut ws).await, Some(close::REVOKED));
    let mut ws = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    tx(&mut ws, &hello(&token, ProfileKind::Adult, Some("Mom"), None)).await;
    assert_eq!(closed_with(&mut ws).await, Some(close::REVOKED));
}

#[tokio::test]
async fn a2_4_the_sixth_mint_from_one_ip_in_an_hour_is_429() {
    let srv = start().await;
    for _ in 0..5 {
        mint(&srv, "192.0.2.50").await;
    }
    let r = http()
        .post(format!("http://{}/v1/session", srv.public))
        .header("Origin", ORIGIN)
        .header("CF-Connecting-IP", "192.0.2.50")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 429);
}

// ---------------------------------------------------------------- F4

#[tokio::test]
async fn a4_3_a_message_flood_closes_4008() {
    let srv = start().await;
    let (mut ws, _, _) = player(&srv, "198.51.100.30", ProfileKind::Adult, "Mom").await;
    for _ in 0..60 {
        let _ = ws.send(Message::Text(encode_client(Uuid::new_v4(), &ClientMsg::Emote { id: 1 }).into())).await;
    }
    assert_eq!(closed_with(&mut ws).await, Some(close::FLOOD));
}

#[tokio::test]
async fn a4_4_a_disallowed_origin_is_403() {
    let srv = start().await;
    for origin in [Some("https://evil.example"), Some("https://localhost"), None] {
        match ws_raw(&srv, "/v1/ws", origin).await {
            Err(tokio_tungstenite::tungstenite::Error::Http(r)) => assert_eq!(r.status(), 403, "{origin:?}"),
            other => panic!("expected 403 for {origin:?}, got {other:?}"),
        }
        let mut req = http().post(format!("http://{}/v1/session", srv.public));
        if let Some(o) = origin {
            req = req.header("Origin", o);
        }
        assert_eq!(req.send().await.unwrap().status(), 403, "{origin:?}");
    }
}

#[tokio::test]
async fn a4_5_the_connection_cap_is_503_at_upgrade() {
    let srv = start_with(|c| c.max_conns = 1).await;
    let _first = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    match ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await {
        Err(tokio_tungstenite::tungstenite::Error::Http(r)) => assert_eq!(r.status(), 503),
        other => panic!("expected 503, got {other:?}"),
    }
}

#[tokio::test]
async fn cors_preflight_for_the_app_origin() {
    let srv = start().await;
    let r = http()
        .request(reqwest::Method::OPTIONS, format!("http://{}/v1/audio/abc", srv.public))
        .header("Origin", ORIGIN)
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "authorization")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    assert_eq!(r.headers()["access-control-allow-origin"], ORIGIN);
    assert!(r.headers()["access-control-allow-headers"].to_str().unwrap().contains("authorization"));
}

// ---------------------------------------------------------------- F5 end to end

#[tokio::test]
async fn a5_resume_over_a_real_socket_replays_what_was_missed() {
    let srv = start().await;
    let (mut a, _, _) = player(&srv, "198.51.100.40", ProfileKind::Adult, "Mom").await;
    let (mut b, _, b_token) = player(&srv, "198.51.100.41", ProfileKind::Adult, "Dad").await;
    let code = create(&mut a).await;
    tx(&mut b, &ClientMsg::JoinRoom { code: code.to_lowercase() }).await;
    let last = until(&mut b, |m| matches!(m, ServerMsg::RoomState(s) if s.seats.len() == 2)).await.seq;
    drop(b);
    tokio::time::sleep(Duration::from_millis(100)).await;
    for _ in 0..3 {
        tx(&mut a, &ClientMsg::Emote { id: 4 }).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut b = ws_raw(&srv, "/v1/ws", Some(ORIGIN)).await.unwrap();
    tx(&mut b, &hello(&b_token, ProfileKind::Adult, Some("Dad"), Some(Resume { room: code, last_seq: last })))
        .await;
    assert!(matches!(rx(&mut b).await.unwrap().msg, ServerMsg::Welcome { .. }));
    let mut seqs = Vec::new();
    let mut emotes = 0;
    while emotes < 3 {
        let o = rx(&mut b).await.unwrap();
        if matches!(o.msg, ServerMsg::Emote { id: 4, .. }) {
            emotes += 1;
        }
        seqs.push(o.seq);
    }
    assert_eq!(seqs[0], last + 1);
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1), "{seqs:?}");
}

// ---------------------------------------------------------------- F11

#[tokio::test]
async fn a11_1_the_public_route_table_is_exactly_four_paths() {
    let srv = start().await;
    let c = http();
    assert_eq!(c.get(format!("http://{}/healthz", srv.public)).send().await.unwrap().status(), 200);
    for path in [
        "/",
        "/admin/stats",
        "/admin/readyz",
        "/admin/drain",
        "/admin/snapshot",
        "/admin/revoke/x",
        "/admin/test-sessions",
        "/v1",
        "/v1/session/x",
        "/v2/ws",
        "/api/speak",
        "/healthz/x",
        "/.env",
    ] {
        for m in [reqwest::Method::GET, reqwest::Method::POST] {
            let r = c.request(m.clone(), format!("http://{}{path}", srv.public)).send().await.unwrap();
            assert_eq!(r.status(), 404, "{m} {path} on the public listener");
        }
    }
    // Wrong method on a real path is 404 too, not 405.
    assert_eq!(c.post(format!("http://{}/healthz", srv.public)).send().await.unwrap().status(), 404);
    // The admin listener has the admin routes and nothing public.
    assert_eq!(c.get(format!("http://{}/admin/stats", srv.admin)).send().await.unwrap().status(), 200);
    assert_eq!(c.get(format!("http://{}/healthz", srv.admin)).send().await.unwrap().status(), 404);
    assert_eq!(c.post(format!("http://{}/v1/session", srv.admin)).send().await.unwrap().status(), 404);
    let snap = c.post(format!("http://{}/admin/snapshot", srv.admin)).send().await.unwrap();
    assert_eq!(snap.status(), 200);
    // F16's batch mint exists on the admin listener only.
    let ts = c.post(format!("http://{}/admin/test-sessions?n=3", srv.admin)).send().await.unwrap();
    assert_eq!(ts.status(), 200);
    let v: serde_json::Value = serde_json::from_str(&ts.text().await.unwrap()).unwrap();
    assert_eq!(v["sessions"].as_array().unwrap().len(), 3);
    assert_eq!(c.post(format!("http://{}/admin/test-sessions?n=5000", srv.admin)).send().await.unwrap().status(), 400);
    // readyz is false here: the test TTS base is unreachable.
    assert_eq!(c.get(format!("http://{}/admin/readyz", srv.admin)).send().await.unwrap().status(), 503);
}

#[tokio::test]
async fn a11_2_a_full_match_logs_no_tokens_names_or_full_ips() {
    let srv = start().await;
    let (mut a, _, ta) = player(&srv, "203.0.113.77", ProfileKind::Adult, "Grandma Jo").await;
    let (mut b, _, tb) = player(&srv, "203.0.113.78", ProfileKind::Adult, "Uncle Bob").await;
    let code = create(&mut a).await;
    tx(&mut b, &ClientMsg::JoinRoom { code: code.clone() }).await;
    until(&mut a, |m| matches!(m, ServerMsg::RoomState(s) if s.seats.len() == 2)).await;
    tx(&mut a, &ClientMsg::StartMatch {}).await;
    // The test mode: 3 submits per seat, alternating, a then b.
    for i in 0..6 {
        let (ws, me) = if i % 2 == 0 { (&mut a, 0) } else { (&mut b, 1) };
        until(ws, |m| matches!(m, ServerMsg::TurnStart { seat, .. } if *seat == me)).await;
        tx(ws, &ClientMsg::NextWord {}).await;
        until(ws, |m| matches!(m, ServerMsg::WordStart { seat, .. } if *seat == me)).await;
        tx(ws, &ClientMsg::Submit { typed: "right".into() }).await;
    }
    until(&mut a, |m| matches!(m, ServerMsg::MatchResult { .. })).await;
    drop(a);
    drop(b);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let logs = log_text();
    assert!(logs.contains("match_started"), "the scrape saw this match");
    for needle in [ta.as_str(), tb.as_str(), "Grandma Jo", "Uncle Bob", "203.0.113.77", "203.0.113.78", &code] {
        assert!(!logs.contains(needle), "{needle:?} leaked into logs");
    }
    assert!(logs.contains("203.0.113.0/24"), "truncated IPs are logged");
}

#[tokio::test]
async fn a11_3_drain_lets_a_live_match_finish_refuses_new_rooms_and_exits_clean() {
    let srv = start().await;
    let (mut a, _, _) = player(&srv, "198.51.100.60", ProfileKind::Adult, "Mom").await;
    let (mut b, _, _) = player(&srv, "198.51.100.61", ProfileKind::Adult, "Dad").await;
    let (mut c, _, _) = player(&srv, "198.51.100.62", ProfileKind::Adult, "Aunt").await;
    let code = create(&mut a).await;
    tx(&mut b, &ClientMsg::JoinRoom { code }).await;
    until(&mut a, |m| matches!(m, ServerMsg::RoomState(s) if s.seats.len() == 2)).await;
    tx(&mut a, &ClientMsg::StartMatch {}).await;
    until(&mut a, |m| matches!(m, ServerMsg::TurnStart { .. })).await;

    let r = http().post(format!("http://{}/admin/drain", srv.admin)).send().await.unwrap();
    assert_eq!(r.status(), 202);
    until(&mut c, |m| matches!(m, ServerMsg::ServerDraining { .. })).await;
    tx(&mut c, &create_msg("test")).await;
    assert_eq!(until(&mut c, |m| matches!(m, ServerMsg::Error { .. })).await.msg, ServerMsg::error("draining"));

    // The live match plays to its end.
    for i in 0..6 {
        let (ws, me) = if i % 2 == 0 { (&mut a, 0) } else { (&mut b, 1) };
        if i > 0 {
            until(ws, |m| matches!(m, ServerMsg::TurnStart { seat, .. } if *seat == me)).await;
        }
        tx(ws, &ClientMsg::NextWord {}).await;
        until(ws, |m| matches!(m, ServerMsg::WordStart { seat, .. } if *seat == me)).await;
        tx(ws, &ClientMsg::Submit { typed: "right".into() }).await;
    }
    until(&mut a, |m| matches!(m, ServerMsg::MatchResult { .. })).await;
    let finished = tokio::time::timeout(Duration::from_secs(5), srv.done).await;
    assert!(matches!(finished, Ok(Ok(Ok(())))), "serve returns cleanly after the drain: {finished:?}");
    assert_eq!(closed_with(&mut c).await, Some(close::SHUTDOWN));
}

#[allow(dead_code)]
fn _close_code_is_u16(c: CloseCode) -> u16 {
    c.into()
}
