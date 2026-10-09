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
use std::collections::VecDeque;

use wasm_bindgen::JsCast;

use crate::boardgame::{self, Action, BoardGameState, Difficulty, Event, GameConfig, Phase, SpellKind};
use crate::boardgame_input::{self as input, Key};
use crate::boardgame_ring as ring;
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
    /// D-P4 / D-P15: Sprint (42 tiles) or Full (84). Solo opens on Sprint; pass-and-play
    /// opens on Full and remembers the last choice. Spell Jr has no picker and ignores it.
    sprint: bool,
}

/// D-P15: the last size picked in pass-and-play ("sprint" or "full"). Solo is not
/// remembered: it always opens on Sprint. `bg_` keeps it inside the wall scan's reach.
const PASS_SIZE_KEY: &str = "bg_size_pass_v1";

fn pass_sprint() -> bool {
    storage::get_raw(PASS_SIZE_KEY).as_deref() == Some("sprint")
}

impl Setup {
    fn fresh() -> Self {
        Setup { solo: true, count: 3, diff: Difficulty::Normal, piece: 0, sprint: true }
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
    /// Polish Phase A: the tile each piece is DRAWN on. It trails the engine's
    /// position while a move is being walked, tile by tile (Feature 6).
    shown: Vec<u32>,
    /// Which game this is, so the persistent piece elements are rebuilt for a new one.
    gid: u64,
    /// Feature 5: the seat the centre stage is about, the last roll (die, destination)
    /// and a counter that changes with every roll so the die tumbles once per roll.
    act: u8,
    roll: Option<(u8, u32)>,
    roll_id: u32,
    roll_at: f64,
    /// Tiles drawn as revealed traps although the game has not revealed them. Only the
    /// dev-build test seam writes this (it lets a layout test see a revealed mark without
    /// playing to a trap); the game never reads or changes it.
    marks: Vec<u32>,
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
    /// Feature 6: the walk in progress (steps left and the time one hop takes).
    static ANIM: RefCell<Option<Anim>> = const { RefCell::new(None) };
    static ANIM_TOKEN: Cell<u32> = const { Cell::new(0) };
    /// The ring's measured geometry, or None while the unrolled track is showing.
    static GEOM: Cell<Option<Geom>> = const { Cell::new(None) };
    static HOP_MS: Cell<i32> = const { Cell::new(0) };
}

/// One drawn step of a move.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    /// Onto the next tile along the path, with a hop and a tick.
    Hop(u8, u32),
    /// Straight to a tile (a trap's teleport, a swap, or any move under Reduce Motion).
    Jump(u8, u32),
}

struct Anim {
    steps: VecDeque<Step>,
    hop_ms: i32,
}

/// Where the ring sits inside `#bgView`, in CSS px.
#[derive(Clone, Copy)]
struct Geom {
    ox: f64,
    oy: f64,
    p: f64,
    w: u32,
    h: u32,
    vw: f64,
    vh: f64,
}

/// Feature 6 timing. A human's own piece walks at most `HUMAN_HOP_MS` a tile and the whole
/// walk never exceeds `HUMAN_WALK_CAP_MS` (6 tiles 0.96 s, 12 tiles 1.7 s: spec <= 1.2 / 2.0 s).
/// An NPC's walk lives INSIDE its `NPC_STEP_MS` step (D10's 7 s budget), never added to it.
const HUMAN_HOP_MS: i32 = 160;
const HUMAN_WALK_CAP_MS: i32 = 1700;
const NPC_WALK_CAP_MS: i32 = 1300;
/// A glide or fade for a teleport / swap / Reduce Motion move.
const JUMP_MS: i32 = 350;

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
    observe_view();
    dom::on::<web_sys::MouseEvent, _>("bgSetup", "click", setup_tap);
    dom::on::<web_sys::MouseEvent, _>("bgKeys", "click", key_tap);
    dom::on::<web_sys::MouseEvent, _>("bgSwap", "click", swap_tap);
    // A tap anywhere skips an NPC run to its end state (D10).
    dom::on::<web_sys::MouseEvent, _>("bgScreen", "click", |_| skip_npc());
}

