use std::cell::RefCell;

use unicode_segmentation::UnicodeSegmentation;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::consts::{is_builtin_lang, tier_time, CORRECT_DELAY_MS, EN, ES, LEVEL_OPTS, MINE, PRAISE, REVIEW};
use crate::flags;
use crate::model::AppState;
use crate::versus::Side;
use crate::{achievements, api, board, dom, misses, native_lang, selection, speech_out, stats, wordstats, words};
use crate::App;

const RING_C: f64 = 2.0 * std::f64::consts::PI * 108.0;

// ---------- naming / access helpers ----------

pub fn code_for(state: &AppState, key: &str) -> String {
    if key == MINE {
        // Per-word "Speak in" language (mixed-language lists) — the current word's
        // batch language wins, then the set default, then English.
        if let Some(l) = state.custom.word_lang.get(&state.word).filter(|l| !l.is_empty()) {
            return l.clone();
        }
        return if state.custom.speak_lang.is_empty() { "en-US".to_string() } else { state.custom.speak_lang.clone() };
    }
    match key {
        ES => "es-ES".to_string(),
        _ => "en-US".to_string(),
    }
}

/// For My Words: the short language code of the "Speak in" selection when it's a
/// language the app fully supports (its own keyboard + a backend TTS voice) —
/// e.g. "ko-KR" -> "ko", "cmn-CN" -> "zh". None for browser-synth-only picks
/// (Romanian/Indonesian/Catalan), which keep the English keyboard + browser
/// voice. This makes an imported Korean/Japanese/Thai/… list get the matching
/// keyboard and native audio, not just English.
pub fn mine_lang(state: &AppState) -> Option<&'static str> {
    let code = if state.custom.speak_lang.is_empty() { "en" } else { state.custom.speak_lang.as_str() };
    match code.split(['-', '_']).next().unwrap_or("") {
        "en" => Some("en"), "es" => Some("es"), "fr" => Some("fr"), "de" => Some("de"),
        "pt" => Some("pt"), "pl" => Some("pl"), "vi" => Some("vi"),
        "ko" => Some("ko"), "ja" => Some("ja"), "th" => Some("th"), "fil" => Some("fil"),
        "cmn" | "zh" => Some("zh"),
        _ => None,
    }
}

/// My Words can route through the backend's real TTS (reliable, unlike the
/// browser's built-in voices which may not exist at all on this machine).
/// The backend accepts any script Python's str.isalpha() allows — including
/// CJK/Thai/Hangul — so a word is backend-speakable when the "Speak in" language
/// is a supported one (mine_lang) and the word carries no spaces/punctuation
/// (which the backend rejects).
fn backend_speakable(state: &AppState, word: &str) -> bool {
    mine_lang(state).is_some()
        && !word.is_empty()
        && word.chars().all(|c| c.is_alphabetic())
}

pub fn name_for(_state: &AppState, key: &str) -> String {
    if key == MINE {
        return "My Words".to_string();
    }
    if key == REVIEW {
        return "Misses".to_string();
    }
    crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(code, _, _, _)| *code == key)
        .map(|(_, name, _, _)| name.to_string())
        .unwrap_or_else(|| "English".to_string())
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
    let pool = if state.lang == MINE {
        pool_for_tier(state, tier)
    } else {
        words::tier_for(&state.lang, tier).iter().map(|s| s.to_string()).collect()
    };
    // Kid Mode "friendly words": drop age-inappropriate terms (the content layer
    // Kid Mode's tier cap doesn't cover). Built-in pools only — My Words is
    // parent-curated (and still runs the global profanity filter).
    if state.kid && state.lang != MINE {
        crate::kid_filter::filter_kid(&state.lang, pool)
    } else {
        pool
    }
}

fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (js_sys::Math::random() * len as f64).floor() as usize % len
}

/// True while there's a word on screen the player can act on (answer,
/// replay...).
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
    if !is_builtin_lang(&s.lang) {
        return;
    }
    let mut tier = if s.level == "climb" {
        tier_for_streak(s.streak).to_string()
    } else {
        s.level.clone()
    };
    // Kid Mode caps difficulty at Hard: only Expert (the spelling nightmares)
    // steps down, so easy→medium→hard stay distinct instead of collapsing to
    // one pool.
    if s.kid && tier == "expert" {
        tier = "hard".to_string();
    }
    let pool = active_word_list(&s, &tier);
    let lang = s.lang.clone();
    drop(s);
    for word in pool.into_iter().take(PRELOAD_AT_OPEN) {
        api::preload_word(&word, &lang);
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

/// Show the coming-soon panel for a not-yet-active study language (registry):
/// fills the localized notice + the Notify Me button (confirmed if already
/// tapped on this device) and hides the play area via `body.coming-soon`. The
/// interface language is switched separately by the caller (uiLang untouched).
pub fn render_coming_soon(lang: &str) {
    let name = crate::consts::BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, n, _, _)| *n).unwrap_or(lang);
    dom::set_text("comingNotice", &crate::i18n::tp("coming.notice", &[("lang", name)]));
    let done = crate::notify::has(lang);
    dom::set_text("notifyBtn", &crate::i18n::t(if done { "coming.confirmed" } else { "coming.notify" }));
    dom::toggle_class("notifyBtn", "confirmed", done);
    dom::set_disabled("notifyBtn", done);
    let _ = dom::el("notifyBtn").set_attribute("data-lang", lang);
    if let Some(b) = dom::doc().body() {
        let _ = b.class_list().add_1("coming-soon");
    }
}

/// Hide the coming-soon panel and restore the play area (active language).
pub fn clear_coming_soon() {
    if let Some(b) = dom::doc().body() {
        let _ = b.class_list().remove_1("coming-soon");
    }
}

/// Climb band index → tier name (0=easy … 3=expert).
fn band_to_tier(band: u8) -> &'static str {
    match band.min(3) {
        0 => "easy",
        1 => "medium",
        2 => "hard",
        _ => "expert",
    }
}

/// Option A — Climb's gentle band. Promote after CLIMB_PROMOTE correct answers;
/// on a miss drop just ONE tier instead of crashing to easy, so hard languages
/// (e.g. Mandarin pinyin, where a low streak is easy to hit) keep climbing. Solo
/// Climb only — versus keeps its per-player chain, and the streak/chain itself is
/// left completely untouched. Session-only.
const CLIMB_PROMOTE: u8 = 3;
fn note_climb(app: &App, correct: bool) {
    let mut s = app.borrow_mut();
    if s.level != "climb" || s.versus.enabled || s.daily.active || s.review {
        return;
    }
    if correct {
        s.climb_prog += 1;
        if s.climb_prog >= CLIMB_PROMOTE {
            s.climb_band = (s.climb_band + 1).min(3);
            s.climb_prog = 0;
        }
    } else {
        s.climb_band = s.climb_band.saturating_sub(1);
        s.climb_prog = 0;
    }
}

// ---------- rendering ----------

/// Render the in-progress answer.
///
/// # Why there are two paths (CC-RTL F4, "parallel" per P0.3)
/// The per-letter `<span>` in [`render_letters_split`] is what lets each typed
/// letter `pop`. It is also what shatters a cursive word: shaping engines join
/// letters within a text RUN, and separate inline elements break the run, so
/// Arabic renders as isolated letterforms — measured at +26.6% median width, i.e.
/// `كتاب` becoming `ك ت ا ب` (see `spike/rtl-feedback/FINDINGS.md`).
///
/// So a cursive script gets ONE text node and no per-letter animation. Everything
/// else keeps today's markup **byte-for-byte** — that is the point of the parallel
/// choice: English, the only Active language, is not touched. Unifying (deleting
/// the span path entirely and giving every script measured markers) is the option
/// P0.3 leaves open to Eric; the split lives behind `script_joins`, so taking that
/// option later means deleting a branch rather than unpicking one.
pub fn render_letters(app: &App, animate_all: bool) {
    reflect_play_direction(app);
    let value = app.borrow().answer.clone();
    if value.is_empty() {
        dom::set_html("letters", &format!("<span class=\"placeholder\">{}</span><span class=\"caret\"></span>", crate::i18n::t("ph.typeHeard")));
        app.borrow_mut().prev_letter_len = 0;
        return;
    }
    let joins = crate::consts::script_joins(&app.borrow().lang);
    let html = if joins {
        render_letters_joined(&value)
    } else {
        render_letters_split(app, &value, animate_all)
    };
    dom::set_html("letters", &html);
    app.borrow_mut().prev_letter_len = value.chars().count();
}

