//! CC-GAME-SERVER F12/F13 — Online Head-to-Head, the client's state.
//!
//! Pure: no DOM, no network, no clock. `online_net` feeds it server frames,
//! `online_screen` draws what it says. Everything a test needs to prove about
//! the client lives here: which panel shows for which server state (A13.1),
//! who may act, what a typed room code becomes, and the entry gate (A10.1,
//! A10.2, A13.4).
//!
//! THE RULE THIS FILE KEEPS. The client never knows the word before the server
//! sends `word_result`, and it never computes a score: every number it shows
//! came from the server (F12 invariant). There is no grader in here at all.

use serde::Deserialize;
use serde_json::{json, Value};

/// F3: room codes use this alphabet (no 0/O/1/I/L/B/8/S/5/2/Z).
pub const CODE_ALPHABET: &str = "ACDEFGHJKMNPQRTUVWXY34679";
pub const CODE_LEN: usize = 6;
/// The preset emotes (G-D4). Ids 0..8 on the wire; glyphs are display only.
// icon-exempt: emotes ARE emoji by design (G-D4's eight preset reactions); they are content a player sends, not UI icons.
pub const EMOTES: [&str; 8] = ["\u{1F44F}", "\u{1F604}", "\u{1F62E}", "\u{1F525}", "\u{1F44D}", "\u{2764}\u{FE0F}", "\u{1F389}", "\u{1F914}"];
/// The protocol this client speaks.
pub const PROTOCOL: u32 = 1;

