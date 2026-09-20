//! CC-WORDGRID Phase B — the Spell Cross screen (app only).
//!
//! The rules live in `crate::wordcross`; this file draws them and routes taps.
//! Clues are heard, never read (D1, F-C1): a clue row is its number, its
//! length and a play button. A word checks when its last cell is filled
//! (F-C5), and the result goes to the base game's own stats (F-X6) — the same
//! learner log a typed answer in the base game writes, with a trap spelling on
//! the trap channel. No shield is read or written (D3).

use std::cell::RefCell;

use wasm_bindgen::JsCast;

use crate::wordcross::layout::Grid;
use crate::wordcross::serve::{self, params, Cross};
use crate::wordsearch::gen::Tier;
use crate::wordsearch::ledger::Ledger;
use crate::wordsearch::lexicon::{eligible, graphemes, tiers_for};
use crate::wordsearch::{lock_in_attempt, trap_attempt};
use crate::wordsearch_ui::LEDGER_KEY;
use crate::{dom, i18n, i18n::t, App};

const DAY_MS: f64 = 86_400_000.0;
const STARS: u32 = 3;

enum Phase {
    Solve,
    /// F-C6: the grid is hidden and the keystone is spelled.
    Keystone { typed: String, missed: bool },
    Done { stars: u32 },
}

struct Game {
    lang: String,
    daily: bool,
    tier: Tier,
    grid: Grid,
    /// What the player has typed in each cell.
    entries: Vec<Option<String>>,
    solved: Vec<bool>,
    wrong: Vec<usize>,
    /// The word being answered, and whether the caret runs across.
    at: usize,
    across: bool,
    cell: usize,
    misses: u32,
    phase: Phase,
    /// The list this came from, for "another one".
    source: Option<String>,
}

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn today() -> u32 {
    (js_sys::Date::now() / DAY_MS) as u32
}

fn ledger() -> Ledger {
    crate::storage::get_json(LEDGER_KEY).unwrap_or_default()
}

pub fn playable(lang: &str) -> bool {
    eligible(lang)
}

fn tier_label(tier: Tier) -> String {
    if tier == Tier::Jr {
        t("ws.jr")
    } else {
        t(&format!("level.{}", tier.id()))
    }
}

fn note(s: &str) {
    dom::set_text("xwNote", s);
}

fn fill_picker(kid: bool, lang: &str) {
    let opts: String = tiers_for(lang, kid)
        .into_iter()
        .map(|t| format!("<option value=\"{}\">{}</option>", t.id(), dom::escape_html(&tier_label(t))))
        .collect();
    dom::set_html("xwPick", &opts);
}

fn picked_tier(kid: bool, lang: &str) -> Tier {
    Tier::from_id(&dom::select("xwPick").value())
        .filter(|t| tiers_for(lang, kid).contains(t))
        .or_else(|| tiers_for(lang, kid).first().copied())
        .unwrap_or(Tier::Jr)
}

fn start(app: &App, c: Cross, source: Option<String>) {
    start_as(app, c, source, false)
}

fn start_as(app: &App, c: Cross, source: Option<String>, daily: bool) {
    let cells = c.grid.cells.len();
    let mut led = ledger();
    let mut words: Vec<String> = c.grid.words.iter().map(|p| p.word.clone()).collect();
    if let Some(k) = &c.grid.keystone {
        words.push(k.clone()); // F-C6: the keystone obeys F-X3 too
    }
    match &source {
        Some(_) => led.counter += 1,
        None => led.record(&c.key, &words, today(), 0),
    }
    let _ = daily;
    crate::storage::set_json(LEDGER_KEY, &led);
    let first = c.grid.words.first().map(|p| (p.cells[0], p.across)).unwrap_or((0, true));
    GAME.with(|g| {
        *g.borrow_mut() = Some(Game {
            lang: app.borrow().lang.clone(),
            daily,
            tier: c.tier,
            grid: c.grid,
            entries: vec![None; cells],
            solved: vec![false; 32],
            wrong: Vec::new(),
            at: 0,
            across: first.1,
            cell: first.0,
            misses: 0,
            phase: Phase::Solve,
            source,
        })
    });
    dom::toggle_class("xwFallback", "btn-hide", true);
    note("");
    render();
}

