//! F3 + F5 — the room layer: live connections, rooms, codes, knock-to-join,
//! host migration, seat holds, per-member event streams and resume.
//!
//! One mutex guards all of it. Every operation is short and synchronous;
//! frames leave through per-connection unbounded channels, so nothing here
//! ever awaits while holding the lock. Time is `tokio::time::Instant`, so the
//! lifetime tests run under `tokio::time::pause` (A3.2).
//!
//! The room layer contains no rule of any game (F7 invariant): it hands
//! `next_word`, `submit`, `replay_audio`, timers and disconnects to the
//! room's `ModeRules` and delivers what comes back.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use uuid::Uuid;

use crate::limits::Limits;
use crate::mode::{self, Audience, Ctx, Event, MakeError, ModeRules};
use crate::protocol::{
    encode_server, ClientMsg, CreateRoom, Inbound, ProfileKind, RoomState, SeatView, ServerMsg, TierMode,
};

/// Room-code alphabet (F3): no 0/O/1/I/L/B/8/S/5/2/Z.
pub const CODE_ALPHABET: &[u8; 25] = b"ACDEFGHJKMNPQRTUVWXY34679";
pub const CODE_LEN: usize = 6;
pub const RING: usize = 512;
pub const PENDING_TTL: Duration = Duration::from_secs(120);
pub const SEAT_HOLD: Duration = Duration::from_secs(60);
pub const EMPTY_TTL: Duration = Duration::from_secs(600);
pub const ROOM_LIFETIME: Duration = Duration::from_secs(3 * 3600);
pub const EMOTES: u8 = 8;

/// What a connection task receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Frame(String),
    Close(u16, &'static str),
}

/// A live connection's identity, set by `hello`.
pub struct Session {
    pub player: String,
    pub conn_id: u64,
    pub kind: ProfileKind,
    pub name: String,
    pub ip: IpAddr,
}

struct Live {
    conn_id: u64,
    tx: UnboundedSender<Out>,
    kind: ProfileKind,
    name: String,
}

struct Outbox {
    next_seq: u64,
    ring: VecDeque<(u64, String)>,
}

impl Outbox {
    fn new() -> Outbox {
        Outbox { next_seq: 1, ring: VecDeque::with_capacity(RING) }
    }
    fn push(&mut self, msg: &ServerMsg) -> String {
        let seq = self.next_seq;
        self.next_seq += 1;
        let frame = encode_server(seq, msg);
        if self.ring.len() == RING {
            self.ring.pop_front();
        }
        self.ring.push_back((seq, frame.clone()));
        frame
    }
}

struct Member {
    player: String,
    name: String,
    kind: ProfileKind,
    /// Join order, for host migration (earliest-joined adult).
    order: u64,
    connected: bool,
    dropped_at: Option<Instant>,
    outbox: Outbox,
}

struct Pending {
    id: u64,
    player: String,
    name: String,
    kind: ProfileKind,
    since: Instant,
}

struct Room {
    code: String,
    req: CreateRoom,
    mode: Box<dyn ModeRules>,
    host: String,
    seats: BTreeMap<u8, Member>,
    pending: Vec<Pending>,
    kid_safe: bool,
    created: Instant,
    empty_since: Option<Instant>,
    /// `submit` cids already graded in this match (F5 idempotence).
    cids: HashSet<Uuid>,
}

impl Room {
    fn seat_of(&self, player: &str) -> Option<u8> {
        self.seats.iter().find(|(_, m)| m.player == player).map(|(s, _)| *s)
    }
    fn occupied(&self) -> Vec<u8> {
        self.seats.keys().copied().collect()
    }
    fn needs_approval(&self, joiner: ProfileKind) -> bool {
        // G-D13 / A3.4: a kid anywhere (already seated, or knocking) forces it.
        self.kid_safe || joiner == ProfileKind::Kid || self.req.approve_joins
    }
    fn state(&self) -> RoomState {
        RoomState {
            code: self.code.clone(),
            mode: self.req.mode.clone(),
            lang: self.req.lang.clone(),
            tier_mode: self.req.tier_mode,
            tier: self.req.tier.clone(),
            timed: self.req.timed,
            kid_safe: self.kid_safe,
            approve_joins: self.needs_approval(ProfileKind::Adult),
            max_seats: self.mode.max_seats(),
            phase: if self.mode.running() { "playing" } else { "lobby" }.into(),
            seats: self
                .seats
                .iter()
                .map(|(s, m)| SeatView {
                    seat: *s,
                    name: m.name.clone(),
                    kind: m.kind,
                    connected: m.connected,
                    host: m.player == self.host,
                })
                .collect(),
            mode_state: if self.mode.running() { self.mode.snapshot() } else { Value::Null },
        }
    }
}

