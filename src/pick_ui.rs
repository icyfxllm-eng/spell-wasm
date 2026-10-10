//! CC-PICK-QUEUE v1 Phase A -- the ONE lineup component (D10).
//!
//! Misses and My Words both render their rows, badges, footer and tap handling
//! from here and read the same `pick_queue::Lineups` (I8). Nothing in this file
//! asks which language it is in: a scope is (surface, list, language) data and
//! the tap handler treats every scope the same way.
//!
//! The Misses list surface is new (Misses had no list screen) and is created on
//! first use, so the markup exists only where this feature is wired: the app,
//! flag on. The site build never calls any of it (O-F).

use std::cell::{Cell, RefCell};

use crate::pick_queue::{self as pq, Pick, Toggled, SURFACE_MINE, SURFACE_MISSES};
use crate::pick_stars::{self as stars, StarToggled};
use crate::{dom, i18n, i18n::t, App};
use wasm_bindgen::JsCast;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// The language the Misses sheet is showing (a lineup never spans two).
    static MISS_LANG: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Set when Jr's cap refused a pick; the next render says why, once.
    static REFUSED: Cell<bool> = const { Cell::new(false) };
    /// Set when the ten-star cap refused a star (A6); the next render says why.
    static STAR_REFUSED: Cell<bool> = const { Cell::new(false) };
}

const SCREEN: &str = "pickScreen";
const BODY: &str = "pickBody";
const END_BTN: &str = "pickEndBtn";

fn esc(s: &str) -> String {
    dom::escape_html(s)
}

pub fn wire(app: &App) {
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));
}

fn key_of(lang: &str, word: &str) -> String {
    crate::word_id::word_id0(lang, word)
}

// ---------- shared rendering ----------

/// A row's number badge plus its tap target. `enabled` is false for a word in
/// another language than the lineup that is already in progress (D9).
pub fn word_html(surface: &str, list: &str, lang: &str, word: &str, enabled: bool) -> String {
    let scope = pq::scope_id(surface, list, lang);
    let num = pq::read(|l| l.number_of(&scope, &key_of(lang, word)));
    let badge = num
        .map(|n| format!("<span class=\"pq-badge\" aria-hidden=\"true\"><bdi>{n}</bdi></span>"))
        .unwrap_or_default();
    let label = match num {
        Some(n) => i18n::tp("pick.remove", &[("word", word), ("n", &n.to_string())]),
        None => i18n::tp("pick.add", &[("word", word)]),
    };
    if !enabled {
        return format!("<span class=\"le-word pq-word pq-off\">{}</span>", esc(word));
    }
    format!(
        "{badge}<button type=\"button\" class=\"le-word pq-word\" data-pq-toggle \
           data-pq-surface=\"{surface}\" data-pq-list=\"{list}\" data-pq-lang=\"{lang}\" data-pq-word=\"{word}\" \
           aria-pressed=\"{pressed}\" aria-label=\"{label}\">{text}</button>",
        surface = esc(surface),
        list = esc(list),
        lang = esc(lang),
        word = esc(word),
        pressed = if num.is_some() { "true" } else { "false" },
        label = esc(&label),
        text = esc(word),
    )
}

// ---------- focus stars (Phase B) ----------

/// The star control, top-right of the word row. Standard players with the flag
/// on only: for Spell Jr, or flag off, this is the empty string -- absent, not
/// locked. Both surfaces call this one function (D10).
pub fn star_html(surface: &str, list: &str, lang: &str, word: &str, on: bool, kid: bool) -> String {
    if !stars::active(kid) {
        return String::new();
    }
    let label = i18n::tp(if on { "star.remove" } else { "star.add" }, &[("word", word)]);
    format!(
        "<button type=\"button\" class=\"pq-star{on_cls}\" data-pq-star \
           data-pq-surface=\"{surface}\" data-pq-list=\"{list}\" data-pq-lang=\"{lang}\" data-pq-word=\"{word}\" \
           aria-pressed=\"{pressed}\" aria-label=\"{label}\"><span aria-hidden=\"true\">{glyph}</span></button>",
        on_cls = if on { " on" } else { "" },
        surface = esc(surface),
        list = esc(list),
        lang = esc(lang),
        word = esc(word),
        pressed = if on { "true" } else { "false" },
        label = esc(&label),
        glyph = if on { "\u{2605}" } else { "\u{2606}" },
    )
}

/// Extra row class that reserves the star's corner (empty when no star shows).
pub fn star_row_class(kid: bool) -> &'static str {
    if stars::active(kid) { " pq-s" } else { "" }
}

/// Whether a Misses word is starred (the one read of that store for the UI).
pub fn miss_starred(lang: &str, word: &str) -> bool {
    stars::read(|s| s.has(&key_of(lang, word)))
}

