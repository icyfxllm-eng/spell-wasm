//! CC-BOARD-GAME-ONLINE F6/F7 -- the board screen in an online room.
//!
//! The game is the server's. `Ui::game` holds `online_board`'s display mirror,
//! and every tap goes to the room as a message; nothing here hands the mirror
//! to `boardgame::apply`. What the server sends comes back through `frame`,
//! is turned into the solo game's own events, and the solo screen draws them:
//! the ring, the walks, the chips and the stage are unchanged.
//!
//! Every seat but mine is drawn as an NPC seat (`seat.npc` on the mirror), so
//! the solo screen's "not your turn" paths (no key grid, the orb waiting) hold
//! for other players too. Which seats the server really plays is `npc`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use serde_json::{json, Value};

use super::{after_apply, finish_record, piece, render, render_keys, with_ui, Ui, HUMAN_BEAT_MS, NPC_STEP_MS, UI};
use crate::boardgame::{Phase, Variant};
use crate::online_board::{Mirror, AWAITING_SPELL};
use crate::online_model::ServerMsg;
use crate::{dom, haptics, i18n, online_net as net};

/// What a tap sent, until the server answers it (the next `turn.begin`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Sent {
    cid: String,
    t: &'static str,
    body: Value,
    phase: Phase,
}

pub(super) struct Online {
    pub mirror: Mirror,
    /// My rules seat; `None` for a seat the server already plays.
    pub me: Option<u8>,
    /// Per rules seat.
    pub names: Vec<String>,
    pub away: Vec<bool>,
    ticket: Option<String>,
    /// The phase's limit in ms, and when its `turn.begin` arrived.
    deadline: Option<(u64, f64)>,
    sent: Option<Sent>,
    pub ranking: Option<Vec<u8>>,
    /// The line under the board, while it is not the solo feed's.
    note: String,
}

thread_local! {
    /// Frames wait here so NPC turns are paced as solo paces them (D10).
    static INBOX: RefCell<VecDeque<(f64, ServerMsg)>> = const { RefCell::new(VecDeque::new()) };
    static PUMPING: Cell<bool> = const { Cell::new(false) };
    static PUMP_TOKEN: Cell<u32> = const { Cell::new(0) };
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// A room seat as the lobby knows it: wire seat and name.
pub struct SeatName {
    pub seat: u8,
    pub name: String,
}

/// Opens (or reopens, after a resume) the board on the room's snapshot
/// (`room_state.mode_state`, F7). `false`: the snapshot is not a board.
pub fn open(state: &Value, lang: &str, kid: bool, my_seat: u8, seats: &[SeatName]) -> bool {
    let Some(view) = state.get("view") else { return false };
    let Some((mirror, mut game)) = Mirror::from_view(view, lang, kid) else { return false };
    let names: Vec<String> = mirror
        .table
        .iter()
        .map(|w| seats.iter().find(|s| s.seat == *w).map(|s| s.name.clone()).unwrap_or_default())
        .collect();
    let me = mirror.index(my_seat).filter(|i| !mirror.npc[*i as usize]);
    let n = mirror.table.len();
    let mut away = vec![false; n];
    if let Some(st) = state.get("seats").and_then(Value::as_array) {
        for s in st {
            let i = s.get("seat").and_then(Value::as_u64).and_then(|w| mirror.index(w as u8));
            if let (Some(i), Some(status)) = (i, s.get("status").and_then(Value::as_str)) {
                away[i as usize] = status == "grace";
            }
        }
    }
    // What a tap sent before the drop survives the rebuild (resend checks it).
    let sent = with_ui(|u| u.online.as_mut().and_then(|o| o.sent.take())).flatten();
    let mut o = Online { mirror, me, names, away, ticket: None, deadline: None, sent, ranking: None, note: String::new() };
    if let (Some(active), Some(phase)) = (state.get("active_seat").and_then(Value::as_u64), state.get("phase").and_then(Value::as_str)) {
        let payload = state.get("payload").cloned().unwrap_or(Value::Null);
        o.mirror.begin(&mut game, active as u8, phase, &payload);
        o.ticket = state.get("audio_ticket").and_then(Value::as_str).map(str::to_string);
        o.deadline = state.get("deadline_ms").and_then(Value::as_u64).map(|ms| (ms, now_ms()));
    }
    mark_mine(&mut game, me);
    let keys = keys_for(lang, kid, game.cfg.variant);
    super::cancel_anim();
    INBOX.with(|i| i.borrow_mut().clear());
    PUMPING.with(|p| p.set(false));
    PUMP_TOKEN.with(|t| t.set(t.get().wrapping_add(1)));
    let shown: Vec<u32> = game.players.iter().map(|p| p.pos).collect();
    let act = game.current_seat();
    let seen = game.events.len();
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui {
            game,
            lang: lang.to_string(),
            kid,
            relaxed: 0,
            day: super::day(),
            typed: String::new(),
            keys,
            pass: false,
            shown_seat: None,
            miss: None,
            seen,
            chips: Vec::new(),
            feed: String::new(),
            recorded: false,
            land: None,
            shown,
            gid: now_ms() as u64,
            act,
            roll: None,
            roll_id: 0,
            roll_at: 0.0,
            marks: Vec::new(),
            star: None,
            online: Some(o),
        })
    });
    for id in ["bgField", "bgKeys"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            dom::set_lang_dir(&e, lang);
        }
    }
    render_keys();
    dom::set_hidden("bgSetup", true);
    dom::set_hidden("bgPlay", false);
    for id in ["bgHand", "bgMiss", "bgOver"] {
        dom::set_hidden(id, true);
    }
    dom::add_class("bgScreen", "playing");
    dom::add_class("bgScreen", "online");
    dom::add_class("bgScreen", "show");
    render();
    true
}

