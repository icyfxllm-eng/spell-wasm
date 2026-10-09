//! CC-BOARD-GAME v1.2 F6/F7/F10 -- the screen.
//!
//! Renders `boardgame::BoardGameState` and forwards actions; it decides
//! nothing (spec section 0). Every rule lives in `src/boardgame/`. This file
//! owns what a reducer cannot: the clock (NPC pacing), the audio, the DOM, the
//! ledger write, and the hand-off between people on one phone.
//!
//! * D20: the key grid is built here from `boardgame_input` (the shared
//!   keyboard's own rows plus the units its long-press hides); it does not use
//!   the shared game keyboard or `surface_hooks`.
//! * D22 / I15: no telemetry from this mode. There is no `set_mode` call and no
//!   event; the podium's numbers are computed on the device and shown, nothing
//!   more.
//! * I11: NPC turns never show the key grid and never show or play a word. The
//!   engine draws none for them, and this file shows the spell panel only for a
//!   human seat in `AwaitSpelling`.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::boardgame::board::tile_to_grid;
use crate::boardgame::{self, Action, BoardGameState, Difficulty, Event, GameConfig, Phase, SpellKind, Variant};
use crate::boardgame_input::{self as input, Key};
use crate::boardgame_pools as pools;
use crate::wordsearch::ledger::Ledger;
use crate::{dom, haptics, i18n, speech_out, storage, App};

/// D23: pieces are icons, in seat/piece order.
const PIECES: [&str; 4] = ["\u{1f3a9}", "\u{1f41d}", "\u{1fa84}", "\u{1f4d6}"];
/// D10 / A10: one NPC turn. Three of them is 4.5 s, inside the 7 s budget; a tap
/// anywhere finishes the whole run at once.
pub const NPC_STEP_MS: i32 = 1500;
/// A beat after a human's own result, so it can be seen before the next actor.
const HUMAN_BEAT_MS: i32 = 800;
/// The chips kept on screen.
const MAX_CHIPS: usize = 6;

#[derive(Clone)]
struct Setup {
    solo: bool,
    count: u8,
    diff: Difficulty,
    piece: u8,
}

impl Setup {
    fn fresh() -> Self {
        Setup { solo: true, count: 3, diff: Difficulty::Normal, piece: 0 }
    }
}

struct Ui {
    game: BoardGameState,
    lang: String,
    kid: bool,
    relaxed: usize,
    day: u32,
    typed: String,
    keys: Vec<Vec<Key>>,
    /// More than one human on this phone (F7).
    pass: bool,
    /// The seat whose hand-off card has been dismissed.
    shown_seat: Option<u8>,
    /// A missed word waiting to be shown to the player who missed it (F7).
    miss: Option<String>,
    seen: usize,
    chips: Vec<String>,
    /// The line under the board: a solo player's missed word, or the switch prompt.
    feed: String,
    recorded: bool,
    /// What the last piece to land landed on (the callout over the board).
    land: Option<Land>,
}

#[derive(Clone, Copy)]
enum Land {
    Tier(boardgame::Tier),
    Trap(boardgame::Trap),
}

thread_local! {
    static SETUP: RefCell<Setup> = RefCell::new(Setup::fresh());
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LANG: RefCell<(String, bool)> = const { RefCell::new((String::new(), false)) };
    static TOKEN: Cell<u32> = const { Cell::new(0) };
    static NPC_RUN: Cell<bool> = const { Cell::new(false) };
    static GATE: Cell<bool> = const { Cell::new(true) };
    /// When the player last pressed a control of this screen. A tap that merely
    /// ended their turn (Submit, Ready, Continue) must not also skip the NPC run
    /// it started.
    static RUN_AT: Cell<f64> = const { Cell::new(0.0) };
}

fn piece(p: u8) -> &'static str {
    PIECES[p as usize % PIECES.len()]
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|u| u.borrow_mut().as_mut().map(f))
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn day() -> u32 {
    (now_ms() / 86_400_000.0) as u32
}

fn load_ledger() -> Ledger {
    storage::get_json(crate::wordsearch_ui::LEDGER_KEY).unwrap_or_default()
}

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("bgOpenBtn", move || open(&a));
    }
    dom::on_click("bgExit", close);
    dom::on_click("bgSetupExit", close);
    dom::on_click("bgStart", start);
    dom::on_click("bgOrb", orb);
    dom::on_click("bgDel", delete);
    dom::on_click("bgGo", submit);
    dom::on_click("bgHandGo", hand_go);
    dom::on_click("bgMissGo", miss_go);
    dom::on_click("bgAgain", again);
    dom::on_click("bgShare", share);
    // F6: the board re-fits whenever the space it has changes (rotation, window).
    dom::on_window::<web_sys::Event, _>("resize", |_| fit_view());
    dom::on::<web_sys::MouseEvent, _>("bgSetup", "click", setup_tap);
    dom::on::<web_sys::MouseEvent, _>("bgKeys", "click", key_tap);
    dom::on::<web_sys::MouseEvent, _>("bgSwap", "click", swap_tap);
    // A tap anywhere skips an NPC run to its end state (D10).
    dom::on::<web_sys::MouseEvent, _>("bgScreen", "click", |_| skip_npc());
}