/// The pieces and the stage are placed in pixels from the measured board, so the board is
/// re-fitted whenever `#bgView` itself changes size (not only on a window resize: the
/// insets, the key rows and a font loading all move it). Uses the browser's ResizeObserver
/// through `Reflect`, so it adds no web-sys feature.
fn observe_view() {
    use js_sys::{Array, Function, Reflect};
    use wasm_bindgen::closure::Closure;
    let win = dom::window();
    let Some(ctor) = Reflect::get(&win, &"ResizeObserver".into()).ok().and_then(|c| c.dyn_into::<Function>().ok()) else {
        return;
    };
    let cb = Closure::<dyn FnMut()>::new(|| {
        let want = dom::doc().get_element_by_id("bgView").map(|v| (v.client_width() as f64, v.client_height() as f64));
        let have = GEOM.with(Cell::get).map(|g| (g.vw, g.vh));
        if want.is_some() && want != have {
            dom::after_ms(0, fit_view);
        }
    });
    let Ok(obs) = Reflect::construct(&ctor, &Array::of1(cb.as_ref())) else { return };
    cb.forget();
    let Some(observe) = Reflect::get(&obs, &"observe".into()).ok().and_then(|f| f.dyn_into::<Function>().ok()) else {
        return;
    };
    let _ = observe.call1(&obs, &dom::el("bgView"));
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
    refresh_gate();
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

/// F9 / I8 for the variant the setup screen is about to start. Sprint needs half of
/// Full's unique words per tier, so a language can open Sprint before it opens Full.
fn refresh_gate() {
    let (lang, kid) = LANG.with(|l| l.borrow().clone());
    let sprint = SETUP.with(|s| s.borrow().sprint);
    let ok = pools::gate(&lang, pools::variant_for(kid, sprint)).is_ok();
    GATE.with(|g| g.set(ok));
    dom::set_text("bgNote", &if ok { String::new() } else { i18n::t("bg.soon") });
    dom::set_disabled("bgStart", !ok);
}

fn close() {
    bump();
    cancel_anim();
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
    // D-P4: the size row. Spell Jr has one fixed size and shows no picker.
    let mut z = String::new();
    if !kid {
        z.push_str(&format!("<span>{}</span>", i18n::t("bg.size")));
        z.push_str(&opt_btn("size:sprint", s.sprint, &i18n::t("bg.sprint")));
        z.push_str(&opt_btn("size:full", !s.sprint, &i18n::t("bg.full")));
    }
    dom::set_html("bgOptSize", &z);
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
                s.sprint = s.solo || pass_sprint();
            }
            "size" => {
                s.sprint = val == "sprint";
                if !s.solo {
                    storage::set_raw(PASS_SIZE_KEY, if s.sprint { "sprint" } else { "full" });
                }
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
    refresh_gate();
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
    let variant = pools::variant_for(kid, s.sprint);
    let Ok(sup) = pools::supply(&lang, variant, &ledger, d, seed) else {
        dom::set_text("bgNote", &i18n::t("bg.soon"));
        return;
    };
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
    let shown: Vec<u32> = game.players.iter().map(|p| p.pos).collect();
    let act = game.current_seat();
    cancel_anim();
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui {
            game,
            lang,
            kid,
            relaxed: sup.relaxed,
            day: d,
            typed: String::new(),
            keys,
            pass,
            shown_seat: None,
            miss: None,
            seen: 0,
            chips: Vec::new(),
            feed: String::new(),
            recorded: false,
            land: None,
            shown,
            gid: seed,
            act,
            roll: None,
            roll_id: 0,
            roll_at: 0.0,
            marks: Vec::new(),
        })
    });
    render_keys();
    dom::set_hidden("bgSetup", true);
    dom::set_hidden("bgPlay", false);
    dom::add_class("bgScreen", "playing");
    dom::set_hidden("bgOver", true);
    advance();
}

fn again() {
    cancel_anim();
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
    // Feature 5: a new human turn puts the stage on that seat, with no roll yet.
    with_ui(|u| {
        if u.game.phase == Phase::AwaitRoll {
            u.act = u.game.current_seat();
            u.roll = None;
            u.land = None;
        }
    });
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
    // The walk lives inside the step (D10): the step is not lengthened for it.
    after_apply(true);
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
    // Cancel any hop in progress: every piece goes to its final tile at once.
    cancel_anim();
    with_ui(|u| {
        let mut guard = 0;
        while u.game.is_npc_turn() && guard < 16 {
            let _ = boardgame::apply(&mut u.game, Action::AdvanceNpc);
            guard += 1;
        }
    });
    after_apply(false);
    advance();
}

/// Turn the engine's new events into result chips, notice a miss, and (when `animate`) start
/// walking the pieces that moved. Returns how long that walk takes, in ms.
fn after_apply(animate: bool) -> i32 {
    let segs = with_ui(|u| {
        let new: Vec<Event> = u.game.events[u.seen..].to_vec();
        u.seen = u.game.events.len();
        let mut segs: Vec<Seg> = Vec::new();
        for e in &new {
            match e {
                Event::Moved { seat, from, to } => {
                    u.land = u.game.board.tiers.get(*to as usize).copied().flatten().map(Land::Tier);
                    segs.push(Seg::Walk(*seat, *from, *to));
                }
                Event::TrapHit { trap, .. } => u.land = Some(Land::Trap(*trap)),
                Event::Rolled { seat, roll, dest } => {
                    u.land = None;
                    u.act = *seat;
                    u.roll = Some((*roll, *dest));
                    u.roll_id = u.roll_id.wrapping_add(1);
                    u.roll_at = now_ms();
                }
                Event::Missed { .. } => u.land = None,
                Event::Teleported { seat, to, .. } => segs.push(Seg::Jump(*seat, *to)),
                Event::Swapped { a, b } => {
                    segs.push(Seg::Jump(*a, u.game.players[*a as usize].pos));
                    segs.push(Seg::Jump(*b, u.game.players[*b as usize].pos));
                }
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
        // Whatever the events did not account for still ends on the engine's tile.
        let mut sim = u.shown.clone();
        for s in &segs {
            match *s {
                Seg::Walk(seat, _, to) | Seg::Jump(seat, to) => {
                    if let Some(x) = sim.get_mut(seat as usize) {
                        *x = to;
                    }
                }
            }
        }
        for (i, p) in u.game.players.iter().enumerate() {
            if sim.get(i).copied() != Some(p.pos) {
                segs.push(Seg::Jump(i as u8, p.pos));
            }
        }
        segs
    })
    .unwrap_or_default();
    if animate {
        play(segs)
    } else {
        cancel_anim();
        0
    }
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
        // While a move is being walked the drawn positions trail the engine's; otherwise
        // they ARE the engine's (the board, the pieces and the stage all read `shown`).
        if !animating() {
            u.shown = g.players.iter().map(|p| p.pos).collect();
        }
        let spelling = g.phase == Phase::AwaitSpelling && !g.is_npc_turn();
        dom::toggle_class("bgScreen", "spelling", spelling);
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
        let timed = spelling && g.cfg.variant.cfg().timed;
        dom::remove_class("bgTimer", "run");
        dom::set_hidden("bgTimer", !timed);
    });
    fit_view();
}

