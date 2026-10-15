//! CC-RANDOSPELL v1 Phase A -- the screen (Sprint).
//!
//! Renders `randospell::Session` and forwards presses; every rule lives in
//! `src/randospell/`. This file owns what a reducer cannot: the clock (the 30 s
//! soft timer, D19), the audio, the DOM and the ledger write.
//!
//! * D8: no new input method. The key grid is built from `boardgame_input`
//!   exactly as Board Game does, and answers go through `boardgame_grade`.
//! * D9: served words are written once, at the end, to the shared no-repeat
//!   history (`wordsearch_ui::LEDGER_KEY`) under `randospell:{lang}`.
//! * I9: nothing here writes Climb, Daily Challenge or shield state, and the
//!   mode adds no telemetry.
//! * F6: Daily is one attempt per (date, language, tier), spent at the START and
//!   kept in the mode-local store; the result is shareable text.
//! * F7: Spell Jr is Sprint only with one die, no pot, no Ward, no shown
//!   multiplier, no timer, and a second try after a miss. Every Jr line is
//!   encouraging; the streak is kept silently for the result card.
//! * F8: Head-to-Head is pass-and-play with a hand-off card between turns.
//! * F5 / D15: in Marathon a correct answer opens a Bank / Roll again card;
//!   the game ends only on a Bank that reaches the goal.
//! * A16 / F1: the die fades in; the shake is CSS and `prefers-reduced-motion`
//!   removes it. The haptic fires either way.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::boardgame_input::{self as input, Key};
use crate::experience::Experience;
use crate::randospell::bank::{self, Bank, MODE_ID};
use crate::randospell::store::{Store, STORE_KEY};
use crate::randospell::table::{Difficulty, Personality, Step, Table};
use crate::randospell::{Call, Mode, Rules, Session, MARATHON_GOAL, SPRINT_GOAL, TIMER_SECS};
use crate::wordsearch::ledger::Ledger;
use crate::{dom, haptics, i18n, speech_out, storage, App};

/// What the setup card can start. Each beyond Sprint has its own flag (F9).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Sprint,
    Marathon,
    Daily,
    H2h,
    /// v1.1 F10: the human against 1-3 computer players.
    Table,
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::Sprint, Kind::Marathon, Kind::Daily, Kind::H2h, Kind::Table];

    fn id(self) -> &'static str {
        match self {
            Kind::Sprint => "sprint",
            Kind::Marathon => "marathon",
            Kind::Daily => "daily",
            Kind::H2h => "h2h",
            Kind::Table => "table",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Kind::Sprint => "rs.mode.sprint",
            Kind::Marathon => "rs.mode.marathon",
            Kind::Daily => "rs.mode.daily",
            Kind::H2h => "rs.mode.h2h",
            Kind::Table => "rs.mode.table",
        }
    }

    fn enabled(self) -> bool {
        match self {
            Kind::Sprint => true,
            Kind::Marathon => crate::flags::randospell_marathon(),
            Kind::Daily => crate::flags::randospell_daily(),
            Kind::H2h => crate::flags::randospell_h2h(),
            Kind::Table => crate::flags::randospell_npc(),
        }
    }

    fn mode(self) -> Mode {
        match self {
            Kind::Sprint => Mode::Sprint,
            Kind::Marathon | Kind::Daily => Mode::Marathon,
            Kind::H2h => Mode::HeadToHead,
            Kind::Table => Mode::Table,
        }
    }
}

struct Ui {
    s: Session,
    /// F10: the computer seats, when this game has them.
    tbl: Option<Table>,
    kind: Kind,
    /// Daily: the store key this attempt spent, and the date it was played for.
    daily_key: Option<String>,
    date: String,
    stored: bool,
    lang: String,
    tier: usize,
    typed: String,
    keys: Vec<Vec<Key>>,
    /// A roll is out and its word is owed.
    spelling: bool,
    /// v1.1: the die has landed and the player may still Stretch or Ear; the
    /// word is not drawn until they listen (D-R9).
    choosing: bool,
    stretch_sel: bool,
    ear_sel: bool,
    /// F15: the seat the player chose to steal from, instead of spelling for points.
    steal_target: Option<usize>,
    recorded: bool,
    day: u32,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    /// The tier index the setup card has selected (default Medium).
    static TIER: Cell<usize> = const { Cell::new(1) };
    /// The mode the setup card has selected. Marathon needs its own flag (F9).
    static KIND: Cell<Kind> = const { Cell::new(Kind::Sprint) };
    /// The profile is Spell Jr (F7): Sprint only, one die, Easy + Medium.
    static KID: Cell<bool> = const { Cell::new(false) };
    /// F10 setup: how many NPCs, which personalities, how hard.
    static NPC_N: Cell<usize> = const { Cell::new(1) };
    static NPC_P: RefCell<[Personality; 3]> = const { RefCell::new([Personality::Cautious; 3]) };
    static NPC_D: Cell<Difficulty> = const { Cell::new(Difficulty::Normal) };
    /// An NPC turn is being played back: the orb waits (D10).
    static NPC_BUSY: Cell<bool> = const { Cell::new(false) };
    /// Bumped on every press, answer and close so a stale 30 s timer is a no-op.
    static TOKEN: Cell<u32> = const { Cell::new(0) };
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|u| u.borrow_mut().as_mut().map(f))
}

fn day() -> u32 {
    (js_sys::Date::now() / 86_400_000.0) as u32
}

fn ledger_key(lang: &str) -> String {
    format!("randospell:{lang}")
}

fn load_ledger() -> Ledger {
    storage::get_json(crate::wordsearch_ui::LEDGER_KEY).unwrap_or_default()
}

fn bump() -> u32 {
    TOKEN.with(|t| {
        t.set(t.get().wrapping_add(1));
        t.get()
    })
}

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("rsOpenBtn", move || open(&a));
    }
    dom::on_click("rsExit", close);
    // The end card covers the top bar, so the exit is re-offered on it.
    dom::on_click("rsOverExit", close);
    // The hand-off card covers the top bar too (F8).
    dom::on_click("rsHandExit", close);
    dom::on_click("rsSetupExit", close);
    dom::on_click("rsStart", start);
    dom::on_click("rsAgain", start);
    dom::on_click("rsOrb", orb);
    dom::on_click("rsDel", delete);
    dom::on_click("rsGo", submit);
    dom::on_click("rsBank", bank_it);
    dom::on::<web_sys::MouseEvent, _>("rsCall", "click", call_tap);
    dom::on::<web_sys::MouseEvent, _>("rsCharms", "click", charm_tap);
    dom::on_click("rsHandGo", || dom::set_hidden("rsHand", true));
    dom::on_click("rsDailyShare", share_daily);
    dom::on_click("rsShare", share_daily);
    dom::on_click("rsRollAgain", roll_again);
    dom::on::<web_sys::MouseEvent, _>("rsModes", "click", mode_tap);
    dom::on::<web_sys::MouseEvent, _>("rsNpcCfg", "click", npc_cfg_tap);
    // A tap anywhere on the table finishes an NPC run at once (board game D10).
    dom::on::<web_sys::MouseEvent, _>("rsTable", "click", |_| skip_npc());
    dom::on::<web_sys::MouseEvent, _>("rsTiers", "click", tier_tap);
    dom::on::<web_sys::MouseEvent, _>("rsKeys", "click", key_tap);
}

pub fn open(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (if s.lang == crate::consts::MINE { crate::consts::EN.to_string() } else { s.lang.clone() }, s.kid)
    };
    KID.with(|k| k.set(kid));
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    UI.with(|u| *u.borrow_mut() = None);
    if kid || !KIND.with(Cell::get).enabled() {
        KIND.with(|m| m.set(Kind::Sprint));
    }
    for id in ["rsField", "rsKeys"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            dom::set_lang_dir(&e, &lang);
        }
    }
    dom::set_hidden("rsSetup", false);
    dom::set_hidden("rsPlay", true);
    dom::set_hidden("rsOver", true);
    dom::remove_class("rsScreen", "playing");
    render_setup();
    dom::add_class("rsScreen", "show");
}

