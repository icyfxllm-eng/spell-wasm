use std::cell::RefCell;

use unicode_normalization::UnicodeNormalization;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::consts::{tier_time, CORRECT_DELAY_MS, EN, LEVEL_OPTS, MAX_TRIES, MINE, PRAISE, REVIEW};
use crate::model::AppState;
use crate::versus::Side;
use crate::{achievements, api, board, dom, drawing, misses, speech_out, stats, wordstats, words};
use crate::App;

const RING_C: f64 = 2.0 * std::f64::consts::PI * 108.0;

// ---------- naming / access helpers ----------

pub fn code_for(state: &AppState, key: &str) -> String {
    if key == MINE {
        return if state.custom.speak_lang.is_empty() { "en-US".to_string() } else { state.custom.speak_lang.clone() };
    }
    "en-US".to_string()
}

/// My Words can route through the backend's real TTS (reliable, unlike the
/// browser's built-in voices which may not exist at all on this machine)
/// as long as the word is plain English letters — the backend rejects
/// anything else (spaces, apostrophes, hyphens, non-English speak_lang).
fn backend_speakable(state: &AppState, word: &str) -> bool {
    let code = code_for(state, MINE);
    let is_english = code.split('-').next().unwrap_or(&code).eq_ignore_ascii_case("en");
    is_english && !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic())
}

pub fn name_for(_state: &AppState, key: &str) -> String {
    if key == MINE {
        return "My Words".to_string();
    }
    if key == REVIEW {
        return "Misses".to_string();
    }
    "English".to_string()
}

fn length_tier(word: &str) -> &'static str {
    let n = word.chars().filter(|c| !c.is_whitespace() && *c != '\'' && *c != '\u{2019}' && *c != '-').count();
    if n <= 4 {
        "easy"
    } else if n <= 7 {
        "medium"
    } else if n <= 11 {
        "hard"
    } else {
        "expert"
    }
}

fn pool_for_tier(state: &AppState, tier: &str) -> Vec<String> {
    let all = &state.custom.words;
    if all.is_empty() {
        return vec!["word".to_string()];
    }
    let filtered: Vec<String> = all.iter().filter(|w| length_tier(w) == tier).cloned().collect();
    if filtered.is_empty() {
        all.clone()
    } else {
        filtered
    }
}

fn active_word_list(state: &AppState, tier: &str) -> Vec<String> {
    if state.lang == MINE {
        pool_for_tier(state, tier)
    } else {
        words::en_tier(tier).iter().map(|s| s.to_string()).collect()
    }
}

fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (js_sys::Math::random() * len as f64).floor() as usize % len
}

/// True while there's a word on screen the player can act on (answer,
/// replay, draw...).
pub fn has_active_word(state: &AppState) -> bool {
    !state.word.is_empty()
}

/// How many of the current English tier's clips to download to on-device
/// storage at app open, so the first turns are instant and playable offline.
/// Kept small ("small, fast" per the plan); every word actually played is
/// cached too, so the offline pack fills in naturally with use.
const PRELOAD_AT_OPEN: usize = 20;

/// On the native build, warm the on-device audio cache with the English tier
/// the first non-review word will come from. No-op on the web (guarded so we
/// don't fire a burst of HTTP-cache warm-ups in the browser).
pub fn preload_pool(app: &App) {
    if !crate::native_audio::available() {
        return;
    }
    let s = app.borrow();
    if s.lang != EN {
        return;
    }
    let mut tier = if s.level == "climb" {
        tier_for_streak(s.streak).to_string()
    } else {
        s.level.clone()
    };
    if s.kid && (tier == "hard" || tier == "expert") {
        tier = "medium".to_string();
    }
    let pool = active_word_list(&s, &tier);
    drop(s);
    for word in pool.into_iter().take(PRELOAD_AT_OPEN) {
        api::preload_word(&word);
    }
}

fn tier_for_streak(streak: u32) -> &'static str {
    if streak < 3 {
        "easy"
    } else if streak < 7 {
        "medium"
    } else if streak < 12 {
        "hard"
    } else {
        "expert"
    }
}

// ---------- rendering ----------

pub fn render_letters(app: &App, animate_all: bool) {
    let value = app.borrow().answer.clone();
    if value.is_empty() {
        dom::set_html("letters", "<span class=\"placeholder\">type what you heard</span><span class=\"caret\"></span>");
        app.borrow_mut().prev_letter_len = 0;
        return;
    }
    let start = if animate_all { 0 } else { app.borrow().prev_letter_len };
    let mut html = String::new();
    for (i, ch) in value.chars().enumerate() {
        if ch == ' ' {
            html.push_str("<span class=\"ltr space\">&nbsp;</span>");
        } else {
            let pop = if i >= start { "pop" } else { "" };
            html.push_str(&format!("<span class=\"ltr {}\">{}</span>", pop, dom::escape_html(&ch.to_string())));
        }
    }
    html.push_str("<span class=\"caret\"></span>");
    dom::set_html("letters", &html);
    app.borrow_mut().prev_letter_len = value.chars().count();
}

// ---------- answer input (no DOM <input>: keeps the iOS keyboard closed) ----------

