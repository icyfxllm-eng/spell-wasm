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
//! * F5 / D15: in Marathon a correct answer opens a Bank / Roll again card;
//!   the game ends only on a Bank that reaches the goal.
//! * A16 / F1: the die fades in; the shake is CSS and `prefers-reduced-motion`
//!   removes it. The haptic fires either way.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::boardgame_input::{self as input, Key};
use crate::experience::Experience;
use crate::randospell::bank::{self, Bank, MODE_ID};
use crate::randospell::{Mode, Session, MARATHON_GOAL, SPRINT_GOAL, TIMER_SECS};
use crate::wordsearch::ledger::Ledger;
use crate::{dom, haptics, i18n, speech_out, storage, App};

struct Ui {
    s: Session,
    lang: String,
    tier: usize,
    typed: String,
    keys: Vec<Vec<Key>>,
    /// A roll is out and its word is owed.
    spelling: bool,
    recorded: bool,
    day: u32,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    /// The tier index the setup card has selected (default Medium).
    static TIER: Cell<usize> = const { Cell::new(1) };
    /// The mode the setup card has selected. Marathon needs its own flag (F9).
    static MODE: Cell<Mode> = const { Cell::new(Mode::Sprint) };
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
    dom::on_click("rsSetupExit", close);
    dom::on_click("rsStart", start);
    dom::on_click("rsAgain", start);
    dom::on_click("rsOrb", orb);
    dom::on_click("rsDel", delete);
    dom::on_click("rsGo", submit);
    dom::on_click("rsBank", bank_it);
    dom::on_click("rsRollAgain", roll_again);
    dom::on::<web_sys::MouseEvent, _>("rsModes", "click", mode_tap);
    dom::on::<web_sys::MouseEvent, _>("rsTiers", "click", tier_tap);
    dom::on::<web_sys::MouseEvent, _>("rsKeys", "click", key_tap);
}

pub fn open(app: &App) {
    let lang = {
        let s = app.borrow();
        if s.lang == crate::consts::MINE { crate::consts::EN.to_string() } else { s.lang.clone() }
    };
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    UI.with(|u| *u.borrow_mut() = None);
    if !marathon_on() {
        MODE.with(|m| m.set(Mode::Sprint));
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
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("rsScreen", "show");
}

/// F9: Marathon is offered only with `randospell_marathon` on.
fn marathon_on() -> bool {
    crate::flags::randospell_marathon()
}

fn mode_tap(e: web_sys::MouseEvent) {
    let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("[data-m]").ok().flatten()) else {
        return;
    };
    let m = match el.get_attribute("data-m").as_deref() {
        Some("marathon") if marathon_on() => Mode::Marathon,
        _ => Mode::Sprint,
    };
    MODE.with(|c| c.set(m));
    render_setup();
}

fn tier_label(ix: usize) -> String {
    i18n::t(&format!("level.{}", crate::consts::TIER_ORDER[ix]))
}