fn close() {
    bump();
    speech_out::stop();
    finish_record();
    // F6: leaving a Daily early keeps the attempt spent and records how far it got.
    let _ = with_ui(|u| {
        if let (Some(k), false) = (u.daily_key.clone(), u.stored) {
            u.stored = true;
            let mut st = load_store();
            st.finish_daily(&k, &u.s, false);
            save_store(&st);
        }
    });
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("rsScreen", "show");
}

fn load_store() -> Store {
    storage::get_json(STORE_KEY).unwrap_or_default()
}

fn save_store(st: &Store) {
    storage::set_json(STORE_KEY, st);
}

/// Local calendar date, YYYY-MM-DD: the day a Daily belongs to.
fn today() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

fn mode_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-m]").ok().flatten()) else {
        return;
    };
    let id = el.get_attribute("data-m").unwrap_or_default();
    if let Some(k) = Kind::ALL.iter().copied().find(|k| k.id() == id && k.enabled()) {
        KIND.with(|c| c.set(k));
        render_setup();
    }
}

fn tier_label(ix: usize) -> String {
    i18n::t(&format!("level.{}", crate::consts::TIER_ORDER[ix]))
}

/// Which audience's bank and rules this screen is serving.
fn exp() -> Experience {
    if KID.with(Cell::get) { Experience::Junior } else { Experience::Standard }
}

/// What the player owns in `lang` (D10): the app's one entitlement resolution.
fn level_of(lang: &str) -> crate::entitlements::AccessLevel {
    crate::play_hub::live_entitlements().lang_level(lang)
}

/// The tiers this player may pool and pick: Full owns every allowed tier,
/// Preview the first, and Jr follows its own policy (free, D10).
fn owned_tiers(lang: &str) -> Vec<usize> {
    let level = level_of(lang);
    crate::experience::allowed_tiers(exp(), MODE_ID)
        .iter()
        .filter_map(|name| crate::consts::TIER_ORDER.iter().position(|t| t == name))
        .filter(|ix| bank::tier_owned(lang, exp(), level, *ix))
        .collect()
}

fn render_setup() {
    let lang = LANG.with(|l| l.borrow().clone());
    let kid = KID.with(Cell::get);
    let level = level_of(&lang);
    let ok = if kid {
        crate::flags::randospell_jr() && bank::jr_eligible(&lang)
    } else {
        bank::eligible_at(&lang, Experience::Standard, level)
    };
    let owned = owned_tiers(&lang);
    // A tier the player does not own cannot stay selected.
    if !kid && !owned.is_empty() && !owned.contains(&TIER.with(Cell::get)) {
        TIER.with(|t| t.set(owned[0]));
    }
    let allowed: Vec<&str> = owned.iter().map(|ix| crate::consts::TIER_ORDER[*ix]).collect();
    let sel = TIER.with(Cell::get);
    let btns: String = allowed
        .iter()
        .filter_map(|name| crate::consts::TIER_ORDER.iter().position(|t| t == name))
        .map(|ix| {
            format!(
                "<button type=\"button\" class=\"rs-btn{}\" data-t=\"{ix}\" aria-pressed=\"{}\">{}</button>",
                if ix == sel { " on" } else { "" },
                ix == sel,
                dom::escape_html(&tier_label(ix))
            )
        })
        .collect();
    // Spell Jr has no tier choice (Easy and Medium pool) and no mode picker.
    dom::set_html("rsTiers", &if kid { String::new() } else { btns });
    // The picker exists only when there is a choice to make.
    let kind = KIND.with(Cell::get);
    // D10: Daily is one shared game for everyone, so it needs the full bank:
    // a Preview player (first tier only) would see different words. Absent, not locked.
    let offered: Vec<Kind> =
        Kind::ALL.iter().copied().filter(|k| k.enabled() && (*k != Kind::Daily || level == crate::entitlements::AccessLevel::Full)).collect();
    if !offered.contains(&kind) {
        KIND.with(|m| m.set(Kind::Sprint));
    }
    let kind = KIND.with(Cell::get);
    let modes: String = if offered.len() > 1 && !kid {
        offered
            .iter()
            .map(|k| {
                format!(
                    "<button type=\"button\" class=\"rs-btn{}\" data-m=\"{}\" aria-pressed=\"{}\">{}</button>",
                    if *k == kind { " on" } else { "" },
                    k.id(),
                    *k == kind,
                    dom::escape_html(&i18n::t(k.label_key()))
                )
            })
            .collect()
    } else {
        String::new()
    };
    dom::set_html("rsModes", &modes);
    render_npc_cfg(kind == Kind::Table && !kid);
    // D4: a remap is shown, never silent ("12 plays as 11").
    let info = if ok {
        let cov = Bank::for_lang_at(&lang, exp(), level).cov;
        // Jr lists remaps in die faces (F7); everyone else in word lengths.
        let pairs: Vec<(usize, usize)> = if kid {
            cov.jr_remaps().into_iter().map(|(a, b)| (a as usize, b as usize)).collect()
        } else {
            cov.remaps().into_iter().map(|(f, l)| (f as usize, l)).collect()
        };
        let lines: Vec<String> = pairs
            .into_iter()
            .map(|(f, l)| format!("<li>{}</li>", dom::escape_html(&i18n::tp("rs.remap", &[("face", &f.to_string()), ("len", &l.to_string())]))))
            .collect();
        let how = format!("<p>{}</p>", dom::escape_html(&i18n::t(match kind {
            Kind::Sprint if kid => "rs.howJr",
            Kind::Sprint => "rs.how",
            Kind::Marathon => "rs.howM",
            Kind::Daily => "rs.dailyHow",
            Kind::H2h => "rs.h2hHow",
            Kind::Table => "rs.tableHow",
        })));
        if lines.is_empty() { how } else { format!("{how}<ul>{}</ul>", lines.concat()) }
    } else {
        String::new()
    };
    dom::set_html("rsInfo", &info);
    // F6: a Daily already spent for this (date, language, tier) cannot start.
    let mut note = if ok { String::new() } else { i18n::t("rs.soon") };
    let mut can_start = ok;
    let mut share_ready = false;
    if ok && kind == Kind::Daily {
        let key = crate::randospell::daily::key(&today(), &lang, sel);
        if let Some(e) = load_store().daily_entry(&key) {
            can_start = false;
            if e.done {
                note = i18n::tp("rs.dailyDone", &[("n", &e.score.to_string()), ("r", &e.rolls.to_string())]);
                share_ready = crate::share::available();
            } else {
                note = i18n::t("rs.dailyGone");
            }
        }
    }
    dom::set_hidden("rsDailyShare", !share_ready);
    dom::set_text("rsNote", &note);
    dom::set_disabled("rsStart", !can_start);
}

fn tier_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-t]").ok().flatten()) else {
        return;
    };
    if let Some(ix) = el.get_attribute("data-t").and_then(|d| d.parse::<usize>().ok()) {
        TIER.with(|t| t.set(ix.min(3)));
        render_setup();
    }
}

