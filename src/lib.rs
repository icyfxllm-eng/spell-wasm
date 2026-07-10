mod achievements;
mod agegate;
mod api;
mod audio_boost;
mod board;
mod climb;
mod consts;
mod daily;
mod deck;
mod dom;
mod drawing;
mod game;
mod haptics;
mod hangul;
mod i18n;
mod importer;
mod keyboard;
mod misses;
mod model;
mod native_audio;
mod norm;
mod notifications;
mod profanity;
mod settings;
mod share;
mod speech_out;
mod stats;
mod storage;
mod versus;
mod viet;
mod word_data;
mod words;
mod wordstats;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use consts::{EN, MINE};
use model::AppState;

pub type App = Rc<RefCell<AppState>>;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let mut state = AppState::default();
    settings::load_prefs(&mut state);
    importer::load_custom(&mut state);
    misses::load(&mut state);
    achievements::load(&mut state);
    stats::load(&mut state);

    state.lang = match state.last_lang.clone() {
        Some(l) if l == MINE && !state.custom.words.is_empty() => MINE.to_string(),
        _ => EN.to_string(),
    };
    state.cur_lang = state.lang.clone();

    // Age gate: a stored "kid" verdict (under the cutoff) locks Kid Mode on
    // every launch; leaving it needs the parent gate. First-launch / existing
    // users with no stored verdict get the DOB prompt after wiring (below).
    if agegate::is_kid_locked() {
        state.kid = true;
        state.age_locked = true;
    }

    let app: App = Rc::new(RefCell::new(state));

    let glow = app.borrow().glow.clone();
    settings::set_glow(&app, &glow);
    let bg_color = app.borrow().bg_color.clone();
    settings::set_bg_color(&app, &bg_color);
    let orb_color = app.borrow().orb_color.clone();
    settings::set_orb_color(&app, &orb_color);
    settings::apply_settings(&app);
    i18n::init(&app.borrow().lang);
    game::build_source_options(&app);
    game::build_level_options(&app);
    game::refresh_mode_buttons(&app);
    game::refresh_daily_btn(&app);
    game::render_letters(&app, false);
    i18n::translate_page();
    {
        let app2 = app.clone();
        speech_out::setup_voice_loading(move || game::update_voice_note(&app2));
    }
    stats::render(&app.borrow());
    achievements::render(&app.borrow());
    board::render(&app.borrow());

    // Native build only: warm the on-device audio cache for the current
    // English tier so the first turns are instant and work offline.
    game::preload_pool(&app);

    // Re-assert the daily-reminder schedule from saved prefs (native only).
    settings::apply_reminder(&app.borrow());

    // Reveal the Share button only where a share sheet actually exists
    // (native app or mobile web); it stays hidden on desktop browsers.
    if share::available() {
        dom::remove_class("shareBtn", "btn-hide");
    }

    wire(&app);

    // First launch (or existing user pre-dating the age gate): ask DOB before
    // anything else. The scrim sits above the game and must be answered.
    if agegate::stored().is_none() {
        agegate::populate_selects();
        dom::add_class("ageScrim", "show");
    }

    Ok(())
}

fn wire(app: &App) {
    wire_orb_and_answer(app);
    wire_glow_and_settings(app);
    wire_drawing(app);
    wire_modes(app);
    wire_source_level(app);
    wire_import(app);
    wire_stats_board_modal(app);
    wire_versus(app);
    wire_age_gate(app);
    keyboard::setup(app);
    climb::setup(app);
}

thread_local! {
    /// Expected answer to the current parent-gate math challenge.
    static PARENT_ANSWER: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
}

/// Opens the parent gate (a fresh worded-math challenge). Passing it lets a
/// grown-up re-run DOB entry / unlock the full app — the only way out of a
/// kid-locked device, so it's never offered directly in kid-reachable UI.
fn open_parent_gate() {
    let (q, ans) = agegate::parent_problem();
    PARENT_ANSWER.with(|c| c.set(ans));
    dom::set_text("parentQ", &q);
    dom::input("parentAnswer").set_value("");
    dom::set_text("parentErr", "");
    dom::add_class("parentScrim", "show");
    dom::input("parentAnswer").focus().ok();
}