fn render_setup() {
    let lang = LANG.with(|l| l.borrow().clone());
    let ok = bank::eligible(&lang, Experience::Standard);
    let allowed = crate::experience::allowed_tiers(Experience::Standard, MODE_ID);
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
    dom::set_html("rsTiers", &btns);
    // The picker exists only when there is a choice to make.
    let mode = MODE.with(Cell::get);
    let modes: String = if marathon_on() {
        [(Mode::Sprint, "sprint", "rs.mode.sprint"), (Mode::Marathon, "marathon", "rs.mode.marathon")]
            .iter()
            .map(|(m, id, key)| {
                format!(
                    "<button type=\"button\" class=\"rs-btn{}\" data-m=\"{id}\" aria-pressed=\"{}\">{}</button>",
                    if *m == mode { " on" } else { "" },
                    *m == mode,
                    dom::escape_html(&i18n::t(key))
                )
            })
            .collect()
    } else {
        String::new()
    };
    dom::set_html("rsModes", &modes);
    // D4: a remap is shown, never silent ("12 plays as 11").
    let info = if ok {
        let cov = Bank::for_lang(&lang, Experience::Standard).cov;
        let lines: Vec<String> = cov
            .remaps()
            .into_iter()
            .map(|(f, l)| format!("<li>{}</li>", dom::escape_html(&i18n::tp("rs.remap", &[("face", &f.to_string()), ("len", &l.to_string())]))))
            .collect();
        let how = format!("<p>{}</p>", dom::escape_html(&i18n::t(if mode == Mode::Marathon { "rs.howM" } else { "rs.how" })));
        if lines.is_empty() { how } else { format!("{how}<ul>{}</ul>", lines.concat()) }
    } else {
        String::new()
    };
    dom::set_html("rsInfo", &info);
    dom::set_text("rsNote", &if ok { String::new() } else { i18n::t("rs.soon") });
    dom::set_disabled("rsStart", !ok);
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
    if !bank::eligible(&lang, Experience::Standard) {
        return;
    }
    let tier = TIER.with(Cell::get);
    let bank = Bank::for_lang(&lang, Experience::Standard);
    let entries: Vec<String> = bank.words().map(str::to_string).collect();
    let keys = input::layout(&lang, false, &entries);
    let led = load_ledger();
    let day = day();
    let key = ledger_key(&lang);
    let avoid = led.seen.iter().filter(|s| s.k == key && led.in_window(s, day)).map(|s| s.w.clone()).collect();
    let seed = (js_sys::Date::now() as u64) ^ led.counter.wrapping_mul(0x9E37_79B9);
    let s = Session::new(seed, MODE.with(Cell::get), tier, bank, avoid);
    UI.with(|u| *u.borrow_mut() = Some(Ui { s, lang, tier, typed: String::new(), keys, spelling: false, recorded: false, day }));
    dom::set_hidden("rsSetup", true);
    dom::set_hidden("rsPlay", false);
    dom::set_hidden("rsOver", true);
    dom::add_class("rsScreen", "playing");
    render_keys();
    render_hud();
    set_die("", false);
    dom::set_text("rsFeed", "");
    spell_panel(false);
    decision(false);
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
    let Some((score, streak, rolls, mode, pot, ward, ward_used)) =
        with_ui(|u| (u.s.score, u.s.streak, u.s.rolls, u.s.mode, u.s.pot, u.s.ward, u.s.ward_used))
    else {
        return;
    };
    let goal = if mode == Mode::Marathon { MARATHON_GOAL } else { SPRINT_GOAL };
    dom::set_text("rsScore", &format!("{score} / {goal}"));
    let m = mode == Mode::Marathon;
    dom::set_hidden("rsPot", !m);
    dom::set_hidden("rsWard", !m);
    if m {
        dom::set_text("rsPot", &i18n::tp("rs.pot", &[("n", &pot.to_string())]));
        dom::set_text("rsWard", &if ward { i18n::t("rs.ward") } else if ward_used { i18n::t("rs.wardUsed") } else { String::new() });
    }
    let mult = streak >= crate::randospell::STREAK_FOR_MULTIPLIER;
    let streak_txt = if streak == 0 {
        String::new()
    } else if mult {
        format!("{} · ×1.5", i18n::tp("rs.streak", &[("n", &streak.to_string())]))
    } else {
        i18n::tp("rs.streak", &[("n", &streak.to_string())])
    };
    dom::set_text("rsStreak", &streak_txt);
    dom::set_text("rsRolls", &i18n::tp("rs.rolls", &[("n", &rolls.to_string())]));
}

fn set_die(text: &str, jackpot: bool) {
    dom::set_text("rsDie", text);
    if jackpot {
        dom::add_class("rsDie", "jackpot");
    } else {
        dom::remove_class("rsDie", "jackpot");
    }
    // Restart the fade/shake by toggling a class the CSS animates.
    dom::remove_class("rsDie", "land");
    dom::after_ms(16, || dom::add_class("rsDie", "land"));
}

fn speak(word: &str, lang: &str) {
    let (w, code) = (word.to_string(), lang.to_string());
    let fallback = w.clone();
    crate::api::play_word(&w, "normal", 1.0, lang, move || speech_out::speak(&fallback, 1.0, &code));
}