fn start() {
    bump();
    speech_out::stop();
    finish_record();
    let lang = LANG.with(|l| l.borrow().clone());
    let kid = KID.with(Cell::get);
    let level = level_of(&lang);
    let allowed = if kid {
        crate::flags::randospell_jr() && bank::jr_eligible(&lang)
    } else {
        bank::eligible_at(&lang, Experience::Standard, level)
    };
    if !allowed {
        return;
    }
    let kind = KIND.with(Cell::get);
    if kind == Kind::Daily && level != crate::entitlements::AccessLevel::Full {
        return; // D10: Daily needs the full bank
    }
    // Jr has no tier choice: it pools Easy and Medium and prefers the easier.
    let mut tier = if kid { 0 } else { TIER.with(Cell::get) };
    let owned = owned_tiers(&lang);
    if !kid && !owned.contains(&tier) {
        tier = owned.first().copied().unwrap_or(0);
    }
    let bank = Bank::for_lang_at(&lang, exp(), level);
    let entries: Vec<String> = bank.words().map(str::to_string).collect();
    let keys = input::layout(&lang, kid, &entries);
    let day = day();
    let date = today();
    let mut daily_key = None;
    let s = if kind == Kind::Daily {
        // F6: spend the day's one attempt before anything is shown, so a
        // reload cannot replay words already heard.
        let key = crate::randospell::daily::key(&date, &lang, tier);
        let mut st = load_store();
        if !st.try_start_daily(&key) {
            render_setup();
            return;
        }
        save_store(&st);
        let Some(s) = Session::new_daily(&date, &lang, tier, bank) else { return };
        daily_key = Some(key);
        s
    } else {
        let led = load_ledger();
        let key = ledger_key(&lang);
        let avoid = led.seen.iter().filter(|s| s.k == key && led.in_window(s, day)).map(|s| s.w.clone()).collect();
        let seed = (js_sys::Date::now() as u64) ^ led.counter.wrapping_mul(0x9E37_79B9);
        Session::new(seed, if kid { Mode::Jr } else { kind.mode() }, tier, bank, avoid)
    };
    let mut s = s;
    s.rules = rules_from_flags();
    // F10: the table. Jr gets exactly one racing NPC whenever the flag is on.
    let want_table = if kid { crate::flags::randospell_npc() } else { kind == Kind::Table };
    let tbl = want_table.then(|| {
        let n = NPC_N.with(Cell::get).clamp(1, 3);
        let pers: Vec<Personality> = NPC_P.with(|p| p.borrow()[..n].to_vec());
        Table::new(js_sys::Date::now() as u64 ^ 0x5EA7, &s, pers, NPC_D.with(Cell::get))
    });
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui { s, tbl, kind, daily_key, date, stored: false, lang, tier, typed: String::new(), keys, spelling: false, choosing: false, stretch_sel: false, ear_sel: false, steal_target: None, recorded: false, day })
    });
    dom::set_hidden("rsSetup", true);
    dom::set_hidden("rsPlay", false);
    dom::set_hidden("rsOver", true);
    dom::set_hidden("rsHand", true);
    dom::add_class("rsScreen", "playing");
    render_keys();
    render_hud();
    set_die("", false, false);
    dom::set_text("rsFeed", "");
    spell_panel(false);
    decision(false);
    refresh_bets();
    render_table();
    // An NPC may be first to move (F10): play it before the human's roll.
    let has_table = with_ui(|u| u.tbl.is_some()).unwrap_or(false);
    if has_table {
        table_run(false);
    }
}

// --------------------------------------------------------------- F10 table

fn npc_icon(seat: usize) -> &'static str {
    ["\u{1f3a9}", "\u{1f41d}", "\u{1fa84}", "\u{1f4d6}"].get(seat.saturating_sub(1)).copied().unwrap_or("")
}

fn npc_name(seat: usize) -> String {
    format!("{} {}", npc_icon(seat), i18n::t(&format!("rs.npc.n{}", seat.saturating_sub(1))))
}

fn render_npc_cfg(show: bool) {
    if !show {
        dom::set_html("rsNpcCfg", "");
        dom::set_hidden("rsNpcCfg", true);
        return;
    }
    let n = NPC_N.with(Cell::get).clamp(1, 3);
    let diff = NPC_D.with(Cell::get);
    let btn = |attr: &str, val: &str, on: bool, label: String| {
        format!(
            "<button type=\"button\" class=\"rs-btn{}\" {attr}=\"{val}\" aria-pressed=\"{on}\">{}</button>",
            if on { " on" } else { "" },
            dom::escape_html(&label)
        )
    };
    let mut h = format!("<div class=\"rs-opt\"><span class=\"rs-bet-label\">{}</span>", dom::escape_html(&i18n::t("rs.npc.count")));
    for c in 1..=3usize {
        h.push_str(&btn("data-n", &c.to_string(), c == n, c.to_string()));
    }
    h.push_str("</div><div class=\"rs-opt\">");
    for d in Difficulty::ALL {
        h.push_str(&btn("data-d", d.id(), d == diff, i18n::t(&format!("rs.diff.{}", d.id()))));
    }
    h.push_str("</div>");
    let pers = NPC_P.with(|p| *p.borrow());
    for i in 0..n {
        h.push_str(&format!("<div class=\"rs-opt\"><span class=\"rs-bet-label\">{}</span>", dom::escape_html(&npc_name(i + 1))));
        for p in Personality::ALL {
            h.push_str(&btn("data-p", &format!("{i}:{}", p.id()), pers[i] == p, i18n::t(&format!("rs.npc.{}", p.id()))));
        }
        h.push_str("</div>");
    }
    dom::set_html("rsNpcCfg", &h);
    dom::set_hidden("rsNpcCfg", false);
}

fn npc_cfg_tap(e: web_sys::MouseEvent) {
    let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
    if let Some(el) = t.closest("[data-n]").ok().flatten() {
        if let Some(n) = el.get_attribute("data-n").and_then(|v| v.parse::<usize>().ok()) {
            NPC_N.with(|c| c.set(n.clamp(1, 3)));
        }
    } else if let Some(el) = t.closest("[data-d]").ok().flatten() {
        let d = match el.get_attribute("data-d").as_deref() {
            Some("easy") => Difficulty::Easy,
            Some("tough") => Difficulty::Tough,
            _ => Difficulty::Normal,
        };
        NPC_D.with(|c| c.set(d));
    } else if let Some(el) = t.closest("[data-p]").ok().flatten() {
        let v = el.get_attribute("data-p").unwrap_or_default();
        if let Some((i, id)) = v.split_once(':') {
            if let (Ok(i), Some(p)) = (i.parse::<usize>(), Personality::ALL.iter().copied().find(|p| p.id() == id)) {
                NPC_P.with(|a| {
                    if i < 3 {
                        a.borrow_mut()[i] = p;
                    }
                });
            }
        }
    }
    render_npc_cfg(true);
}

/// The seats at the table: banked score and what each has at risk.
fn render_table() {
    let rows = with_ui(|u| {
        let t = u.tbl.as_ref()?;
        let mut out = String::new();
        for seat in 0..t.seats() {
            let (name, score, pot, active) = if seat == 0 {
                (i18n::t("rs.you"), u.s.score, u.s.pot, t.turn == 0)
            } else {
                let n = &t.npcs[seat - 1];
                (npc_name(seat), n.score, n.pot, t.turn == seat)
            };
            // F16: the leader of the Final Round is marked, in a class and in text.
            let lead = t.final_leader == Some(seat);
            out.push_str(&format!(
                "<div class=\"rs-seat{}{}\" data-seat=\"{seat}\"><b>{}</b><span class=\"rs-seat-score\">{score}</span><span>{}</span></div>",
                if active { " on" } else { "" },
                if lead { " lead" } else { "" },
                dom::escape_html(&name),
                dom::escape_html(&i18n::tp("rs.pot", &[("n", &pot.to_string())]))
            ));
        }
        Some(out)
    })
    .flatten();
    match rows {
        Some(h) => {
            dom::set_html("rsTable", &h);
            dom::set_hidden("rsTable", false);
        }
        None => {
            dom::set_html("rsTable", "");
            dom::set_hidden("rsTable", true);
        }
    }
}