pub fn open(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (if s.lang == crate::consts::MINE { crate::consts::EN.to_string() } else { s.lang.clone() }, s.kid)
    };
    LANG.with(|l| *l.borrow_mut() = (lang.clone(), kid));
    UI.with(|u| *u.borrow_mut() = None);
    SETUP.with(|s| {
        let mut s = s.borrow_mut();
        *s = Setup::fresh();
        if kid {
            s.diff = Difficulty::Normal;
        }
    });
    let dir = crate::consts::dir_attr(&lang);
    for id in ["bgField", "bgKeys"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", &lang);
            let _ = e.set_attribute("dir", dir);
        }
    }
    let ok = pools::gate(&lang, kid).is_ok();
    GATE.with(|g| g.set(ok));
    dom::set_text("bgNote", &if ok { String::new() } else { i18n::t("bg.soon") });
    dom::set_disabled("bgStart", !ok);
    dom::set_hidden("bgSetup", false);
    dom::set_hidden("bgPlay", true);
    dom::remove_class("bgScreen", "playing");
    for id in ["bgHand", "bgMiss", "bgOver"] {
        dom::set_hidden(id, true);
    }
    dom::set_html("bgChips", "");
    dom::remove_class("bgScreen", "spelling");
    render_setup();
    dom::add_class("bgScreen", "show");
}

fn close() {
    bump();
    speech_out::stop();
    finish_record();
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("bgScreen", "show");
}

fn bump() -> u32 {
    TOKEN.with(|t| {
        t.set(t.get().wrapping_add(1));
        t.get()
    })
}

// -------------------------------------------------------------------- setup

fn opt_btn(key: &str, on: bool, label: &str) -> String {
    format!("<button type=\"button\" class=\"bg-btn{}\" data-bg=\"{key}\">{label}</button>", if on { " on" } else { "" })
}

fn render_setup() {
    let s = SETUP.with(|s| s.borrow().clone());
    let (_, kid) = LANG.with(|l| l.borrow().clone());
    dom::set_html(
        "bgOptMode",
        &format!(
            "<span>{}</span>{}{}",
            i18n::t("bg.name"),
            opt_btn("mode:solo", s.solo, &i18n::t("bg.solo")),
            opt_btn("mode:pass", !s.solo, &i18n::t("bg.pass"))
        ),
    );
    let (label, range) = if s.solo { ("bg.rivals", 1..=3u8) } else { ("bg.players", 2..=4u8) };
    let mut h = format!("<span>{}</span>", i18n::t(label));
    for n in range {
        h.push_str(&opt_btn(&format!("count:{n}"), s.count == n, &n.to_string()));
    }
    dom::set_html("bgOptCount", &h);
    // F5: the difficulty setting is Standard only; Spell Jr has one fixed delta
    // and shows none.
    let mut d = String::new();
    if s.solo && !kid {
        for (id, key, who) in [("easy", "bg.diff.easy", Difficulty::Easy), ("normal", "bg.diff.normal", Difficulty::Normal), ("tough", "bg.diff.tough", Difficulty::Tough)] {
            d.push_str(&opt_btn(&format!("diff:{id}"), s.diff == who, &i18n::t(key)));
        }
    }
    dom::set_html("bgOptDiff", &d);
    let mut p = String::new();
    if s.solo {
        for i in 0..4u8 {
            p.push_str(&format!(
                "<button type=\"button\" class=\"bg-btn{}\" data-bg=\"piece:{i}\" aria-label=\"{} {}\">{}</button>",
                if s.piece == i { " on" } else { "" },
                piece(i),
                i + 1,
                piece(i)
            ));
        }
    }
    dom::set_html("bgOptPiece", &p);
}

fn setup_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-bg]").ok().flatten()) else {
        return;
    };
    let Some(v) = el.get_attribute("data-bg") else { return };
    let Some((k, val)) = v.split_once(':') else { return };
    SETUP.with(|s| {
        let mut s = s.borrow_mut();
        match k {
            "mode" => {
                s.solo = val == "solo";
                s.count = if s.solo { 3 } else { 2 };
            }
            "count" => s.count = val.parse().unwrap_or(s.count),
            "diff" => {
                s.diff = match val {
                    "easy" => Difficulty::Easy,
                    "tough" => Difficulty::Tough,
                    _ => Difficulty::Normal,
                }
            }
            "piece" => s.piece = val.parse().unwrap_or(0),
            _ => {}
        }
    });
    render_setup();
}

