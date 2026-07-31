//! CC-PRACTICE v2 screen — the front porch, now a CONVERSATION loop: ghost
//! letters in the slots, a tile tray with decoys, a coaching orb, trap
//! micro-interactions, echo-spelling, a free hint ladder and choice beats.
//! Failure-proof by construction (D9): no lives, no fail
//! states; wrong tiles wiggle, replay is free and prominent, and the only
//! thing that ever persists is Practice's own per-language progress record
//! (Invariant I2 — hint presses and micro misses are NOT persisted, D11).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::practice::{self, Phase};
use crate::{api, dom, haptics, i18n, App};

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static POS: Cell<usize> = const { Cell::new(0) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    /// A modal card currently blocking input (intro/choice/micro).
    static CARD_UP: Cell<bool> = const { Cell::new(false) };
    /// Generation counter: bumping it cancels every pending flash/echo step.
    static GEN: Cell<u64> = const { Cell::new(0) };
    /// The current word's typing units (tray phases) or NFC chars (audio).
    static UNITS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Correctly placed leading units.
    static FILLED: Cell<usize> = const { Cell::new(0) };
    /// Tray tiles + used flags (index-aligned).
    static TILES: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
    /// D7 hint ladder: slots ghosted up to this absolute unit index…
    static REVEALED: Cell<usize> = const { Cell::new(0) };
    /// …or the whole word (step 3).
    static GHOST_ALL: Cell<bool> = const { Cell::new(false) };
    /// D6 echo-spelling in flight (tap prSlots to skip).
    static ECHOING: Cell<bool> = const { Cell::new(false) };
    /// D2 chunk delivery: unit counts at each chunk boundary + spoken-up-to.
    static CHUNK_ENDS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    static CHUNK_SPOKEN: Cell<usize> = const { Cell::new(0) };
    /// D5 micro-interaction state: (kind, needed taps, misses, plays left).
    static MICRO: RefCell<Option<Micro>> = const { RefCell::new(None) };
}

