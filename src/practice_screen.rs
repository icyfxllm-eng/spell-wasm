//! CC-PRACTICE screen — the front porch. Failure-proof by construction: no
//! timers, no lives, no scores; wrong units pulse, replay is free and
//! prominent, and the only thing that ever persists is Practice's own
//! per-language progress record (Invariant I2).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::practice::{self, Phase};
use crate::{api, dom, haptics, i18n, App};

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static POS: Cell<usize> = const { Cell::new(0) };
    static TYPED: RefCell<String> = const { RefCell::new(String::new()) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    /// Intro card currently blocking input (dismiss to continue).
    static INTRO_UP: Cell<bool> = const { Cell::new(false) };
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("practiceOpen", move || open(&a));
    let a = app.clone();
    dom::on_click("prExit", move || close(&a));
    let a = app.clone();
    dom::on_click("prReplay", move || replay(&a));
    let a = app.clone();
    dom::on_click("prIntroOk", move || {
        INTRO_UP.with(|c| c.set(false));
        dom::remove_class("prIntro", "show");
        start_word(&a);
    });
    let a = app.clone();
    dom::on_click("prRestart", move || {
        let lang = LANG.with(|l| l.borrow().clone());
        practice::restart(&lang);
        POS.with(|c| c.set(0));
        render_all(&a);
        begin_pos(&a);
    });
    // Ceremony / graduation / what's-next doors.
    let a = app.clone();
    dom::on_click("prCeremonyGo", move || {
        dom::remove_class("prCeremony", "show");
        begin_pos(&a); // first graduation word
    });
    dom::on_click("prShare", || {
        let lang = LANG.with(|l| l.borrow().clone());
        crate::share::share_practice(&lang);
    });
    let a = app.clone();
    dom::on_click("prNextReplay", move || {
        // Replay the difficult lap: back to word 21.
        let lang = LANG.with(|l| l.borrow().clone());
        let mut p = practice::load(&lang);
        p.pos = practice::FIRST_CONTACT;
        practice::save(&lang, &p);
        POS.with(|c| c.set(practice::FIRST_CONTACT));
        dom::remove_class("prNext", "show");
        render_all(&a);
        begin_pos(&a);
    });
    let a = app.clone();
    dom::on_click("prNextStandard", move || close(&a));
    let a = app.clone();
    dom::on_click("prNextDaily", move || {
        close(&a);
        dom::el("dailyBtn").dyn_ref::<web_sys::HtmlElement>().map(|e| e.click());
    });
    // The typing surface: a plain input; per-unit feedback on every keystroke.
    let a = app.clone();
    dom::on::<web_sys::Event, _>("prInput", "input", move |_| on_typed(&a));
}

pub fn open(app: &App) {
    let lang = app.borrow().lang.clone();
    if !crate::consts::practice(&lang) || practice::curriculum(&lang).is_none() {
        return;
    }
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    let p = practice::load(&lang);
    POS.with(|c| c.set(p.pos.min(practice::TOTAL)));
    OPEN.with(|c| c.set(true));
    dom::add_class("practiceScreen", "show");
    render_all(app);
    if POS.with(Cell::get) >= practice::TOTAL {
        show_next();
    } else {
        begin_pos(app);
    }
}

fn close(app: &App) {
    OPEN.with(|c| c.set(false));
    dom::remove_class("practiceScreen", "show");
    for id in ["prIntro", "prCeremony", "prNext"] {
        dom::remove_class(id, "show");
    }
    api::stop();
    let _ = app;
}

/// Start (or resume) the word at POS: intro card first if this position
/// introduces a trap class that hasn't been shown this run.
fn begin_pos(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    if pos >= practice::TOTAL {
        show_next();
        return;
    }
    if pos == practice::FIRST_CONTACT {
        // Entering the graduation lap: quick heads-up line on the field.
        dom::set_text("prPhaseHint", &i18n::t("practice.graduation"));
    }
    if let Some(t) = practice::intro_at(c, pos) {
        let mut p = practice::load(&lang);
        if !p.shown.contains(&t.id) {
            p.shown.push(t.id.clone());
            practice::save(&lang, &p);
            dom::set_text("prIntroText", &t.intro);
            dom::add_class("prIntro", "show");
            INTRO_UP.with(|x| x.set(true));
            return; // start_word on dismiss
        }
    }
    start_word(app);
}

