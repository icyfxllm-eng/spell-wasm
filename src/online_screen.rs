//! CC-GAME-SERVER F12/F13/F10 (client) — the Online Head-to-Head screen.
//!
//! Its own screen in its own files, by Eric's call (2026-10-25): it borrows the
//! shared keyboard, the orb's look and the versus bar's styling, and never runs
//! a line of the solo play path. Solo state is not read or written here; the
//! one shared thing is the parent gate (F10).
//!
//! The entry button lives in the local Head-to-Head setup sheet and exists only
//! when `online_model::entry_visible` says so. With the build flag off it is
//! hidden and this module makes no request at all (A13.4).

use std::cell::{Cell, RefCell};

use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::online_model::{self as om, Effect, Model, Panel};
use crate::online_net::{self as net, close};
use crate::{dom, App};

/// Parent-gated per-device setting: "Play online with family" (F10).
const FAMILY_KEY: &str = "spell_online_family_v1";
/// The adult's typed name, remembered for next time (a convenience only).
const NAME_KEY: &str = "spell_online_name_v1";
/// O: Eric supplies the id; until then the card links to this placeholder.
const APP_STORE_URL: &str = "https://apps.apple.com/app/idXXXXXXXXXX";

thread_local! {
    static MODEL: RefCell<Model> = RefCell::new(Model::default());
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    /// Bad-open count with no room (F13: two, then the resting card).
    static FAILS: Cell<u32> = const { Cell::new(0) };
    /// When a drop with a room began (ms), for the 60 s reconnect overlay.
    static DROPPED_AT: Cell<Option<f64>> = const { Cell::new(None) };
    static SUBMITTED: Cell<bool> = const { Cell::new(false) };
    static KEYS_LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static LEAVING: Cell<bool> = const { Cell::new(false) };
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn is_kid() -> bool {
    APP.with(|a| a.borrow().as_ref().is_some_and(|a| a.borrow().kid))
}

fn family_ok() -> bool {
    crate::storage::get_raw(FAMILY_KEY).as_deref() == Some("on")
}

/// F10 + F13: does the entry exist right now? It follows the BUILD's flag:
/// a remote kill leaves the entry in place and opening it shows the resting
/// card (F13 table); only a build with the flag off has no entry at all.
pub fn entry_visible() -> bool {
    om::entry_visible(crate::flags::online_enabled_built(), crate::flags::online_kids(), is_kid(), family_ok())
}

fn with_model<R>(f: impl FnOnce(&mut Model) -> R) -> R {
    MODEL.with(|m| f(&mut m.borrow_mut()))
}

// ---------------------------------------------------------------- setup

pub fn setup(app: &App) {
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));
    // The setup sheet opens from vsBtn; decide the entry each time it does.
    dom::on_click("vsBtn", || dom::set_hidden("vsOnline", !entry_visible()));
    dom::set_hidden("vsOnline", true);
    dom::on_click("vsOnline", || {
        if !entry_visible() {
            return;
        }
        dom::remove_class("vsSetupScrim", "show");
        open_screen();
    });
    wire_family();
    wire_controls();
    net::set_sink(on_net);
    wire_keys_capture();
    wire_visibility();
}

/// F10: the per-device "Play online with family" switch, shown only for a kid
/// profile while `online.kids` is on. Changing it always asks a grown-up.
fn wire_family() {
    let reflect = || {
        let show = crate::flags::online_kids() && crate::flags::online_enabled_built() && is_kid();
        dom::set_hidden("olFamilyRow", !show);
        dom::input("olFamily").set_checked(family_ok());
    };
    reflect();
    dom::on_click("setBtn", reflect);
    dom::on::<web_sys::Event, _>("olFamily", "change", move |_| {
        let want = dom::input("olFamily").checked();
        // Undo the tap until the grown-up has answered.
        dom::input("olFamily").set_checked(!want);
        crate::parent_gate_then(Box::new(move || {
            crate::storage::set_raw(FAMILY_KEY, if want { "on" } else { "off" });
            dom::input("olFamily").set_checked(want);
        }));
    });
}

