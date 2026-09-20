//! CC-WORDGRID v1 Phase A — the Spell Search screen (app only).
//!
//! The rules live in `crate::wordsearch`; this file draws them and routes
//! touches. A slot shows only a word's length and a play button (F-S1): the
//! player hears the word and finds it. Everything a round records goes to the
//! base game's learner log (F-X6). There is no leaderboard, no score service,
//! and no mode-specific stats store.

use std::cell::RefCell;

use wasm_bindgen::JsCast;

use crate::wordsearch::{lock_in_attempt, trap_attempt};
use crate::wordsearch::gen::{hash, judge, path, Hit, Puzzle, Tier};
use crate::wordsearch::ledger::Ledger;
use crate::wordsearch::lexicon::{eligible, graphemes, tiers_for};
use crate::wordsearch::serve::{self, Served};
use crate::wordsearch::hint;
use crate::spelldoku::rng::Rng;
use crate::{dom, i18n::t, App};

/// The seen-word ledger (F-X3). Spell Cross will share it.
pub const LEDGER_KEY: &str = "spell_wordgrid_seen_v1";
const DAY_MS: f64 = 86_400_000.0;
const STARS: u32 = 3;

#[derive(Clone)]
enum Source {
    Bank,
    Daily,
    List(String),
}

enum Phase {
    Find,
    /// F-S4: the found words replay in a shuffled order, grid hidden.
    LockIn { order: Vec<usize>, at: usize, typed: String, misses: u32 },
    Done { stars: u32 },
}

struct Game {
    lang: String,
    kid: bool,
    source: Source,
    puzzle: Puzzle,
    found: Vec<bool>,
    /// Decoys already picked: a trap costs its star-point once.
    tripped: Vec<bool>,
    traps: u32,
    /// The slot whose word was played last: the Meaning hint is about it.
    active: Option<usize>,
    /// Where a drag began, and the tap-then-tap anchor.
    drag_from: Option<usize>,
    drag_path: Vec<usize>,
    flash: Vec<usize>,
    phase: Phase,
}

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
}

fn today() -> u32 {
    (js_sys::Date::now() / DAY_MS) as u32
}

fn ymd() -> u32 {
    let d = js_sys::Date::new_0();
    d.get_full_year() * 10_000 + (d.get_month() + 1) * 100 + d.get_date()
}

fn ledger() -> Ledger {
    crate::storage::get_json(LEDGER_KEY).unwrap_or_default()
}

/// Is Spell Search played in this language at all?
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

fn fill_picker(kid: bool, lang: &str) {
    let opts: String = tiers_for(lang, kid)
        .into_iter()
        .map(|tier| format!("<option value=\"{}\">{}</option>", tier.id(), dom::escape_html(&tier_label(tier))))
        .collect();
    dom::set_html("wsPick", &opts);
}

fn picked_tier(kid: bool, lang: &str) -> Tier {
    let v = dom::select("wsPick").value();
    Tier::from_id(&v)
        .filter(|t| tiers_for(lang, kid).contains(t))
        .or_else(|| tiers_for(lang, kid).first().copied())
        .unwrap_or(Tier::Jr)
}

/// Keys from the in-app keyboard's own layout file, as SpellDoku reads them.
pub fn layout(lang: &str) -> Vec<Vec<String>> {
    let raw = match lang {
        "es" => include_str!("../assets/keyboards/es.json"),
        "fr" => include_str!("../assets/keyboards/fr.json"),
        "de" => include_str!("../assets/keyboards/de.json"),
        "pt" => include_str!("../assets/keyboards/pt.json"),
        "pl" => include_str!("../assets/keyboards/pl.json"),
        "ru" => include_str!("../assets/keyboards/ru.json"),
        "fil" => include_str!("../assets/keyboards/fil.json"),
        _ => include_str!("../assets/keyboards/en.json"),
    };
    let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    let mut rows: Vec<Vec<String>> = v["rows"]
        .as_array()
        .map(|a| a.iter().filter_map(|r| r.as_str()).map(|r| r.chars().map(|c| c.to_string()).collect()).collect())
        .unwrap_or_default();
    let mut extras: Vec<String> = Vec::new();
    if let Some(lp) = v["longPress"].as_object() {
        for alts in lp.values() {
            for c in alts.as_str().unwrap_or("").chars() {
                let k = c.to_string();
                if !extras.contains(&k) && !rows.iter().flatten().any(|x| *x == k) {
                    extras.push(k);
                }
            }
        }
    }
    if !extras.is_empty() {
        rows.push(extras);
    }
    rows
}

