//! F1 — protocol v1: the versioned envelope, every message type, and the
//! frame decoder.
//!
//! Wire shape: `{"v":1,"t":"<type>","seq":<u64>,"cid":"<uuid>","body":{...}}`.
//! `seq` travels server→client only, `cid` client→server only. Text frames
//! only, at most [`MAX_FRAME`] bytes. Decoding is strict: unknown fields,
//! unknown types and malformed bodies are errors, never guesses.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// The newest protocol this server speaks.
pub const PROTOCOL_CURRENT: u32 = 1;
/// The envelope version (`v`). One today.
pub const ENVELOPE_V: u32 = 1;
/// The largest text frame accepted (A1.3).
pub const MAX_FRAME: usize = 4096;

/// WebSocket close codes this server uses.
pub mod close {
    pub const TOO_LARGE: u16 = 1009;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const UPDATE_REQUIRED: u16 = 4001;
    pub const NO_HELLO: u16 = 4002;
    pub const TOO_MANY_BAD: u16 = 4003;
    pub const FLOOD: u16 = 4008;
    /// Expired or unknown token: the client mints a new session (F2).
    pub const SESSION: u16 = 4009;
    pub const REVOKED: u16 = 4010;
    pub const REPLACED: u16 = 4011;
    pub const IDLE: u16 = 4012;
    pub const SHUTDOWN: u16 = 4013;
    pub const NAME: u16 = 4014;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Adult,
    Kid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resume {
    pub room: String,
    pub last_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub protocol: u32,
    pub app_build: String,
    pub platform: String,
    pub profile_kind: ProfileKind,
    pub ui_lang: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<Resume>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierMode {
    Climb,
    Fixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Normal,
    Slow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRoom {
    pub mode: String,
    pub lang: String,
    pub tier_mode: TierMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub timed: bool,
    /// O8: the optional approve-joins toggle for adult-only rooms.
    #[serde(default)]
    pub approve_joins: bool,
}

/// Every client→server message (F1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "body", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMsg {
    Hello(Hello),
    CreateRoom(CreateRoom),
    JoinRoom { code: String },
    ApproveJoin { request_id: u64 },
    DenyJoin { request_id: u64 },
    Leave {},
    StartMatch {},
    NextWord {},
    Submit { typed: String },
    ReplayAudio { variant: Variant },
    Emote { id: u8 },
}

impl ClientMsg {
    pub const TYPES: [&'static str; 11] = [
        "hello",
        "create_room",
        "join_room",
        "approve_join",
        "deny_join",
        "leave",
        "start_match",
        "next_word",
        "submit",
        "replay_audio",
        "emote",
    ];

    pub fn type_name(&self) -> &'static str {
        match self {
            ClientMsg::Hello(_) => "hello",
            ClientMsg::CreateRoom(_) => "create_room",
            ClientMsg::JoinRoom { .. } => "join_room",
            ClientMsg::ApproveJoin { .. } => "approve_join",
            ClientMsg::DenyJoin { .. } => "deny_join",
            ClientMsg::Leave {} => "leave",
            ClientMsg::StartMatch {} => "start_match",
            ClientMsg::NextWord {} => "next_word",
            ClientMsg::Submit { .. } => "submit",
            ClientMsg::ReplayAudio { .. } => "replay_audio",
            ClientMsg::Emote { .. } => "emote",
        }
    }
}

/// One decoded client frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inbound {
    pub cid: Uuid,
    pub msg: ClientMsg,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooLarge,
    BadJson,
    BadVersion,
    UnknownType,
    BadBody,
}