/// CC-RTL **F1** — carry the study language's direction onto the play surface.
///
/// # Why this exists, and why `<html lang>` could not do it
/// `<html lang>` is the **UI locale** — the language of the chrome. The study
/// language is a different thing entirely, and they routinely differ: an Arabic
/// speaker learning English has `lang="ar"` on the document and is spelling
/// Latin words. So any rule keyed on `<html lang>` — `:lang(ar)`, `[dir=rtl]`
/// inherited from the root — fires for exactly the wrong person. Nothing carried
/// the study language before this, which is why `.reveal`'s `letter-spacing`
/// could not be fixed for cursive scripts: there was no selector that could tell
/// "spelling Arabic" from "reading Arabic".
///
/// Now the play surfaces carry `lang` + `dir` for the word being spelled, both
/// straight from the registry (D3: "no hardcoded language→direction checks
/// anywhere else"). CSS can finally ask.
fn reflect_play_direction(app: &App) {
    // `cur_lang`, NOT `lang`. `lang` is the language SELECTED in the picker;
    // `cur_lang` is the language of the word actually on screen, and they diverge:
    // Misses replays each word in its own language (game.rs sets
    // `cur_lang = m.lang`), and Daily uses its own locale. Keying off `lang` would
    // put the picker's direction on someone else's word — the same class of
    // mistake as keying off <html lang>, one level down.
    let (cur, sel) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.lang.clone())
    };
    let code = if !cur.is_empty() { cur } else { sel };
    // My Words / Misses are pseudo-languages; fall back to the imported list's
    // speak-language where there is one, exactly as the keyboard does.
    let code = if code == crate::consts::MINE {
        mine_lang(&app.borrow()).unwrap_or("en").to_string()
    } else {
        code
    };
    let dir = crate::consts::dir_attr(&code);
    // `hintLine` shows the same word masked, so it carries the same direction and
    // was simply missed by F1's first pass — a word surface with no direction is
    // the bug F1 exists to prevent, just somewhere less obvious than #letters.
    //
    // Say It's word deliberately is NOT here. It looks like it belongs, but this
    // function keys off `cur_lang` (the word on the PLAY surface) while Say It
    // picks its own word from `lang` (the picker), and Misses makes those two
    // diverge. Stamping this direction there would put one word's direction on a
    // different word — F1's own mistake, one surface over. say_it sets its own.
    for id in ["letters", "feedback", "meaning", "hintLine"] {
        let el = dom::el(id);
        let _ = el.set_attribute("lang", &code);
        let _ = el.set_attribute("dir", dir);
    }
}

/// Today's markup, unchanged: one element per character so each can `pop`.
/// Correct for every non-joining script — Latin, Cyrillic, Hangul, kana — where
/// splitting costs nothing because nothing joins.
fn render_letters_split(app: &App, value: &str, animate_all: bool) -> String {
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
    html
}