struct Micro {
    /// Remaining correct option texts (TAP_STACK needs two; others one).
    needed: Vec<String>,
    misses: u32,
    plays_left: u32,
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("practiceOpen", move || open(&a));
    let a = app.clone();
    dom::on_click("prExit", move || close(&a));
    let a = app.clone();
    dom::on_click("prReplay", move || replay(&a));
    let a = app.clone();
    dom::on_click("prHintBtn", move || hint(&a));
    let a = app.clone();
    dom::on_click("prIntroOk", move || {
        dom::remove_class("prIntro", "show");
        // Intro dismissed → the trap's micro-interaction, if it has one (D5).
        let lang = LANG.with(|l| l.borrow().clone());
        let pos = POS.with(Cell::get);
        let has_micro = practice::curriculum(&lang)
            .and_then(|c| practice::intro_at(c, pos))
            .is_some_and(|t| t.template.is_some());
        if has_micro {
            micro_open(&a);
        } else {
            CARD_UP.with(|c| c.set(false));
            start_word(&a);
        }
    });
    let a = app.clone();
    dom::on_click("prRestart", move || {
        let lang = LANG.with(|l| l.borrow().clone());
        practice::restart(&lang);
        POS.with(|c| c.set(0));
        render_all(&a);
        begin_pos(&a);
    });
    // D8 choice beat buttons.
    let a = app.clone();
    dom::on_click("prChoiceA", move || choice_pick(&a, 0));
    let a = app.clone();
    dom::on_click("prChoiceB", move || choice_pick(&a, 1));
    // D5 micro option taps (delegated) + the HEAR_PICK replay.
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("prMicroOpts", "click", move |e| micro_tap(&a, &e));
    let a = app.clone();
    dom::on_click("prMicroPlay", move || micro_play(&a));
    // D3 tile tray (delegated).
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("prTray", "click", move |e| tile_tap(&a, &e));
    // D6: tapping the slot line skips an in-flight echo.
    dom::on::<web_sys::MouseEvent, _>("prSlots", "click", move |_| {
        if ECHOING.with(Cell::get) {
            GEN.with(|g| g.set(g.get() + 1));
            ECHOING.with(|c| c.set(false));
        }
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
    // Phase-3 typing surface: per-keystroke feedback, ko-IME-safe.
    let a = app.clone();
    dom::on::<web_sys::Event, _>("prInput", "input", move |_| on_typed(&a));
    dom::on_before_input("prInput", |ty, len| {
        crate::input_provenance::note_insert("prInput", &ty, len)
    });
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
    GEN.with(|g| g.set(g.get() + 1));
    ECHOING.with(|c| c.set(false));
    CARD_UP.with(|c| c.set(false));
    dom::remove_class("practiceScreen", "show");
    for id in ["prIntro", "prCeremony", "prNext", "prChoice", "prMicro"] {
        dom::remove_class(id, "show");
    }
    api::stop();
    let _ = app;
}

fn cur_word(lang: &str, pos: usize) -> Option<String> {
    let c = practice::curriculum(lang)?;
    let p = practice::load(lang);
    practice::word_at(c, pos, &p).map(str::to_string)
}

/// Start (or resume) the position at POS: choice beat first (D8), then the
/// trap intro card (D4), then its micro-interaction (D5), then the word.
fn begin_pos(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    if pos >= practice::TOTAL {
        show_next();
        return;
    }
    if pos == practice::FIRST_CONTACT {
        dom::set_text("prPhaseHint", &i18n::t("practice.graduation"));
    }
    // D8: a choice beat fires before anything else at its position — unless a
    // pick is already recorded (resume replays identically).
    if pos < practice::FIRST_CONTACT {
        let p = practice::load(&lang);
        if let Some(b) = practice::choice_at(c, pos) {
            if !p.picks.iter().any(|k| k.at == pos) {
                dom::set_text(
                    "prChoiceA",
                    &format!("{} {}", b.emoji.first().map(String::as_str).unwrap_or(""), c.words[pos]),
                );
                dom::set_text(
                    "prChoiceB",
                    &format!("{} {}", b.emoji.get(1).map(String::as_str).unwrap_or(""), b.alt),
                );
                dom::add_class("prChoice", "show");
                CARD_UP.with(|x| x.set(true));
                return; // continues in choice_pick
            }
        }
    }
    after_choice(app, c, pos, &lang);
}

fn after_choice(app: &App, c: &'static practice::Curriculum, pos: usize, lang: &str) {
    if let Some(t) = practice::intro_at(c, pos) {
        let mut p = practice::load(lang);
        if !p.shown.contains(&t.id) {
            p.shown.push(t.id.clone());
            practice::save(lang, &p);
            dom::set_text("prIntroText", &t.intro);
            dom::add_class("prIntro", "show");
            CARD_UP.with(|x| x.set(true));
            return; // continues in prIntroOk → micro_open/start_word
        }
    }
    CARD_UP.with(|x| x.set(false));
    start_word(app);
}

fn choice_pick(app: &App, which: usize) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    if let Some(b) = practice::choice_at(c, pos) {
        let word = if which == 0 { c.words[pos].clone() } else { b.alt.clone() };
        let mut p = practice::load(&lang);
        p.picks.retain(|k| k.at != pos);
        p.picks.push(practice::Pick { at: pos, word });
        practice::save(&lang, &p);
    }
    dom::remove_class("prChoice", "show");
    haptics::key_tap();
    after_choice(app, c, pos, &lang);
}

// ---------- D5 micro-interactions ----------

fn micro_open(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pos = POS.with(Cell::get);
    let Some(c) = practice::curriculum(&lang) else { return };
    let Some(trap) = practice::intro_at(c, pos) else { return };
    let Some(tpl) = &trap.template else { return };
    let Some(word) = cur_word(&lang, pos) else { return };
    let seed = practice::seed_for(&lang, &word);
    let (prompt_key, opts, needed, has_audio): (&str, Vec<String>, Vec<String>, bool) = match tpl.id.as_str() {
        "TAP_SILENT_UNIT" => {
            let unit = tpl.params.get("unit").cloned().unwrap_or_default();
            (
                "practice.micro.silent",
                practice::units(&lang, &word),
                vec![unit],
                false,
            )
        }
        "HEAR_PICK" => {
            let foil = tpl.params.get("foil").cloned().unwrap_or_default();
            let mut o = vec![word.clone(), foil];
            if seed & 1 == 1 {
                o.swap(0, 1);
            }
            ("practice.micro.hear", o, vec![word.clone()], true)
        }
        "TAP_STACK" => {
            let pair = tpl.params.get("pair").cloned().unwrap_or_default();
            let parts: Vec<String> = pair.chars().map(|ch| ch.to_string()).collect();
            let mut o = parts.clone();
            for u in practice::units(&lang, &word) {
                if o.len() >= 5 {
                    break;
                }
                if !o.contains(&u) {
                    o.push(u);
                }
            }
            // Deterministic order (seed-rotated, not shuffled — tiny set).
            let k = (seed % o.len().max(1) as u64) as usize;
            o.rotate_left(k);
            ("practice.micro.stack", o, parts, false)
        }
        _ => return,
    };
    dom::set_text("prMicroPrompt", &i18n::t(prompt_key));
    let mut html = String::new();
    for (i, o) in opts.iter().enumerate() {
        html.push_str(&format!(
            "<button type=\"button\" class=\"pr-tile\" data-mi=\"{i}\" data-u=\"{}\">{}</button>",
            dom::escape_html(o),
            dom::escape_html(o)
        ));
    }
    dom::set_html("prMicroOpts", &html);
    dom::toggle_class("prMicroPlay", "btn-hide", !has_audio);
    MICRO.with(|m| *m.borrow_mut() = Some(Micro { needed, misses: 0, plays_left: 1 }));
    dom::add_class("prMicro", "show");
    CARD_UP.with(|x| x.set(true));
    if has_audio {
        replay_word_audio(app, &lang, &word);
    }
    let _ = app;
}

fn micro_play(app: &App) {
    let ok = MICRO.with(|m| {
        let mut b = m.borrow_mut();
        match b.as_mut() {
            Some(mi) if mi.plays_left > 0 => {
                mi.plays_left -= 1;
                true
            }
            _ => false,
        }
    });
    if !ok {
        return; // D5: two audio plays total — the button just goes quiet
    }
    let lang = LANG.with(|l| l.borrow().clone());
    if let Some(word) = cur_word(&lang, POS.with(Cell::get)) {
        replay_word_audio(app, &lang, &word);
    }
}

fn micro_tap(app: &App, e: &web_sys::MouseEvent) {
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|t| t.closest("[data-u]").ok().flatten())
    else {
        return;
    };
    let text = el.get_attribute("data-u").unwrap_or_default();
    let done = MICRO.with(|m| {
        let mut b = m.borrow_mut();
        let Some(mi) = b.as_mut() else { return Some(false) };
        if let Some(i) = mi.needed.iter().position(|n| *n == text) {
            mi.needed.remove(i);
            el.class_list().add_1("good").ok();
            haptics::key_tap();
            if mi.needed.is_empty() { Some(true) } else { None }
        } else {
            mi.misses += 1;
            el.class_list().add_1("wiggle").ok();
            let el2 = el.clone();
            after(380, move || {
                el2.class_list().remove_1("wiggle").ok();
            });
            if mi.misses >= practice::MICRO_MISSES { Some(false) } else { None }
        }
    });
    match done {
        Some(true) => micro_done(app, false),
        Some(false) => micro_done(app, true), // two misses → unfailable auto-reveal
        None => {}
    }
}