/// The landing the board is about to show, or last showed. A human about to spell sees the
/// destination's tier before typing (never the word); otherwise the last landing. Trap names
/// are shown only once a trap has been triggered (F4: hidden until then).
fn land_view(u: &Ui) -> Option<Land> {
    let spelling_dest = if u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn() {
        u.game.pending.as_ref().filter(|p| p.kind == SpellKind::Landing).and_then(|p| u.game.board.tiers.get(p.dest as usize).copied().flatten())
    } else {
        None
    };
    spelling_dest.map(Land::Tier).or(u.land)
}

fn tier_name(t: boardgame::Tier) -> String {
    i18n::t(&format!("level.{}", t.name()))
}

/// The callout over the board when the centre stage is not shown (the track, or a ring too
/// small to hold the stage): a pill naming the tier, or the trap.
fn land_pill(u: &Ui) -> String {
    match land_view(u) {
        Some(Land::Tier(t)) => format!("<span class=\"bg-pill\"><i class=\"bg-sw t-{0}\"></i><span data-tier>{1}</span></span>", t.name(), dom::escape_html(&tier_name(t))),
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
/// (the next tiles along the path, in one row, full panel width). The ring's shape
/// (Polish Feature 7) is the allowed one with the largest tile pitch in the real
/// `#bgView` box; the ring is presentation only (I-P2).
fn fit_view() {
    let Some((tiles, spelling)) = with_ui(|u| (u.game.board.len(), u.game.phase == Phase::AwaitSpelling && !u.game.is_npc_turn())) else {
        return;
    };
    let view = dom::el("bgView");
    let board = dom::el("bgBoard");
    // Measure with the strip class off: the view then takes all the leftover height.
    let _ = view.class_list().remove_1("strip");
    let (vw, vh) = (view.client_width() as f64, view.client_height() as f64);
    if vw <= 0.0 || vh <= 0.0 {
        return; // screen not shown
    }
    let (w, h) = ring::choose(tiles, vw, vh);
    let p = ring::pitch(w, h, vw, vh);
    let full = !spelling || p >= MIN_FULL_CELL_PX;
    if full {
        let _ = board.remove_attribute("class");
        let _ = board.set_attribute("viewBox", &format!("0 0 {w} {h}"));
        let _ = board.set_attribute("preserveAspectRatio", "xMidYMid meet");
        let _ = board.set_attribute("data-shape", &format!("{w}x{h}"));
        let _ = board.set_attribute("data-pitch", &format!("{p:.3}"));
        let _ = with_ui(|u| dom::set_html("bgBoard", &board_svg(u, w, h, p)));
        GEOM.with(|g| g.set(Some(Geom { ox: (vw - w as f64 * p) / 2.0, oy: (vh - h as f64 * p) / 2.0, p, w, h, vw, vh })));
        paint_overlays();
        return;
    }
    GEOM.with(|g| g.set(None));
    let _ = view.class_list().add_1("strip");
    let _ = board.set_attribute("class", "tr");
    let _ = board.remove_attribute("data-shape");
    // The svg is the leftover panel under the pill; its own size sets the track's proportions.
    let (sw, sh) = (board.client_width() as f64, board.client_height() as f64);
    let (sw, sh) = (if sw > 0.0 { sw } else { vw }, if sh > 0.0 { sh } else { vh });
    let (n, tvh) = track_dims(sw, sh);
    let _ = with_ui(|u| dom::set_html("bgBoard", &track_svg(u, n, tvh, sw / (n as f64 + 2.0 * TRACK_PAD))));
    // A little air each side, so the mover's ring is not clipped at the end tile.
    let _ = board.set_attribute("viewBox", &format!("{} 0 {} {tvh}", -TRACK_PAD, n as f64 + 2.0 * TRACK_PAD));
    let _ = board.set_attribute("preserveAspectRatio", "xMidYMid meet");
    paint_overlays();
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
/// `unit_px` is the size of one track tile on screen, so a piece can be 1.6 tiles
/// clamped to 24-40 px as on the ring.
fn track_svg(u: &Ui, n: u32, vh: f64, unit_px: f64) -> String {
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
    let present = present_tiers(b);
    let mut s = String::new();
    for k in 0..n {
        let i = (start + k) % len;
        let class = match b.tiers[i as usize] {
            Some(t) => format!("bg-t t-{}", t.name()),
            None if i == 0 => "bg-t start".to_string(),
            None => "bg-t end".to_string(),
        };
        let dcl = if dest == Some(i) { " dest" } else { "" };
        s.push_str(&format!("<rect class=\"{class}{dcl}\" x=\"{}\" y=\"{y0}\" width=\"0.9\" height=\"{th}\" rx=\"0.16\" data-slot=\"{}\"/>", slot(k) + 0.05, slot(k)));
    }
    let cy = vh / 2.0;
    // Tier pips, the start arrow and the finish flag, as on the ring.
    for k in 0..n {
        let i = (start + k) % len;
        let cx = slot(k) + 0.5;
        match b.tiers[i as usize] {
            Some(t) => s.push_str(&pips_svg(ring::pips(t, &present), cx, cy + th * 0.28, 0.1, 0.26)),
            None if i == 0 => s.push_str(&start_svg(cx, cy, th * 0.3)),
            None => s.push_str(&format!("<text class=\"bg-flag\" style=\"font-size:{}px\" x=\"{cx}\" y=\"{cy}\">\u{1f3c1}</text>", th * 0.62)),
        }
    }
    for &t in u.game.revealed.iter().chain(u.marks.iter()) {
        if let Some(k) = at(t) {
            s.push_str(&format!("<text class=\"bg-trap\" style=\"font-size:{}px\" x=\"{}\" y=\"{}\">\u{26a0}</text>", th * 0.5, slot(k) + 0.5, cy - th * 0.12));
        }
    }
    // Pieces: 1.6 tiles on a coloured disc, clamped to 24-40 px, fanned out when they share a tile.
    let d = (ring::piece_px(unit_px / 1.6) / unit_px).min(vh * 0.96).max(0.3);
    let me = u.game.current_seat();
    let order: Vec<usize> = (0..u.game.players.len()).filter(|&i| i as u8 != me).chain(std::iter::once(me as usize)).collect();
    for i in order {
        let p = &u.game.players[i];
        let Some(k) = at(u.shown.get(i).copied().unwrap_or(p.pos)) else { continue };
        let mates: Vec<usize> = (0..u.game.players.len()).filter(|&j| u.shown.get(j).copied().unwrap_or(0) == u.shown.get(i).copied().unwrap_or(0)).collect();
        let (fx, fy) = ring::fan(mates.len(), mates.iter().position(|&j| j == i).unwrap_or(0));
        // The group is held inside the panel as one, like the ring's.
        let (lo, hi) = (0..mates.len()).map(|m| ring::fan(mates.len(), m).0).fold((f64::MAX, f64::MIN), |a, f| (a.0.min(f), a.1.max(f)));
        let base = slot(k) + 0.5;
        let (min_x, max_x) = (base + lo * d - d / 2.0, base + hi * d + d / 2.0);
        let dx = if min_x < -TRACK_PAD { -TRACK_PAD - min_x } else if max_x > n as f64 + TRACK_PAD { n as f64 + TRACK_PAD - max_x } else { 0.0 };
        let (x, y) = (base + fx * d + dx, (cy + fy * d).clamp(d / 2.0, (vh - d / 2.0).max(d / 2.0)));
        let on = if i as u8 == me && u.game.phase != Phase::Finished { " act" } else { "" };
        s.push_str(&format!(
            "<circle class=\"bg-disc c{}{on}\" cx=\"{x}\" cy=\"{y}\" r=\"{}\"/><text class=\"bg-p\" style=\"font-size:{}px\" x=\"{x}\" y=\"{y}\">{}</text>",
            p.seat.piece % 4,
            d / 2.0,
            d * 0.62,
            piece(p.seat.piece)
        ));
    }
    s
}

/// The tiers a board uses (Jr two, Standard three), in ladder order: the scale the pips count on.
fn present_tiers(b: &boardgame::Board) -> Vec<boardgame::Tier> {
    let mut v: Vec<boardgame::Tier> = b.tiers.iter().flatten().copied().collect();
    v.sort();
    v.dedup();
    v
}

/// `n` dots in a row centred on (cx, cy): the tier read without colour (I-P11).
fn pips_svg(n: u8, cx: f64, cy: f64, r: f64, gap: f64) -> String {
    let mut s = String::new();
    for k in 0..n {
        s.push_str(&format!("<circle class=\"bg-pip\" cx=\"{}\" cy=\"{cy}\" r=\"{r}\"/>", cx + (k as f64 - (n as f64 - 1.0) / 2.0) * gap));
    }
    s
}

/// The start marker: an arrow pointing the way the pieces travel.
fn start_svg(cx: f64, cy: f64, r: f64) -> String {
    format!("<polygon class=\"bg-start\" points=\"{},{} {},{} {},{}\"/>", cx, cy - r, cx - r * 0.9, cy + r * 0.7, cx + r * 0.9, cy + r * 0.7)
}

/// The ring: tiles with their tier pips, the start arrow, the finish flag, a number on
/// every tenth tile (drawn on the interior side) and the traps that have been triggered.
/// Pieces are not here: they are persistent elements over the board (see `layout_pieces`).
fn board_svg(u: &Ui, w: u32, h: u32, p: f64) -> String {
    let b = &u.game.board;
    let last = b.last();
    let present = present_tiers(b);
    let dest = u.game.pending.as_ref().filter(|p| p.kind == SpellKind::Landing).map(|p| p.dest);
    let mut s = String::new();
    let mut extras = String::new();
    for i in 0..b.len() as u32 {
        let (x, y) = ring::tile_to_cell(i, w, h);
        let (xf, yf) = (x as f64, y as f64);
        let (cx, cy) = (xf + 0.5, yf + 0.5);
        let tier = b.tiers[i as usize];
        let class = match tier {
            Some(t) => format!("bg-t t-{}", t.name()),
            None if i == 0 => "bg-t start".to_string(),
            None => "bg-t end".to_string(),
        };
        let dcl = if dest == Some(i) { " dest" } else { "" };
        s.push_str(&format!("<rect class=\"{class}{dcl}\" x=\"{}\" y=\"{}\" width=\"0.9\" height=\"0.9\" rx=\"0.16\"/>", xf + 0.05, yf + 0.05));
        // A revealed trap stays on the map for the rest of the game (F4); its tile keeps its pips below the icon.
        let trap = u.game.revealed.contains(&i) || u.marks.contains(&i);
        match tier {
            Some(t) => extras.push_str(&pips_svg(ring::pips(t, &present), cx, if trap { cy + 0.3 } else { cy }, 0.1, 0.26)),
            None if i == 0 => extras.push_str(&start_svg(cx, cy, 0.3)),
            None => extras.push_str(&format!("<text class=\"bg-flag\" x=\"{cx}\" y=\"{cy}\">\u{1f3c1}</text>")),
        }
        if trap {
            extras.push_str(&format!("<circle class=\"bg-trapbg\" cx=\"{cx}\" cy=\"{}\" r=\"0.27\"/><text class=\"bg-trap\" x=\"{cx}\" y=\"{}\">\u{26a0}</text>", cy - 0.12, cy - 0.12));
        }
    }
    s.push_str(&extras);
    // Every tenth tile is numbered, on the interior side of the ring.
    let fs = (p * 0.55).clamp(9.0, 12.0) / p;
    let mut i = 10;
    while i < last {
        let (x, y) = ring::tile_to_cell(i, w, h);
        let ix = if x == 0 { 1 } else if x == w - 1 { w - 2 } else { x };
        let iy = if y == 0 { 1 } else if y == h - 1 { h - 2 } else { y };
        s.push_str(&format!("<text class=\"bg-num\" style=\"font-size:{fs:.3}px\" x=\"{}\" y=\"{}\">{i}</text>", ix as f64 + 0.5, iy as f64 + 0.5));
        i += 10;
    }
    s
}

// ------------------------------------------------------------------ overlays

/// Everything drawn over the board that is not part of its SVG: the persistent piece
/// elements and the centre stage (or, when there is no room for it, the tier pill).
fn paint_overlays() {
    let geom = GEOM.with(Cell::get);
    let Some(g) = geom else {
        dom::set_hidden("bgPieces", true);
        dom::set_hidden("bgStage", true);
        let _ = with_ui(|u| dom::set_html("bgLand", &land_pill(u)));
        return;
    };
    dom::set_hidden("bgPieces", false);
    ensure_pieces();
    layout_pieces(None);
    let staged = paint_stage(g);
    if staged {
        dom::set_html("bgLand", "");
    } else {
        let _ = with_ui(|u| dom::set_html("bgLand", &land_pill(u)));
    }
}

/// One element per seat, created once per game and then only moved, so a hop can animate.
fn ensure_pieces() {
    let Some((gid, html)) = with_ui(|u| {
        let h: String = u
            .game
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "<div class=\"bg-pc c{}\" data-seat=\"{i}\" role=\"img\" aria-label=\"{} {}\"><span>{}</span></div>",
                    p.seat.piece % 4,
                    piece(p.seat.piece),
                    i + 1,
                    piece(p.seat.piece)
                )
            })
            .collect();
        (u.gid.to_string(), h)
    }) else {
        return;
    };
    let el = dom::el("bgPieces");
    if el.get_attribute("data-gid").as_deref() != Some(gid.as_str()) {
        el.set_inner_html(&html);
        let _ = el.set_attribute("data-gid", &gid);
    }
}