fn start() {
    RUN_AT.with(|r| r.set(now_ms()));
    if !GATE.with(Cell::get) {
        return;
    }
    let (lang, kid) = LANG.with(|l| l.borrow().clone());
    let s = SETUP.with(|s| s.borrow().clone());
    let seed = now_ms() as u64 ^ (js_sys::Math::random() * 4_294_967_296.0) as u64;
    let ledger = load_ledger();
    let d = day();
    let Ok(sup) = pools::supply(&lang, kid, &ledger, d, seed) else {
        dom::set_text("bgNote", &i18n::t("bg.soon"));
        return;
    };
    let variant = pools::variant_for(kid);
    let cfg = if s.solo {
        GameConfig::solo(variant, s.diff, s.count, s.piece, &lang, kid, crate::boardgame_grade::grade)
    } else {
        GameConfig::pass_and_play(variant, s.count, &lang, kid, crate::boardgame_grade::grade)
    };
    // The keys come from every word that can be served this game.
    let mut entries: Vec<String> = sup.pools.tiers.iter().flatten().cloned().collect();
    entries.extend(sup.pools.long_word.iter().cloned());
    let keys = input::layout(&lang, kid, &entries);
    let Ok(game) = boardgame::new_game(seed, cfg, sup.pools) else {
        dom::set_text("bgNote", &i18n::t("bg.soon"));
        return;
    };
    let pass = !s.solo;
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui { game, lang, kid, relaxed: sup.relaxed, day: d, typed: String::new(), keys, pass, shown_seat: None, miss: None, seen: 0, chips: Vec::new(), feed: String::new(), recorded: false, land: None })
    });
    render_keys();
    dom::set_hidden("bgSetup", true);
    dom::set_hidden("bgPlay", false);
    dom::add_class("bgScreen", "playing");
    dom::set_hidden("bgOver", true);
    advance();
}

fn again() {
    finish_record();
    UI.with(|u| *u.borrow_mut() = None);
    bump();
    dom::set_hidden("bgOver", true);
    dom::set_hidden("bgPlay", true);
    dom::remove_class("bgScreen", "playing");
    dom::set_hidden("bgSetup", false);
    dom::remove_class("bgScreen", "spelling");
    dom::set_html("bgChips", "");
    render_setup();
}

// -------------------------------------------------------------------- driver

/// Decide what the screen is waiting for and show it.
fn advance() {
    let Some((finished, npc, miss, hand)) = with_ui(|u| {
        let g = &u.game;
        let finished = g.phase == Phase::Finished;
        let cur = g.current_seat();
        let hand = u.pass && !finished && u.shown_seat != Some(cur);
        (finished, g.is_npc_turn(), u.miss.clone(), hand)
    }) else {
        return;
    };
    if finished {
        NPC_RUN.with(|n| n.set(false));
        render();
        show_over();
        return;
    }
    if let Some(word) = miss {
        render();
        // F7: the correct spelling, to the player who missed it, before the card.
        dom::set_text("bgMissText", &i18n::tp("bg.word", &[("word", crate::boardgame_grade::citation(&word))]));
        dom::set_hidden("bgMiss", false);
        return;
    }
    if npc {
        NPC_RUN.with(|n| n.set(true));
        render();
        npc_step();
        return;
    }
    NPC_RUN.with(|n| n.set(false));
    if hand {
        render();
        show_hand();
        return;
    }
    render();
}

fn miss_go() {
    RUN_AT.with(|r| r.set(now_ms()));
    dom::set_hidden("bgMiss", true);
    with_ui(|u| u.miss = None);
    advance();
}

fn show_hand() {
    let Some((seat, p)) = with_ui(|u| {
        let s = u.game.current_seat();
        (s, u.game.players[s as usize].seat.piece)
    }) else {
        return;
    };
    dom::set_text("bgHandIcon", piece(p));
    dom::set_text("bgHandText", &i18n::tp("bg.handoff", &[("n", &(seat + 1).to_string()), ("piece", piece(p))]));
    // Nothing the previous player typed may be visible behind the card.
    with_ui(|u| u.typed.clear());
    render();
    dom::set_hidden("bgHand", false);
}

fn hand_go() {
    RUN_AT.with(|r| r.set(now_ms()));
    dom::set_hidden("bgHand", true);
    with_ui(|u| {
        let s = u.game.current_seat();
        u.shown_seat = Some(s);
    });
    advance();
}

fn npc_step() {
    let token = bump();
    let done = with_ui(|u| {
        let _ = boardgame::apply(&mut u.game, Action::AdvanceNpc);
    });
    if done.is_none() {
        return;
    }
    after_apply();
    render();
    dom::after_ms(NPC_STEP_MS, move || {
        if TOKEN.with(Cell::get) == token {
            advance();
        }
    });
}

/// D10: a tap while NPCs are playing finishes the run now.
fn skip_npc() {
    // State, not a flag: a tap during the beat before the first NPC moves
    // skips the run too.
    let npc_now = with_ui(|u| u.game.is_npc_turn()).unwrap_or(false);
    if !npc_now || now_ms() - RUN_AT.with(Cell::get) < 250.0 {
        return;
    }
    bump();
    with_ui(|u| {
        let mut guard = 0;
        while u.game.is_npc_turn() && guard < 16 {
            let _ = boardgame::apply(&mut u.game, Action::AdvanceNpc);
            guard += 1;
        }
    });
    after_apply();
    advance();
}

