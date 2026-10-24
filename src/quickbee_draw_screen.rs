//! CC-QUICKBEE-DRAW v1 Phase B -- the screen: a 60 second run where the player
//! hears a word and DRAWS it one character at a time.
//!
//! Renders `quickbee_draw::run::Run` and forwards what the pad produces; the
//! rules live in `src/quickbee_draw/run.rs`. This file owns what a reducer
//! cannot: the clock, the pad, the voice, the DOM and the two stores.
//!
//! * One recogniser path: `drawing::recognize`, the Vision call every other pad
//!   uses (CC-CJK-INK I3). Ink never leaves the device and strokes are never
//!   kept (I1, I2): the pad is cleared after every cell.
//! * The pad is the app's one drawing pad. Its wrapper element is MOVED into this
//!   screen for the run and put back after, so there is one pointer wiring and one
//!   canvas, and `ink_probe` is untouched.
//! * Auto-advance has no confirm button: a poll watches the stroke count and the
//!   idle time and asks `run::ready_to_judge`.
//! * History joins the shared ledger under `quickbee_draw:{lang}` (F6, I4); the
//!   leaderboard is local, per language and labelled "drawn" (F3, I1).
//! * Nothing here touches typed Quick Bee, Climb, the Daily or shields (I5).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::quickbee_draw::run::{judge, ready_to_judge, Run, Step, Verdict, RUN_SECS};
use crate::spelldoku::rng::Rng;
use crate::wordsearch::ledger::Ledger;
use crate::{dom, haptics, i18n, speech_out, storage, App};

use crate::quickbee_draw::board::{Entry, Store, STORE_KEY};
const POLL_MS: i32 = 50;
const TICK_MS: i32 = 100;
/// More words than a minute can reach, so the pool never runs dry mid-run.
const WORDS_PER_RUN: usize = 60;

struct Ui {
    run: Run,
    /// Bank rows parallel to the run's words, for the voice (zh needs pinyin).
    rows: Vec<(String, String)>,
    lang: String,
    started_at: f64,
    judging: bool,
    last_count: usize,
    last_change: f64,
    recorded: bool,
    day: u32,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static TIER: Cell<usize> = const { Cell::new(1) };
    static CLOCK: Cell<i32> = const { Cell::new(0) };
    static POLL: Cell<i32> = const { Cell::new(0) };
    /// The wrapper's home, so it can go back where the Write-it pad expects it.
    static HOME: RefCell<Option<(web_sys::Node, Option<web_sys::Node>)>> = const { RefCell::new(None) };
    #[cfg(feature = "testseam")]
    static FORCE: RefCell<Option<Verdict>> = const { RefCell::new(None) };
}

fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|u| u.borrow_mut().as_mut().map(f))
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn tier_name(ix: usize) -> &'static str {
    crate::consts::TIER_ORDER[ix.min(3)]
}

fn load_store() -> Store {
    storage::get_json(STORE_KEY).unwrap_or_default()
}

fn ledger_key(lang: &str) -> String {
    format!("quickbee_draw:{lang}")
}

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("qbdOpenBtn", move || open(&a));
    }
    dom::on_click("qbdSetupExit", close);
    dom::on_click("qbdExit", close);
    dom::on_click("qbdOverExit", close);
    dom::on_click("qbdStart", start);
    dom::on_click("qbdAgain", start);
    dom::on_click("qbdHear", hear);
    dom::on_click("qbdClear", || {
        crate::drawing::clear_canvas();
        with_ui(|u| u.last_count = 0);
    });
    dom::on::<web_sys::MouseEvent, _>("qbdTiers", "click", tier_tap);
}

pub fn open(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    // F4/I2: absent where it is not offered. The hub already hides the tile; this
    // is the belt for a direct call.
    if !crate::quickbee_draw::available(&lang, kid, crate::flags::quickbee_draw_lang(&lang)) {
        return;
    }
    LANG.with(|l| *l.borrow_mut() = lang);
    UI.with(|u| *u.borrow_mut() = None);
    dom::set_hidden("qbdSetup", false);
    dom::set_hidden("qbdPlay", true);
    dom::set_hidden("qbdOver", true);
    render_setup();
    dom::add_class("qbdScreen", "show");
}