// ---------------------------------------------------------------- the wire (server → client)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Adult,
    Kid,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SeatView {
    pub seat: u8,
    pub name: String,
    pub kind: Kind,
    pub connected: bool,
    pub host: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct RoomState {
    pub code: String,
    pub mode: String,
    pub lang: String,
    pub tier_mode: String,
    #[serde(default)]
    pub tier: Option<String>,
    pub timed: bool,
    pub kid_safe: bool,
    pub approve_joins: bool,
    pub max_seats: u8,
    pub phase: String,
    pub seats: Vec<SeatView>,
    #[serde(default)]
    pub mode_state: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Standing {
    pub seat: u8,
    pub best: u32,
}

/// Every server message (mirrors server/spell-server/src/protocol.rs; the
/// `parses_the_servers_golden_protocol` test pins the two together).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "t", content = "body", rename_all = "snake_case")]
pub enum ServerMsg {
    Welcome { player_id: String, protocol: u32 },
    UpdateRequired { min: u32, current: u32 },
    RoomState(RoomState),
    JoinRequest { request_id: u64, name: String, kind: Kind },
    JoinPending { code: String, request_id: u64 },
    Joined { code: String, seat: u8 },
    Denied { reason: String },
    TurnStart { seat: u8, turn_no: u32, turns_left: u32 },
    WordStart {
        seat: u8,
        word_no: u32,
        tier: String,
        audio_ticket: String,
        #[serde(default)]
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
    Emote { seat: u8, id: u8 },
    Audio { variant: String, audio_ticket: String },
    Error { code: String },
}

/// One decoded frame: its per-member sequence (0 = a direct reply) and message.
pub fn decode(text: &str) -> Option<(u64, ServerMsg)> {
    let v: Value = serde_json::from_str(text).ok()?;
    let seq = v.get("seq")?.as_u64()?;
    let t = v.get("t")?.clone();
    let body = v.get("body").cloned().unwrap_or_else(|| json!({}));
    let msg = serde_json::from_value(json!({ "t": t, "body": body })).ok()?;
    Some((seq, msg))
}

/// A client frame: `{"v":1,"t":..,"cid":..,"body":..}`.
pub fn frame(t: &str, cid: &str, body: Value) -> String {
    json!({ "v": 1, "t": t, "cid": cid, "body": body }).to_string()
}

// ---------------------------------------------------------------- inputs

/// F12: a typed code, uppercased; `Err(bad)` names the first character
/// outside the alphabet for the inline hint.
pub fn clean_code(raw: &str) -> Result<String, char> {
    let mut out = String::new();
    for c in raw.chars().filter(|c| !c.is_whitespace()) {
        let u = c.to_ascii_uppercase();
        if !CODE_ALPHABET.contains(u) {
            return Err(c);
        }
        if out.chars().count() < CODE_LEN {
            out.push(u);
        }
    }
    Ok(out)
}

/// The share sheet's text: plain, with the code (F12).
pub fn share_text(code: &str) -> String {
    crate::i18n::tp("ol.shareText", &[("code", code)])
}

/// F10 + F13: does the "Play online" entry exist at all? With the build flag
/// off it does not, and nothing online is ever requested (A13.4). A kid also
/// needs the kids flag and a parent's "Play online with family" (A10.1, A10.2).
pub fn entry_visible(enabled: bool, kids_flag: bool, kid: bool, family_ok: bool) -> bool {
    enabled && (!kid || (kids_flag && family_ok))
}

// ---------------------------------------------------------------- the state

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Panel {
    /// Create or join.
    Entry,
    /// Minting a session / opening the socket.
    Connecting,
    /// Knocked, waiting for the host.
    Pending,
    Lobby,
    Match,
    Result,
    /// F13: killed remotely or unreachable. Local Head-to-Head still works.
    Resting,
    /// F13: `update_required`.
    Update,
    /// `match_ended{reason}` outside a lobby (the room is gone).
    Ended,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    pub seat: u8,
    pub word_no: u32,
    pub tier: String,
    pub ticket: String,
    pub deadline_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flash {
    pub seat: u8,
    pub correct: bool,
    /// The bank entry (`pinyin|hanzi` for Mandarin), shown only after it resolves.
    pub word: String,
}

/// What a frame asks the screen to do besides redraw.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Play this ticket's clip (both players hear the word, as on one phone).
    Play(String),
    /// Clear the typed answer.
    ClearAnswer,
    Haptic(bool),
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub panel_override: Option<Panel>,
    pub connecting: bool,
    pub player_id: Option<String>,
    pub room: Option<RoomState>,
    pub my_seat: Option<u8>,
    pub pending_code: Option<String>,
    pub requests: Vec<(u64, String, Kind)>,
    pub turn: Option<(u8, u32)>,
    pub word: Option<Live>,
    pub slow_ticket: Option<String>,
    pub flash: Option<Flash>,
    pub paused: bool,
    /// Per seat: (current chain, best), exactly as the server last said.
    pub score: Vec<(u8, u32, u32)>,
    pub result: Option<(Vec<Standing>, Option<u8>)>,
    pub in_match: bool,
    pub draining_until: Option<u64>,
    pub ended: Option<String>,
    pub entry_error: Option<String>,
    pub last_seq: u64,
    pub emote: Option<(u8, u8)>,
    pub typed: String,
    /// The last error the server sent (shown inline where it applies).
    pub last_error: Option<String>,
}

impl Model {
    /// The panel the screen shows, decided only from what the server said.
    pub fn panel(&self) -> Panel {
        if let Some(p) = &self.panel_override {
            return p.clone();
        }
        if self.ended.is_some() && self.room.is_none() {
            return Panel::Ended;
        }
        if self.result.is_some() && !self.in_match {
            return Panel::Result;
        }
        if self.in_match {
            return Panel::Match;
        }
        if self.room.is_some() {
            return Panel::Lobby;
        }
        if self.pending_code.is_some() {
            return Panel::Pending;
        }
        if self.connecting {
            return Panel::Connecting;
        }
        Panel::Entry
    }

    pub fn am_host(&self) -> bool {
        let (Some(r), Some(me)) = (&self.room, self.my_seat) else { return false };
        r.seats.iter().any(|s| s.seat == me && s.host)
    }

    pub fn my_turn(&self) -> bool {
        matches!((self.turn, self.my_seat), (Some((s, _)), Some(me)) if s == me)
    }

    /// May I tap the orb for a word? My turn, no live word, not paused.
    pub fn can_tap(&self) -> bool {
        self.in_match && self.my_turn() && self.word.is_none()
    }

    /// May I type and submit? A live word that is mine.
    pub fn can_answer(&self) -> bool {
        self.in_match && self.word.as_ref().is_some_and(|w| Some(w.seat) == self.my_seat)
    }

    pub fn name_of(&self, seat: u8) -> String {
        self.room
            .as_ref()
            .and_then(|r| r.seats.iter().find(|s| s.seat == seat))
            .map(|s| s.name.clone())
            .unwrap_or_default()
    }

    pub fn score_of(&self, seat: u8) -> (u32, u32) {
        self.score.iter().find(|x| x.0 == seat).map(|x| (x.1, x.2)).unwrap_or((0, 0))
    }

    fn set_score(&mut self, seat: u8, current: u32, best: u32) {
        match self.score.iter_mut().find(|x| x.0 == seat) {
            Some(x) => *x = (seat, current, best),
            None => self.score.push((seat, current, best)),
        }
    }

    /// The resume point for `hello{resume}`.
    pub fn resume(&self) -> Option<(String, u64)> {
        self.room.as_ref().map(|r| (r.code.clone(), self.last_seq))
    }

    /// Applies one server frame.
    pub fn apply(&mut self, seq: u64, msg: ServerMsg) -> Vec<Effect> {
        if seq > 0 {
            self.last_seq = self.last_seq.max(seq);
        }
        let mut fx = Vec::new();
        match msg {
            ServerMsg::Welcome { player_id, .. } => {
                self.player_id = Some(player_id);
                self.connecting = false;
            }
            ServerMsg::UpdateRequired { .. } => self.panel_override = Some(Panel::Update),
            ServerMsg::RoomState(r) => {
                // A large-gap resume arrives as a snapshot: take the scores
                // and whose turn it is from the mode's own part.
                if let Some(players) = r.mode_state.get("players").and_then(|p| p.as_array()) {
                    for p in players {
                        let g = |k: &str| p.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
                        self.set_score(g("seat") as u8, g("current") as u32, g("best") as u32);
                    }
                    if let Some(a) = r.mode_state.get("active_seat").and_then(|x| x.as_u64()) {
                        let left = players
                            .iter()
                            .find(|p| p.get("seat").and_then(|x| x.as_u64()) == Some(a))
                            .and_then(|p| p.get("turns_left").and_then(|x| x.as_u64()))
                            .unwrap_or(0);
                        self.turn = Some((a as u8, left as u32));
                    }
                }
                self.in_match = r.phase == "playing";
                if r.phase == "playing" && r.mode_state.get("live").is_some_and(|l| l.is_null()) {
                    self.word = None;
                }
                self.pending_code = None;
                self.ended = None;
                self.room = Some(r);
            }
            ServerMsg::JoinRequest { request_id, name, kind } => {
                self.requests.retain(|x| x.0 != request_id);
                self.requests.push((request_id, name, kind));
            }
            ServerMsg::JoinPending { code, .. } => {
                self.pending_code = Some(code);
                self.connecting = false;
            }
            ServerMsg::Joined { seat, .. } => {
                self.my_seat = Some(seat);
                self.pending_code = None;
                self.entry_error = None;
                self.ended = None;
            }
            ServerMsg::Denied { reason } => {
                self.pending_code = None;
                self.entry_error = Some(if reason == "expired" { "denied_expired".into() } else { "denied".into() });
            }
            ServerMsg::TurnStart { seat, turns_left, .. } => {
                if !self.in_match {
                    // A new match: drop the last one's numbers.
                    self.score.clear();
                    self.result = None;
                }
                self.in_match = true;
                self.turn = Some((seat, turns_left));
                self.word = None;
                self.flash = None;
                self.paused = false;
                fx.push(Effect::ClearAnswer);
            }
            ServerMsg::WordStart { seat, word_no, tier, audio_ticket, deadline_ms } => {
                self.word = Some(Live { seat, word_no, tier, ticket: audio_ticket.clone(), deadline_ms });
                self.slow_ticket = None;
                self.flash = None;
                self.paused = false;
                fx.push(Effect::ClearAnswer);
                fx.push(Effect::Play(audio_ticket));
            }
            ServerMsg::WordPaused { .. } => {
                self.word = None;
                self.paused = true;
            }
            ServerMsg::WordResult { seat, correct, chain, best, word } => {
                self.set_score(seat, chain, best);
                self.word = None;
                self.flash = Some(Flash { seat, correct, word });
                if Some(seat) == self.my_seat {
                    fx.push(Effect::Haptic(correct));
                }
            }
            ServerMsg::TurnEnd { seat } => {
                let (_, best) = self.score_of(seat);
                self.set_score(seat, 0, best);
                self.word = None;
            }
            ServerMsg::MatchResult { standings, winner_seat } => {
                for s in &standings {
                    let (cur, _) = self.score_of(s.seat);
                    self.set_score(s.seat, cur, s.best);
                }
                self.result = Some((standings, winner_seat));
                self.in_match = false;
                self.turn = None;
                self.word = None;
            }
            ServerMsg::MatchEnded { reason } => {
                self.in_match = false;
                self.turn = None;
                self.word = None;
                self.result = None;
                // A closed room (or our own leave) takes us out of it.
                let closes = matches!(
                    reason.as_str(),
                    "host_left" | "room_expired" | "room_empty" | "room_closed" | "left" | "server_update"
                );
                if closes {
                    self.room = None;
                    self.my_seat = None;
                    self.requests.clear();
                }
                self.ended = Some(reason);
            }
            ServerMsg::ServerDraining { deadline_unix_ms } => self.draining_until = Some(deadline_unix_ms),
            ServerMsg::Emote { seat, id } => self.emote = Some((seat, id)),
            ServerMsg::Audio { variant, audio_ticket } => {
                if variant == "slow" {
                    self.slow_ticket = Some(audio_ticket.clone());
                }
                fx.push(Effect::Play(audio_ticket));
            }
            ServerMsg::Error { code } => {
                if matches!(
                    code.as_str(),
                    "room_not_found" | "bad_code" | "rate_limited" | "room_full" | "match_in_progress" | "draining"
                        | "already_in_room" | "lang_unavailable" | "not_allowed"
                ) {
                    self.entry_error = Some(code.clone());
                }
                self.last_error = Some(code);
            }
        }
        fx
    }

    /// Back to a clean entry (Leave, or a fresh open).
    pub fn reset(&mut self) {
        let player_id = self.player_id.take();
        *self = Model { player_id, ..Model::default() };
    }
}

// ---------------------------------------------------------------- what the panels say

/// The inline message for an entry error code.
pub fn entry_error_key(code: &str) -> &'static str {
    match code {
        "room_not_found" | "bad_code" => "ol.err.code",
        "rate_limited" => "ol.err.rateLimited",
        "room_full" => "ol.err.full",
        "match_in_progress" => "ol.err.playing",
        "draining" => "ol.err.draining",
        "denied" => "ol.err.denied",
        "denied_expired" => "ol.err.expired",
        "lang_unavailable" => "ol.err.lang",
        "not_allowed" => "ol.err.notAllowed",
        _ => "ol.err.generic",
    }
}

/// The message for `match_ended{reason}`: specific, with no penalty language (F13).
pub fn ended_key(reason: &str) -> &'static str {
    match reason {
        "server_update" => "ol.end.serverUpdate",
        "player_left" => "ol.end.playerLeft",
        "audio_unavailable" => "ol.end.audio",
        "bank_exhausted" => "ol.end.bank",
        "host_left" => "ol.end.hostLeft",
        "room_expired" => "ol.end.expired",
        "room_empty" | "room_closed" => "ol.end.closed",
        "left" => "ol.end.left",
        _ => "ol.end.generic",
    }
}

/// The DOM id of each panel (one visible at a time).
pub fn panel_id(p: &Panel) -> &'static str {
    match p {
        Panel::Entry => "olEntry",
        Panel::Connecting => "olConnecting",
        Panel::Pending => "olPending",
        Panel::Lobby => "olLobby",
        Panel::Match => "olMatch",
        Panel::Result => "olResult",
        Panel::Resting => "olResting",
        Panel::Update => "olUpdate",
        Panel::Ended => "olEnded",
    }
}