/// The keys come from the room language's own bank, as solo builds them.
fn keys_for(lang: &str, kid: bool, v: Variant) -> Vec<Vec<crate::boardgame_input::Key>> {
    let entries: Vec<String> = crate::boardgame_pools::supply(lang, v, &Default::default(), 0, 0)
        .map(|s| s.pools.tiers.iter().flatten().chain(s.pools.long_word.iter()).cloned().collect())
        .unwrap_or_default();
    crate::boardgame_input::layout(lang, kid, &entries)
}

/// Every seat but mine reads as an NPC seat to the solo screen (see the file note).
fn mark_mine(game: &mut crate::boardgame::BoardGameState, me: Option<u8>) {
    for (i, p) in game.players.iter_mut().enumerate() {
        p.seat.npc = Some(i as u8) != me;
    }
}

pub fn is_open() -> bool {
    with_ui(|u| u.online.is_some()).unwrap_or(false)
}

/// The board closes without leaving the room (the room is the online screen's).
pub fn close() {
    INBOX.with(|i| i.borrow_mut().clear());
    PUMP_TOKEN.with(|t| t.set(t.get().wrapping_add(1)));
    PUMPING.with(|p| p.set(false));
    super::cancel_anim();
    crate::speech_out::stop();
    finish_record();
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("bgScreen", "online");
    dom::remove_class("bgScreen", "show");
    dom::remove_class("bgScreen", "playing");
}

// -------------------------------------------------------------------- taps

fn my_turn(u: &Ui) -> bool {
    u.online.as_ref().is_some_and(|o| o.me == Some(u.game.current_seat())) && u.game.phase != Phase::Finished
}

/// A tap is waiting for its answer: nothing more may be sent or typed.
pub(super) fn locked(u: &Ui) -> bool {
    u.online.as_ref().is_some_and(|o| o.sent.is_some())
}

fn send(t: &'static str, body: Value) {
    let Some(phase) = with_ui(|u| (my_turn(u) && !locked(u)).then_some(u.game.phase)).flatten() else { return };
    let cid = net::cid();
    net::send_as(t, &cid, body.clone());
    with_ui(|u| {
        if let Some(o) = u.online.as_mut() {
            o.sent = Some(Sent { cid, t, body, phase });
        }
    });
    render();
}

pub(super) fn orb() {
    let Some((phase, mine, ticket)) = with_ui(|u| (u.game.phase, my_turn(u), u.online.as_ref().and_then(|o| o.ticket.clone()))) else { return };
    match phase {
        Phase::AwaitRoll if mine => {
            with_ui(|u| {
                u.chips.clear();
                u.feed.clear();
            });
            haptics::tap();
            send("next_word", json!({}));
        }
        // The same ticket again, locally (as the online Head-to-Head replay).
        Phase::AwaitSpelling => {
            if let Some(t) = ticket {
                crate::online_audio::play(t, || {});
            }
        }
        _ => {}
    }
}

pub(super) fn submit(typed: String) {
    let ok = with_ui(|u| my_turn(u) && u.game.phase == Phase::AwaitSpelling).unwrap_or(false);
    if ok {
        send("turn.answer", json!({ "answer": typed }));
    }
}

