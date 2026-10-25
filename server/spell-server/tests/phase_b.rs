//! Phase B over real sockets: A9.1 (no answer leakage), A9.2 (ticket
//! access), A9.3 (outage, end to end), A10.3 (kid names), and a full Online
//! Head-to-Head match at the real 1300 ms hand-over.
#![cfg(feature = "test-modes")]

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use md5::{Digest, Md5};
use spell_server::app::AppState;
use spell_server::audio::{AudioSource, Fetch};
use spell_server::config::Config;
use spell_server::mode::AudioKey;
use spell_server::protocol::{
    close, decode_server, encode_client, ClientMsg, CreateRoom, Hello, Outbound, ProfileKind, ServerMsg, TierMode,
    Variant,
};
use spell_server::store::Store;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

const ORIGIN: &str = "capacitor://localhost";

/// A TTS stand-in whose clips are booby-trapped: the word sits in an ID3v2
/// title and an ID3v1 tag, exactly where a real MP3 could carry it. The
/// server must strip both.
#[derive(Default)]
struct Trapped {
    down: AtomicBool,
    calls: AtomicU32,
    seen: Mutex<Vec<AudioKey>>,
}

fn trapped_clip(text: &str) -> Vec<u8> {
    let title = format!("TIT2{text}");
    let size = title.len() as u32;
    let mut v = b"ID3\x04\x00\x00".to_vec();
    v.extend([(size >> 21 & 0x7f) as u8, (size >> 14 & 0x7f) as u8, (size >> 7 & 0x7f) as u8, (size & 0x7f) as u8]);
    v.extend(title.as_bytes());
    v.extend([0xFF, 0xFB, 0x90, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let mut tag = b"TAG".to_vec();
    tag.extend(text.as_bytes().iter().take(30));
    tag.resize(128, 0);
    v.extend(tag);
    v
}

impl AudioSource for Trapped {
    fn fetch<'a>(&'a self, key: &'a AudioKey) -> Fetch<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.seen.lock().unwrap().push(key.clone());
            if self.down.load(Ordering::SeqCst) {
                return Err("tts down".into());
            }
            Ok(trapped_clip(&key.text))
        })
    }
}

struct Srv {
    public: SocketAddr,
    tts: Arc<Trapped>,
    _app: Arc<AppState>,
}