#[derive(Default)]
struct State {
    live: HashMap<String, Live>,
    rooms: HashMap<String, Room>,
    room_of: HashMap<String, String>,
    pending_of: HashMap<String, (String, u64)>,
    limits: Limits,
    draining: Option<u64>,
    next_id: u64,
    order: u64,
}

/// Counters for `/admin/stats`.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct Stats {
    pub connections: usize,
    pub rooms: usize,
    pub matches: usize,
    pub players_seated: usize,
}

pub struct Hub {
    state: Mutex<State>,
}

impl Default for Hub {
    fn default() -> Self {
        Hub { state: Mutex::new(State::default()) }
    }
}

/// A fresh room code: uniform over the alphabet (rejection sampling).
pub fn new_code() -> String {
    let mut out = String::with_capacity(CODE_LEN);
    while out.len() < CODE_LEN {
        for b in crate::store::random_bytes::<16>() {
            if b < 250 && out.len() < CODE_LEN {
                out.push(CODE_ALPHABET[(b % 25) as usize] as char);
            }
        }
    }
    out
}

/// Normalizes a typed code: uppercase; `None` if it cannot be a code.
pub fn normalize_code(raw: &str) -> Option<String> {
    let up = raw.trim().to_ascii_uppercase();
    (up.len() == CODE_LEN && up.bytes().all(|b| CODE_ALPHABET.contains(&b))).then_some(up)
}

impl State {
    fn direct(&self, player: &str, msg: &ServerMsg) {
        if let Some(l) = self.live.get(player) {
            let _ = l.tx.send(Out::Frame(encode_server(0, msg)));
        }
    }

    fn close_conn(&self, player: &str, code: u16, why: &'static str) {
        if let Some(l) = self.live.get(player) {
            let _ = l.tx.send(Out::Close(code, why));
        }
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Queues `msg` on one member's stream (and sends it if connected).
    fn to_member(live: &HashMap<String, Live>, m: &mut Member, msg: &ServerMsg) {
        let frame = m.outbox.push(msg);
        if m.connected {
            if let Some(l) = live.get(&m.player) {
                let _ = l.tx.send(Out::Frame(frame));
            }
        }
    }

    fn broadcast(&mut self, code: &str, msg: &ServerMsg) {
        let State { live, rooms, .. } = self;
        if let Some(room) = rooms.get_mut(code) {
            for m in room.seats.values_mut() {
                Self::to_member(live, m, msg);
            }
        }
    }

    fn send_room_state(&mut self, code: &str) {
        if let Some(room) = self.rooms.get(code) {
            let msg = ServerMsg::RoomState(room.state());
            self.broadcast(code, &msg);
        }
    }

    /// Delivers mode events; `Over` returns the room to its lobby.
    fn deliver(&mut self, code: &str, events: Vec<Event>) {
        let mut over = false;
        let State { live, rooms, .. } = self;
        let Some(room) = rooms.get_mut(code) else { return };
        for ev in events {
            match ev {
                Event::Send { to: Audience::All, msg } => {
                    for m in room.seats.values_mut() {
                        Self::to_member(live, m, &msg);
                    }
                }
                Event::Send { to: Audience::Seat(s), msg } => {
                    if let Some(m) = room.seats.get_mut(&s) {
                        Self::to_member(live, m, &msg);
                    }
                }
                Event::Over => over = true,
            }
        }
        if over {
            room.cids.clear();
            self.send_room_state(code);
        }
    }

    fn ctx_parts(room: &Room) -> (Vec<u8>, bool) {
        (room.occupied(), room.kid_safe)
    }

    /// Ends a running match with a reason (no penalty anywhere, G-D5).
    fn end_match(&mut self, code: &str, reason: &str) {
        let running = self.rooms.get(code).is_some_and(|r| r.mode.running());
        if !running {
            return;
        }
        if let Some(r) = self.rooms.get_mut(code) {
            r.mode.abort();
            r.cids.clear();
        }
        self.broadcast(code, &ServerMsg::MatchEnded { reason: reason.into() });
        self.send_room_state(code);
    }

    fn close_room(&mut self, code: &str, reason: &str) {
        let Some(mut room) = self.rooms.remove(code) else { return };
        room.mode.abort();
        let msg = ServerMsg::MatchEnded { reason: reason.into() };
        for m in room.seats.values_mut() {
            Self::to_member(&self.live, m, &msg);
            self.room_of.remove(&m.player);
        }
        for p in room.pending {
            self.pending_of.remove(&p.player);
            self.direct(&p.player, &ServerMsg::Denied { reason: "room_closed".into() });
        }
        tracing::info!(event = "room_closed", reason);
    }

    /// Removes a member (leave or seat-hold expiry) and repairs the room.
    fn remove_member(&mut self, code: &str, player: &str, now: Instant) {
        let Some(room) = self.rooms.get_mut(code) else { return };
        let Some(seat) = room.seat_of(player) else { return };
        room.seats.remove(&seat);
        self.room_of.remove(player);
        let (seats, kid_safe) = Self::ctx_parts(room);
        let ev = room.mode.on_disconnect(seat, true, &Ctx { now, seats: &seats, kid_safe });
        if room.seats.is_empty() {
            room.empty_since = Some(now);
        }
        let host_left = room.host == player;
        self.deliver(code, ev);
        if host_left {
            let room = self.rooms.get_mut(code).unwrap();
            // A3.3: earliest-joined adult; kids never hold host.
            let next = room
                .seats
                .values()
                .filter(|m| m.kind == ProfileKind::Adult)
                .min_by_key(|m| m.order)
                .map(|m| m.player.clone());
            match next {
                Some(p) => room.host = p,
                None => {
                    self.close_room(code, "host_left");
                    return;
                }
            }
        }
        let room = self.rooms.get(code).unwrap();
        if room.mode.running() && (room.seats.len() as u8) < room.mode.min_seats() {
            self.end_match(code, "player_left");
        } else {
            self.send_room_state(code);
        }
    }
}

impl Hub {
    pub fn new() -> Hub {
        Hub::default()
    }