pub(super) fn stretch(take: bool) {
    haptics::tap();
    send("turn.choice", json!({ "choice": if take { "yes" } else { "no" } }));
}

pub(super) fn swap(target: Option<u8>) {
    let choice = match target {
        Some(i) => with_ui(|u| u.online.as_ref().map(|o| o.mirror.wire(i).to_string())).flatten().unwrap_or_default(),
        None => "nobody".to_string(),
    };
    send("turn.choice", json!({ "choice": choice }));
}

/// After a resume settled: a tap the server still waits for is sent again,
/// with its own cid, so one that did arrive is not counted twice (F5).
pub fn resend() {
    let again = with_ui(|u| {
        let phase = u.game.phase;
        let mine = my_turn(u);
        let o = u.online.as_mut()?;
        let s = o.sent.clone()?;
        if mine && s.phase == phase {
            Some(s)
        } else {
            o.sent = None;
            None
        }
    })
    .flatten();
    if let Some(s) = again {
        net::send_as(s.t, &s.cid, s.body);
    }
    render();
}

// -------------------------------------------------------------------- frames

/// One turn-engine frame from the room. Frames are handled in order, with an
/// NPC's turn given the time solo gives it.
pub fn frame(msg: ServerMsg) {
    if !is_open() {
        return;
    }
    INBOX.with(|i| i.borrow_mut().push_back((now_ms(), msg)));
    if !PUMPING.with(Cell::get) {
        pump();
    }
}

fn pump() {
    loop {
        let Some((at, msg)) = INBOX.with(|i| i.borrow_mut().pop_front()) else {
            PUMPING.with(|p| p.set(false));
            return;
        };
        let wait = handle(at, msg);
        if wait > 0 {
            PUMPING.with(|p| p.set(true));
            let token = PUMP_TOKEN.with(Cell::get);
            dom::after_ms(wait, move || {
                if PUMP_TOKEN.with(Cell::get) == token {
                    pump();
                }
            });
            return;
        }
    }
}

fn name_of(u: &Ui, i: u8) -> String {
    u.online.as_ref().and_then(|o| o.names.get(i as usize).cloned()).filter(|n| !n.is_empty()).unwrap_or_else(|| piece(u.game.players[i as usize].seat.piece).to_string())
}