fn wire_age_gate(app: &App) {
    // DOB prompt → compute age locally, store the verdict only.
    {
        let a = app.clone();
        dom::on_click("ageSubmit", move || {
            let (y, m, d) = agegate::read_selection();
            if !agegate::valid_date(y, m, d) {
                dom::set_text("ageErr", "Please pick a real date that isn't in the future.");
                return;
            }
            let was_locked = a.borrow().age_locked;
            let full = agegate::save(agegate::age_from(y, m, d));
            {
                let mut s = a.borrow_mut();
                if full {
                    s.age_locked = false;
                    // Reaching a full verdict via the parent gate reveals the
                    // full app; a first-launch full verdict leaves prefs as-is.
                    if was_locked {
                        s.kid = false;
                    }
                } else {
                    s.kid = true;
                    s.age_locked = true;
                }
            }
            settings::save_prefs(&a.borrow());
            settings::apply_settings(&a);
            dom::set_text("ageErr", "");
            dom::remove_class("ageScrim", "show");
        });
    }
    // Parent gate answer (uses no app state — just the challenge answer).
    dom::on_click("parentSubmit", || {
        let expected = PARENT_ANSWER.with(|c| c.get());
        let given = dom::input("parentAnswer").value().trim().parse::<i32>().unwrap_or(i32::MIN);
        if given == expected {
            dom::remove_class("parentScrim", "show");
            dom::set_text("parentErr", "");
            // Let the grown-up re-run DOB entry (an adult date unlocks).
            agegate::populate_selects();
            dom::add_class("ageScrim", "show");
        } else {
            dom::set_text("parentErr", "Not quite \u{2014} try again.");
        }
    });
    dom::on_click("parentCancel", || dom::remove_class("parentScrim", "show"));
    dom::on::<web_sys::KeyboardEvent, _>("parentAnswer", "keydown", |e| {
        if e.key() == "Enter" {
            if let Some(el) = dom::el("parentSubmit").dyn_ref::<web_sys::HtmlElement>() {
                el.click();
            }
        }
    });
}