    pub fn stats(&self) -> Stats {
        let s = self.state.lock().unwrap();
        Stats {
            connections: s.live.len(),
            rooms: s.rooms.len(),
            matches: s.rooms.values().filter(|r| r.mode.running()).count(),
            players_seated: s.room_of.len(),
        }
    }

    pub fn draining(&self) -> bool {
        self.state.lock().unwrap().draining.is_some()
    }

    pub fn live_matches(&self) -> usize {
        self.state.lock().unwrap().rooms.values().filter(|r| r.mode.running()).count()
    }

    /// `hello` accepted: registers the connection and sends `welcome`, then
    /// either replays the member's stream after `last_seq`, or (gap too large,
    /// or no resume point) a full `room_state` snapshot.
    pub fn attach(&self, sess: &Session, tx: UnboundedSender<Out>, resume: Option<(String, u64)>, _now: Instant) {
        let mut s = self.state.lock().unwrap();
        if let Some(old) = s.live.get(&sess.player) {
            if old.conn_id != sess.conn_id {
                let _ = old.tx.send(Out::Close(crate::protocol::close::REPLACED, "replaced"));
            }
        }
        s.live.insert(
            sess.player.clone(),
            Live { conn_id: sess.conn_id, tx: tx.clone(), kind: sess.kind, name: sess.name.clone() },
        );
        let _ = tx.send(Out::Frame(encode_server(
            0,
            &ServerMsg::Welcome { player_id: sess.player.clone(), protocol: crate::protocol::PROTOCOL_CURRENT },
        )));
        if let Some(deadline) = s.draining {
            let _ = tx.send(Out::Frame(encode_server(0, &ServerMsg::ServerDraining { deadline_unix_ms: deadline })));
        }
        let Some(code) = s.room_of.get(&sess.player).cloned() else { return };
        let room = s.rooms.get_mut(&code).unwrap();
        let seat = room.seat_of(&sess.player).unwrap();
        {
            let m = room.seats.get_mut(&seat).unwrap();
            m.connected = true;
            m.dropped_at = None;
        }
        let state = room.state();
        let m = room.seats.get_mut(&seat).unwrap();
        let replay: Option<Vec<String>> = match resume {
            Some((rc, last)) if rc == code => {
                let oldest = m.outbox.ring.front().map(|(q, _)| *q).unwrap_or(m.outbox.next_seq);
                if last + 1 >= oldest && last < m.outbox.next_seq {
                    Some(m.outbox.ring.iter().filter(|(q, _)| *q > last).map(|(_, f)| f.clone()).collect())
                } else {
                    None
                }
            }
            _ => None,
        };
        match replay {
            Some(frames) => {
                for f in frames {
                    let _ = tx.send(Out::Frame(f));
                }
            }
            None => {
                let frame = m.outbox.push(&ServerMsg::RoomState(state));
                let _ = tx.send(Out::Frame(frame));
            }
        }
        drop(s);
        // Others see the seat reconnect.
        self.state.lock().unwrap().send_room_state(&code);
    }