pub fn row_class(surface: &str, list: &str, lang: &str, word: &str, enabled: bool) -> &'static str {
    let scope = pq::scope_id(surface, list, lang);
    let on = pq::read(|l| l.number_of(&scope, &key_of(lang, word)).is_some());
    match (enabled, on) {
        (false, _) => "pq-row pq-off",
        (true, true) => "pq-row on",
        (true, false) => "pq-row",
    }
}

/// Hint line + the start button and its "Just these" toggle, for one scope.
pub fn footer_html(surface: &str, list: &str, lang: &str, kid: bool) -> String {
    let extras = crate::experience::pick_caps(crate::experience::of_kid(kid)).extras;
    let scope = pq::scope_id(surface, list, lang);
    let (n, just) = pq::read(|l| (l.picks(&scope).len(), l.just_these(&scope)));
    let mut html = format!("<div class=\"lists-head\" role=\"note\">{}</div>", esc(&t("pick.hint")));
    if REFUSED.with(|c| c.replace(false)) {
        html.push_str(&format!("<div class=\"pq-note\" role=\"status\">{}</div>", esc(&t("pick.capJr"))));
    }
    if stars::active(kid) {
        html.push_str(&format!("<div class=\"lists-head\" role=\"note\">{}</div>", esc(&t("star.hint"))));
        if STAR_REFUSED.with(|c| c.replace(false)) {
            html.push_str(&format!("<div class=\"pq-note\" role=\"status\">{}</div>", esc(&t("star.cap"))));
        }
    }
    if n > 0 {
        html.push_str(&format!(
            "<div class=\"lists-actions pq-actions\">\
               <button type=\"button\" class=\"btn btn-check\" data-pq-start data-pq-surface=\"{s}\" data-pq-list=\"{li}\" data-pq-lang=\"{la}\">{start}</button>\
               {just_btn}\
             </div>",
            s = esc(surface),
            li = esc(list),
            la = esc(lang),
            start = esc(&i18n::tp("pick.start", &[("n", &n.to_string())])),
            just_btn = if extras {
                format!(
                    "<button type=\"button\" class=\"ghost{}\" data-pq-just data-pq-surface=\"{}\" data-pq-list=\"{}\" data-pq-lang=\"{}\" aria-pressed=\"{}\">{}</button>",
                    if just { " on" } else { "" },
                    esc(surface),
                    esc(list),
                    esc(lang),
                    if just { "true" } else { "false" },
                    esc(&t("pick.justThese")),
                )
            } else {
                String::new()
            },
        ));
    }
    html
}

/// The language of the lineup already in progress in this list, if any: while
/// one exists, words in other languages are not tappable (D9).
pub fn active_lang(surface: &str, list: &str) -> Option<String> {
    pq::read(|l| {
        l.scopes.values().find(|s| s.surface == surface && s.list == list && !s.picks.is_empty()).map(|s| s.lang.clone())
    })
}

/// Drop picks whose word has left the list (O-D) or graduated (O-C).
pub fn prune_mine(list: &str, entries: &[(String, String)]) {
    let ids: Vec<String> = pq::read(|l| {
        l.scopes.iter().filter(|(_, s)| s.surface == SURFACE_MINE && s.list == list).map(|(k, _)| k.clone()).collect()
    });
    for id in ids {
        pq::write(|l| {
            l.prune(&id, |p| entries.iter().any(|(text, lang)| *text == p.word && *lang == p.lang));
        });
    }
}

fn prune_misses(app: &App) {
    let live: Vec<String> = app
        .borrow()
        .misses
        .iter()
        .map(|m| crate::misses::miss_key(&m.word, &m.lang))
        .collect();
    let ids: Vec<String> = pq::read(|l| {
        l.scopes.iter().filter(|(_, s)| s.surface == SURFACE_MISSES).map(|(k, _)| k.clone()).collect()
    });
    for id in ids {
        pq::write(|l| {
            l.prune(&id, |p| live.contains(&p.key));
        });
    }
    // A starred miss that graduated or was evicted loses its star silently (O-D).
    stars::prune_misses(|p| live.contains(&p.key));
}

// ---------- tap handling (both surfaces) ----------