impl DecodeError {
    /// The `error{code}` sent back for a frame that is not fatal.
    pub fn code(&self) -> &'static str {
        match self {
            DecodeError::TooLarge => "too_large",
            DecodeError::BadJson => "bad_frame",
            DecodeError::BadVersion => "bad_version",
            DecodeError::UnknownType => "unknown_type",
            DecodeError::BadBody => "bad_body",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    v: u32,
    t: String,
    cid: Uuid,
    #[serde(default = "empty_object")]
    body: Value,
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

/// Decodes one text frame. Never panics on any input (A1.6).
pub fn decode(text: &str) -> Result<Inbound, DecodeError> {
    if text.len() > MAX_FRAME {
        return Err(DecodeError::TooLarge);
    }
    let raw: RawEnvelope = serde_json::from_str(text).map_err(|_| DecodeError::BadJson)?;
    if raw.v != ENVELOPE_V {
        return Err(DecodeError::BadVersion);
    }
    if !ClientMsg::TYPES.contains(&raw.t.as_str()) {
        return Err(DecodeError::UnknownType);
    }
    let tagged = serde_json::json!({ "t": raw.t, "body": raw.body });
    let msg: ClientMsg = serde_json::from_value(tagged).map_err(|_| DecodeError::BadBody)?;
    Ok(Inbound { cid: raw.cid, msg })
}

/// Encodes a client frame (bots and tests).
pub fn encode_client(cid: Uuid, msg: &ClientMsg) -> String {
    let mut v = serde_json::to_value(msg).expect("client msg serializes");
    let obj = v.as_object_mut().expect("adjacently tagged");
    obj.insert("v".into(), ENVELOPE_V.into());
    obj.insert("cid".into(), Value::String(cid.to_string()));
    obj.entry("body").or_insert_with(empty_object);
    v.to_string()
}

// ---------------------------------------------------------------- server → client

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatView {
    pub seat: u8,
    pub name: String,
    pub kind: ProfileKind,
    pub connected: bool,
    pub host: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomState {
    pub code: String,
    pub mode: String,
    pub lang: String,
    pub tier_mode: TierMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub timed: bool,
    pub kid_safe: bool,
    pub approve_joins: bool,
    pub max_seats: u8,
    /// "lobby" or "playing".
    pub phase: String,
    pub seats: Vec<SeatView>,
    /// The mode's own snapshot (resume after a large gap, F5). Null in the lobby.
    pub mode_state: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub seat: u8,
    pub best: u32,
}

/// Every server→client message (F1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "body", rename_all = "snake_case")]
pub enum ServerMsg {
    Welcome { player_id: String, protocol: u32 },
    UpdateRequired { min: u32, current: u32 },
    RoomState(RoomState),
    JoinRequest { request_id: u64, name: String, kind: ProfileKind },
    JoinPending { code: String, request_id: u64 },
    Joined { code: String, seat: u8 },
    Denied { reason: String },
    TurnStart { seat: u8, turn_no: u32, turns_left: u32 },
    WordStart {
        seat: u8,
        word_no: u32,
        tier: String,
        audio_ticket: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deadline_ms: Option<u64>,
    },
    WordPaused { reason: String },
    WordResult { seat: u8, correct: bool, chain: u32, best: u32, word: String },
    TurnEnd { seat: u8 },
    MatchResult {
        standings: Vec<Standing>,
        #[serde(default)]
        winner_seat: Option<u8>,
    },
    MatchEnded { reason: String },
    ServerDraining { deadline_unix_ms: u64 },
    /// An emote relayed to the room (`emote{seat,id}`); preset ids only.
    Emote { seat: u8, id: u8 },
    Error { code: String },
}

impl ServerMsg {
    pub const TYPES: [&'static str; 17] = [
        "welcome",
        "update_required",
        "room_state",
        "join_request",
        "join_pending",
        "joined",
        "denied",
        "turn_start",
        "word_start",
        "word_paused",
        "word_result",
        "turn_end",
        "match_result",
        "match_ended",
        "server_draining",
        "emote",
        "error",
    ];

    pub fn error(code: &str) -> ServerMsg {
        ServerMsg::Error { code: code.to_string() }
    }

    pub fn type_name(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.get("t").and_then(|t| t.as_str()).map(str::to_string))
            .unwrap_or_default()
    }
}

/// Encodes a server frame with its `seq` (0 for direct, non-room replies).
pub fn encode_server(seq: u64, msg: &ServerMsg) -> String {
    let mut v = serde_json::to_value(msg).expect("server msg serializes");
    let obj = v.as_object_mut().expect("adjacently tagged");
    obj.insert("v".into(), ENVELOPE_V.into());
    obj.insert("seq".into(), seq.into());
    obj.entry("body").or_insert_with(empty_object);
    v.to_string()
}

/// A decoded server frame (bots and tests).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outbound {
    pub seq: u64,
    pub msg: ServerMsg,
}

pub fn decode_server(text: &str) -> Option<Outbound> {
    let v: Value = serde_json::from_str(text).ok()?;
    let seq = v.get("seq")?.as_u64()?;
    let t = v.get("t")?.clone();
    let body = v.get("body").cloned().unwrap_or_else(empty_object);
    let msg: ServerMsg = serde_json::from_value(serde_json::json!({ "t": t, "body": body })).ok()?;
    Some(Outbound { seq, msg })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cid() -> Uuid {
        Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap()
    }

    #[test]
    fn decode_round_trips_every_client_type() {
        let samples = crate::protocol::samples::client();
        assert_eq!(samples.len(), ClientMsg::TYPES.len());
        for m in samples {
            let text = encode_client(cid(), &m);
            let back = decode(&text).expect(&text);
            assert_eq!(back.msg, m);
            assert_eq!(back.cid, cid());
        }
    }

    #[test]
    fn server_frames_round_trip_every_type() {
        let samples = crate::protocol::samples::server();
        let names: Vec<String> = samples.iter().map(|m| m.type_name()).collect();
        for t in ServerMsg::TYPES {
            assert!(names.iter().any(|n| n == t), "no sample for {t}");
        }
        for (i, m) in samples.into_iter().enumerate() {
            let text = encode_server(i as u64, &m);
            let back = decode_server(&text).expect(&text);
            assert_eq!(back.msg, m);
            assert_eq!(back.seq, i as u64);
        }
    }

    #[test]
    fn strictness() {
        let ok = r#"{"v":1,"t":"leave","cid":"00000000-0000-4000-8000-000000000001","body":{}}"#;
        assert!(decode(ok).is_ok());
        let missing_body = r#"{"v":1,"t":"leave","cid":"00000000-0000-4000-8000-000000000001"}"#;
        assert!(decode(missing_body).is_ok(), "an empty body may be omitted");
        let seq_from_client =
            r#"{"v":1,"t":"leave","seq":3,"cid":"00000000-0000-4000-8000-000000000001","body":{}}"#;
        assert_eq!(decode(seq_from_client), Err(DecodeError::BadJson));
        let no_cid = r#"{"v":1,"t":"leave","body":{}}"#;
        assert_eq!(decode(no_cid), Err(DecodeError::BadJson));
        let v2 = r#"{"v":2,"t":"leave","cid":"00000000-0000-4000-8000-000000000001","body":{}}"#;
        assert_eq!(decode(v2), Err(DecodeError::BadVersion));
        let unknown = r#"{"v":1,"t":"chat","cid":"00000000-0000-4000-8000-000000000001","body":{}}"#;
        assert_eq!(decode(unknown), Err(DecodeError::UnknownType));
        let extra = r#"{"v":1,"t":"emote","cid":"00000000-0000-4000-8000-000000000001","body":{"id":1,"x":2}}"#;
        assert_eq!(decode(extra), Err(DecodeError::BadBody));
        assert_eq!(decode(&"x".repeat(MAX_FRAME + 1)), Err(DecodeError::TooLarge));
    }

    proptest! {
        // A1.6 runs the 1M-case version in tests/frame_decoder_proptest.rs.
        #[test]
        fn decoder_never_panics(s in ".{0,600}") {
            let _ = decode(&s);
        }
    }
}

/// One sample of every message type: the A1.1 golden snapshot source.
pub mod samples {
    use super::*;