/// Cursive scripts: the whole answer in ONE text node, so the shaper sees the
/// word and joins it. `dir="rtl"` is carried on the run itself and
/// `unicode-bidi: isolate` (CSS) stops an RTL answer reordering against the LTR
/// chrome around it.
///
/// No per-letter `pop`: you cannot animate a glyph you have not split, and
/// splitting is precisely the bug. That is a real loss, and it is why this path is
/// confined to the scripts that need it.
fn render_letters_joined(value: &str) -> String {
    format!(
        "<span class=\"ltr joined\" dir=\"rtl\">{}</span><span class=\"caret\"></span>",
        dom::escape_html(value),
    )
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
    let (enabled, punct) = {
        let s = app.borrow();
        // Apostrophe/hyphen keys are for English imports (contractions, hyphens);
        // My Words in another language uses that language's own keyboard instead.
        let punct = s.lang == MINE && matches!(mine_lang(&s), None | Some("en") | Some("zh"));
        (can_type(&s), punct)
    };
    dom::toggle_class("gameKeyboard", "locked", !enabled);
    dom::toggle_class("gameKeyboard", "show-punct", punct);
    dom::toggle_class("spellbox", "focus", enabled);
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

/// Route a key press to the right input path for the active keyboard: Korean
/// composes jamo into syllables (crate::hangul); everything else appends.
pub fn emit_key(app: &App, ch: char) {
    if crate::keyboard::active_is_korean(app) {
        type_jamo(app, ch);
    } else {
        type_char(app, ch);
    }
}

/// Korean: feed one jamo through the Hangul composition automaton and replace
/// the answer with the recomposed buffer.
pub fn type_jamo(app: &App, jamo: char) {
    if !can_type(&app.borrow()) {
        return;
    }
    let composed = crate::hangul::feed(&app.borrow().answer, jamo);
    app.borrow_mut().answer = composed;
    render_letters(app, true);
    crate::haptics::key_tap();
    announce(&jamo.to_string());
}

/// Vietnamese tone key: apply `tone` to the last typed vowel (replacing any
/// existing tone). No-op if the last character isn't a Vietnamese vowel.
pub fn apply_vi_tone(app: &App, tone: char) {
    if !can_type(&app.borrow()) {
        return;
    }
    let mut ans = app.borrow().answer.clone();
    let Some(last) = ans.chars().last() else {
        return;
    };
    if let Some(retoned) = crate::viet::retone(last, tone) {
        ans.pop();
        ans.push_str(&retoned);
        app.borrow_mut().answer = ans;
        render_letters(app, true);
        crate::haptics::key_tap();
    }
}

/// Replace the answer buffer from an external input method (voice spelling) and
/// re-render, respecting the same `can_type` gate as the on-screen keyboard. Used
/// by Spell It Out Loud to append parsed letters / revert on a rejected utterance;
/// it produces exactly what typing would (Invariant I1) and never auto-submits.
pub fn set_answer(app: &App, text: &str) {
    if !can_type(&app.borrow()) {
        return;
    }
    app.borrow_mut().answer = text.to_string();
    render_letters(app, false);
}

/// Delete the last character of the answer.
pub fn backspace(app: &App) {
    if !can_type(&app.borrow()) {
        return;
    }
    let ko = crate::keyboard::active_is_korean(app);
    let ans = app.borrow().answer.clone();
    if ans.is_empty() {
        return;
    }
    // Korean decomposes one jamo at a time (한 → 하 → ㅎ → ∅); others pop a char.
    let next = if ko {
        crate::hangul::backspace(&ans)
    } else {
        let mut s = ans;
        s.pop();
        s
    };
    app.borrow_mut().answer = next;
    render_letters(app, true);
    crate::haptics::key_tap();
    announce("delete");
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
    if is_builtin_lang(&cur_lang) || (cur_lang == MINE && backend_speakable(&s, &word)) {
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

/// The legacy per-word retry pips are RETIRED (build-54: one attempt per word;
/// the only retries are shields / the extra-attempts toggle). This keeps the
/// retired row cleared wherever the old mode-entry/reset flows still call it, so
/// a stray pip can never render.
pub fn render_tries(_app: &App) {
    dom::set_html("triesLine", "");
}

pub fn refresh_mode_buttons(app: &App) {
    let s = app.borrow();
    let total = s.misses.len();
    let due = misses::due_misses(&s).len();
    // Data-chip markup: icon + label + live count badge (home-regroup F5). In
    // review mode the chip becomes the "exit" affordance (no badge). Labels come
    // from our own locale strings (no untrusted HTML).
    let html = if s.review {
        format!("<span class=\"dc-ico\">\u{21bb}</span><span class=\"dc-lbl\">{}</span>", crate::i18n::t("top.missesExit"))
    } else {
        format!(
            "<span class=\"dc-ico\">\u{21bb}</span><span class=\"dc-lbl\">{}</span><b class=\"dc-badge\">{}</b>",
            crate::i18n::t("top.misses"),
            due
        )
    };
    dom::set_html("missesBtn", &html);
    dom::el("missesBtn").set_attribute("title", &if total > 0 { format!("{} saved \u{b7} {} due now", total, due) } else { String::new() }).ok();
    dom::toggle_class("missesBtn", "on", s.review);
    dom::set_disabled("missesBtn", !s.review && total == 0);
}

/// Keep the setup chip's summary in lock-step with the three round-parameter
/// pickers (home-regroup F3 — the chip and the actual round config never
/// disagree). Labels are computed from the same state + locale strings the
/// selects are built from, so the chip always matches what a tap would start.
/// The trailing affordance arrow on "Climb →" is trimmed for the value list.
pub fn update_setup_chip(app: &App) {
    let s = app.borrow();
    let lang_label = if s.lang == MINE {
        "My Words".to_string()
    } else {
        crate::consts::BUILTIN_LANGS
            .iter()
            .find(|(c, _, _, _)| *c == s.lang)
            .map(|(_, n, _, _)| (*n).to_string())
            .unwrap_or_else(|| s.lang.clone())
    };
    let level_label = crate::i18n::t(&format!("level.{}", s.level));
    let level_label = level_label.trim_end_matches(|c: char| c == '\u{2192}' || c.is_whitespace());
    let timing_label = crate::i18n::t(if s.timed { "mode.timed" } else { "mode.untimed" });
    dom::set_text("setupChipText", &format!("{} \u{b7} {} \u{b7} {}", lang_label, level_label, timing_label));
    // Per-difficulty explanation under the level picker (Pillar 3): a localized
    // one-liner for whatever level is selected, surfaced right where it's chosen.
    dom::set_text("levelDesc", &crate::i18n::t(&format!("level.{}.desc", s.level)));
}


pub fn build_source_options(app: &App) {
    let s = app.borrow();
    let mut opts = String::new();
    // NOTE: play-gating of coming-soon languages in the picker is intentionally
    // NOT applied here yet — langSel currently drives BOTH the study language and
    // the UI language (home-regroup unification), so disabling coming-soon here
    // would also strand UI-language selection, which this feature must leave
    // untouched. Pending the study/UI separation decision. Registry is the source
    // of truth (consts::is_active_lang) once that lands.
    for (code, name, _status, _) in crate::consts::BUILTIN_LANGS {
        opts.push_str(&format!("<option value=\"{}\">{}</option>", code, name));
    }
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
    update_setup_chip(app);
}

pub fn build_level_options(app: &App) {
    let s = app.borrow();
    let opts: String = LEVEL_OPTS
        .iter()
        .map(|(v, _)| {
            let label = crate::i18n::t(&format!("level.{v}"));
            format!("<option value=\"{v}\">{label}</option>")
        })
        .collect();
    dom::set_html("levelSel", &opts);
    dom::select("levelSel").set_value(&s.level);
    drop(s);
    update_setup_chip(app);
}

// ---------- meanings (dictionary lookups) ----------

thread_local! {
    static MEANING_SEQ: RefCell<u64> = RefCell::new(0);
    // One beloved-word sparkle per session.
    static BELOVED_SHOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn clear_meaning() {
    MEANING_SEQ.with(|c| *c.borrow_mut() += 1);
    dom::set_html("meaning", "");
    dom::remove_class("meaning", "show");
    crate::word_stories::clear();
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
    // Language beauty layer: an optional per-word insight (etymology, character
    // components, tone family…). Keyed on the hanzi for Mandarin (the insight is
    // about the character, not the pinyin), else the word. Shown as a subordinate
    // second line; and since non-English words have no backend definition, it
    // renders the card on its own when present.
    let insight_html = {
        let s = app.borrow();
        let key = if s.cur_lang == crate::consts::ZH { s.spoken.clone() } else { word.clone() };
        crate::enrich::insight(&lang_key, &key).and_then(|ins| {
            // Word Stories (F5): when the flag is on, etymology insights render in
            // their own "did you know?" card, so drop them from the meaning card's
            // second line to avoid a double display. Flag off -> unchanged.
            if crate::flags::word_stories() && ins.kind == "etymology" {
                return None;
            }
            // Beloved words (no_equivalent / usage_gem gems) get a subtle sparkle,
            // at most once per session.
            let sparkle = if ins.is_beloved() && !BELOVED_SHOWN.with(|c| c.replace(true)) {
                " \u{2728}"
            } else {
                ""
            };
            Some(format!("<span class=\"m-insight\">{}{}</span>", dom::escape_html(&ins.text), sparkle))
        })
    };
    // Word Stories (F5): a small dismissible etymology card on the result
    // surface, shown only when a story exists and the flag is on (Kid Mode too).
    // A no-op while the flag is off, so it never blocks the next word.
    crate::word_stories::render(&lang_key, &word);
    if let Some(ref ins) = insight_html {
        dom::set_html("meaning", &format!("<span class=\"m-word\">{}</span>{}", dom::escape_html(&word), ins));
        dom::add_class("meaning", "show");
    }
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
        if let Some(ins) = &insight_html {
            html.push_str(ins);
        }
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
                dom::set_html("meaning", &format!("<span class=\"m-pos\">{}</span>", crate::i18n::t("meaning.noDef")));
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
                dom::set_html("meaning", &format!("<span class=\"m-pos\">{}</span>", crate::i18n::t("meaning.noExample")));
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
    // Speak `spoken` (the hanzi for Mandarin; identical to `word` elsewhere).
    let word = if s.spoken.is_empty() { s.word.clone() } else { s.spoken.clone() };
    let code = code_for(&s, &s.cur_lang);

    if is_builtin_lang(&s.cur_lang) {
        let lang = s.cur_lang.clone();
        drop(s);
        api::play_word(&word, variant, rate as f64, &lang, || {});
        return;
    }
    if s.cur_lang == MINE && backend_speakable(&s, &word) {
        // My Words with a supported "Speak in" language: use that language's
        // native backend voice (ko-KR, ja-JP, …), falling back to the browser's
        // voice for that locale if the backend request fails.
        let lang = mine_lang(&s).unwrap_or(EN);
        drop(s);
        let fallback_word = word.clone();
        let fallback_rate = if variant == "slow" { 0.55 } else { rate };
        api::play_word(&word, variant, rate as f64, lang, move || speech_out::speak(&fallback_word, fallback_rate, &code));
        return;
    }
    drop(s);
    let browser_rate = if variant == "slow" { 0.55 } else { rate };
    speech_out::speak(&word, browser_rate, &code);
}

pub fn next_word(app: &App) {
    clear_meaning();
    // Daily Challenge: finish once the fixed set is exhausted.
    if {
        let s = app.borrow();
        s.daily.active && s.daily.idx >= s.daily.words.len()
    } {
        finish_daily(app);
        return;
    }
    {
        let mut s = app.borrow_mut();
        if s.daily.active {
            let w = s.daily.words[s.daily.idx].clone();
            s.daily.idx += 1;
            s.cur_lang = s.daily.locale.clone();
            s.cur_tier = length_tier(&w).to_string();
            s.word = w;
        } else if s.review {
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
            // Climb tier: head-to-head tracks the active player's current chain;
            // solo uses the gentle band (Option A) so a miss drops one tier, not
            // all the way to easy — hard languages keep progressing.
            let mut tier = if s.level == "climb" {
                if s.versus.enabled {
                    tier_for_streak(s.versus.active_player().current).to_string()
                } else {
                    band_to_tier(s.climb_band).to_string()
                }
            } else {
                s.level.clone()
            };
            // Kid Mode caps at Hard (Expert nightmares excluded) while keeping
            // easy→medium→hard distinct.
            if s.kid && tier == "expert" {
                tier = "hard".to_string();
            }
            s.cur_tier = tier.clone();
            let pool = active_word_list(&s, &tier);
            if !pool.is_empty() {
                let key = format!("{}:{}", s.lang, tier);
                // Every mode draws from the same persisted, no-repeat shuffled
                // deck per (language, tier): a word never recurs until the whole
                // pool is exhausted, and the deck survives app restarts (I3/I4).
                // Missed-word resurfacing is handled separately by Misses review
                // (spaced repetition), which the deck deliberately doesn't touch.
                let w = s.decks.entry(key.clone()).or_default().next(&pool);
                s.word = w;
                // Warm the browser's audio cache for whatever this same
                // pool will hand out next time, so that turn's playback is
                // instant instead of waiting on a fresh TTS fetch.
                if is_builtin_lang(&s.cur_lang) {
                    if let Some(next_up) = s.decks.get(&key).and_then(|d| d.peek()) {
                        api::preload_word(&next_up, &s.cur_lang);
                    }
                }
            }
        }
    }

    // Persist the decks so the free-play no-repeat cursor survives an app
    // restart and carries across days (Feature 2 / I4).
    crate::storage::set_json(crate::model::DECKS_KEY, &app.borrow().decks);

    // Mandarin stores "pinyin|hanzi": the player types the pinyin, but TTS speaks
    // and the reveal shows the hanzi. Every other language sets spoken = word.
    {
        let mut s = app.borrow_mut();
        if let Some((pinyin, hanzi)) = s.word.split_once('|') {
            let (p, h) = (pinyin.to_string(), hanzi.to_string());
            s.word = p;
            s.spoken = h;
        } else {
            s.spoken = s.word.clone();
        }
    }

    app.borrow_mut().answered = false;
    // CC-ATTEMPTS-SHIELDS: a new word refreshes its one-retry budget (I4/PD5).
    crate::attempts::start_word(&mut app.borrow_mut());
    update_shield_hud(app);
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    dom::set_html("orbGlyph", &crate::i18n::t("orb.listen"));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
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
    if app.borrow().daily.active {
        render_daily_bar(app);
    }
    speak_current(app);
    let cur_tier = app.borrow().cur_tier.clone();
    start_timer(app, &cur_tier);
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
    let (answered, composing, cur_lang, word, kid) = {
        let s = app.borrow();
        (s.answered, s.composing, s.cur_lang.clone(), s.word.clone(), s.kid)
    };
    // Never validate while an IME composition is open: the typed answer isn't
    // final until compositionend. Shared guard on the single submission path
    // (I1), so it holds for every language and every submit affordance.
    if composing {
        return;
    }
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
        spawn_local(backend_verify(app.clone(), word, typed, kid));
        return;
    }
    // Mandarin compares tone-numbered pinyin (v→ü, neutral-5 optional); every
    // other language uses the NFC/case fold (accent-strict, Kid-lenient).
    let correct = if cur_lang == crate::consts::ZH {
        crate::pinyin::matches(&typed, &word)
    } else if cur_lang == crate::consts::KO {
        // Korean grades at jamo granularity (Phase 3); an exact block match is
        // score 1.0. The per-jamo diff drives the wrong-answer coaching below.
        crate::jamo::grade(&typed, &word).correct
    } else {
        // Normal fold, plus the data-driven accept-any homophone layer: a real
        // homophone of the prompt (audio can't carry b/v, silent h, seseo,
        // yeísmo) is accepted. Empty table for languages without a file -> no-op.
        crate::norm::answer_matches(&typed, &word, kid)
            || crate::homophones::accepts(&cur_lang, &word, &typed)
    };
    if correct {
        on_correct(app);
    } else {
        on_wrong(app);
    }
}

/// English words get double-checked against the backend's /api/check; if
/// that request fails (server down, etc.) we fall back to comparing
/// locally so a network hiccup doesn't stall the game.
async fn backend_verify(app: App, word: String, typed: String, kid: bool) {
    let correct = match api::check_answer(&word, &typed).await {
        Ok(c) => c,
        Err(_) => crate::norm::answer_matches(&typed, &word, kid),
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
        return crate::i18n::t("fb.gotItReview");
    }
    let idx = rand_index(PRAISE.len());
    let mut base = crate::i18n::t(&format!("praise.{}", idx + 1));
    if s.streak >= 5 {
        base.push_str(&crate::i18n::tp("fb.inARow", &[("n", &s.streak.to_string())]));
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
    if app.borrow().daily.active {
        daily_answer(app, true);
        // Daily auto-advances after the success feedback. This lives in the one
        // language-agnostic validated-correct handler (on_correct) — downstream
        // of the single NFC-normalised comparison in submit_guess — so it has
        // ZERO per-language conditionals (I4). Reuses CORRECT_DELAY_MS, the same
        // beat the solo correct→next path already uses, keeping Daily's rhythm
        // identical to solo (synced to the success feedback). The orb stays a
        // live instant-skip (D3); `daily_auto_advance` is guarded so the timer
        // is a no-op if the player already advanced — at most one advance per
        // word (I6).
        // Pin the timer to the word index it was scheduled for: if the player
        // orb-skips to a later word faster than CORRECT_DELAY_MS, this stale timer
        // must be a no-op rather than advancing an extra, unscored word.
        let at_idx = app.borrow().daily.idx;
        schedule(app, CORRECT_DELAY_MS, move |app| daily_auto_advance(app, at_idx));
        return;
    }
    crate::haptics::correct();
    let (cur_lang, cur_tier, word) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.cur_tier.clone(), s.word.clone())
    };

    // CC-ATTEMPTS-SHIELDS PD4/I9: a word rescued by an extra attempt or a shield
    // is CORRECT-on-retry — the run continues, but this is NOT a "correct" for
    // scoring. Its first miss was already recorded; it must not re-record, must
    // not promote the miss out of spaced-rep, must not extend the streak or earn
    // streak, and (I9) must never be mistaken for an auto-advancing correct.
    let rescued = flags::attempts_shields() && app.borrow().aids.retry_used;
    if !rescued {
        stats::record(&mut app.borrow_mut(), &cur_lang, &cur_tier, true);
        // Adaptive word stats: solo practice only (Misses/review has its own SR).
        if !app.borrow().review {
            wordstats::record(&cur_lang, &word, true);
            selection::note_outcome(&cur_lang, &word, true);
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
        note_climb(app, true); // Option A: advance the Climb band on a correct answer
        // Ghost racing (F6): in a solo Climb run, log this correct answer's elapsed
        // time and refresh the live pace marker. Flag-gated inside crate::ghost.
        {
            let (climb_run, run_start, lang) = {
                let s = app.borrow();
                (s.level == "climb" && !s.review, s.run_start_ms, s.lang.clone())
            };
            if climb_run {
                if streak == 1 {
                    crate::ghost::start_run(&lang);
                }
                crate::ghost::note_correct(streak, now_ms() - run_start);
            }
        }
        // Shields (The Climb): earn one every SHIELD_EARN_STREAK consecutive
        // genuine corrects, capped. Earning flashes the HUD. (Two statements so
        // the immutable borrow is released before the mutable earn call —
        // otherwise RefCell would double-borrow at runtime.)
        let in_climb = shield_ctx(&app.borrow());
        let earned = in_climb && crate::attempts::shield_on_correct(&mut app.borrow_mut());
        dom::set_text("streakNum", &streak.to_string());
        dom::set_text("bestNum", &app.borrow().best.to_string());
        dom::add_class("orbWrap", "good");
        spell_feedback(true);
        set_streak_tier(streak);
        dom::set_text("orbGlyph", "\u{2713}");
        if earned {
            flash_shield_earned(app);
        } else {
            dom::set_text("feedback", &pick_praise(app));
            dom::el("feedback").set_class_name("feedback good");
        }
        update_shield_hud(app);
        show_meaning(app, word, cur_lang);
        achievements::check_streak(&mut app.borrow_mut());
    } else {
        // Rescued: advance without scoring. The streak stays where the miss left
        // it (0); the shield/attempt kept the run alive.
        dom::add_class("orbWrap", "good");
        spell_feedback(true);
        dom::set_text("orbGlyph", "\u{2713}");
        dom::set_text("feedback", &pick_praise(app));
        dom::el("feedback").set_class_name("feedback good");
        update_shield_hud(app);
        show_meaning(app, word, cur_lang);
    }

    schedule(app, CORRECT_DELAY_MS, |app| next_word(app));
}

/// Feedback color state on the spell box (F1). GLOBAL — identical in every
/// language and in Kid + non-Kid mode (kids learn one signal). Removing both
/// classes first restarts cleanly on rapid consecutive answers; the class is
/// then cleared on animationend (wired in lib.rs).
fn spell_feedback(correct: bool) {
    dom::remove_class("spellbox", "is-correct");
    dom::remove_class("spellbox", "is-wrong");
    dom::add_class("spellbox", if correct { "is-correct" } else { "is-wrong" });
}

/// Streak warmth tier (F2, D5): 0 (<3 in a row), 1 (3–5), 2 (6+). Warms the
/// `--glow` accent only — never text or button ink. A miss sets 0 immediately.
fn set_streak_tier(streak: u32) {
    let tier = if streak >= 6 { "2" } else if streak >= 3 { "1" } else { "0" };
    let _ = dom::el("stage").set_attribute("data-streak-tier", tier);
}

/// Record the miss and reveal the word.
fn finalize_incorrect(app: &App, glyph: &str, prefix: &str, feedback_class: &str) {
    finalize_incorrect_ex(app, glyph, prefix, feedback_class, true);
}

/// As `finalize_incorrect`, but `record` controls whether the learning outcome
/// (accuracy, Misses/spaced-rep, word stats) is written here. CC-ATTEMPTS-SHIELDS
/// passes `record = false` when the first submission already recorded the miss —
/// a retry (extra attempt or shield) changes session flow, not the learning
/// record, so the outcome must never be double-counted (I2 / A2 / A9).
fn finalize_incorrect_ex(app: &App, glyph: &str, prefix: &str, feedback_class: &str, record: bool) {
    let (versus_on, cur_lang, cur_tier, word) = {
        let s = app.borrow();
        (s.versus.enabled, s.cur_lang.clone(), s.cur_tier.clone(), s.word.clone())
    };
    // Head-to-head records nothing persistent (accuracy, Misses, leaderboard,
    // word stats) — the match never counts toward solo progress.
    if record && !versus_on {
        stats::record(&mut app.borrow_mut(), &cur_lang, &cur_tier, false);
        misses::add_miss(&mut app.borrow_mut(), &word, &cur_lang, &cur_tier);
        refresh_mode_buttons(app);
    }
    // Adaptive word stats: a loss (out of tries / timeout / give-up) is a miss,
    // for solo practice only (not head-to-head, not Misses/review).
    if record && !versus_on && !app.borrow().review {
        wordstats::record(&cur_lang, &word, false);
        selection::note_outcome(&cur_lang, &word, false);
    }

    dom::add_class("orbWrap", "bad");
    spell_feedback(false);
    set_streak_tier(0);
    dom::set_text("orbGlyph", glyph);
    // Mandarin reveals the pinyin answer + the hanzi it stands for.
    let reveal = if cur_lang == crate::consts::ZH {
        let hanzi = app.borrow().spoken.clone();
        format!("{} {}", word, hanzi)
    } else {
        word.clone()
    };
    // F7 "Syllable replay on misses" (es only): reveal the spelling split into
    // syllables with a "hear it slowly" control that replays the word
    // syllable-by-syllable, highlighting each in turn.
    //
    // This comment used to claim "Flag OFF (default) => zero behavioral
    // difference". That was false: flags::syllable_replay() is resolve(stored,
    // TRUE) — it defaults ON. What actually makes this unreachable is the `es`
    // gate, since Spanish is ComingSoon and cannot be played. Different reason,
    // same outcome, but "it's behind an off flag" would have been a dangerous
    // thing to believe while editing it.
    //
    // CC-RTL F4: the `.syl` branch below emits one <span> PER SYLLABLE, which is
    // the same shattering that F4 removed from #letters — a cursive word split
    // into syllable spans breaks its joins at every boundary. It is safe today
    // only because `es` is not a cursive script; `syllable_replay_cannot_reach_a_
    // cursive_script` pins that, and turns extending this to Arabic into a
    // deliberate act rather than a silent regression.
    let sylls: Vec<String> = if crate::flags::syllable_replay() && cur_lang == ES {
        crate::syllable::syllabify(&word)
    } else {
        Vec::new()
    };
    if sylls.len() >= 2 {
        let mut spans = String::new();
        for (i, s) in sylls.iter().enumerate() {
            spans.push_str(&format!("<span class=\"syl\" data-syl=\"{}\">{}</span>", i, dom::escape_html(s)));
        }
        let label = dom::escape_html(&crate::i18n::t("btn.hearSlowly"));
        dom::set_html(
            "feedback",
            &format!(
                "{}<span class=\"reveal syllabified\">{}</span> <button type=\"button\" class=\"syl-slow\" id=\"sylSlowBtn\" aria-label=\"{}\">\u{1f50a} {}</button>",
                prefix, spans, label, label
            ),
        );
        dom::el("feedback").set_class_name(feedback_class);
        wire_syllable_replay(app, sylls);
    } else {
        dom::set_html("feedback", &format!("{}<span class=\"reveal\">{}</span>", prefix, dom::escape_html(&reveal)));
        dom::el("feedback").set_class_name(feedback_class);
    }
    show_meaning(app, word, cur_lang);
    if versus_on {
        versus_end_turn(app);
    } else {
        end_chain(app);
    }
}

// ---------- F7: syllable replay on the reveal surface ----------

/// Wire the "hear it slowly" button injected into the reveal. Clicking replays
/// the word syllable-by-syllable, highlighting each `.syl` span in turn.
fn wire_syllable_replay(app: &App, sylls: Vec<String>) {
    let Ok(btn) = dom::el("sylSlowBtn").dyn_into::<web_sys::HtmlElement>() else {
        return;
    };
    let a = app.clone();
    let cb = Closure::<dyn FnMut()>::new(move || play_syllables(&a, &sylls));
    let _ = btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Replay the reveal word syllable-by-syllable. Native path: AVSpeech speaks the
/// tokens in one utterance and its boundary callbacks drive the highlight (exact
/// timing). Web path: the browser voice speaks the word slowly and the SAME
/// highlight is driven by per-syllable time estimates — identical on screen.
/// Capability is discovered via `native_lang::available()`; no platform ifs.
fn play_syllables(app: &App, sylls: &[String]) {
    clear_syllable_highlight();
    let rate = app.borrow().rate as f64;
    if native_lang::available() {
        let owned: Vec<String> = sylls.to_vec();
        spawn_local(async move {
            // The native path needs an installed es voice; if none, use the web
            // timer path so the affordance still works.
            let Some(voice) = native_lang::session_voice(ES).await else {
                web_syllable_fallback(&owned, rate);
                return;
            };
            let on_index = Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(|v: wasm_bindgen::JsValue| {
                if let Some(idx) = v.as_f64() {
                    highlight_syllable(idx as usize);
                }
            });
            let f: &js_sys::Function = on_index.as_ref().unchecked_ref();
            match native_lang::speak_syllables(&owned, &voice, rate.min(0.7), f) {
                Some(promise) => {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    clear_syllable_highlight();
                }
                None => web_syllable_fallback(&owned, rate),
            }
            // Keep the boundary callback alive for the whole utterance, then let
            // it leak (bounded: one per manual replay tap), matching the app's
            // other forget()-ed one-shot DOM closures.
            on_index.forget();
        });
    } else {
        web_syllable_fallback(sylls, rate);
    }
}

/// No native boundary callbacks (web/PWA): speak the whole word slowly through
/// the browser voice (syllables joined by spaces so it separates them audibly)
/// and drive the reveal highlight from char-count-weighted time estimates.
fn web_syllable_fallback(sylls: &[String], _rate: f64) {
    speech_out::speak(&sylls.join(" "), 0.55, "es-ES");
    let Some(win) = web_sys::window() else { return };
    let mut acc = 0.0f64;
    for (i, s) in sylls.iter().enumerate() {
        let idx = i;
        let cb = Closure::<dyn FnMut()>::new(move || highlight_syllable(idx));
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), acc as i32);
        cb.forget();
        // Slow-rate estimate: a base per syllable plus per-character time.
        acc += 420.0 + 150.0 * s.chars().count() as f64;
    }
    let done = Closure::<dyn FnMut()>::new(clear_syllable_highlight);
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(done.as_ref().unchecked_ref(), acc as i32);
    done.forget();
}

/// Highlight syllable `index` in the revealed spelling, clearing the others.
fn highlight_syllable(index: usize) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Ok(nodes) = doc.query_selector_all(".reveal.syllabified .syl") else { return };
    for k in 0..nodes.length() {
        if let Some(el) = nodes.item(k).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            if k as usize == index {
                let _ = el.class_list().add_1("syl-on");
            } else {
                let _ = el.class_list().remove_1("syl-on");
            }
        }
    }
}