fn start_word(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    let Some(word) = practice::word_at(c, pos) else { return };
    TYPED.with(|t| t.borrow_mut().clear());
    if let Ok(inp) = dom::el("prInput").dyn_into::<web_sys::HtmlInputElement>() {
        inp.set_value("");
        let _ = inp.focus();
    }
    dom::set_text("prSlots", &slot_line(word, 0));
    // Phase presentation (D2); graduation is always audio-only.
    let ph = if pos >= practice::FIRST_CONTACT { Phase::Audio } else { practice::phase(pos) };
    match ph {
        Phase::Copy => dom::set_text("prWord", word),
        Phase::Flash => {
            dom::set_text("prWord", word);
            let w = word.to_string();
            after(practice::FLASH_MS as i32, move || {
                // Hide only if the same word is still up (player may have advanced).
                if dom::el("prWord").text_content().as_deref() == Some(w.as_str()) {
                    dom::set_text("prWord", "");
                }
            });
        }
        Phase::Audio => dom::set_text("prWord", ""),
    }
    dom::set_text(
        "prPhaseHint",
        &i18n::t(match ph {
            Phase::Copy => "practice.copyHint",
            Phase::Flash => "practice.flashHint",
            Phase::Audio => "practice.audioHint",
        }),
    );
    replay(app); // D3: the orb speaks (slow preset) at word start; replay free.
}

/// Speak the current word with the SLOW preset (D3).
fn replay(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let Some(word) = practice::word_at(c, POS.with(Cell::get)) else { return };
    let w = word.to_string();
    let code = format!("{}-{}", lang, lang.to_uppercase());
    let _ = app;
    api::play_word(&w.clone(), "slow", 1.0, &lang, move || {
        crate::speech_out::speak(&w, 0.55, &code)
    });
}

/// Every keystroke: hold the correct prefix, pulse on a wrong unit, replay
/// audio automatically, never fail (D3).
fn on_typed(app: &App) {
    if INTRO_UP.with(Cell::get) {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    let Some(word) = practice::word_at(c, pos) else { return };
    let Ok(inp) = dom::el("prInput").dyn_into::<web_sys::HtmlInputElement>() else { return };
    let value = inp.value();
    let (ok, complete) = practice::check_prefix(word, &value);
    let value_units = value.chars().count();
    if value_units > ok && !complete {
        // Wrong unit: pulse, auto-replay, hold position (truncate to the good prefix).
        let good: String = word.chars().take(ok).collect();
        inp.set_value(&good);
        dom::add_class("prSlots", "pulse");
        after(400, || dom::remove_class("prSlots", "pulse"));
        haptics::key_tap();
        replay(app);
    }
    dom::set_text("prSlots", &slot_line(word, ok.min(word.chars().count())));
    if complete {
        word_done(app);
    }
}

fn word_done(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pos = POS.with(Cell::get) + 1;
    POS.with(|cll| cll.set(pos));
    let mut p = practice::load(&lang);
    p.pos = pos;
    practice::save(&lang, &p);
    haptics::key_tap();
    render_all(app);
    if pos == practice::FIRST_CONTACT {
        // Ceremony (D8): celebration + share; upsell deliberately absent (no
        // purchase surface exists in this build — Invariant I5 trivially holds).
        dom::add_class("prCeremony", "show");
        return;
    }
    if pos >= practice::TOTAL {
        show_next();
        return;
    }
    let a = app.clone();
    after(450, move || begin_pos(&a));
}

fn show_next() {
    dom::add_class("prNext", "show");
}

/// The path-of-20 dots (+5 graduation stars) and per-word slot line.
fn render_all(app: &App) {
    let pos = POS.with(Cell::get);
    let mut dots = String::new();
    for i in 0..practice::TOTAL {
        let cls = if i < pos { "done" } else if i == pos { "now" } else { "todo" };
        let glyph = if i < practice::FIRST_CONTACT { "●" } else { "★" };
        dots.push_str(&format!("<span class=\"pr-dot {cls}\">{glyph}</span>"));
    }
    dom::set_html("prPath", &dots);
    let _ = app;
}

fn slot_line(word: &str, ok: usize) -> String {
    word.chars()
        .enumerate()
        .map(|(i, _)| if i < ok { '●' } else { '＿' })
        .collect::<Vec<char>>()
        .iter()
        .collect()
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}
