//! CC-LETTER-FORGE F1 — the forge screen.
//!
//! Seven units in a honeycomb, one of them the centre. Tap to compose, submit,
//! and the board answers immediately. The engine ([`crate::forge`]) owns every
//! rule — which words are playable, what a submission scores, where the rank
//! rungs sit — so this file only composes input, shows state, and picks the
//! reaction. A rule that appears to live here is in the wrong file.
//!
//! # The three reactions (D5), none of which sting
//!
//! Valid, duplicate and invalid are three DISTINCT feels: a colour and the
//! score, a grey shake, a soft bounce. Duplicate and invalid cost nothing — no
//! penalty, no red, no counter moving backwards. A mode built on hundreds of
//! submissions per session cannot punish wrong guesses, because wrong guesses
//! are how the right ones get found.
//!
//! # Korean assembles blocks (D1)
//!
//! The honeycomb holds jamo, the player taps jamo, and the composed answer is
//! SYLLABLE BLOCKS. That is [`crate::hangul::feed`]'s job and it is already the
//! base game's composition path, so the forge inherits the automaton instead of
//! growing a second one that could disagree with it.
//!
//! # The pool total is not in this file
//!
//! D4 forbids showing it before "Reveal remaining", so nothing here binds it.
//! The bar renders [`crate::forge::rank_progress`], a ratio; the denominator
//! never reaches the DOM. "12 of 214" demoralizes, "Great" motivates. The
//! FOUND count is shown — that is the player's own number, not the answer key's.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use wasm_bindgen::JsCast;

use crate::forge::{self, Puzzle};
use crate::{dom, haptics, i18n, norm::fold_strict, storage, App};

thread_local! {
    static PUZZLE: RefCell<Option<Puzzle>> = const { RefCell::new(None) };
    /// Every playable word, folded — the answer set a submission is checked
    /// against. Folded once at open so a keystroke never pays for it.
    static POOL: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    static FOUND: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    /// Display order of the six outer units. Shuffle rotates this and nothing
    /// else: the puzzle IS its unit set, so rearranging tiles cannot change
    /// which words are playable, and the composed word survives untouched.
    static ORDER: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static CURRENT: RefCell<String> = const { RefCell::new(String::new()) };
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static SCORE: Cell<u32> = const { Cell::new(0) };
    static MAX: Cell<u32> = const { Cell::new(0) };
}

/// Local calendar date, YYYY-MM-DD.
///
/// Deliberately NOT `daily::today()`. D3 says the Daily Forge follows the same
/// determinism doctrine as the Daily Challenge and shares no code with it, so a
/// change to that module's date handling must not be able to move this one's
/// puzzle. Three lines is a cheap price for that independence.
fn today() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