/// Place every piece on its drawn tile. `ms` is the transition (a hop, a glide); `None`
/// keeps whatever walk is running and otherwise snaps (a re-fit must not animate).
fn layout_pieces(ms: Option<i32>) {
    let Some(g) = GEOM.with(Cell::get) else { return };
    let ms = ms.unwrap_or_else(|| if animating() { HOP_MS.with(Cell::get) } else { 0 });
    let _ = with_ui(|u| {
        let s = ring::piece_px(g.p);
        let n = u.game.players.len();
        let shown = |i: usize| u.shown.get(i).copied().unwrap_or(u.game.players[i].pos);
        for i in 0..n {
            let t = shown(i);
            let mates: Vec<usize> = (0..n).filter(|&j| shown(j) == t).collect();
            let place = |j: usize| -> (f64, f64) {
                let (cx, cy) = ring::tile_to_cell(shown(j), g.w, g.h);
                let (fx, fy) = ring::fan(mates.len(), mates.iter().position(|&m| m == j).unwrap_or(0));
                (g.ox + (cx as f64 + 0.5) * g.p + fx * s, g.oy + (cy as f64 + 0.5) * g.p + fy * s)
            };
            // The group moves as one inside the board, so a fan-out at a corner is not clipped.
            let pts: Vec<(f64, f64)> = mates.iter().map(|&j| place(j)).collect();
            let (minx, maxx) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
            let (miny, maxy) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
            // A few px of air, so the active piece's glow is not clipped at the board's edge.
            const AIR: f64 = 5.0;
            let shift = |lo: f64, hi: f64, max: f64| -> f64 {
                if lo - s / 2.0 < AIR {
                    s / 2.0 + AIR - lo
                } else if hi + s / 2.0 > max - AIR {
                    max - AIR - s / 2.0 - hi
                } else {
                    0.0
                }
            };
            let (x, y) = place(i);
            let (x, y) = (x + shift(minx, maxx, g.vw), y + shift(miny, maxy, g.vh));
            let active = i as u8 == u.act;
            let sel = format!("#bgPieces [data-seat=\"{i}\"]");
            if let Ok(Some(el)) = dom::doc().query_selector(&sel) {
                let _ = el.set_attribute(
                    "style",
                    &format!(
                        "width:{s:.1}px;height:{s:.1}px;font-size:{:.1}px;transform:translate({:.1}px,{:.1}px);transition:transform {ms}ms ease-in-out;z-index:{};--hop:{ms}ms",
                        s * 0.62,
                        x - s / 2.0,
                        y - s / 2.0,
                        if active { 30 } else { 10 + i }
                    ),
                );
                let _ = el.class_list().toggle_with_force("act", active);
            }
        }
    });
}