fn close() {
    stop_timers();
    unmount_pad();
    crate::drawing::clear_canvas();
    speech_out::stop();
    record_history();
    UI.with(|u| *u.borrow_mut() = None);
    dom::remove_class("qbdScreen", "show");
}

/// The tiers this player owns in this language (D10 of Randospell, same law):
/// Full owns every allowed tier, Preview the first, nothing else.
fn owned_tiers(lang: &str) -> Vec<usize> {
    let level = crate::play_hub::live_entitlements().lang_level(lang);
    let exp = crate::experience::Experience::Standard;
    crate::experience::allowed_tiers(exp, "quickbee_draw")
        .iter()
        .filter_map(|n| crate::consts::TIER_ORDER.iter().position(|t| t == n))
        .filter(|ix| match level {
            crate::entitlements::AccessLevel::Full => true,
            crate::entitlements::AccessLevel::Preview => crate::entitlements::preview_allows(lang, crate::entitlements::GameMode::Standard, *ix as u32 + 1),
            crate::entitlements::AccessLevel::None => false,
        })
        .collect()
}

fn render_setup() {
    let lang = LANG.with(|l| l.borrow().clone());
    let owned = owned_tiers(&lang);
    if !owned.is_empty() && !owned.contains(&TIER.with(Cell::get)) {
        TIER.with(|t| t.set(owned[0]));
    }
    let sel = TIER.with(Cell::get);
    let btns: String = owned
        .iter()
        .map(|ix| {
            format!(
                "<button type=\"button\" class=\"qbd-btn{}\" data-t=\"{ix}\" aria-pressed=\"{}\">{}</button>",
                if *ix == sel { " on" } else { "" },
                *ix == sel,
                dom::escape_html(&i18n::t(&format!("level.{}", tier_name(*ix))))
            )
        })
        .collect();
    dom::set_html("qbdTiers", &btns);
    let best = load_store().best.get(&lang).copied().unwrap_or(0);
    dom::set_text("qbdBest", &if best > 0 { i18n::tp("qbd.best", &[("n", &best.to_string())]) } else { String::new() });
    dom::set_html("qbdBoard", &board_html(&load_store(), &lang));
    dom::set_disabled("qbdStart", owned.is_empty() || crate::quickbee_draw::pool_by_cells(&lang, tier_name(sel)).iter().sum::<usize>() == 0);
}

/// F3: this device's best drawn runs for one language, labelled "drawn", and
/// nothing from any other language or from typed Quick Bee (I1, A6).
fn board_html(st: &Store, lang: &str) -> String {
    let rows = st.board(lang);
    let head = format!("<h4 class=\"qbd-board-h\">{}</h4>", dom::escape_html(&i18n::t("qbd.boardTitle")));
    if rows.is_empty() {
        return format!("{head}<p class=\"qbd-board-empty\">{}</p>", dom::escape_html(&i18n::t("qbd.boardEmpty")));
    }
    let li: String = rows
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "<li data-rank=\"{}\" data-score=\"{}\"><b>{}</b><span>{}</span><i>{}</i></li>",
                i + 1,
                e.score,
                e.score,
                dom::escape_html(&i18n::tp("qbd.words", &[("n", &e.words.to_string())])),
                dom::escape_html(&i18n::t(&format!("level.{}", e.tier)))
            )
        })
        .collect();
    format!("{head}<ol class=\"qbd-board-list\">{li}</ol>")
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

fn mount_pad() {
    let doc = dom::doc();
    let (Some(wrap), Some(host)) = (doc.get_element_by_id("inkCanvasWrap"), doc.get_element_by_id("qbdPadHost")) else { return };
    HOME.with(|h| {
        if h.borrow().is_none() {
            if let Some(parent) = wrap.parent_node() {
                *h.borrow_mut() = Some((parent, wrap.next_sibling()));
            }
        }
    });
    let _ = host.append_child(&wrap);
    crate::drawing::size_canvas();
    crate::drawing::clear_canvas();
}