/// D5: close-out — on auto-reveal, light the correct options and let the coach
/// give the reveal line; either way the word starts right after.
fn micro_done(app: &App, auto_reveal: bool) {
    let lang = LANG.with(|l| l.borrow().clone());
    let needed = MICRO.with(|m| m.borrow().as_ref().map(|mi| mi.needed.clone()).unwrap_or_default());
    if auto_reveal {
        if let Ok(list) = dom::el("prMicroOpts").query_selector_all("[data-u]") {
            for i in 0..list.length() {
                if let Some(o) = list.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                    if needed.contains(&o.get_attribute("data-u").unwrap_or_default()) {
                        o.class_list().add_1("good").ok();
                    }
                }
            }
        }
        coach(app, CoachSlot::Reveal);
    }
    MICRO.with(|m| *m.borrow_mut() = None);
    let a = app.clone();
    after(if auto_reveal { 1500 } else { 600 }, move || {
        dom::remove_class("prMicro", "show");
        CARD_UP.with(|x| x.set(false));
        start_word(&a);
    });
    let _ = lang;
}

// ---------- the word loop ----------

fn start_word(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pos = POS.with(Cell::get);
    let Some(word) = cur_word(&lang, pos) else { return };
    GEN.with(|g| g.set(g.get() + 1));
    let gen = GEN.with(Cell::get);
    ECHOING.with(|c| c.set(false));
    FILLED.with(|c| c.set(0));
    REVEALED.with(|c| c.set(0));
    GHOST_ALL.with(|c| c.set(false));
    dom::set_text("prBuilt", "");
    dom::set_text("prCoach", "");

    // Graduation is always audio-only (D2).
    let ph = if pos >= practice::FIRST_CONTACT { Phase::Audio } else { practice::phase(pos) };

    // Coach setup line at each trap block start + phase announcements (D4).
    let c = practice::curriculum(&lang);
    if let Some(c) = c {
        if practice::intro_at(c, pos).is_some() {
            coach(app, CoachSlot::Setup);
        } else if pos == practice::GHOST_UNTIL || pos == practice::CUED_UNTIL {
            coach(app, CoachSlot::PhaseShift);
        }
    }

    match ph {
        Phase::Audio => {
            // The real keyboard fades in and replaces the tray (D2) — the
            // transition to base-game input is itself part of the curriculum.
            UNITS.with(|u| *u.borrow_mut() = word.chars().map(|ch| ch.to_string()).collect());
            TILES.with(|t| t.borrow_mut().clear());
            dom::set_html("prTray", "");
            dom::remove_class("prInput", "btn-hide");
            if let Ok(inp) = dom::el("prInput").dyn_into::<web_sys::HtmlInputElement>() {
                inp.set_value("");
                let _ = inp.focus();
            }
            dom::set_text("prWord", "");
            // D2 chunk delivery (data-driven; es syllables today).
            let chunks = practice::chunks(&lang, &word);
            let mut ends = Vec::new();
            let mut acc = 0;
            for ch in &chunks {
                acc += ch.chars().count();
                ends.push(acc);
            }
            CHUNK_ENDS.with(|e| *e.borrow_mut() = if chunks.len() > 1 { ends } else { Vec::new() });
            CHUNK_SPOKEN.with(|s| s.set(0));
            if chunks.len() > 1 {
                speak_chunk(app, &lang, &chunks[0]);
                CHUNK_SPOKEN.with(|s| s.set(1));
            } else {
                replay(app);
            }
        }
        tray_phase => {
            let units = practice::units(&lang, &word);
            UNITS.with(|u| *u.borrow_mut() = units.clone());
            dom::add_class("prInput", "btn-hide");
            let Some(c) = c else { return };
            let tiles = practice::tray(&lang, c, pos, &word);
            TILES.with(|t| *t.borrow_mut() = tiles.iter().map(|x| (x.clone(), false)).collect());
            render_tray();
            CHUNK_ENDS.with(|e| e.borrow_mut().clear());
            if tray_phase == Phase::Cued {
                // Flash ~2s then hide (generation-guarded).
                dom::set_text("prWord", &word);
                let a = app.clone();
                after(practice::FLASH_MS as i32, move || {
                    if GEN.with(Cell::get) == gen {
                        dom::set_text("prWord", "");
                    }
                    let _ = &a;
                });
            } else {
                dom::set_text("prWord", "");
            }
            replay(app);
        }
    }
    render_slots(&lang, ph);
    dom::set_text(
        "prPhaseHint",
        &i18n::t(match ph {
            Phase::GhostTrace => "practice.copyHint",
            Phase::Cued => "practice.flashHint",
            Phase::Audio => "practice.audioHint",
        }),
    );
}