fn start(app: &App, served: Served, source: Source) {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    let mut led = ledger();
    let words: Vec<String> = served.puzzle.targets.iter().map(|t| t.word.clone()).collect();
    match source {
        // My Words repeats the player's own words on purpose: only the counter
        // moves, so the next puzzle from the list lays out differently.
        Source::List(_) => led.counter += 1,
        _ => led.record(&served.key, &words, today(), served.relaxed),
    }
    crate::storage::set_json(LEDGER_KEY, &led);
    let n_t = served.puzzle.targets.len();
    let n_d = served.puzzle.decoys.len();
    GAME.with(|g| {
        *g.borrow_mut() = Some(Game {
            lang,
            kid,
            source,
            puzzle: served.puzzle,
            found: vec![false; n_t],
            tripped: vec![false; n_d],
            traps: 0,
            active: None,
            drag_from: None,
            drag_path: Vec::new(),
            flash: Vec::new(),
            phase: Phase::Find,
        })
    });
    note("");
    dom::set_html("wsMeaning", "");
    render();
}

fn serve_bank(app: &App, tier: Tier) -> bool {
    let lang = app.borrow().lang.clone();
    match serve::bank(&lang, tier, &ledger(), today()) {
        Some(s) => {
            start(app, s, Source::Bank);
            true
        }
        None => {
            // Never a dead button: say so. (The 10,000-seed sweep serves every seed.)
            note(&t("ws.avail"));
            false
        }
    }
}

fn serve_daily(app: &App) -> bool {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    // The Daily is Easy, or Jr for an under-13 player (F-X5).
    let tier = if kid { Tier::Jr } else { Tier::Easy };
    match serve::daily(&lang, tier, ymd(), today()) {
        Some(s) => {
            start(app, s, Source::Daily);
            true
        }
        None => false,
    }
}

fn note(s: &str) {
    dom::set_text("wsNote", s);
}

/// Underscores only, one per letter: the length and nothing else (F-S1).
fn masked(word: &str) -> String {
    "_".repeat(graphemes(word).len())
}