fn orb() {
    let Some((spelling, deciding)) = with_ui(|u| (u.spelling, u.s.deciding)) else { return };
    if deciding {
        return; // Bank or Roll again first (F5)
    }
    if spelling {
        if let Some((w, l)) = with_ui(|u| u.s.pending().map(|r| (r.word.clone(), u.lang.clone()))).flatten() {
            speak(&w, &l);
        }
        return;
    }
    let rolled = with_ui(|u| u.s.press()).flatten();
    let Some(r) = rolled else {
        // No unused word at that length: the session ends rather than repeat (I3).
        show_over();
        return;
    };
    haptics::key_tap();
    // F1: the orb shakes as the number lands; reduce-motion removes it in CSS.
    dom::add_class("rsOrb", "shake");
    dom::after_ms(450, || dom::remove_class("rsOrb", "shake"));
    let token = bump();
    set_die(&r.len.to_string(), crate::randospell::is_jackpot(r.len as u8));
    dom::set_text("rsFeed", "");
    with_ui(|u| {
        u.spelling = true;
        u.typed.clear();
    });
    dom::set_text("rsField", "");
    spell_panel(true);
    render_hud();
    let lang = with_ui(|u| u.lang.clone()).unwrap_or_default();
    speak(&r.word, &lang);
    // D19: restart the visible 30 s bar and arm the timeout.
    dom::remove_class("rsTimer", "run");
    dom::after_ms(30, || dom::add_class("rsTimer", "run"));
    dom::after_ms((TIMER_SECS * 1000) as i32, move || {
        if TOKEN.with(Cell::get) == token {
            resolve(None);
        }
    });
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
    bump();
    dom::remove_class("rsTimer", "run");
    let correct = typed.as_deref().is_some_and(|t| {
        let tier = crate::boardgame::Tier::ALL[tier.min(3)];
        crate::boardgame_grade::grade(&lang, false, t, &word, tier)
    });
    let out = with_ui(|u| {
        u.spelling = false;
        u.typed.clear();
        if typed.is_some() { u.s.answer(correct) } else { u.s.timeout() }
    })
    .flatten();
    dom::set_text("rsField", "");
    spell_panel(false);
    if correct {
        haptics::correct();
        let pts = out.as_ref().map_or(0, |o| o.points);
        dom::set_text("rsFeed", &i18n::tp("rs.plus", &[("n", &pts.to_string())]));
    } else {
        haptics::incorrect(false);
        dom::set_text("rsFeed", &i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))]));
    }
    render_hud();
    if let Some(o) = &out {
        if !correct {
            if o.warded {
                dom::set_text("rsFeed", &format!("{} {}", i18n::t("rs.warded"), i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))])));
            } else if o.lost > 0 {
                dom::set_text("rsFeed", &format!("{} {}", i18n::tp("rs.lost", &[("n", &o.lost.to_string())]), i18n::tp("rs.word", &[("word", crate::boardgame_grade::citation(&word))])));
            }
        }
        if o.deciding {
            decision(true);
        }
    }
    if out.is_some_and(|o| o.finished) {
        show_over();
    }
}

/// F5: bank the pot. The only way a Marathon ends (D15).
fn bank_it() {
    let b = with_ui(|u| u.s.bank()).flatten();
    let Some(b) = b else { return };
    haptics::correct();
    decision(false);
    dom::set_text("rsFeed", &i18n::tp("rs.banked", &[("n", &b.banked.to_string())]));
    render_hud();
    if b.finished {
        show_over();
    }
}

/// F5: keep the pot at risk and roll again.
fn roll_again() {
    if with_ui(|u| u.s.roll_again()).unwrap_or(false) {
        decision(false);
        orb();
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

fn show_over() {
    finish_record();
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
    let goal = if mode == Mode::Marathon { MARATHON_GOAL } else { SPRINT_GOAL };
    dom::set_text("rsOverTitle", &i18n::t(if mode == Mode::Marathon { "rs.overM" } else { "rs.over" }));
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
    if mode == Mode::Marathon {
        stats.push(i18n::tp("rs.bestBank", &[("n", &best_bank.to_string())]));
        stats.push(i18n::tp("rs.bestLost", &[("n", &best_lost.to_string())]));
        stats.push(i18n::t(if ward_used { "rs.wardUsed" } else { "rs.wardKept" }));
    }
    let list: String = stats.iter().map(|s| format!("<li>{}</li>", dom::escape_html(s))).collect();
    dom::set_html("rsStats", &list);
    // D20: the histogram is on every result card.
    let max = hist.iter().copied().max().unwrap_or(0).max(1);
    let bars: String = hist
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                "<div class=\"rs-bar\" data-roll=\"{r}\" data-n=\"{n}\"><span>{r}</span><i style=\"width:{w}%\"></i><b>{n}</b></div>",
                r = i + 2,
                w = n * 100 / max
            )
        })
        .collect();
    dom::set_html("rsHist", &bars);
    spell_panel(false);
    decision(false);
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
                    "pot": u.s.pot,
                    "ward": u.s.ward,
                    "deciding": u.s.deciding,
                    "bestBank": u.s.largest_banked,
                    "bestLost": u.s.largest_lost,
                    "word": r.map(|r| r.word.clone()),
                    "roll": r.map(|r| r.len),
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
        for banned in ["telemetry::", "set_mode", "schema::Mode", "climb::", "shield", "daily::"] {
            assert!(!code.contains(banned), "randospell_screen.rs uses {banned}");
        }
    }
}
