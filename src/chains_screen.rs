//! CC-WORD-CHAINS F1 — the chain screen.
//!
//! The hook unit is the largest thing on screen because it is the whole
//! question the player is answering: "what starts with 리?". Everything else —
//! the ribbon, the keys, the Pass button — is subordinate to it.
//!
//! [`crate::chains`] owns every rule. This file composes input, renders state
//! and picks a reaction; it never decides what chains.
//!
//! # Memory is the game's job (D5)
//!
//! The chain is a visible ribbon, and re-submitting a word already in it costs
//! nothing but a grey shake. Losing to your own memory of a list you cannot see
//! is not difficulty, it is bookkeeping.
//!
//! # Always an exit (D4)
//!
//! Pass is always available and always draws a word that keeps the chain alive
//! — never a Japanese ん-ending, which would end the run in the engine's own
//! rules. When nothing is left the run ENDS rather than fails: the closing card
//! celebrates the length reached.
//!
//! # What is not built
//!
//! Timed mood (D3) and its clock ring, and the dead-end rejection that only
//! applies there. Pass-and-play (D3 Phase 2) is explicitly a later phase of the
//! same file. Untimed is the PREVIEW-free mood, so this is the half a player
//! reaches without an entitlement.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use wasm_bindgen::JsCast;

use crate::{chains, dom, haptics, i18n, storage, App};

/// D6: Pass keeps momentum but is not free. One link off the score, floored at
/// zero — a cost the player can always afford, because a Pass they cannot
/// afford is not an exit.
const PASS_COST: u32 = 1;

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    /// The unit the next word must start with. The screen's whole subject.
    static HOOK: RefCell<String> = const { RefCell::new(String::new()) };
    /// The chain in play order — the ribbon renders this directly.
    static CHAIN: RefCell<Vec<String>> = RefCell::new(Vec::new());
    /// Fast membership for the duplicate check; CHAIN keeps the order.
    static USED: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    static CURRENT: RefCell<String> = const { RefCell::new(String::new()) };
    static PASSES: Cell<u32> = const { Cell::new(0) };
    static OVER: Cell<bool> = const { Cell::new(false) };
    static SEED: Cell<u64> = const { Cell::new(0) };
}

fn best_key(lang: &str) -> String {
    format!("spell_chains_best_{lang}")
}

/// Score is chain length less what Pass cost (D6), never below zero.
fn score() -> u32 {
    let n = CHAIN.with(|c| c.borrow().len()) as u32;
    n.saturating_sub(PASSES.with(Cell::get) * PASS_COST)
}

/// Stamp the played language on the surfaces that hold words.
///
/// CC-RTL F1: a word surface without a direction is a bug. The hook, the
/// composed word and the ribbon are all word surfaces, and Arabic is a live
/// chain language — its hooks read right-to-left.
fn reflect_direction(lang: &str) {
    let dir = crate::consts::dir_attr(lang);
    for id in ["cnHook", "cnWord", "cnRibbon", "cnKeys", "cnOverBody"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", lang);
            let _ = e.set_attribute("dir", dir);
        }
    }
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("chainsOpen", move || open(&a));
    dom::on_click("cnExit", close);
    dom::on_click("cnDelete", delete);
    dom::on_click("cnSubmit", submit);
    dom::on_click("cnPass", pass);
    let a = app.clone();
    dom::on_click("cnReplay", move || open(&a));
    // One delegated listener: the key grid is rebuilt on every language change.
    dom::on::<web_sys::MouseEvent, _>("cnKeys", "click", tap);
}

thread_local! {
    /// CC-ONBOARD-JR: this chain is a Spell Jr chain. Set at open; Pass has no App.
    static JUNIOR: Cell<bool> = Cell::new(false);
}

fn experience() -> crate::experience::Experience {
    crate::experience::of_kid(JUNIOR.with(Cell::get))
}