pub const PANELS: [Panel; 9] = [
    Panel::Entry,
    Panel::Connecting,
    Panel::Pending,
    Panel::Lobby,
    Panel::Match,
    Panel::Result,
    Panel::Resting,
    Panel::Update,
    Panel::Ended,
];

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The lobby's player list.
pub fn players_html(m: &Model) -> String {
    let Some(r) = &m.room else { return String::new() };
    r.seats
        .iter()
        .map(|s| {
            let mut tags = String::new();
            if s.host {
                tags.push_str(&format!("<span class=\"ol-tag\">{}</span>", esc(&crate::i18n::t("ol.host"))));
            }
            if Some(s.seat) == m.my_seat {
                tags.push_str(&format!("<span class=\"ol-tag\">{}</span>", esc(&crate::i18n::t("ol.you"))));
            }
            if !s.connected {
                tags.push_str(&format!("<span class=\"ol-tag off\">{}</span>", esc(&crate::i18n::t("ol.away"))));
            }
            format!("<li class=\"ol-player\" data-seat=\"{}\"><b>{}</b>{tags}</li>", s.seat, esc(&s.name))
        })
        .collect()
}

/// The host's knock list: each request with Let in / Not now.
pub fn requests_html(m: &Model) -> String {
    if !m.am_host() {
        return String::new();
    }
    m.requests
        .iter()
        .map(|(id, name, kind)| {
            let kid = if *kind == Kind::Kid { format!(" <span class=\"ol-tag\">{}</span>", esc(&crate::i18n::t("ol.kid"))) } else { String::new() };
            format!(
                "<li class=\"ol-knock\"><span>{}</span>{kid}<button class=\"btn btn-check\" data-approve=\"{id}\">{}</button><button class=\"btn\" data-deny=\"{id}\">{}</button></li>",
                esc(&crate::i18n::tp("ol.knock", &[("name", name)])),
                esc(&crate::i18n::t("ol.letIn")),
                esc(&crate::i18n::t("ol.notNow")),
            )
        })
        .collect()
}