/// Remove the current-syllable highlight from every span.
fn clear_syllable_highlight() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Ok(nodes) = doc.query_selector_all(".reveal.syllabified .syl.syl-on") else { return };
    for k in 0..nodes.length() {
        if let Some(el) = nodes.item(k).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            let _ = el.class_list().remove_1("syl-on");
        }
    }
}

// ---------- CC-ATTEMPTS-SHIELDS wiring (flag-gated; OFF => zero diff) ----------
//
// Both features hook the SINGLE validation path (submit_guess -> on_correct /
// on_wrong). They read ONLY the validated-correct / validated-incorrect events —
// never the language, input method, or raw text. Timeouts and give-ups are NOT
// attempts, so they keep the legacy path untouched.

/// Shields apply in a solo, ranked Climb run (the adaptive `climb` level) while
/// the dark flag is on. Not in Daily / Versus / Misses-review.
fn shield_ctx(s: &AppState) -> bool {
    flags::attempts_shields() && s.level == "climb" && !s.versus.enabled && !s.daily.active && !s.review
}

/// The "Extra attempt on misses" retry applies in normal (fixed-level) solo
/// practice. In Kid Mode it is ON BY DEFAULT (Eric, 2026-07-15): little spellers
/// keep a one-retry safety net without needing the toggle, since retiring the
/// 3-try mechanic would otherwise make one-shot too harsh for the youngest
/// users. Normal (non-kid) users still opt in via the toggle.
fn extra_attempt_ctx(s: &AppState) -> bool {
    flags::attempts_shields()
        && (s.extra_attempts || s.kid)
        && s.level != "climb"
        && !s.versus.enabled
        && !s.daily.active
        && !s.review
}