    /// The connection ended. The seat (if any) is held for `SEAT_HOLD`.
    pub fn detach(&self, player: &str, conn_id: u64, now: Instant) {
        let mut s = self.state.lock().unwrap();
        if s.live.get(player).map(|l| l.conn_id) != Some(conn_id) {
            return; // already replaced by a newer connection
        }
        s.live.remove(player);
        if let Some((code, id)) = s.pending_of.remove(player) {
            if let Some(r) = s.rooms.get_mut(&code) {
                r.pending.retain(|p| p.id != id);
            }
        }
        let Some(code) = s.room_of.get(player).cloned() else { return };
        let room = s.rooms.get_mut(&code).unwrap();
        let seat = room.seat_of(player).unwrap();
        let m = room.seats.get_mut(&seat).unwrap();
        m.connected = false;
        m.dropped_at = Some(now);
        let (seats, kid_safe) = State::ctx_parts(room);
        let ev = room.mode.on_disconnect(seat, false, &Ctx { now, seats: &seats, kid_safe });
        s.deliver(&code, ev);
        s.send_room_state(&code);
    }

    /// Handles one decoded frame from an authenticated connection.
    pub fn handle(&self, sess: &Session, inbound: Inbound, now: Instant) {
        let mut s = self.state.lock().unwrap();
        let p = sess.player.as_str();
        match inbound.msg {
            ClientMsg::Hello(_) => s.direct(p, &ServerMsg::error("already_hello")),
            ClientMsg::CreateRoom(req) => Self::create(&mut s, sess, req, now),
            ClientMsg::JoinRoom { code } => Self::join(&mut s, sess, &code, now),
            ClientMsg::ApproveJoin { request_id } => Self::decide(&mut s, p, request_id, true, now),
            ClientMsg::DenyJoin { request_id } => Self::decide(&mut s, p, request_id, false, now),
            ClientMsg::Leave {} => {
                if let Some((code, id)) = s.pending_of.remove(p) {
                    if let Some(r) = s.rooms.get_mut(&code) {
                        r.pending.retain(|x| x.id != id);
                    }
                } else if let Some(code) = s.room_of.get(p).cloned() {
                    s.remove_member(&code, p, now);
                    s.direct(p, &ServerMsg::MatchEnded { reason: "left".into() });
                } else {
                    s.direct(p, &ServerMsg::error("not_in_room"));
                }
            }
            ClientMsg::StartMatch {} => Self::start(&mut s, p, now),
            ClientMsg::Emote { id } => {
                let Some(code) = s.room_of.get(p).cloned() else {
                    return s.direct(p, &ServerMsg::error("not_in_room"));
                };
                if id >= EMOTES {
                    return s.direct(p, &ServerMsg::error("bad_emote"));
                }
                let seat = s.rooms[&code].seat_of(p).unwrap();
                s.broadcast(&code, &ServerMsg::Emote { seat, id });
            }
            msg @ (ClientMsg::NextWord {} | ClientMsg::Submit { .. } | ClientMsg::ReplayAudio { .. }) => {
                let Some(code) = s.room_of.get(p).cloned() else {
                    return s.direct(p, &ServerMsg::error("not_in_room"));
                };
                let room = s.rooms.get_mut(&code).unwrap();
                if !room.mode.running() {
                    return s.direct(p, &ServerMsg::error("no_match"));
                }
                let seat = room.seat_of(p).unwrap();
                let (seats, kid_safe) = State::ctx_parts(room);
                let ctx = Ctx { now, seats: &seats, kid_safe };
                let ev = match msg {
                    ClientMsg::NextWord {} => room.mode.on_next(seat, &ctx),
                    ClientMsg::ReplayAudio { variant } => room.mode.on_replay(seat, variant, &ctx),
                    ClientMsg::Submit { typed } => {
                        // F5: a duplicate cid is acknowledged by silence; never re-graded.
                        if !room.cids.insert(inbound.cid) {
                            return;
                        }
                        room.mode.on_submit(seat, &typed, &ctx)
                    }
                    _ => unreachable!(),
                };
                s.deliver(&code, ev);
            }
        }
    }