fn wire_controls() {
    // Every panel has a way out, the end panels included (mode-exit-check).
    dom::on_click("olExit", close_screen);
    dom::on_click("olResultExit", close_screen);
    dom::on_click("olRestingExit", close_screen);
    dom::on_click("olUpdateExit", close_screen);
    dom::on_click("olEndedExit", close_screen);
    dom::on_click("olUpdateBtn", || {
        let _ = dom::window().open_with_url(APP_STORE_URL);
    });
    dom::on_click("olCreate", create_room);
    dom::on_click("olJoin", join_room);
    dom::on::<web_sys::Event, _>("olCode", "input", |_| clean_code_field());
    dom::on_click("olShare", || {
        let code = with_model(|m| m.room.as_ref().map(|r| r.code.clone()));
        if let Some(c) = code {
            crate::share::share_text(&om::share_text(&c));
        }
    });
    dom::on_click("olStart", || {
        net::send("start_match", json!({}));
    });
    dom::on_click("olRematch", || {
        net::send("start_match", json!({}));
    });
    dom::on_click("olLeave", leave_room);
    dom::on_click("olPendingCancel", leave_room);
    dom::on_click("olEndedBack", || {
        with_model(|m| m.ended = None);
        render();
    });
    dom::on_click("olOrb", tap_orb);
    dom::on_click("olReplay", replay_normal);
    dom::on_click("olSlow", || {
        if with_model(|m| m.word.is_some()) {
            net::send("replay_audio", json!({ "variant": "slow" }));
        }
    });
    dom::on_click("olDel", || {
        let lang = room_lang();
        with_model(|m| m.typed = crate::boardgame_input::backspace(&lang, &m.typed));
        render_letters();
    });
    dom::on_click("olCheck", submit);
    // Delegated: emotes and the host's knock list.
    dom::on::<web_sys::MouseEvent, _>("olEmotes", "click", |e| {
        if let Some(id) = attr_of(&e, "data-emote").and_then(|v| v.parse::<u8>().ok()) {
            net::send("emote", json!({ "id": id }));
        }
    });
    dom::on::<web_sys::MouseEvent, _>("olRequests", "click", |e| {
        if let Some(id) = attr_of(&e, "data-approve").and_then(|v| v.parse::<u64>().ok()) {
            net::send("approve_join", json!({ "request_id": id }));
            with_model(|m| m.requests.retain(|r| r.0 != id));
            render();
        } else if let Some(id) = attr_of(&e, "data-deny").and_then(|v| v.parse::<u64>().ok()) {
            net::send("deny_join", json!({ "request_id": id }));
            with_model(|m| m.requests.retain(|r| r.0 != id));
            render();
        }
    });
    let emotes: String = om::EMOTES
        .iter()
        .enumerate()
        .map(|(i, g)| format!("<button type=\"button\" class=\"ol-emote\" data-emote=\"{i}\">{g}</button>"))
        .collect();
    dom::set_html("olEmotes", &emotes);
}

fn attr_of(e: &web_sys::MouseEvent, name: &str) -> Option<String> {
    let t: web_sys::Element = e.target()?.dyn_into().ok()?;
    let hit = t.closest(&format!("[{name}]")).ok()??;
    hit.get_attribute(name)
}

