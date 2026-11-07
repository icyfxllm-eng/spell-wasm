//! CC-REPORTS-SHARE F5–F7, extended by CC-GUARDIAN-REPORT F4–F7 — the
//! Guardian Report screen. App-shell only. The launcher opens the adult gate
//! first (F6/F4), so the drawer row, `#progress-report` and the Monday ping's
//! `#guardian-report?week=` link all pass it. The text is the composer's, and
//! it leaves only through the OS share sheet (D-R1). The one write is Add to
//! My Words, after the gate (I8), through the CC-MYWORDS-LISTS keyed create
//! (I9). Practice these is absent: there is no profile switch (census C7).

use std::cell::Cell;

use crate::guardian_report::{self as gr, GuardianInput, LiveBank, PingCopy};
use crate::report_days::{self as rd, DaysState, WeekdaySet};
use crate::learner_query::DEVICE;
use crate::reports_share::{render_text, ComposeInput, DeviceZone, Report, Voice, Which};

thread_local! {
    static WHICH: Cell<Which> = const { Cell::new(Which::LastWeek) };
    /// I8: the adult gate was passed in this app session.
    static GATE_PASSED: Cell<bool> = const { Cell::new(false) };
    /// F7: the OS said no; the toggle reads off whatever was stored.
    static DENIED: Cell<bool> = const { Cell::new(false) };
    /// Report Days (flag): the weekdays being picked, not yet saved.
    static PENDING: Cell<u8> = const { Cell::new(0) };
    /// The Custom chips were opened by hand.
    static CUSTOM: Cell<bool> = const { Cell::new(false) };
    /// The screen shows the read-only Last N days range, not the scheduled report.
    static ON_DEMAND: Cell<bool> = const { Cell::new(false) };
    static RANGE_N: Cell<u32> = const { Cell::new(7) };
}

fn days_on() -> bool {
    crate::flags::guardian_report_days()
}

fn days_state() -> DaysState {
    crate::report_days_store::load().unwrap_or_else(|| DaysState::default_at(now_s()))
}

/// The latest scheduled window. Seeing it marks it delivered (the pin for the
/// next window); the on-demand range never comes through here (F7).
fn scheduled_which() -> Which {
    let mut st = days_state();
    let w = st.view_latest(now_s(), &DeviceZone);
    crate::report_days_store::save(&st);
    w.map_or(Which::LastWeek, |w| w.which(false))
}

fn on_demand_which() -> Which {
    rd::on_demand(now_s(), RANGE_N.with(Cell::get), &DeviceZone).map_or(Which::LastWeek, |w| w.which(true))
}

/// A weekday name in the UI language, through Intl (no copy to translate).
fn weekday_label(w: u8) -> String {
    let monday = crate::reports_share::days_from_civil(2026, 11, 2);
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(rd::day_noon_ms(monday + i64::from(w))));
    let opts = js_sys::Object::new();
    for (k, v) in [("weekday", "short"), ("timeZone", "UTC")] {
        let _ = js_sys::Reflect::set(&opts, &wasm_bindgen::JsValue::from_str(k), &wasm_bindgen::JsValue::from_str(v));
    }
    date.to_locale_date_string(&crate::i18n::current(), &opts).into()
}

fn range_label() -> String {
    match RANGE_N.with(Cell::get) {
        1 => crate::i18n::t("greport.days.last1"),
        n => crate::i18n::tp("greport.days.last", &[("n", &n.to_string())]),
    }
}

fn render_days() {
    let on = days_on();
    crate::dom::toggle_class("rdaysBox", "btn-hide", !on);
    crate::dom::toggle_class("rdaysLastN", "btn-hide", !on);
    crate::dom::toggle_class("rshareNow", "btn-hide", on);
    if !on {
        return;
    }
    let pending = PENDING.with(Cell::get);
    let preset = WeekdaySet::from_bits(pending).map(rd::preset_of);
    let custom_open = CUSTOM.with(Cell::get) || matches!(preset, Some(rd::Preset::Custom) | None);
    for (id, p) in [("rdaysWeekly", rd::Preset::Weekly), ("rdaysTwice", rd::Preset::TwiceAWeek), ("rdaysDaily", rd::Preset::Daily)] {
        let _ = crate::dom::el(id).set_attribute("aria-pressed", if !custom_open && preset == Some(p) { "true" } else { "false" });
    }
    let _ = crate::dom::el("rdaysCustom").set_attribute("aria-pressed", if custom_open { "true" } else { "false" });
    crate::dom::toggle_class("rdaysChips", "btn-hide", !custom_open);
    for w in 0..7u8 {
        let id = format!("rdaysD{w}");
        crate::dom::set_text(&id, &weekday_label(w));
        let _ = crate::dom::el(&id).set_attribute("aria-pressed", if pending & (1 << w) != 0 { "true" } else { "false" });
    }
    // A12: Save stays disabled while the set is empty.
    if pending == 0 {
        let _ = crate::dom::el("rdaysSave").set_attribute("disabled", "");
    } else {
        let _ = crate::dom::el("rdaysSave").remove_attribute("disabled");
    }
    crate::dom::set_text("rdaysNLabel", &range_label());
    crate::dom::set_text("rdaysLastN", &range_label());
    crate::dom::set_text("rshareLast", &crate::i18n::t("greport.days.latest"));
    let od = ON_DEMAND.with(Cell::get);
    crate::dom::toggle_class("rshareLast", "on", !od);
    crate::dom::toggle_class("rdaysLastN", "on", od);
}