fn wire_orb_and_answer(app: &App) {
    {
        let a = app.clone();
        dom::on_click("orbWrap", move || {
            audio_boost::unlock();
            let (answered, active) = {
                let s = a.borrow();
                (s.answered, game::has_active_word(&s))
            };
            if !active || answered {
                game::next_word(&a);
            } else {
                game::speak_current(&a);
            }
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::KeyboardEvent, _>("orbWrap", "keydown", move |e| {
            let key = e.key();
            if key == "Enter" || key == " " {
                e.prevent_default();
                let (answered, active) = {
                    let s = a.borrow();
                    (s.answered, game::has_active_word(&s))
                };
                if !active || answered {
                    game::next_word(&a);
                } else {
                    game::speak_current(&a);
                }
            }
        });
    }
    {
        let a = app.clone();
        dom::on_click("replayBtn", move || game::speak_current(&a));
    }
    {
        let a = app.clone();
        dom::on_click("slowBtn", move || game::replay_slow(&a));
    }
    {
        let a = app.clone();
        dom::on_click("shareBtn", move || {
            let s = a.borrow();
            share::share_result(s.streak, s.best);
        });
    }
    {
        let a = app.clone();
        dom::on_click("checkBtn", move || {
            // If the player drew their answer instead of typing it (and
            // hasn't already run "Read my writing" to fill the box), read
            // the drawing first so Check works directly on a drawn
            // submission — not just on typed/spoken text.
            if a.borrow().answer.trim().is_empty() && drawing::has_strokes() {
                let a2 = a.clone();
                dom::set_disabled("checkBtn", true);
                spawn_local(async move {
                    dom::set_text("drawStatus", "Reading your writing\u{2026}");
                    match drawing::read_writing().await {
                        drawing::OcrOutcome::Confident(txt) => {
                            game::set_answer(&a2, &txt);
                            dom::set_text("drawStatus", &format!("Read \u{201c}{}\u{201d} \u{2014} checking\u{2026}", txt));
                            dom::set_disabled("checkBtn", false);
                            game::submit_guess(&a2);
                        }
                        drawing::OcrOutcome::Unsure(txt) => {
                            // Not confident enough to score automatically —
                            // a bad read would otherwise silently mark a
                            // correctly-spelled word wrong. Fill the box for
                            // the player to confirm or fix, then Check again.
                            game::set_answer(&a2, &txt);
                            dom::set_text("drawStatus", &format!("Not sure I read that right \u{2014} got \u{201c}{}\u{201d}. Fix it if needed, then press Check again.", txt));
                            dom::set_disabled("checkBtn", false);
                        }
                        drawing::OcrOutcome::Empty => {
                            dom::set_text("drawStatus", "Couldn't read that \u{2014} try clearer block letters, or type it.");
                            dom::set_disabled("checkBtn", false);
                        }
                        drawing::OcrOutcome::Failed => {
                            dom::set_text("drawStatus", "The handwriting reader couldn't load here \u{2014} type the letters to check.");
                            dom::set_disabled("checkBtn", false);
                        }
                    }
                });
                return;
            }
            game::submit_guess(&a);
        });
    }
    // NOTE: the answer is typed via the custom on-screen keyboard + physical
    // keydown (see wire_keyboard) — there is no #guess <input>, so the system
    // keyboard (dictation / autocorrect) never opens during a round.
    {
        let a = app.clone();
        dom::on_click("hintBtn", move || game::show_hint(&a));
    }
    {
        let a = app.clone();
        dom::on_click("defBtn", move || game::show_definition_hint(&a));
    }
    {
        let a = app.clone();
        dom::on_click("sentenceBtn", move || game::show_sentence_hint(&a));
    }
    {
        let a = app.clone();
        dom::on_click("giveupBtn", move || game::give_up(&a));
    }
}

fn wire_swatches(selector: &str, app: &App, setter: fn(&App, &str)) {
    if let Ok(list) = dom::doc().query_selector_all(selector) {
        for i in 0..list.length() {
            if let Some(node) = list.get(i) {
                if let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() {
                    let a = app.clone();
                    let color = el.get_attribute("data-c").unwrap_or_default();
                    let cb = Closure::<dyn FnMut()>::new(move || setter(&a, &color));
                    el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
                    cb.forget();
                }
            }
        }
    }
}

fn wire_glow_and_settings(app: &App) {
    wire_swatches("#glowPick .swatch[data-c]", app, settings::set_glow);
    wire_swatches("#bgPick .theme-swatch[data-c]", app, settings::set_bg_color);
    wire_swatches("#orbPick .theme-swatch[data-c]", app, settings::set_orb_color);
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("glowColor", "input", move |_| {
            let v = dom::input("glowColor").value();
            settings::set_glow(&a, &v);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("bgColorInput", "input", move |_| {
            let v = dom::input("bgColorInput").value();
            settings::set_bg_color(&a, &v);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("orbColorInput", "input", move |_| {
            let v = dom::input("orbColorInput").value();
            settings::set_orb_color(&a, &v);
        });
    }

    {
        let a = app.clone();
        dom::on_click("setBtn", move || {
            settings::apply_settings(&a);
            dom::add_class("setScrim", "show");
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("kidToggle", "change", move |_| {
            let v = dom::input("kidToggle").checked();
            // Under-cutoff (age-gate) device: Kid Mode can't be turned off
            // without the parent gate. Re-check the box and challenge instead.
            if !v && a.borrow().age_locked {
                dom::input("kidToggle").set_checked(true);
                open_parent_gate();
                return;
            }
            a.borrow_mut().kid = v;
            settings::save_prefs(&a.borrow());
            settings::apply_settings(&a);
            // Kid Mode suppresses the daily reminder — reschedule/cancel.
            settings::apply_reminder(&a.borrow());
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("readToggle", "change", move |_| {
            let v = dom::input("readToggle").checked();
            a.borrow_mut().readable = v;
            settings::save_prefs(&a.borrow());
            settings::apply_settings(&a);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("bigTextToggle", "change", move |_| {
            let v = dom::input("bigTextToggle").checked();
            a.borrow_mut().big_text = v;
            settings::save_prefs(&a.borrow());
            settings::apply_settings(&a);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("slowToggle", "change", move |_| {
            let v = dom::input("slowToggle").checked();
            a.borrow_mut().slow = v;
            settings::save_prefs(&a.borrow());
            settings::apply_settings(&a);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("remindToggle", "change", move |_| {
            a.borrow_mut().remind = dom::input("remindToggle").checked();
            settings::save_prefs(&a.borrow());
            settings::apply_reminder(&a.borrow());
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("remindTime", "change", move |_| {
            let t = dom::input("remindTime").value();
            if !t.is_empty() {
                a.borrow_mut().remind_time = t;
            }
            settings::save_prefs(&a.borrow());
            settings::apply_reminder(&a.borrow());
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("volumeSlider", "input", move |_| {
            let v: f32 = dom::input("volumeSlider").value().parse().unwrap_or(1.0);
            settings::set_volume(&a, v);
        });
    }
    dom::on_click("setDone", || dom::remove_class("setScrim", "show"));
    dom::on::<web_sys::Event, _>("setScrim", "click", |e| {
        if dom::is_self_target(&e, "setScrim") {
            dom::remove_class("setScrim", "show");
        }
    });
}

fn set_active_tool_button(id: &str) {
    for b in ["toolPen", "toolEraser", "toolLine"] {
        dom::toggle_class(b, "on", b == id);
    }
}

fn wire_drawing(app: &App) {
    dom::on_click("drawBtn", || {
        let showing = !dom::el("drawpad").class_list().contains("show");
        dom::toggle_class("drawpad", "show", showing);
        dom::toggle_class("drawBtn", "on", showing);
        if showing {
            drawing::size_canvas();
            dom::set_text("drawStatus", "Write the word, then tap \u{201c}Read my writing\u{201d}.");
        }
    });

    dom::on::<web_sys::PointerEvent, _>("canvas", "pointerdown", |e| drawing::start_stroke(&e));
    dom::on::<web_sys::PointerEvent, _>("canvas", "pointermove", |e| drawing::move_stroke(&e));
    dom::on::<web_sys::PointerEvent, _>("canvas", "pointerup", |e| drawing::end_stroke(&e));
    dom::on::<web_sys::PointerEvent, _>("canvas", "pointercancel", |e| drawing::end_stroke(&e));
    dom::on::<web_sys::PointerEvent, _>("canvas", "pointerleave", |e| drawing::end_stroke(&e));

    dom::on_click("undoStroke", || drawing::undo_stroke());
    dom::on_click("clearCanvas", || {
        drawing::clear_canvas();
        dom::set_text("drawStatus", "");
    });

    dom::on_click("toolPen", || {
        drawing::set_tool(drawing::Tool::Pen);
        set_active_tool_button("toolPen");
    });
    dom::on_click("toolEraser", || {
        drawing::set_tool(drawing::Tool::Eraser);
        set_active_tool_button("toolEraser");
    });
    dom::on_click("toolLine", || {
        drawing::set_tool(drawing::Tool::Line);
        set_active_tool_button("toolLine");
    });
    dom::on::<web_sys::Event, _>("brushSize", "input", |_| {
        let v: f64 = dom::input("brushSize").value().parse().unwrap_or(4.0);
        drawing::set_brush(v);
    });
    dom::on_click("guideToggle", || {
        let now_on = !dom::el("guideToggle").class_list().contains("on");
        drawing::set_guide(now_on);
        dom::toggle_class("guideToggle", "on", now_on);
    });
    if let Ok(list) = dom::doc().query_selector_all(".d-swatch[data-c]") {
        for i in 0..list.length() {
            if let Some(node) = list.get(i) {
                if let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() {
                    let color = el.get_attribute("data-c").unwrap_or_default();
                    let cb = Closure::<dyn FnMut()>::new(move || drawing::set_color(&color));
                    el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
                    cb.forget();
                }
            }
        }
    }
    dom::on::<web_sys::Event, _>("drawColor", "input", |_| {
        let v = dom::input("drawColor").value();
        drawing::set_color(&v);
    });

    dom::on_window::<web_sys::Event, _>("resize", |_| {
        if dom::el("drawpad").class_list().contains("show") {
            // size_canvas() already calls redraw_all(), which replays the
            // existing strokes at the new scale — no need to (and
            // shouldn't) clear them first, or an orientation change would
            // wipe out whatever the player had drawn.
            drawing::size_canvas();
        }
    });

    {
        let a = app.clone();
        dom::on_click("readWriting", move || {
            let a2 = a.clone();
            spawn_local(async move {
                dom::set_disabled("readWriting", true);
                let (answered, active) = {
                    let s = a2.borrow();
                    (s.answered, game::has_active_word(&s))
                };
                if !active || answered {
                    dom::set_text("drawStatus", "Press the orb for a word, then write it.");
                    dom::set_disabled("readWriting", false);
                    return;
                }
                if !drawing::has_strokes() {
                    dom::set_text("drawStatus", "Write a word on the pad first.");
                    dom::set_disabled("readWriting", false);
                    return;
                }
                dom::set_text("drawStatus", "Reading your writing\u{2026} (first use loads the reader)");
                match drawing::read_writing().await {
                    drawing::OcrOutcome::Confident(txt) => {
                        game::set_answer(&a2, &txt);
                        dom::set_text("drawStatus", &format!("Read \u{201c}{}\u{201d} \u{2014} confirm or fix, then Check.", txt));
                    }
                    drawing::OcrOutcome::Unsure(txt) => {
                        game::set_answer(&a2, &txt);
                        dom::set_text("drawStatus", &format!("Not sure I read that right \u{2014} got \u{201c}{}\u{201d}. Fix it if needed, then Check.", txt));
                    }
                    drawing::OcrOutcome::Empty => {
                        dom::set_text("drawStatus", "Couldn't read that \u{2014} try clearer block letters, or type it.");
                    }
                    drawing::OcrOutcome::Failed => {
                        dom::set_text("drawStatus", "The handwriting reader couldn't load here \u{2014} your writing is saved; type the letters to check.");
                    }
                }
                dom::set_disabled("readWriting", false);
            });
        });
    }
}

fn wire_modes(app: &App) {
    {
        let a = app.clone();
        dom::on_click("missesBtn", move || {
            if a.borrow().review {
                game::exit_review(&a, Some("Back to normal play."));
            } else {
                game::enter_review(&a);
            }
        });
    }
    {
        let a = app.clone();
        dom::on_click("dailyBtn", move || {
            if a.borrow().daily.active {
                game::exit_daily(&a);
            } else if daily::is_done_today() {
                game::show_today_result(&a);
            } else {
                game::enter_daily(&a);
            }
        });
    }
    dom::on_click("dailyResClose", || dom::remove_class("dailyResScrim", "show"));
    {
        let a = app.clone();
        dom::on_click("dailyShare", move || {
            let r = daily::load();
            let today = daily::today();
            let correct = r.history.get(&today).copied().unwrap_or(0);
            let (lang, kid) = {
                let s = a.borrow();
                (s.lang.clone(), s.kid)
            };
            let (_, words) = daily::build_words(&lang, &today, kid);
            share::share_daily(correct, words.len() as u32, r.streak);
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("modeSel", "change", move |_| {
            let on = dom::select("modeSel").value() == "on";
            a.borrow_mut().timed = on;
            dom::toggle_class("orbWrap", "timed", on);
            let (active, answered, cur_tier) = {
                let s = a.borrow();
                (game::has_active_word(&s), s.answered, s.cur_tier.clone())
            };
            if on && active && !answered {
                game::start_timer(&a, &cur_tier);
            } else {
                game::stop_timer(true);
            }
        });
    }
}

fn wire_source_level(app: &App) {
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("langSel", "change", move |_| {
            let v = dom::select("langSel").value();
            {
                let mut s = a.borrow_mut();
                s.lang = v.clone();
                s.cur_lang = v.clone();
            }
            // UI language follows the word-list selector (one setting). Keep the
            // last built-in locale when "My Words" is picked (it isn't a locale).
            if i18n::is_supported(&v) {
                i18n::set_and_persist(&v);
            }
            game::update_voice_note(&a);
            settings::save_prefs(&a.borrow());
            game::stop_timer(true);
            game::clear_meaning();
            {
                let mut s = a.borrow_mut();
                s.word = String::new();
                s.answered = false;
            }
            dom::set_html("orbGlyph", &i18n::t("orb.tap"));
            a.borrow_mut().answer.clear();
            game::render_letters(&a, false);
            drawing::clear_canvas();
            dom::set_text("feedback", "");
            dom::set_text("hintLine", "");
            game::build_level_options(&a);
            game::refresh_mode_buttons(&a);
            game::refresh_daily_btn(&a);
            keyboard::rebuild(&a);
            i18n::translate_page();
            climb::reflect_auth();
            stats::render(&a.borrow());
            board::render(&a.borrow());
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("levelSel", "change", move |_| {
            a.borrow_mut().level = dom::select("levelSel").value();
            // Pull a fresh word at the newly-selected difficulty right away,
            // instead of leaving the old (wrong-tier) word on screen until
            // the current round ends on its own.
            if a.borrow().review {
                return;
            }
            game::stop_timer(true);
            game::next_word(&a);
        });
    }
}

fn build_import_lang_options(app: &App) {
    let s = app.borrow();
    let opts: String = words::LANGUAGES
        .iter()
        .map(|(_, l)| format!("<option value=\"{}\">{}</option>", l.code, dom::escape_html(l.name)))
        .collect();
    dom::set_html("importLang", &opts);
    let value = if !s.custom.speak_lang.is_empty() { s.custom.speak_lang.clone() } else { "en-US".to_string() };
    dom::select("importLang").set_value(&value);
}

fn update_import_count() {
    let n = importer::extract_words(&dom::textarea("importText").value()).len();
    dom::set_text("importCount", &format!("{} word{}", n, if n == 1 { "" } else { "s" }));
}

fn wire_import(app: &App) {
    {
        let a = app.clone();
        dom::on_click("importBtn", move || {
            build_import_lang_options(&a);
            let joined = a.borrow().custom.words.join(" ");
            dom::textarea("importText").set_value(&joined);
            update_import_count();
            dom::set_text(
                "importNote",
                "Words stay on this device. Fetching a link can be blocked by the site \u{2014} if it fails, copy the text and paste it above.",
            );
            dom::add_class("importScrim", "show");
            dom::textarea("importText").focus().ok();
        });
    }
    dom::on::<web_sys::Event, _>("importText", "input", |_| update_import_count());

    dom::on_click("fetchUrl", || {
        let url = dom::input("importUrl").value().trim().to_string();
        if url.is_empty() {
            return;
        }
        dom::set_text("importNote", "Fetching\u{2026}");
        spawn_local(async move {
            match importer::fetch_words_from_url(&url).await {
                Ok(words) => {
                    let ta = dom::textarea("importText");
                    let existing = ta.value();
                    let joined = words.join(" ");
                    ta.set_value(&if existing.is_empty() { joined.clone() } else { format!("{}\n{}", existing, joined) });
                    update_import_count();
                    if words.is_empty() {
                        dom::set_text("importNote", "No words found at that link.");
                    } else {
                        dom::set_text("importNote", &format!("Added {} words from the link.", words.len()));
                    }
                }
                Err(_) => {
                    dom::set_text("importNote", "Couldn't fetch that link (the site may block it). Copy the text and paste it above instead.");
                }
            }
        });
    });

    {
        let a = app.clone();
        dom::on_click("saveWords", move || {
            let words = importer::extract_words(&dom::textarea("importText").value());
            if words.is_empty() {
                dom::set_text("importNote", "Add at least one word to save.");
                return;
            }
            // Screen out profanity before saving (also re-checked on load). This
            // guards Kid Mode — a custom list can't smuggle in slurs.
            let (words, blocked) = profanity::filter_allowed(words);
            if words.is_empty() {
                dom::set_text("importNote", profanity::rejection_message());
                return;
            }
            let speak_lang = dom::select("importLang").value();
            let count = words.len();
            importer::save_words(&mut a.borrow_mut(), words, speak_lang);
            achievements::unlock(&mut a.borrow_mut(), "importer");
            let was_review = a.borrow().review;
            if was_review {
                game::exit_review(&a, None);
            }
            {
                let mut s = a.borrow_mut();
                s.lang = MINE.to_string();
                s.cur_lang = MINE.to_string();
            }
            game::update_voice_note(&a);
            settings::save_prefs(&a.borrow());
            game::build_source_options(&a);
            game::build_level_options(&a);
            stats::render(&a.borrow());
            board::render(&a.borrow());
            game::refresh_mode_buttons(&a);
            {
                let mut s = a.borrow_mut();
                s.word = String::new();
                s.answered = false;
            }
            dom::set_html("orbGlyph", "tap to<br/>hear a word");
            a.borrow_mut().answer.clear();
            game::render_letters(&a, false);
            game::clear_meaning();
            dom::remove_class("importScrim", "show");
            let saved_msg = if blocked > 0 {
                format!(
                    "Saved {} of your words ({} skipped) \u{2014} press the orb to start.",
                    count, blocked
                )
            } else {
                format!("Saved {} of your words \u{2014} press the orb to start.", count)
            };
            dom::set_text("feedback", &saved_msg);
            dom::el("feedback").set_class_name("feedback good");
        });
    }

    {
        let a = app.clone();
        dom::on_click("clearWords", move || {
            importer::clear_words(&mut a.borrow_mut());
            dom::textarea("importText").set_value("");
            update_import_count();
            if a.borrow().lang == MINE {
                {
                    let mut s = a.borrow_mut();
                    s.lang = EN.to_string();
                    s.cur_lang = EN.to_string();
                }
                game::update_voice_note(&a);
                settings::save_prefs(&a.borrow());
            }
            game::build_source_options(&a);
            game::build_level_options(&a);
            stats::render(&a.borrow());
            dom::set_text("importNote", "Your words were cleared.");
        });
    }
    dom::on_click("cancelImport", || dom::remove_class("importScrim", "show"));
    dom::on::<web_sys::Event, _>("importScrim", "click", |e| {
        if dom::is_self_target(&e, "importScrim") {
            dom::remove_class("importScrim", "show");
        }
    });
}

fn wire_versus(app: &App) {
    {
        let a = app.clone();
        dom::on_click("vsBtn", move || {
            // If a match is already running, this button just re-opens setup —
            // exit the current one first so names/turns start clean.
            if a.borrow().versus.enabled {
                game::exit_versus(&a);
            }
            let (n1, n2) = {
                let s = a.borrow();
                (s.versus.p1.name.clone(), s.versus.p2.name.clone())
            };
            dom::input("vsName1").set_value(&n1);
            dom::input("vsName2").set_value(&n2);
            dom::add_class("vsSetupScrim", "show");
            dom::input("vsName1").focus().ok();
        });
    }
    {
        let a = app.clone();
        dom::on_click("vsStart", move || {
            let n1 = dom::input("vsName1").value();
            let n2 = dom::input("vsName2").value();
            dom::remove_class("vsSetupScrim", "show");
            game::start_versus(&a, n1, n2);
        });
    }
    dom::on_click("vsCancel", || dom::remove_class("vsSetupScrim", "show"));
    dom::on::<web_sys::Event, _>("vsSetupScrim", "click", |e| {
        if dom::is_self_target(&e, "vsSetupScrim") {
            dom::remove_class("vsSetupScrim", "show");
        }
    });
    // Mid-match "End" confirms before forfeiting (leaving affects the other
    // player); the setup/lobby "Cancel" and the result screen exit instantly.
    dom::on_click("vsExit", || dom::add_class("vsQuitScrim", "show"));
    {
        let a = app.clone();
        dom::on_click("vsQuitConfirm", move || {
            dom::remove_class("vsQuitScrim", "show");
            game::exit_versus(&a);
        });
    }
    dom::on_click("vsQuitCancel", || dom::remove_class("vsQuitScrim", "show"));
    dom::on::<web_sys::Event, _>("vsQuitScrim", "click", |e| {
        if dom::is_self_target(&e, "vsQuitScrim") {
            dom::remove_class("vsQuitScrim", "show");
        }
    });
    {
        let a = app.clone();
        dom::on_click("vsRematch", move || game::versus_rematch(&a));
    }
    {
        let a = app.clone();
        dom::on_click("vsResultClose", move || game::exit_versus(&a));
    }
    // Escape mirrors the on-screen exit controls: dismiss an open dialog, exit
    // from the result screen, or (mid-match) open the forfeit confirm. (Hardware
    // "back" would wire the same flow — TODO once the app adds back-handling.)
    {
        let a = app.clone();
        dom::on_window::<web_sys::KeyboardEvent, _>("keydown", move |e| {
            if e.key() != "Escape" {
                return;
            }
            if dom::el("vsQuitScrim").class_list().contains("show") {
                dom::remove_class("vsQuitScrim", "show");
            } else if dom::el("vsSetupScrim").class_list().contains("show") {
                dom::remove_class("vsSetupScrim", "show");
            } else if dom::el("vsResultScrim").class_list().contains("show") {
                game::exit_versus(&a);
            } else if a.borrow().versus.enabled {
                dom::add_class("vsQuitScrim", "show");
            }
        });
    }
}

fn wire_stats_board_modal(app: &App) {
    {
        let a = app.clone();
        dom::on_click("resetStats", move || {
            stats::reset_current_lang(&mut a.borrow_mut());
            // Also clear adaptive word stats (the only reset affordance today).
            wordstats::clear();
        });
    }
    {
        let a = app.clone();
        dom::on_click("saveScore", move || game::commit_save(&a));
    }
    {
        let a = app.clone();
        dom::on_click("skipSave", move || game::close_save(&a));
    }
    {
        let a = app.clone();
        dom::on::<web_sys::KeyboardEvent, _>("nameInput", "keydown", move |e| {
            if e.key() == "Enter" {
                game::commit_save(&a);
            }
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("scrim", "click", move |e| {
            if dom::is_self_target(&e, "scrim") {
                game::close_save(&a);
            }
        });
    }
}