/// True while the player may type — there's a live, unanswered word on screen.
pub fn can_type(state: &AppState) -> bool {
    has_active_word(state) && !state.answered
}

/// Enable/disable the on-screen keyboard and answer caret to match `can_type`,
/// and reveal the apostrophe/hyphen keys only for My Words (the built-in
/// English tiers never use them).
pub fn sync_keyboard(app: &App) {
    let (enabled, mine) = {
        let s = app.borrow();
        (can_type(&s), s.lang == MINE)
    };
    dom::toggle_class("gameKeyboard", "locked", !enabled);
    dom::toggle_class("gameKeyboard", "show-punct", mine);
    dom::toggle_class("spellbox", "focus", enabled);
}

/// Replace the whole answer (mic transcript, handwriting OCR) and re-render.
pub fn set_answer(app: &App, value: &str) {
    app.borrow_mut().answer = value.to_string();
    render_letters(app, true);
}

/// Append one typed character (on-screen key or physical keydown). Single char
/// only — multi-char inserts (dictation/paste) can't reach this path.
pub fn type_char(app: &App, ch: char) {
    if !can_type(&app.borrow()) {
        return;
    }
    app.borrow_mut().answer.push(ch);
    render_letters(app, false);
    crate::haptics::key_tap();
    announce(&ch.to_string());
}

/// Delete the last character of the answer.
pub fn backspace(app: &App) {
    if !can_type(&app.borrow()) {
        return;
    }
    let popped = app.borrow_mut().answer.pop();
    render_letters(app, false);
    if popped.is_some() {
        crate::haptics::key_tap();
        announce("delete");
    }
}

/// Screen-reader announcement of the last key, via a visually-hidden live region.
fn announce(msg: &str) {
    dom::set_text("kbAnnounce", msg);
}

pub fn update_voice_note(app: &App) {
    let s = app.borrow();
    let cur_lang = s.cur_lang.clone();
    let word = s.word.clone();
    // Backend-voiced words (built-in English, or plain-English My Words)
    // don't depend on a browser voice at all.
    if cur_lang == EN || (cur_lang == MINE && backend_speakable(&s, &word)) {
        drop(s);
        dom::set_text("voiceNote", "");
        return;
    }
    let code = code_for(&s, &cur_lang);
    drop(s);
    if speech_out::voice_for_code(&code).is_some() {
        dom::set_text("voiceNote", "");
        return;
    }
    dom::set_text(
        "voiceNote",
        "No voice for your words is installed in this browser \u{2014} you won't hear audio for them here, but spelling still counts.",
    );
}

pub fn render_tries(app: &App) {
    let s = app.borrow();
    if !has_active_word(&s) {
        drop(s);
        dom::set_html("triesLine", "");
        return;
    }
    let left = s.tries_left;
    drop(s);
    let mut html = String::new();
    for i in 0..MAX_TRIES {
        let cls = if i < left { "try-dot" } else { "try-dot used" };
        html.push_str(&format!("<span class=\"{}\">\u{25cf}</span>", cls));
    }
    dom::set_html("triesLine", &html);
}

pub fn refresh_mode_buttons(app: &App) {
    let s = app.borrow();
    let total = s.misses.len();
    let due = misses::due_misses(&s).len();
    dom::set_text("missesBtn", &if s.review { "Exit misses".to_string() } else { format!("\u{21bb} Misses \u{b7} {}", due) });
    dom::el("missesBtn").set_attribute("title", &if total > 0 { format!("{} saved \u{b7} {} due now", total, due) } else { String::new() }).ok();
    dom::toggle_class("missesBtn", "on", s.review);
    dom::set_disabled("missesBtn", !s.review && total == 0);
}


pub fn build_source_options(app: &App) {
    let s = app.borrow();
    let mut opts = format!("<option value=\"{}\">English</option>", EN);
    if !s.custom.words.is_empty() {
        opts.push_str(&format!("<option value=\"{}\">My Words \u{270f}</option>", MINE));
    }
    dom::set_html("langSel", &opts);
    drop(s);
    let mut s = app.borrow_mut();
    if s.lang == MINE && s.custom.words.is_empty() {
        s.lang = EN.to_string();
    }
    let lang = s.lang.clone();
    drop(s);
    dom::select("langSel").set_value(&lang);
}

pub fn build_level_options(app: &App) {
    let s = app.borrow();
    let opts: String = LEVEL_OPTS.iter().map(|(v, l)| format!("<option value=\"{}\">{}</option>", v, l)).collect();
    dom::set_html("levelSel", &opts);
    dom::select("levelSel").set_value(&s.level);
}

// ---------- meanings (dictionary lookups) ----------

thread_local! {
    static MEANING_SEQ: RefCell<u64> = RefCell::new(0);
}

pub fn clear_meaning() {
    MEANING_SEQ.with(|c| *c.borrow_mut() += 1);
    dom::set_html("meaning", "");
    dom::remove_class("meaning", "show");
}

