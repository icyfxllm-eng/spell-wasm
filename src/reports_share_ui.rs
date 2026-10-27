//! CC-REPORTS-SHARE F5–F7 — the parent report screen. App-shell only. The
//! launcher opens the adult gate first (F6), so the drawer row and the
//! `#progress-report` deep link both pass it. The text is the composer's, and
//! it leaves only through the OS share sheet (F5, D-R1). Nothing here writes.

use std::cell::Cell;

use crate::reports_share::{compose, render_text, ComposeInput, DeviceZone, Voice, Which};

thread_local! {
    static WHICH: Cell<Which> = const { Cell::new(Which::LastWeek) };
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

/// The report text for the current window, every active language.
fn text(app: &crate::App) -> String {
    let s = app.borrow();
    let q = crate::learner_query::live(&s);
    let langs: Vec<&str> = crate::consts::BUILTIN_LANGS
        .iter()
        .map(|(c, _, _, _)| *c)
        .filter(|c| crate::consts::is_active_lang(c))
        .collect();
    let report = compose(&ComposeInput {
        q: &q,
        profile: crate::learner_query::DEVICE,
        langs: &langs,
        now_s: (js_sys::Date::now() / 1000.0) as i64,
        zone: &DeviceZone,
        which: WHICH.with(Cell::get),
    });
    let t = |k: &str| crate::i18n::t(k);
    render_text(&report, &Voice { t: &t, date: &date_label })
}

fn render(app: &crate::App) {
    crate::dom::set_text("rshareBody", &text(app));
    let last = WHICH.with(Cell::get) == Which::LastWeek;
    crate::dom::toggle_class("rshareLast", "on", last);
    crate::dom::toggle_class("rshareNow", "on", !last);
    // F5: absent where it cannot run (Kid Mode, no share mechanism), not disabled.
    crate::dom::toggle_class("rshareShare", "btn-hide", !crate::share::available());
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("rshareScrim") {
        return;
    }
    {
        let a = app.clone();
        crate::dom::on_click("rshareOpenBtn", move || {
            let a = a.clone();
            crate::parent_gate_then(Box::new(move || {
                WHICH.with(|w| w.set(Which::LastWeek));
                render(&a);
                crate::dom::add_class("rshareScrim", "show");
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
        crate::dom::on_click("rshareShare", move || crate::share::share_text(&text(&a)));
    }
    crate::dom::on_click("rshareDone", || crate::dom::remove_class("rshareScrim", "show"));
    crate::dom::on::<web_sys::Event, _>("rshareScrim", "click", |e| {
        if crate::dom::is_self_target(&e, "rshareScrim") {
            crate::dom::remove_class("rshareScrim", "show");
        }
    });
}