fn step_text(s: &Step) -> String {
    let nm = |seat: usize| npc_name(seat);
    match s {
        Step::Rolled { seat, len, doubles, stretched } => {
            let mut t = i18n::tp("rs.npcRolled", &[("name", &nm(*seat)), ("n", &len.to_string())]);
            if *doubles {
                t = format!("{t} · {}", i18n::t("rs.glow"));
            }
            if *stretched {
                t = format!("{t} · {}", i18n::t("rs.stretch"));
            }
            t
        }
        Step::Hit { seat, points, .. } => i18n::tp("rs.npcHit", &[("name", &nm(*seat)), ("n", &points.to_string())]),
        Step::Banked { seat, amount, .. } => i18n::tp("rs.npcBank", &[("name", &nm(*seat)), ("n", &amount.to_string())]),
        Step::Bust { seat, lost } if *lost > 0 => i18n::tp("rs.npcBust", &[("name", &nm(*seat)), ("n", &lost.to_string())]),
        Step::Bust { seat, .. } => i18n::tp("rs.npcMiss", &[("name", &nm(*seat))]),
        Step::Warded { seat, pot } => i18n::tp("rs.npcWard", &[("name", &nm(*seat)), ("n", &pot.to_string())]),
        Step::Raced { seat, points, .. } if *points > 0 => i18n::tp("rs.npcHit", &[("name", &nm(*seat)), ("n", &points.to_string())]),
        Step::Raced { seat, .. } => i18n::tp("rs.npcMiss", &[("name", &nm(*seat))]),
        Step::Stole { seat, target, amount } => {
            let tn = if *target == 0 { i18n::t("rs.you") } else { nm(*target) };
            i18n::tp("rs.npcStole", &[("name", &nm(*seat)), ("target", &tn), ("n", &amount.to_string())])
        }
        Step::StealFailed { seat, .. } => i18n::tp("rs.npcStealFail", &[("name", &nm(*seat))]),
        Step::FinalRound { leader } => {
            let who = if *leader == 0 { i18n::t("rs.you") } else { nm(*leader) };
            i18n::tp("rs.finalRound", &[("name", &who)])
        }
        Step::Won { seat } => i18n::tp("rs.nameWins", &[("name", &nm(*seat))]),
    }
}

/// Ask the table to run its NPCs; `human_done` is true after the human's turn.
fn table_run(human_done: bool) {
    let steps = with_ui(|u| {
        let t = u.tbl.as_mut()?;
        if human_done {
            t.human_turn_over(&u.s);
        }
        Some(t.run_npcs(&mut u.s))
    })
    .flatten();
    let Some(steps) = steps else { return };
    if steps.is_empty() {
        table_settled();
        return;
    }
    // D10: one NPC run in under 7 s, however many steps it has.
    let each = (6500 / steps.len().max(1) as i32).clamp(120, 700);
    NPC_BUSY.with(|b| b.set(true));
    dom::set_disabled("rsOrb", true);
    let token = bump();
    play_step(steps, 0, each, token);
}

fn play_step(steps: Vec<Step>, i: usize, each: i32, token: u32) {
    if TOKEN.with(Cell::get) != token || !NPC_BUSY.with(Cell::get) {
        return; // skipped, or the screen was closed
    }
    if i >= steps.len() {
        NPC_BUSY.with(|b| b.set(false));
        table_settled();
        return;
    }
    dom::set_text("rsFeed", &step_text(&steps[i]));
    if matches!(steps[i], Step::Banked { .. } | Step::Bust { .. } | Step::Warded { .. } | Step::Won { .. }) {
        render_table();
    }
    dom::after_ms(each, move || play_step(steps, i + 1, each, token));
}

/// A tap finishes the run now: the engine has already decided everything.
fn skip_npc() {
    if NPC_BUSY.with(Cell::get) {
        NPC_BUSY.with(|b| b.set(false));
        bump();
        table_settled();
    }
}

/// The NPCs are done: show the table, end the game or hand the turn back.
fn table_settled() {
    NPC_BUSY.with(|b| b.set(false));
    render_table();
    render_hud();
    let over = with_ui(|u| u.tbl.as_ref().map_or(false, |t| t.winner.is_some()) || u.s.ended.is_some()).unwrap_or(false);
    if over {
        show_over();
    } else {
        dom::set_disabled("rsOrb", false);
        refresh_bets();
    }
}

/// F5: show or hide the Bank / Roll again card, and lock the orb while it shows.
fn decision(on: bool) {
    dom::set_hidden("rsDecide", !on);
    dom::set_disabled("rsOrb", on);
    if on {
        let pot = with_ui(|u| u.s.pot).unwrap_or(0);
        dom::set_text("rsBank", &i18n::tp("rs.bank", &[("n", &pot.to_string())]));
    }
}

fn spell_panel(on: bool) {
    dom::set_hidden("rsSpell", !on);
    if on {
        dom::add_class("rsScreen", "spelling");
    } else {
        dom::remove_class("rsScreen", "spelling");
    }
    if !on {
        dom::set_disabled("rsOrb", false);
    }
    dom::set_text("rsOrb", &i18n::t(if on { "rs.replay" } else { "rs.roll" }));
    if let Some(e) = dom::doc().get_element_by_id("rsOrb") {
        let _ = e.set_attribute("data-state", if on { "replay" } else { "roll" });
    }
}

fn render_keys() {
    let html: String = with_ui(|u| {
        u.keys
            .iter()
            .map(|row| {
                let keys: String = row
                    .iter()
                    .map(|k| {
                        format!(
                            "<button type=\"button\" class=\"rs-key{}\" data-k=\"{}\">{}</button>",
                            if matches!(k, Key::Mod(_)) { " mod" } else { "" },
                            dom::escape_html(&k.data()),
                            dom::escape_html(&k.face())
                        )
                    })
                    .collect();
                format!("<div class=\"rs-row\">{keys}</div>")
            })
            .collect()
    })
    .unwrap_or_default();
    dom::set_html("rsKeys", &html);
}

fn render_hud() {
    let Some((score, streak, rolls, mode, pot, ward, ward_used, turn, scores, bonus)) = with_ui(|u| {
        (u.s.score, u.s.streak, u.s.rolls, u.s.mode, u.s.pot, u.s.ward, u.s.ward_used, u.s.turn, [u.s.score_of(0), u.s.score_of(1)], u.s.bonus)
    }) else {
        return;
    };
    let goal = mode.goal();
    let h2h = mode == Mode::HeadToHead;
    if h2h {
        dom::set_text("rsScore", &i18n::tp("rs.h2hScore", &[("a", &scores[0].to_string()), ("b", &scores[1].to_string())]));
        let mut t = i18n::tp("rs.turnOf", &[("n", &(turn + 1).to_string())]);
        if bonus {
            t = format!("{t} · {}", i18n::t("rs.bonusRoll"));
        }
        dom::set_text("rsTurn", &t);
    } else {
        dom::set_text("rsScore", &format!("{score} / {goal}"));
        dom::set_text("rsTurn", "");
    }
    dom::set_hidden("rsTurn", !h2h);
    // F16: the Final Round is stated in words, not only by a locked lantern.
    let final_on = with_ui(|u| u.tbl.as_ref().map_or(false, |t| t.final_active()) || u.s.final_leader.is_some()).unwrap_or(false);
    dom::set_text("rsFinal", &if final_on { i18n::t("rs.finalBanner") } else { String::new() });
    dom::set_hidden("rsFinal", !final_on);
    let m = mode != Mode::Sprint && mode != Mode::Jr;
    dom::set_hidden("rsPot", !m);
    dom::set_hidden("rsWard", !m);
    if m {
        dom::set_text("rsPot", &i18n::tp("rs.pot", &[("n", &pot.to_string())]));
        dom::set_text("rsWard", &if ward { i18n::t("rs.ward") } else if ward_used { i18n::t("rs.wardUsed") } else { String::new() });
    }
    // F11: the heat pips and the multiplier they pay on the NEXT word, as
    // numerals and filled pips (I20: readable without the visuals).
    let (heat, heat_on) = with_ui(|u| (u.s.heat, u.s.rules.heat)).unwrap_or((0, false));
    if heat_on && m {
        let (n, d) = crate::randospell::payout::heat_factor(heat);
        let mult_txt = if d == 1 { format!("×{n}") } else { format!("×{}.{}", n / d, (n * 10 / d) % 10) };
        let pips: String = (0..7).map(|i| if i < heat { '\u{25CF}' } else { '\u{25CB}' }).collect();
        dom::set_text("rsHeat", &format!("{} {} · {} {}", i18n::tp("rs.heat", &[("n", &heat.to_string())]), pips, mult_txt, ""));
    }
    let mult = streak >= crate::randospell::STREAK_FOR_MULTIPLIER;
    let streak_txt = if streak == 0 || mode == Mode::Jr {
        String::new() // Jr keeps the streak silently (F7)
    } else if mult {
        format!("{} · ×1.5", i18n::tp("rs.streak", &[("n", &streak.to_string())]))
    } else {
        i18n::tp("rs.streak", &[("n", &streak.to_string())])
    };
    dom::set_text("rsStreak", &streak_txt);
    dom::set_text("rsRolls", &i18n::tp("rs.rolls", &[("n", &rolls.to_string())]));
}