/// Post-answer reveal only (the round is already over, so there's nothing
/// left to protect) — English routes through our own backend's cached
/// `/api/meaning` proxy; other languages (My Words with a non-English
/// speak_lang) still go straight to dictionaryapi.dev, since our backend
/// only knows English.
async fn fetch_definition(word: String, code: String) -> Option<(String, String, String)> {
    let base = code.split('-').next().unwrap_or(&code).to_string();
    if base.eq_ignore_ascii_case("en") {
        let (pos, def, example) = api::fetch_meaning(&word, false).await.ok()?;
        return if def.is_empty() { None } else { Some((pos, def, example)) };
    }
    let api_lang = crate::consts::def_lang(&base)?;
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/{}/{}", api_lang, urlencode(&word));
    let text = crate::storage::fetch_text(&url).await.ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entry = json.as_array()?.first()?;
    let mean = entry.get("meanings")?.as_array()?.first()?;
    let pos = mean.get("partOfSpeech").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let d0 = mean.get("definitions")?.as_array()?.first()?;
    let def = d0.get("definition")?.as_str()?.to_string();
    let example = d0.get("example").and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some((pos, def, example))
}

fn urlencode(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_else(|| s.to_string())
}

pub fn show_meaning(app: &App, word: String, lang_key: String) {
    clear_meaning();
    let my_seq = MEANING_SEQ.with(|c| *c.borrow());
    let code = code_for(&app.borrow(), &lang_key);
    let app = app.clone();
    spawn_local(async move {
        let Some((pos, def, example)) = fetch_definition(word.clone(), code).await else { return };
        let current = MEANING_SEQ.with(|c| *c.borrow());
        if current != my_seq {
            return;
        }
        let mut html = format!(
            "<span class=\"m-word\">{}</span><span class=\"m-pos\">{}</span> \u{2014} {}",
            dom::escape_html(&word),
            dom::escape_html(&pos),
            dom::escape_html(&def)
        );
        if !example.is_empty() {
            html.push_str(&format!(
                "<span class=\"m-ex\">\u{201c}{}\u{201d} <button class=\"m-spk\" id=\"spkEx\">\u{1f50a} sentence</button></span>",
                dom::escape_html(&example)
            ));
        }
        dom::set_html("meaning", &html);
        dom::add_class("meaning", "show");
        if !example.is_empty() {
            if let Ok(el) = dom::el("spkEx").dyn_into::<web_sys::HtmlElement>() {
                let rate = app.borrow().rate.min(0.92);
                let cb = Closure::<dyn FnMut()>::new(move || speech_out::speak(&example, rate, "en-US"));
                el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
                cb.forget();
            }
        }
    });
}

/// Pre-answer hint: shows this word's masked definition, so a homonym pair
/// (their/there/they're) can be disambiguated without giving away the
/// spelling. Available mid-round with no score penalty — English only,
/// since our backend's masking proxy doesn't cover other languages.
pub fn show_definition_hint(app: &App) {
    let (word, cur_lang) = {
        let s = app.borrow();
        (s.word.clone(), s.cur_lang.clone())
    };
    if word.is_empty() || cur_lang != EN {
        return;
    }
    spawn_local(async move {
        let (pos, definition) = match api::fetch_meaning(&word, true).await {
            Ok((pos, definition, _)) if !definition.is_empty() => (pos, definition),
            _ => {
                dom::set_html("meaning", "<span class=\"m-pos\">No definition found for this word.</span>");
                dom::add_class("meaning", "show");
                return;
            }
        };
        dom::set_html(
            "meaning",
            &format!("<span class=\"m-pos\">{}</span> \u{2014} {}", dom::escape_html(&pos), dom::escape_html(&definition)),
        );
        dom::add_class("meaning", "show");
    });
}

/// Pre-answer hint: shows this word's masked example sentence and speaks
/// the real (unmasked) sentence aloud — hearing it doesn't give away the
/// spelling the way reading it would. Same availability as the definition
/// hint above.
pub fn show_sentence_hint(app: &App) {
    let (word, cur_lang) = {
        let s = app.borrow();
        (s.word.clone(), s.cur_lang.clone())
    };
    if word.is_empty() || cur_lang != EN {
        return;
    }
    api::play_sentence_audio(&word);
    spawn_local(async move {
        let example = match api::fetch_meaning(&word, true).await {
            Ok((_, _, example)) if !example.is_empty() => example,
            _ => {
                dom::set_html("meaning", "<span class=\"m-pos\">No example sentence found for this word.</span>");
                dom::add_class("meaning", "show");
                return;
            }
        };
        dom::set_html("meaning", &format!("<span class=\"m-ex\">\u{201c}{}\u{201d}</span>", dom::escape_html(&example)));
        dom::add_class("meaning", "show");
    });
}

// ---------- main flow ----------

pub fn speak_current(app: &App) {
    speak_word(app, "normal", app.borrow().rate);
}

/// Plays a real, separately-synthesized slow variant for backend-voiced
/// words (clearer than just slowing down the normal clip's playback rate);
/// browser-TTS words (non-English My Words) fall back to a slow playback
/// rate since the backend can't voice them.
pub fn replay_slow(app: &App) {
    speak_word(app, "slow", 1.0);
}