/// Record the FIRST-submission miss exactly once: accuracy, spaced-rep/Misses,
/// and adaptive word stats. The retry outcome never re-records (I2 / A2 / A9).
fn record_first_miss(app: &App) {
    let (cur_lang, cur_tier, word, review) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.cur_tier.clone(), s.word.clone(), s.review)
    };
    stats::record(&mut app.borrow_mut(), &cur_lang, &cur_tier, false);
    misses::add_miss(&mut app.borrow_mut(), &word, &cur_lang, &cur_tier);
    refresh_mode_buttons(app);
    if !review {
        wordstats::record(&cur_lang, &word, false);
        selection::note_outcome(&cur_lang, &word, false);
    }
}

/// Re-present the SAME word for its one retry: clear the input (never
/// pre-filled), replay the audio exactly once, re-enable controls, restart the
/// timer. Shared by the extra-attempt and shield-accept flows.
fn grant_retry(app: &App, feedback_key: &str) {
    app.borrow_mut().answered = false;
    crate::attempts::prepare_retry(&mut app.borrow_mut()); // input starts empty (A5)
    dom::set_disabled("checkBtn", false);
    dom::set_disabled("hintBtn", false);
    dom::set_disabled("giveupBtn", false);
    let is_en = app.borrow().cur_lang == EN;
    dom::set_disabled("defBtn", !is_en);
    dom::set_disabled("sentenceBtn", !is_en);
    render_letters(app, false);
    dom::add_class("orbWrap", "bad");
    spell_feedback(false);
    set_streak_tier(0);
    dom::set_text("orbGlyph", "\u{2717}");
    dom::set_text("feedback", &crate::i18n::t(feedback_key));
    dom::el("feedback").set_class_name("feedback bad");
    sync_keyboard(app);
    schedule(app, 550, |_| {
        dom::remove_class("orbWrap", "bad");
        dom::set_html("orbGlyph", &crate::i18n::t("orb.listen"));
    });
    speak_current(app); // audio replays automatically, exactly once (A5)
    let cur_tier = app.borrow().cur_tier.clone();
    start_timer(app, &cur_tier);
}

/// Reset the correct-streak to zero because a real miss happened (the run may
/// still continue via a retry, but the chain does NOT keep growing — PD4).
fn break_streak(app: &App) {
    app.borrow_mut().streak = 0;
    dom::set_text("streakNum", "0");
    set_streak_tier(0);
}

/// Feature 1 (normal mode) — a validated-incorrect under the extra-attempts
/// toggle. First miss: record once + one clean retry. Second miss on the same
/// word: normal consequence, nothing re-recorded.
fn on_wrong_extra_attempt(app: &App) {
    crate::haptics::incorrect(app.borrow().kid);
    let already = app.borrow().aids.retry_used;
    if !already {
        record_first_miss(app);
        app.borrow_mut().aids.retry_used = true;
        break_streak(app); // the miss is real; the chain stops growing (PD4)
        grant_retry(app, "attempts.tryAgain");
        return;
    }
    // Retry was also wrong -> normal miss consequence; miss already recorded.
    finalize_incorrect_ex(app, "\u{2717}", &crate::i18n::t("fb.itWas"), "feedback bad", false);
}

/// Feature 2 (The Climb) — a validated-incorrect during a run. Any miss resets
/// the shield earn streak. If a shield is available and none has been spent on
/// this word, record the first miss and offer the "Use a shield?" prompt;
/// otherwise the normal miss consequence applies.
fn on_wrong_climb(app: &App) {
    crate::haptics::incorrect(app.borrow().kid);
    crate::attempts::shield_note_miss(&mut app.borrow_mut()); // earn streak resets (PD2)
    if crate::attempts::shield_available(&app.borrow()) {
        record_first_miss(app); // first miss recorded regardless of the choice
        break_streak(app);
        show_shield_prompt(app);
        return;
    }
    // No shield to spend (or already used one on this word) -> normal miss.
    finalize_incorrect(app, "\u{2717}", &crate::i18n::t("fb.itWas"), "feedback bad");
}

/// Show the localized "Use a shield?" prompt (player choice, not auto-spend).
fn show_shield_prompt(app: &App) {
    let n = app.borrow().aids.shields;
    dom::set_text("shieldPromptCount", &n.to_string());
    dom::add_class("shieldScrim", "show");
}

fn hide_shield_prompt() {
    dom::remove_class("shieldScrim", "show");
}

/// Player ACCEPTED: consume exactly one shield and retry the same word.
pub fn shield_accept(app: &App) {
    hide_shield_prompt();
    if !crate::attempts::spend_shield(&mut app.borrow_mut()) {
        return; // defensive: nothing to spend
    }
    update_shield_hud(app);
    grant_retry(app, "shield.used");
}

/// Player DECLINED: the run's normal miss consequence applies; the shield is
/// kept. The first miss was already recorded when the prompt appeared.
pub fn shield_decline(app: &App) {
    hide_shield_prompt();
    finalize_incorrect_ex(app, "\u{2717}", &crate::i18n::t("fb.itWas"), "feedback bad", false);
}

