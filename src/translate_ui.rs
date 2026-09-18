//! CC-TRANSLATE-SCREEN — the Translate screen's DOM. The rules live in
//! translate_screen.rs; this file renders them and routes taps, nothing more.
//!
//! App-only (D5, signed). It replaces the CC-TRANSLATE-TOOLS pop-up in place
//! (signed 2026-09-12), so there is one Translate surface; that pop-up's daily
//! word, Passport count and Home Pair live on below the action row, where
//! nothing they do can move it (I1).
//!
//! Every state is inline: suggestions and the language picker are overlays on
//! the spine, declines are text where the player is already looking, and there
//! is no modal, no alert and no toast.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use wasm_bindgen::JsCast;

use crate::experience::Experience;
use crate::translate_screen::{self as rules, Pair, SpellVerdict, Suggestion};
use crate::{dom, i18n, App};

thread_local! {
    /// The question on screen. Kept across closing the screen, so returning
    /// from Spell it restores the pair (F10).
    static PAIR: RefCell<Option<Pair>> = const { RefCell::new(None) };
    static SUGGESTIONS: RefCell<Vec<Suggestion>> = const { RefCell::new(Vec::new()) };
    /// Tool 3's toggle, per target language. Absent means D1's default.
    static TRANSLIT: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    static PICKER_FOR: Cell<Option<&'static str>> = const { Cell::new(None) };
    /// The word whose senses the player expanded past the first four (D9).
    static EXPANDED: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn t(k: &str) -> String {
    i18n::t(k)
}

fn experience(app: &App) -> Experience {
    crate::experience::of_kid(app.borrow().kid)
}

fn pair_of() -> Pair {
    PAIR.with(|p| p.borrow().clone()).unwrap_or_default()
}

fn set_pair(p: Pair) {
    PAIR.with(|x| *x.borrow_mut() = Some(p));
}

fn endonym(lang: &str) -> String {
    crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(c, _, _, _)| *c == lang)
        .map(|(_, n, _, _)| n.to_string())
        .unwrap_or_else(|| lang.to_string())
}

/// The "Speak in" code My Words stores for a word, from the same language table
/// the import screen offers. Falls back to the bare code.
fn speak_code(lang: &str) -> String {
    crate::words::LANGUAGES
        .iter()
        .find(|(k, _)| *k == lang)
        .map(|(_, l)| l.code.to_string())
        .unwrap_or_else(|| lang.to_string())
}

fn default_pair(app: &App) -> Pair {
    let study = app.borrow().lang.clone();
    let listable = rules::picker_languages((&study, ""), i18n::device_lang().as_deref());
    let source = listable.iter().copied().find(|l| *l == study).unwrap_or(crate::consts::EN);
    let target = listable.iter().copied().find(|l| *l != source).unwrap_or("");
    Pair::new(source, target)
}

fn hide_overlays() {
    let _ = dom::el("trSuggest").set_attribute("hidden", "");
    let _ = dom::el("trPicker").set_attribute("hidden", "");
    PICKER_FOR.with(|p| p.set(None));
}

fn set_audio(id: &str, state: &str) {
    let _ = dom::el(id).set_attribute("data-state", state);
}

pub fn open(app: &App) {
    if !dom::exists("trScreen") {
        return;
    }
    let listable = rules::picker_languages(("", ""), None);
    let pair = PAIR
        .with(|p| p.borrow().clone())
        .filter(|p| listable.iter().any(|l| *l == p.source) && listable.iter().any(|l| *l == p.target))
        .unwrap_or_else(|| default_pair(app));
    let shown = pair.source_entry.as_deref().map(crate::translate::display_word).unwrap_or_default();
    set_pair(pair);
    dom::input("trSrcInput").set_value(&shown);
    hide_overlays();
    cancel_save();
    dom::set_text("trNote", "");
    render(app);
    dom::add_class("trScreen", "show");
}

fn close() {
    hide_overlays();
    dom::remove_class("trScreen", "show");
}

fn set_card_lang(card: &str, word_el: &str, lang: &str) {
    let rtl = !lang.is_empty() && crate::consts::direction(lang) == crate::consts::Direction::Rtl;
    let _ = dom::el(card).set_attribute("dir", if rtl { "rtl" } else { "ltr" });
    let _ = dom::el(word_el).set_attribute("lang", lang);
}