fn speak_word(app: &App, variant: &str, rate: f32) {
    let s = app.borrow();
    if s.word.is_empty() {
        return;
    }
    let word = s.word.clone();
    let code = code_for(&s, &s.cur_lang);

    if s.cur_lang == EN {
        drop(s);
        api::play_word(&word, variant, rate as f64, || {});
        return;
    }
    if s.cur_lang == MINE && backend_speakable(&s, &word) {
        drop(s);
        let fallback_word = word.clone();
        let fallback_rate = if variant == "slow" { 0.55 } else { rate };
        api::play_word(&word, variant, rate as f64, move || speech_out::speak(&fallback_word, fallback_rate, &code));
        return;
    }
    drop(s);
    let browser_rate = if variant == "slow" { 0.55 } else { rate };
    speech_out::speak(&word, browser_rate, &code);
}

pub fn next_word(app: &App) {
    clear_meaning();
    {
        let mut s = app.borrow_mut();
        s.tries_left = MAX_TRIES;
        if s.review {
            let due = misses::due_misses(&s);
            if due.is_empty() {
                drop(s);
                exit_review(app, Some("All caught up \u{2014} your misses are scheduled for later review."));
                return;
            }
            // Deck-select over the due misses (keyed by the same lang::word
            // identity misses.rs already uses) so review turns don't repeat
            // the same due word back-to-back either.
            let due_keys: Vec<String> = due.iter().map(|&i| misses::miss_key(&s.misses[i].word, &s.misses[i].lang)).collect();
            let pick_key = s.decks.entry("__review".to_string()).or_default().next(&due_keys);
            let pick = due
                .iter()
                .find(|&&i| misses::miss_key(&s.misses[i].word, &s.misses[i].lang) == pick_key)
                .copied()
                .unwrap_or(due[0]);
            let m = s.misses[pick].clone();
            s.cur_lang = m.lang.clone();
            s.word = m.word.clone();
            s.cur_tier = m.tier.clone();
        } else {
            s.cur_lang = s.lang.clone();
            // In head-to-head, "Climb" difficulty tracks the active player's
            // current chain rather than the (unused) single-player streak.
            let climb_from = if s.versus.enabled { s.versus.active_player().current } else { s.streak };
            let mut tier = if s.level == "climb" { tier_for_streak(climb_from).to_string() } else { s.level.clone() };
            if s.kid && (tier == "hard" || tier == "expert") {
                tier = "medium".to_string();
            }
            s.cur_tier = tier.clone();
            let pool = active_word_list(&s, &tier);
            if !pool.is_empty() {
                let key = format!("{}:{}", s.lang, tier);
                // Solo practice uses adaptive, spaced-repetition-weighted picking;
                // head-to-head keeps the plain shuffled deck so both players face
                // the same distribution. Adaptive falls back to the deck if the
                // pool somehow yields nothing.
                let w = if s.versus.enabled {
                    s.decks.entry(key.clone()).or_default().next(&pool)
                } else {
                    match wordstats::pick(&pool) {
                        Some(w) => w,
                        None => s.decks.entry(key.clone()).or_default().next(&pool),
                    }
                };
                s.word = w;
                // Warm the browser's audio cache for whatever this same
                // pool will hand out next time, so that turn's playback is
                // instant instead of waiting on a fresh TTS fetch.
                if s.cur_lang == EN {
                    if let Some(next_up) = s.decks.get(&key).and_then(|d| d.peek()) {
                        api::preload_word(&next_up);
                    }
                }
            }
        }
    }

    app.borrow_mut().answered = false;
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    dom::set_html("orbGlyph", "listen\u{2026}");
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    drawing::clear_canvas();
    dom::set_disabled("checkBtn", false);
    dom::set_disabled("hintBtn", false);
    dom::set_disabled("giveupBtn", false);
    dom::set_disabled("replayBtn", false);
    dom::set_disabled("slowBtn", false);
    // Definition/Sentence hints route through our backend's English-only
    // masking proxy — hide them for other languages rather than offering a
    // button that will just fail.
    let is_en = app.borrow().cur_lang == EN;
    dom::toggle_class("defBtn", "btn-hide", !is_en);
    dom::toggle_class("sentenceBtn", "btn-hide", !is_en);
    dom::set_disabled("defBtn", !is_en);
    dom::set_disabled("sentenceBtn", !is_en);
    dom::set_text("hintLine", "");
    render_tries(app);
    dom::el("feedback").set_class_name("feedback");
    dom::set_text("feedback", "");
    update_voice_note(app);
    sync_keyboard(app);
    speak_current(app);
    let cur_tier = app.borrow().cur_tier.clone();
    start_timer(app, &cur_tier);
}

fn norm(s: &str) -> String {
    s.nfd().filter(|c| !(*c >= '\u{0300}' && *c <= '\u{036f}')).collect::<String>().to_lowercase().chars().filter(|c| !c.is_whitespace()).collect()
}

fn lock_inputs() {
    dom::add_class("gameKeyboard", "locked");
    dom::remove_class("spellbox", "focus");
    dom::set_disabled("checkBtn", true);
    dom::set_disabled("hintBtn", true);
    dom::set_disabled("giveupBtn", true);
    dom::set_disabled("defBtn", true);
    dom::set_disabled("sentenceBtn", true);
}

