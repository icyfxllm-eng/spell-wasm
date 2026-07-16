mod achievements;
mod agegate;
mod api;
mod attempts;
mod audio_boost;
mod board;
mod climb;
mod consts;
mod daily;
mod deck;
mod dom;
pub mod editor;
mod enrich;
pub mod entitlements;
mod flags;
mod game;
mod ghost;
mod haptics;
mod hangul;
mod homophones;
mod i18n;
mod importer;
mod jamo;
mod keyboard;
mod kid_filter;
mod misses;
mod model;
mod native_audio;
mod native_lang;
mod photo_list;
mod norm;
mod online_spelloff;
mod pinyin;
mod notifications;
mod notify;
mod profanity;
mod say_it;
mod selection;
mod spell_aloud;
mod settings;
mod share;
mod speech_out;
mod stats;
mod storage;
mod syllable;
mod tools_hub;
#[cfg(feature = "testseam")]
mod testseam;
mod versus;
mod viet;
mod word_data;
mod word_stories;
mod words;
mod wordstats;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
    // Restore the persisted free-play decks so no-repeat continues across
    // restarts and days (Feature 2 / I4), rather than reshuffling every launch.
    state.decks = storage::get_json(model::DECKS_KEY).unwrap_or_default();

    // Study language: a saved choice wins; otherwise the device's language (if
    // supported), else English. NOTE: study-play gating to active languages is
    // NOT applied here yet — state.lang also drives the UI-language fallback
    // (home-regroup unification), and gating it would force English UI on a
    // coming-soon-language device, which this feature must NOT do (uiLang stays
    // untouched). Deferred to the study/UI separation decision.
    state.lang = match state.last_lang.clone() {
        Some(l) if l == MINE && !state.custom.words.is_empty() => MINE.to_string(),
        Some(l) if consts::is_builtin_lang(&l) => l,
        _ => i18n::device_lang().unwrap_or_else(|| EN.to_string()),
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

    // Language availability (registry): retry any queued Notify Me taps, and if
    // the current study language isn't active yet, show the coming-soon panel —
    // play is gated while the interface stays in that language (uiLang untouched).
    notify::flush();
    {
        let lang = app.borrow().lang.clone();
        if lang != MINE && !consts::is_active_lang(&lang) {
            game::render_coming_soon(&lang);
        }
    }

    // Observation-only E2E test seam — dev builds only (`--features testseam`);
    // stripped from production (proven by scripts/seam-absence-check.mjs).
    #[cfg(feature = "testseam")]
    testseam::install(&app);

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
    wire_modes(app);
    wire_source_level(app);
    wire_import(app);
    photo_list::wire(app);
    wire_stats_board_modal(app);
    wire_versus(app);
    wire_age_gate(app);
    keyboard::setup(app);
    climb::setup(app);
    // Feature F2 "Say It" — wires nothing unless the flag is on (ships dark).
    say_it::wire(app);
    say_it::reflect_gating(app);
    // Online Spell Off (async 1v1) — no entry unless flag on + signed in + not kid.
    online_spelloff::setup(app);
    // "Spell It Out Loud" — voice spelling INPUT. Wires nothing unless the flag
    // is on (ships dark, Invariant I6); the mic is hidden until config voiceSpell
    // + on-device availability both hold (Invariant I3).
    spell_aloud::wire(app);
    spell_aloud::reflect(app);
    // Pillar 3 — the "Tools & Features" hub in Settings: wire every tool switch,
    // then reflect current state so the panel is correct before it first opens.
    tools_hub::wire(app);
    tools_hub::reflect(app);

    // Notify Me (coming-soon languages): record anonymous interest for the
    // language on the panel, then flip the button to its confirmed state.
    dom::on_click("notifyBtn", || {
        if let Some(lang) = dom::el("notifyBtn").get_attribute("data-lang") {
            notify::record(&lang);
            game::render_coming_soon(&lang);
        }
    });

    // Clear the spell-box feedback color state (F1) when its animation ends —
    // animationend, never a timeout, so it can't race a rapid next answer.
    dom::on::<web_sys::Event, _>("spellbox", "animationend", |_| {
        dom::remove_class("spellbox", "is-correct");
        dom::remove_class("spellbox", "is-wrong");
    });
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
                dom::set_text("ageErr", &i18n::t("age.badDate"));
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
            dom::set_text("parentErr", &i18n::t("parent.tryAgain"));
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
    // IME composition guard on the shared input path (not a per-language branch):
    // while a composition is open we never validate or auto-advance. Window-level
    // so it covers every input surface without touching language components.
    {
        let a = app.clone();
        dom::on_window::<web_sys::Event, _>("compositionstart", move |_| {
            a.borrow_mut().composing = true;
        });
    }
    {
        let a = app.clone();
        dom::on_window::<web_sys::Event, _>("compositionend", move |_| {
            a.borrow_mut().composing = false;
        });
    }
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
        // CC-ATTEMPTS-SHIELDS Feature 1 toggle (only reachable when the row is
        // shown, i.e. the dark flag is on).
        let a = app.clone();
        dom::on::<web_sys::Event, _>("extraAttemptsToggle", "change", move |_| {
            a.borrow_mut().extra_attempts = dom::input("extraAttemptsToggle").checked();
            settings::save_prefs(&a.borrow());
        });
    }
    {
        // CC-ATTEMPTS-SHIELDS Feature 2 "Use a shield?" prompt (player choice).
        let a = app.clone();
        dom::on_click("shieldAccept", move || game::shield_accept(&a));
    }
    {
        let a = app.clone();
        dom::on_click("shieldDecline", move || game::shield_decline(&a));
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

    // Setup sheet (home-regroup F3): the chip opens the relocated pickers; Done
    // and a backdrop tap close it. The pickers themselves are unchanged.
    dom::on_click("setupChip", || dom::add_class("setupScrim", "show"));
    dom::on_click("setupDone", || dom::remove_class("setupScrim", "show"));
    dom::on::<web_sys::Event, _>("setupScrim", "click", |e| {
        if dom::is_self_target(&e, "setupScrim") {
            dom::remove_class("setupScrim", "show");
        }
    });
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
            game::update_setup_chip(&a);
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
            dom::set_text("feedback", "");
            dom::set_text("hintLine", "");
            game::build_level_options(&a);
            game::refresh_mode_buttons(&a);
            game::refresh_daily_btn(&a);
            keyboard::rebuild(&a);
            i18n::translate_page();
            climb::reflect_auth();
            game::update_setup_chip(&a);
            stats::render(&a.borrow());
            board::render(&a.borrow());
            // One picker, gate play: the interface switched to `v` above (uiLang
            // untouched); if `v` isn't an active study language, show the
            // coming-soon panel instead of a round, else restore the play area.
            if v == MINE || consts::is_active_lang(&v) {
                game::clear_coming_soon();
            } else {
                game::render_coming_soon(&v);
            }
        });
    }
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("levelSel", "change", move |_| {
            a.borrow_mut().level = dom::select("levelSel").value();
            game::update_setup_chip(&a);
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

/// Persist a screened batch of "My Words" and refresh every surface that
/// reflects the active list. Shared by the typed importer and the photo
/// importer (`photo_list::confirm`) — the caller has already run the words
/// through the charset + profanity gate and owns the "saved" message.
pub(crate) fn apply_saved_words(app: &App, words: Vec<String>, speak_lang: String) {
    importer::save_words(&mut app.borrow_mut(), words, speak_lang);
    achievements::unlock(&mut app.borrow_mut(), "importer");
    let was_review = app.borrow().review;
    if was_review {
        game::exit_review(app, None);
    }
    {
        let mut s = app.borrow_mut();
        s.lang = MINE.to_string();
        s.cur_lang = MINE.to_string();
    }
    game::update_voice_note(app);
    settings::save_prefs(&app.borrow());
    game::build_source_options(app);
    game::build_level_options(app);
    keyboard::rebuild(app);
    stats::render(&app.borrow());
    board::render(&app.borrow());
    game::refresh_mode_buttons(app);
    {
        let mut s = app.borrow_mut();
        s.word = String::new();
        s.answered = false;
    }
    dom::set_html("orbGlyph", &i18n::t("orb.tap"));
    app.borrow_mut().answer.clear();
    game::render_letters(app, false);
    game::clear_meaning();
}

fn wire_import(app: &App) {
    {
        let a = app.clone();
        dom::on_click("importBtn", move || {
            build_import_lang_options(&a);
            // Do NOT repopulate with saved words (that was the stale-input bug),
            // and do NOT wipe an in-progress draft either — the textarea keeps
            // whatever the player last typed. Only a successful Save clears it
            // (saved words persist; save is additive).
            update_import_count();
            dom::set_text("importNote", &i18n::t("import.note"));
            dom::add_class("importScrim", "show");
            dom::textarea("importText").focus().ok();
        });
    }
    dom::on::<web_sys::Event, _>("importText", "input", |_| update_import_count());

    {
        let a = app.clone();
        dom::on_click("saveWords", move || {
            let words = importer::extract_words(&dom::textarea("importText").value());
            if words.is_empty() {
                dom::set_text("importNote", &i18n::t("import.needWord"));
                return;
            }
            let speak_lang = dom::select("importLang").value();
            let kid = a.borrow().kid;
            let policy = native_lang::wordcheck_policy(kid);
            // On web (no native bridge) or with the gate off, save exactly as
            // before — behavior is byte-identical off iOS.
            if !native_lang::available() || policy == native_lang::WordCheckPolicy::Off {
                commit_import(&a, words, speak_lang, None);
                return;
            }
            // Native: additional on-device gate. Gate order per word is
            // charset (extract_words) -> NFC (in the checker) -> UITextChecker ->
            // profanity (inside commit_import, ALWAYS runs). A word iOS has no
            // dictionary for (supported:false) is never judged here.
            let a2 = a.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let mut kept = Vec::with_capacity(words.len());
                let mut dropped = 0usize;
                for w in words {
                    let v = native_lang::check_word_await(&w, &speak_lang).await;
                    if v.supported && !v.is_word && policy == native_lang::WordCheckPolicy::Block {
                        dropped += 1; // Kid-Mode default: block non-words
                        continue;
                    }
                    kept.push(w);
                }
                if kept.is_empty() {
                    dom::set_text("importNote", &i18n::t("import.dictAllSkipped"));
                    return;
                }
                // Feature 5: non-blocking language hint (never blocks the save).
                let hint = lang_hint(&kept, &speak_lang).await;
                let mut notes: Vec<String> = Vec::new();
                if dropped > 0 {
                    notes.push(i18n::tp("import.dictSkipped", &[("d", &dropped.to_string())]));
                }
                if let Some(h) = hint {
                    notes.push(h);
                }
                let extra = if notes.is_empty() { None } else { Some(notes.join(" ")) };
                commit_import(&a2, kept, speak_lang, extra);
            });
        });
    }

    // Nested items (capture nothing; shared by the handlers above/below).

    /// Profanity-screen (ALWAYS), persist, and refresh the UI for an import.
    /// Shared by the plain (web) path and the native dictionary-gated path;
    /// `extra_note` is appended to the success message (skip count / lang hint).
    fn commit_import(a: &App, words: Vec<String>, speak_lang: String, extra_note: Option<String>) {
    // Screen out profanity before saving (also re-checked on load). This guards
    // Kid Mode — a custom list can't smuggle in slurs — and runs regardless of
    // the dictionary verdict (a real word can still be a blocked word).
    let (words, blocked) = profanity::filter_allowed(words);
    if words.is_empty() {
        dom::set_text("importNote", profanity::rejection_message());
        return;
    }
    let count = words.len();
    importer::save_words(&mut a.borrow_mut(), words, speak_lang);
    achievements::unlock(&mut a.borrow_mut(), "importer");
    let was_review = a.borrow().review;
    if was_review {
        game::exit_review(a, None);
    }
    {
        let mut s = a.borrow_mut();
        s.lang = MINE.to_string();
        s.cur_lang = MINE.to_string();
    }
    game::update_voice_note(a);
    settings::save_prefs(&a.borrow());
    game::build_source_options(a);
    game::build_level_options(a);
    keyboard::rebuild(a); // match the imported list's "Speak in" language
    stats::render(&a.borrow());
    board::render(&a.borrow());
    game::refresh_mode_buttons(a);
    {
        let mut s = a.borrow_mut();
        s.word = String::new();
        s.answered = false;
    }
    dom::set_html("orbGlyph", &i18n::t("orb.tap"));
    a.borrow_mut().answer.clear();
    game::render_letters(a, false);
    game::clear_meaning();
    // Clean slate: clear the input now it's saved (words persist — save is
    // additive), so reopening My Words is empty.
    dom::textarea("importText").set_value("");
    update_import_count();
    dom::remove_class("importScrim", "show");
    let mut saved_msg = if blocked > 0 {
        i18n::tp("import.savedSkipped", &[("n", &count.to_string()), ("b", &blocked.to_string())])
    } else {
        i18n::tp("import.saved", &[("n", &count.to_string())])
    };
    if let Some(note) = extra_note {
        saved_msg.push(' ');
        saved_msg.push_str(&note);
    }
    dom::set_text("feedback", &saved_msg);
    dom::el("feedback").set_class_name("feedback good");
}

/// Feature 5: if the saved set reads as a different app language with high
/// confidence, return a gentle, non-blocking hint. `None` otherwise.
async fn lang_hint(words: &[String], speak_lang: &str) -> Option<String> {
    let sample = words.join(" ");
    let (supported, lang, confidence) = native_lang::detect_language_await(&sample).await;
    // High bar — single words give NLLanguageRecognizer a weak signal.
    if !supported || confidence < 0.7 || lang == speak_lang {
        return None;
    }
    // Only hint toward a language the app actually has.
    let name = consts::BUILTIN_LANGS.iter().find(|(c, _, _)| *c == lang).map(|(_, n, _)| *n)?;
    Some(i18n::tp("import.langHint", &[("lang", name)]))
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
            dom::set_text("importNote", &i18n::t("import.cleared"));
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