async fn start() -> Srv {
    let dir = std::env::temp_dir().join(format!("spell-server-b-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("spell.db").to_string_lossy().into_owned();
    let tts = Arc::new(Trapped::default());
    let app = AppState::with_audio(Config::for_tests(&db), Store::open(&db, Default::default()).unwrap(), tts.clone());
    let (tx, rx) = tokio::sync::oneshot::channel();
    let a = app.clone();
    tokio::spawn(async move { spell_server::serve(a, Some(tx)).await });
    let (public, _) = rx.await.unwrap();
    Srv { public, tts, _app: app }
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// One test client: its socket, its token, and every byte the server sent it.
struct Client {
    ws: Ws,
    token: String,
    /// In arrival order: ("ws", frame) or ("audio", body).
    got: Vec<(&'static str, Vec<u8>)>,
}

async fn client(srv: &Srv, ip: &str, kind: ProfileKind, name: &str, ui: &str) -> Client {
    let r = reqwest::Client::new()
        .post(format!("http://{}/v1/session", srv.public))
        .header("Origin", ORIGIN)
        .header("CF-Connecting-IP", ip)
        .send()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    let token = v["token"].as_str().unwrap().to_string();
    let mut req = format!("ws://{}/v1/ws", srv.public).into_client_request().unwrap();
    req.headers_mut().insert("Origin", ORIGIN.parse().unwrap());
    let (ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
    let mut c = Client { ws, token: token.clone(), got: Vec::new() };
    c.tx(&ClientMsg::Hello(Hello {
        protocol: 1,
        app_build: "300".into(),
        platform: "ios".into(),
        profile_kind: kind,
        ui_lang: ui.into(),
        display_name: Some(name.into()),
        token,
        resume: None,
    }))
    .await;
    c
}

impl Client {
    async fn tx(&mut self, m: &ClientMsg) {
        self.ws.send(Message::Text(encode_client(Uuid::new_v4(), m).into())).await.unwrap();
    }

    async fn rx(&mut self) -> Result<Outbound, Option<u16>> {
        loop {
            match tokio::time::timeout(Duration::from_secs(20), self.ws.next()).await {
                Ok(Some(Ok(Message::Text(t)))) => {
                    self.got.push(("ws", t.as_bytes().to_vec()));
                    return Ok(decode_server(&t).expect("frame"));
                }
                Ok(Some(Ok(Message::Close(f)))) => return Err(f.map(|f| u16::from(f.code))),
                Ok(Some(Ok(_))) => continue,
                Ok(Some(Err(_))) | Ok(None) => return Err(None),
                Err(_) => panic!("timed out waiting for a frame"),
            }
        }
    }

    async fn until(&mut self, pred: impl Fn(&ServerMsg) -> bool) -> ServerMsg {
        loop {
            let o = self.rx().await.unwrap_or_else(|c| panic!("closed {c:?}"));
            if pred(&o.msg) {
                return o.msg;
            }
        }
    }

    async fn audio(&mut self, srv: &Srv, ticket: &str) -> reqwest::Response {
        let r = reqwest::Client::new()
            .get(format!("http://{}/v1/audio/{ticket}", srv.public))
            .header("Origin", ORIGIN)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .unwrap();
        r
    }

    async fn audio_body(&mut self, srv: &Srv, ticket: &str) -> Vec<u8> {
        let r = self.audio(srv, ticket).await;
        assert_eq!(r.status(), 200);
        let b = r.bytes().await.unwrap().to_vec();
        self.got.push(("audio", b.clone()));
        b
    }
}

fn create(mode: &str, lang: &str) -> ClientMsg {
    ClientMsg::CreateRoom(CreateRoom {
        mode: mode.into(),
        lang: lang.into(),
        tier_mode: TierMode::Climb,
        tier: None,
        timed: false,
        approve_joins: false,
    })
}

async fn room(srv: &Srv, mode: &str, lang: &str) -> (Client, Client) {
    // Distinct IPs per room: the session limit is 5 mints per IP per hour.
    static NEXT: AtomicU32 = AtomicU32::new(1);
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let mut a = client(srv, &format!("10.9.{n}.1"), ProfileKind::Adult, "Zqxj", "en").await;
    let mut b = client(srv, &format!("10.9.{n}.2"), ProfileKind::Adult, "Vkwp", "en").await;
    a.tx(&create(mode, lang)).await;
    let code = match a.until(|m| matches!(m, ServerMsg::Joined { .. })).await {
        ServerMsg::Joined { code, .. } => code,
        _ => unreachable!(),
    };
    b.tx(&ClientMsg::JoinRoom { code }).await;
    a.until(|m| matches!(m, ServerMsg::RoomState(s) if s.seats.len() == 2)).await;
    (a, b)
}

fn active_languages() -> Vec<&'static str> {
    use spell_wasm::native_api as n;
    n::BUILTIN_LANGS.iter().map(|r| r.0).filter(|l| n::is_active_lang(l)).collect()
}

// ---------------------------------------------------------------- A9.1

/// Every form the word could take on the wire: NFC, NFD, lowercase, each
/// half of a Mandarin entry, and the Mac mini's cache-key digests (R4).
fn forms(lang: &str, entry: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut parts: Vec<String> = entry.split('|').map(str::to_string).collect();
    parts.push(entry.to_string());
    for p in parts {
        for f in [p.nfc().collect::<String>(), p.nfd().collect(), p.to_lowercase()] {
            out.insert(f);
        }
        let lower = p.to_lowercase();
        let base = if lang == "en" { lower.clone() } else { format!("{lang}:{lower}") };
        let voice = match lang {
            "en" => Some("en-US-Neural2-E"),
            "de" => Some("de-DE-Neural2-A"),
            "ru" => Some("ru-RU-Wavenet-A"),
            "ko" => Some("ko-KR-Wavenet-D"),
            _ => None,
        };
        let mut keys = vec![base.clone()];
        if let Some(v) = voice {
            keys.push(format!("{base}|voice:{v}"));
        }
        for k in keys {
            out.insert(hex::encode(Md5::digest(k.as_bytes())));
        }
    }
    out
}

/// Protocol vocabulary that may equal a short word without being a leak
/// (a bank word "easy" is not leaked by `tier:"easy"`).
fn is_vocabulary(key: &str, v: &str) -> bool {
    match key {
        "tier" => ["easy", "medium", "hard", "expert"].contains(&v),
        "phase" | "mode" | "lang" | "tier_mode" | "variant" | "t" | "kind" | "platform" => true,
        _ => false,
    }
}

fn strings(v: &serde_json::Value, key: &str, out: &mut Vec<(String, String)>) {
    match v {
        serde_json::Value::String(s) => out.push((key.to_string(), s.clone())),
        serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, key, out)),
        serde_json::Value::Object(m) => m.iter().for_each(|(k, x)| strings(x, k, out)),
        _ => {}
    }
}

fn leaks(got: &[(&'static str, Vec<u8>)], forms: &BTreeSet<String>) -> Vec<String> {
    let mut hits = Vec::new();
    for (kind, bytes) in got {
        match *kind {
            "ws" => {
                let v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                let mut s = Vec::new();
                strings(&v, "", &mut s);
                for (k, val) in s {
                    if is_vocabulary(&k, &val) {
                        continue;
                    }
                    let low = val.to_lowercase();
                    for f in forms {
                        let long = f.chars().count() >= 4;
                        if val == *f || low == *f || (long && (val.contains(f.as_str()) || low.contains(f.as_str()))) {
                            hits.push(format!("ws {k}={val:?} ~ {f:?}"));
                        }
                    }
                }
            }
            _ => {
                for f in forms.iter().filter(|f| f.len() >= 4) {
                    if bytes.windows(f.len()).any(|w| w == f.as_bytes()) {
                        hits.push(format!("audio bytes ~ {f:?}"));
                    }
                }
            }
        }
    }
    hits
}

#[tokio::test]
async fn a9_1_no_byte_of_the_word_reaches_a_client_before_its_result() {
    let srv = start().await;
    let langs = active_languages();
    let per_lang = 500usize.div_ceil(langs.len());
    let mut words_checked = 0;
    for lang in &langs {
        let (mut a, mut b) = room(&srv, "h2h_fast", lang).await;
        let mut done = 0;
        while done < per_lang {
            a.tx(&ClientMsg::StartMatch {}).await;
            // Six misses (3 turns each) per match; rematch until enough words.
            for turn in 0..6 {
                let me = (turn % 2) as u8;
                let ws_active = if me == 0 { &mut a } else { &mut b };
                ws_active.until(|m| matches!(m, ServerMsg::TurnStart { seat, .. } if *seat == me)).await;
                ws_active.tx(&ClientMsg::NextWord {}).await;
                let ticket = match ws_active.until(|m| matches!(m, ServerMsg::WordStart { seat, .. } if *seat == me)).await {
                    ServerMsg::WordStart { audio_ticket, .. } => audio_ticket,
                    _ => unreachable!(),
                };
                // Both players fetch and hear the clip, as both would in the app.
                a.audio_body(&srv, &ticket).await;
                b.audio_body(&srv, &ticket).await;
                // A slow replay too: its own ticket, also fetched by both.
                let ws_active = if me == 0 { &mut a } else { &mut b };
                ws_active.tx(&ClientMsg::ReplayAudio { variant: Variant::Slow }).await;
                let slow = match ws_active.until(|m| matches!(m, ServerMsg::Audio { .. })).await {
                    ServerMsg::Audio { audio_ticket, .. } => audio_ticket,
                    _ => unreachable!(),
                };
                assert_ne!(slow, ticket);
                a.audio_body(&srv, &slow).await;
                b.audio_body(&srv, &slow).await;
                // Everything either client holds now came before the result.
                let before_a = a.got.clone();
                let before_b = b.got.clone();
                let ws_active = if me == 0 { &mut a } else { &mut b };
                ws_active.tx(&ClientMsg::Submit { typed: "zzzzqqq".into() }).await;
                let entry = match ws_active.until(|m| matches!(m, ServerMsg::WordResult { .. })).await {
                    ServerMsg::WordResult { word, .. } => word,
                    _ => unreachable!(),
                };
                let f = forms(lang, &entry);
                let mut hits = leaks(&before_a, &f);
                hits.extend(leaks(&before_b, &f));
                assert!(hits.is_empty(), "{lang} {entry:?} leaked before its result: {hits:?}");
                words_checked += 1;
                done += 1;
            }
            a.until(|m| matches!(m, ServerMsg::MatchResult { .. })).await;
            b.until(|m| matches!(m, ServerMsg::MatchResult { .. })).await;
            a.got.clear();
            b.got.clear();
        }
    }
    assert!(words_checked >= 500, "{words_checked} words");
    // The trap was real: the source served tagged clips with the word inside.
    let seen = srv.tts.seen.lock().unwrap();
    assert!(seen.iter().any(|k| k.variant == Variant::Slow));
    assert!(trapped_clip("cat").windows(3).any(|w| w == b"cat"));
}

// ---------------------------------------------------------------- A9.2

#[tokio::test]
async fn a9_2_only_room_members_get_audio_and_everyone_else_gets_404() {
    let srv = start().await;
    let (mut a, mut b) = room(&srv, "h2h", "en").await;
    let mut z = client(&srv, "198.51.100.9", ProfileKind::Adult, "Outsider", "en").await;
    z.until(|m| matches!(m, ServerMsg::Welcome { .. })).await;
    a.tx(&ClientMsg::StartMatch {}).await;
    a.until(|m| matches!(m, ServerMsg::TurnStart { .. })).await;
    a.tx(&ClientMsg::NextWord {}).await;
    let ticket = match a.until(|m| matches!(m, ServerMsg::WordStart { .. })).await {
        ServerMsg::WordStart { audio_ticket, .. } => audio_ticket,
        _ => unreachable!(),
    };
    let r = a.audio(&srv, &ticket).await;
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["content-type"], "audio/mpeg");
    assert_eq!(r.headers()["cache-control"], "no-store");
    assert!(r.headers().get("etag").is_none());
    assert!(r.headers().get("content-disposition").is_none());
    let body = r.bytes().await.unwrap();
    assert!(!body.starts_with(b"ID3"), "tags stripped");
    assert_eq!(b.audio(&srv, &ticket).await.status(), 200);
    assert_eq!(z.audio(&srv, &ticket).await.status(), 404, "a stranger's token");
    let anon = reqwest::Client::new()
        .get(format!("http://{}/v1/audio/{ticket}", srv.public))
        .header("Origin", ORIGIN)
        .send()
        .await
        .unwrap();
    assert_eq!(anon.status(), 404, "no token");
    assert_eq!(a.audio(&srv, "0123456789abcdef0123456789abcdef").await.status(), 404, "unknown ticket");
}

// ---------------------------------------------------------------- A9.3 end to end

#[tokio::test]
async fn a9_3_a_tts_outage_pauses_three_words_then_ends_the_match() {
    let srv = start().await;
    let (mut a, mut b) = room(&srv, "h2h", "en").await;
    srv.tts.down.store(true, Ordering::SeqCst);
    a.tx(&ClientMsg::StartMatch {}).await;
    a.until(|m| matches!(m, ServerMsg::TurnStart { .. })).await;
    for i in 0..3 {
        a.tx(&ClientMsg::NextWord {}).await;
        let m = b.until(|m| matches!(m, ServerMsg::WordPaused { .. } | ServerMsg::WordStart { .. })).await;
        assert!(matches!(&m, ServerMsg::WordPaused { reason } if reason == "audio_unavailable"), "pause {i}: {m:?}");
    }
    let m = b.until(|m| matches!(m, ServerMsg::MatchEnded { .. })).await;
    assert_eq!(m, ServerMsg::MatchEnded { reason: "audio_unavailable".into() });
    // 1 try + 3 retries per word.
    assert_eq!(srv.tts.calls.load(Ordering::SeqCst), 12);
}

// ---------------------------------------------------------------- A10.3 and hello checks

#[tokio::test]
async fn a10_3_a_kid_gets_a_generated_name_in_their_language() {
    let srv = start().await;
    let mut a = client(&srv, "198.51.100.20", ProfileKind::Adult, "Zqxj", "en").await;
    a.tx(&create("h2h", "es")).await;
    let code = match a.until(|m| matches!(m, ServerMsg::Joined { .. })).await {
        ServerMsg::Joined { code, .. } => code,
        _ => unreachable!(),
    };
    let mut k = client(&srv, "198.51.100.21", ProfileKind::Kid, "Typed By Kid", "es").await;
    k.tx(&ClientMsg::JoinRoom { code }).await;
    k.until(|m| matches!(m, ServerMsg::JoinPending { .. })).await;
    let (id, name) = match a.until(|m| matches!(m, ServerMsg::JoinRequest { .. })).await {
        ServerMsg::JoinRequest { request_id, name, kind } => {
            assert_eq!(kind, ProfileKind::Kid);
            (request_id, name)
        }
        _ => unreachable!(),
    };
    assert_ne!(name, "Typed By Kid", "the kid's display_name is ignored");
    let piece = ["Sombrero", "Varita", "Abeja", "Libro"];
    assert!(piece.iter().any(|p| name.starts_with(p)), "a Spanish generated name: {name}");
    a.tx(&ClientMsg::ApproveJoin { request_id: id }).await;
    let st = match a.until(|m| matches!(m, ServerMsg::RoomState(s) if s.seats.len() == 2)).await {
        ServerMsg::RoomState(s) => s,
        _ => unreachable!(),
    };
    assert!(st.kid_safe);
    assert!(st.seats.iter().any(|s| s.name == name && s.kind == ProfileKind::Kid));
}

#[tokio::test]
async fn an_unsupported_ui_language_is_refused_explicitly() {
    let srv = start().await;
    let mut c = client(&srv, "198.51.100.30", ProfileKind::Adult, "Zqxj", "xx").await;
    assert_eq!(c.rx().await.unwrap().msg, ServerMsg::error("bad_ui_lang"));
    assert_eq!(loop {
        match c.rx().await {
            Ok(_) => continue,
            Err(code) => break code,
        }
    }, Some(close::NAME));
}

// ---------------------------------------------------------------- a whole match, real timing

#[tokio::test]
async fn a_full_online_head_to_head_match_at_real_speed() {
    let srv = start().await;
    let (mut a, mut b) = room(&srv, "h2h", "fr").await;
    a.tx(&ClientMsg::StartMatch {}).await;
    for turn in 0..6 {
        let me = (turn % 2) as u8;
        let c = if me == 0 { &mut a } else { &mut b };
        c.until(|m| matches!(m, ServerMsg::TurnStart { seat, .. } if *seat == me)).await;
        c.tx(&ClientMsg::NextWord {}).await;
        c.until(|m| matches!(m, ServerMsg::WordStart { seat, .. } if *seat == me)).await;
        c.tx(&ClientMsg::Submit { typed: "zzzzqqq".into() }).await;
    }
    match a.until(|m| matches!(m, ServerMsg::MatchResult { .. })).await {
        ServerMsg::MatchResult { standings, winner_seat } => {
            assert_eq!(standings.len(), 2);
            assert!(standings.iter().all(|s| s.best == 0));
            assert_eq!(winner_seat, None, "0 vs 0 is a tie");
        }
        _ => unreachable!(),
    }
    let st = match b.until(|m| matches!(m, ServerMsg::RoomState(_))).await {
        ServerMsg::RoomState(s) => s,
        _ => unreachable!(),
    };
    assert_eq!(st.phase, "lobby", "back in the lobby, ready for a rematch");
}
