//! CC-CALENDAR v2 — the kid surface. App-shell-only (dom::exists), all
//! strings from the audited cal.* pool (conquest register, shared I4
//! discipline). No X's, no gray shame states, no failure renders: an
//! unplayed past day is simply empty (law 2).

use wasm_bindgen::JsCast;

use crate::calendar::{
    active_cheer, active_goal, check_goal_done, goal_progress, plan_word, planned_for, plans,
    select_goal, unplan_word, week_hand, week_start, GoalKind,
};

thread_local! {
    static VIEW_MONTH: std::cell::Cell<(i64, u32)> = const { std::cell::Cell::new((0, 0)) };
    static SEL_DAY: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn t(k: &str) -> String {
    crate::i18n::t(k)
}
fn tp1(k: &str, key: &str, v: &str) -> String {
    crate::i18n::tp(k, &[(key, v)])
}

fn today() -> u32 {
    (js_sys::Date::now() / 86_400_000.0) as u32
}

fn date_str(day: u32) -> String {
    let (y, m, d) = crate::yearbook::ymd_pub(day);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn open(app: &crate::App) {
    if !crate::dom::exists("calScrim") {
        return;
    }
    let (y, m, _) = crate::yearbook::ymd_pub(today());
    VIEW_MONTH.with(|c| c.set((y, m)));
    SEL_DAY.with(|c| c.set(0));
    render(app);
    crate::dom::add_class("calScrim", "show");
}

fn render(app: &crate::App) {
    let lang = app.borrow().lang.clone();
    let today = today();
    check_goal_done(&lang);
    let mut html = String::new();

    // ---- header: goal ring (or this week's dealt hand) + cheer card
    if let Some(g) = active_goal(&lang) {
        if g.week == week_start(today) || g.done {
            let p = goal_progress(&lang, &g).min(g.target);
            html.push_str(&format!(
                "<div class=\"cal-goal\"><span class=\"cal-ring\">{}</span> {} <b>{p}/{}</b>{}</div>",
                if g.done { "\u{2B50}" } else { "\u{25D0}" },
                tp1(g.kind.card_key(), "n", &g.target.to_string()),
                g.target,
                if g.done { format!(" {}", t("cal.done")) } else { String::new() }
            ));
        }
    }
    let deal_needed = match active_goal(&lang) {
        Some(g) => g.week != week_start(today),
        None => true,
    };
    if deal_needed {
        // The dealt hand (feature 4): 2–3 cards from the kid's own pools;
        // an unmet last-week goal simply gets re-dealt — no residue.
        let s = app.borrow();
        let conquer = crate::reports::words_to_conquer(&s, &lang, 12).len();
        let due = crate::reports::rematch_set(&s, &lang, js_sys::Date::now(), 36.0 * 3600.0 * 1000.0).len();
        drop(s);
        html.push_str(&format!("<div class=\"cal-goal\">{}</div><div class=\"cal-hand\">", t("cal.pick")));
        for (kind, target) in week_hand(conquer, due) {
            html.push_str(&format!(
                "<button class=\"cal-card\" data-cal-goal=\"{:?}\" data-cal-target=\"{target}\">{}</button>",
                kind,
                tp1(kind.card_key(), "n", &target.to_string())
            ));
        }
        html.push_str("</div>");
    }
    if let Some(line) = active_cheer(&lang) {
        html.push_str(&format!(
            "<div class=\"cal-cheer\">\u{1F4E3} {}</div>",
            t(&format!("cal.cheer.{line}"))
        ));
    }

    // ---- month grid
    let (vy, vm) = VIEW_MONTH.with(std::cell::Cell::get);
    let first = crate::yearbook::day_of_pub(vy, vm, 1);
    let days_in_month = {
        let (ny, nm) = if vm == 12 { (vy + 1, 1) } else { (vy, vm + 1) };
        crate::yearbook::day_of_pub(ny, nm, 1) - first
    };
    html.push_str(&format!(
        "<div class=\"cal-nav\"><button class=\"ghost\" data-cal-nav=\"-1\">\u{2039}</button>\
         <b>{vy}-{vm:02}</b><button class=\"ghost\" data-cal-nav=\"1\">\u{203A}</button></div>\
         <div class=\"cal-grid\">"
    ));
    for _ in 0..crate::calendar::dow(first) {
        html.push_str("<span></span>");
    }
    for i in 0..days_in_month {
        let day = first + i;
        let date = date_str(day);
        let mut dots = String::new();
        if let Some(e) = crate::journal::entry_for(&lang, &date) {
            if !e.practiced.is_empty() {
                dots.push('\u{25CF}');
            }
            if !e.mastered.is_empty() {
                dots.push('\u{2B50}');
            }
        }
        // Future plans render WRAPPED — anticipation, never obligation.
        let planned = planned_for(&lang, day).len();
        if day > today && planned > 0 {
            dots.push('\u{1F381}');
        }
        let cls = if day == today { "cal-cell today" } else { "cal-cell" };
        html.push_str(&format!(
            "<button class=\"{cls}\" data-cal-day=\"{day}\">{}<small>{dots}</small></button>",
            i + 1
        ));
    }
    html.push_str("</div><div id=\"calDetail\"></div>");
    crate::dom::set_html("calBody", &html);

    let sel = SEL_DAY.with(std::cell::Cell::get);
    if sel != 0 {
        render_detail(app, sel);
    }
}

/// Past day -> Recap; today -> the session launcher; future -> Planner.
fn render_detail(app: &crate::App, day: u32) {
    let lang = app.borrow().lang.clone();
    let today = today();
    let date = date_str(day);
    let mut html = String::new();

    if day < today {
        // Recap: the day's story, rows are doors (shared I4 pool).
        match crate::journal::entry_for(&lang, &date) {
            Some(e) => {
                let sec = |title: &str, words: &[String], door: bool| {
                    if words.is_empty() {
                        return String::new();
                    }
                    let rows: String = words
                        .iter()
                        .map(|w| {
                            if door {
                                format!(
                                    "<div class=\"rep-row door\" data-cal-door=\"{}\"><span>{}</span><span>\u{25B6}</span></div>",
                                    crate::dom::escape_html(w),
                                    crate::dom::escape_html(w)
                                )
                            } else {
                                format!("<div class=\"rep-row\"><span>{}</span></div>", crate::dom::escape_html(w))
                            }
                        })
                        .collect();
                    format!("<div class=\"rep-head\">{title}</div>{rows}")
                };
                html.push_str(&sec(&t("cal.practiced"), &e.practiced, false));
                html.push_str(&sec(&t("cal.missedThatDay"), &e.missed, true));
                html.push_str(&sec(&t("cal.masteredThatDay"), &e.mastered, false));
            }
            None => html.push_str(&format!("<div class=\"rep-empty\">{}</div>", t("cal.quietDay"))),
        }
    } else if day == today {
        html.push_str(&format!(
            "<button class=\"btn btn-check\" id=\"calPlayToday\">{}</button>",
            t("cal.playToday")
        ));
    } else {
        // Planner. D2 (signed): beyond the current week is Complete.
        let in_free_week = week_start(day) == week_start(today);
        // AUDITPASS F12 — Spell Jr promises "no prices", and this was the
        // one kid-reachable surface that broke it. Calendar is
        // kidSafe:true, so a child looking past the current week met
        // "Unlocks with Complete" — an upsell, on the surface whose own
        // settings row says there are none.
        //
        // modes.rs already states the Little Speller zero-purchase-surface
        // doctrine and a playhub spec enforces it FOR TILES; nothing
        // extended it to copy rendered inside a surface, which is exactly
        // how this survived. In Kid Mode the planner shows ABSENCE — the
        // same "absent, never locked" rule the hub follows.
        let kid = crate::dom::doc()
            .body()
            .map(|b| b.class_list().contains("kid"))
            .unwrap_or(false);
        if !in_free_week && !crate::play_hub::live_entitlements().progress_reports {
            if !kid {
                html.push_str(&format!("<div class=\"gd-chip\">{}</div>", t("yb.locked")));
            }
        } else {
            let planned = planned_for(&lang, day);
            if !planned.is_empty() {
                html.push_str(&format!("<div class=\"rep-head\">{}</div>", tp1("cal.wrapped", "n", &planned.len().to_string())));
                for w in &planned {
                    html.push_str(&format!(
                        "<div class=\"rep-row\"><span>{}</span><button class=\"ghost\" data-cal-unplan=\"{}\">\u{2715}</button></div>",
                        crate::dom::escape_html(w),
                        crate::dom::escape_html(w)
                    ));
                }
            }
            // Sources: the kid's own pools only (no free text anywhere).
            let s = app.borrow();
            let mut sources: Vec<String> = crate::reports::rematch_set(&s, &lang, js_sys::Date::now(), 36.0 * 3600.0 * 1000.0);
            sources.extend(crate::reports::words_to_conquer(&s, &lang, 12).into_iter().map(|(w, _)| w));
            drop(s);
            sources.dedup();
            if !sources.is_empty() {
                html.push_str(&format!("<div class=\"rep-head\">{}</div><div>", t("cal.plan")));
                for w in sources.iter().take(10) {
                    html.push_str(&format!(
                        "<button class=\"ghost\" data-cal-plan=\"{}\">{}</button>",
                        crate::dom::escape_html(w),
                        crate::dom::escape_html(w)
                    ));
                }
                html.push_str("</div>");
            }
        }
    }
    crate::dom::set_html("calDetail", &html);
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("calScrim") {
        return;
    }
    {
        let a = app.clone();
        crate::dom::on_click("calOpenBtn", move || open(&a));
    }
    crate::dom::on_click("calDone", || crate::dom::remove_class("calScrim", "show"));
    crate::dom::on::<web_sys::Event, _>("calScrim", "click", |e| {
        if crate::dom::is_self_target(&e, "calScrim") {
            crate::dom::remove_class("calScrim", "show");
        }
    });
    let a = app.clone();
    crate::dom::on::<web_sys::MouseEvent, _>("calBody", "click", move |e| {
        let Some(el) = e.target().and_then(|x| x.dyn_into::<web_sys::Element>().ok()) else { return };
        let el = match el.closest("[data-cal-day],[data-cal-goal],[data-cal-nav],[data-cal-plan],[data-cal-unplan],[data-cal-door],#calPlayToday").ok().flatten() {
            Some(e) => e,
            None => return,
        };
        let lang = a.borrow().lang.clone();
        if let Some(d) = el.get_attribute("data-cal-day").and_then(|d| d.parse::<u32>().ok()) {
            SEL_DAY.with(|c| c.set(d));
            render_detail(&a, d);
        } else if let Some(kind) = el.get_attribute("data-cal-goal") {
            let target: u32 = el.get_attribute("data-cal-target").and_then(|t| t.parse().ok()).unwrap_or(5);
            let kind = match kind.as_str() {
                "Rematch" => GoalKind::Rematch,
                "ConquerBoss" => GoalKind::ConquerBoss,
                "Dailies" => GoalKind::Dailies,
                _ => GoalKind::LearnNew,
            };
            // I7: THE goal-writing door — a kid tap on a dealt card.
            select_goal(&lang, kind, target, today());
            crate::calendar::clear_cheer(&lang); // a new quest retires the old cheer
            render(&a);
        } else if let Some(n) = el.get_attribute("data-cal-nav").and_then(|n| n.parse::<i64>().ok()) {
            VIEW_MONTH.with(|c| {
                let (mut y, mut m) = c.get();
                let mm = m as i64 + n;
                if mm < 1 {
                    y -= 1;
                    m = 12;
                } else if mm > 12 {
                    y += 1;
                    m = 1;
                } else {
                    m = mm as u32;
                }
                c.set((y, m));
            });
            SEL_DAY.with(|c| c.set(0));
            render(&a);
        } else if let Some(w) = el.get_attribute("data-cal-plan") {
            let day = SEL_DAY.with(std::cell::Cell::get);
            if !plan_word(&lang, day, today(), &w) {
                // One honest audited reason, never a silent drop.
                crate::dom::show_toast(&t("cal.declined"));
            }
            render_detail(&a, day);
            render(&a);
        } else if let Some(w) = el.get_attribute("data-cal-unplan") {
            let day = SEL_DAY.with(std::cell::Cell::get);
            unplan_word(&lang, day, &w);
            render_detail(&a, day);
        } else if el.get_attribute("data-cal-door").is_some() {
            crate::dom::remove_class("calScrim", "show");
            crate::game::enter_review(&a);
        } else if el.get_attribute("id").as_deref() == Some("calPlayToday") {
            crate::dom::remove_class("calScrim", "show");
            if crate::dom::exists("practiceOpen") {
                crate::dom::click("practiceOpen");
            }
        }
    });
}