/// D2 / F-C7: a list that will not interlock is offered as a Spell Search
/// instead, with one button.
fn offer_search(app: &App, list: Option<String>) {
    let _ = app;
    LIST.with(|c| *c.borrow_mut() = list);
    dom::set_html("xwGrid", "");
    dom::set_html("xwClues", "");
    dom::toggle_class("xwFallback", "btn-hide", false);
    note(&t("xw.better"));
}

thread_local! {
    static LIST: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Phase C: the Daily Spell Cross, the same for everyone on this date.
fn serve_daily(app: &App) -> bool {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    let tier = if kid { Tier::Jr } else { Tier::Easy };
    let d = js_sys::Date::new_0();
    let ymd = d.get_full_year() * 10_000 + (d.get_month() + 1) * 100 + d.get_date();
    match serve::daily(&lang, tier, ymd, today()) {
        Some(c) => {
            start_as(app, c, None, true);
            true
        }
        None => false,
    }
}

fn serve_bank(app: &App, tier: Tier) -> bool {
    let lang = app.borrow().lang.clone();
    match serve::bank(&lang, tier, &ledger(), today()) {
        Some(c) => {
            start(app, c, None);
            true
        }
        None => {
            offer_search(app, None);
            false
        }
    }
}

/// F-X1: "Make a crossword" on a My Words list.
pub fn open_list(app: &App, list_id: &str) -> bool {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    if !playable(&lang) {
        dom::show_toast(&t("ws.avail"));
        return false;
    }
    let lists = crate::word_lists::load();
    let Some(list) = lists.lists.iter().find(|l| l.id == list_id) else { return false };
    let words: Vec<String> = list.entries.iter().filter(|e| e.lang == lang).map(|e| e.text.clone()).collect();
    let tier = if kid { Tier::Jr } else { Tier::Easy };
    fill_picker(kid, &lang);
    dom::select("xwPick").set_value(tier.id());
    dom::add_class("xwScreen", "show");
    match serve::from_list(&lang, tier, list_id, &words, ledger().counter) {
        Some(c) => {
            start(app, c, Some(list_id.to_string()));
            true
        }
        None => {
            offer_search(app, Some(list_id.to_string()));
            true
        }
    }
}

fn word_cells(g: &Game, i: usize) -> Vec<usize> {
    g.grid.words[i].cells.clone()
}

/// The word a cell belongs to, in the wanted direction if there is one.
fn word_at(g: &Game, cell: usize, across: bool) -> Option<usize> {
    g.grid
        .words
        .iter()
        .position(|p| p.across == across && p.cells.contains(&cell))
        .or_else(|| g.grid.words.iter().position(|p| p.cells.contains(&cell)))
}

fn render() {
    GAME.with(|c| {
        let gb = c.borrow();
        let Some(g) = gb.as_ref() else { return };
        let grid = &g.grid;
        let solving = matches!(g.phase, Phase::Solve);
        dom::toggle_class("xwSolve", "btn-hide", !solving);
        dom::toggle_class("xwKey", "btn-hide", !matches!(g.phase, Phase::Keystone { .. }));
        dom::toggle_class("xwDone", "btn-hide", !matches!(g.phase, Phase::Done { .. }));
        dom::toggle_class("xwKeys", "btn-hide", matches!(g.phase, Phase::Done { .. }));
        dom::toggle_class("xwDaily", "on", g.daily);

        let here = word_at(g, g.cell, g.across);
        let lit: Vec<usize> = here.map(|i| word_cells(g, i)).unwrap_or_default();
        let mut html = String::new();
        for i in 0..grid.cells.len() {
            let Some(_) = &grid.cells[i] else {
                html.push_str("<div class=\"xw-cell blank\"></div>");
                continue;
            };
            let mut cls = String::from("xw-cell");
            if lit.contains(&i) {
                cls.push_str(" lit");
            }
            if g.cell == i {
                cls.push_str(" here");
            }
            if g.wrong.contains(&i) {
                cls.push_str(" wrong");
            }
            if grid.shaded.contains(&i) {
                cls.push_str(" shaded");
            }
            let num = grid.words.iter().find(|p| p.cells[0] == i).map(|p| p.number);
            let letter = g.entries[i].clone().unwrap_or_default();
            html.push_str(&format!(
                "<div class=\"{cls}\" data-xw-cell=\"{i}\">{}{}</div>",
                num.map(|n| format!("<small>{n}</small>")).unwrap_or_default(),
                dom::escape_html(&letter)
            ));
        }
        dom::set_html("xwGrid", &html);
        let _ = dom::el("xwGrid").set_attribute("style", &format!("--xw-w:{}", grid.w));

        // F-C1: a clue is a number, a length and a play button. Never text.
        let mut clues = String::new();
        for (dir, across) in [(t("xw.across"), true), (t("xw.down"), false)] {
            clues.push_str(&format!("<div class=\"xw-head\">{}</div>", dom::escape_html(&dir)));
            for (i, p) in grid.words.iter().enumerate().filter(|(_, p)| p.across == across) {
                let done = g.solved[i];
                clues.push_str(&format!(
                    "<div class=\"xw-clue{}{}\" data-xw-clue=\"{i}\"><b>{}</b>\
                     <button type=\"button\" class=\"ws-play\" data-xw-say=\"{i}\" aria-label=\"{}\">\u{25B6}</button>\
                     <span class=\"xw-len\">{}</span></div>",
                    if done { " done" } else { "" },
                    if here == Some(i) { " at" } else { "" },
                    p.number,
                    dom::escape_html(&t("ws.play").replace("{n}", &p.number.to_string())),
                    "_".repeat(p.cells.len())
                ));
            }
        }
        dom::set_html("xwClues", &clues);

        let keys: String = crate::wordsearch_ui::layout(&g.lang)
            .into_iter()
            .map(|row| {
                let ks: String = row
                    .iter()
                    .map(|k| format!("<button type=\"button\" class=\"kb-key\" data-xw-key=\"{0}\">{0}</button>", dom::escape_html(k)))
                    .collect();
                format!("<div class=\"sd-row\">{ks}</div>")
            })
            .collect::<String>()
            + "<div class=\"sd-row\"><button type=\"button\" class=\"kb-key wide\" data-xw-back>\u{232B}</button></div>";
        dom::set_html("xwKeys", &keys);

        match &g.phase {
            Phase::Keystone { typed, .. } => dom::set_text("xwTyped", typed),
            Phase::Done { stars } => {
                let s: String = (0..STARS).map(|i| if i < *stars { '\u{2605}' } else { '\u{2606}' }).collect();
                dom::set_text("xwStars", &s);
            }
            Phase::Solve => {}
        }
    });
}

fn speak_word(lang: &str, word: &str) {
    crate::api::play_word(word, "normal", 1.0, lang, || note(&t("ws.audioOff")));
}

fn say(i: usize) {
    let said = GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let g = gb.as_mut()?;
        let p = g.grid.words.get(i)?;
        g.at = i;
        g.across = p.across;
        g.cell = p.cells[0];
        Some((g.lang.clone(), p.word.clone()))
    });
    if let Some((lang, word)) = said {
        speak_word(&lang, &word);
    }
    render();
}