/// FNV-1a over "date:lang" — one puzzle per language per day, so one language's
/// Tuesday is not another's.
fn daily_seed(lang: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("{}:{}", today(), lang).bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

thread_local! {
    /// CC-ONBOARD-JR: the board on screen is a Spell Jr board. Set at open.
    static JUNIOR: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

/// Junior and standard boards differ, so their found-word lists must not mix.
/// The standard key is unchanged.
fn save_key(lang: &str) -> String {
    if JUNIOR.with(|j| j.get()) {
        format!("spell_forge_jr_{lang}_{}", today())
    } else {
        format!("spell_forge_{lang}_{}", today())
    }
}

/// The comparable form of a submission. Mirrors what the generator did when it
/// built the pool — `fold_strict` over the typed half of a `pinyin|hanzi` entry
/// — so the board and the answer set can never disagree about a word.
fn key(w: &str) -> String {
    fold_strict(w.split('|').next().unwrap_or(w))
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("forgeOpen", move || open(&a));
    dom::on_click("fgExit", close);
    dom::on_click("fgShuffle", shuffle);
    dom::on_click("fgDelete", delete);
    dom::on_click("fgSubmit", submit);
    dom::on_click("fgReveal", reveal);
    // One delegated listener for seven tiles: the comb is re-rendered on every
    // shuffle, and per-tile handlers would leak a closure each time.
    dom::on::<web_sys::MouseEvent, _>("fgComb", "click", tap);
}

/// Stamp the board's language and direction on the surfaces that hold words.
///
/// CC-RTL F1's rule, applied here: a word surface with no direction is a bug.
/// It also makes the `:lang()` tracking resets in index.html actually match —
/// `.fg-word` sets letter-spacing, which prises Arabic cursive joins apart and
/// detaches Devanagari matras, and the companion reset keys off `:lang()`. With
/// no lang attribute the reset would never fire and the CSS would be decoration.
///
/// Keyed off the FORGE's language rather than `<html lang>`, per the warning on
/// that reset block: the UI locale is not the language being spelled, and using
/// it would strip tracking for an Arabic speaker forging English words while
/// leaving it on for an English speaker forging Arabic ones.
fn reflect_direction(lang: &str) {
    let dir = crate::consts::dir_attr(lang);
    for id in ["fgWord", "fgComb", "fgFound", "fgRemainingList"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", lang);
            let _ = e.set_attribute("dir", dir);
        }
    }
}

pub fn open(app: &App) {
    let lang = app.borrow().lang.clone();
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    reflect_direction(&lang);
    dom::add_class("forge", "show");
    dom::remove_class("fgRemaining", "show");
    dom::set_text("fgStatus", "");

    // A language whose bank cannot sustain a board says so plainly rather than
    // dealing a thin one. forge_ready is the engine's call, not the screen's.
    // CC-ONBOARD-JR: a Spell Jr board is its own board (Easy + Medium letters
    // and pool), with its own readiness and its own saved progress.
    let exp = crate::experience::of_kid(app.borrow().kid);
    JUNIOR.with(|j| j.set(exp == crate::experience::Experience::Junior));
    if !forge::forge_ready_for(exp, &lang) {
        dom::set_html("fgComb", "");
        dom::set_text("fgWord", "");
        dom::set_text("fgStatus", &i18n::t("forge.unavailable"));
        return;
    }

    let Some((p, pool, _walk)) = forge::generate_for(exp, &lang, daily_seed(&lang), 512) else {
        dom::set_html("fgComb", "");
        dom::set_text("fgWord", "");
        dom::set_text("fgStatus", &i18n::t("forge.unavailable"));
        return;
    };

    MAX.with(|c| c.set(forge::max_score(&lang, &p, &pool)));
    POOL.with(|s| *s.borrow_mut() = pool.iter().map(|w| key(w)).collect());
    ORDER.with(|o| {
        *o.borrow_mut() = p.units.iter().filter(|u| **u != p.centre).cloned().collect();
    });
    PUZZLE.with(|c| *c.borrow_mut() = Some(p));
    CURRENT.with(|c| c.borrow_mut().clear());

    // Same-day resume (D3): the daily is one puzzle, not one sitting.
    let saved: Vec<String> = storage::get_json(&save_key(&lang)).unwrap_or_default();
    FOUND.with(|f| *f.borrow_mut() = saved.into_iter().collect());
    rescore();
    render_all();
}

fn close() {
    dom::remove_class("forge", "show");
}

/// Recompute the score from the found set, so it can never drift from what is
/// actually on the board — a resumed session and a fresh one agree.
fn rescore() {
    let lang = LANG.with(|l| l.borrow().clone());
    let total = PUZZLE.with(|p| {
        p.borrow().as_ref().map_or(0, |p| {
            FOUND.with(|f| f.borrow().iter().map(|w| forge::score_word(&lang, p, w)).sum())
        })
    });
    SCORE.with(|c| c.set(total));
}

fn persist() {
    let lang = LANG.with(|l| l.borrow().clone());
    let words: Vec<String> = FOUND.with(|f| f.borrow().iter().cloned().collect());
    storage::set_json(&save_key(&lang), &words);
}

// ------------------------------------------------------------------ render

fn render_all() {
    render_comb();
    render_word();
    render_found();
    render_rank();
}

fn render_comb() {
    let Some(centre) = PUZZLE.with(|p| p.borrow().as_ref().map(|p| p.centre.clone())) else {
        return;
    };
    let mut html = String::new();
    ORDER.with(|o| {
        let outer = o.borrow();
        // The centre sits mid-grid so the six ring it in a 3x3 flow.
        for (i, u) in outer.iter().enumerate() {
            if i == 3 {
                html.push_str(&cell(&centre, true));
            }
            html.push_str(&cell(u, false));
        }
        if outer.len() < 4 {
            html.push_str(&cell(&centre, true));
        }
    });
    dom::set_html("fgComb", &html);
}

fn cell(unit: &str, centre: bool) -> String {
    format!(
        "<button type=\"button\" class=\"fg-cell{}\" data-u=\"{}\">{}</button>",
        if centre { " centre" } else { "" },
        dom::escape_html(unit),
        dom::escape_html(unit)
    )
}

fn render_word() {
    CURRENT.with(|c| dom::set_text("fgWord", &c.borrow()));
}

fn render_found() {
    let mut words: Vec<String> = FOUND.with(|f| f.borrow().iter().cloned().collect());
    words.sort();
    let html: String = words
        .iter()
        .map(|w| format!("<span class=\"fg-found-word\">{}</span>", dom::escape_html(w)))
        .collect();
    dom::set_html("fgFound", &html);
    dom::set_text("fgFoundCount", &words.len().to_string());
}

fn render_rank() {
    let (score, max) = (SCORE.with(Cell::get), MAX.with(Cell::get));
    let name = match forge::rank_index(score, max) {
        Some(i) => i18n::t(forge::RANK_KEYS[i]),
        None => i18n::t("forge.rank.start"),
    };
    dom::set_text("fgRank", &name);
    dom::set_text("fgScore", &score.to_string());
    let pct = forge::rank_progress(score, max);
    if let Some(h) = dom::doc()
        .get_element_by_id("fgBarFill")
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = h.style().set_property("width", &format!("{pct}%"));
    }
}