/// Sync the shield-count HUD. Visible only while the flag is on and this is a
/// Climb run; hidden everywhere else (byte-for-byte zero diff when OFF).
pub fn update_shield_hud(app: &App) {
    let (show, n) = {
        let s = app.borrow();
        (shield_ctx(&s), s.aids.shields)
    };
    dom::set_text("shieldCount", &n.to_string());
    dom::toggle_class("shieldHud", "btn-hide", !show);
}

/// Brief localized HUD flash when a shield is earned.
fn flash_shield_earned(app: &App) {
    dom::set_text("feedback", &crate::i18n::t("shield.earned"));
    dom::el("feedback").set_class_name("feedback good");
    update_shield_hud(app);
}

/// Korean (Phase 3): on a wrong answer, point the player at the jamo that's off
/// (initial / medial / 받침); in Kid Mode also spell the target out in jamo.
/// Rendered in the hint line; no-op for other languages. Cleared with the hint
/// line on the next word. (build-54: the old "after 2 misses" gate is gone with
/// the 3-try mechanic — there is only one submission now, so the Kid spelled-out
/// jamo hint accompanies that single miss's reveal.)
fn korean_coaching(app: &App) {
    let (lang, typed, word, kid) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.answer.clone(), s.word.clone(), s.kid)
    };
    if lang != crate::consts::KO {
        return;
    }
    let g = crate::jamo::grade(&typed, &word);
    let mut msg = g
        .syllables
        .iter()
        .find(|s| !s.wrong.is_empty())
        .map(|s| match s.wrong[0] {
            crate::jamo::Part::Initial => crate::i18n::t("jamo.checkInitial"),
            crate::jamo::Part::Medial => crate::i18n::t("jamo.checkMedial"),
            crate::jamo::Part::Final => crate::i18n::t("jamo.checkFinal"),
        })
        .unwrap_or_default();
    if kid {
        let hint = crate::i18n::tp("jamo.spelledOut", &[("j", &crate::jamo::spell_jamo(&word))]);
        msg = if msg.is_empty() { hint } else { format!("{msg} \u{b7} {hint}") };
    }
    if !msg.is_empty() {
        dom::set_text("hintLine", &msg);
    }
}

fn on_wrong(app: &App) {
    if app.borrow().daily.active {
        daily_answer(app, false);
        return;
    }
    // CC-ATTEMPTS-SHIELDS (flag-gated): shields in The Climb, or the extra-
    // attempts toggle in normal mode. Either fully handles the miss and returns.
    if shield_ctx(&app.borrow()) {
        on_wrong_climb(app);
        return;
    }
    if extra_attempt_ctx(&app.borrow()) {
        on_wrong_extra_attempt(app);
        return;
    }
    // build-54: the legacy 3-try mechanic is RETIRED. The base game is now one
    // attempt per word — a validated-incorrect is immediately the miss. The only
    // retries are shields (The Climb) and the extra-attempts toggle, both handled
    // above. The Korean per-jamo hint still shows once alongside the reveal.
    crate::haptics::incorrect(app.borrow().kid);
    korean_coaching(app);
    finalize_incorrect(app, "\u{2717}", &crate::i18n::t("fb.itWas"), "feedback bad");
}

fn on_timeout(app: &App) {
    let answered = app.borrow().answered;
    if answered {
        return;
    }
    app.borrow_mut().answered = true;
    lock_inputs();

    // A timeout is NOT a submitted attempt, so it never earns/spends a shield or
    // grants an extra attempt (the contract: only validated-correct/incorrect
    // count). With the 3-try mechanic retired, a timeout is simply the miss in
    // every mode. When shields/extra-attempts own the run, it still resets the
    // shield earn streak (the miss was real).
    if shield_ctx(&app.borrow()) || extra_attempt_ctx(&app.borrow()) {
        crate::attempts::shield_note_miss(&mut app.borrow_mut());
    }
    finalize_incorrect(app, "\u{23f1}", &crate::i18n::t("fb.timesUp"), "feedback bad");
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

    // build-54: no tries to zero out — a give-up is the single miss.
    render_tries(app);
    finalize_incorrect(app, "\u{2013}", &crate::i18n::t("fb.wordWas"), "feedback");
}

fn end_chain(app: &App) {
    let (reached, level, run_start, kid, review) = {
        let s = app.borrow();
        (s.streak, s.level.clone(), s.run_start_ms, s.kid, s.review)
    };
    // Post to The Climb: ranked difficulty (submit_run filters to medium/hard/
    // expert) and logged-in only; never in Kid Mode. A plausible run duration
    // feeds the server-side anti-cheat.
    if reached > 0 && !kid {
        let duration = (now_ms() - run_start).max(0.0);
        crate::climb::submit_run(&level, reached, duration);
    }
    // Ghost racing (F6): a solo Climb run just ended. Record the terminating
    // miss, then keep the run if it's a new best; celebrate beating a prior
    // ghost. Local-only; independent of The Climb leaderboard above.
    if level == "climb" && !review {
        crate::ghost::note_incorrect((now_ms() - run_start).max(0.0));
        if let crate::ghost::Outcome::Beat = crate::ghost::finish_run() {
            dom::show_toast(&crate::i18n::t("ghost.beat"));
        }
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
    note_climb(app, false); // Option A: drop the Climb band one step (not to easy)
    crate::ghost::hide_pace(); // F6: no live ghost between runs
    app.borrow_mut().streak = 0;
    dom::set_text("streakNum", "0");
    // CC-ATTEMPTS-SHIELDS: the run just ended — shields are per-run and do NOT
    // carry into the next one (I7). Reset all run aids.
    crate::attempts::reset_run(&mut app.borrow_mut());
    update_shield_hud(app);
    let review = app.borrow().review;
    dom::set_html("orbGlyph", &if review { crate::i18n::t("orb.continue") } else { crate::i18n::t("orb.next") });
}

/// The hint's masked word and its letter count. Pure, so it can be tested without
/// a DOM — the counting rule is the part with the bugs in it.
///
/// Counts and masks by GRAPHEME CLUSTER, not by `char`. `chars()` counts Unicode
/// codepoints, but "how many letters is this word" is a question about what a
/// reader sees, not about how the text is encoded. The two agree for English and
/// diverge everywhere else:
///   * Arabic كِتَاب — each harakat is its own codepoint sitting ON a letter, so
///     `chars()` over-counts and the mask would emit a bullet for a vowel mark.
///   * Devanagari क्ष is one akshara built from three codepoints (see the akshara
///     work on feature/hindi-akshara).
///   * An imported list can hold decomposed "café" (e + U+0301) or an emoji ZWJ
///     sequence — the only case LIVE today, since English is the one Active
///     language and My Words accepts arbitrary text.
///
/// For English this is byte-for-byte what it did before: no ASCII letter is ever
/// more than one cluster.
fn mask_word(word: &str) -> (String, usize) {
    let clusters: Vec<&str> = word.graphemes(true).collect();
    let masked = clusters
        .iter()
        .enumerate()
        .map(|(i, g)| if i == 0 || *g == " " || *g == "'" || *g == "-" { *g } else { "\u{2022}" })
        .collect();
    (masked, clusters.len())
}

pub fn show_hint(app: &App) {
    let s = app.borrow();
    if s.word.is_empty() {
        return;
    }
    let w = s.word.clone();
    drop(s);
    let (masked, n) = mask_word(&w);
    // The count is now translated (was hardcoded English, shown in English on
    // every non-English UI). Singular/plural split so English reads "1 letter",
    // not "1 letters": the codebase keeps English grammatical for counts even
    // where it lets other locales use one form (cf. daily.streakDays). n==1 is
    // only reachable via a one-letter My Words entry — no built-in word is a
    // single letter — but the old string already mis-rendered it, so this only
    // improves it.
    let key = if n == 1 { "hint.letterOne" } else { "hint.letters" };
    let count = crate::i18n::tp(key, &[("n", &n.to_string())]);
    // U+2068 FSI / U+2069 PDI isolate the translated count from the word. The word
    // can be RTL while the count is Latin (or vice-versa), and without an isolate
    // the bidi algorithm resolves them as ONE run: the parentheses mirror and the
    // count lands on the wrong side of the word. FSI (not LRI) so the isolate
    // takes direction from the count string's OWN first strong character, which is
    // what makes it correct once the count itself is in an RTL script.
    dom::set_text("hintLine", &format!("{}   \u{2068}{}\u{2069}", masked, count));
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

// ---------- Daily Challenge (see crate::daily) ----------

/// Start today's Daily Challenge: a fixed, date+language-seeded set played once
/// through, one attempt per word, isolated from streak/stats/Misses/Climb.
pub fn enter_daily(app: &App) {
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    let date = crate::daily::today();
    let (locale, words) = crate::daily::build_words(&lang, &date, kid);
    if words.is_empty() {
        return;
    }
    {
        let mut s = app.borrow_mut();
        s.review = false;
        s.daily.active = true;
        s.daily.spelloff = false; // an ordinary Daily run, not an online Spell Off
        s.daily.locale = locale;
        s.daily.date = date;
        s.daily.words = words;
        s.daily.idx = 0;
        s.daily.correct = 0;
        s.word = String::new();
        s.answered = false;
    }
    stop_timer(true);
    crate::ghost::hide_pace(); // F6: leaving Climb for the Daily Challenge
    clear_meaning();
    dom::set_disabled("langSel", true);
    dom::set_disabled("levelSel", true);
    dom::set_disabled("modeSel", true);
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    dom::set_text("hintLine", "");
    dom::set_text("feedback", "");
    dom::el("feedback").set_class_name("feedback");
    dom::set_html("orbGlyph", &crate::i18n::t("daily.tapStart"));
    dom::remove_class("dailyBar", "btn-hide");
    render_tries(app);
    render_daily_bar(app);
    refresh_daily_btn(app);
}

/// Leave the Daily Challenge mid-run (progress is discarded — it isn't recorded
/// until the run is finished).
pub fn exit_daily(app: &App) {
    {
        let mut s = app.borrow_mut();
        s.daily.active = false;
        s.daily.spelloff = false;
        s.word = String::new();
        s.answered = false;
        s.cur_lang = s.lang.clone();
    }
    leave_daily_ui(app);
}

/// Start an online Spell Off run over a server-seeded word list (§online). It
/// reuses the Daily Challenge run machinery (fixed list, one attempt per word,
/// cumulative-correct scoring) but is flagged `spelloff` so `finish_daily`
/// submits the result to the match server instead of the Daily streak. The
/// caller (online_spelloff.rs) owns the words (derived from the shared seed) and
/// the match code / timing.
pub fn start_spelloff_run(app: &App, locale: String, words: Vec<String>) {
    if words.is_empty() {
        return;
    }
    if app.borrow().review {
        exit_review(app, None);
    }
    if app.borrow().versus.enabled {
        exit_versus(app);
    }
    {
        let mut s = app.borrow_mut();
        s.review = false;
        s.daily.active = true;
        s.daily.spelloff = true;
        s.daily.locale = locale;
        s.daily.date = crate::daily::today();
        s.daily.words = words;
        s.daily.idx = 0;
        s.daily.correct = 0;
        s.word = String::new();
        s.answered = false;
    }
    stop_timer(true);
    clear_meaning();
    dom::set_disabled("langSel", true);
    dom::set_disabled("levelSel", true);
    dom::set_disabled("modeSel", true);
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    dom::set_text("hintLine", "");
    dom::set_text("feedback", "");
    dom::el("feedback").set_class_name("feedback");
    dom::set_html("orbGlyph", &crate::i18n::t("daily.tapStart"));
    dom::remove_class("dailyBar", "btn-hide");
    render_tries(app);
    render_daily_bar(app);
}

fn finish_daily(app: &App) {
    let (date, correct, total, spelloff) = {
        let s = app.borrow();
        (s.daily.date.clone(), s.daily.correct, s.daily.words.len() as u32, s.daily.spelloff)
    };
    // Online Spell Off: same fixed-list run, but the result goes to the match
    // server (and shows the head-to-head outcome) instead of the Daily streak.
    if spelloff {
        {
            let mut s = app.borrow_mut();
            s.daily.active = false;
            s.daily.spelloff = false;
            s.word = String::new();
            s.answered = false;
            s.cur_lang = s.lang.clone();
        }
        leave_daily_ui(app);
        crate::online_spelloff::finish_run(app, correct, total);
        return;
    }
    let (streak, best) = crate::daily::record_result(&date, correct);
    {
        let mut s = app.borrow_mut();
        s.daily.active = false;
        s.word = String::new();
        s.answered = false;
        s.cur_lang = s.lang.clone();
    }
    leave_daily_ui(app);
    show_daily_result(app, correct, total, streak, best);
}

/// Shared teardown when leaving daily mode (finish or abandon).
fn leave_daily_ui(app: &App) {
    stop_timer(true);
    clear_meaning();
    dom::set_disabled("langSel", false);
    dom::set_disabled("levelSel", false);
    dom::set_disabled("modeSel", false);
    dom::add_class("dailyBar", "btn-hide");
    dom::set_html("orbGlyph", &crate::i18n::t("orb.tap"));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    dom::set_text("feedback", "");
    dom::el("feedback").set_class_name("feedback");
    dom::set_text("hintLine", "");
    render_tries(app);
    update_voice_note(app);
    refresh_daily_btn(app);
}

/// One daily answer — count it, reveal on a miss, show the meaning; the player
/// taps the orb to advance (which runs `next_word` → serves the next word or
/// finishes). Single attempt: no retries in the Daily Challenge.
fn daily_answer(app: &App, correct: bool) {
    let (cur_lang, word, kid) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.word.clone(), s.kid)
    };
    if correct {
        crate::haptics::correct();
        app.borrow_mut().daily.correct += 1;
        dom::add_class("orbWrap", "good");
        spell_feedback(true);
        dom::set_text("orbGlyph", "\u{2713}");
        dom::set_text("feedback", &crate::i18n::t(&format!("praise.{}", rand_index(8) + 1)));
        dom::el("feedback").set_class_name("feedback good");
    } else {
        crate::haptics::incorrect(kid);
        dom::add_class("orbWrap", "bad");
        spell_feedback(false);
        dom::set_text("orbGlyph", "\u{2717}");
        dom::set_html("feedback", &format!("{}<span class=\"reveal\">{}</span>", crate::i18n::t("fb.itWas"), dom::escape_html(&word)));
        dom::el("feedback").set_class_name("feedback bad");
    }
    // Streak warmth in Daily rides the run's cumulative correct count.
    set_streak_tier(app.borrow().daily.correct);
    show_meaning(app, word, cur_lang);
    render_daily_bar(app);
}