/// F19: the v1.1 mechanics that are switched on. All off is v1.
fn rules_from_flags() -> Rules {
    Rules {
        heat: crate::flags::randospell_heat(),
        call: crate::flags::randospell_call(),
        stretch: crate::flags::randospell_stretch(),
        ear: crate::flags::randospell_ear(),
        doubles: crate::flags::randospell_doubles(),
        steal: crate::flags::randospell_steal(),
        lastcall: crate::flags::randospell_lastcall(),
    }
}

fn set_die(text: &str, jackpot: bool, glow: bool) {
    dom::set_text("rsDie", text);
    for (cls, on) in [("jackpot", jackpot), ("glow", glow)] {
        if on {
            dom::add_class("rsDie", cls);
        } else {
            dom::remove_class("rsDie", cls);
        }
    }
    // F14: the glow is the only sign of doubles, and it is spoken too, so the
    // state is not colour alone.
    dom::set_text("rsGlow", &if glow { i18n::t("rs.glow") } else { String::new() });
    // Restart the fade/shake by toggling a class the CSS animates.
    dom::remove_class("rsDie", "land");
    dom::after_ms(16, || dom::add_class("rsDie", "land"));
}

fn speak(word: &str, lang: &str) {
    let (w, code) = (word.to_string(), lang.to_string());
    let fallback = w.clone();
    crate::api::play_word(&w, "normal", 1.0, lang, move || speech_out::speak(&fallback, 1.0, &code));
}

/// F12: the three-segment guess under the orb, shown only where it can be set.
fn refresh_bets() {
    let st = with_ui(|u| {
        let ready = !u.spelling && !u.choosing && !u.s.deciding && u.s.ended.is_none();
        (u.s.rules.call && u.s.mode != Mode::Jr && ready, u.s.call(), u.s.rules.heat && u.s.mode != Mode::Sprint && u.s.mode != Mode::Jr)
    });
    let Some((show_call, cur, show_heat)) = st else { return };
    let seg = |c: Call, id: &str, key: &str| {
        format!(
            "<button type=\"button\" class=\"rs-btn{}\" data-c=\"{id}\" aria-pressed=\"{}\">{}</button>",
            if cur == Some(c) { " on" } else { "" },
            cur == Some(c),
            dom::escape_html(&i18n::t(key))
        )
    };
    let html = if show_call {
        format!(
            "<span class=\"rs-bet-label\">{}</span>{}{}{}",
            dom::escape_html(&i18n::t("rs.call")),
            seg(Call::Low, "low", "rs.call.low"),
            seg(Call::Seven, "seven", "rs.call.seven"),
            seg(Call::High, "high", "rs.call.high")
        )
    } else {
        String::new()
    };
    dom::set_html("rsCall", &html);
    dom::set_hidden("rsCall", !show_call);
    dom::set_hidden("rsHeat", !show_heat);
}

fn call_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-c]").ok().flatten()) else {
        return;
    };
    let pick = match el.get_attribute("data-c").as_deref() {
        Some("low") => Call::Low,
        Some("seven") => Call::Seven,
        Some("high") => Call::High,
        _ => return,
    };
    // One tap selects, a second tap on the same segment clears it (F12).
    with_ui(|u| {
        let next = if u.s.call() == Some(pick) { None } else { Some(pick) };
        u.s.set_call(next);
    });
    refresh_bets();
}

/// F13/F17: the Stretch and Ear charms, shown after the die lands and before
/// the word is drawn, with the Listen button that draws it.
/// F15: who the player could steal from on this roll, as (seat, pot). Empty
/// unless the roll is a 7 in a mode that has Steal (I18) and a pot is there to take.
fn steal_options(u: &Ui) -> Vec<(usize, u32)> {
    let opp: Vec<(usize, u32)> = match (&u.tbl, u.s.mode) {
        (Some(t), Mode::Table) => t.steal_targets(),
        (None, Mode::HeadToHead) => {
            let p = u.s.h2h_opponent_pot();
            if p >= 1 { vec![(1, p)] } else { Vec::new() }
        }
        _ => Vec::new(),
    };
    let best = opp.iter().map(|(_, p)| *p).max().unwrap_or(0);
    if u.s.steal_offered(best) { opp } else { Vec::new() }
}

fn render_charms() {
    let Some((on, stretch_ok, ear_ok, stretch_sel, ear_sel, steals, stealing)) = with_ui(|u| {
        let stealing = u.steal_target.is_some();
        (
            u.choosing,
            u.s.stretch_offered() && !stealing,
            u.s.ear_offered(u.stretch_sel) && !stealing,
            u.stretch_sel,
            u.ear_sel && u.s.ear_offered(u.stretch_sel),
            steal_options(u),
            u.steal_target,
        )
    }) else {
        return;
    };
    let charm = |id: &str, key: &str, sel: bool| {
        format!(
            "<button type=\"button\" class=\"rs-btn{}\" data-b=\"{id}\" aria-pressed=\"{sel}\">{}</button>",
            if sel { " on" } else { "" },
            dom::escape_html(&i18n::t(key))
        )
    };
    let mut html = String::new();
    if stretch_ok {
        html.push_str(&charm("stretch", "rs.stretch", stretch_sel));
    }
    if ear_ok {
        html.push_str(&charm("ear", "rs.ear", ear_sel));
    }
    // F15: a roll of 7 may be a Steal instead of a points word.
    for (seat, pot) in steals {
        let name = if seat == 0 { i18n::t("rs.you") } else { npc_name(seat) };
        let label = i18n::tp("rs.steal", &[("name", &name), ("n", &pot.to_string())]);
        html.push_str(&charm(&format!("steal:{seat}"), "", stealing == Some(seat)).replace("></button>", &format!(">{}</button>", dom::escape_html(&label))));
    }
    html.push_str(&format!("<button type=\"button\" class=\"rs-btn rs-listen\" data-b=\"listen\">{}</button>", dom::escape_html(&i18n::t("rs.listen"))));
    dom::set_html("rsCharms", &html);
    dom::set_hidden("rsCharms", !on);
}

fn charm_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-b]").ok().flatten()) else {
        return;
    };
    match el.get_attribute("data-b").as_deref() {
        Some("stretch") => {
            with_ui(|u| {
                if u.choosing && u.s.stretch_offered() {
                    u.stretch_sel = !u.stretch_sel;
                    // Ear may stop being on offer once a big Stretch is chosen.
                    if !u.s.ear_offered(u.stretch_sel) {
                        u.ear_sel = false;
                    }
                }
            });
            render_charms();
        }
        Some("ear") => {
            with_ui(|u| {
                if u.choosing && u.s.ear_offered(u.stretch_sel) {
                    u.ear_sel = !u.ear_sel;
                }
            });
            render_charms();
        }
        Some("listen") => listen(),
        Some(v) if v.starts_with("steal:") => {
            let seat = v["steal:".len()..].parse::<usize>().ok();
            with_ui(|u| {
                if u.choosing {
                    let offered = steal_options(u).iter().any(|(s, _)| Some(*s) == seat);
                    if offered {
                        // A second tap on the same target clears it.
                        u.steal_target = if u.steal_target == seat { None } else { seat };
                        if u.steal_target.is_some() {
                            u.stretch_sel = false;
                            u.ear_sel = false;
                        }
                    }
                }
            });
            render_charms();
        }
        _ => {}
    }
}

