//! CC-REPORTS-SHARE F5–F7, extended by CC-GUARDIAN-REPORT F4–F7 — the
//! Guardian Report screen. App-shell only. The launcher opens the adult gate
//! first (F6/F4), so the drawer row, `#progress-report` and the Monday ping's
//! `#guardian-report?week=` link all pass it. The text is the composer's, and
//! it leaves only through the OS share sheet (D-R1). The one write is Add to
//! My Words, after the gate (I8), through the CC-MYWORDS-LISTS keyed create
//! (I9). Practice these is absent: there is no profile switch (census C7).

use std::cell::Cell;

use crate::guardian_report::{self as gr, GuardianInput, LiveBank, PingCopy};
use crate::learner_query::DEVICE;
use crate::reports_share::{render_text, ComposeInput, DeviceZone, Report, Voice, Which};

thread_local! {
    static WHICH: Cell<Which> = const { Cell::new(Which::LastWeek) };
    /// I8: the adult gate was passed in this app session.
    static GATE_PASSED: Cell<bool> = const { Cell::new(false) };
    /// F7: the OS said no; the toggle reads off whatever was stored.
    static DENIED: Cell<bool> = const { Cell::new(false) };
}

/// A date in the UI language, through Intl. Noon UTC, read back in UTC, so no
/// zone can move it across a day boundary.
fn date_label(y: i64, m: u32, d: u32, with_year: bool) -> String {
    let ms = crate::reports_share::days_from_civil(y, m, d) as f64 * 86_400_000.0 + 43_200_000.0;
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    let opts = js_sys::Object::new();
    let mut fields = vec![("month", "short"), ("day", "numeric"), ("timeZone", "UTC")];
    if with_year {
        fields.push(("year", "numeric"));
    }
    for (k, v) in fields {
        let _ = js_sys::Reflect::set(&opts, &wasm_bindgen::JsValue::from_str(k), &wasm_bindgen::JsValue::from_str(v));
    }
    let s: String = date.to_locale_date_string(&crate::i18n::current(), &opts).into();
    if s.is_empty() {
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        s
    }
}

fn now_s() -> i64 {
    (js_sys::Date::now() / 1000.0) as i64
}

/// Every active language, in registry order.
fn langs() -> Vec<&'static str> {
    crate::consts::BUILTIN_LANGS.iter().map(|(c, _, _, _)| *c).filter(|c| crate::consts::is_active_lang(c)).collect()
}

/// The Guardian Report for `which`, every active language.
fn report_for(app: &crate::App, which: Which) -> Report {
    let junior = crate::play_hub::ctx(app).audience == crate::experience::Audience::Junior;
    let s = app.borrow();
    let q = crate::learner_query::live(&s);
    let langs = langs();
    let bank = LiveBank::default();
    gr::compose(&GuardianInput {
        base: ComposeInput { q: &q, profile: DEVICE, langs: &langs, now_s: now_s(), zone: &DeviceZone, which },
        bank: &bank,
        junior,
    })
}

fn text(r: &Report) -> String {
    let t = |k: &str| crate::i18n::t(k);
    render_text(r, &Voice { t: &t, date: &date_label })
}

fn render(app: &crate::App) {
    let which = WHICH.with(Cell::get);
    let _ = crate::dom::el("rshareNote").set_attribute("hidden", "");
    crate::dom::set_text("rshareBody", &text(&report_for(app, which)));
    crate::dom::toggle_class("rshareLast", "on", which == Which::LastWeek);
    crate::dom::toggle_class("rshareNow", "on", which == Which::ThisWeek);
    // Absent where it cannot run (Kid Mode, no share mechanism), not disabled.
    crate::dom::toggle_class("rshareShare", "btn-hide", !crate::share::available());
    render_ping();
}

/// F7: the toggle exists only where local notifications do. It reads on only
/// if the parent turned it on AND the OS allows it.
fn render_ping() {
    let here = crate::notifications::available();
    crate::dom::toggle_class("rsharePing", "btn-hide", !here);
    let on = crate::notifications::digest_on() && !DENIED.with(Cell::get);
    crate::dom::toggle_class("rsharePing", "on", here && on);
    let _ = crate::dom::el("rsharePing").set_attribute("aria-pressed", if here && on { "true" } else { "false" });
    crate::dom::toggle_class("rsharePingDenied", "btn-hide", !(here && DENIED.with(Cell::get)));
}

/// What the OS says now, without asking: a revoked permission reads off.
fn check_ping() {
    if !crate::notifications::digest_on() {
        return;
    }
    crate::notifications::permission(false, |granted| {
        DENIED.with(|d| d.set(granted == Some(false)));
        render_ping();
    });
}

/// F7 scheduling: cancel, then schedule the one notification for the next
/// Monday 08:00 with the closing week's counts, or nothing for a week with no
/// sessions (G-D12). The copy is the Guardian Report's where its row is
/// reachable, else CC-GUARDIAN-DASH's digest line (the Phase D merge).
pub fn refresh_ping(app: &crate::App) {
    if !crate::notifications::available() {
        return;
    }
    if !crate::notifications::digest_on() {
        crate::notifications::guardian_ping(None);
        return;
    }
    let ctx = crate::play_hub::ctx(app);
    let reachable = crate::modes::catalog(&crate::modes::all(), &ctx).iter().any(|m| m.id == "progress_report");
    let copy = if reachable { PingCopy::Guardian } else { PingCopy::Digest };
    let ping_monday = gr::next_ping_monday(now_s(), &DeviceZone);
    let week = {
        let s = app.borrow();
        let q = crate::learner_query::live(&s);
        let langs = langs();
        let bank = LiveBank::default();
        let input = GuardianInput {
            base: ComposeInput { q: &q, profile: DEVICE, langs: &langs, now_s: now_s(), zone: &DeviceZone, which: Which::LastWeek },
            bank: &bank,
            junior: ctx.audience == crate::experience::Audience::Junior,
        };
        gr::profile_week(&input, ping_monday - 7)
    };
    let t = |k: &str| crate::i18n::t(k);
    crate::notifications::guardian_ping(gr::ping(copy, ping_monday, &[week], &t).as_ref());
}