fn unmount_pad() {
    let Some(wrap) = dom::doc().get_element_by_id("inkCanvasWrap") else { return };
    HOME.with(|h| {
        if let Some((parent, next)) = h.borrow_mut().take() {
            let _ = parent.insert_before(&wrap, next.as_ref());
        }
    });
}

fn stop_timers() {
    let win = dom::window();
    for c in [&CLOCK, &POLL] {
        let id = c.with(Cell::get);
        if id != 0 {
            win.clear_interval_with_handle(id);
            c.with(|x| x.set(0));
        }
    }
}

fn start() {
    stop_timers();
    unmount_pad();
    record_history();
    let lang = LANG.with(|l| l.borrow().clone());
    let tier = TIER.with(Cell::get);
    let tname = tier_name(tier);
    let rows_all = crate::quickbee_draw::pool_rows(&lang, tname);
    if rows_all.is_empty() {
        return;
    }
    // F6: fresh words first from the shared ledger; never the same word twice.
    let led: Ledger = storage::get_json(crate::wordsearch_ui::LEDGER_KEY).unwrap_or_default();
    let day = (js_sys::Date::now() / 86_400_000.0) as u32;
    let mut rng = Rng::new((js_sys::Date::now() as u64) ^ led.counter.wrapping_mul(0x9E37_79B9));
    let drawn: Vec<String> = rows_all.iter().map(|(_, d)| d.clone()).collect();
    let mut picked: Vec<String> = Vec::new();
    led.select(&ledger_key(&lang), &drawn, &mut picked, WORDS_PER_RUN.min(drawn.len()), day, &mut rng, &|_, _| true);
    let rows: Vec<(String, String)> =
        picked.iter().filter_map(|d| rows_all.iter().find(|(_, x)| x == d).cloned()).collect();
    let words: Vec<String> = rows.iter().map(|(_, d)| d.clone()).collect();
    let run = Run::new(&lang, tname, words);
    UI.with(|u| {
        *u.borrow_mut() = Some(Ui { run, rows, lang: lang.clone(), started_at: now_ms(), judging: false, last_count: 0, last_change: now_ms(), recorded: false, day })
    });
    dom::set_hidden("qbdSetup", true);
    dom::set_hidden("qbdOver", true);
    dom::set_hidden("qbdPlay", false);
    mount_pad();
    dom::set_text("qbdFeed", "");
    render();
    hear();
    let win = dom::window();
    let tick = Closure::<dyn FnMut()>::new(tick);
    let id = win.set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), TICK_MS).unwrap_or(0);
    tick.forget();
    CLOCK.with(|c| c.set(id));
    let poll = Closure::<dyn FnMut()>::new(poll);
    let id = win.set_interval_with_callback_and_timeout_and_arguments_0(poll.as_ref().unchecked_ref(), POLL_MS).unwrap_or(0);
    poll.forget();
    POLL.with(|c| c.set(id));
}

fn remaining_ms() -> i32 {
    with_ui(|u| (RUN_SECS as f64 * 1000.0 - (now_ms() - u.started_at)).max(0.0) as i32).unwrap_or(0)
}

fn tick() {
    let left = remaining_ms();
    dom::set_text("qbdTime", &format!("{}", (left + 999) / 1000));
    if let Some(e) = dom::doc().get_element_by_id("qbdBar") {
        let _ = e.set_attribute("style", &format!("width:{}%", left * 100 / (RUN_SECS as i32 * 1000)));
    }
    if left <= 0 {
        end();
    }
}

/// Watch the pad: judge a cell once the expected strokes are in and the pen rests.
fn poll() {
    let ready = with_ui(|u| {
        if u.judging || u.run.over {
            return false;
        }
        let count = crate::drawing::stroke_count();
        let t = now_ms();
        if count != u.last_count {
            u.last_count = count;
            u.last_change = t;
        }
        let expected = u.run.expected().and_then(|c| crate::stroke_counts::strokes(&c));
        ready_to_judge(&u.lang, count, expected, (t - u.last_change).max(0.0) as u32)
    })
    .unwrap_or(false);
    if ready {
        judge_cell();
    }
}