/// The slot line: one span per unit. Ghost letters show in ghost-trace phase,
/// on hint reveals (D7 steps 1–2) and on the step-3 full ghost.
fn render_slots(lang: &str, ph: Phase) {
    let filled = FILLED.with(Cell::get);
    let revealed = REVEALED.with(Cell::get);
    let ghost_all = GHOST_ALL.with(Cell::get) || ph == Phase::GhostTrace;
    let mut html = String::new();
    UNITS.with(|u| {
        for (i, unit) in u.borrow().iter().enumerate() {
            let (cls, text) = if i < filled {
                ("pr-slot fill", unit.clone())
            } else if ghost_all || i < revealed {
                ("pr-slot ghost", unit.clone())
            } else {
                ("pr-slot", "\u{00a0}".to_string())
            };
            html.push_str(&format!("<span class=\"{cls}\">{}</span>", dom::escape_html(&text)));
        }
    });
    dom::set_html("prSlots", &html);
    // ko: the assembled blocks under the jamo slots (I8 — real IME assembly).
    if lang == "ko" && ph != Phase::Audio {
        let built: Vec<String> = UNITS.with(|u| u.borrow().iter().take(filled).cloned().collect());
        dom::set_text("prBuilt", &practice::assemble(lang, &built));
    }
}