    fn create(s: &mut State, sess: &Session, req: CreateRoom, now: Instant) {
        let p = sess.player.as_str();
        if s.draining.is_some() {
            return s.direct(p, &ServerMsg::error("draining"));
        }
        if sess.kind == ProfileKind::Kid {
            return s.direct(p, &ServerMsg::error("not_allowed")); // O3, A3.6
        }
        if s.room_of.contains_key(p) || s.pending_of.contains_key(p) {
            return s.direct(p, &ServerMsg::error("already_in_room"));
        }
        if !s.limits.allow_create(p, now) {
            return s.direct(p, &ServerMsg::error("rate_limited"));
        }
        if req.tier_mode == TierMode::Fixed
            && !req.tier.as_deref().is_some_and(|t| spell_wasm::native_api::TIER_ORDER.contains(&t))
        {
            return s.direct(p, &ServerMsg::error("bad_tier"));
        }
        if !spell_wasm::native_api::is_active_lang(&req.lang) {
            return s.direct(p, &ServerMsg::error("lang_unavailable"));
        }
        let mode = match mode::make(&req) {
            Ok(m) => m,
            Err(MakeError::Unavailable) => return s.direct(p, &ServerMsg::error("mode_unavailable")),
            Err(MakeError::Unknown) => return s.direct(p, &ServerMsg::error("unknown_mode")),
        };
        let code = loop {
            let c = new_code();
            if !s.rooms.contains_key(&c) {
                break c;
            }
        };
        s.order += 1;
        let mut seats = BTreeMap::new();
        seats.insert(
            0,
            Member {
                player: p.to_string(),
                name: sess.name.clone(),
                kind: sess.kind,
                order: s.order,
                connected: true,
                dropped_at: None,
                outbox: Outbox::new(),
            },
        );
        s.rooms.insert(
            code.clone(),
            Room {
                code: code.clone(),
                req,
                mode,
                host: p.to_string(),
                seats,
                pending: Vec::new(),
                kid_safe: false,
                created: now,
                empty_since: None,
                cids: HashSet::new(),
            },
        );
        s.room_of.insert(p.to_string(), code.clone());
        tracing::info!(event = "room_created", player_id = p);
        let State { live, rooms, .. } = s;
        let room = rooms.get_mut(&code).unwrap();
        let m = room.seats.get_mut(&0).unwrap();
        State::to_member(live, m, &ServerMsg::Joined { code: code.clone(), seat: 0 });
        s.send_room_state(&code);
    }

    fn join(s: &mut State, sess: &Session, raw: &str, now: Instant) {
        let p = sess.player.as_str();
        if s.limits.join_blocked(sess.ip, p, now) {
            return s.direct(p, &ServerMsg::error("rate_limited"));
        }
        let fail = |s: &mut State, code: &str| {
            let tripped = s.limits.join_failed(sess.ip, p, now);
            s.direct(p, &ServerMsg::error(if tripped { "rate_limited" } else { code }));
            tracing::info!(event = "join_failed", player_id = p, code);
        };
        if s.room_of.contains_key(p) || s.pending_of.contains_key(p) {
            return s.direct(p, &ServerMsg::error("already_in_room"));
        }
        let Some(code) = normalize_code(raw) else { return fail(s, "bad_code") };
        let Some(room) = s.rooms.get_mut(&code) else { return fail(s, "room_not_found") };
        if room.mode.running() {
            return fail(s, "match_in_progress");
        }
        if room.seats.len() as u8 >= room.mode.max_seats() {
            return fail(s, "room_full");
        }
        if room.needs_approval(sess.kind) {
            let id = s.id();
            let room = s.rooms.get_mut(&code).unwrap();
            room.pending.push(Pending { id, player: p.into(), name: sess.name.clone(), kind: sess.kind, since: now });
            let host = room.host.clone();
            s.pending_of.insert(p.into(), (code.clone(), id));
            s.direct(p, &ServerMsg::JoinPending { code: code.clone(), request_id: id });
            let State { live, rooms, .. } = s;
            let room = rooms.get_mut(&code).unwrap();
            let hs = room.seat_of(&host).unwrap();
            State::to_member(
                live,
                room.seats.get_mut(&hs).unwrap(),
                &ServerMsg::JoinRequest { request_id: id, name: sess.name.clone(), kind: sess.kind },
            );
            return;
        }
        Self::seat(s, &code, p, sess.name.clone(), sess.kind, now);
    }