/// Turn the engine's new events into result chips, and notice a miss.
fn after_apply() {
    with_ui(|u| {
        let new: Vec<Event> = u.game.events[u.seen..].to_vec();
        u.seen = u.game.events.len();
        for e in &new {
            match e {
                Event::Moved { to, .. } => u.land = u.game.board.tiers.get(*to as usize).copied().flatten().map(Land::Tier),
                Event::TrapHit { trap, .. } => u.land = Some(Land::Trap(*trap)),
                Event::Rolled { .. } | Event::Missed { .. } => u.land = None,
                _ => {}
            }
            if let Some(c) = chip_for(u, e) {
                u.chips.push(c);
            }
            // F7: a human's miss is shown to that human alone, before the hand-off.
            if let Event::Missed { seat, word: Some(w), .. } = e {
                if u.pass && !u.game.players[*seat as usize].seat.npc {
                    u.miss = Some(w.clone());
                }
            }
        }
        let n = u.chips.len();
        if n > MAX_CHIPS {
            u.chips.drain(0..n - MAX_CHIPS);
        }
    });
}

fn seat_piece(u: &Ui, seat: u8) -> &'static str {
    piece(u.game.players[seat as usize].seat.piece)
}

fn chip_for(u: &Ui, e: &Event) -> Option<String> {
    let p = |s: u8| seat_piece(u, s);
    Some(match e {
        Event::Rolled { seat, roll, .. } => format!("{} \u{1f3b2} {roll}", p(*seat)),
        Event::Moved { seat, from, to } => format!("{} \u{2713} {from}\u{2192}{to}", p(*seat)),
        Event::Missed { seat, at, .. } => format!("{} \u{2717} {at}", p(*seat)),
        Event::TrapHit { seat, trap, .. } => format!("{} \u{26a0} {}", p(*seat), i18n::t(&format!("bg.trap.{}", trap.key()))),
        Event::TrapSpellOk { seat, .. } => format!("{} \u{2713}", p(*seat)),
        Event::Teleported { seat, from, to } => format!("{} \u{21a9} {from}\u{2192}{to}", p(*seat)),
        Event::SkipSet { seat } | Event::Skipped { seat } => format!("{} \u{23ed}", p(*seat)),
        Event::Swapped { a, b } => format!("{} \u{21c4} {}", p(*a), p(*b)),
        Event::SwitchDeclined { .. } => return None,
        Event::Finished { seat } => format!("{} \u{1f3c1}", p(*seat)),
    })
}

// -------------------------------------------------------------------- render

fn render() {
    let _ = with_ui(|u| {
        // Chips are built here from the event tail so they carry piece icons.
        let g = &u.game;
        dom::set_html("bgChips", &hud(u));
        dom::set_html("bgLand", &land_pill(u));
        let spelling = g.phase == Phase::AwaitSpelling && !g.is_npc_turn();
        dom::toggle_class("bgScreen", "spelling", spelling);
        dom::set_html("bgBoard", &board_svg(u));
        dom::set_html("bgResults", &u.chips.iter().map(|c| format!("<span class=\"bg-res\">{}</span>", dom::escape_html(c))).collect::<String>());

        let human_turn = !g.is_npc_turn() && g.phase != Phase::Finished;
        // Orb: Roll, Replay, or waiting.
        let (state, face, aria) = if human_turn && g.phase == Phase::AwaitRoll {
            ("roll", i18n::t("bg.roll"), i18n::t("bg.roll"))
        } else if human_turn && g.phase == Phase::AwaitSpelling {
            ("replay", "\u{25b6}".to_string(), i18n::t("bg.replay"))
        } else {
            ("wait", "\u{2026}".to_string(), String::new())
        };
        let orb = dom::el("bgOrb");
        let _ = orb.set_attribute("data-state", state);
        let _ = orb.set_attribute("aria-label", &aria);
        if state == "wait" {
            let _ = orb.set_attribute("disabled", "");
        } else {
            let _ = orb.remove_attribute("disabled");
        }
        dom::set_text("bgOrb", &face);

        // I11: the key grid exists on screen for a human's spelling only.
        dom::set_hidden("bgSpell", !spelling);
        dom::set_hidden("bgSwap", !(human_turn && g.phase == Phase::AwaitSwitchTarget));
        if human_turn && g.phase == Phase::AwaitSwitchTarget {
            let me = g.current_seat();
            let mut h = String::new();
            for s in 0..g.players.len() as u8 {
                if s != me {
                    h.push_str(&format!(
                        "<button type=\"button\" class=\"bg-btn\" data-sw=\"{s}\" aria-label=\"{} {}\">{}</button>",
                        seat_piece(u, s),
                        s + 1,
                        seat_piece(u, s)
                    ));
                }
            }
            h.push_str(&format!("<button type=\"button\" class=\"bg-btn\" data-sw=\"none\">{}</button>", i18n::t("bg.nobody")));
            dom::set_html("bgSwap", &h);
            dom::set_text("bgFeed", &i18n::t("bg.pick"));
        } else {
            dom::set_text("bgFeed", &u.feed);
        }
        dom::set_text("bgField", &u.typed);
        // D16: a soft 30 s ring in Standard, never an auto-fail.
        let timed = spelling && g.cfg.variant == Variant::Standard;
        dom::remove_class("bgTimer", "run");
        dom::set_hidden("bgTimer", !timed);
    });
    fit_view();
}