/// Feature 5. The ring's interior: the active seat's piece large, its seat icon, "Tile N of M",
/// the die and the landing. Shown only when the full ring fits and its interior has room;
/// otherwise the tier pill carries the landing. Returns whether the stage is shown.
fn paint_stage(g: Geom) -> bool {
    // The interior minus the tile numbers and the fan-out zone along the ring: pieces that share a
    // corner tile fan inward by up to ~2.9 tiles once the group is held inside the board.
    const INSET: f64 = 3.0;
    let (left, top) = (g.ox + INSET * g.p, g.oy + INSET * g.p);
    let (sw, sh) = ((g.w as f64 - 2.0 * INSET) * g.p, (g.h as f64 - 2.0 * INSET) * g.p);
    if sw < 130.0 || sh < 90.0 {
        dom::set_hidden("bgStage", true);
        return false;
    }
    let compact = sh < 170.0;
    let Some(info) = with_ui(|u| {
        let seat = u.act.min(u.game.players.len() as u8 - 1);
        let pl = &u.game.players[seat as usize];
        let tile = u.shown.get(seat as usize).copied().unwrap_or(pl.pos);
        (seat, pl.seat.piece, tile, u.game.board.last(), u.roll, u.roll_id, u.roll_at, land_view(u))
    }) else {
        return false;
    };
    let (seat, pc, tile, last, roll, roll_id, roll_at, land) = info;
    let rtl = crate::consts::dir_attr(&LANG.with(|l| l.borrow().0.clone())) == "rtl";
    let arrow = if rtl { "\u{2190}" } else { "\u{2192}" };
    let stage = dom::el("bgStage");
    let _ = stage.set_attribute(
        "style",
        &format!("left:{left:.1}px;top:{top:.1}px;width:{sw:.1}px;height:{sh:.1}px;--stpc:{:.0}px", (sh * 0.26).clamp(30.0, 64.0)),
    );
    dom::set_hidden("bgStage", false);
    let _ = stage.class_list().toggle_with_force("compact", compact);
    let tile_txt = i18n::tp("bg.tileOf", &[("n", &tile.to_string()), ("m", &last.to_string())]);
    // The big piece and the tile line change with the walk; the die and landing are rewritten
    // only when a roll or a landing changes, so a tumble plays once per roll.
    set_html_sig(
        "bgStPc",
        &format!("{seat}:{pc}"),
        &format!("<div class=\"bg-pc big c{}\"><span>{}</span><i class=\"bg-st-n\">{}</i></div>", pc % 4, piece(pc), seat + 1),
    );
    dom::set_text("bgStTile", &tile_txt);
    set_html_sig("bgStDie", &roll_id.to_string(), &roll.map(|(r, _)| format!("<b class=\"bg-die\">{r}</b>")).unwrap_or_default());
    // The landing waits for the tumble (<= 0.8 s) when the roll is fresh.
    let wait = (700.0 - (now_ms() - roll_at)).clamp(0.0, 700.0) as i32;
    let dest = roll.map(|r| r.1);
    let (land_html, land_txt, key) = match land {
        Some(Land::Tier(t)) => {
            let lead = dest.map(|d| format!("{arrow} {d} \u{b7}\u{a0}")).unwrap_or_default();
            (
                format!("<span dir=\"ltr\">{lead}</span><i class=\"bg-sw t-{0}\"></i><span data-tier>{1}</span>", t.name(), dom::escape_html(&tier_name(t))),
                tier_name(t),
                format!("t{}", t.name()),
            )
        }
        Some(Land::Trap(t)) => {
            let name = i18n::t(&format!("bg.trap.{}", t.key()));
            (format!("<i class=\"bg-sw trap\">\u{26a0}</i><span data-land=\"trap\">{}</span>", dom::escape_html(&name)), name, format!("x{}", t.key()))
        }
        None => match dest {
            Some(d) if d == last => (format!("<span dir=\"ltr\">{arrow} {d} </span>\u{1f3c1}"), String::new(), "f".to_string()),
            _ => (String::new(), String::new(), String::new()),
        },
    };
    set_html_sig("bgStLand", &format!("{roll_id}:{key}"), &land_html);
    let _ = dom::el("bgStLand").set_attribute("style", &format!("animation-delay:{wait}ms"));
    // One label reads the facts in order: whose turn, roll, landing tile, tier.
    let mut parts = vec![i18n::tp("bg.turnOf", &[("piece", piece(pc)), ("n", &(seat + 1).to_string())])];
    if let Some((r, d)) = roll {
        parts.push(i18n::tp("bg.rolled", &[("n", &r.to_string())]));
        parts.push(i18n::tp("bg.toTile", &[("n", &d.to_string())]));
    }
    if !land_txt.is_empty() {
        parts.push(land_txt);
    }
    let _ = stage.set_attribute("aria-label", &parts.join(". "));
    true
}