    pub fn client() -> Vec<ClientMsg> {
        vec![
            ClientMsg::Hello(Hello {
                protocol: 1,
                app_build: "300".into(),
                platform: "ios".into(),
                profile_kind: ProfileKind::Adult,
                ui_lang: "en".into(),
                display_name: Some("Mom".into()),
                token: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
                resume: Some(Resume { room: "ACDEFG".into(), last_seq: 7 }),
            }),
            ClientMsg::CreateRoom(CreateRoom {
                mode: "h2h".into(),
                lang: "en".into(),
                tier_mode: TierMode::Fixed,
                tier: Some("medium".into()),
                timed: true,
                approve_joins: false,
            }),
            ClientMsg::JoinRoom { code: "ACDEFG".into() },
            ClientMsg::ApproveJoin { request_id: 1 },
            ClientMsg::DenyJoin { request_id: 2 },
            ClientMsg::Leave {},
            ClientMsg::StartMatch {},
            ClientMsg::NextWord {},
            ClientMsg::Submit { typed: "cat".into() },
            ClientMsg::ReplayAudio { variant: Variant::Slow },
            ClientMsg::Emote { id: 3 },
        ]
    }

    pub fn server() -> Vec<ServerMsg> {
        vec![
            ServerMsg::Welcome { player_id: "00112233445566778899aabbccddeeff".into(), protocol: 1 },
            ServerMsg::UpdateRequired { min: 1, current: 1 },
            ServerMsg::RoomState(RoomState {
                code: "ACDEFG".into(),
                mode: "h2h".into(),
                lang: "en".into(),
                tier_mode: TierMode::Climb,
                tier: None,
                timed: false,
                kid_safe: true,
                approve_joins: true,
                max_seats: 2,
                phase: "lobby".into(),
                seats: vec![SeatView {
                    seat: 0,
                    name: "Mom".into(),
                    kind: ProfileKind::Adult,
                    connected: true,
                    host: true,
                }],
                mode_state: Value::Null,
            }),
            ServerMsg::JoinRequest { request_id: 1, name: "Teal Bee 07".into(), kind: ProfileKind::Kid },
            ServerMsg::JoinPending { code: "ACDEFG".into(), request_id: 1 },
            ServerMsg::Joined { code: "ACDEFG".into(), seat: 1 },
            ServerMsg::Denied { reason: "expired".into() },
            ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 },
            ServerMsg::WordStart {
                seat: 0,
                word_no: 1,
                tier: "easy".into(),
                audio_ticket: "0123456789abcdef0123456789abcdef".into(),
                deadline_ms: Some(12000),
            },
            ServerMsg::WordPaused { reason: "audio_unavailable".into() },
            ServerMsg::WordResult { seat: 0, correct: true, chain: 1, best: 1, word: "cat".into() },
            ServerMsg::TurnEnd { seat: 0 },
            ServerMsg::MatchResult {
                standings: vec![Standing { seat: 0, best: 4 }, Standing { seat: 1, best: 2 }],
                winner_seat: Some(0),
            },
            ServerMsg::MatchEnded { reason: "server_update".into() },
            ServerMsg::ServerDraining { deadline_unix_ms: 1_800_000_000_000 },
            ServerMsg::Emote { seat: 1, id: 3 },
            ServerMsg::error("unknown_type"),
        ]
    }
}