/// The callout over the board. A human about to spell sees the destination's tier before
/// typing (never the word); otherwise the last landing. Trap names are shown only once a
/// trap has been triggered (F4: hidden until then).
fn land_pill(u: &Ui) -> String {
    let spelling_dest = if u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn() {
        u.game.pending.as_ref().filter(|p| p.kind == SpellKind::Landing).and_then(|p| u.game.board.tiers.get(p.dest as usize).copied().flatten())
    } else {
        None
    };
    let land = spelling_dest.map(Land::Tier).or(u.land);
    match land {
        Some(Land::Tier(t)) => format!("<span class=\"bg-pill\"><i class=\"bg-sw t-{0}\"></i>{1}</span>", t.name(), dom::escape_html(&i18n::t(&format!("level.{}", t.name())))),
        Some(Land::Trap(t)) => format!("<span class=\"bg-pill\" data-land=\"trap\"><i class=\"bg-sw trap\">\u{26a0}</i>{}</span>", dom::escape_html(&i18n::t(&format!("bg.trap.{}", t.key())))),
        None => String::new(),
    }
}

fn hud(u: &Ui) -> String {
    let cur = u.game.current_seat();
    let mut h = String::new();
    for (i, p) in u.game.players.iter().enumerate() {
        let on = i as u8 == cur && u.game.phase != Phase::Finished;
        h.push_str(&format!(
            "<span class=\"bg-chip{}\" aria-label=\"{} {}\"><i style=\"font-style:normal\">{}</i><b>{}</b></span>",
            if on { " on" } else { "" },
            piece(p.seat.piece),
            i + 1,
            piece(p.seat.piece),
            p.pos
        ));
    }
    h
}

/// The smallest cell, in CSS px, at which the whole ring is worth showing in the
/// spell state. Below it the board falls back to the compact strip around the
/// destination tile (F6). The decision is made on the measured space, never on
/// a device name.
const MIN_FULL_CELL_PX: f64 = 12.0;

/// One layout function for both states: give the board the height that is left,
/// then show the whole ring if it fits at a legible size, else the unrolled track
/// (the next tiles along the path, in one row, full panel width).
fn fit_view() {
    let Some((g, spelling)) = with_ui(|u| (u.game.board.grid as f64, u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn())) else {
        return;
    };
    let view = dom::el("bgView");
    let board = dom::el("bgBoard");
    // Measure with the strip class off: the view then takes all the leftover height.
    let was_strip = view.class_list().contains("strip");
    let _ = view.class_list().remove_1("strip");
    let (w, h) = (view.client_width() as f64, view.client_height() as f64);
    if w <= 0.0 || h <= 0.0 {
        return; // screen not shown
    }
    let full = !spelling || w.min(h) / g >= MIN_FULL_CELL_PX;
    if full {
        if was_strip {
            let _ = with_ui(|u| dom::set_html("bgBoard", &board_svg(u)));
        }
        let _ = board.remove_attribute("class");
        let _ = board.set_attribute("viewBox", &format!("0 0 {g} {g}"));
        let _ = board.set_attribute("preserveAspectRatio", "xMidYMid meet");
        return;
    }
    let _ = view.class_list().add_1("strip");
    let _ = board.set_attribute("class", "tr");
    // The svg is the leftover panel under the pill; its own size sets the track's proportions.
    let (sw, sh) = (board.client_width() as f64, board.client_height() as f64);
    let (sw, sh) = (if sw > 0.0 { sw } else { w }, if sh > 0.0 { sh } else { h });
    let (n, vh) = track_dims(sw, sh);
    let _ = with_ui(|u| dom::set_html("bgBoard", &track_svg(u, n, vh)));
    // A little air each side, so the mover's ring is not clipped at the end tile.
    let _ = board.set_attribute("viewBox", &format!("{} 0 {} {vh}", -TRACK_PAD, n as f64 + 2.0 * TRACK_PAD));
    let _ = board.set_attribute("preserveAspectRatio", "xMidYMid meet");
}

const TRACK_PAD: f64 = 0.2;

/// How many tiles the unrolled track shows and the viewBox height, from the
/// measured panel only: about 32 css px a tile, 8 to 12 of them. The viewBox
/// has the panel's own aspect ratio, so the track always spans the full width.
fn track_dims(w: f64, h: f64) -> (u32, f64) {
    let n = ((w / 32.0).floor() as u32).clamp(8, 12);
    (n, ((n as f64 + 2.0 * TRACK_PAD) * h / w).max(0.5))
}