/// `set_html`, but only when `sig` changed, so an element that is animating is not restarted.
fn set_html_sig(id: &str, sig: &str, html: &str) {
    let el = dom::el(id);
    if el.get_attribute("data-sig").as_deref() != Some(sig) {
        el.set_inner_html(html);
        let _ = el.set_attribute("data-sig", sig);
    }
}

// ------------------------------------------------------------------ walking

fn reduce_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn animating() -> bool {
    ANIM.with(|a| a.borrow().is_some())
}

/// A piece's move, from the engine's events.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seg {
    Walk(u8, u32, u32),
    Jump(u8, u32),
}

fn anim_counter(attr: &str) {
    let el = dom::el("bgPieces");
    let n = el.get_attribute(attr).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let _ = el.set_attribute(attr, &(n + 1).to_string());
}

/// Walk the pieces through `segs`, tile by tile with a tick per hop (Feature 6), and return
/// how long the whole walk takes. A human's own piece gets `HUMAN_HOP_MS` a tile (never more
/// than `HUMAN_WALK_CAP_MS` in all); an NPC's walk is squeezed inside its `NPC_STEP_MS` step
/// (D10). With Reduce Motion on, a move is one cross-fade: no hops, no ticks.
fn play(segs: Vec<Seg>) -> i32 {
    if segs.is_empty() || GEOM.with(|g| g.get().is_none()) {
        return 0;
    }
    let npc = with_ui(|u| {
        let seat = match segs[0] {
            Seg::Walk(s, ..) | Seg::Jump(s, _) => s,
        };
        u.game.players.get(seat as usize).map(|p| p.seat.npc).unwrap_or(false)
    })
    .unwrap_or(false);
    let reduce = reduce_motion();
    let mut steps: VecDeque<Step> = VecDeque::new();
    for s in &segs {
        match *s {
            Seg::Walk(seat, from, to) if !reduce && to > from => {
                // Start the walk from the tile it left, if the picture lagged behind.
                if with_ui(|u| u.shown.get(seat as usize).copied() != Some(from)).unwrap_or(false) {
                    steps.push_back(Step::Jump(seat, from));
                }
                steps.extend((from + 1..=to).map(|t| Step::Hop(seat, t)));
            }
            Seg::Walk(seat, _, to) | Seg::Jump(seat, to) => steps.push_back(Step::Jump(seat, to)),
        }
    }
    // A Jump to the tile a piece is already drawn on costs nothing.
    let hops = steps.iter().filter(|s| matches!(s, Step::Hop(..))).count() as i32;
    let jumps = steps.iter().filter(|s| matches!(s, Step::Jump(..))).count() as i32;
    let cap = if npc { NPC_WALK_CAP_MS } else { HUMAN_WALK_CAP_MS };
    let hop_ms = if hops == 0 { 0 } else { ((cap - jumps * JUMP_MS) / hops).clamp(24, HUMAN_HOP_MS) };
    let total = hops * hop_ms + jumps * JUMP_MS;
    let token = ANIM_TOKEN.with(|t| {
        t.set(t.get().wrapping_add(1));
        t.get()
    });
    ANIM.with(|a| *a.borrow_mut() = Some(Anim { steps, hop_ms }));
    HOP_MS.with(|h| h.set(hop_ms));
    let _ = dom::el("bgPieces").set_attribute("data-moving", "1");
    anim_step(token);
    total
}