/// While the screen is open it owns the keyboard: Escape leaves, Enter checks,
/// letters type. Captured on the window so the solo game's own key handler
/// never sees a key meant for the online word.
fn wire_keys_capture() {
    let f = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|e: web_sys::KeyboardEvent| {
        if !OPEN.with(Cell::get) {
            return;
        }
        let in_field = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA"));
        // Typing in the screen's own fields (name, code) goes to them.
        if in_field && e.key() != "Escape" {
            return;
        }
        e.stop_immediate_propagation();
        match e.key().as_str() {
            "Escape" => close_screen(),
            "Enter" if !in_field => submit(),
            "Backspace" if !in_field => dom::click("olDel"),
            k if !in_field && k.chars().count() == 1 && with_model(|m| m.can_answer()) => {
                let lang = room_lang();
                let key = crate::spellkb::Key::Unit(k.to_lowercase());
                with_model(|m| m.typed = crate::boardgame_input::press(&lang, &m.typed, &key));
                render_letters();
            }
            _ => {}
        }
    });
    let _ = dom::window().add_event_listener_with_callback_and_bool("keydown", f.as_ref().unchecked_ref(), true);
    f.forget();
}

/// F13 / A13.3: coming back to the app re-reads the kill file; a remote kill
/// while the screen is open shows the resting card.
fn wire_visibility() {
    let f = Closure::<dyn FnMut()>::new(|| {
        let visible = dom::doc().visibility_state() == web_sys::VisibilityState::Visible;
        if visible && OPEN.with(Cell::get) {
            spawn_local(async {
                crate::killflags::refresh().await;
                if !crate::flags::online_enabled() {
                    rest();
                }
            });
        }
    });
    let _ = dom::doc().add_event_listener_with_callback("visibilitychange", f.as_ref().unchecked_ref());
    f.forget();
}

// ---------------------------------------------------------------- opening and closing

fn open_screen() {
    OPEN.with(|o| o.set(true));
    FAILS.with(|f| f.set(0));
    DROPPED_AT.with(|d| d.set(None));
    with_model(Model::reset);
    fill_entry_form();
    dom::add_class("olScreen", "show");
    render();
    // The kill file is read on entering (F13), not only at launch. The socket
    // opens on Create or Join, so `hello` carries the name just typed.
    spawn_local(async {
        crate::killflags::refresh().await;
        if OPEN.with(Cell::get) && !crate::flags::online_enabled() {
            rest();
        }
    });
}

fn close_screen() {
    if with_model(|m| m.room.is_some() || m.pending_code.is_some()) {
        net::send("leave", json!({}));
    }
    OPEN.with(|o| o.set(false));
    net::stop();
    crate::online_audio::stop();
    with_model(Model::reset);
    dom::remove_class("olScreen", "show");
    dom::set_hidden("olReconnect", true);
}

/// The resting card: killed remotely, or unreachable (F13).
fn rest() {
    net::stop();
    with_model(|m| m.panel_override = Some(Panel::Resting));
    dom::set_hidden("olReconnect", true);
    render();
}

fn hello() -> net::Hello {
    net::Hello {
        kid: is_kid(),
        ui_lang: crate::i18n::current(),
        name: player_name(),
        resume: with_model(|m| m.resume()),
    }
}

fn connect() {
    net::open(hello());
}

fn player_name() -> String {
    let typed = dom::input("olName").value();
    let t = typed.trim();
    if !t.is_empty() {
        crate::storage::set_raw(NAME_KEY, t);
        return t.to_string();
    }
    crate::storage::get_raw(NAME_KEY).unwrap_or_else(|| crate::i18n::t("ph.vsName1"))
}

fn fill_entry_form() {
    let kid = is_kid();
    dom::set_hidden("olNameRow", kid);
    if dom::input("olName").value().is_empty() {
        dom::input("olName").set_value(&crate::storage::get_raw(NAME_KEY).unwrap_or_default());
    }
    // F12 + O2: every language local Head-to-Head offers.
    let current = APP.with(|a| a.borrow().as_ref().map(|a| a.borrow().lang.clone())).unwrap_or_default();
    let opts: String = crate::consts::BUILTIN_LANGS
        .iter()
        .filter(|row| crate::consts::is_active_lang(row.0))
        .map(|row| {
            let sel = if row.0 == current { " selected" } else { "" };
            format!("<option value=\"{}\"{sel}>{}</option>", row.0, dom::escape_html(row.1))
        })
        .collect();
    dom::set_html("olLang", &opts);
    // A kid cannot host (O3): the create half is not shown at all.
    dom::set_hidden("olCreateBox", kid);
    dom::set_text("olEntryErr", "");
    dom::set_text("olCodeHint", "");
}