fn render(app: &App) {
    let pair = pair_of();
    dom::set_text("trSrcLang", &endonym(&pair.source));
    dom::set_text(
        "trTgtLang",
        &if pair.target.is_empty() { t("tr.pickLang") } else { endonym(&pair.target) },
    );
    set_card_lang("trSrcCard", "trSrcInput", &pair.source);
    set_card_lang("trTgtCard", "trTgtWord", &pair.target);
    // F3: the sense the player asked for stays visible under their word --
    // unless it would only repeat that word (English is its own pivot).
    let src_shown = pair.source_entry.as_deref().map(crate::translate::display_word).unwrap_or_default();
    dom::set_text("trSrcSub", &rules::sense_line(&src_shown, pair.concept.as_deref()));
    render_target(&pair);
    let empty = pair.source_entry.is_none() && pair.target_entry.is_none();
    let _ = dom::el("trSwap").set_attribute("aria-disabled", if empty { "true" } else { "false" });
    for id in ["trSrcAudio", "trTgtAudio"] {
        let _ = dom::el(id).set_attribute("aria-label", &t("tr.hear"));
    }
    let _ = dom::el("trSwap").set_attribute("aria-label", &t("tr.swap"));
    render_extras(app);
}

fn translit_on(target: &str) -> bool {
    TRANSLIT
        .with(|m| m.borrow().get(target).copied())
        .unwrap_or_else(|| rules::translit_default_on(&i18n::current(), target))
}

fn render_target(pair: &Pair) {
    let word = pair.target_entry.as_deref().map(crate::translate::display_word).unwrap_or_default();
    dom::set_text("trTgtWord", &word);
    let translit = pair.target_entry.as_deref().and_then(crate::translate::transliteration).map(|(_, r)| r);
    let on = translit_on(&pair.target);
    let toggle = dom::el("trTranslitToggle");
    let _ = toggle.set_attribute("aria-pressed", if on { "true" } else { "false" });
    // The toggle keeps its place when a language ships no scheme, so the card
    // never reflows (I1).
    let _ = toggle.set_attribute("style", if translit.is_some() { "" } else { "visibility:hidden" });
    let declined = pair.concept.is_some() && pair.target_entry.is_none() && !pair.target.is_empty();
    if declined {
        // F12 #2 / I6: say so in the target card, with a live way forward.
        dom::set_html(
            "trTgtTranslit",
            &format!(
                "{} <button type=\"button\" class=\"ghost\" data-tr-pick=\"target\">{}</button>",
                dom::escape_html(&i18n::tp("tr.noAnswer", &[("lang", &endonym(&pair.target))])),
                dom::escape_html(&t("tr.pickLang")),
            ),
        );
    } else {
        // I4: transliteration is secondary and never the only form shown.
        dom::set_text("trTgtTranslit", &if on && !word.is_empty() { translit.unwrap_or_default() } else { String::new() });
    }
}

fn render_extras(app: &App) {
    let (lang, tier) = {
        let s = app.borrow();
        (s.lang.clone(), s.cur_tier.clone())
    };
    let mut html = String::new();
    let day = (js_sys::Date::now() / 86_400_000.0) as u32;
    if let Some(dw) = crate::translate::daily_word(&lang, &tier, day) {
        let stamps = crate::translate::concept_of(&lang, &dw)
            .as_deref()
            .map(crate::translate::passport_count)
            .unwrap_or(0);
        html.push_str(&format!(
            "<div class=\"rep-head\">{}</div>\
             <button type=\"button\" class=\"ghost tr-daily\" data-tr-daily=\"{}\">{}</button>\
             <span class=\"tr-passport\">{}</span>",
            dom::escape_html(&t("tr.daily")),
            dom::escape_html(&dw),
            dom::escape_html(&crate::translate::display_word(&dw)),
            dom::escape_html(&i18n::tp("tr.passport", &[("n", &stamps.to_string())])),
        ));
    }
    if let Some((a, b)) = crate::translate::home_pair() {
        html.push_str(&format!(
            "<div class=\"tr-pair\">{} {} \u{2194} {}</div>",
            dom::escape_html(&t("tr.homePair")),
            dom::escape_html(&a),
            dom::escape_html(&b),
        ));
    }
    dom::set_html("trExtras", &html);
}