/// The line under the orb: whose turn, and what to do.
pub fn status_text(m: &Model) -> String {
    if m.paused {
        return crate::i18n::t("ol.paused");
    }
    if let Some(f) = &m.flash {
        let key = if f.correct { "ol.right" } else { "ol.wrong" };
        return crate::i18n::tp(key, &[("name", &m.name_of(f.seat)), ("word", &display_word(&f.word))]);
    }
    let Some((seat, _)) = m.turn else { return String::new() };
    let name = m.name_of(seat);
    match (&m.word, m.my_turn()) {
        (None, true) => crate::i18n::t("ol.yourTurn"),
        (None, false) => crate::i18n::tp("ol.theirTurn", &[("name", &name)]),
        (Some(_), true) => crate::i18n::t("ol.spellIt"),
        (Some(_), false) => crate::i18n::tp("ol.spelling", &[("name", &name)]),
    }
}

/// A resolved bank entry as the player reads it: Mandarin shows its hanzi
/// and pinyin; every other language its word.
pub fn display_word(entry: &str) -> String {
    match entry.split_once('|') {
        Some((pinyin, hanzi)) => format!("{hanzi} ({pinyin})"),
        None => entry.to_string(),
    }
}

/// The result panel: who won, and both best chains, from `match_result` only.
pub fn result_html(m: &Model) -> (String, String) {
    let Some((standings, winner)) = &m.result else { return (String::new(), String::new()) };
    let title = match winner {
        Some(w) if Some(*w) == m.my_seat => crate::i18n::t("ol.youWin"),
        Some(w) => crate::i18n::tp("ol.wins", &[("name", &m.name_of(*w))]),
        None => crate::i18n::t("ol.tie"),
    };
    let rows: String = standings
        .iter()
        .map(|s| {
            format!(
                "<li><b>{}</b> {}</li>",
                esc(&m.name_of(s.seat)),
                esc(&crate::i18n::tp("ol.bestChain", &[("n", &s.best.to_string())]))
            )
        })
        .collect();
    (title, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(phase: &str, _me_host: bool, kid: bool) -> ServerMsg {
        ServerMsg::RoomState(RoomState {
            code: "ACDEFG".into(),
            mode: "h2h".into(),
            lang: "en".into(),
            tier_mode: "climb".into(),
            tier: None,
            timed: false,
            kid_safe: kid,
            approve_joins: kid,
            max_seats: 2,
            phase: phase.into(),
            seats: vec![
                // The creator sits in seat 0 and is the host.
                SeatView { seat: 0, name: "Mom".into(), kind: Kind::Adult, connected: true, host: true },
                SeatView { seat: 1, name: "Teal Bee 07".into(), kind: if kid { Kind::Kid } else { Kind::Adult }, connected: true, host: false },
            ],
            mode_state: Value::Null,
        })
    }

    fn lobby(me: u8) -> Model {
        let mut m = Model::default();
        m.apply(0, ServerMsg::Welcome { player_id: "p".into(), protocol: 1 });
        m.apply(1, ServerMsg::Joined { code: "ACDEFG".into(), seat: me });
        m.apply(2, room("lobby", me == 0, false));
        m
    }

    #[test]
    fn parses_the_servers_golden_protocol() {
        // The server's A1.1 golden: every message type's exact wire form. If
        // the two sides drift, this fails on the app side too.
        let golden = include_str!("../server/spell-server/tests/golden/protocol.txt");
        let server_part = golden.split("# server -> client\n").nth(1).expect("server section");
        let mut n = 0;
        for line in server_part.lines().filter(|l| !l.trim().is_empty()) {
            assert!(decode(line).is_some(), "the client cannot read: {line}");
            n += 1;
        }
        assert_eq!(n, 18, "one golden line per server message type");
    }

    #[test]
    fn client_frames_match_the_servers_golden_shape() {
        let golden = include_str!("../server/spell-server/tests/golden/protocol.txt");
        let ours = frame("join_room", "00000000-0000-4000-8000-000000000001", json!({ "code": "ACDEFG" }));
        let theirs = golden.lines().find(|l| l.contains("\"t\":\"join_room\"")).unwrap();
        let a: Value = serde_json::from_str(&ours).unwrap();
        let b: Value = serde_json::from_str(theirs).unwrap();
        assert_eq!(a, b);
    }

    // ---- A13.1: every server state fixture lands on the right panel

    #[test]
    fn a13_1_every_state_renders_the_right_panel() {
        let mut m = Model::default();
        assert_eq!(m.panel(), Panel::Entry);
        m.connecting = true;
        assert_eq!(m.panel(), Panel::Connecting);

        let mut m = Model::default();
        m.apply(0, ServerMsg::JoinPending { code: "ACDEFG".into(), request_id: 3 });
        assert_eq!(m.panel(), Panel::Pending);
        m.apply(0, ServerMsg::Denied { reason: "expired".into() });
        assert_eq!(m.panel(), Panel::Entry);
        assert_eq!(entry_error_key(m.entry_error.as_deref().unwrap()), "ol.err.expired");

        let mut m = lobby(0);
        assert_eq!(m.panel(), Panel::Lobby);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        assert_eq!(m.panel(), Panel::Match);
        m.apply(4, ServerMsg::MatchResult { standings: vec![Standing { seat: 0, best: 2 }, Standing { seat: 1, best: 1 }], winner_seat: Some(0) });
        assert_eq!(m.panel(), Panel::Result);
        assert_eq!(result_html(&m).0, crate::i18n::t("ol.youWin"));

        let mut m = lobby(1);
        m.apply(3, ServerMsg::MatchEnded { reason: "host_left".into() });
        assert_eq!(m.panel(), Panel::Ended);
        assert_eq!(ended_key(m.ended.as_deref().unwrap()), "ol.end.hostLeft");

        let mut m = lobby(0);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        m.apply(4, ServerMsg::MatchEnded { reason: "player_left".into() });
        assert_eq!(m.panel(), Panel::Lobby, "the room survives a player leaving");
        assert_eq!(m.ended.as_deref(), Some("player_left"));

        let mut m = Model::default();
        m.apply(0, ServerMsg::UpdateRequired { min: 2, current: 2 });
        assert_eq!(m.panel(), Panel::Update);

        let mut m = lobby(0);
        m.panel_override = Some(Panel::Resting);
        assert_eq!(m.panel(), Panel::Resting);
    }

    #[test]
    fn every_match_ended_reason_has_its_own_calm_message() {
        for r in ["server_update", "player_left", "audio_unavailable", "bank_exhausted", "host_left", "room_expired", "left"] {
            let k = ended_key(r);
            assert_ne!(k, "ol.end.generic", "{r}");
            let text = crate::i18n::t(k).to_lowercase();
            let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
            for bad in ["penalty", "forfeit", "forfeited", "lose", "lost", "loss"] {
                assert!(!words.contains(&bad), "{r}: {text}");
            }
        }
    }

    #[test]
    fn a13_draining_shows_a_banner_without_leaving_the_match() {
        let mut m = lobby(0);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        m.apply(0, ServerMsg::ServerDraining { deadline_unix_ms: 99 });
        assert_eq!(m.panel(), Panel::Match);
        assert_eq!(m.draining_until, Some(99));
    }

    // ---- the turn, from the server's words only

    #[test]
    fn whose_turn_and_what_i_may_do() {
        let mut m = lobby(1);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        assert!(!m.can_tap() && !m.can_answer());
        assert_eq!(status_text(&m), crate::i18n::tp("ol.theirTurn", &[("name", "Mom")]));
        let fx = m.apply(4, ServerMsg::WordStart { seat: 0, word_no: 1, tier: "easy".into(), audio_ticket: "t1".into(), deadline_ms: None });
        assert!(fx.contains(&Effect::Play("t1".into())), "the watcher hears it too");
        assert!(!m.can_answer());
        m.apply(5, ServerMsg::WordResult { seat: 0, correct: false, chain: 0, best: 0, word: "cat".into() });
        m.apply(6, ServerMsg::TurnEnd { seat: 0 });
        m.apply(7, ServerMsg::TurnStart { seat: 1, turn_no: 2, turns_left: 3 });
        assert!(m.can_tap());
        assert_eq!(status_text(&m), crate::i18n::t("ol.yourTurn"));
        m.apply(8, ServerMsg::WordStart { seat: 1, word_no: 2, tier: "easy".into(), audio_ticket: "t2".into(), deadline_ms: Some(12000) });
        assert!(m.can_answer() && !m.can_tap());
        assert_eq!(m.last_seq, 8);
    }

    #[test]
    fn the_client_never_knows_the_word_before_its_result() {
        let mut m = lobby(0);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        m.apply(4, ServerMsg::WordStart { seat: 0, word_no: 1, tier: "easy".into(), audio_ticket: "t".into(), deadline_ms: None });
        let dump = format!("{m:?}");
        assert!(!dump.contains("cat"));
        m.apply(5, ServerMsg::WordResult { seat: 0, correct: true, chain: 1, best: 1, word: "cat".into() });
        assert_eq!(m.flash.as_ref().unwrap().word, "cat");
    }

    #[test]
    fn scores_are_only_what_the_server_sent() {
        let mut m = lobby(0);
        m.apply(3, ServerMsg::TurnStart { seat: 0, turn_no: 1, turns_left: 3 });
        m.apply(4, ServerMsg::WordResult { seat: 0, correct: true, chain: 1, best: 1, word: "a".into() });
        m.apply(5, ServerMsg::WordResult { seat: 0, correct: true, chain: 2, best: 2, word: "b".into() });
        assert_eq!(m.score_of(0), (2, 2));
        m.apply(6, ServerMsg::WordResult { seat: 0, correct: false, chain: 0, best: 2, word: "c".into() });
        m.apply(7, ServerMsg::TurnEnd { seat: 0 });
        assert_eq!(m.score_of(0), (0, 2));
    }

    #[test]
    fn a_large_gap_snapshot_restores_scores_and_turn() {
        let mut m = lobby(1);
        let mut r = match room("playing", false, false) {
            ServerMsg::RoomState(r) => r,
            _ => unreachable!(),
        };
        r.mode_state = json!({
            "phase": "await_next", "active_seat": 1, "turn_no": 3, "word_no": 4,
            "players": [ { "seat": 0, "current": 0, "best": 3, "turns_left": 2 }, { "seat": 1, "current": 0, "best": 1, "turns_left": 2 } ],
            "live": null
        });
        m.apply(40, ServerMsg::RoomState(r));
        assert_eq!(m.panel(), Panel::Match);
        assert_eq!(m.score_of(0), (0, 3));
        assert_eq!(m.turn, Some((1, 2)));
        assert!(m.can_tap());
    }

    #[test]
    fn the_host_sees_knocks_and_nobody_else_does() {
        let mut host = lobby(0);
        host.apply(3, ServerMsg::JoinRequest { request_id: 9, name: "Teal Bee 07".into(), kind: Kind::Kid });
        let h = requests_html(&host);
        assert!(h.contains("data-approve=\"9\"") && h.contains("data-deny=\"9\""));
        let mut guest = lobby(1);
        guest.apply(3, ServerMsg::JoinRequest { request_id: 9, name: "x".into(), kind: Kind::Kid });
        assert!(requests_html(&guest).is_empty());
    }

    #[test]
    fn names_are_escaped() {
        let mut m = lobby(0);
        if let Some(r) = m.room.as_mut() {
            r.seats[1].name = "<img src=x>".into();
        }
        assert!(!players_html(&m).contains("<img"));
    }

    // ---- inputs and gates

    #[test]
    fn codes_uppercase_and_reject_outside_the_alphabet() {
        assert_eq!(clean_code("acd efg"), Ok("ACDEFG".into()));
        assert_eq!(clean_code("acdefgh"), Ok("ACDEFG".into()), "capped at six");
        assert_eq!(clean_code("ac0"), Err('0'));
        assert_eq!(clean_code("acb"), Err('b'));
        for bad in "0O1IL B8S52Z".chars().filter(|c| !c.is_whitespace()) {
            assert!(clean_code(&bad.to_string()).is_err(), "{bad}");
        }
    }

    #[test]
    fn a10_a13_4_the_entry_gate() {
        // Build flag off: no entry for anyone, so nothing is ever requested.
        assert!(!entry_visible(false, true, false, true));
        assert!(!entry_visible(false, true, true, true));
        // Adults need only the build flag.
        assert!(entry_visible(true, false, false, false));
        // A10.1: a kid without the parent's toggle.
        assert!(!entry_visible(true, true, true, false));
        // A10.2: toggle on but online.kids off.
        assert!(!entry_visible(true, false, true, true));
        assert!(entry_visible(true, true, true, true));
    }

    #[test]
    fn mandarin_results_read_as_hanzi_and_pinyin() {
        assert_eq!(display_word("ni3 hao3|你好"), "你好 (ni3 hao3)");
        assert_eq!(display_word("cat"), "cat");
    }
}