/// The unrolled track: tiles `pos .. pos + n` of the ring (wrapping at the
/// finish), one row. The path runs the way the language reads: left to right,
/// and right to left for Arabic, so the mover's tile is where reading begins.
fn track_svg(u: &Ui, n: u32, vh: f64) -> String {
    let b = &u.game.board;
    let len = b.len() as u32;
    let n = n.min(len);
    let rtl = crate::consts::dir_attr(&u.lang) == "rtl";
    let slot = |k: u32| if rtl { n - 1 - k } else { k } as f64;
    let start = u.game.players[u.game.current_seat() as usize].pos;
    let at = |tile: u32| -> Option<u32> {
        let k = (tile + len - start % len) % len;
        (k < n).then_some(k)
    };
    let th = (vh * 0.9).min(0.9);
    let y0 = (vh - th) / 2.0;
    let dest = u.game.pending.as_ref().filter(|p| p.kind == SpellKind::Landing).map(|p| p.dest);
    let mut s = String::new();
    for k in 0..n {
        let i = (start + k) % len;
        let class = match b.tiers[i as usize] {
            Some(t) => format!("bg-t t-{}", t.name()),
            None => "bg-t end".to_string(),
        };
        let dcl = if dest == Some(i) { " dest" } else { "" };
        s.push_str(&format!("<rect class=\"{class}{dcl}\" x=\"{}\" y=\"{y0}\" width=\"0.9\" height=\"{th}\" rx=\"0.16\" data-slot=\"{}\"/>", slot(k) + 0.05, slot(k)));
    }
    let cy = vh / 2.0;
    let fs = th * 0.8;
    for &t in &u.game.revealed {
        if let Some(k) = at(t) {
            s.push_str(&format!("<text class=\"bg-trap\" style=\"font-size:{}px\" x=\"{}\" y=\"{cy}\">\u{26a0}</text>", th * 0.62, slot(k) + 0.5));
        }
    }
    if u.game.phase != Phase::Finished {
        s.push_str(&format!("<circle class=\"bg-me\" cx=\"{}\" cy=\"{cy}\" r=\"{}\"/>", slot(0) + 0.5, (th * 0.62).min(vh / 2.0 - 0.04).max(0.1)));
    }
    const OFF: [(f64, f64); 4] = [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)];
    for (i, p) in u.game.players.iter().enumerate() {
        let Some(k) = at(p.pos) else { continue };
        let shared = u.game.players.iter().enumerate().any(|(j, q)| j != i && q.pos == p.pos);
        let (dx, dy) = if shared { OFF[i % 4] } else { (0.0, 0.0) };
        s.push_str(&format!("<text class=\"bg-p\" style=\"font-size:{fs}px\" x=\"{}\" y=\"{}\">{}</text>", slot(k) + 0.5 + dx, cy + dy * th, piece(p.seat.piece)));
    }
    s
}

fn board_svg(u: &Ui) -> String {
    let b = &u.game.board;
    let g = b.grid;
    let mut s = String::new();
    let dest = u.game.pending.as_ref().filter(|p| p.kind == SpellKind::Landing).map(|p| p.dest);
    for i in 0..b.len() as u32 {
        let (x, y) = tile_to_grid(i, g);
        let class = match b.tiers[i as usize] {
            Some(t) => format!("bg-t t-{}", t.name()),
            None => "bg-t end".to_string(),
        };
        let dcl = if dest == Some(i) { " dest" } else { "" };
        s.push_str(&format!("<rect class=\"{class}{dcl}\" x=\"{}\" y=\"{}\" width=\"0.9\" height=\"0.9\" rx=\"0.16\"/>", x as f64 + 0.05, y as f64 + 0.05));
    }
    // Revealed traps stay on the map for the rest of the game (F4).
    for &t in &u.game.revealed {
        let (x, y) = tile_to_grid(t, g);
        s.push_str(&format!("<text class=\"bg-trap\" x=\"{}\" y=\"{}\">\u{26a0}</text>", x as f64 + 0.5, y as f64 + 0.5));
    }
    // The mover's piece sits on a ring so it is findable on a full-size board.
    if u.game.phase != Phase::Finished {
        let (x, y) = tile_to_grid(u.game.players[u.game.current_seat() as usize].pos, g);
        s.push_str(&format!("<circle class=\"bg-me\" cx=\"{}\" cy=\"{}\" r=\"0.62\"/>", x as f64 + 0.5, y as f64 + 0.5));
    }
    const OFF: [(f64, f64); 4] = [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)];
    for (i, p) in u.game.players.iter().enumerate() {
        let (x, y) = tile_to_grid(p.pos, g);
        let shared = u.game.players.iter().enumerate().any(|(j, q)| j != i && q.pos == p.pos);
        let (dx, dy) = if shared { OFF[i % 4] } else { (0.0, 0.0) };
        s.push_str(&format!("<text class=\"bg-p\" x=\"{}\" y=\"{}\">{}</text>", x as f64 + 0.5 + dx, y as f64 + 0.5 + dy, piece(p.seat.piece)));
    }
    s
}

fn render_keys() {
    let html: String = with_ui(|u| {
        u.keys
            .iter()
            .map(|row| {
                let keys: String = row
                    .iter()
                    .map(|k| format!("<button type=\"button\" class=\"bg-key{}\" data-k=\"{}\">{}</button>", if matches!(k, Key::Mod(_)) { " mod" } else { "" }, dom::escape_html(&k.data()), dom::escape_html(&k.face())))
                    .collect();
                format!("<div class=\"bg-row\">{keys}</div>")
            })
            .collect()
    })
    .unwrap_or_default();
    dom::set_html("bgKeys", &html);
}