fn render_tray() {
    let mut html = String::new();
    TILES.with(|t| {
        for (i, (unit, used)) in t.borrow().iter().enumerate() {
            html.push_str(&format!(
                "<button type=\"button\" class=\"pr-tile{}\" data-ti=\"{i}\">{}</button>",
                if *used { " used" } else { "" },
                dom::escape_html(unit)
            ));
        }
    });
    dom::set_html("prTray", &html);
}

fn tile_tap(app: &App, e: &web_sys::MouseEvent) {
    if CARD_UP.with(Cell::get) || ECHOING.with(Cell::get) {
        return;
    }
    let Some(el) = e
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|t| t.closest("[data-ti]").ok().flatten())
    else {
        return;
    };
    let Some(i) = el.get_attribute("data-ti").and_then(|v| v.parse::<usize>().ok()) else { return };
    let lang = LANG.with(|l| l.borrow().clone());
    let expect = UNITS.with(|u| u.borrow().get(FILLED.with(Cell::get)).cloned());
    let Some(expect) = expect else { return };
    let tile = TILES.with(|t| t.borrow().get(i).cloned());
    let Some((unit, used)) = tile else { return };
    if used {
        return;
    }
    if unit == expect {
        // D2 ghost-trace: each matched unit solidifies with a haptic tick.
        TILES.with(|t| t.borrow_mut()[i].1 = true);
        FILLED.with(|c| c.set(c.get() + 1));
        haptics::key_tap();
        render_tray();
        let ph = phase_now();
        render_slots(&lang, ph);
        let done = UNITS.with(|u| FILLED.with(Cell::get) >= u.borrow().len());
        if done {
            echo_then_done(app);
        }
    } else {
        // D9: wrong tile wiggles, audio replays, position holds. No fail.
        el.class_list().add_1("wiggle").ok();
        let el2 = el.clone();
        after(380, move || {
            el2.class_list().remove_1("wiggle").ok();
        });
        haptics::key_tap();
        replay(app);
    }
}

fn phase_now() -> Phase {
    let pos = POS.with(Cell::get);
    if pos >= practice::FIRST_CONTACT { Phase::Audio } else { practice::phase(pos) }
}