fn on_input(app: &App) {
    let pair = pair_of();
    let q = dom::input("trSrcInput").value();
    let found = rules::search(experience(app), &pair.source, &pair.target, &q);
    SUGGESTIONS.with(|s| *s.borrow_mut() = found.clone());
    let _ = dom::el("trPicker").set_attribute("hidden", "");
    render_suggestions(&q, &found);
}

fn render_suggestions(q: &str, found: &[Suggestion]) {
    let panel = dom::el("trSuggest");
    if rules::normalize(q).is_empty() {
        let _ = panel.set_attribute("hidden", "");
        return;
    }
    let mut html = String::new();
    if found.is_empty() {
        // F12 #1 / I2 / I6: the bound is announced before anything is
        // committed, where the player is looking, with a live way forward.
        html.push_str(&format!(
            "<div class=\"tr-decline\">{}</div><button type=\"button\" data-tr-pick=\"source\">{}</button>",
            dom::escape_html(&t("tr.notInList")),
            dom::escape_html(&t("tr.pickLang")),
        ));
    } else {
        let expanded = EXPANDED.with(|e| e.borrow().clone());
        let mut done: Vec<&str> = Vec::new();
        for s in found {
            if done.contains(&s.entry.as_str()) {
                continue;
            }
            done.push(&s.entry);
            // F3 / D9: a word with several senses lists each by its gloss --
            // four inline, then "more meanings".
            let rows: Vec<(usize, &Suggestion)> =
                found.iter().enumerate().filter(|(_, x)| x.entry == s.entry).collect();
            let all: Vec<String> = rows.iter().map(|(_, x)| x.concept.clone()).collect();
            let (show, more) = rules::visible_senses(&all, expanded.as_deref() == Some(s.entry.as_str()));
            for (idx, x) in rows.iter().filter(|(_, x)| show.contains(&x.concept)) {
                html.push_str(&format!(
                    "<button type=\"button\" role=\"option\" data-tr-sugg=\"{idx}\">{} <span class=\"tr-dir\">{}</span></button>",
                    dom::escape_html(&x.shown),
                    dom::escape_html(&x.concept),
                ));
            }
            if more {
                html.push_str(&format!(
                    "<button type=\"button\" data-tr-more=\"{}\">{}</button>",
                    dom::escape_html(&s.entry),
                    dom::escape_html(&t("tr.moreSenses")),
                ));
            }
        }
    }
    dom::set_html("trSuggest", &html);
    let _ = panel.remove_attribute("hidden");
}

/// F2: the ONLY way a word is committed is picking a suggestion. Enter does
/// nothing, so there is no free-text submit and no post-commit miss (I2).
fn commit(app: &App, idx: usize) {
    let Some(s) = SUGGESTIONS.with(|v| v.borrow().get(idx).cloned()) else { return };
    set_pair(pair_of().commit(&s));
    dom::input("trSrcInput").set_value(&s.shown);
    EXPANDED.with(|e| *e.borrow_mut() = None);
    hide_overlays();
    dom::set_text("trNote", "");
    render(app);
}

fn open_picker(app: &App, which: &'static str) {
    let pair = pair_of();
    let langs = rules::picker_languages((&pair.source, &pair.target), i18n::device_lang().as_deref());
    let html: String = langs
        .iter()
        .map(|l| {
            format!(
                "<button type=\"button\" role=\"option\" data-tr-lang=\"{}\">{}</button>",
                dom::escape_html(l),
                dom::escape_html(&endonym(l)),
            )
        })
        .collect();
    dom::set_html("trPickerList", &html);
    PICKER_FOR.with(|p| p.set(Some(which)));
    let _ = dom::el("trSuggest").set_attribute("hidden", "");
    let _ = dom::el("trPicker").remove_attribute("hidden");
    let _ = app;
}