fn render() {
    GAME.with(|cell| {
        let gb = cell.borrow();
        let Some(g) = gb.as_ref() else { return };
        let p = &g.puzzle;
        let n = p.size;
        let finding = matches!(g.phase, Phase::Find);
        dom::toggle_class("wsFind", "btn-hide", !finding);
        dom::toggle_class("wsLock", "btn-hide", !matches!(g.phase, Phase::LockIn { .. }));
        dom::toggle_class("wsDone", "btn-hide", !matches!(g.phase, Phase::Done { .. }));
        dom::toggle_class("wsDaily", "on", matches!(g.source, Source::Daily));

        // F-S1: a slot is the word's length and a play button, until it is found.
        let slow = p.tier.slow_replay();
        let mut slots = String::new();
        for (i, tg) in p.targets.iter().enumerate() {
            let shown = if g.found[i] { dom::escape_html(&tg.word) } else { masked(&tg.word) };
            slots.push_str(&format!(
                "<div class=\"ws-slot{}{}\" data-ws-slot=\"{i}\">\
                   <button type=\"button\" class=\"ws-play\" data-ws-play=\"{i}\" aria-label=\"{}\">\u{25B6}</button>\
                   <span class=\"ws-word\" dir=\"auto\">{shown}</span>{}\
                 </div>",
                if g.found[i] { " found" } else { "" },
                if g.active == Some(i) { " active" } else { "" },
                dom::escape_html(&t("ws.play").replace("{n}", &(i + 1).to_string())),
                if slow {
                    format!(
                        "<button type=\"button\" class=\"ws-slow\" data-ws-slow=\"{i}\" aria-label=\"{}\">\u{1F422}</button>",
                        dom::escape_html(&t("ws.slow"))
                    )
                } else {
                    String::new()
                },
            ));
        }
        dom::set_html("wsSlots", &slots);

        let found_cells: Vec<usize> =
            p.targets.iter().zip(&g.found).filter(|(_, f)| **f).flat_map(|(t, _)| t.cells.iter().copied()).collect();
        let mut html = String::new();
        for (i, ch) in p.grid.iter().enumerate() {
            let mut cls = String::from("ws-cell");
            if found_cells.contains(&i) {
                cls.push_str(" found");
            }
            if g.drag_path.contains(&i) || g.drag_from == Some(i) {
                cls.push_str(" sel");
            }
            if g.flash.contains(&i) {
                cls.push_str(" trap");
            }
            html.push_str(&format!("<div class=\"{cls}\" data-ws-cell=\"{i}\">{}</div>", dom::escape_html(ch)));
        }
        dom::set_html("wsGrid", &html);
        let _ = dom::el("wsGrid").set_attribute("style", &format!("--ws-n:{n}"));
        let meaning = finding && p.tier.definition_hint() && crate::api::meaning_supported(&g.lang);
        dom::toggle_class("wsMeaningBtn", "btn-hide", !meaning);

        match &g.phase {
            Phase::LockIn { order, at, typed, .. } => {
                dom::set_text("wsLockCount", &format!("{} / {}", at + 1, order.len()));
                dom::set_text("wsTyped", typed);
                dom::toggle_class("wsSkip", "btn-hide", p.tier.lock_in_required());
                let mut k = String::new();
                for row in layout(&g.lang) {
                    k.push_str("<div class=\"sd-row\">");
                    for c in row {
                        k.push_str(&format!(
                            "<button type=\"button\" class=\"kb-key\" data-ws-key=\"{0}\">{0}</button>",
                            dom::escape_html(&c)
                        ));
                    }
                    k.push_str("</div>");
                }
                k.push_str(
                    "<div class=\"sd-row\"><button type=\"button\" class=\"kb-key wide\" data-ws-back>\u{232B}</button>\
                     <button type=\"button\" class=\"kb-key wide go\" data-ws-go>\u{2713}</button></div>",
                );
                dom::set_html("wsKeys", &k);
            }
            Phase::Done { stars } => {
                let s: String = (0..STARS).map(|i| if i < *stars { '\u{2605}' } else { '\u{2606}' }).collect();
                dom::set_text("wsStars", &s);
            }
            Phase::Find => {}
        }
    });
}

fn play(i: usize, slow: bool) {
    let Some((word, lang)) = GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let g = gb.as_mut()?;
        g.active = Some(i);
        Some((g.puzzle.targets.get(i)?.word.clone(), g.lang.clone()))
    }) else {
        return;
    };
    dom::set_html("wsMeaning", "");
    render();
    crate::api::play_word(&word, if slow { "slow" } else { "normal" }, 1.0, &lang, || note(&t("ws.audioOff")));
}

fn play_lock_in_word() {
    let Some((word, lang)) = GAME.with(|c| {
        let gb = c.borrow();
        let g = gb.as_ref()?;
        match &g.phase {
            Phase::LockIn { order, at, .. } => Some((g.puzzle.targets[order[*at]].word.clone(), g.lang.clone())),
            _ => None,
        }
    }) else {
        return;
    };
    crate::api::play_word(&word, "normal", 1.0, &lang, || note(&t("ws.audioOff")));
}

