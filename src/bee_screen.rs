//! CC-BEE-SIM F1 — the stage.
//!
//! Podium, a strip of contestants who fall away around you, "Your word is…",
//! and the three ceremonial requests. [`crate::bee`] resolves the whole bee
//! from its seed; this file performs it.
//!
//! # Ceremony and dignity, in that order (D4)
//!
//! A miss is a bell, the correct spelling shown plainly — no red X, no cross,
//! no "wrong" — then a placement and a way back in one round below. A player
//! who goes out in round two should feel like a contestant, not a failure, so
//! nothing on the elimination path is styled as an error.
//!
//! # Requests are strategy (D3)
//!
//! Repeat and Slower are unlimited and cost nothing. The end screen counts
//! CLEAN words — spelled with no request — as a bragging stat, never a penalty.
//! Definition is absent: the app's definitions arrive over the network and D7
//! makes this mode offline, so [`crate::bee::definitions_available`] answers no
//! and the button is not rendered at all (I1).
//!
//! # Kid Mode is the same ceremony without the stakes (D6)
//!
//! Everyone spells every round, nobody is eliminated, and the bee ends in
//! ribbons for the whole field. The elimination path is not merely hidden — it
//! is never entered, so there is no moment a parent could read as the app
//! judging their child.

use std::cell::{Cell, RefCell};

use wasm_bindgen::JsCast;

use crate::bee::{self, Contestant};
use crate::{dom, haptics, i18n, speech_out, storage, App};

/// D5: bee-authentic input commits each unit and forbids going back. Off by
/// default; the toggle lives inside the mode.
const OFFICIAL_KEY: &str = "spell_bee_official";
const FIELD: usize = 12;
const MAX_ROUNDS: u32 = 12;

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static KID: Cell<bool> = const { Cell::new(false) };
    static ROSTER: RefCell<Vec<Contestant>> = RefCell::new(Vec::new());
    static WORDS: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static ROUND: Cell<u32> = const { Cell::new(1) };
    static CURRENT: RefCell<String> = const { RefCell::new(String::new()) };
    /// Requests used on the CURRENT word — a word spelled with none is "clean".
    static ASKED: Cell<bool> = const { Cell::new(false) };
    static CLEAN: Cell<u32> = const { Cell::new(0) };
    static OVER: Cell<bool> = const { Cell::new(false) };
    /// D4's hook: the round the NEXT bee begins at, one below where the player
    /// fell. Held here because the end screen promises it by name and the
    /// button has to honour that.
    static REENTRY: Cell<u32> = const { Cell::new(1) };
}

fn official() -> bool {
    storage::get_raw(OFFICIAL_KEY).as_deref() == Some("on")
}

fn best_key(lang: &str) -> String {
    format!("spell_bee_best_{lang}")
}

/// Every word surface carries the language being spelled (CC-RTL F1).
fn reflect_direction(lang: &str) {
    let dir = crate::consts::dir_attr(lang);
    for id in ["beeWord", "beeKeys", "beeSpelling", "beeStrip"] {
        if let Some(e) = dom::doc().get_element_by_id(id) {
            let _ = e.set_attribute("lang", lang);
            let _ = e.set_attribute("dir", dir);
        }
    }
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("beeOpen", move || open(&a));
    dom::on_click("beeExit", close);
    let a = app.clone();
    dom::on_click("beeRepeat", move || request_repeat(&a, 0.9));
    let a = app.clone();
    dom::on_click("beeSlower", move || request_repeat(&a, 0.55));
    dom::on_click("beeDelete", delete);
    let a = app.clone();
    dom::on_click("beeSubmit", move || submit(&a));
    let a = app.clone();
    dom::on_click("beeNext", move || {
        // D4: re-entry, not a restart. The end screen names the round; opening
        // at 1 would make that copy a lie, which is worse than not offering it.
        let at = REENTRY.with(Cell::get);
        open_at(&a, at);
    });
    dom::on_click("beeOfficial", toggle_official);
    dom::on::<web_sys::MouseEvent, _>("beeKeys", "click", tap);
}

pub fn open(app: &App) {
    open_at(app, 1);
}