/// F-C5: a word checks when its last cell is filled. Below Hard a wrong letter
/// may flash as it is typed; at Hard and Expert nothing is checked until then.
fn type_key(k: &str) {
    let mut finished: Option<(usize, String, String)> = None;
    GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        match &mut g.phase {
            Phase::Keystone { typed, .. } => {
                if typed.chars().count() < 24 {
                    typed.push_str(k);
                }
                return;
            }
            Phase::Done { .. } => return,
            Phase::Solve => {}
        }
        let Some(i) = word_at(g, g.cell, g.across) else { return };
        g.wrong.clear();
        g.entries[g.cell] = Some(k.to_string());
        if params(g.tier).letter_check && g.grid.cells[g.cell].as_deref() != Some(k) {
            g.wrong.push(g.cell);
        }
        let cells = word_cells(g, i);
        // The caret moves on one cell, as a crossword's does, so typing a word
        // straight through works even where a crossing is already filled.
        if let Some(&n) = cells.iter().find(|&&c| c > g.cell) {
            g.cell = n;
        }
        if cells.iter().all(|&c| g.entries[c].is_some()) {
            let typed: String = cells.iter().filter_map(|&c| g.entries[c].clone()).collect();
            finished = Some((i, typed, g.grid.words[i].word.clone()));
        }
    });
    if let Some((i, typed, want)) = finished {
        check_word(i, &typed, &want);
    }
    render();
}