/// D7 — the hint ladder: press 1 ghosts the first unresolved unit, press 2
/// the next, press 3 ghosts the whole word; further presses keep ghosting the
/// next unit so there is ALWAYS a next move. Free, unlimited, untracked.
fn hint(app: &App) {
    if CARD_UP.with(Cell::get) {
        return;
    }
    let filled = FILLED.with(Cell::get);
    let revealed = REVEALED.with(Cell::get).max(filled);
    if GHOST_ALL.with(Cell::get) || revealed >= filled.saturating_add(2) {
        GHOST_ALL.with(|c| c.set(true));
        REVEALED.with(|c| c.set(revealed + 1));
    } else {
        REVEALED.with(|c| c.set(revealed + 1));
    }
    let lang = LANG.with(|l| l.borrow().clone());
    render_slots(&lang, phase_now());
    // In tray phases, also light the tile the player needs next.
    if phase_now() != Phase::Audio {
        let expect = UNITS.with(|u| u.borrow().get(FILLED.with(Cell::get)).cloned());
        if let Some(expect) = expect {
            TILES.with(|t| {
                if let Some(idx) = t.borrow().iter().position(|(u, used)| !used && *u == expect) {
                    if let Ok(Some(el)) =
                        dom::el("prTray").query_selector(&format!("[data-ti=\"{idx}\"]"))
                    {
                        el.class_list().add_1("hintlit").ok();
                        let el2 = el.clone();
                        after(1200, move || {
                            el2.class_list().remove_1("hintlit").ok();
                        });
                    }
                }
            });
        }
    }
    let _ = app;
}

/// Speak the current word with the SLOW preset (D9), or the current chunk in
/// chunked phase-3 delivery.
fn replay(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(word) = cur_word(&lang, POS.with(Cell::get)) else { return };
    let chunked = CHUNK_ENDS.with(|e| !e.borrow().is_empty());
    if chunked {
        let idx = CHUNK_SPOKEN.with(Cell::get).saturating_sub(1);
        let chunks = practice::chunks(&lang, &word);
        if let Some(ch) = chunks.get(idx) {
            speak_chunk(app, &lang, ch);
            return;
        }
    }
    replay_word_audio(app, &lang, &word);
}

fn replay_word_audio(app: &App, lang: &str, word: &str) {
    let w = word.to_string();
    let code = format!("{}-{}", lang, lang.to_uppercase());
    let _ = app;
    api::play_word(&w.clone(), "slow", 1.0, lang, move || {
        crate::speech_out::speak(&w, 0.55, &code)
    });
}

fn speak_chunk(app: &App, lang: &str, chunk: &str) {
    let code = format!("{}-{}", lang, lang.to_uppercase());
    crate::speech_out::speak(chunk, 0.55, &code);
    let _ = app;
}