    fn seat(s: &mut State, code: &str, player: &str, name: String, kind: ProfileKind, _now: Instant) {
        s.order += 1;
        let order = s.order;
        let connected = s.live.contains_key(player);
        let State { live, rooms, room_of, .. } = s;
        let room = rooms.get_mut(code).unwrap();
        let seat = (0..room.mode.max_seats()).find(|i| !room.seats.contains_key(i)).unwrap();
        let mut m = Member {
            player: player.into(),
            name,
            kind,
            order,
            connected,
            dropped_at: if connected { None } else { Some(_now) },
            outbox: Outbox::new(),
        };
        State::to_member(live, &mut m, &ServerMsg::Joined { code: code.into(), seat });
        room.seats.insert(seat, m);
        room.empty_since = None;
        if kind == ProfileKind::Kid {
            room.kid_safe = true; // G-D13: sticky for the room's life
        }
        room_of.insert(player.into(), code.into());
        s.send_room_state(code);
    }

    fn decide(s: &mut State, host: &str, id: u64, approve: bool, now: Instant) {
        let Some(code) = s.room_of.get(host).cloned() else {
            return s.direct(host, &ServerMsg::error("not_in_room"));
        };
        let room = s.rooms.get_mut(&code).unwrap();
        if room.host != host {
            return s.direct(host, &ServerMsg::error("not_host"));
        }
        let Some(pos) = room.pending.iter().position(|x| x.id == id) else {
            return s.direct(host, &ServerMsg::error("no_request"));
        };
        if approve && room.seats.len() as u8 >= room.mode.max_seats() {
            return s.direct(host, &ServerMsg::error("room_full"));
        }
        if approve && room.mode.running() {
            return s.direct(host, &ServerMsg::error("match_in_progress"));
        }
        let req = room.pending.remove(pos);
        s.pending_of.remove(&req.player);
        if approve {
            Self::seat(s, &code, &req.player, req.name, req.kind, now);
        } else {
            s.direct(&req.player, &ServerMsg::Denied { reason: "host".into() });
        }
    }

    fn start(s: &mut State, p: &str, now: Instant) {
        let Some(code) = s.room_of.get(p).cloned() else {
            return s.direct(p, &ServerMsg::error("not_in_room"));
        };
        if s.draining.is_some() {
            return s.direct(p, &ServerMsg::error("draining"));
        }
        let room = s.rooms.get_mut(&code).unwrap();
        if room.host != p {
            return s.direct(p, &ServerMsg::error("not_host"));
        }
        if room.mode.running() {
            return s.direct(p, &ServerMsg::error("match_in_progress"));
        }
        let n = room.seats.len() as u8;
        if n < room.mode.min_seats() || n > room.mode.max_seats() {
            return s.direct(p, &ServerMsg::error("not_enough_players"));
        }
        if room.seats.values().any(|m| !m.connected) {
            return s.direct(p, &ServerMsg::error("seat_disconnected"));
        }
        room.cids.clear();
        let (seats, kid_safe) = State::ctx_parts(room);
        let ev = room.mode.on_start(&Ctx { now, seats: &seats, kid_safe });
        tracing::info!(event = "match_started", player_id = p);
        s.send_room_state(&code);
        s.deliver(&code, ev);
    }