fn check_word(i: usize, typed: &str, want: &str) {
    let mut all_done = false;
    let mut keystone: Option<(String, String)> = None;
    GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        let correct = crate::norm::answer_matches(typed, want, false);
        let cells = word_cells(g, i);
        if correct {
            g.solved[i] = true;
            note(&t("ws.found"));
        } else {
            g.misses += 1;
            // F-C5: only the wrong cells flash.
            g.wrong = cells
                .iter()
                .enumerate()
                .filter(|(k, &cell)| g.entries[cell].as_deref() != graphemes(want).get(*k).map(String::as_str))
                .map(|(_, &cell)| cell)
                .collect();
            for &cell in &g.wrong {
                g.entries[cell] = None;
            }
            note(&t("xw.notQuite"));
        }
        // F-X6: the base game's own log, with a trap spelling on the trap
        // channel, exactly as the same misspelling would be recorded there.
        let day = crate::learner::day_now();
        let trap = !correct && crate::wordsearch::confusion::decoys(&g.lang, want).iter().any(|d| d == typed);
        let a = if trap {
            trap_attempt(&g.lang, want, typed, day)
        } else {
            lock_in_attempt(&g.lang, want, typed, correct, day)
        };
        crate::learner::note_built(&g.lang, a);
        if !correct {
            APP.with(|ap| {
                if let Some(app) = ap.borrow().as_ref() {
                    crate::misses::add_miss(&mut app.borrow_mut(), want, &g.lang, g.tier.bank_tier());
                }
            });
        }
        if g.grid.words.iter().enumerate().all(|(k, _)| g.solved[k]) {
            all_done = true;
            match g.grid.keystone.clone() {
                Some(k) => {
                    g.phase = Phase::Keystone { typed: String::new(), missed: false };
                    keystone = Some((g.lang.clone(), k));
                }
                None => g.phase = Phase::Done { stars: STARS.saturating_sub(g.misses) },
            }
        }
    });
    if all_done {
        if let Some((lang, word)) = keystone {
            note(&t("xw.keystone"));
            speak_word(&lang, &word);
        } else {
            note(&t("ws.done"));
        }
    }
    render();
}

fn submit_keystone() {
    let mut done = false;
    GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        let Some(word) = g.grid.keystone.clone() else { return };
        let Phase::Keystone { typed, missed } = &mut g.phase else { return };
        if typed.is_empty() {
            return;
        }
        let correct = crate::norm::answer_matches(typed, &word, false);
        crate::learner::note_built(&g.lang, lock_in_attempt(&g.lang, &word, typed, correct, crate::learner::day_now()));
        if !correct {
            *missed = true;
        }
        let lost = g.misses + u32::from(*missed);
        note(&if correct { t("ws.right") } else { t("ws.wrong").replace("{word}", &word) });
        g.phase = Phase::Done { stars: STARS.saturating_sub(lost) };
        done = true;
    });
    if done {
        render();
    }
}

fn again(app: &App) {
    let (lang, kid, source) = match GAME.with(|c| c.borrow().as_ref().map(|g| (g.lang.clone(), false, g.source.clone()))) {
        Some(x) => x,
        None => (app.borrow().lang.clone(), app.borrow().kid, None),
    };
    let _ = kid;
    match source {
        Some(id) => {
            open_list(app, &id);
        }
        None => {
            let kid = app.borrow().kid;
            serve_bank(app, picked_tier(kid, &lang));
        }
    }
}