// ---------------------------------------------------------------- entry actions

fn create_room() {
    let lang = dom::select("olLang").value();
    let fixed = dom::select("olTierMode").value() != "climb";
    let tier = dom::select("olTier").value();
    let mut body = json!({
        "mode": "h2h",
        "lang": lang,
        "tier_mode": if fixed { "fixed" } else { "climb" },
        "timed": dom::input("olTimed").checked(),
        "approve_joins": dom::input("olApprove").checked(),
    });
    if fixed {
        body["tier"] = json!(tier);
    }
    send_or_connect("create_room", body);
}

fn clean_code_field() {
    let raw = dom::input("olCode").value();
    match om::clean_code(&raw) {
        Ok(code) => {
            if code != raw {
                dom::input("olCode").set_value(&code);
            }
            dom::set_text("olCodeHint", "");
        }
        Err(bad) => {
            // Keep only what is valid, and say why the rest went.
            let kept: String = raw.chars().filter(|c| om::CODE_ALPHABET.contains(c.to_ascii_uppercase())).collect();
            dom::input("olCode").set_value(&om::clean_code(&kept).unwrap_or_default());
            dom::set_text("olCodeHint", &crate::i18n::tp("ol.codeHint", &[("c", &bad.to_string())]));
        }
    }
}

fn join_room() {
    let code = om::clean_code(&dom::input("olCode").value()).unwrap_or_default();
    if code.chars().count() != om::CODE_LEN {
        with_model(|m| m.entry_error = Some("bad_code".into()));
        render();
        return;
    }
    send_or_connect("join_room", json!({ "code": code }));
}

/// Sends now if the socket is up; otherwise opens it and sends once welcomed.
fn send_or_connect(t: &'static str, body: serde_json::Value) {
    with_model(|m| m.entry_error = None);
    if net::send(t, body.clone()) {
        render();
        return;
    }
    PENDING_SEND.with(|p| *p.borrow_mut() = Some((t, body)));
    with_model(|m| m.connecting = true);
    render();
    connect();
}