    /// Periodic housekeeping: pending expiry, seat holds, room lifetimes, and
    /// mode deadlines. Safe to call at any cadence.
    pub fn tick(&self, now: Instant) {
        let mut s = self.state.lock().unwrap();
        let codes: Vec<String> = s.rooms.keys().cloned().collect();
        for code in codes {
            // Pending requests (A3.5).
            let expired: Vec<Pending> = {
                let room = s.rooms.get_mut(&code).unwrap();
                let (gone, keep): (Vec<_>, Vec<_>) =
                    room.pending.drain(..).partition(|p| now.duration_since(p.since) >= PENDING_TTL);
                room.pending = keep;
                gone
            };
            for p in expired {
                s.pending_of.remove(&p.player);
                s.direct(&p.player, &ServerMsg::Denied { reason: "expired".into() });
            }
            // Seat holds (A5.2).
            let released: Vec<String> = s.rooms[&code]
                .seats
                .values()
                .filter(|m| m.dropped_at.is_some_and(|t| now.duration_since(t) >= SEAT_HOLD))
                .map(|m| m.player.clone())
                .collect();
            for player in released {
                s.remove_member(&code, &player, now);
                if !s.rooms.contains_key(&code) {
                    break;
                }
            }
            let Some(room) = s.rooms.get_mut(&code) else { continue };
            // Lifetimes.
            if now.duration_since(room.created) >= ROOM_LIFETIME {
                s.close_room(&code, "room_expired");
                continue;
            }
            if room.seats.is_empty() && room.empty_since.is_some_and(|t| now.duration_since(t) >= EMPTY_TTL) {
                s.close_room(&code, "room_empty");
                continue;
            }
            // Mode deadlines.
            if room.mode.running() && room.mode.next_deadline().is_some_and(|d| d <= now) {
                let (seats, kid_safe) = State::ctx_parts(room);
                let ev = room.mode.on_timer(&Ctx { now, seats: &seats, kid_safe });
                s.deliver(&code, ev);
            }
        }
        s.limits.gc(now);
    }

    /// F11 drain: refuse new rooms and tell everyone when the server goes.
    pub fn start_drain(&self, deadline_unix_ms: u64) {
        let mut s = self.state.lock().unwrap();
        if s.draining.is_some() {
            return;
        }
        s.draining = Some(deadline_unix_ms);
        let msg = ServerMsg::ServerDraining { deadline_unix_ms };
        let seated: HashSet<String> = s.room_of.keys().cloned().collect();
        for p in s.live.keys().filter(|p| !seated.contains(*p)) {
            s.direct(p, &msg);
        }
        let codes: Vec<String> = s.rooms.keys().cloned().collect();
        for c in codes {
            s.broadcast(&c, &msg);
        }
        tracing::info!(event = "drain_started");
    }

    /// The drain deadline passed: end every match with `server_update`.
    pub fn end_all(&self, reason: &str) {
        let mut s = self.state.lock().unwrap();
        let codes: Vec<String> = s.rooms.keys().cloned().collect();
        for c in codes {
            s.close_room(&c, reason);
        }
        let players: Vec<String> = s.live.keys().cloned().collect();
        for p in players {
            s.close_conn(&p, crate::protocol::close::SHUTDOWN, "shutdown");
        }
    }

    /// Admin revocation: drop that player's live connection now (A2.3).
    pub fn kick(&self, player: &str, code: u16, why: &'static str) {
        self.state.lock().unwrap().close_conn(player, code, why);
    }

    /// A2.4: session mints share the hub's limiter.
    pub fn allow_mint(&self, ip: IpAddr, now: Instant) -> bool {
        self.state.lock().unwrap().limits.allow_mint(ip, now)
    }

    pub fn is_connected(&self, player: &str) -> bool {
        self.state.lock().unwrap().live.contains_key(player)
    }

    /// Tests: is this player seated, and where?
    pub fn seat_of(&self, player: &str) -> Option<(String, u8)> {
        let s = self.state.lock().unwrap();
        let code = s.room_of.get(player)?.clone();
        let seat = s.rooms.get(&code)?.seat_of(player)?;
        Some((code, seat))
    }

    pub fn host_of(&self, code: &str) -> Option<String> {
        self.state.lock().unwrap().rooms.get(code).map(|r| r.host.clone())
    }

    pub fn room_state(&self, code: &str) -> Option<RoomState> {
        self.state.lock().unwrap().rooms.get(code).map(|r| r.state())
    }

    pub fn room_exists(&self, code: &str) -> bool {
        self.state.lock().unwrap().rooms.contains_key(code)
    }

    pub fn live_name(&self, player: &str) -> Option<String> {
        self.state.lock().unwrap().live.get(player).map(|l| l.name.clone())
    }

    pub fn live_kind(&self, player: &str) -> Option<ProfileKind> {
        self.state.lock().unwrap().live.get(player).map(|l| l.kind)
    }
}

#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;