// ------------------------------------------------------------------ input

fn tap(e: web_sys::MouseEvent) {
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|t| t.closest("[data-u]").ok().flatten())
    else {
        return;
    };
    let Some(unit) = el.get_attribute("data-u") else { return };
    let lang = LANG.with(|l| l.borrow().clone());
    CURRENT.with(|c| {
        let mut cur = c.borrow_mut();
        if lang == "ko" {
            // The jamo goes through the base game's automaton, so what the
            // player watches assemble is a real syllable block, not a jamo run.
            if let Some(ch) = unit.chars().next() {
                *cur = crate::hangul::feed(&cur, ch);
            }
        } else {
            cur.push_str(&unit);
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

/// D5: one shuffle, prominent, unlimited, free. It rotates the outer ring only
/// — the centre is a RULE (every word must use it), and moving it around would
/// make that rule look like decoration.
fn shuffle() {
    ORDER.with(|o| {
        let mut v = o.borrow_mut();
        if !v.is_empty() {
            v.rotate_left(1);
        }
    });
    haptics::key_tap();
    render_comb();
}

fn flash(class: &'static str) {
    dom::add_class("fgWord", class);
    after(420, move || dom::remove_class("fgWord", class));
}

fn submit() {
    let word = CURRENT.with(|c| c.borrow().clone());
    if word.is_empty() {
        return;
    }
    let k = key(&word);
    let lang = LANG.with(|l| l.borrow().clone());
    let already = FOUND.with(|f| f.borrow().contains(&k));
    let playable = POOL.with(|p| p.borrow().contains(&k));

    if already {
        // Grey shake. No penalty — the player already earned this one.
        flash("dup");
        dom::set_text("fgStatus", &i18n::t("forge.dup"));
        haptics::key_tap();
    } else if playable {
        let (pangram, gained) = PUZZLE.with(|p| {
            p.borrow().as_ref().map_or((false, 0), |p| {
                (forge::is_pangram(&lang, p, &k), forge::score_word(&lang, p, &k))
            })
        });
        FOUND.with(|f| {
            f.borrow_mut().insert(k);
        });
        SCORE.with(|c| c.set(c.get() + gained));
        persist();
        flash(if pangram { "pangram" } else { "good" });
        dom::set_text(
            "fgStatus",
            &if pangram { i18n::t("forge.pangram") } else { format!("+{gained}") },
        );
        if pangram {
            haptics::correct();
        } else {
            haptics::key_tap();
        }
        render_found();
        render_rank();
    } else {
        // Soft bounce. Not red, not a buzz, not a score change.
        flash("miss");
        dom::set_text("fgStatus", &i18n::t("forge.invalid"));
        haptics::key_tap();
    }

    CURRENT.with(|c| c.borrow_mut().clear());
    render_word();
}

/// D4: the total appears here and nowhere earlier, which is why it is a
/// deliberate tap rather than a passive panel — it ends the run for ranking.
fn reveal() {
    let found = FOUND.with(|f| f.borrow().clone());
    let mut rest: Vec<String> =
        POOL.with(|p| p.borrow().iter().filter(|w| !found.contains(*w)).cloned().collect());
    rest.sort();
    let html: String = rest
        .iter()
        .map(|w| format!("<span class=\"fg-found-word rest\">{}</span>", dom::escape_html(w)))
        .collect();
    dom::set_html("fgRemainingList", &html);
    dom::add_class("fgRemaining", "show");
    dom::set_text("fgStatus", &i18n::t("forge.revealed"));
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = wasm_bindgen::closure::Closure::once(f);
    let _ = dom::window().set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        ms,
    );
    cb.forget();
}