fn open_at(app: &App, start: u32) {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    KID.with(|c| c.set(kid));
    reflect_direction(&lang);
    dom::add_class("bee", "show");
    dom::remove_class("beeOver", "show");
    ROUND.with(|c| c.set(start.max(1)));
    CLEAN.with(|c| c.set(0));
    OVER.with(|c| c.set(false));
    ASKED.with(|c| c.set(false));
    CURRENT.with(|c| c.borrow_mut().clear());

    let seed = js_sys::Date::now() as u64;
    // Generated for the WHOLE ladder even when starting deeper, so round 4 of
    // a re-entry bee draws round 4's tier rather than the opening one.
    let ws = bee::words(&lang, seed, MAX_ROUNDS, kid);
    let empty = ws.len() < start as usize;
    WORDS.with(|w| *w.borrow_mut() = ws);
    ROSTER.with(|r| *r.borrow_mut() = bee::roster(seed, FIELD));
    if empty {
        dom::set_html("beeKeys", "");
        dom::set_text("beeAnnounce", &i18n::t("bee.unavailable"));
        return;
    }
    reflect_official();
    render_keys(&lang);
    render_round(app);
}

fn close() {
    speech_out::stop();
    dom::remove_class("bee", "show");
}

fn word_now() -> Option<String> {
    WORDS.with(|w| w.borrow().get(ROUND.with(Cell::get) as usize - 1).cloned())
}

// ------------------------------------------------------------------ render

fn render_round(app: &App) {
    let n = ROUND.with(Cell::get);
    dom::set_text("beeRound", &i18n::tp("bee.round", &[("n", &n.to_string())]));
    dom::set_text("beeAnnounce", &i18n::t("bee.yourword"));
    dom::set_text("beeSpelling", "");
    CURRENT.with(|c| c.borrow_mut().clear());
    ASKED.with(|c| c.set(false));
    render_word();
    render_strip();
    speak(app, 0.9);
}

/// The contestant strip. An eliminated contestant is dimmed, never crossed out
/// — D4's dignity rule applies to the synthetics too, since a board full of red
/// crosses is the atmosphere this mode is trying not to have.
fn render_strip() {
    if KID.with(Cell::get) {
        // D6: no elimination state EXISTS in Kid Mode, so none can render.
        let html: String = ROSTER.with(|r| {
            r.borrow()
                .iter()
                .map(|c| format!(
                    "<span class=\"bee-c\"><i class=\"bee-av t{}\">{}</i>{}</span>",
                    c.tint, initial(c.name), dom::escape_html(c.name)
                ))
                .collect()
        });
        dom::set_html("beeStrip", &html);
        return;
    }
    let n = ROUND.with(Cell::get);
    let html: String = ROSTER.with(|r| {
        r.borrow()
            .iter()
            .map(|c| {
                let out = matches!(c.out_on, Some(r) if r <= n);
                format!(
                    "<span class=\"bee-c{}\"><i class=\"bee-av t{}\">{}</i>{}</span>",
                    if out { " out" } else { "" },
                    c.tint,
                    initial(c.name),
                    dom::escape_html(c.name)
                )
            })
            .collect()
    });
    dom::set_html("beeStrip", &html);
}

/// The avatar face: one letter, escaped. Names are curated ASCII, but this is
/// rendered into markup, so it is escaped like anything else that came from a
/// table rather than trusted because the table looks safe today.
fn initial(name: &str) -> String {
    dom::escape_html(&name.chars().next().unwrap_or('?').to_uppercase().to_string())
}

fn render_word() {
    CURRENT.with(|c| dom::set_text("beeWord", &c.borrow()));
}

fn render_keys(lang: &str) {
    let mut html = String::new();
    for row in crate::keyboard::unit_rows(lang) {
        html.push_str("<div class=\"bee-row\">");
        for c in row.chars() {
            html.push_str(&format!(
                "<button type=\"button\" class=\"bee-key\" data-k=\"{0}\">{0}</button>",
                dom::escape_html(&c.to_string())
            ));
        }
        html.push_str("</div>");
    }
    dom::set_html("beeKeys", &html);
}

fn reflect_official() {
    let on = official();
    dom::set_text("beeOfficial", if on { "\u{2713}" } else { "\u{2014}" });
    // I5: with Official Rules on there is no way to take a letter back, so the
    // control is REMOVED rather than disabled — a greyed button still invites
    // the tap it will refuse.
    dom::toggle_class("beeDelete", "btn-hide", on);
}

fn toggle_official() {
    let next = if official() { "off" } else { "on" };
    storage::set_raw(OFFICIAL_KEY, next);
    reflect_official();
}

// ------------------------------------------------------------------ play

/// D3: unlimited, free, and never a mark against the speller — only the clean
/// count notices.
fn request_repeat(app: &App, rate: f32) {
    ASKED.with(|c| c.set(true));
    speak(app, rate);
}