thread_local! {
    static PENDING_SEND: RefCell<Option<(&'static str, serde_json::Value)>> = const { RefCell::new(None) };
}

fn leave_room() {
    net::send("leave", json!({}));
    LEAVING.with(|l| l.set(true));
    with_model(|m| {
        let id = m.player_id.clone();
        m.reset();
        m.player_id = id;
    });
    crate::online_audio::stop();
    fill_entry_form();
    render();
}

// ---------------------------------------------------------------- match actions

fn room_lang() -> String {
    with_model(|m| m.room.as_ref().map(|r| r.lang.clone())).unwrap_or_else(|| "en".into())
}

fn tap_orb() {
    let (tap, live) = with_model(|m| (m.can_tap(), m.word.is_some()));
    if tap {
        net::send("next_word", json!({}));
    } else if live {
        replay_normal();
    }
}

/// The normal clip again: the same ticket, replayed locally (F9: "the same
/// ticket for the same variant").
fn replay_normal() {
    if let Some(t) = with_model(|m| m.word.as_ref().map(|w| w.ticket.clone())) {
        crate::online_audio::play(t, || {});
    }
}

fn submit() {
    let (ok, typed) = with_model(|m| (m.can_answer(), m.typed.trim().to_string()));
    if !ok || typed.is_empty() || SUBMITTED.with(Cell::get) {
        return;
    }
    if net::send("submit", json!({ "typed": typed })) {
        SUBMITTED.with(|s| s.set(true));
        render_match();
    }
}

fn press_key(k: &crate::spellkb::Key) {
    if !with_model(|m| m.can_answer()) || SUBMITTED.with(Cell::get) {
        return;
    }
    let lang = room_lang();
    with_model(|m| m.typed = crate::boardgame_input::press(&lang, &m.typed, k));
    crate::haptics::key_tap();
    render_letters();
}

fn ensure_keyboard() {
    let lang = room_lang();
    if KEYS_LANG.with(|k| *k.borrow() == lang) {
        return;
    }
    KEYS_LANG.with(|k| *k.borrow_mut() = lang.clone());
    use crate::spellkb::{Attr, Edges, Surface};
    let s = Surface { host: "olKeys", row_class: "ol-row", key_class: "ol-key", attr: Attr::Raw("data-k"), long_press: true, field: "olLetters" };
    crate::spellkb::mount(s, &crate::keyboard::shared_rows(&lang), Edges::default(), press_key);
}

// ---------------------------------------------------------------- the socket

fn on_net(e: net::Event) {
    if !OPEN.with(Cell::get) {
        return;
    }
    match e {
        net::Event::Open => {
            FAILS.with(|f| f.set(0));
            DROPPED_AT.with(|d| d.set(None));
            dom::set_hidden("olReconnect", true);
        }
        net::Event::Frame(text) => on_frame(&text),
        net::Event::Closed(code) => on_closed(code),
        net::Event::Unreachable => on_drop(),
    }
}

fn on_frame(text: &str) {
    let Some((seq, msg)) = om::decode(text) else { return };
    // Our own Leave already took us back to the entry.
    if LEAVING.with(Cell::get) {
        if matches!(&msg, om::ServerMsg::MatchEnded { reason } if reason == "left") {
            LEAVING.with(|l| l.set(false));
            return;
        }
    }
    let welcomed = matches!(msg, om::ServerMsg::Welcome { .. });
    let fx = with_model(|m| m.apply(seq, msg));
    for f in fx {
        match f {
            Effect::Play(t) => crate::online_audio::play(t, || {}),
            Effect::ClearAnswer => {
                SUBMITTED.with(|s| s.set(false));
                with_model(|m| m.typed.clear());
            }
            Effect::Haptic(true) => crate::haptics::correct(),
            Effect::Haptic(false) => crate::haptics::incorrect(is_kid()),
        }
    }
    if welcomed {
        if let Some((t, body)) = PENDING_SEND.with(|p| p.borrow_mut().take()) {
            net::send(t, body);
        }
    }
    render();
}

fn on_closed(code: u16) {
    match code {
        close::UPDATE_REQUIRED => {
            with_model(|m| m.panel_override = Some(Panel::Update));
            render();
        }
        close::SESSION => {
            // Expired or unknown token: mint a fresh one once, then reconnect.
            spawn_local(async {
                net::forget_token().await;
                connect();
            });
        }
        close::REVOKED => rest(),
        close::REPLACED => {}
        close::NAME => {
            with_model(|m| {
                m.connecting = false;
                m.entry_error = Some("name".into());
            });
            PENDING_SEND.with(|p| *p.borrow_mut() = None);
            render();
        }
        _ => on_drop(),
    }
}

/// F5 / F13: a lost connection. In a room, keep trying for the 60 s seat
/// hold behind the reconnect overlay. Outside one, two failed tries show the
/// resting card.
fn on_drop() {
    let in_room = with_model(|m| m.room.is_some());
    if in_room {
        let started = DROPPED_AT.with(|d| {
            if d.get().is_none() {
                d.set(Some(now_ms()));
            }
            d.get().unwrap()
        });
        if now_ms() - started >= net::RECONNECT_WINDOW_MS {
            DROPPED_AT.with(|d| d.set(None));
            dom::set_hidden("olReconnect", true);
            with_model(|m| {
                m.room = None;
                m.in_match = false;
                m.ended = Some("dropped".into());
            });
            render();
            return;
        }
        dom::set_hidden("olReconnect", false);
        tick_overlay();
        dom::after_ms(2000, || {
            if OPEN.with(Cell::get) && !net::is_open() {
                connect();
            }
        });
        return;
    }
    let fails = FAILS.with(|f| {
        f.set(f.get() + 1);
        f.get()
    });
    if fails >= net::UNREACHABLE_AFTER {
        return rest();
    }
    dom::after_ms(1500, || {
        if OPEN.with(Cell::get) && !net::is_open() {
            connect();
        }
    });
}

fn tick_overlay() {
    let Some(start) = DROPPED_AT.with(Cell::get) else { return };
    let left = ((net::RECONNECT_WINDOW_MS - (now_ms() - start)) / 1000.0).ceil().max(0.0) as u32;
    dom::set_text("olReconnectText", &crate::i18n::tp("ol.reconnecting", &[("s", &left.to_string())]));
    dom::after_ms(1000, tick_overlay);
}

// ---------------------------------------------------------------- drawing

fn render() {
    let panel = with_model(|m| m.panel());
    for p in om::PANELS.iter() {
        dom::set_hidden(om::panel_id(p), *p != panel);
    }
    render_drain();
    match panel {
        Panel::Entry => {
            let err = with_model(|m| m.entry_error.clone());
            let text = err
                .map(|e| if e == "name" { crate::i18n::t("ol.err.name") } else { crate::i18n::t(om::entry_error_key(&e)) })
                .unwrap_or_default();
            dom::set_text("olEntryErr", &text);
        }
        Panel::Lobby => render_lobby(),
        Panel::Match => render_match(),
        Panel::Result => render_result(),
        Panel::Ended => {
            let r = with_model(|m| m.ended.clone()).unwrap_or_default();
            let key = if r == "dropped" { "ol.end.dropped" } else { om::ended_key(&r) };
            dom::set_text("olEndedMsg", &crate::i18n::t(key));
        }
        _ => {}
    }
}

fn render_drain() {
    let until = with_model(|m| m.draining_until);
    dom::set_hidden("olDrain", until.is_none());
    if let Some(u) = until {
        let mins = (((u as f64) - now_ms()) / 60_000.0).ceil().max(1.0) as u32;
        dom::set_text("olDrain", &crate::i18n::tp("ol.draining", &[("m", &mins.to_string())]));
    }
}

fn render_lobby() {
    let (code, kid_safe, players, requests, host, n, note) = with_model(|m| {
        let r = m.room.as_ref();
        (
            r.map(|r| r.code.clone()).unwrap_or_default(),
            r.is_some_and(|r| r.kid_safe),
            om::players_html(m),
            om::requests_html(m),
            m.am_host(),
            r.map(|r| r.seats.len()).unwrap_or(0),
            m.ended.clone(),
        )
    });
    dom::set_text("olRoomCode", &code);
    dom::set_hidden("olKidBadge", !kid_safe);
    dom::set_html("olPlayers", &players);
    dom::set_html("olRequests", &requests);
    dom::set_hidden("olStart", !host);
    dom::set_disabled("olStart", n < 2);
    // A child does not share codes (share.rs refuses in Kid Mode anyway).
    dom::set_hidden("olShare", is_kid());
    let wait = if n < 2 {
        crate::i18n::t("ol.needTwo")
    } else if host {
        String::new()
    } else {
        crate::i18n::t("ol.waitHost")
    };
    let line = match note {
        Some(r) => format!("{} {}", crate::i18n::t(om::ended_key(&r)), wait),
        None => wait,
    };
    dom::set_text("olLobbyNote", line.trim());
}

fn render_match() {
    ensure_keyboard();
    let snap = with_model(|m| {
        let seats: Vec<u8> = m.room.as_ref().map(|r| r.seats.iter().map(|s| s.seat).collect()).unwrap_or_default();
        let bars: Vec<(String, u32, u32, bool)> = seats
            .iter()
            .map(|s| {
                let (cur, best) = m.score_of(*s);
                (m.name_of(*s), cur, best, m.turn.is_some_and(|t| t.0 == *s))
            })
            .collect();
        (
            bars,
            om::status_text(m),
            m.can_tap(),
            m.can_answer(),
            m.word.as_ref().and_then(|w| w.deadline_ms),
            m.word.as_ref().map(|w| w.word_no),
            m.emote.map(|(s, id)| (m.name_of(s), id)),
        )
    });
    let (bars, status, can_tap, can_answer, deadline, word_no, emote) = snap;
    for (i, (name, cur, best, active)) in bars.iter().enumerate().take(2) {
        let n = i + 1;
        dom::set_text(&format!("olP{n}Name"), name);
        dom::set_text(&format!("olP{n}Cur"), &cur.to_string());
        dom::set_text(&format!("olP{n}Best"), &best.to_string());
        dom::toggle_class(&format!("olP{n}"), "active", *active);
    }
    dom::set_text("olStatus", &status);
    let orb = if can_tap {
        crate::i18n::t("ol.tapOrb")
    } else if can_answer {
        crate::i18n::t("ol.listen")
    } else {
        String::new()
    };
    dom::set_text("olOrbGlyph", &orb);
    dom::toggle_class("olOrb", "ready", can_tap);
    dom::toggle_class("olOrb", "live", can_answer);
    let submitted = SUBMITTED.with(Cell::get);
    dom::set_disabled("olCheck", !can_answer || submitted);
    dom::set_disabled("olDel", !can_answer || submitted);
    dom::set_disabled("olReplay", word_no.is_none());
    dom::set_disabled("olSlow", word_no.is_none());
    dom::toggle_class("olKeys", "locked", !can_answer || submitted);
    // The countdown is display only; the server's clock decides (G-D17).
    match (deadline, word_no) {
        (Some(ms), Some(n)) => restart_timer(ms, n),
        _ => dom::set_hidden("olTimer", true),
    }
    if let Some((name, id)) = emote {
        let glyph = om::EMOTES.get(id as usize).copied().unwrap_or("");
        dom::set_text("olEmoteShow", &format!("{name} {glyph}"));
        dom::remove_class("olEmoteShow", "pop");
        dom::after_ms(16, || dom::add_class("olEmoteShow", "pop"));
        with_model(|m| m.emote = None);
    }
    render_letters();
}

thread_local! {
    static TIMER_FOR: Cell<u32> = const { Cell::new(0) };
}

fn restart_timer(ms: u64, word_no: u32) {
    dom::set_hidden("olTimer", false);
    if TIMER_FOR.with(Cell::get) == word_no {
        return;
    }
    TIMER_FOR.with(|t| t.set(word_no));
    let el: web_sys::HtmlElement = dom::el("olTimerFill").unchecked_into();
    let _ = el.style().set_property("animation-duration", &format!("{ms}ms"));
    dom::remove_class("olTimerFill", "run");
    dom::after_ms(16, || dom::add_class("olTimerFill", "run"));
}

fn render_letters() {
    let (typed, lang) = with_model(|m| (m.typed.clone(), m.room.as_ref().map(|r| r.lang.clone()).unwrap_or_default()));
    let shown = if lang == crate::consts::ZH { crate::pinyin::display_partial(&typed) } else { typed };
    if shown.is_empty() {
        dom::set_html("olLetters", &format!("<span class=\"placeholder\">{}</span>", dom::escape_html(&crate::i18n::t("ph.typeHeard"))));
        return;
    }
    let html: String = shown
        .chars()
        .map(|c| format!("<span class=\"ltr\">{}</span>", dom::escape_html(&c.to_string())))
        .collect();
    dom::set_html("olLetters", &html);
}

fn render_result() {
    let (title, rows, host) = with_model(|m| {
        let (t, r) = om::result_html(m);
        (t, r, m.am_host())
    });
    dom::set_text("olResultTitle", &title);
    dom::set_html("olResultRows", &rows);
    dom::set_hidden("olRematch", !host);
    dom::set_text("olResultNote", &if host { String::new() } else { crate::i18n::t("ol.waitRematch") });
}