/// Handles one frame; returns how long the next one waits (ms).
fn handle(at: f64, msg: ServerMsg) -> i32 {
    match msg {
        ServerMsg::TurnBegin { seat, phase, payload, deadline_ms } => {
            let mine = with_ui(|u| {
                let o = u.online.as_mut()?;
                o.mirror.begin(&mut u.game, seat, &phase, &payload);
                o.sent = None;
                o.ticket = None;
                o.note.clear();
                o.deadline = deadline_ms.map(|ms| (ms, at));
                let me = o.me;
                mark_mine(&mut u.game, me);
                let mine = me == Some(u.game.current_seat());
                if mine && phase == AWAITING_SPELL {
                    u.typed.clear();
                }
                if u.game.phase == Phase::AwaitRoll {
                    u.act = u.game.current_seat();
                    u.roll = None;
                }
                Some(mine)
            })
            .flatten()
            .unwrap_or(false);
            if mine {
                haptics::tap();
            }
            render();
            0
        }
        ServerMsg::TurnAudio { audio_ticket, .. } => {
            with_ui(|u| {
                if let Some(o) = u.online.as_mut() {
                    o.ticket = Some(audio_ticket.clone());
                }
            });
            // Every seat hears the word, as everyone at one phone would.
            crate::online_audio::play(audio_ticket, || {});
            0
        }
        ServerMsg::TurnVerdict { seat, word, answer, correct, effects } => {
            let info = with_ui(|u| {
                let o = u.online.as_mut()?;
                let i = o.mirror.index(seat)?;
                let mine = o.me == Some(i);
                if mine {
                    Mirror::my_verdict(&mut u.game, i, &word, correct);
                }
                let shown = crate::boardgame_grade::citation(&word).to_string();
                u.feed = if correct { format!("\u{2713} {shown}") } else { i18n::tp("bg.word", &[("word", &shown)]) };
                let fx = o.mirror.clone();
                fx.effects(&mut u.game, &effects);
                Some((mine, u.kid))
            })
            .flatten();
            let Some((mine, kid)) = info else { return 0 };
            if mine {
                crate::keyboard_moments::react_for("boardgame", "bgKeys", correct, &answer, 0);
                if correct {
                    haptics::correct();
                    crate::themes::note_correct();
                } else {
                    haptics::incorrect(kid);
                }
            }
            let walk = after_apply(true);
            render();
            HUMAN_BEAT_MS.max(walk + 200)
        }
        ServerMsg::TurnEffects { seat, effects } => {
            let npc = with_ui(|u| {
                let o = u.online.as_ref()?;
                let i = o.mirror.index(seat)?;
                let npc = o.mirror.npc.get(i as usize).copied().unwrap_or(false);
                let m = o.mirror.clone();
                m.effects(&mut u.game, &effects);
                Some(npc && !effects.is_empty())
            })
            .flatten()
            .unwrap_or(false);
            let walk = after_apply(true);
            render();
            // D10: an NPC's whole turn is one step, its walk inside it.
            if npc {
                NPC_STEP_MS
            } else if walk > 0 {
                walk + 200
            } else {
                0
            }
        }
        ServerMsg::TurnAuto { seat, reason } => {
            with_ui(|u| {
                let i = u.online.as_ref().and_then(|o| o.mirror.index(seat));
                if let Some(i) = i {
                    let key = if reason == "audio_unavailable" { "ol.end.audio" } else { "ol.timeUp" };
                    let line = i18n::tp(key, &[("name", &name_of(u, i))]);
                    if let Some(o) = u.online.as_mut() {
                        o.note = line;
                        o.sent = None;
                    }
                }
            });
            render();
            0
        }
        ServerMsg::SeatStatus { seat, status } => {
            with_ui(|u| {
                let Some(o) = u.online.as_mut() else { return };
                let Some(i) = o.mirror.index(seat) else { return };
                o.away[i as usize] = status == "grace";
                o.mirror.npc[i as usize] = status == "npc";
                // My own seat handed to the server: I watch from here on.
                if status == "npc" && o.me == Some(i) {
                    o.me = None;
                }
            });
            render();
            0
        }
        ServerMsg::GameEnd { ranking, summary } => {
            with_ui(|u| {
                let Some(o) = u.online.as_mut() else { return };
                let order = o.mirror.end(&mut u.game, &ranking, &summary);
                o.ranking = Some(order);
                o.deadline = None;
                o.sent = None;
            });
            let walk = after_apply(true);
            render();
            let token = PUMP_TOKEN.with(Cell::get);
            dom::after_ms(walk + 300, move || {
                if PUMP_TOKEN.with(Cell::get) == token {
                    show_over();
                }
            });
            0
        }
        _ => 0,
    }
}

/// Test builds: the online part of `seam_state`.
#[cfg(feature = "testseam")]
pub(super) fn seam(u: &Ui) -> Value {
    let Some(o) = u.online.as_ref() else { return Value::Null };
    json!({
        "me": o.me,
        "mine": my_turn(u),
        "npc": o.mirror.npc,
        "away": o.away,
        "locked": o.sent.is_some(),
        "table": o.mirror.table,
        "pieces": u.game.players.iter().map(|p| p.seat.piece).collect::<Vec<_>>(),
        "ticket": o.ticket.is_some(),
        "ranking": o.ranking,
    })
}

// -------------------------------------------------------------------- drawing

/// An emote from the room, on the line under the board for a moment (F6).
pub fn emote(name: &str, glyph: &str) {
    let line = format!("{name} {glyph}");
    with_ui(|u| {
        if let Some(o) = u.online.as_mut() {
            o.note = line.clone();
        }
    });
    render();
    let token = PUMP_TOKEN.with(Cell::get);
    dom::after_ms(2500, move || {
        if PUMP_TOKEN.with(Cell::get) != token {
            return;
        }
        let stale = with_ui(|u| match u.online.as_mut() {
            Some(o) if o.note == line => {
                o.note.clear();
                true
            }
            _ => false,
        })
        .unwrap_or(false);
        if stale {
            render();
        }
    });
}