// -------------------------------------------------------------------- play

fn speak(entry: &str, lang: &str) {
    // The same audio routes Bee uses: the resolver, with the device voice as the
    // last resort, and Mandarin always through the forced pinyin reading.
    if lang == crate::consts::ZH {
        let (pinyin, hanzi) = entry.split_once('|').unwrap_or((entry, entry));
        if let Some(py) = crate::pinyin::phoneme_reading(pinyin) {
            crate::api::play_word_with(hanzi, Some(&py), "normal", 1.0, lang, || {});
        }
        return;
    }
    let word = entry.split('|').next().unwrap_or(entry).to_string();
    let (fallback, code) = (word.clone(), lang.to_string());
    crate::api::play_word(&word, "normal", 1.0, lang, move || {
        // zh-ok(audio): the zh branch above returns through play_word_with with a forced
        // reading, so this is unreachable for Mandarin
        speech_out::speak(&fallback, 1.0, &code)
    });
}

fn pending_word() -> Option<(String, String)> {
    with_ui(|u| u.game.pending.as_ref().map(|p| (p.word.clone(), u.lang.clone()))).flatten()
}

fn orb() {
    RUN_AT.with(|r| r.set(now_ms()));
    let Some((phase, npc)) = with_ui(|u| (u.game.phase, u.game.is_npc_turn())) else { return };
    if npc {
        return;
    }
    match phase {
        Phase::AwaitRoll => {
            with_ui(|u| {
                u.chips.clear();
                u.feed.clear();
            });
            let ok = with_ui(|u| boardgame::apply(&mut u.game, Action::Roll).is_ok()).unwrap_or(false);
            if !ok {
                return;
            }
            after_apply();
            haptics::key_tap();
            after_roll();
        }
        Phase::AwaitSpelling => {
            if let Some((w, l)) = pending_word() {
                speak(&w, &l);
            }
        }
        _ => {}
    }
}

/// A roll either finished the game or owes a word.
fn after_roll() {
    render();
    let finished = with_ui(|u| u.game.phase == Phase::Finished).unwrap_or(false);
    if finished {
        advance();
        return;
    }
    start_spelling();
}

fn start_spelling() {
    with_ui(|u| u.typed.clear());
    render();
    restart_timer();
    if let Some((w, l)) = pending_word() {
        speak(&w, &l);
    }
}

fn restart_timer() {
    let std = with_ui(|u| u.game.cfg.variant == Variant::Standard).unwrap_or(false);
    if std {
        dom::after_ms(30, || dom::add_class("bgTimer", "run"));
    }
}

fn key_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-k]").ok().flatten()) else {
        return;
    };
    let Some(k) = el.get_attribute("data-k").and_then(|d| Key::parse(&d)) else { return };
    with_ui(|u| {
        if u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn() {
            u.typed = input::press(&u.lang, &u.typed, &k);
        }
    });
    haptics::key_tap();
    let t = with_ui(|u| u.typed.clone()).unwrap_or_default();
    dom::set_text("bgField", &t);
}

fn delete() {
    with_ui(|u| {
        if u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn() {
            u.typed = input::backspace(&u.lang, &u.typed);
        }
    });
    haptics::key_tap();
    let t = with_ui(|u| u.typed.clone()).unwrap_or_default();
    dom::set_text("bgField", &t);
}

fn submit() {
    RUN_AT.with(|r| r.set(now_ms()));
    let Some(typed) = with_ui(|u| u.typed.clone()) else { return };
    if typed.is_empty() {
        return;
    }
    let Some((before_seat, kid)) = with_ui(|u| (u.game.current_seat(), u.kid)) else {
        return;
    };
    let ok = with_ui(|u| boardgame::apply(&mut u.game, Action::SubmitSpelling(typed)).is_ok()).unwrap_or(false);
    if !ok {
        return;
    }
    with_ui(|u| u.typed.clear());
    after_apply();
    // Feedback: a hit or a miss, from the events this submit produced.
    let (missed_word, solo) = with_ui(|u| {
        let m = u.game.events.iter().rev().take(4).find_map(|e| match e {
            Event::Missed { seat, word: Some(w), .. } if *seat == before_seat => Some(w.clone()),
            _ => None,
        });
        (m, !u.pass)
    })
    .unwrap_or((None, true));
    if missed_word.is_some() {
        haptics::incorrect(kid);
        if solo {
            let w = missed_word.unwrap_or_default();
            let line = i18n::tp("bg.word", &[("word", crate::boardgame_grade::citation(&w))]);
            with_ui(|u| u.feed = line);
        }
    } else {
        haptics::correct();
    }
    // Still the same human and a new word owed (a trap's spelling): keep going.
    let spelling_again = with_ui(|u| u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn() && u.game.current_seat() == before_seat).unwrap_or(false);
    if spelling_again {
        start_spelling();
        return;
    }
    render();
    let token = bump();
    dom::after_ms(HUMAN_BEAT_MS, move || {
        if TOKEN.with(Cell::get) == token {
            advance();
        }
    });
}