fn anim_step(token: u32) {
    if ANIM_TOKEN.with(Cell::get) != token {
        return;
    }
    let next = ANIM.with(|a| a.borrow_mut().as_mut().and_then(|a| a.steps.pop_front().map(|s| (s, a.hop_ms))));
    let Some((step, hop_ms)) = next else {
        end_anim();
        return;
    };
    let delay = match step {
        Step::Hop(seat, tile) => {
            with_ui(|u| {
                if let Some(s) = u.shown.get_mut(seat as usize) {
                    *s = tile;
                }
            });
            HOP_MS.with(|h| h.set(hop_ms));
            layout_pieces(Some(hop_ms));
            // Restart the little bounce: alternate between two keyframe names.
            if let Ok(Some(el)) = dom::doc().query_selector(&format!("#bgPieces [data-seat=\"{seat}\"]")) {
                let v = if el.get_attribute("data-h").as_deref() == Some("a") { "b" } else { "a" };
                let _ = el.set_attribute("data-h", v);
            }
            anim_counter("data-hops");
            if !reduce_motion() {
                anim_counter("data-ticks");
                haptics::key_tap();
            }
            hop_ms
        }
        Step::Jump(seat, tile) => {
            let moved = with_ui(|u| {
                let cur = u.shown.get(seat as usize).copied();
                if let Some(s) = u.shown.get_mut(seat as usize) {
                    *s = tile;
                }
                cur != Some(tile)
            })
            .unwrap_or(false);
            if reduce_motion() {
                // One cross-fade, no travel.
                layout_pieces(Some(0));
                if let Ok(Some(el)) = dom::doc().query_selector(&format!("#bgPieces [data-seat=\"{seat}\"]")) {
                    let v = if el.get_attribute("data-f").as_deref() == Some("a") { "b" } else { "a" };
                    let _ = el.set_attribute("data-f", v);
                }
            } else {
                layout_pieces(Some(if moved { JUMP_MS } else { 0 }));
            }
            if moved { JUMP_MS } else { 0 }
        }
    };
    if let Some(g) = GEOM.with(Cell::get) {
        paint_stage(g);
    }
    if delay == 0 {
        anim_step(token);
    } else {
        dom::after_ms(delay, move || anim_step(token));
    }
}

