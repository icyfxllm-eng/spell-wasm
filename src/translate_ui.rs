//! CC-TRANSLATE-TOOLS — the tab surface. App-shell-only, pick-only
//! input (I4), audited strings only (I5), doors not dead-ends: every
//! card ends in "Now spell it" and "Add to plan".

use wasm_bindgen::JsCast;

thread_local! {
    static CARD_WORD: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static SHOW_ROMAN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static PAGE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn t(k: &str) -> String {
    crate::i18n::t(k)
}

pub fn open(app: &crate::App) {
    if !crate::dom::exists("trScrim") {
        return;
    }
    CARD_WORD.with(|c| *c.borrow_mut() = None);
    PAGE.with(|c| c.set(0));
    render(app);
    crate::dom::add_class("trScrim", "show");
}

/// The current tier's bank page — the pick-only lookup (24 chips/page).
fn render(app: &crate::App) {
    let s = app.borrow();
    let (lang, tier) = (s.lang.clone(), s.cur_tier.clone());
    let pool = crate::game::active_word_list(&s, &tier);
    let _ = &lang;
    drop(s);
    let mut html = String::new();

    if let Some(word) = CARD_WORD.with(|c| c.borrow().clone()) {
        html.push_str(&card_html(&word));
    } else {
        let page = PAGE.with(std::cell::Cell::get);
        let start = page * 24;
        html.push_str(&format!("<div class=\"rep-head\">{}</div><div>", t("tr.pick")));
        for entry in pool.iter().skip(start).take(24) {
            html.push_str(&format!(
                "<button class=\"ghost\" data-tr-word=\"{}\">{}</button>",
                crate::dom::escape_html(entry),
                crate::dom::escape_html(&crate::translate::display_word(entry))
            ));
        }
        html.push_str("</div><div class=\"cal-nav\">");
        if start > 0 {
            html.push_str("<button class=\"ghost\" data-tr-page=\"-1\">\u{2039}</button>");
        }
        if start + 24 < pool.len() {
            html.push_str("<button class=\"ghost\" data-tr-page=\"1\">\u{203A}</button>");
        }
        html.push_str("</div>");
    }
    crate::dom::set_html("trBody", &html);
}

fn card_html(entry: &str) -> String {
    let shown = if SHOW_ROMAN.with(std::cell::Cell::get) {
        crate::translate::transliteration(entry).map(|(_, r)| r)
    } else {
        None
    }
    .unwrap_or_else(|| crate::translate::display_word(entry));

    let mut html = format!(
        "<button class=\"ghost\" id=\"trBack\">\u{2039}</button>\
         <div class=\"tr-word\">{}</div>\
         <div class=\"gd-row\">\
         <button class=\"ghost\" data-tr-hear=\"{}\">{}</button>",
        crate::dom::escape_html(&shown),
        crate::dom::escape_html(entry),
        t("tr.hear"),
    );
    if crate::translate::transliteration(entry).is_some() {
        html.push_str(&format!("<button class=\"ghost\" id=\"trRoman\">{}</button>", t("tr.translit")));
    }
    html.push_str("</div>");

    // The fan-out seam: rows appear here when CC-BANK-TRANSLATE lands.
    // Today the resolver answers empty and rows are simply ABSENT.
    // (No placeholder, no lock chip, no tease — widget D3 precedent.)

    // Every card ends in doors (thesis 2): spell it now, or plan it.
    html.push_str(&format!(
        "<div class=\"gd-row\">\
         <button class=\"btn btn-check\" data-tr-spell=\"{0}\">{1}</button>\
         <button class=\"ghost\" data-tr-plan=\"{0}\">{2}</button></div>",
        crate::dom::escape_html(entry),
        t("tr.spellIt"),
        t("tr.addPlan"),
    ));
    html
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("trScrim") {
        return;
    }
    {
        let a = app.clone();
        crate::dom::on_click("trOpenBtn", move || open(&a));
    }
    crate::dom::on_click("trDone", || crate::dom::remove_class("trScrim", "show"));
    crate::dom::on::<web_sys::Event, _>("trScrim", "click", |e| {
        if crate::dom::is_self_target(&e, "trScrim") {
            crate::dom::remove_class("trScrim", "show");
        }
    });
    let a = app.clone();
    crate::dom::on::<web_sys::MouseEvent, _>("trBody", "click", move |e| {
        let Some(el) = e.target().and_then(|x| x.dyn_into::<web_sys::Element>().ok()) else { return };
        if let Some(w) = el.get_attribute("data-tr-word") {
            CARD_WORD.with(|c| *c.borrow_mut() = Some(w));
            SHOW_ROMAN.with(|c| c.set(false));
            render(&a);
        } else if let Some(n) = el.get_attribute("data-tr-page").and_then(|n| n.parse::<i64>().ok()) {
            PAGE.with(|c| c.set((c.get() as i64 + n).max(0) as usize));
            render(&a);
        } else if el.get_attribute("id").as_deref() == Some("trBack") {
            CARD_WORD.with(|c| *c.borrow_mut() = None);
            render(&a);
        } else if el.get_attribute("id").as_deref() == Some("trRoman") {
            SHOW_ROMAN.with(|c| c.set(!c.get()));
            render(&a);
        } else if let Some(entry) = el.get_attribute("data-tr-hear") {
            let lang = a.borrow().lang.clone();
            crate::api::play_word(&crate::translate::display_word(&entry), "normal", 1.0, &lang, || {});
        } else if let Some(entry) = el.get_attribute("data-tr-spell") {
            // Door 1: the REAL engine via the shared request door — scored
            // normally, zero translator session logic.
            crate::game::request_word(entry.clone());
            crate::dom::remove_class("trScrim", "show");
            if crate::dom::exists("practiceOpen") {
                crate::dom::click("practiceOpen");
            }
        } else if let Some(entry) = el.get_attribute("data-tr-plan") {
            // Door 2: tomorrow via the planner's OWN legality + honest
            // decline — no third outcome (acceptance #3).
            let lang = a.borrow().lang.clone();
            let today = (js_sys::Date::now() / 86_400_000.0) as u32;
            if crate::calendar::plan_word(&lang, today + 1, today, &entry) {
                crate::dom::show_toast(&t("tr.planned"));
            } else {
                crate::dom::show_toast(&t("cal.declined"));
            }
        }
    });
}