/// Phase-3 keystrokes: hold the correct prefix, pulse on a wrong unit, replay
/// audio automatically, never fail (D9). ko composition is never "wrong"
/// mid-block (practice::prefix_viable, I8).
fn on_typed(app: &App) {
    if CARD_UP.with(Cell::get) || ECHOING.with(Cell::get) {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let pos = POS.with(Cell::get);
    let Some(word) = cur_word(&lang, pos) else { return };
    let Ok(inp) = dom::el("prInput").dyn_into::<web_sys::HtmlInputElement>() else { return };
    let value = inp.value();
    let (ok, complete) = practice::check_prefix(&lang, &word, &value);
    if !practice::prefix_viable(&lang, &word, &value) && !complete {
        // Wrong unit: truncate to the good prefix, pulse, auto-replay.
        let good: String = word.chars().take(ok).collect();
        inp.set_value(&good);
        dom::add_class("prSlots", "pulse");
        after(400, || dom::remove_class("prSlots", "pulse"));
        haptics::key_tap();
        replay(app);
    }
    FILLED.with(|c| c.set(ok.min(word.chars().count())));
    render_slots(&lang, Phase::Audio);
    // Chunked delivery: crossing a chunk boundary speaks the next chunk (D2).
    let next_chunk = CHUNK_ENDS.with(|e| {
        let ends = e.borrow();
        if ends.is_empty() {
            return None;
        }
        let spoken = CHUNK_SPOKEN.with(Cell::get);
        if spoken < ends.len() && ok >= ends[spoken - 1] {
            Some(spoken)
        } else {
            None
        }
    });
    if let Some(idx) = next_chunk {
        let chunks = practice::chunks(&lang, &word);
        if let Some(ch) = chunks.get(idx) {
            speak_chunk(app, &lang, ch);
            CHUNK_SPOKEN.with(|s| s.set(idx + 1));
        }
    }
    if complete {
        echo_then_done(app);
    }
}

// ---------- D6 echo-spelling ----------

/// After every correct word: the orb spells it back unit-by-unit while each
/// unit lights in sequence. Tap the slots to skip. Then the word completes.
fn echo_then_done(app: &App) {
    ECHOING.with(|c| c.set(true));
    GEN.with(|g| g.set(g.get() + 1));
    let gen = GEN.with(Cell::get);
    let lang = LANG.with(|l| l.borrow().clone());
    // Echo highlights every slot as filled first.
    FILLED.with(|c| c.set(UNITS.with(|u| u.borrow().len())));
    render_slots(&lang, phase_now());
    echo_step(app.clone(), 0, gen);
}

fn echo_step(app: App, i: usize, gen: u64) {
    if GEN.with(Cell::get) != gen {
        // Skipped or superseded — settle the word now.
        if ECHOING.with(Cell::get) {
            ECHOING.with(|c| c.set(false));
        }
        word_done(&app);
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let n = UNITS.with(|u| u.borrow().len());
    if i >= n {
        ECHOING.with(|c| c.set(false));
        word_done(&app);
        return;
    }
    // Light unit i.
    if let Ok(Some(el)) = dom::el("prSlots").query_selector(&format!(".pr-slot:nth-child({})", i + 1)) {
        el.class_list().add_1("echo").ok();
        let el2 = el.clone();
        after(practice::ECHO_UNIT_MS as i32, move || {
            el2.class_list().remove_1("echo").ok();
        });
    }
    let unit = UNITS.with(|u| u.borrow().get(i).cloned()).unwrap_or_default();
    let code = format!("{}-{}", lang, lang.to_uppercase());
    crate::speech_out::speak(&unit, 0.9, &code);
    after(practice::ECHO_UNIT_MS as i32, move || echo_step(app, i + 1, gen));
}

// ---------- coach orb (D4) ----------

enum CoachSlot {
    Done,
    Setup,
    PhaseShift,
    Reveal,
    Ceremony,
}

/// A deterministic coach line for the slot (D4): bubble text + spoken via the
/// existing TTS pipeline. Empty pools stay silent — never a placeholder.
fn coach(app: &App, slot: CoachSlot) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(c) = practice::curriculum(&lang) else { return };
    let pos = POS.with(Cell::get);
    let word = cur_word(&lang, pos).unwrap_or_default();
    let seed = practice::seed_for(&lang, &word);
    let line = match slot {
        CoachSlot::Done => practice::coach_line(&c.coach.word_done, pos, seed),
        CoachSlot::Setup => practice::coach_line(&c.coach.word_setup, pos, seed),
        CoachSlot::PhaseShift => {
            // Direct index: line 0 announces Cued, the last announces the
            // keyboard — written in that order in the pool.
            let p = &c.coach.phase;
            if pos >= practice::CUED_UNTIL { p.last() } else { p.first() }.map(String::as_str)
        }
        CoachSlot::Reveal => practice::coach_line(&c.coach.reveal, pos, seed),
        CoachSlot::Ceremony => c.coach.ceremony.first().map(String::as_str),
    };
    if let Some(line) = line {
        dom::set_text("prCoach", line);
        let code = format!("{}-{}", lang, lang.to_uppercase());
        crate::speech_out::speak(line, 0.9, &code);
    }
    let _ = app;
}

fn word_done(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pos = POS.with(Cell::get) + 1;
    POS.with(|cll| cll.set(pos));
    let mut p = practice::load(&lang);
    p.pos = pos;
    practice::save(&lang, &p);
    haptics::key_tap();
    coach(app, CoachSlot::Done);
    render_all(app);
    if pos == practice::FIRST_CONTACT {
        // Ceremony (D12): celebration + share; upsell deliberately absent (no
        // purchase surface exists in this build — I6 trivially holds).
        coach(app, CoachSlot::Ceremony);
        dom::add_class("prCeremony", "show");
        return;
    }
    if pos >= practice::TOTAL {
        show_next();
        return;
    }
    let a = app.clone();
    after(650, move || begin_pos(&a));
}

fn show_next() {
    dom::add_class("prNext", "show");
}

/// The path-of-20 dots (+5 graduation stars).
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

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}
