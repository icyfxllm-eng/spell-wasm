//! BD-3 CC-GUARDIAN-DASH core surface. App-shell-only; view-only (no share
//! affordance in this wave); every read goes through ReportsQuery or the
//! learner's own report. The gate is a fresh worded-math challenge per
//! open — passing it never touches the agegate verdict.

use std::cell::Cell;

thread_local! {
    static GATE_ANSWER: Cell<i32> = const { Cell::new(-1) };
}

fn t(k: &str) -> String {
    crate::i18n::t(k)
}

fn section(title_key: &str, body: &str) -> String {
    format!("<div class=\"gd-sec\"><div class=\"gd-h\">{}</div>{}</div>", t(title_key), body)
}

fn render_body(app: &crate::App) {
    let s = app.borrow();
    let lang = s.lang.clone();
    let mut html = String::new();

    // Placement summary + mastery map, parent-grouped from the learner report.
    let st = crate::learner::load_for(&lang);
    let r = crate::learner::guardian_report(&st, crate::learner::current_day());
    html.push_str(&section("gdash.placement", &format!(
        "<div class=\"gd-row\">{}</div>",
        crate::i18n::tp("gdash.attempts", &[("c", &r.correct.to_string()), ("n", &r.attempts.to_string())])
    )));
    let group = |ids: &[String]| -> String {
        ids.iter().take(6).map(|id| format!("<span class=\"gd-chip\">{}</span>", t(&format!("skill.{id}")))).collect()
    };
    let dev: Vec<String> = r.skills.iter()
        .filter(|(id, m, _)| *m >= 0.45 && *m < 0.8 && !r.strengths.contains(id))
        .map(|(id, _, _)| id.clone()).collect();
    html.push_str(&section("gdash.mastery", &format!(
        "<div class=\"gd-row\"><b>{}</b> {}</div><div class=\"gd-row\"><b>{}</b> {}</div><div class=\"gd-row\"><b>{}</b> {}</div>",
        t("gdash.strong"), group(&r.strengths),
        t("gdash.dev"), group(&dev),
        t("gdash.focus"), group(&r.focus),
    )));

    // Trouble spots: the classifier's confusion pairs, most frequent first.
    let pairs = crate::reports::confusion_pairs(&lang);
    if !pairs.is_empty() {
        let rows: String = pairs.iter().take(5).map(|((a, b), _, n)| {
            format!("<div class=\"gd-row\">{}</div>",
                crate::i18n::tp("gdash.troubleRow", &[("a", &a.to_string()), ("b", &b.to_string()), ("n", &n.to_string())]))
        }).collect();
        html.push_str(&section("gdash.trouble", &rows));
    }

    // Correct but hesitant — the keystroke-timing capture's first surface.
    let shaky = crate::reports::hesitant_words(&lang, 5);
    if !shaky.is_empty() {
        let rows: String = shaky.iter()
            .map(|(w, _)| format!("<span class=\"gd-chip\">{}</span>", crate::dom::escape_html(w)))
            .collect();
        html.push_str(&section("gdash.shaky", &format!("<div class=\"gd-row\">{rows}</div>")));
    }

    // What's next: the due set, framed as the plan.
    let due = crate::reports::rematch_set(&s, &lang, js_sys::Date::now(), 36.0 * 3600.0 * 1000.0);
    html.push_str(&section("gdash.next", &format!(
        "<div class=\"gd-row\">{}</div>",
        crate::i18n::tp("gdash.nextRematch", &[("n", &due.len().to_string())])
    )));

    crate::family_voices::dash_section_into(&mut html);
    if let Some(g) = crate::calendar::active_goal(&lang) {
        let p = crate::calendar::goal_progress(&lang, &g).min(g.target);
        html.push_str(&cheer_section(&g, p, &lang));
    }
    crate::yearbook_ui::dash_section_into(&mut html);
    crate::dom::set_html("gdashBody", &html);
    crate::family_voices::fill_dash_section();
    crate::yearbook_ui::fill_dash_section(app);
    crate::dom::remove_class("gdashBody", "btn-hide");
    crate::dom::add_class("gdashGate", "btn-hide");
}

/// Read-and-cheer only (CAL feature 7 / I7): the goal renders read-only
/// with exactly one action — send a canned cheer. No create, no edit,
/// no delete, no free text. The gate greps this file to keep it true.
fn cheer_section(g: &crate::calendar::Goal, p: u32, lang: &str) -> String {
    let _ = lang;
    let mut html = format!(
        "<div class=\"gd-sec\"><div class=\"gd-h\">{}</div><div class=\"gd-row\">{} <b>{p}/{}</b></div><div class=\"gd-row\">",
        crate::i18n::t("cal.name"),
        crate::i18n::tp(g.kind.card_key(), &[("n", &g.target.to_string())]),
        g.target
    );
    for i in 0..crate::calendar::CHEER_POOL {
        html.push_str(&format!(
            "<button class=\"ghost\" data-gd-cheer=\"{i}\">{}</button>",
            crate::i18n::t(&format!("cal.cheer.{i}"))
        ));
    }
    html.push_str("</div></div>");
    html
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("gdash") {
        return;
    }
    crate::dom::on_click("gdashOpen", || {
        let (q, ans) = crate::agegate::parent_problem();
        GATE_ANSWER.with(|c| c.set(ans));
        crate::dom::set_text("gdashQ", &q);
        crate::dom::input("gdashAnswer").set_value("");
        crate::dom::set_text("gdashErr", "");
        crate::dom::add_class("gdashBody", "btn-hide");
        crate::dom::remove_class("gdashGate", "btn-hide");
        crate::dom::input("gdashAnswer").focus().ok();
    });
    let a = app.clone();
    {
        let a = app.clone();
        crate::dom::on::<web_sys::MouseEvent, _>("gdashBody", "click", move |e| {
            use wasm_bindgen::JsCast;
            let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            if let Some(i) = el.get_attribute("data-gd-cheer").and_then(|i| i.parse::<usize>().ok()) {
                let lang = a.borrow().lang.clone();
                crate::calendar::send_cheer(&lang, i);
                crate::dom::show_toast(&crate::i18n::t("cal.cheered"));
            }
        });
    }
    crate::dom::on_click("gdashGo", move || {
        let given: i32 = crate::dom::input("gdashAnswer").value().trim().parse().unwrap_or(-1);
        if given == GATE_ANSWER.with(|c| c.get()) && given >= 0 {
            render_body(&a);
        } else {
            crate::dom::set_text("gdashErr", &crate::i18n::t("gdash.wrong"));
        }
    });
}