fn swap_tap(e: web_sys::MouseEvent) {
    RUN_AT.with(|r| r.set(now_ms()));
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-sw]").ok().flatten()) else {
        return;
    };
    let target = match el.get_attribute("data-sw").as_deref() {
        Some("none") => None,
        Some(n) => n.parse::<u8>().ok(),
        None => return,
    };
    let ok = with_ui(|u| boardgame::apply(&mut u.game, Action::ChooseSwitchTarget(target)).is_ok()).unwrap_or(false);
    if !ok {
        return;
    }
    after_apply();
    render();
    let token = bump();
    dom::after_ms(HUMAN_BEAT_MS, move || {
        if TOKEN.with(Cell::get) == token {
            advance();
        }
    });
}

// -------------------------------------------------------------------- result

/// D19: the ledger takes the words that were actually drawn, once, for a
/// finished or an abandoned game.
fn finish_record() {
    let Some(()) = with_ui(|u| {
        if u.recorded {
            return;
        }
        u.recorded = true;
        let mut led = load_ledger();
        pools::record(&mut led, &u.lang, &u.game, u.day, u.relaxed);
        storage::set_json(crate::wordsearch_ui::LEDGER_KEY, &led);
    }) else {
        return;
    };
}

fn show_over() {
    finish_record();
    let rows = with_ui(|u| {
        let g = &u.game;
        let mut h = String::new();
        for (rank, seat) in g.standings().into_iter().enumerate() {
            let p = &g.players[seat as usize];
            let medal = ["\u{1f947}", "\u{1f948}", "\u{1f949}"].get(rank).copied().unwrap_or("");
            let mut tail = i18n::tp("bg.turns", &[("n", &p.rolls.to_string())]);
            if !p.seat.npc && p.attempts > 0 {
                tail = format!(
                    "{} · {} · {}",
                    i18n::tp("bg.words", &[("n", &p.hits.to_string())]),
                    i18n::tp("bg.acc", &[("n", &(p.hits * 100 / p.attempts).to_string())]),
                    tail
                );
            }
            h.push_str(&format!(
                "<div class=\"bg-pod\" aria-label=\"{} {}\"><span>{}</span><span>{}</span><span>{}</span><b>{}</b></div>",
                piece(p.seat.piece),
                seat + 1,
                medal,
                piece(p.seat.piece),
                p.pos,
                dom::escape_html(&tail)
            ));
        }
        h
    })
    .unwrap_or_default();
    dom::set_html("bgPodium", &rows);
    dom::set_hidden("bgShare", !crate::share::available());
    dom::set_hidden("bgOver", false);
}

fn share() {
    let place = with_ui(|u| {
        let order = u.game.standings();
        order.iter().position(|&s| !u.game.players[s as usize].seat.npc).map(|p| p + 1).unwrap_or(1)
    })
    .unwrap_or(1);
    crate::share::share_text(&i18n::tp("bg.shareText", &[("n", &place.to_string())]));
}

// ---------------------------------------------------------------------- seam

/// Test builds only: where the game stands, including the word a human owes
/// (it is heard, never shown, so a browser test cannot read it off the screen).
#[cfg(feature = "testseam")]
pub fn seam_state() -> String {
    UI.with(|u| {
        u.borrow()
            .as_ref()
            .map(|u| {
                let g = &u.game;
                serde_json::json!({
                    "phase": format!("{:?}", g.phase),
                    "seat": g.current_seat(),
                    "npc": g.is_npc_turn(),
                    "word": g.pending.as_ref().map(|p| p.word.clone()),
                    "tier": g.pending.as_ref().map(|p| p.tier.name()),
                    "kind": g.pending.as_ref().map(|p| format!("{:?}", p.kind)),
                    "pos": g.players.iter().map(|p| p.pos).collect::<Vec<_>>(),
                    "winner": g.winner,
                    "hand": u.pass && g.phase != Phase::Finished && u.shown_seat != Some(g.current_seat()),
                    "variant": format!("{:?}", g.cfg.variant),
                    "traps": g.board.trap_count(),
                })
                .to_string()
            })
            .unwrap_or_default()
    })
}


#[cfg(test)]
mod tests {
    /// I15 / D22: this mode adds no telemetry. No event, no `Mode` variant, no
    /// `set_mode` call, and nothing from the mode reaches the telemetry module.
    #[test]
    fn i15_the_mode_calls_no_telemetry() {
        for (name, src) in [
            ("boardgame_screen.rs", include_str!("boardgame_screen.rs")),
            ("boardgame_pools.rs", include_str!("boardgame_pools.rs")),
            ("boardgame_input.rs", include_str!("boardgame_input.rs")),
            ("boardgame_grade.rs", include_str!("boardgame_grade.rs")),
        ] {
            let code: String = src.split("#[cfg(test)]").next().unwrap().lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
            for banned in ["telemetry::", "set_mode", "schema::Mode"] {
                assert!(!code.contains(banned), "{name} uses {banned}");
            }
        }
        let schema = include_str!("telemetry/schema.rs");
        assert!(!schema.to_lowercase().contains("boardgame"), "the telemetry Mode enum grew a Board Game variant");
    }
}