/// A drag (or two taps) covered `cells`: a target, a trap, or nothing.
fn select(cells: Vec<usize>) {
    let mut entered_lock_in = false;
    GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        if !matches!(g.phase, Phase::Find) {
            return;
        }
        g.flash.clear();
        match judge(&g.puzzle, &cells) {
            Hit::Target(i) => {
                if !g.found[i] {
                    g.found[i] = true;
                    note(&t("ws.found"));
                }
                if g.found.iter().all(|f| *f) {
                    let mut order: Vec<usize> = (0..g.found.len()).collect();
                    Rng::new(hash(&g.puzzle)).shuffle(&mut order);
                    g.phase = Phase::LockIn { order, at: 0, typed: String::new(), misses: 0 };
                    entered_lock_in = true;
                }
            }
            Hit::Decoy(j) => {
                // F-S2: the trap spelling. One star-point, once per decoy; the
                // miss goes to the base game's log as TRAP_MISS.
                note(&t("ws.trap"));
                g.flash = g.puzzle.decoys[j].0.cells.clone();
                if !g.tripped[j] {
                    g.tripped[j] = true;
                    g.traps += 1;
                    let (d, ti) = &g.puzzle.decoys[j];
                    let target = &g.puzzle.targets[*ti].word;
                    crate::learner::note_built(&g.lang, trap_attempt(&g.lang, target, &d.word, crate::learner::day_now()));
                }
            }
            Hit::Miss => {}
        }
    });
    render();
    if entered_lock_in {
        note(&t("ws.lockHelp"));
        play_lock_in_word();
    }
}

fn finish(misses: u32, locked_in: bool) {
    GAME.with(|c| {
        if let Some(g) = c.borrow_mut().as_mut() {
            // Stars only; nothing here touches the run-protection count (I11).
            let lost = g.traps + misses;
            let stars = if !locked_in && g.puzzle.tier.lock_in_required() { 0 } else { STARS.saturating_sub(lost) };
            g.phase = Phase::Done { stars };
        }
    });
    note(&t("ws.done"));
    render();
}

fn submit_lock_in() {
    let mut next_word = false;
    let mut done: Option<u32> = None;
    GAME.with(|c| {
        let mut gb = c.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        let lang = g.lang.clone();
        let kid = g.kid;
        let Phase::LockIn { order, at, typed, misses } = &mut g.phase else { return };
        if typed.is_empty() {
            return;
        }
        let word = g.puzzle.targets[order[*at]].word.clone();
        // F-S4: graded as the base game grades, and recorded through the same
        // door on the typed channel (F-X6).
        let correct = crate::norm::answer_matches(typed, &word, kid)
            || (crate::homophones::accepts(&lang, &word, typed)
                && crate::norm::fold_strict(typed) != crate::norm::fold_strict(&word));
        crate::learner::note_built(&lang, lock_in_attempt(&lang, &word, typed, correct, crate::learner::day_now()));
        if correct {
            note(&t("ws.right"));
        } else {
            *misses += 1;
            note(&t("ws.wrong").replace("{word}", &word));
        }
        typed.clear();
        *at += 1;
        if *at >= order.len() {
            done = Some(*misses);
        } else {
            next_word = true;
        }
    });
    if let Some(m) = done {
        finish(m, true);
        return;
    }
    render();
    if next_word {
        play_lock_in_word();
    }
}

fn show_meaning() {
    let Some((word, lang)) = GAME.with(|c| {
        let gb = c.borrow();
        let g = gb.as_ref()?;
        let i = g.active?;
        Some((g.puzzle.targets[i].word.clone(), g.lang.clone()))
    }) else {
        dom::set_text("wsMeaning", &t("ws.pickSlot"));
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let text = match crate::api::fetch_meaning(&word, true, &lang).await {
            // F-X4 / I10: a definition that shows the word or a chunk of it is
            // not shown at all; it is never rewritten.
            Ok((_, def, _)) if !def.is_empty() && hint::passes(&word, &def) => def,
            _ => t("ws.noMeaning"),
        };
        dom::set_text("wsMeaning", &text);
    });
}

/// The cell under a pointer, from the grid's own box.
fn cell_at(x: f64, y: f64) -> Option<usize> {
    let n = GAME.with(|c| c.borrow().as_ref().map(|g| g.puzzle.size))?;
    let r = dom::el("wsGrid").get_bounding_client_rect();
    if x < r.left() || y < r.top() || x >= r.right() || y >= r.bottom() {
        return None;
    }
    let col = ((x - r.left()) / (r.width() / n as f64)) as usize;
    let row = ((y - r.top()) / (r.height() / n as f64)) as usize;
    Some(row.min(n - 1) * n + col.min(n - 1))
}