/// Handle a lineup control. Returns true if `el` was one; the host screen then
/// re-renders. `close` closes the host screen when a session starts.
pub fn handle_click(app: &App, el: &web_sys::Element, close: &dyn Fn()) -> bool {
    let attr = |n: &str| el.get_attribute(n).unwrap_or_default();
    let has = |n: &str| el.has_attribute(n);
    if has("data-pq-toggle") {
        let (surface, list, lang, word) = (attr("data-pq-surface"), attr("data-pq-list"), attr("data-pq-lang"), attr("data-pq-word"));
        let id = pq::scope_id(&surface, &list, &lang);
        let pick = Pick { key: key_of(&lang, &word), word, lang: lang.clone() };
        let kid = app.borrow().kid;
        if pq::toggle(&id, &surface, &list, &lang, pick, kid) == Toggled::Refused {
            REFUSED.with(|c| c.set(true));
        }
        return true;
    }
    if has("data-pq-star") {
        let (surface, list, lang, word) = (attr("data-pq-surface"), attr("data-pq-list"), attr("data-pq-lang"), attr("data-pq-word"));
        // Spell Jr has no stars (I7); a stray tap changes nothing.
        if !stars::active(app.borrow().kid) {
            return true;
        }
        let r = if surface == SURFACE_MINE {
            toggle_mine_star(app, &list, &lang, &word)
        } else {
            let live: Vec<String> =
                app.borrow().misses.iter().map(|m| crate::misses::miss_key(&m.word, &m.lang)).collect();
            let pick = Pick { key: key_of(&lang, &word), word, lang };
            stars::toggle_miss(pick, |p| live.contains(&p.key))
        };
        if r == StarToggled::Refused {
            STAR_REFUSED.with(|c| c.set(true));
        }
        return true;
    }
    if has("data-pq-just") {
        // Spell Jr has no such control (D4); a stray tap changes nothing.
        if !crate::experience::pick_caps(crate::experience::of_kid(app.borrow().kid)).extras {
            return true;
        }
        let id = pq::scope_id(&attr("data-pq-surface"), &attr("data-pq-list"), &attr("data-pq-lang"));
        pq::write(|l| {
            let on = !l.just_these(&id);
            l.set_just_these(&id, on);
        });
        return true;
    }
    if has("data-pq-start") {
        let (surface, list, lang) = (attr("data-pq-surface"), attr("data-pq-list"), attr("data-pq-lang"));
        start(app, &surface, &list, &lang, close);
        return true;
    }
    if has("data-pq-langchip") {
        MISS_LANG.with(|c| *c.borrow_mut() = Some(attr("data-pq-lang")));
        return true;
    }
    if has("data-pq-all") {
        close();
        crate::game::enter_review(app);
        // Stars lead in the language the sheet was showing.
        if let Some(l) = MISS_LANG.with(|c| c.borrow().clone()) {
            pq::set_star_lang(&l);
        }
        return true;
    }
    false
}

/// A My Words star IS `ListEntry.starred` (O-A): the same field "In my order"
/// has always read, written through the same `set_star`. The cap counts the
/// language's distinct starred words across every live list.
fn toggle_mine_star(app: &App, list: &str, lang: &str, word: &str) -> StarToggled {
    let mut lists = crate::word_lists::load();
    let Some((idx, on)) = lists
        .lists
        .iter()
        .find(|l| l.id == list && l.live())
        .and_then(|l| l.entries.iter().position(|e| e.text == word && e.lang == lang).map(|i| (i, !l.entries[i].starred)))
    else {
        return StarToggled::Unstarred;
    };
    if on && !stars::cap_allows(crate::word_lists::starred_count(&lists, lang)) {
        return StarToggled::Refused;
    }
    crate::word_lists::set_star(&mut lists, list, idx, on, js_sys::Date::now());
    crate::word_lists::store(&lists);
    // The pool's order follows the star under "In my order".
    crate::lists_ui::refresh_pool(app);
    if on { StarToggled::Starred } else { StarToggled::Unstarred }
}

/// "Practice N first". The order is changed; nothing else is (I1).
fn start(app: &App, surface: &str, list: &str, lang: &str, close: &dyn Fn()) {
    let id = pq::scope_id(surface, list, lang);
    if pq::read(|l| l.picks(&id).is_empty()) {
        return;
    }
    if app.borrow().review {
        crate::game::exit_review(app, None);
    }
    pq::begin_session(&id, app.borrow().kid);
    close();
    if surface == SURFACE_MINE {
        crate::lists_ui::play(app, &[list.to_string()]);
    } else {
        crate::game::enter_review(app);
    }
    sync_end_control();
}

// ---------- the End control (a WORD exit, on the play screen) ----------

/// Show the End control exactly while a lineup session runs. Cheap and
/// DOM-free when the feature is off.
pub fn sync_end_control() {
    let active = pq::session_active();
    let exists = dom::exists(END_BTN);
    if active && !exists {
        if !dom::exists("feedback") {
            return;
        }
        let html = format!(
            "<button type=\"button\" class=\"ghost pq-end\" id=\"{END_BTN}\">{}</button>",
            esc(&t("pick.end"))
        );
        let _ = dom::el("feedback").insert_adjacent_html("afterend", &html);
        dom::on_click(END_BTN, || {
            let app = APP.with(|a| a.borrow().clone());
            if let Some(app) = app {
                // The player ended it: the lineup goes with the session.
                pq::clear_session_scope();
                crate::game::exit_review(&app, Some(&t("pick.ended")));
            }
        });
    } else if !active && exists {
        dom::el(END_BTN).remove();
    }
}