pub fn open(app: &App) {
    crate::telemetry::set_mode(crate::telemetry::schema::Mode::WordChains);
    let lang = app.borrow().lang.clone();
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    reflect_direction(&lang);
    dom::add_class("chains", "show");
    dom::remove_class("cnOver", "show");
    dom::set_text("cnStatus", "");
    CHAIN.with(|c| c.borrow_mut().clear());
    USED.with(|u| u.borrow_mut().clear());
    CURRENT.with(|c| c.borrow_mut().clear());
    PASSES.with(|c| c.set(0));
    OVER.with(|c| c.set(false));
    SEED.with(|c| c.set(js_sys::Date::now() as u64));
    JUNIOR.with(|j| j.set(app.borrow().kid));

    if !chains::chains_ready(&lang) {
        dom::set_text("cnHook", "");
        dom::set_html("cnKeys", "");
        dom::set_text("cnStatus", &i18n::t("chains.unavailable"));
        return;
    }

    // Open on a hook the bank can answer. Drawn from the language's own first
    // key rather than a fixed letter, so this works in every script.
    let first = crate::keyboard::unit_rows(&lang)
        .first()
        .and_then(|r| r.chars().next())
        .unwrap_or('a')
        .to_string();
    let used = BTreeSet::new();
    let opener = chains::pass_draw_for(experience(), &lang, &first, &used, SEED.with(Cell::get));
    match opener {
        Some(w) => {
            let h = chains::hook(&lang, &w).unwrap_or(first);
            accept(&lang, w, h);
        }
        None => {
            HOOK.with(|h| *h.borrow_mut() = first);
        }
    }
    render_keys(&lang);
    render_all();
}

fn close() {
    dom::remove_class("chains", "show");
}

/// Put a word into the chain and move the hook on.
fn accept(lang: &str, word: String, next_hook: String) {
    USED.with(|u| {
        u.borrow_mut().insert(word.clone());
    });
    CHAIN.with(|c| c.borrow_mut().push(word));
    HOOK.with(|h| *h.borrow_mut() = next_hook);
    let _ = lang;
}

// ------------------------------------------------------------------ render

fn render_all() {
    render_hook();
    render_word();
    render_ribbon();
    render_score();
}

fn render_hook() {
    HOOK.with(|h| dom::set_text("cnHook", &h.borrow()));
}

fn render_word() {
    CURRENT.with(|c| dom::set_text("cnWord", &c.borrow()));
}

/// The ribbon renders newest-last and is scrolled to the end, so the most
/// recent word — the one whose hook is live — is always the one in view.
fn render_ribbon() {
    let html: String = CHAIN.with(|c| {
        c.borrow()
            .iter()
            .map(|w| format!("<span class=\"cn-link\">{}</span>", dom::escape_html(shown(w))))
            .collect()
    });
    dom::set_html("cnRibbon", &html);
    if let Some(h) = dom::doc()
        .get_element_by_id("cnRibbon")
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        h.set_scroll_left(h.scroll_width());
    }
}

/// zh entries are `pinyin|hanzi`; the chain is played and shown in pinyin.
fn shown(w: &str) -> &str {
    w.split('|').next().unwrap_or(w)
}

fn render_score() {
    dom::set_text("cnScore", &score().to_string());
    let lang = LANG.with(|l| l.borrow().clone());
    let best: u32 = storage::get_json(&best_key(&lang)).unwrap_or(0);
    dom::set_text("cnBest", &best.to_string());
}

fn render_keys(lang: &str) {
    let mut html = String::new();
    for row in crate::keyboard::unit_rows(lang) {
        html.push_str("<div class=\"cn-row\">");
        for c in row.chars() {
            html.push_str(&format!(
                "<button type=\"button\" class=\"cn-key\" data-k=\"{0}\">{0}</button>",
                dom::escape_html(&c.to_string())
            ));
        }
        html.push_str("</div>");
    }
    dom::set_html("cnKeys", &html);
}

// ------------------------------------------------------------------ input

fn tap(e: web_sys::MouseEvent) {
    if OVER.with(Cell::get) {
        return;
    }
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|t| t.closest("[data-k]").ok().flatten())
    else {
        return;
    };
    let Some(k) = el.get_attribute("data-k") else { return };
    let lang = LANG.with(|l| l.borrow().clone());
    CURRENT.with(|c| {
        let mut cur = c.borrow_mut();
        if lang == "ko" {
            // Korean composes through the base game's automaton, so the player
            // watches real syllable blocks assemble rather than a jamo run.
            if let Some(ch) = k.chars().next() {
                *cur = crate::hangul::feed(&cur, ch);
            }
        } else {
            cur.push_str(&k);
        }
    });
    haptics::key_tap();
    render_word();
}

fn delete() {
    let lang = LANG.with(|l| l.borrow().clone());
    CURRENT.with(|c| {
        let mut cur = c.borrow_mut();
        if lang == "ko" {
            *cur = crate::hangul::backspace(&cur);
        } else {
            cur.pop();
        }
    });
    haptics::key_tap();
    render_word();
}