pub fn open(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    if !playable(&lang) {
        return;
    }
    fill_picker(kid, &lang);
    let tier = picked_tier(kid, &lang);
    dom::select("xwPick").set_value(tier.id());
    dom::add_class("xwScreen", "show");
    serve_bank(app, tier);
}

fn close() {
    dom::remove_class("xwScreen", "show");
}

pub fn wire(app: &App) {
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));
    {
        let a = app.clone();
        dom::on_click("xwOpenBtn", move || open(&a));
    }
    dom::on_click("xwExit", close);
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("xwPick", "change", move |_| {
            let (lang, kid) = {
                let s = a.borrow();
                (s.lang.clone(), s.kid)
            };
            serve_bank(&a, picked_tier(kid, &lang));
        });
    }
    {
        let a = app.clone();
        dom::on_click("xwNew", move || again(&a));
    }
    {
        let a = app.clone();
        dom::on_click("xwDaily", move || {
            serve_daily(&a);
        });
    }
    {
        let a = app.clone();
        dom::on_click("xwNext", move || again(&a));
    }
    {
        // D2 / F-C7: the one button on the fallback.
        let a = app.clone();
        dom::on_click("xwToSearch", move || {
            close();
            match LIST.with(|c| c.borrow().clone()) {
                Some(id) => {
                    crate::wordsearch_ui::open_list(&a, &id);
                }
                None => crate::wordsearch_ui::open(&a),
            }
        });
    }
    dom::on_click("xwKeyPlay", || {
        let said = GAME.with(|c| {
            let gb = c.borrow();
            let g = gb.as_ref()?;
            g.grid.keystone.clone().map(|k| (g.lang.clone(), k))
        });
        if let Some((lang, word)) = said {
            speak_word(&lang, &word);
        }
    });
    dom::on_click("xwKeyGo", submit_keystone);
    dom::on::<web_sys::Event, _>("xwScreen", "click", |ev| {
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target.closest("[data-xw-cell],[data-xw-clue],[data-xw-say],[data-xw-key],[data-xw-back]").ok().flatten() else {
            return;
        };
        if let Some(i) = el.get_attribute("data-xw-say").and_then(|v| v.parse::<usize>().ok()) {
            say(i);
        } else if let Some(i) = el.get_attribute("data-xw-clue").and_then(|v| v.parse::<usize>().ok()) {
            say(i);
        } else if let Some(i) = el.get_attribute("data-xw-cell").and_then(|v| v.parse::<usize>().ok()) {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    // Tapping the cell you are on turns the corner.
                    if g.cell == i {
                        g.across = !g.across;
                    }
                    g.cell = i;
                    if let Some(w) = word_at(g, i, g.across) {
                        g.at = w;
                    }
                    g.wrong.clear();
                }
            });
            render();
        } else if let Some(k) = el.get_attribute("data-xw-key") {
            type_key(&k);
        } else if el.get_attribute("data-xw-back").is_some() {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    match &mut g.phase {
                        Phase::Keystone { typed, .. } => {
                            typed.pop();
                        }
                        _ => {
                            g.entries[g.cell] = None;
                            g.wrong.clear();
                        }
                    }
                }
            });
            render();
        }
    });
}

/// Test builds only: the served crossword, so a browser test knows the answers.
#[cfg(feature = "testseam")]
pub fn seam_board() -> String {
    GAME.with(|c| {
        c.borrow()
            .as_ref()
            .map(|g| {
                serde_json::json!({
                    "w": g.grid.w,
                    "h": g.grid.h,
                    "lang": g.lang,
                    "tier": g.tier.id(),
                    "daily": g.daily,
                    "keystone": g.grid.keystone,
                    "shaded": g.grid.shaded,
                    "words": g.grid.words.iter().map(|p| serde_json::json!({
                        "word": p.word, "cells": p.cells, "across": p.across, "number": p.number,
                    })).collect::<Vec<_>>(),
                })
                .to_string()
            })
            .unwrap_or_default()
    })
}