/// The walk is over: the drawn positions are the engine's again.
fn end_anim() {
    ANIM.with(|a| *a.borrow_mut() = None);
    let _ = dom::el("bgPieces").remove_attribute("data-moving");
    with_ui(|u| u.shown = u.game.players.iter().map(|p| p.pos).collect());
    layout_pieces(Some(0));
    if let Some(g) = GEOM.with(Cell::get) {
        paint_stage(g);
    }
}

/// Stop any walk and put every piece on its final tile (a tap that skips, a new game, leaving).
fn cancel_anim() {
    ANIM_TOKEN.with(|t| t.set(t.get().wrapping_add(1)));
    if animating() {
        end_anim();
    }
}

/// Test seam: draw the revealed-trap mark on a tile without touching the game.
pub fn seam_mark(tile: u32) {
    with_ui(|u| u.marks.push(tile));
    fit_view();
}

/// Test seam: walk a seat's piece `n` tiles forward from where it stands (the real walk, the
/// real timing and counters), and return the time the walk will take.
pub fn seam_hop(seat: u8, n: u32) -> i32 {
    let from = with_ui(|u| u.shown.get(seat as usize).copied()).flatten().unwrap_or(0);
    play(vec![Seg::Walk(seat, from, from + n)])
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
            let walk = after_apply(true);
            haptics::key_tap();
            after_roll(walk);
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
fn after_roll(walk: i32) {
    render();
    let finished = with_ui(|u| u.game.phase == Phase::Finished).unwrap_or(false);
    if finished {
        // Let the last walk be seen before the podium covers it.
        let token = bump();
        dom::after_ms(walk + 300, move || {
            if TOKEN.with(Cell::get) == token {
                advance();
            }
        });
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
    let std = with_ui(|u| u.game.cfg.variant.cfg().timed).unwrap_or(false);
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
    let walk = after_apply(true);
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
    dom::after_ms(HUMAN_BEAT_MS.max(walk + 200), move || {
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
    let walk = after_apply(true);
    render();
    let token = bump();
    dom::after_ms(HUMAN_BEAT_MS.max(walk + 200), move || {
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
                    "tiles": g.board.len(),
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