fn flash(class: &'static str) {
    dom::add_class("cnWord", class);
    after(420, move || dom::remove_class("cnWord", class));
}

fn submit() {
    if OVER.with(Cell::get) {
        return;
    }
    let word = CURRENT.with(|c| c.borrow().clone());
    if word.is_empty() {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let hook = HOOK.with(|h| h.borrow().clone());

    // D5: a word already in the chain costs nothing. Checked FIRST, because a
    // duplicate is also a valid word and a valid link — reporting it as either
    // would be true and useless.
    if USED.with(|u| u.borrow().iter().any(|w| shown(w) == word)) {
        flash("dup");
        dom::set_text("cnStatus", &i18n::t("chains.dup"));
        haptics::key_tap();
    } else if !crate::word_index::is_valid(&lang, &word) {
        flash("miss");
        dom::set_text("cnStatus", &i18n::t("chains.invalid"));
        haptics::key_tap();
    } else if chains::head(&lang, &word).as_deref() != Some(hook.as_str()) {
        // Right word, wrong hook. Named separately so the player learns the
        // rule rather than concluding the word was rejected.
        flash("miss");
        dom::set_text("cnStatus", &i18n::t("chains.nolink"));
        haptics::key_tap();
    } else {
        let next = chains::hook(&lang, &word);
        match next {
            Some(h) => {
                accept(&lang, word, h);
                flash("good");
                dom::set_text("cnStatus", "");
                haptics::correct();
            }
            None => {
                // A ja ん-ending: a legal move the player chose, which ends the
                // run. It still counts — the word joins the chain first.
                accept(&lang, word, String::new());
                haptics::correct();
                finish(&i18n::t("chains.ended.n"));
            }
        }
    }
    CURRENT.with(|c| c.borrow_mut().clear());
    render_all();
    check_exit(&lang);
}

/// D4: Pass draws a live successor. Always available, never a dead end.
fn pass() {
    if OVER.with(Cell::get) {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let hook = HOOK.with(|h| h.borrow().clone());
    let used = USED.with(|u| u.borrow().clone());
    let seed = SEED.with(Cell::get).wrapping_add(CHAIN.with(|c| c.borrow().len() as u64));
    match chains::pass_draw_for(experience(), &lang, &hook, &used, seed) {
        Some(w) => {
            let h = chains::hook(&lang, &w).unwrap_or_default();
            PASSES.with(|c| c.set(c.get() + 1));
            accept(&lang, w, h);
            dom::set_text("cnStatus", &i18n::t("chains.passed"));
            haptics::key_tap();
            CURRENT.with(|c| c.borrow_mut().clear());
            render_all();
            check_exit(&lang);
        }
        None => finish(&i18n::t("chains.ended.out")),
    }
}

/// The run ends when the live hook has nothing left. Checked after every move
/// so the closing card arrives on the move that exhausted the chain, not one
/// submission later.
fn check_exit(lang: &str) {
    if OVER.with(Cell::get) {
        return;
    }
    let hook = HOOK.with(|h| h.borrow().clone());
    let used = USED.with(|u| u.borrow().clone());
    if hook.is_empty() || !chains::has_successor(lang, &hook, &used) {
        finish(&i18n::t("chains.ended.out"));
    }
}

/// D4/D6: a finished chain is an achievement, never a loss. The card leads with
/// the length reached and the longest word found.
fn finish(why: &str) {
    OVER.with(|c| c.set(true));
    let lang = LANG.with(|l| l.borrow().clone());
    let final_score = score();
    let best: u32 = storage::get_json(&best_key(&lang)).unwrap_or(0);
    if final_score > best {
        storage::set_json(&best_key(&lang), &final_score);
    }
    let longest = CHAIN.with(|c| {
        c.borrow()
            .iter()
            .map(|w| shown(w).to_string())
            .max_by_key(|w| w.chars().count())
            .unwrap_or_default()
    });
    dom::set_text("cnOverWhy", why);
    dom::set_text("cnOverLen", &final_score.to_string());
    dom::set_text("cnOverLongest", &longest);
    dom::add_class("cnOver", "show");
    render_score();
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = wasm_bindgen::closure::Closure::once(f);
    let _ = dom::window().set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        ms,
    );
    cb.forget();
}