// ---------- the Misses list surface ----------

fn ensure_screen() {
    if dom::exists(SCREEN) {
        return;
    }
    let html = format!(
        "<div class=\"lists-screen\" id=\"{SCREEN}\" role=\"dialog\" aria-modal=\"true\" aria-labelledby=\"pickTitle\">\
           <div class=\"lists-top\">\
             <button class=\"ghost\" id=\"pickClose\">{}</button>\
             <h2 id=\"pickTitle\">{}</h2>\
           </div>\
           <div class=\"lists-wrap\" id=\"{BODY}\"></div>\
         </div>",
        esc(&t("aria.close")),
        esc(&t("pick.title")),
    );
    if let Some(body) = dom::doc().body() {
        let _ = body.insert_adjacent_html("beforeend", &html);
    }
    dom::on_click("pickClose", || close_misses());
    dom::on::<web_sys::Event, _>(SCREEN, "click", move |ev| {
        let Some(app) = APP.with(|a| a.borrow().clone()) else { return };
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target
            .closest("[data-pq-toggle],[data-pq-star],[data-pq-start],[data-pq-just],[data-pq-langchip],[data-pq-all]")
            .ok()
            .flatten()
        else {
            return;
        };
        if handle_click(&app, &el, &|| close_misses()) {
            render_misses(&app);
        }
    });
}

fn close_misses() {
    if dom::exists(SCREEN) {
        dom::remove_class(SCREEN, "show");
    }
}

/// Open the Misses lineup surface (flag on). Replaces the straight-to-practice
/// tap on the Misses button; "Practice all misses" is the old behaviour.
pub fn open_misses(app: &App) {
    ensure_screen();
    prune_misses(app);
    render_misses(app);
    dom::add_class(SCREEN, "show");
}

fn render_misses(app: &App) {
    if !dom::exists(BODY) {
        return;
    }
    prune_misses(app);
    let s = app.borrow();
    let mut langs: Vec<String> = Vec::new();
    for m in &s.misses {
        if !langs.contains(&m.lang) {
            langs.push(m.lang.clone());
        }
    }
    let working = pq::read(|l| {
        l.scopes.values().find(|sc| sc.surface == SURFACE_MISSES && !sc.picks.is_empty()).map(|sc| sc.lang.clone())
    });
    let wanted = MISS_LANG.with(|c| c.borrow().clone());
    let lang = [wanted, working, Some(s.lang.clone())]
        .into_iter()
        .flatten()
        .find(|l| langs.contains(l))
        .or_else(|| langs.first().cloned())
        .unwrap_or_default();
    MISS_LANG.with(|c| *c.borrow_mut() = Some(lang.clone()));
    let mut html = String::new();
    if langs.len() > 1 {
        html.push_str("<div class=\"lists-row pq-langs\">");
        for l in &langs {
            html.push_str(&format!(
                "<button type=\"button\" class=\"ghost{}\" data-pq-langchip data-pq-lang=\"{}\" aria-pressed=\"{}\">{}</button>",
                if *l == lang { " on" } else { "" },
                esc(l),
                if *l == lang { "true" } else { "false" },
                esc(l),
            ));
        }
        html.push_str("</div>");
    }
    let rows: Vec<&crate::model::MissEntry> = s.misses.iter().filter(|m| m.lang == lang).collect();
    if rows.is_empty() {
        html.push_str(&format!("<div class=\"lists-empty\">{}</div>", esc(&t("pick.empty"))));
    }
    html.push_str(&footer_html(SURFACE_MISSES, "", &lang, s.kid));
    for m in rows {
        html.push_str(&format!(
            "<div class=\"lists-entry {}{}\">{}<span class=\"lists-chip\">{}</span>{}</div>",
            row_class(SURFACE_MISSES, "", &m.lang, &m.word, true),
            star_row_class(s.kid),
            word_html(SURFACE_MISSES, "", &m.lang, &m.word, true),
            esc(&m.lang),
            star_html(SURFACE_MISSES, "", &m.lang, &m.word, miss_starred(&m.lang, &m.word), s.kid),
        ));
    }
    html.push_str(&format!(
        "<div class=\"lists-actions\"><button type=\"button\" class=\"ghost\" data-pq-all>{}</button></div>",
        esc(&t("pick.practiceAll")),
    ));
    dom::set_html(BODY, &html);
}