/// The choices are made: draw the word (D-R9) and begin the spelling.
fn listen() {
    let drawn = with_ui(|u| {
        if !u.choosing {
            return None;
        }
        u.choosing = false;
        if u.steal_target.is_some() {
            Some(u.s.reveal_steal())
        } else {
            Some(u.s.reveal(u.stretch_sel, u.ear_sel))
        }
    })
    .flatten();
    dom::set_hidden("rsCharms", true);
    match drawn {
        Some(Some(r)) => begin_word(&r),
        Some(None) => show_over(), // nothing left to draw (I3)
        None => {}
    }
}

/// The die has landed: shake, number, glow, haptic.
fn land_die(len: usize, doubles: bool) {
    haptics::key_tap();
    // F1: the orb shakes as the number lands; reduce-motion removes it in CSS.
    dom::add_class("rsOrb", "shake");
    dom::after_ms(450, || dom::remove_class("rsOrb", "shake"));
    let (jr, glow) = with_ui(|u| (u.s.mode == Mode::Jr, doubles && u.s.rules.doubles)).unwrap_or((false, false));
    // Jr shows the die (length - 1) and says how many letters it spells.
    let shown = if jr { len - 1 } else { len };
    set_die(&shown.to_string(), !jr && crate::randospell::is_jackpot(len as u8), glow);
    dom::set_text("rsLetters", &if jr { i18n::tp("rs.letters", &[("n", &len.to_string())]) } else { String::new() });
    dom::set_text("rsFeed", "");
}

/// The word is drawn: hear it once (or again, unless Ear), and start the clock.
fn begin_word(r: &crate::randospell::Roll) {
    let token = bump();
    let jr = with_ui(|u| {
        u.spelling = true;
        u.typed.clear();
        u.s.mode == Mode::Jr
    })
    .unwrap_or(false);
    dom::set_text("rsField", "");
    spell_panel(true);
    // F17: with Ear the word plays once and the replay control folds away.
    if r.ear {
        dom::set_disabled("rsOrb", true);
    }
    refresh_bets();
    render_hud();
    let lang = with_ui(|u| u.lang.clone()).unwrap_or_default();
    if with_ui(|u| u.s.request_audio()).unwrap_or(true) {
        speak(&r.word, &lang);
    }
    // D19: restart the visible 30 s bar and arm the timeout. Jr has no timer.
    if jr {
        return;
    }
    dom::remove_class("rsTimer", "run");
    dom::after_ms(30, || dom::add_class("rsTimer", "run"));
    dom::after_ms((TIMER_SECS * 1000) as i32, move || {
        if TOKEN.with(Cell::get) == token {
            resolve(None);
        }
    });
}

fn orb() {
    if NPC_BUSY.with(Cell::get) {
        return; // the table is playing (D10); a tap skips it instead
    }
    let Some((spelling, deciding, choosing, two_step)) =
        with_ui(|u| (u.spelling, u.s.deciding, u.choosing, (u.s.rules.stretch || u.s.rules.ear || (u.s.rules.steal && matches!(u.s.mode, Mode::Table | Mode::HeadToHead))) && u.s.mode != Mode::Jr))
    else {
        return;
    };
    if deciding {
        return; // Bank or Roll again first (F5)
    }
    if choosing {
        listen(); // the orb is also the "go" once the choices are made
        return;
    }
    if spelling {
        // Replay, unless Ear folded it away (I15).
        if let Some((w, l)) = with_ui(|u| u.s.pending().map(|r| (r.word.clone(), u.lang.clone()))).flatten() {
            if with_ui(|u| u.s.request_audio()).unwrap_or(true) {
                speak(&w, &l);
            }
        }
        return;
    }
    if two_step {
        // v1.1: the dice first, the choices, then the word (D-R9).
        let rolled = with_ui(|u| u.s.press_roll()).flatten();
        let Some(r) = rolled else {
            show_over();
            return;
        };
        land_die(r.len, r.doubles);
        with_ui(|u| {
            u.choosing = true;
            u.stretch_sel = false;
            u.ear_sel = false;
            u.steal_target = None;
        });
        refresh_bets();
        render_charms();
        render_hud();
        return;
    }
    let rolled = with_ui(|u| u.s.press()).flatten();
    let Some(r) = rolled else {
        // No unused word at that length: the session ends rather than repeat (I3).
        show_over();
        return;
    };
    land_die(r.len, r.doubles);
    begin_word(&r);
}

fn key_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-k]").ok().flatten()) else {
        return;
    };
    let Some(k) = el.get_attribute("data-k").and_then(|d| Key::parse(&d)) else { return };
    with_ui(|u| {
        if u.spelling {
            u.typed = input::press(&u.lang, &u.typed, &k);
        }
    });
    haptics::key_tap();
    let t = with_ui(|u| u.typed.clone()).unwrap_or_default();
    dom::set_text("rsField", &t);
}

fn delete() {
    with_ui(|u| {
        if u.spelling {
            u.typed = input::backspace(&u.lang, &u.typed);
        }
    });
    haptics::key_tap();
    let t = with_ui(|u| u.typed.clone()).unwrap_or_default();
    dom::set_text("rsField", &t);
}

fn submit() {
    let Some(typed) = with_ui(|u| if u.spelling { u.typed.clone() } else { String::new() }) else { return };
    if typed.is_empty() {
        return;
    }
    resolve(Some(typed));
}

/// Grade `typed` (None = the timer ran out, a miss) and move the game on.
fn resolve(typed: Option<String>) {
    let Some((word, tier, lang)) = with_ui(|u| u.s.pending().map(|r| (r.word.clone(), r.tier, u.lang.clone()))).flatten() else {
        return;
    };
    let kid = KID.with(Cell::get);
    bump();
    dom::remove_class("rsTimer", "run");
    let correct = typed.as_deref().is_some_and(|t| {
        let tier = crate::boardgame::Tier::ALL[tier.min(3)];
        crate::boardgame_grade::grade(&lang, kid, t, &word, tier)
    });
    let out = with_ui(|u| {
        u.spelling = false;
        u.typed.clear();
        if typed.is_some() { u.s.answer(correct) } else { u.s.timeout() }
    })
    .flatten();
    if out.as_ref().is_some_and(|o| o.retry) {
        // F7: a first miss replays the word and the child tries once more.
        haptics::incorrect(true);
        with_ui(|u| {
            u.spelling = true;
            u.typed.clear();
        });
        dom::set_text("rsField", "");
        dom::set_text("rsFeed", &i18n::t("rs.jrRetry"));
        speak(&word, &lang);
        render_hud();
        return;
    }
    // F15: a landed steal moves floor(half) of the target's pot to the player.
    let stolen = if out.as_ref().is_some_and(|o| o.steal == 1) {
        with_ui(|u| {
            let seat = u.steal_target;
            match (seat, u.tbl.as_mut(), u.s.mode) {
                (Some(seat), Some(t), Mode::Table) => t.resolve_human_steal(&mut u.s, seat),
                (_, None, Mode::HeadToHead) => u.s.resolve_steal_h2h(),
                _ => 0,
            }
        })
    } else {
        None
    };
    with_ui(|u| u.steal_target = None);
    dom::set_text("rsField", "");
    spell_panel(false);
    if kid {
        // Spell Jr: every line is encouraging, and a miss never shows a loss.
        if correct {
            haptics::correct();
            let pts = out.as_ref().map_or(0, |o| o.points);
            dom::set_text("rsFeed", &i18n::tp("rs.jrRight", &[("n", &pts.to_string())]));
        } else {
            haptics::incorrect(true);
            dom::set_text("rsFeed", &i18n::tp("rs.jrWord", &[("word", crate::boardgame_grade::citation(&word))]));
        }
    } else if correct {
        haptics::correct();
        let pts = out.as_ref().map_or(0, |o| o.points);
        dom::set_text("rsFeed", &i18n::tp("rs.plus", &[("n", &pts.to_string())]));
    } else {
        haptics::incorrect(false);
        dom::set_text("rsFeed", &i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))]));
    }
    if let Some(n) = stolen {
        dom::set_text("rsFeed", &i18n::tp("rs.stealHit", &[("n", &n.to_string())]));
    }
    render_hud();
    refresh_bets();
    render_table();
    if let Some(o) = &out {
        if o.steal == 2 {
            dom::set_text("rsFeed", &format!("{} {}", i18n::t("rs.stealFail"), i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))])));
        } else if !correct {
            if o.warded {
                dom::set_text("rsFeed", &format!("{} {}", i18n::t("rs.warded"), i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))])));
            } else if o.lost > 0 {
                dom::set_text("rsFeed", &format!("{} {}", i18n::tp("rs.lost", &[("n", &o.lost.to_string())]), i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))])));
            }
        }
        if o.deciding {
            decision(true);
        }
        let has_table = with_ui(|u| u.tbl.is_some()).unwrap_or(false);
        let jr = with_ui(|u| u.s.mode == Mode::Jr).unwrap_or(false);
        if !has_table && o.passed && !o.finished {
            let feed = dom::el("rsFeed").text_content().unwrap_or_default();
            show_hand(&feed);
        }
        // F10: a miss or a Bank hands the turn on; Jr's racer answers every roll.
        if has_table && !o.finished && (o.passed || jr) {
            table_run(true);
        }
    }
    if out.is_some_and(|o| o.finished) {
        with_ui(|u| {
            if let Some(t) = u.tbl.as_mut() {
                t.settle(&u.s);
            }
        });
        show_over();
    }
}