/// Auto-advance after a correct Daily answer, fired CORRECT_DELAY_MS later by
/// `on_correct`. Guarded so it advances at most once per word (I6): if the
/// player tapped the orb to skip — which runs `next_word` and clears `answered`
/// — or left Daily, this timer becomes a no-op. Also holds while an IME
/// composition is open. On the final word, `next_word` routes to the results
/// screen, so the last word gets the same delay-then-advance as any other
/// (final word not special-cased). Backgrounding only delays the timeout; the
/// guard still guarantees a single advance. The last word (score-recording via
/// finish_daily) is READ-ONLY here (I5) — this only chooses *when* to advance.
fn daily_auto_advance(app: &App, at_idx: usize) {
    let go = {
        let s = app.borrow();
        // `daily.idx == at_idx` ensures we only advance the word this timer was
        // scheduled for — a stale timer from an already-skipped word is dropped.
        s.daily.active && s.answered && !s.composing && s.daily.idx == at_idx
    };
    if go {
        next_word(app);
    }
}

fn render_daily_bar(app: &App) {
    let s = app.borrow();
    let n = s.daily.words.len();
    let i = s.daily.idx.min(n);
    let progress = crate::i18n::tp("daily.progress", &[("i", &i.to_string()), ("n", &n.to_string()), ("c", &s.daily.correct.to_string())]);
    // Expert finale: the last two words (idx 8-9 of the 10-word arc) get a "peak"
    // badge — the visual beat that the hard part has begun.
    let is_finale = !s.kid && s.daily.idx >= crate::daily::FINALE_START && s.daily.idx < n;
    dom::toggle_class("dailyBar", "finale", is_finale);
    if is_finale {
        dom::set_html("dailyBar", &format!("{}<span class=\"daily-finale\">{}</span>", progress, crate::i18n::t("daily.finale")));
    } else {
        dom::set_html("dailyBar", &progress);
    }
}

fn show_daily_result(app: &App, correct: u32, total: u32, streak: u32, best: u32) {
    dom::set_text("dailyResScore", &format!("{correct}/{total}"));
    dom::set_text("dailyResStreak", &crate::i18n::tp("daily.streakDays", &[("n", &streak.to_string())]));
    dom::set_text("dailyResBest", &crate::i18n::tp("daily.bestStreak", &[("n", &best.to_string())]));
    // Occasionally (~1 in 3, seeded by date+language so it's stable per day and
    // non-repeating) surface a proverb in the session's language beneath the
    // result — a language-honoring beat, not a new screen. Else stays blank.
    let lang = app.borrow().lang.clone();
    let date = crate::daily::today();
    let seed = date.bytes().chain(lang.bytes()).fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    match crate::enrich::proverb(&lang, seed) {
        Some(pv) => dom::set_html("dailyResProverb", &format!("\u{201c}{}\u{201d}<span class=\"dp-tr\">{}</span>", dom::escape_html(&pv.o), dom::escape_html(&pv.t))),
        None => dom::set_text("dailyResProverb", ""),
    }
    dom::add_class("dailyResScrim", "show");
}

/// Re-show today's result (when the Daily button is tapped after it's done).
pub fn show_today_result(app: &App) {
    let r = crate::daily::load();
    let today = crate::daily::today();
    let correct = r.history.get(&today).copied().unwrap_or(0);
    let (lang, kid) = {
        let s = app.borrow();
        (s.lang.clone(), s.kid)
    };
    let (_, words) = crate::daily::build_words(&lang, &today, kid);
    show_daily_result(app, correct, words.len() as u32, r.streak, r.best_streak);
}

/// Update the Daily button label/state (Rust-managed, like the Misses button).
pub fn refresh_daily_btn(app: &App) {
    let active = app.borrow().daily.active;
    let done = crate::daily::is_done_today();
    let label = if active {
        crate::i18n::t("daily.exit")
    } else if done {
        format!("\u{1F5D3} {} \u{2713}", crate::i18n::t("daily.title"))
    } else {
        format!("\u{1F5D3} {}", crate::i18n::t("daily.title"))
    };
    dom::set_text("dailyBtn", &label);
    dom::toggle_class("dailyBtn", "on", active);
    dom::toggle_class("dailyBtn", "done", done && !active);
}