pub fn submit_guess(app: &App) {
    let (answered, cur_lang, word) = {
        let s = app.borrow();
        (s.answered, s.cur_lang.clone(), s.word.clone())
    };
    if answered || word.is_empty() {
        return;
    }
    let typed = app.borrow().answer.trim().to_string();
    if typed.is_empty() {
        return;
    }
    app.borrow_mut().answered = true;
    stop_timer(true);
    lock_inputs();

    if cur_lang == EN {
        spawn_local(backend_verify(app.clone(), word, typed));
        return;
    }
    if norm(&typed) == norm(&word) {
        on_correct(app);
    } else {
        on_wrong(app);
    }
}

/// English words get double-checked against the backend's /api/check; if
/// that request fails (server down, etc.) we fall back to comparing
/// locally so a network hiccup doesn't stall the game.
async fn backend_verify(app: App, word: String, typed: String) {
    let correct = match api::check_answer(&word, &typed).await {
        Ok(c) => c,
        Err(_) => norm(&typed) == norm(&word),
    };
    if correct {
        on_correct(&app);
    } else {
        on_wrong(&app);
    }
}

fn pick_praise(app: &App) -> String {
    let s = app.borrow();
    if s.review {
        return "Got it \u{2014} scheduled to return later.".to_string();
    }
    let mut base = PRAISE[rand_index(PRAISE.len())].to_string();
    if s.streak >= 5 {
        base.push_str(&format!(" {} in a row.", s.streak));
    }
    base
}

fn bump_streak(app: &App) -> u32 {
    let mut s = app.borrow_mut();
    s.streak += 1;
    if s.streak > s.best {
        s.best = s.streak;
    }
    s.streak
}

fn on_correct(app: &App) {
    if app.borrow().versus.enabled {
        versus_on_correct(app);
        return;
    }
    crate::haptics::correct();
    let (cur_lang, cur_tier, word) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.cur_tier.clone(), s.word.clone())
    };
    stats::record(&mut app.borrow_mut(), &cur_lang, &cur_tier, true);
    // Adaptive word stats: solo practice only (Misses/review has its own SR).
    if !app.borrow().review {
        wordstats::record(&word, true);
    }
    let cleared = misses::promote_miss(&mut app.borrow_mut(), &word, &cur_lang);
    refresh_mode_buttons(app);
    if cleared {
        achievements::unlock(&mut app.borrow_mut(), "cleared");
    }

    // Stamp the run's start when a fresh chain begins (for The Climb's timing).
    if app.borrow().streak == 0 {
        app.borrow_mut().run_start_ms = now_ms();
    }
    let streak = bump_streak(app);
    dom::set_text("streakNum", &streak.to_string());
    dom::set_text("bestNum", &app.borrow().best.to_string());
    dom::add_class("orbWrap", "good");
    dom::set_text("orbGlyph", "\u{2713}");
    dom::set_text("feedback", &pick_praise(app));
    dom::el("feedback").set_class_name("feedback good");
    show_meaning(app, word, cur_lang);
    achievements::check_streak(&mut app.borrow_mut());

    schedule(app, CORRECT_DELAY_MS, |app| next_word(app));
}

/// Consumes one attempt at the current word, returning how many remain.
fn use_a_try(app: &App) -> u32 {
    let mut s = app.borrow_mut();
    s.tries_left = s.tries_left.saturating_sub(1);
    s.tries_left
}

/// Lets the player take another swing at the same word instead of ending
/// the round, since they still have tries left.
fn retry_wrong(app: &App, tries_left: u32, verb: &str) {
    app.borrow_mut().answered = false;
    dom::set_disabled("checkBtn", false);
    dom::set_disabled("hintBtn", false);
    dom::set_disabled("giveupBtn", false);
    let is_en = app.borrow().cur_lang == EN;
    dom::set_disabled("defBtn", !is_en);
    dom::set_disabled("sentenceBtn", !is_en);
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    drawing::clear_canvas();
    dom::add_class("orbWrap", "bad");
    dom::set_text("orbGlyph", "\u{2717}");
    dom::set_html(
        "feedback",
        &format!("{} \u{2014} {} {} left", verb, tries_left, if tries_left == 1 { "try" } else { "tries" }),
    );
    dom::el("feedback").set_class_name("feedback bad");
    sync_keyboard(app);
    schedule(app, 550, |_| {
        dom::remove_class("orbWrap", "bad");
        dom::set_html("orbGlyph", "listen\u{2026}");
    });
    let cur_tier = app.borrow().cur_tier.clone();
    start_timer(app, &cur_tier);
}