fn choose_lang(app: &App, lang: &str) {
    let exp = experience(app);
    let pair = pair_of();
    let next = match PICKER_FOR.with(|p| p.get()) {
        Some("source") => pair.with_source_lang(exp, lang),
        _ => pair.with_target_lang(exp, lang),
    };
    let shown = next.source_entry.as_deref().map(crate::translate::display_word).unwrap_or_default();
    set_pair(next);
    dom::input("trSrcInput").set_value(&shown);
    hide_overlays();
    render(app);
}

fn swap(app: &App) {
    let next = pair_of().swapped(experience(app));
    let shown = next.source_entry.as_deref().map(crate::translate::display_word).unwrap_or_default();
    set_pair(next);
    dom::input("trSrcInput").set_value(&shown);
    hide_overlays();
    dom::set_text("trNote", "");
    render(app);
}

fn clear(app: &App) {
    set_pair(pair_of().cleared());
    dom::input("trSrcInput").set_value("");
    EXPANDED.with(|e| *e.borrow_mut() = None);
    hide_overlays();
    dom::set_text("trNote", "");
    render(app);
    let _ = dom::input("trSrcInput").focus();
}

/// F6: one resolver, three states. The router reports failure, not success,
/// so a control still marked loading once it has had time to fail is playing.
/// D12: offline, the router fails and the control says audio is unavailable.
fn hear(which: &'static str) {
    let pair = pair_of();
    let (entry, lang, btn) = if which == "source" {
        (pair.source_entry, pair.source, "trSrcAudio")
    } else {
        (pair.target_entry, pair.target, "trTgtAudio")
    };
    let Some(entry) = entry else { return };
    set_audio(btn, "loading");
    // A "pinyin|hanzi" entry carries its own reading, which the audio server
    // requires; the reading comes from the entry's shape, not a language check.
    let py = entry.contains('|').then(|| crate::pinyin::phoneme_reading(&entry)).flatten();
    let failed = std::rc::Rc::new(Cell::new(false));
    let f = failed.clone();
    crate::api::play_word_with(&crate::translate::display_word(&entry), py.as_deref(), "normal", 1.0, &lang, move || {
        f.set(true);
        set_audio(btn, "unavailable");
        dom::set_text("trNote", &t("tr.audioUnavailable"));
    });
    crate::game::schedule_raw(1500, move || {
        if !failed.get() {
            set_audio(btn, "ready");
        }
    });
}

/// F9 / D10: save the TARGET word through the existing My Words write path,
/// tagged with its own "Speak in" language. No navigation: the player stays.
/// F9 / D5 (signed): Save opens the same destination control the other two
/// sheets use, already set to today's list, so it is a one-tap confirm with
/// Cancel always there. Nothing reaches My Words unless this is confirmed.
fn save(app: &App) {
    let pair = pair_of();
    if pair.target_entry.is_none() {
        return;
    }
    let _ = app;
    crate::lists_ui::populate("trDest", "trSaveGo");
    let _ = dom::el("trSaveRow").remove_attribute("hidden");
    dom::set_text("trNote", "");
}

fn cancel_save() {
    let _ = dom::el("trSaveRow").set_attribute("hidden", "");
}

/// The confirmed save: one word, into the list the player chose.
fn save_confirm(app: &App) {
    let pair = pair_of();
    let Some(entry) = pair.target_entry.clone() else { return };
    let (words, _blocked) = crate::profanity::filter_allowed(vec![entry]);
    if words.is_empty() {
        cancel_save();
        dom::set_text("trNote", &crate::profanity::rejection_message().to_string());
        return;
    }
    let lang = speak_code(&pair.target);
    let entries: Vec<(String, String)> =
        words.iter().map(|w| (w.clone(), lang.clone())).collect();
    let dest = crate::lists_ui::chosen("trDest");
    let (list_name, _) = crate::lists_ui::commit(dest, &entries, crate::word_lists::ListSource::Translate);
    crate::importer::save_words(&mut app.borrow_mut(), words, lang, &[]);
    crate::lists_ui::refresh_pool(app);
    crate::game::build_source_options(app);
    cancel_save();
    dom::set_text("trNote", &crate::lists_ui::saved_note(&list_name, 1));
}