fn judge_cell() {
    let Some((lang, expected, tier)) = with_ui(|u| {
        u.judging = true;
        u.run.expected().map(|e| (u.lang.clone(), e, u.run.tier.clone()))
    })
    .flatten() else {
        return;
    };
    spawn_local(async move {
        #[cfg(feature = "testseam")]
        let forced = FORCE.with(|f| f.borrow_mut().take());
        #[cfg(not(feature = "testseam"))]
        let forced: Option<Verdict> = None;
        let verdict = match forced {
            Some(v) => v,
            None => {
                let cands = crate::drawing::recognize(&lang).await;
                judge(&expected, &cands, &tier)
            }
        };
        // The cell is read; the ink goes now (I2: strokes are never kept).
        crate::drawing::clear_canvas();
        let step = with_ui(|u| {
            u.last_count = 0;
            u.last_change = now_ms();
            if u.run.over {
                return None;
            }
            Some(u.run.apply(verdict))
        })
        .flatten();
        with_ui(|u| u.judging = false);
        let Some(step) = step else { return };
        match &step {
            Step::CellRight { .. } => {
                haptics::correct();
                dom::set_text("qbdFeed", "");
            }
            Step::WordDone { .. } => {
                haptics::correct();
                dom::set_text("qbdFeed", "");
                hear();
            }
            Step::WordEnded { expected, .. } => {
                haptics::wrong();
                dom::set_text("qbdFeed", &i18n::tp("qbd.itWas", &[("c", expected)]));
                hear();
            }
            Step::Redraw => {
                dom::set_text("qbdFeed", &i18n::t("qbd.redraw"));
            }
            Step::Over => {}
        }
        render();
        if step == Step::Over {
            end();
        }
    });
}

fn render() {
    let Some((score, cells, cell)) = with_ui(|u| (u.run.score, u.run.cell_count(), u.run.cell())) else { return };
    dom::set_text("qbdScore", &i18n::tp("qbd.score", &[("n", &score.to_string())]));
    // The cells of this word: done, current, to come. The characters themselves
    // are not shown; the word is heard.
    let dots: String = (0..cells)
        .map(|i| {
            let cls = if i < cell { "done" } else if i == cell { "cur" } else { "todo" };
            format!("<span class=\"qbd-cell {cls}\" aria-hidden=\"true\">{}</span>", if i < cell { "\u{2713}" } else { "" })
        })
        .collect();
    dom::set_html("qbdCells", &dots);
}

fn hear() {
    let Some((row, drawn, lang)) = with_ui(|u| {
        let w = u.run.word()?.to_string();
        let row = u.rows.iter().find(|(_, d)| *d == w).map(|(r, _)| r.clone()).unwrap_or_else(|| w.clone());
        Some((row, w, u.lang.clone()))
    })
    .flatten() else {
        return;
    };
    // The same voice routes every mode uses; zh is read through its pinyin so a
    // polyphonic character is not guessed.
    if lang == crate::consts::ZH {
        let (py, hz) = row.split_once('|').unwrap_or((row.as_str(), drawn.as_str()));
        // No reading means no voice: the device voice would guess the polyphone.
        if let Some(p) = crate::pinyin::phoneme_reading(py) {
            crate::api::play_word_with(hz, Some(&p), "normal", 1.0, &lang, || {});
        }
        return;
    }
    let (fallback, code) = (drawn.clone(), lang.clone());
    // zh-ok(audio): the zh branch above has returned, so Mandarin never reaches the device voice
    crate::api::play_word(&drawn, "normal", 1.0, &lang, move || speech_out::speak(&fallback, 1.0, &code));
}