/// F8: the hand-off card between two people on one phone. The turn has
/// already moved on, so the card names the player who plays next.
fn show_hand(note: &str) {
    let Some(n) = with_ui(|u| u.s.turn + 1) else { return };
    let line = i18n::tp("rs.handoff", &[("n", &n.to_string())]);
    let text = if note.is_empty() { line } else { format!("{note}\n{line}") };
    dom::set_text("rsHandText", &text);
    dom::set_hidden("rsHand", false);
}

/// F5: bank the pot. The only way a Marathon ends (D15).
fn bank_it() {
    let b = with_ui(|u| u.s.bank()).flatten();
    let Some(b) = b else { return };
    haptics::correct();
    decision(false);
    dom::set_text("rsFeed", &i18n::tp("rs.banked", &[("n", &b.banked.to_string())]));
    render_hud();
    refresh_bets();
    let has_table = with_ui(|u| u.tbl.is_some()).unwrap_or(false);
    if b.finished {
        with_ui(|u| {
            if let Some(t) = u.tbl.as_mut() {
                t.settle(&u.s);
            }
        });
        show_over();
    } else if b.passed {
        if has_table {
            // F16: a Bank to the goal opens the Final Round for everyone else.
            if b.final_round {
                let opened = with_ui(|u| u.tbl.as_mut().and_then(|t| t.human_banked_goal(&u.s))).flatten();
                if let Some(step) = opened {
                    dom::set_text("rsFeed", &step_text(&step));
                }
            }
            table_run(true);
        } else if b.final_round {
            // Head-to-Head: the other player has exactly one final turn.
            let me = with_ui(|u| u.s.score_of(1 - u.s.turn)).unwrap_or(0);
            show_hand(&format!("{} {}", i18n::tp("rs.finalRound", &[("name", &i18n::tp("rs.playerN", &[("n", &with_ui(|u| 2 - u.s.turn).unwrap_or(1).to_string())]))]), i18n::tp("rs.finalTurn", &[("n", &me.to_string())])));
        } else {
            show_hand(&i18n::tp("rs.banked", &[("n", &b.banked.to_string())]));
        }
    }
}

/// F5: keep the pot at risk and roll again.
fn roll_again() {
    if with_ui(|u| u.s.roll_again()).unwrap_or(false) {
        decision(false);
        refresh_bets();
        // With the guess on offer the player gets a beat to place it; otherwise
        // rolling again rolls (v1).
        let wait = with_ui(|u| u.s.rules.call && u.s.mode != Mode::Jr).unwrap_or(false);
        if !wait {
            orb();
        }
    }
}

fn finish_record() {
    let _ = with_ui(|u| {
        if u.recorded || u.s.served().is_empty() {
            return;
        }
        u.recorded = true;
        let mut led = load_ledger();
        led.record(&ledger_key(&u.lang), u.s.served(), u.day, 0);
        storage::set_json(crate::wordsearch_ui::LEDGER_KEY, &led);
    });
}

/// Fold a finished game into the mode-local store (F9), once.
fn store_result() {
    let _ = with_ui(|u| {
        if u.stored {
            return;
        }
        u.stored = true;
        let mut st = load_store();
        if let Some(k) = &u.daily_key {
            st.finish_daily(k, &u.s, true);
        }
        st.record_game(&u.s);
        save_store(&st);
    });
}

fn share_daily() {
    let text = with_ui(|u| {
        let acc = if u.s.rolls > 0 { u.s.correct * 100 / u.s.rolls } else { 0 };
        i18n::tp(
            "rs.dailyShare",
            &[
                ("date", &u.date),
                ("lang", &u.lang.to_uppercase()),
                ("tier", &i18n::t(&format!("level.{}", crate::consts::TIER_ORDER[u.tier.min(3)]))),
                ("n", &u.s.score.to_string()),
                ("r", &u.s.rolls.to_string()),
                ("acc", &acc.to_string()),
                ("b", &u.s.largest_banked.to_string()),
            ],
        )
    })
    .or_else(|| {
        // From the setup card: today's stored result for the selected tier.
        let lang = LANG.with(|l| l.borrow().clone());
        let tier = TIER.with(Cell::get);
        let e = load_store().daily_entry(&crate::randospell::daily::key(&today(), &lang, tier)).cloned()?;
        let acc = if e.rolls > 0 { e.correct * 100 / e.rolls } else { 0 };
        Some(i18n::tp(
            "rs.dailyShare",
            &[
                ("date", &today()),
                ("lang", &lang.to_uppercase()),
                ("tier", &tier_label(tier)),
                ("n", &e.score.to_string()),
                ("r", &e.rolls.to_string()),
                ("acc", &acc.to_string()),
                ("b", &e.best_pot.to_string()),
            ],
        ))
    });
    if let Some(t) = text {
        crate::share::share_text(&t);
    }
}