/// F10: hand the target word to the standard session, in its own language.
/// A word the player may not be served gets an honest inline decline.
fn spell_it(app: &App) {
    let pair = pair_of();
    let Some(entry) = pair.target_entry.clone() else { return };
    let level = crate::play_hub::live_entitlements().lang_level(&pair.target);
    match rules::spell_it_verdict(experience(app), level, &pair.target, &entry) {
        SpellVerdict::Serve => {
            crate::game::request_word(entry, pair.target.clone());
            close();
            crate::game::stop_timer(true);
            crate::game::next_word(app);
        }
        SpellVerdict::Decline => dom::set_text("trNote", &t("cal.declined")),
    }
}

fn daily(app: &App, dw: &str) {
    let lang = app.borrow().lang.clone();
    let exp = experience(app);
    let mut pair = pair_of();
    if pair.source != lang {
        pair = pair.with_source_lang(exp, &lang);
    }
    set_pair(pair.clone());
    let shown = crate::translate::display_word(dw);
    dom::input("trSrcInput").set_value(&shown);
    let found = rules::search(exp, &pair.source, &pair.target, &shown);
    SUGGESTIONS.with(|s| *s.borrow_mut() = found.clone());
    match found.iter().position(|s| s.entry == dw) {
        Some(idx) => commit(app, idx),
        None => render_suggestions(&shown, &found),
    }
}

fn toggle_translit(app: &App) {
    let pair = pair_of();
    let now = translit_on(&pair.target);
    TRANSLIT.with(|m| {
        m.borrow_mut().insert(pair.target.clone(), !now);
    });
    render(app);
}

pub fn wire(app: &App) {
    if !dom::exists("trScreen") {
        return;
    }
    {
        let a = app.clone();
        dom::on_click("trOpenBtn", move || open(&a));
    }
    dom::on_click("trExit", close);
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("trSrcInput", "input", move |_| on_input(&a));
    }
    {
        let a = app.clone();
        dom::on_click("trSrcLang", move || open_picker(&a, "source"));
    }
    {
        let a = app.clone();
        dom::on_click("trTgtLang", move || open_picker(&a, "target"));
    }
    {
        let a = app.clone();
        dom::on_click("trSwap", move || swap(&a));
    }
    {
        let a = app.clone();
        dom::on_click("trClear", move || clear(&a));
    }
    {
        let a = app.clone();
        dom::on_click("trSave", move || save(&a));
    }
    {
        let a = app.clone();
        dom::on_click("trSaveGo", move || save_confirm(&a));
    }
    {
        dom::on_click("trSaveCancel", cancel_save);
        // Leaving the screen never leaves a half-finished save behind.
        dom::on_click("trExit", cancel_save);
    }
    {
        let a = app.clone();
        dom::on_click("trSpell", move || spell_it(&a));
    }
    dom::on_click("trSrcAudio", || hear("source"));
    dom::on_click("trTgtAudio", || hear("target"));
    {
        let a = app.clone();
        dom::on_click("trTranslitToggle", move || toggle_translit(&a));
    }
    // Delegated taps: suggestions, "more meanings", picker rows, the declines'
    // way-forward buttons, and the daily word.
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("trScreen", "click", move |e| {
        let Some(target) = e.target().and_then(|x| x.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target
            .closest("[data-tr-sugg],[data-tr-more],[data-tr-lang],[data-tr-pick],[data-tr-daily]")
            .ok()
            .flatten()
        else {
            return;
        };
        if let Some(i) = el.get_attribute("data-tr-sugg").and_then(|n| n.parse::<usize>().ok()) {
            commit(&a, i);
        } else if let Some(entry) = el.get_attribute("data-tr-more") {
            EXPANDED.with(|x| *x.borrow_mut() = Some(entry));
            let q = dom::input("trSrcInput").value();
            let found = SUGGESTIONS.with(|s| s.borrow().clone());
            render_suggestions(&q, &found);
        } else if let Some(l) = el.get_attribute("data-tr-lang") {
            choose_lang(&a, &l);
        } else if let Some(which) = el.get_attribute("data-tr-pick") {
            open_picker(&a, if which == "source" { "source" } else { "target" });
        } else if let Some(dw) = el.get_attribute("data-tr-daily") {
            daily(&a, &dw);
        }
    });
}