fn speak(app: &App, rate: f32) {
    let Some(w) = word_now() else { return };
    let lang = LANG.with(|l| l.borrow().clone());
    let _ = app;
    // CC-ZH-TONE F6. This used to hand the PINYIN half to the device voice, so
    // a Mandarin voice read romanization aloud and zh Bee audio was wrong
    // outright — the main game speaks the hanzi. Mandarin now takes the same
    // server clip as everywhere else, with its reading forced, and never the
    // on-device voice, which cannot be given a reading (Invariant 4).
    if lang == crate::consts::ZH {
        let (pinyin, hanzi) = w.split_once('|').unwrap_or((w.as_str(), w.as_str()));
        if let Some(py) = crate::pinyin::phoneme_reading(pinyin) {
            crate::api::play_word_with(hanzi, Some(&py), "normal", rate as f64, &lang, || {});
        }
        return;
    }
    speech_out::speak(w.split('|').next().unwrap_or(&w), rate, &lang);
}

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
    // I5: committed means committed. Belt and braces with the hidden control.
    if official() || OVER.with(Cell::get) {
        return;
    }
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

fn submit(app: &App) {
    if OVER.with(Cell::get) {
        return;
    }
    let Some(target) = word_now() else { return };
    let typed = CURRENT.with(|c| c.borrow().clone());
    if typed.is_empty() {
        return;
    }
    let target_word = target.split('|').next().unwrap_or(&target);
    // CC-ZH-TONE F2: Mandarin grades through the canonicalizer like every other
    // zh surface. This used to be a plain fold_strict, which looks correct for
    // English and silently drops tone for Mandarin -- lv4 would not match lü4
    // and a tone-perfect answer could be marked wrong. Tone-blindness for
    // Little Speller is the matcher's FLAG, not a branch of its own.
    let correct = if LANG.with(|l| l.borrow().clone()) == crate::consts::ZH {
        let mode = crate::pinyin::ToneMode::for_kid(KID.with(Cell::get));
        crate::pinyin::matches_with(&typed, target_word, mode)
    } else {
        crate::norm::fold_strict(&typed) == crate::norm::fold_strict(target_word)
    };

    if correct {
        if !ASKED.with(Cell::get) {
            CLEAN.with(|c| c.set(c.get() + 1));
        }
        haptics::correct();
        let n = ROUND.with(Cell::get) + 1;
        let have = WORDS.with(|w| w.borrow().len()) as u32;
        if n > have || n > MAX_ROUNDS {
            finish(None); // stood to the end of the ladder
        } else {
            ROUND.with(|c| c.set(n));
            render_round(app);
        }
    } else if KID.with(Cell::get) {
        // D6: no elimination. The word is shown and the bee moves on.
        dom::set_text("beeSpelling", target_word);
        haptics::key_tap();
        let n = ROUND.with(Cell::get) + 1;
        let have = WORDS.with(|w| w.borrow().len()) as u32;
        if n > have || n > MAX_ROUNDS {
            finish(None);
        } else {
            ROUND.with(|c| c.set(n));
            render_round(app);
        }
    } else {
        // D4: the bell, then the spelling shown plainly, then the placement.
        crate::audio_boost::chime();
        haptics::key_tap();
        dom::set_text("beeSpelling", target_word);
        finish(Some(ROUND.with(Cell::get)));
    }
}

fn finish(out_on: Option<u32>) {
    OVER.with(|c| c.set(true));
    speech_out::stop();
    let lang = LANG.with(|l| l.borrow().clone());
    let kid = KID.with(Cell::get);
    let clean = CLEAN.with(Cell::get);

    REENTRY.with(|c| c.set(1));
    if kid {
        // D6: ribbons for the whole field, and no placement at all — a ranking
        // is the judgement this variant exists to avoid.
        dom::set_text("beeOverTitle", &i18n::t("bee.ribbon.title"));
        dom::set_text("beeOverPlace", "");
        dom::set_text("beeOverNext", "");
    } else {
        // No special case here any more: placement understands survival.
        let place = ROSTER.with(|x| bee::placement(&x.borrow(), out_on));
        let best: u32 = storage::get_json(&best_key(&lang)).unwrap_or(u32::MAX);
        if (place as u32) < best {
            storage::set_json(&best_key(&lang), &(place as u32));
        }
        dom::set_text(
            "beeOverTitle",
            &i18n::t(if out_on.is_none() { "bee.win.title" } else { "bee.out.title" }),
        );
        dom::set_text(
            "beeOverPlace",
            &i18n::tp(
                "bee.out.placed",
                &[("n", &place.to_string()), ("m", &(FIELD + 1).to_string())],
            ),
        );
        // D4: the hook. Always a real round, always one below where you fell.
        let back = bee::reentry_round(out_on.unwrap_or(1));
        REENTRY.with(|c| c.set(back));
        dom::set_text("beeOverNext", &i18n::tp("bee.next", &[("n", &back.to_string())]));
    }
    dom::set_text("beeOverClean", &clean.to_string());
    dom::add_class("beeOver", "show");
}