/// F7 toggle, shared with the dashboard: turning it on asks the OS (the only
/// place this screen ever does); a refusal leaves it off with one line.
/// `done` learns whether the OS refused.
pub fn toggle_ping(app: &crate::App, done: impl Fn(bool) + 'static) {
    if crate::notifications::digest_on() && !DENIED.with(Cell::get) {
        crate::notifications::set_digest_on(false);
        refresh_ping(app);
        return done(false);
    }
    let a = app.clone();
    crate::notifications::permission(true, move |granted| {
        let granted = granted == Some(true);
        DENIED.with(|d| d.set(!granted));
        crate::notifications::set_digest_on(granted);
        refresh_ping(&a);
        done(!granted);
    });
}

/// F6 Add: one dated list per language holding that language's practice
/// words, keyed `(profile, week, language)`. A second Add writes nothing and
/// opens the list the first made. Returns the list to open, and what to say.
fn add(app: &crate::App) -> Option<(String, String)> {
    if !GATE_PASSED.with(Cell::get) {
        return None; // I8: never without the gate, whatever called this
    }
    let r = report_for(app, WHICH.with(Cell::get));
    let by_lang: Vec<(&str, &[String])> = r.sections.iter().filter(|s| !s.practice.is_empty()).map(|s| (s.lang.as_str(), s.practice.as_slice())).collect();
    if by_lang.is_empty() {
        // Never disabled (D-R7): an empty week says so instead.
        crate::dom::set_text("rshareNote", &crate::i18n::t("rshare.empty"));
        let _ = crate::dom::el("rshareNote").remove_attribute("hidden");
        return None;
    }
    let (y, m, d) = crate::reports_share::civil(r.monday);
    let base = crate::i18n::tp("greport.listName", &[("date", &date_label(y, m, d, false))]);
    let now = js_sys::Date::now();
    let mut lists = crate::word_lists::load();
    let (mut open, mut created) = (None, false);
    for (lang, words) in &by_lang {
        // Several languages: one list each, told apart by language, not "(2)".
        let name = if by_lang.len() > 1 { format!("{base} · {}", crate::reports_share::lang_name(lang)) } else { base.clone() };
        let id = match crate::word_lists::create_keyed(&mut lists, &gr::list_key(DEVICE, r.monday, lang), &name, now) {
            crate::word_lists::Keyed::Existing(id) => id,
            crate::word_lists::Keyed::Created(id) => {
                let entries: Vec<(String, String)> = words.iter().map(|w| (w.clone(), lang.to_string())).collect();
                crate::word_lists::add_entries(&mut lists, &id, &entries, now);
                created = true;
                id
            }
        };
        open.get_or_insert(id);
    }
    if created {
        crate::word_lists::store(&lists);
    }
    open.map(|id| (id, crate::i18n::t(if created { "greport.added" } else { "greport.already" })))
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("rshareScrim") {
        return;
    }
    {
        let a = app.clone();
        crate::dom::on_click("rshareOpenBtn", move || {
            // A deep link names its week; the drawer row opens last week.
            let this_monday = crate::reports_share::week_monday(now_s(), &DeviceZone);
            let which = crate::take_route()
                .and_then(|h| gr::week_of_route(&h))
                .filter(|m| *m <= this_monday)
                .map_or(Which::LastWeek, |m| if m == this_monday - 7 { Which::LastWeek } else { Which::Week(m) });
            let a = a.clone();
            crate::parent_gate_then(Box::new(move || {
                GATE_PASSED.with(|g| g.set(true));
                WHICH.with(|w| w.set(which));
                render(&a);
                crate::dom::add_class("rshareScrim", "show");
                check_ping();
            }));
        });
    }
    for (id, which) in [("rshareLast", Which::LastWeek), ("rshareNow", Which::ThisWeek)] {
        let a = app.clone();
        crate::dom::on_click(id, move || {
            WHICH.with(|w| w.set(which));
            render(&a);
        });
    }
    {
        let a = app.clone();
        crate::dom::on_click("rshareShare", move || crate::share::share_text(&text(&report_for(&a, WHICH.with(Cell::get)))));
    }
    {
        let a = app.clone();
        crate::dom::on_click("rshareAdd", move || {
            if let Some((id, said)) = add(&a) {
                crate::dom::remove_class("rshareScrim", "show");
                crate::lists_screen::open_list_noting(&a, &id, &said);
            }
        });
    }
    {
        let a = app.clone();
        crate::dom::on_click("rsharePing", move || toggle_ping(&a, |_| render_ping()));
    }
    crate::dom::on_click("rshareDone", || crate::dom::remove_class("rshareScrim", "show"));
    crate::dom::on::<web_sys::Event, _>("rshareScrim", "click", |e| {
        if crate::dom::is_self_target(&e, "rshareScrim") {
            crate::dom::remove_class("rshareScrim", "show");
        }
    });
    // F7: reschedule on every foreground and every session end (the app
    // leaving the screen), and route the notification's action.
    {
        let a = app.clone();
        use wasm_bindgen::JsCast;
        let f = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || refresh_ping(&a));
        let _ = crate::dom::doc().add_event_listener_with_callback("visibilitychange", f.as_ref().unchecked_ref());
        f.forget();
    }
    crate::notifications::register_guardian_actions(|route| {
        let _ = crate::dom::window().location().set_hash(&route);
    });
    refresh_ping(app);
}