fn end() {
    stop_timers();
    let Some((score, words, ended, over)) = with_ui(|u| {
        u.run.time_up();
        (u.run.score, u.run.words_done, u.run.words_ended, u.recorded)
    }) else {
        return;
    };
    if over {
        return;
    }
    with_ui(|u| u.recorded = true);
    let lang = LANG.with(|l| l.borrow().clone());
    let mut st = load_store();
    let tier = with_ui(|u| u.run.tier.clone()).unwrap_or_default();
    let day = (js_sys::Date::now() / 86_400_000.0) as u32;
    let rec = st.record(&lang, Entry { score, words, tier, day });
    let (new_best, best_now) = (rec.new_best, rec.best);
    storage::set_json(STORE_KEY, &st);
    record_history();
    unmount_pad();
    crate::drawing::clear_canvas();
    speech_out::stop();
    let mut lines = vec![
        i18n::tp("qbd.score", &[("n", &score.to_string())]),
        i18n::tp("qbd.words", &[("n", &words.to_string())]),
        i18n::tp("qbd.best", &[("n", &best_now.to_string())]),
    ];
    // F3: where this run sits on the DRAWN board for this language, and the board.
    if let Some(r) = rec.rank {
        lines.push(i18n::tp("qbd.rank", &[("n", &r.to_string())]));
    }
    if new_best && score > 0 {
        lines.push(i18n::t("qbd.drawn"));
    }
    let _ = ended;
    dom::set_html("qbdOverBoard", &board_html(&st, &lang));
    dom::set_html("qbdStats", &lines.iter().map(|l| format!("<li>{}</li>", dom::escape_html(l))).collect::<String>());
    dom::set_hidden("qbdPlay", true);
    dom::set_hidden("qbdOver", false);
}

/// F6/I4: the words this run reached go into the shared ledger, once.
fn record_history() {
    let _ = with_ui(|u| {
        let served: Vec<String> = u.run.served().to_vec();
        if served.is_empty() || u.day == u32::MAX {
            return;
        }
        let mut led: Ledger = storage::get_json(crate::wordsearch_ui::LEDGER_KEY).unwrap_or_default();
        led.record(&ledger_key(&u.lang), &served, u.day, 0);
        storage::set_json(crate::wordsearch_ui::LEDGER_KEY, &led);
        u.day = u32::MAX; // written once
    });
}

// ---------------------------------------------------------------------- seam

/// Test builds only: the web build has no native recogniser, so a test names
/// what the next cell reads as.
#[cfg(feature = "testseam")]
pub fn seam_force(v: &str) {
    let verdict = match v {
        "right" => Verdict::Right,
        "wrong" => Verdict::Wrong,
        _ => Verdict::Unsure,
    };
    FORCE.with(|f| *f.borrow_mut() = Some(verdict));
}

/// Test builds only: pretend `ms` of the run have passed, so the 60 second clock
/// can be run out in a test without waiting a minute.
#[cfg(feature = "testseam")]
pub fn seam_age(ms: u32) {
    with_ui(|u| u.started_at -= ms as f64);
}

#[cfg(feature = "testseam")]
pub fn seam_state() -> String {
    UI.with(|u| {
        u.borrow()
            .as_ref()
            .map(|u| {
                serde_json::json!({
                    "word": u.run.word(),
                    "cell": u.run.cell(),
                    "cells": u.run.cell_count(),
                    "expected": u.run.expected(),
                    "score": u.run.score,
                    "wordsDone": u.run.words_done,
                    "over": u.run.over,
                    "judging": u.judging,
                    "redraws": u.run.redraws_used,
                    "served": u.run.served(),
                    "lang": u.lang,
                    "padMoved": crate::dom::doc().get_element_by_id("qbdPadHost").map_or(false, |h| h.query_selector("#inkCanvasWrap").ok().flatten().is_some()),
                })
                .to_string()
            })
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    /// The drawn Quick Bee writes to no typed-mode, Climb, Daily or shield
    /// record, and sends ink nowhere (I5, CC-CJK-INK I1 and I2).
    #[test]
    fn the_screen_touches_no_other_mode_and_keeps_no_ink() {
        let src = include_str!("quickbee_draw_screen.rs");
        let code: String = src.split("#[cfg(test)]").next().unwrap().lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for banned in ["telemetry::", "crate::climb::", "crate::daily::", "shield", "fetch(", "to_data_url", "xhr", "set_json(\"spell_ink"] {
            assert!(!code.contains(banned), "quickbee_draw_screen.rs uses {banned}");
        }
    }
}