/// Out of tries (or timed out / gave up): record the miss and reveal the word.
fn finalize_incorrect(app: &App, glyph: &str, prefix: &str, feedback_class: &str) {
    let (versus_on, cur_lang, cur_tier, word) = {
        let s = app.borrow();
        (s.versus.enabled, s.cur_lang.clone(), s.cur_tier.clone(), s.word.clone())
    };
    // Head-to-head records nothing persistent (accuracy, Misses, leaderboard,
    // word stats) — the match never counts toward solo progress.
    if !versus_on {
        stats::record(&mut app.borrow_mut(), &cur_lang, &cur_tier, false);
        misses::add_miss(&mut app.borrow_mut(), &word, &cur_lang, &cur_tier);
        refresh_mode_buttons(app);
    }
    // Adaptive word stats: a loss (out of tries / timeout / give-up) is a miss,
    // for solo practice only (not head-to-head, not Misses/review).
    if !versus_on && !app.borrow().review {
        wordstats::record(&word, false);
    }

    dom::add_class("orbWrap", "bad");
    dom::set_text("orbGlyph", glyph);
    dom::set_html("feedback", &format!("{}<span class=\"reveal\">{}</span>", prefix, dom::escape_html(&word)));
    dom::el("feedback").set_class_name(feedback_class);
    show_meaning(app, word, cur_lang);
    if versus_on {
        versus_end_turn(app);
    } else {
        end_chain(app);
    }
}

fn on_wrong(app: &App) {
    crate::haptics::incorrect(app.borrow().kid);
    let tries_left = use_a_try(app);
    render_tries(app);
    if tries_left > 0 {
        retry_wrong(app, tries_left, "Not quite");
        return;
    }
    finalize_incorrect(app, "\u{2717}", "It was ", "feedback bad");
}

fn on_timeout(app: &App) {
    let answered = app.borrow().answered;
    if answered {
        return;
    }
    app.borrow_mut().answered = true;
    lock_inputs();

    let tries_left = use_a_try(app);
    render_tries(app);
    if tries_left > 0 {
        retry_wrong(app, tries_left, "Time's up");
        return;
    }
    finalize_incorrect(app, "\u{23f1}", "Time's up \u{2014} it was ", "feedback bad");
}

pub fn give_up(app: &App) {
    let (answered, word) = {
        let s = app.borrow();
        (s.answered, s.word.clone())
    };
    if answered || word.is_empty() {
        return;
    }
    app.borrow_mut().answered = true;
    stop_timer(true);
    lock_inputs();

    app.borrow_mut().tries_left = 0;
    render_tries(app);
    finalize_incorrect(app, "\u{2013}", "The word was ", "feedback");
}

fn end_chain(app: &App) {
    let (reached, level, run_start, kid) = {
        let s = app.borrow();
        (s.streak, s.level.clone(), s.run_start_ms, s.kid)
    };
    // Post to The Climb: ranked difficulty (submit_run filters to medium/hard/
    // expert) and logged-in only; never in Kid Mode. A plausible run duration
    // feeds the server-side anti-cheat.
    if reached > 0 && !kid {
        let duration = (now_ms() - run_start).max(0.0);
        crate::climb::submit_run(&level, reached, duration);
    }
    let delay = if reached > 0 { 950 } else { 650 };
    let app2 = app.clone();
    schedule_raw(delay, move || {
        if reached > 0 && board::qualifies(reached) {
            open_save(&app2, reached);
        } else {
            reset_chain_soft(&app2);
        }
    });
}

fn reset_chain_soft(app: &App) {
    app.borrow_mut().streak = 0;
    dom::set_text("streakNum", "0");
    let review = app.borrow().review;
    dom::set_html("orbGlyph", if review { "tap to<br/>continue" } else { "tap for<br/>next word" });
}

pub fn show_hint(app: &App) {
    let s = app.borrow();
    if s.word.is_empty() {
        return;
    }
    let w = s.word.clone();
    drop(s);
    let n = w.chars().count();
    let masked: String = w
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 || c == ' ' || c == '\'' || c == '-' { c } else { '\u{2022}' })
        .collect();
    dom::set_text("hintLine", &format!("{}   ({} letters)", masked, n));
}

// ---------- timer ----------

struct TimerState {
    interval_id: Option<i32>,
    deadline: f64,
    total: f64,
}

thread_local! {
    static TIMER: RefCell<TimerState> = RefCell::new(TimerState { interval_id: None, deadline: 0.0, total: 0.0 });
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn start_timer(app: &App, tier: &str) {
    stop_timer(true);
    let timed = app.borrow().timed;
    if !timed {
        return;
    }
    let total = tier_time(tier) as f64 * 1000.0;
    TIMER.with(|t| {
        let mut t = t.borrow_mut();
        t.total = total;
        t.deadline = now_ms() + total;
    });
    dom::remove_class("tProg", "low");
    dom::remove_class("timerBadge", "low");
    tick(app);
    let app2 = app.clone();
    let cb = Closure::<dyn FnMut()>::new(move || tick(&app2));
    let id = dom::window()
        .set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 60)
        .unwrap_or(0);
    cb.forget();
    TIMER.with(|t| t.borrow_mut().interval_id = Some(id));
}

fn tick(app: &App) {
    let (remain, total) = TIMER.with(|t| {
        let t = t.borrow();
        ((t.deadline - now_ms()).max(0.0), t.total)
    });
    let frac = if total > 0.0 { remain / total } else { 0.0 };
    let offset = RING_C * (1.0 - frac);
    if let Ok(prog) = dom::el("tProg").dyn_into::<web_sys::Element>() {
        let _ = prog.set_attribute("style", &format!("stroke-dashoffset:{}", offset));
    }
    dom::set_text("timerBadge", &format!("{}s", (remain / 1000.0).ceil() as i64));
    if remain <= 5000.0 {
        dom::add_class("tProg", "low");
        dom::add_class("timerBadge", "low");
    }
    if remain <= 0.0 {
        stop_timer(true);
        on_timeout(app);
    }
}