/// Mark the cells under the current drag without rebuilding the grid.
fn paint_sel() {
    let sel: Vec<usize> = GAME
        .with(|c| c.borrow().as_ref().map(|g| g.drag_path.iter().copied().chain(g.drag_from).collect()))
        .unwrap_or_default();
    let cells = dom::el("wsGrid").children();
    for i in 0..cells.length() {
        if let Some(el) = cells.item(i) {
            let _ = el.class_list().toggle_with_force("sel", sel.contains(&(i as usize)));
        }
    }
}

fn drag_to(i: usize) {
    GAME.with(|c| {
        if let Some(g) = c.borrow_mut().as_mut() {
            if let Some(a) = g.drag_from {
                g.drag_path = path(g.puzzle.size, a, i).unwrap_or_default();
            }
        }
    });
    paint_sel();
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
    dom::select("wsPick").set_value(tier.id());
    if serve_bank(app, tier) {
        dom::add_class("wsScreen", "show");
    }
}

/// F-X1: "Make a puzzle" on a My Words list. The list's words in the app's
/// language; the grid is Jr for an under-13 player and Easy otherwise.
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
    match serve::from_list(&lang, tier, list_id, &words, ledger().counter) {
        Some(s) => {
            fill_picker(kid, &lang);
            dom::select("wsPick").set_value(tier.id());
            start(app, s, Source::List(list_id.to_string()));
            dom::add_class("wsScreen", "show");
            true
        }
        None => {
            dom::show_toast(&t("ws.listShort"));
            false
        }
    }
}

fn close() {
    dom::remove_class("wsScreen", "show");
}

fn again(app: &App) {
    let (source, lang, kid) = match GAME.with(|c| c.borrow().as_ref().map(|g| (g.source.clone(), g.lang.clone(), g.kid))) {
        Some(x) => x,
        None => return,
    };
    match source {
        Source::List(id) => {
            open_list(app, &id);
        }
        _ => {
            serve_bank(app, picked_tier(kid, &lang));
        }
    }
}

pub fn wire(app: &App) {
    {
        let a = app.clone();
        dom::on_click("wsOpenBtn", move || open(&a));
    }
    dom::on_click("wsExit", close);
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("wsPick", "change", move |_| {
            let (lang, kid) = {
                let s = a.borrow();
                (s.lang.clone(), s.kid)
            };
            serve_bank(&a, picked_tier(kid, &lang));
        });
    }
    {
        let a = app.clone();
        dom::on_click("wsNew", move || again(&a));
    }
    {
        let a = app.clone();
        dom::on_click("wsNext", move || again(&a));
    }
    {
        let a = app.clone();
        dom::on_click("wsDaily", move || {
            serve_daily(&a);
        });
    }
    dom::on_click("wsMeaningBtn", show_meaning);
    dom::on_click("wsLockPlay", play_lock_in_word);
    dom::on_click("wsSkip", || {
        let skip = GAME.with(|c| c.borrow().as_ref().is_some_and(|g| !g.puzzle.tier.lock_in_required()));
        if skip {
            finish(0, false);
        }
    });

    // Drag across the grid. A tap that starts and ends on one cell sets an
    // anchor, and a second tap on another cell selects the line between them.
    dom::on::<web_sys::PointerEvent, _>("wsGrid", "pointerdown", |e| {
        let Some(i) = cell_at(e.client_x() as f64, e.client_y() as f64) else { return };
        e.prevent_default();
        // Every move and the release come to the grid, wherever the finger goes.
        let _ = dom::el("wsGrid").set_pointer_capture(e.pointer_id());
        let anchored = GAME.with(|c| c.borrow().as_ref().and_then(|g| g.drag_from.filter(|_| g.drag_path.len() <= 1)));
        if let Some(a) = anchored.filter(|a| *a != i) {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    g.drag_path = path(g.puzzle.size, a, i).unwrap_or_default();
                }
            });
            paint_sel();
            return;
        }
        GAME.with(|c| {
            if let Some(g) = c.borrow_mut().as_mut() {
                g.drag_from = Some(i);
                g.drag_path = vec![i];
            }
        });
        paint_sel();
    });
    dom::on::<web_sys::PointerEvent, _>("wsGrid", "pointermove", |e| {
        if e.buttons() == 0 {
            return;
        }
        if let Some(i) = cell_at(e.client_x() as f64, e.client_y() as f64) {
            drag_to(i);
        }
    });
    dom::on::<web_sys::PointerEvent, _>("wsGrid", "pointerup", |_| {
        let drag = GAME.with(|c| c.borrow().as_ref().map(|g| g.drag_path.clone())).unwrap_or_default();
        if drag.len() >= 2 {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    g.drag_from = None;
                    g.drag_path.clear();
                }
            });
            select(drag);
        } else {
            paint_sel(); // one cell: it stays as the anchor
        }
    });

    dom::on::<web_sys::Event, _>("wsScreen", "click", |ev| {
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target.closest("[data-ws-play],[data-ws-slow],[data-ws-key],[data-ws-back],[data-ws-go]").ok().flatten() else {
            return;
        };
        if let Some(i) = el.get_attribute("data-ws-play").and_then(|v| v.parse::<usize>().ok()) {
            play(i, false);
        } else if let Some(i) = el.get_attribute("data-ws-slow").and_then(|v| v.parse::<usize>().ok()) {
            play(i, true);
        } else if let Some(k) = el.get_attribute("data-ws-key") {
            GAME.with(|c| {
                if let Some(Game { phase: Phase::LockIn { typed, .. }, .. }) = c.borrow_mut().as_mut() {
                    if typed.chars().count() < 24 {
                        typed.push_str(&k);
                    }
                }
            });
            render();
        } else if el.get_attribute("data-ws-back").is_some() {
            GAME.with(|c| {
                if let Some(Game { phase: Phase::LockIn { typed, .. }, .. }) = c.borrow_mut().as_mut() {
                    typed.pop();
                }
            });
            render();
        } else if el.get_attribute("data-ws-go").is_some() {
            submit_lock_in();
        }
    });
}