/// Online additions to `render`: names on the seat chips, the shared timer,
/// and who the room is waiting on.
pub(super) fn decorate(u: &mut Ui) {
    let Some(o) = u.online.as_ref() else { return };
    let g = &u.game;
    let cur = g.current_seat();
    let mine = o.me == Some(cur) && g.phase != Phase::Finished;
    let waiting = !mine && g.phase != Phase::Finished;
    dom::toggle_class("bgOrb", "other", waiting && g.phase == Phase::AwaitSpelling);
    if waiting {
        let name = name_of(u, cur);
        let key = match g.phase {
            Phase::AwaitSpelling => "ol.spelling",
            Phase::AwaitStretch | Phase::AwaitSwitchTarget => "ol.choosing",
            _ => "ol.rolling",
        };
        let mut line = i18n::tp(key, &[("name", &name)]);
        if o.away.get(cur as usize).copied().unwrap_or(false) {
            line = i18n::tp("ol.waitBack", &[("name", &name)]);
        }
        if !o.note.is_empty() {
            line = format!("{} {}", o.note, line);
        }
        dom::set_text("bgFeed", &line);
    } else if !o.note.is_empty() {
        dom::set_text("bgFeed", &o.note);
    }
    if locked(u) {
        let _ = dom::el("bgOrb").set_attribute("disabled", "");
        dom::set_disabled("bgGo", true);
    } else {
        dom::set_disabled("bgGo", false);
    }
    // The server's clock decides (D-O17); this bar only shows it, for every seat.
    match o.deadline {
        Some((ms, at)) if g.phase == Phase::AwaitSpelling || mine => {
            let left = (ms as f64 - (now_ms() - at)).max(0.0);
            dom::set_hidden("bgTimer", false);
            let el: web_sys::HtmlElement = wasm_bindgen::JsCast::unchecked_into(dom::el("bgTimer"));
            let sig = format!("{at}");
            if el.get_attribute("data-for").as_deref() != Some(sig.as_str()) {
                let _ = el.set_attribute("data-for", &sig);
                let _ = el.style().set_property("--motion-mode-boardgame-timer", &format!("{}ms", left as u64));
                dom::remove_class("bgTimer", "run");
                dom::after_ms(30, || dom::add_class("bgTimer", "run"));
            }
        }
        _ => {
            dom::set_hidden("bgTimer", true);
            dom::remove_class("bgTimer", "run");
        }
    }
}

/// The name a seat chip carries online, and its state.
pub(super) fn chip_label(u: &Ui, i: usize) -> Option<(String, String)> {
    let o = u.online.as_ref()?;
    let name = o.names.get(i).cloned().unwrap_or_default();
    let mut tag = String::new();
    if o.me == Some(i as u8) {
        tag = i18n::t("ol.you");
    } else if o.away.get(i).copied().unwrap_or(false) {
        tag = i18n::t("ol.away");
    } else if o.mirror.npc.get(i).copied().unwrap_or(false) {
        tag = i18n::t("ol.cpu");
    }
    Some((name, tag))
}

/// F8: the result card, with every seat's row and the online badge.
fn show_over() {
    finish_record();
    let rows = with_ui(|u| {
        let o = u.online.as_ref()?;
        let order = o.ranking.clone().unwrap_or_else(|| u.game.standings());
        let mut h = String::new();
        for (rank, seat) in order.into_iter().enumerate() {
            let p = &u.game.players[seat as usize];
            let medal = super::medal(rank);
            let (name, tag) = chip_label(u, seat as usize).unwrap_or_default();
            let who = if tag.is_empty() { name.clone() } else { format!("{name} \u{b7} {tag}") };
            h.push_str(&format!(
                "<div class=\"bg-pod\" aria-label=\"{} {}\"><span>{}</span><span>{}</span><span>{}</span><b>{}</b></div>",
                piece(p.seat.piece),
                dom::escape_html(&name),
                medal,
                piece(p.seat.piece),
                p.pos,
                dom::escape_html(&who)
            ));
        }
        Some(h)
    })
    .flatten()
    .unwrap_or_default();
    record_match();
    dom::set_html("bgPodium", &format!("<p class=\"bg-online\">{}</p>{rows}", dom::escape_html(&i18n::t("ol.onlineBadge"))));
    dom::set_hidden("bgShare", true);
    dom::set_text("bgAgain", &i18n::t("ol.backToRoom"));
    dom::set_hidden("bgOver", false);
}

/// F8: a local count of online games, for a future badge. Nothing leaves the phone.
const MATCHES_KEY: &str = "bg_online_matches_v1";

fn record_match() {
    let won = with_ui(|u| u.online.as_ref().is_some_and(|o| o.me.is_some() && o.ranking.as_ref().and_then(|r| r.first()) == o.me.as_ref())).unwrap_or(false);
    let mut v: Value = crate::storage::get_json(MATCHES_KEY).unwrap_or_else(|| json!({ "played": 0, "won": 0 }));
    let bump = |v: &mut Value, k: &str| v[k] = json!(v[k].as_u64().unwrap_or(0) + 1);
    bump(&mut v, "played");
    if won {
        bump(&mut v, "won");
    }
    crate::storage::set_json(MATCHES_KEY, &v);
}
