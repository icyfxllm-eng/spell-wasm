//! CC-REPORTS kid Quest Log — render + doors. App-shell-only (dom::exists);
//! reads ONLY through ReportsQuery (I1 lives there, not here). I4: every
//! string comes from the lint-audited reports.* pool; every row is a door,
//! and today every door opens smart review (the DEEP_LINKS interim until
//! CC-TRAP-BOSSES).

use wasm_bindgen::JsCast;

fn pips(n: u8) -> String {
    // Capped at 4 upstream by construction; belt-and-suspenders here.
    let n = n.min(4) as usize;
    format!("{}{}", "\u{25CF}".repeat(n), "\u{25CB}".repeat(4 - n))
}

fn render(app: &crate::App) {
    let s = app.borrow();
    let lang = s.lang.clone();

    let mut html = String::new();
    for (word, p) in crate::reports::words_to_conquer(&s, &lang, 12) {
        html.push_str(&format!(
            "<div class=\"rep-row door\" data-door=\"conquer\"><span>{}</span><span class=\"rep-pips\">{}</span></div>",
            crate::dom::escape_html(&word), pips(p)
        ));
    }
    if html.is_empty() {
        html = format!("<div class=\"rep-empty\">{}</div>", crate::i18n::t("reports.empty"));
    }
    crate::dom::set_html("repConquer", &html);

    let mut html = String::new();
    for word in crate::learner_query::LearnerQuery::at_risk_set(&crate::learner_query::live(&s), crate::learner_query::DEVICE, &lang, 1.5) {
        html.push_str(&format!(
            "<div class=\"rep-row door\" data-door=\"rematch\"><span>{}</span><span>\u{25B6}</span></div>",
            crate::dom::escape_html(&word)
        ));
    }
    if html.is_empty() {
        html = format!("<div class=\"rep-empty\">{}</div>", crate::i18n::t("reports.empty"));
    }
    crate::dom::set_html("repRematch", &html);

    let mut html = String::new();
    for r in crate::reports::redemptions(&lang).iter().rev().take(8) {
        html.push_str(&format!(
            "<div class=\"rep-row\"><span>{}</span><span>\u{2B50}</span></div>",
            crate::dom::escape_html(&r.word)
        ));
    }
    if html.is_empty() {
        html = format!("<div class=\"rep-empty\">{}</div>", crate::i18n::t("reports.empty"));
    }
    crate::dom::set_html("repRedeemed", &html);
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("repScrim") {
        return;
    }
    let a = app.clone();
    crate::dom::on_click("repOpenBtn", move || {
        render(&a);
        crate::dom::add_class("repScrim", "show");
    });
    crate::dom::on_click("repDone", || crate::dom::remove_class("repScrim", "show"));
    crate::dom::on::<web_sys::Event, _>("repScrim", "click", |e| {
        if crate::dom::is_self_target(&e, "repScrim") {
            crate::dom::remove_class("repScrim", "show");
        }
    });
    // Doors: any .door row -> close the log, enter smart review. The row's
    // word is IN the misses set by construction (both queries derive from
    // it), so review is the honest target, not a detour.
    let a = app.clone();
    crate::dom::on::<web_sys::MouseEvent, _>("repScrim", "click", move |e| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if el.closest(".door").ok().flatten().is_some() {
            crate::dom::remove_class("repScrim", "show");
            crate::game::enter_review(&a);
        }
    });
}