fn show_over() {
    finish_record();
    store_result();
    bump();
    speech_out::stop();
    let (score, rolls, correct, longest, hist, goal_met, mode, best_bank, best_lost, ward_used) = with_ui(|u| {
        (
            u.s.score,
            u.s.rolls,
            u.s.correct,
            u.s.longest.clone(),
            u.s.histogram,
            u.s.ended == Some(crate::randospell::End::Goal),
            u.s.mode,
            u.s.largest_banked,
            u.s.largest_lost,
            u.s.ward_used,
        )
    })
    .unwrap_or((0, 0, 0, None, [0; 11], false, Mode::Sprint, 0, 0, false));
    let goal = mode.goal();
    let h2h = mode == Mode::HeadToHead;
    let jr = mode == Mode::Jr;
    let best_streak = with_ui(|u| u.s.best_streak).unwrap_or(0);
    let (winner, scores, kind) = with_ui(|u| (u.s.winner, [u.s.score_of(0), u.s.score_of(1)], u.kind)).unwrap_or((None, [0, 0], Kind::Sprint));
    let table_result = with_ui(|u| {
        u.tbl.as_ref().map(|t| {
            let seats: Vec<(usize, u32)> = (0..t.seats()).map(|i| (i, t.score_of(&u.s, i))).collect();
            (t.winner, seats)
        })
    })
    .flatten();
    let title = if let Some((w, _)) = &table_result {
        match w {
            Some(0) => i18n::t("rs.youWin"),
            Some(s) => i18n::tp("rs.nameWins", &[("name", &npc_name(*s))]),
            None => i18n::t("rs.draw"),
        }
    } else if h2h {
        match winner {
            Some(w) => i18n::tp("rs.winner", &[("n", &(w + 1).to_string())]),
            None => i18n::t("rs.draw"),
        }
    } else if jr {
        i18n::t("rs.jrDone")
    } else {
        i18n::t(if mode == Mode::Marathon { "rs.overM" } else { "rs.over" })
    };
    dom::set_text("rsOverTitle", &title);
    let acc = if rolls > 0 { correct * 100 / rolls } else { 0 };
    let mut stats = vec![
        // The Sprint metric: "50 points in N rolls".
        if goal_met {
            i18n::tp("rs.result", &[("goal", &goal.to_string()), ("n", &rolls.to_string())])
        } else {
            i18n::tp("rs.score", &[("n", &score.to_string())])
        },
        i18n::tp("rs.acc", &[("n", &acc.to_string())]),
    ];
    if let Some(w) = longest {
        stats.push(i18n::tp("rs.longest", &[("word", crate::boardgame_grade::citation(&w))]));
    }
    if jr {
        stats.clear();
        stats.push(if goal_met {
            i18n::tp("rs.result", &[("goal", &goal.to_string()), ("n", &rolls.to_string())])
        } else {
            i18n::tp("rs.score", &[("n", &score.to_string())])
        });
        stats.push(i18n::tp("rs.jrCount", &[("n", &correct.to_string())]));
        stats.push(i18n::tp("rs.bestStreak", &[("n", &best_streak.to_string())]));
    }
    if let Some((_, seats)) = &table_result {
        // F10: every seat's banked score, then the human's own line.
        let mut lines: Vec<String> = seats
            .iter()
            .map(|(i, s)| format!("{}: {s}", if *i == 0 { i18n::t("rs.you") } else { npc_name(*i) }))
            .collect();
        lines.push(i18n::tp("rs.rolls", &[("n", &rolls.to_string())]));
        stats = lines;
    }
    if h2h {
        stats.clear();
        stats.push(i18n::tp("rs.h2hScore", &[("a", &scores[0].to_string()), ("b", &scores[1].to_string())]));
        stats.push(i18n::tp("rs.rolls", &[("n", &rolls.to_string())]));
    }
    if mode == Mode::Marathon {
        stats.push(i18n::tp("rs.bestBank", &[("n", &best_bank.to_string())]));
        stats.push(i18n::tp("rs.bestLost", &[("n", &best_lost.to_string())]));
        stats.push(i18n::t(if ward_used { "rs.wardUsed" } else { "rs.wardKept" }));
    }
    let list: String = stats.iter().map(|s| format!("<li>{}</li>", dom::escape_html(s))).collect();
    dom::set_html("rsStats", &list);
    // D20: the histogram is on every result card.
    let hist: Vec<u32> = if jr { hist[..6].to_vec() } else { hist.to_vec() };
    let max = hist.iter().copied().max().unwrap_or(0).max(1);
    let bars: String = hist
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                "<div class=\"rs-bar\" data-roll=\"{r}\" data-n=\"{n}\"><span>{r}</span><i style=\"width:{w}%\"></i><b>{n}</b></div>",
                r = if jr { i + 1 } else { i + 2 },
                w = n * 100 / max
            )
        })
        .collect();
    dom::set_html("rsHist", &bars);
    spell_panel(false);
    decision(false);
    dom::set_hidden("rsHand", true);
    // F6: a Daily has one attempt, so there is no "play again" on its card.
    dom::set_hidden("rsAgain", kind == Kind::Daily);
    dom::set_hidden("rsShare", !(kind == Kind::Daily && crate::share::available()));
    dom::set_hidden("rsOver", false);
}

// ---------------------------------------------------------------------- seam

/// Test builds only: where the game stands, including the word a player owes
/// (it is heard, never shown, so a browser test cannot read it off the screen).
#[cfg(feature = "testseam")]
pub fn seam_state() -> String {
    UI.with(|u| {
        u.borrow()
            .as_ref()
            .map(|u| {
                let r = u.s.pending();
                serde_json::json!({
                    "spelling": u.spelling,
                    "mode": format!("{:?}", u.s.mode),
                    "kind": format!("{:?}", u.kind),
                    "table": u.tbl.as_ref().map(|t| serde_json::json!({
                        "turn": t.turn,
                        "winner": t.winner,
                        "scores": (0..t.seats()).map(|i| t.score_of(&u.s, i)).collect::<Vec<_>>(),
                        "pots": std::iter::once(u.s.pot).chain(t.npcs.iter().map(|n| n.pot)).collect::<Vec<_>>(),
                        "people": t.npcs.iter().map(|n| n.personality.id()).collect::<Vec<_>>(),
                        "accMilli": t.npc_acc_milli(&u.s),
                        "busy": NPC_BUSY.with(Cell::get),
                        "finalActive": t.final_active(),
                        "finalLeader": t.final_leader,
                        "finalPending": t.final_pending(),
                    })),
                    "stealOptions": steal_options(u).iter().map(|(s, p)| vec![*s as u32, *p]).collect::<Vec<_>>(),
                    "heat": u.s.heat,
                    "call": u.s.call().map(|c| format!("{c:?}")),
                    "choosing": u.choosing,
                    "stretchOffered": u.s.stretch_offered(),
                    "earOffered": u.s.ear_offered(u.stretch_sel),
                    "ear": r.map(|r| r.ear),
                    "stretch": r.map(|r| r.stretch),
                    "wordTier": r.map(|r| r.tier),
                    "doubles": r.map(|r| r.doubles),
                    "events": format!("{:?}", u.s.events),
                    "jr": u.s.mode == Mode::Jr,
                    "bestStreak": u.s.best_streak,
                    "dailyKey": u.daily_key,
                    "turn": u.s.turn,
                    "scores": [u.s.score_of(0), u.s.score_of(1)],
                    "bonus": u.s.bonus,
                    "winner": u.s.winner,
                    "pot": u.s.pot,
                    "ward": u.s.ward,
                    "deciding": u.s.deciding,
                    "bestBank": u.s.largest_banked,
                    "bestLost": u.s.largest_lost,
                    "word": r.map(|r| r.word.clone()),
                    "roll": r.map(|r| r.len),
                    "rolled": u.s.rolled().map(|r| r.len),
                    "face": r.map(|r| r.face),
                    "score": u.s.score,
                    "rolls": u.s.rolls,
                    "streak": u.s.streak,
                    "ended": u.s.ended.map(|e| format!("{e:?}")),
                    "histogram": u.s.histogram,
                    "served": u.s.served(),
                    "lang": u.lang,
                    "tier": u.tier,
                })
                .to_string()
            })
            .unwrap_or_default()
    })
}

/// Test builds only: put a pot in front of an NPC so a Steal can be tried (F15).
#[cfg(feature = "testseam")]
pub fn seam_npc_pot(seat: usize, n: u32) {
    with_ui(|u| {
        if let Some(t) = u.tbl.as_mut() {
            if seat >= 1 && seat <= t.npcs.len() {
                t.npcs[seat - 1].pot = n;
            }
        }
    });
}

/// Test builds only: set the score, so a Sprint can be driven to its end
/// without playing fifty points (A7).
#[cfg(feature = "testseam")]
pub fn seam_score(n: u32) {
    with_ui(|u| u.s.set_score_for_test(n));
}

#[cfg(test)]
mod tests {
    /// The mode adds no telemetry (I9 of the spec's isolation, and the pattern
    /// Board Game's I15 set).
    #[test]
    fn the_screen_calls_no_telemetry_and_touches_no_climb_state() {
        let src = include_str!("randospell_screen.rs");
        let code: String = src.split("#[cfg(test)]").next().unwrap().lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for banned in ["telemetry::", "set_mode", "schema::Mode", "climb::", "shield", "crate::daily::"] {
            assert!(!code.contains(banned), "randospell_screen.rs uses {banned}");
        }
    }
}