pub fn start_timer(app: &App, tier: &str) {
    stop_timer(true);
    let s = app.borrow();
    let timed = s.timed && !s.daily.active; // the Daily Challenge is always untimed
    drop(s);
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
        dom::set_text("feedback", &crate::i18n::t("fb.noMisses"));
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    let due = misses::due_misses(&app.borrow()).len();
    if due == 0 {
        dom::set_text("feedback", &if total == 1 { crate::i18n::t("fb.allCaughtOne") } else { crate::i18n::tp("fb.allCaughtMany", &[("n", &total.to_string())]) });
        dom::el("feedback").set_class_name("feedback");
        return;
    }
    app.borrow_mut().review = true;
    stop_timer(true);
    crate::ghost::hide_pace(); // F6: leaving Climb for Misses review
    clear_meaning();
    dom::set_disabled("langSel", true);
    dom::set_disabled("levelSel", true);
    {
        let mut s = app.borrow_mut();
        s.word = String::new();
        s.answered = false;
    }
    dom::set_html("orbGlyph", &crate::i18n::t("orb.practiceMisses"));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
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
    dom::set_html("orbGlyph", &crate::i18n::t("orb.tap"));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
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
    crate::attempts::reset_run(&mut app.borrow_mut());
    update_shield_hud(app);
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    dom::set_html("orbGlyph", &crate::i18n::t("orb.tap"));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
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
    }
    dom::remove_class("orbWrap", "good");
    dom::remove_class("orbWrap", "bad");
    let name = app.borrow().versus.active_player().name.clone();
    dom::set_html("orbGlyph", &format!("{}<br/>tap for a word", dom::escape_html(&name)));
    app.borrow_mut().answer.clear();
    render_letters(app, false);
    sync_keyboard(app);
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
    spell_feedback(true);
    // Head-to-head warmth rides the active player's current chain.
    set_streak_tier(app.borrow().versus.active_player().current);
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

#[cfg(test)]
mod climb_band_tests {
    //! Option A — the solo Climb band promotes after CLIMB_PROMOTE correct
    //! answers and drops exactly ONE tier on a miss (never straight to easy), so
    //! hard languages keep progressing. The streak/chain is not involved here.

    // ---- CC-RTL F4: the two render paths ----

    /// A cursive answer must reach the shaper as ONE run. If this ever splits,
    /// Arabic shatters into isolated letterforms (+26.6% median width — see
    /// spike/rtl-feedback/FINDINGS.md). Counting elements is the cheap proxy for
    /// "did we hand the shaper a single run".
    #[test]
    fn joined_path_emits_one_run_not_one_span_per_letter() {
        let html = super::render_letters_joined("\u{643}\u{62a}\u{627}\u{628}"); // كتاب
        assert_eq!(html.matches("<span").count(), 2, "one span for the word + one caret; never one per letter: {html}");
        assert!(html.contains("dir=\"rtl\""), "the run carries its own direction");
        assert!(html.contains("\u{643}\u{62a}\u{627}\u{628}"), "the word survives intact");
        assert!(!html.contains("pop"), "no per-letter pop is possible on an unsplit run");
    }

    /// One text node is not an excuse to stop escaping — the answer is user input.
    #[test]
    fn joined_path_still_escapes() {
        let html = super::render_letters_joined("<img onerror=x>");
        assert!(!html.contains("<img"), "user input must be escaped: {html}");
        assert!(html.contains("&lt;img"));
    }

    /// The syllabified reveal splits a word into one span PER SYLLABLE — exactly
    /// the shattering F4 removed from #letters. A cursive word would break its
    /// joins at every syllable boundary.
    ///
    /// It is safe purely because the feature is gated to `es`, and Spanish does
    /// not join. That is a coincidence of the current gate, not a property of the
    /// code, so it is pinned here: extending syllable replay to a cursive language
    /// must fail this test and force the author to reach for the joined path.
    #[test]
    fn syllable_replay_cannot_reach_a_cursive_script() {
        assert!(
            !crate::consts::script_joins(crate::consts::ES),
            "syllable replay is gated to `es` and emits per-syllable spans; if Spanish ever \
             counted as cursive, that reveal would shatter it"
        );
        // The guard is only meaningful if a cursive language exists to guard
        // against — otherwise this passes vacuously forever.
        assert!(
            crate::consts::BUILTIN_LANGS.iter().any(|(c, _, _, _)| crate::consts::script_joins(c)),
            "no cursive language in the registry — this test would pass vacuously"
        );
    }

    // ---- CC-RTL F4 remainder: the hint is a word surface too ----

    /// The whole point of the grapheme rewrite: "how many letters" is a question
    /// about what a READER sees, and `chars()` answers a question about encoding.
    /// Each case below is one where the two disagree.
    #[test]
    fn hint_counts_letters_not_codepoints() {
        // Decomposed "café" — e + U+0301 is ONE letter wearing an accent.
        // Reachable TODAY: My Words takes arbitrary imported text.
        let (_, n) = super::mask_word("cafe\u{301}");
        assert_eq!(n, 4, "decomposed café is 4 letters, not 5 codepoints");

        // Arabic كِتَاب — the harakat are marks ON letters, not letters.
        let (_, n) = super::mask_word("\u{643}\u{650}\u{62a}\u{64e}\u{627}\u{628}");
        assert_eq!(n, 4, "kitāb is 4 letters; its vowel marks are not letters");

        // Devanagari क्ष — one akshara, three codepoints.
        let (_, n) = super::mask_word("\u{915}\u{94d}\u{937}");
        assert_eq!(n, 1, "क्ष is one akshara");

        // A ZWJ emoji sequence is one glyph, not five.
        let (_, n) = super::mask_word("\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}");
        assert_eq!(n, 1, "a ZWJ family emoji is one cluster");
    }

    /// The mask must not tear a cluster apart: replacing a base letter but leaving
    /// its combining mark behind would orphan the mark onto the bullet — the mark
    /// would render ON the •. This is the same shattering bug as F4's, one surface
    /// over: it is about clusters rather than joins, but the cause is identical
    /// (treating a codepoint as if it were a letter).
    #[test]
    fn hint_masks_whole_clusters_never_orphaning_a_combining_mark() {
        let (masked, _) = super::mask_word("cafe\u{301}");
        assert_eq!(masked, "c\u{2022}\u{2022}\u{2022}", "one bullet per letter, and no stray U+0301");
        assert!(!masked.contains('\u{301}'), "a combining acute must not survive onto a bullet: {masked:?}");

        // كِتَاب masks to "كِ•••": the FIRST cluster is shown in full and it is
        // كِ — kaf *wearing* its kasra. That mark surviving is correct; it belongs
        // to a letter the player is being shown. The bug would be a mark landing
        // on a BULLET, where its base letter is gone.
        let (masked, n) = super::mask_word("\u{643}\u{650}\u{62a}\u{64e}\u{627}\u{628}");
        assert_eq!(n, 4);
        assert_eq!(masked, "\u{643}\u{650}\u{2022}\u{2022}\u{2022}", "first letter whole (with its kasra), then one bullet per remaining letter");
        assert!(!masked.contains("\u{2022}\u{64e}"), "the fatha's base was masked, so the fatha must go with it: {masked:?}");
        // The general rule, script-agnostic: nothing combining may follow a bullet.
        let orphan = masked
            .chars()
            .zip(masked.chars().skip(1))
            .any(|(a, b)| a == '\u{2022}' && matches!(b, '\u{300}'..='\u{36f}' | '\u{64b}'..='\u{65f}' | '\u{900}'..='\u{903}' | '\u{93a}'..='\u{94f}'));
        assert!(!orphan, "a combining mark orphaned onto a bullet: {masked:?}");
    }

    /// English is the only Active language and 100% of today's players: this
    /// rewrite must be invisible to them.
    #[test]
    fn hint_is_unchanged_for_english() {
        for (word, want, n) in [
            ("cat", "c\u{2022}\u{2022}", 3),
            ("ice cream", "i\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}", 9),
            ("don't", "d\u{2022}\u{2022}'\u{2022}", 5),
            ("well-known", "w\u{2022}\u{2022}\u{2022}-\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}", 10),
        ] {
            let (masked, got) = super::mask_word(word);
            assert_eq!(masked, want, "{word}");
            assert_eq!(got, n, "{word}");
        }
        // The old chars()-based rule, restated: for ASCII the two must agree
        // exactly, or this refactor changed what players see.
        for word in ["cat", "ice cream", "don't", "well-known", "rhythm"] {
            assert_eq!(super::mask_word(word).1, word.chars().count(), "{word}: grapheme and char counts must agree for ASCII");
        }
    }

    use super::*;
    use crate::model::AppState;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn app() -> App {
        Rc::new(RefCell::new(AppState::default())) // default level == "climb", solo
    }

    #[test]
    fn promotes_after_three_and_drops_one_on_miss() {
        let a = app();
        assert_eq!(band_to_tier(a.borrow().climb_band), "easy");
        for _ in 0..3 {
            note_climb(&a, true);
        }
        assert_eq!(band_to_tier(a.borrow().climb_band), "medium"); // 3 correct → +1
        for _ in 0..3 {
            note_climb(&a, true);
        }
        assert_eq!(band_to_tier(a.borrow().climb_band), "hard");
        note_climb(&a, false); // one miss → down ONE, not to easy
        assert_eq!(band_to_tier(a.borrow().climb_band), "medium");
        note_climb(&a, false);
        note_climb(&a, false); // saturates at easy, never below
        assert_eq!(band_to_tier(a.borrow().climb_band), "easy");
    }

    #[test]
    fn untouched_on_a_fixed_level() {
        let a = app();
        a.borrow_mut().level = "expert".into();
        for _ in 0..5 {
            note_climb(&a, true);
        }
        note_climb(&a, false);
        assert_eq!(a.borrow().climb_band, 0); // fixed level: band never moves
    }
}