pub fn stop_timer(reset: bool) {
    TIMER.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(id) = t.interval_id.take() {
            dom::window().clear_interval_with_handle(id);
        }
        if reset {
            if let Ok(prog) = dom::el("tProg").dyn_into::<web_sys::Element>() {
                let _ = prog.set_attribute("style", "stroke-dashoffset:0");
            }
            dom::set_text("timerBadge", "");
        }
    });
}

// ---------- delayed callback helpers ----------

pub fn schedule(app: &App, delay_ms: i32, f: impl FnOnce(&App) + 'static) {
    let app2 = app.clone();
    let cb = Closure::once(move || f(&app2));
    let _ = dom::window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), delay_ms);
    cb.forget();
}

pub fn schedule_raw(delay_ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once(f);
    let _ = dom::window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), delay_ms);
    cb.forget();
}

// ---------- misses / review mode ----------

pub fn enter_review(app: &App) {
    let total = app.borrow().misses.len();
    if total == 0 {
        dom::set_text("feedback", "No misses yet \u{2014} they collect here as you play.");
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    let due = misses::due_misses(&app.borrow()).len();
    if due == 0 {
        dom::set_text("feedback", &format!("All caught up \u{2014} {} word{} scheduled for later review.", total, if total == 1 { "" } else { "s" }));
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    app.borrow_mut().review = true;
    stop_timer(true);
    clear_meaning();
    dom::set_disabled("langSel", true);
    dom::set_disabled("levelSel", true);
    {
        let mut s = app.borrow_mut();
        s.word = String::new();
        s.answered = false;
    }
    dom::set_html("orbGlyph", "tap to<br/>practice misses");
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    drawing::clear_canvas();
    dom::set_text("hintLine", "");
    render_tries(app);
    dom::set_text("feedback", &format!("Practicing your misses \u{2014} {} due now.", due));
    dom::el("feedback").set_class_name("feedback");
    refresh_mode_buttons(app);
}

pub fn exit_review(app: &App, msg: Option<&str>) {
    app.borrow_mut().review = false;
    stop_timer(true);
    clear_meaning();
    dom::set_disabled("langSel", false);
    dom::set_disabled("levelSel", false);
    {
        let mut s = app.borrow_mut();
        s.cur_lang = s.lang.clone();
        s.word = String::new();
        s.answered = false;
    }
    dom::set_html("orbGlyph", "tap to<br/>hear a word");
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    drawing::clear_canvas();
    dom::set_text("hintLine", "");
    render_tries(app);
    update_voice_note(app);
    refresh_mode_buttons(app);
    if let Some(m) = msg {
        dom::set_text("feedback", m);
        dom::el("feedback").set_class_name("feedback good");
    } else {
        dom::set_text("feedback", "");
        dom::el("feedback").set_class_name("feedback");
    }
}

// ---------- save-score modal ----------

pub fn open_save(app: &App, streak: u32) {
    app.borrow_mut().pending_score = streak;
    dom::set_text("modalScore", &streak.to_string());
    dom::set_text("modalMsg", &format!("{} word{} spelled in a row. Add your name to the board.", streak, if streak == 1 { "" } else { "s" }));
    let name = app.borrow().saved_name.clone();
    dom::input("nameInput").set_value(&name);
    dom::add_class("scrim", "show");
    dom::input("nameInput").focus().ok();
}

pub fn commit_save(app: &App) {
    let name = dom::input("nameInput").value();
    let streak = app.borrow().pending_score;
    board::save_score(&mut app.borrow_mut(), &name, streak);
    close_save(app);
}

pub fn close_save(app: &App) {
    dom::remove_class("scrim", "show");
    reset_chain_soft(app);
}

// ---------- head-to-head (versus) ----------

fn clean_name(name: &str, fallback: &str) -> String {
    let t = name.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// Enters head-to-head and starts the first turn. Kid Mode shortens the match
/// (2 turns each vs 3) so it stays quick for younger players.
pub fn start_versus(app: &App, name1: String, name2: String) {
    if app.borrow().review {
        exit_review(app, None);
    }
    stop_timer(true);
    clear_meaning();
    let turns = if app.borrow().kid { 2 } else { 3 };
    let n1 = clean_name(&name1, "Player 1");
    let n2 = clean_name(&name2, "Player 2");
    app.borrow_mut().versus = crate::versus::Versus::start(n1, n2, turns);
    if let Some(body) = dom::doc().body() {
        let _ = body.class_list().add_1("versus");
    }
    dom::remove_class("vsBar", "btn-hide");
    dom::remove_class("vsResultScrim", "show");
    // Misses/review don't apply during a match.
    dom::set_disabled("missesBtn", true);
    render_versus_bar(app);
    begin_versus_turn(app);
}

/// Leaves head-to-head and restores normal single-player play.
pub fn exit_versus(app: &App) {
    app.borrow_mut().versus = crate::versus::Versus::default();
    if let Some(body) = dom::doc().body() {
        let _ = body.class_list().remove_1("versus");
    }
    dom::add_class("vsBar", "btn-hide");
    dom::remove_class("vsResultScrim", "show");
    dom::remove_class("vsSetupScrim", "show");
    dom::remove_class("vsQuitScrim", "show");
    dom::set_disabled("missesBtn", false);
    stop_timer(true);
    // Tear down any in-flight word audio so nothing keeps playing after exit.
    api::stop();
    speech_out::stop();
    clear_meaning();
    {
        let mut s = app.borrow_mut();
        s.streak = 0;
        s.word = String::new();
        s.answered = false;
    }
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    dom::set_html("orbGlyph", "tap to<br/>hear a word");
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    drawing::clear_canvas();
    dom::set_text("hintLine", "");
    render_tries(app);
    dom::set_text("streakNum", "0");
    dom::set_text("bestNum", &app.borrow().best.to_string());
    dom::set_text("feedback", "");
    dom::el("feedback").set_class_name("feedback");
    refresh_mode_buttons(app);
    update_voice_note(app);
}

/// Restarts a fresh match with the same two players (from the winner screen).
pub fn versus_rematch(app: &App) {
    let (n1, n2) = {
        let v = &app.borrow().versus;
        (v.p1.name.clone(), v.p2.name.clone())
    };
    dom::remove_class("vsResultScrim", "show");
    start_versus(app, n1, n2);
}

/// Sets the stage for the active player to start their turn (idle, waiting for
/// an orb tap to hear the first word).
fn begin_versus_turn(app: &App) {
    stop_timer(true);
    clear_meaning();
    {
        let mut s = app.borrow_mut();
        s.word = String::new();
        s.answered = false;
        s.tries_left = MAX_TRIES;
    }
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    let name = app.borrow().versus.active_player().name.clone();
    dom::set_html("orbGlyph", &format!("{}<br/>tap for a word", dom::escape_html(&name)));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    sync_keyboard(app);
    drawing::clear_canvas();
    dom::set_text("hintLine", "");
    render_tries(app);
    dom::set_text(
        "feedback",
        &format!("{}\u{2019}s turn \u{2014} spell as many as you can before a miss.", name),
    );
    dom::el("feedback").set_class_name("feedback");
    render_versus_bar(app);
}

/// Versus counterpart of `on_correct`: extends the active player's chain and
/// keeps their turn going, without the single-player streak/board/misses/
/// achievement machinery.
fn versus_on_correct(app: &App) {
    crate::haptics::correct();
    // Head-to-head writes NOTHING to persistent stats (accuracy, achievements,
    // leaderboard, misses, word stats) — an abandoned or finished match never
    // pollutes solo progress. Only the in-memory versus scoreboard updates.
    let (cur_lang, word) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.word.clone())
    };
    app.borrow_mut().versus.record_correct();
    render_versus_bar(app);

    dom::add_class("orbWrap", "good");
    dom::set_text("orbGlyph", "\u{2713}");
    let praise = {
        let s = app.borrow();
        let p = s.versus.active_player();
        format!("{} \u{2014} chain of {}", p.name, p.current)
    };
    dom::set_text("feedback", &praise);
    dom::el("feedback").set_class_name("feedback good");
    show_meaning(app, word, cur_lang);
    schedule(app, CORRECT_DELAY_MS, |app| next_word(app));
}

/// A miss ends the active player's turn; hand off to the other player, or show
/// the winner screen if the match is over.
fn versus_end_turn(app: &App) {
    app.borrow_mut().versus.end_turn();
    render_versus_bar(app);
    let over = app.borrow().versus.over;
    if over {
        schedule(app, 1300, |app| show_versus_result(app));
    } else {
        schedule(app, 1300, |app| begin_versus_turn(app));
    }
}

/// Repaints the two-player scoreboard (names, current + best chain, active
/// indicator). Safe no-op outside a match.
pub fn render_versus_bar(app: &App) {
    let s = app.borrow();
    if !s.versus.enabled {
        return;
    }
    let v = &s.versus;
    for (side, name_id, cur_id, best_id, cell_id) in [
        (Side::P1, "vsP1Name", "vsP1Cur", "vsP1Best", "vsP1"),
        (Side::P2, "vsP2Name", "vsP2Cur", "vsP2Best", "vsP2"),
    ] {
        let p = v.player(side);
        dom::set_text(name_id, &p.name);
        dom::set_text(cur_id, &p.current.to_string());
        dom::set_text(best_id, &p.best.to_string());
        dom::toggle_class(cell_id, "active", !v.over && v.active == side);
    }
}

fn show_versus_result(app: &App) {
    let (title, msg) = {
        let s = app.borrow();
        let v = &s.versus;
        match v.winner() {
            Some(side) => (
                format!("{} wins!", v.player(side).name),
                format!("{} {} vs {} {} \u{2014} longest chain wins.", v.p1.name, v.p1.best, v.p2.name, v.p2.best),
            ),
            None => (
                "It\u{2019}s a tie!".to_string(),
                format!("Both reached a chain of {}.", v.p1.best),
            ),
        }
    };
    dom::set_text("vsResultTitle", &title);
    dom::set_text("vsResultMsg", &msg);
    dom::add_class("vsResultScrim", "show");
}