/// A date in the UI language, through Intl. Noon UTC, read back in UTC, so no
/// zone can move it across a day boundary.
fn date_label(y: i64, m: u32, d: u32, with_year: bool) -> String {
    let ms = crate::report_days::day_noon_ms(crate::reports_share::days_from_civil(y, m, d));
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
    let input = GuardianInput {
        base: ComposeInput { q: &q, profile: DEVICE, langs: &langs, now_s: now_s(), zone: &DeviceZone, which },
        bank: &bank,
        junior,
    };
    // Flag off: v1's composer, untouched. On: any window, with G-D26 applied.
    if days_on() && matches!(which, Which::Span { .. }) {
        gr::compose_days(&input, None)
    } else {
        gr::compose(&input)
    }
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
    render_days();
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
    // Report Days (flag): the next report day of the parent's schedule, and the
    // window that ends on it. Nothing in it = nothing scheduled (F6). A schedule
    // edit, a foreground and a session end all come through here (F8), and the
    // zone is read fresh each time, so a changed zone is picked up too.
    if days_on() {
        let log = days_state().log;
        let Some(w) = rd::next_ping_window(now_s(), &log, &DeviceZone, gr::PING_HOUR) else {
            crate::notifications::guardian_ping(None);
            return;
        };
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
            gr::profile_window(&input, &w)
        };
        let t = |k: &str| crate::i18n::t(k);
        crate::notifications::guardian_ping(gr::ping_for_day(copy, w.last_day + 1, &[week], &t).as_ref());
        return;
    }
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
        let id = match crate::word_lists::create_keyed(&mut lists, &gr::list_key_span(DEVICE, r.monday, r.last_day, lang), &name, now) {
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

/// F1 and F7. Nothing here reaches the ping: a schedule edit's reschedule is
/// F8 (Phase D): a Save reschedules the ping; the on-demand range never touches state (A10).
fn wire_days(app: &crate::App) {
    for (id, days) in [("rdaysWeekly", WeekdaySet::WEEKLY), ("rdaysTwice", WeekdaySet::TWICE_A_WEEK), ("rdaysDaily", WeekdaySet::DAILY)] {
        let a = app.clone();
        crate::dom::on_click(id, move || {
            CUSTOM.with(|c| c.set(false));
            PENDING.with(|p| p.set(days.bits()));
            render(&a);
        });
    }
    {
        let a = app.clone();
        crate::dom::on_click("rdaysCustom", move || {
            CUSTOM.with(|c| c.set(true));
            render(&a);
        });
    }
    for w in 0..7u8 {
        let a = app.clone();
        crate::dom::on_click(&format!("rdaysD{w}"), move || {
            CUSTOM.with(|c| c.set(true));
            PENDING.with(|p| p.set(p.get() ^ (1 << w)));
            render(&a);
        });
    }
    {
        let a = app.clone();
        crate::dom::on_click("rdaysSave", move || {
            // The model refuses an empty set whatever the button says (A12).
            let Some(days) = WeekdaySet::from_bits(PENDING.with(Cell::get)) else { return };
            let mut st = days_state();
            st.save(days, now_s());
            crate::report_days_store::save(&st);
            ON_DEMAND.with(|o| o.set(false));
            WHICH.with(|w| w.set(scheduled_which()));
            render(&a);
            // F8: a schedule edit moves the next ping.
            refresh_ping(&a);
        });
    }
    {
        let a = app.clone();
        crate::dom::on::<web_sys::Event, _>("rdaysN", "input", move |_| {
            let n = crate::dom::input("rdaysN").value().parse::<u32>().unwrap_or(7).clamp(1, 7);
            RANGE_N.with(|r| r.set(n));
            ON_DEMAND.with(|o| o.set(true));
            WHICH.with(|w| w.set(on_demand_which()));
            render(&a);
        });
    }
    {
        let a = app.clone();
        crate::dom::on_click("rdaysLastN", move || {
            ON_DEMAND.with(|o| o.set(true));
            WHICH.with(|w| w.set(on_demand_which()));
            render(&a);
        });
    }
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
                .map(|m| if m == this_monday - 7 { Which::LastWeek } else { Which::Week(m) });
            let a = a.clone();
            crate::parent_gate_then(Box::new(move || {
                GATE_PASSED.with(|g| g.set(true));
                // Flag on, no deep link: the latest scheduled report.
                ON_DEMAND.with(|o| o.set(false));
                PENDING.with(|p| p.set(days_state().current().bits()));
                CUSTOM.with(|c| c.set(false));
                let which = which.unwrap_or_else(|| if days_on() { scheduled_which() } else { Which::LastWeek });
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
            ON_DEMAND.with(|o| o.set(false));
            WHICH.with(|w| w.set(if days_on() && which == Which::LastWeek { scheduled_which() } else { which }));
            render(&a);
        });
    }
    wire_days(app);
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