/// Test builds only: the grid digest as this build's WebAssembly computes it
/// (test 2 compares it with the host's pinned value).
#[cfg(feature = "testseam")]
pub fn seam_golden() -> String {
    format!("{:#x}", serve::golden_digest(500))
}

/// Test builds only: the served puzzle, so a browser test knows where words are.
#[cfg(feature = "testseam")]
pub fn seam_board() -> String {
    GAME.with(|c| {
        c.borrow()
            .as_ref()
            .map(|g| {
                let p = &g.puzzle;
                let asking = match &g.phase {
                    Phase::LockIn { order, at, .. } => Some(p.targets[order[*at]].word.clone()),
                    _ => None,
                };
                let stars = match &g.phase {
                    Phase::Done { stars } => Some(*stars),
                    _ => None,
                };
                serde_json::json!({
                    "asking": asking,
                    "stars": stars,
                    "tier": p.tier.id(),
                    "size": p.size,
                    "hash": format!("{:#x}", hash(p)),
                    "targets": p.targets.iter().map(|t| serde_json::json!({"word": t.word, "cells": t.cells})).collect::<Vec<_>>(),
                    "decoys": p.decoys.iter().map(|(d, t)| serde_json::json!({"word": d.word, "cells": d.cells, "of": t})).collect::<Vec<_>>(),
                })
                .to_string()
            })
            .unwrap_or_default()
    })
}

/// Test builds only: milliseconds to build one Easy puzzle from `words` (test 11).
#[cfg(feature = "testseam")]
pub fn seam_time_list(lang: &str, words_json: &str, counter: u32) -> f64 {
    let words: Vec<String> = serde_json::from_str(words_json).unwrap_or_default();
    let perf = web_sys::window().and_then(|w| w.performance());
    let t0 = perf.as_ref().map_or(0.0, |p| p.now());
    let ok = serve::from_list(lang, Tier::Easy, "perf", &words, counter as u64).is_some();
    let t1 = perf.as_ref().map_or(0.0, |p| p.now());
    if ok { t1 - t0 } else { -1.0 }
}
