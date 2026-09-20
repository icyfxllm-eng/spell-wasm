//! CC-LEARNING-ENGINE-L0 R4 — the dev-only learner inspector.
//!
//! The gradable artifact: per language, the review queue with each word's
//! next-review date and interval, and WHY the word on screen was served.
//! Eric plays a session, sees the reasons, and says whether review timing
//! improved (acceptance 13).
//!
//! **Never in a player's build.** The whole module is behind the
//! `dev_preview` cargo feature — not a runtime flag — so release, auditor
//! and education builds do not contain it (acceptance 12). It also builds
//! its own button and panel at runtime, so `index.html` carries no markup
//! for it either; `scripts/seam-absence-check.mjs` greps the shipped
//! bundle for its marker.
//!
//! **Read-only** and contract-only: every number here comes from
//! `LearnerQuery` (I6). The inspector never writes learner state.

use crate::dom;
use crate::learner_query::{self, LearnerQuery, NextWordReason, PlacementStatus, DEVICE};
use crate::App;

/// The marker the absence scan greps for. Also the panel's element id.
pub const MARKER: &str = "spellLearnerInspector";

fn reason_label(r: NextWordReason) -> &'static str {
    match r {
        NextWordReason::DueReview => "due review (missed word came back)",
        NextWordReason::ToneDrill => "tone drill",
        NextWordReason::Placement => "placement probe",
        NextWordReason::Requested => "requested (a door asked for this word)",
        NextWordReason::Planned => "planned for today (calendar)",
        NextWordReason::LearnerPick => "learner pick (weakest/most due skill in band)",
        NextWordReason::Deck => "deck order (no-repeat shuffle)",
        NextWordReason::InOrder => "list order",
        NextWordReason::Daily => "daily challenge",
    }
}

fn placement_label(p: PlacementStatus) -> &'static str {
    match p {
        PlacementStatus::NotOffered => "not offered yet",
        PlacementStatus::Skipped => "skipped",
        PlacementStatus::Completed => "completed",
    }
}

/// Whole days since the epoch → "YYYY-MM-DD" (the yearbook's civil date).
fn date_of(ms: f64) -> String {
    let (y, m, d) = crate::yearbook::ymd_pub((ms / 86_400_000.0) as u32);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A human interval. Learning cards come back in minutes, so days alone
/// ("in 0 d") hides exactly what the dev pass is judging.
fn interval_label(due_ms: f64, now_ms: f64) -> String {
    let mins = ((due_ms - now_ms) / 60_000.0).round() as i64;
    match mins {
        m if m <= 0 && m > -60 => "due now".to_string(),
        m if m < 0 => {
            let over = -m;
            if over < 2880 { format!("{} h overdue", over / 60) } else { format!("{} d overdue", over / 1440) }
        }
        m if m < 90 => format!("in {m} min"),
        m if m < 2880 => format!("in {} h", m / 60),
        m => format!("in {} d", m / 1440),
    }
}

fn row(cells: &[String]) -> String {
    let tds: String = cells.iter().map(|c| format!("<td>{}</td>", dom::escape_html(c))).collect();
    format!("<tr>{tds}</tr>")
}

/// The panel's HTML for one language, from the contract only.
fn body(q: &dyn LearnerQuery, lang: &str, current_word: &str, now_ms: f64) -> String {
    let mut h = String::new();
    h.push_str(&format!("<h4>{} — learner inspector</h4>", dom::escape_html(lang)));

    // Why the word on screen was served.
    let why = match q.next_word_reason(DEVICE, current_word) {
        Some(r) => format!("<b>{}</b> — {}", dom::escape_html(current_word), reason_label(r)),
        None => "no word served yet in this session".to_string(),
    };
    h.push_str(&format!("<p style=\"margin:4px 0\">Serving: {why}</p>"));

    let stats = q.review_stats(DEVICE, lang);
    h.push_str(&format!(
        "<p style=\"margin:4px 0;opacity:.8\">{} attempts, {} correct · placement {} · {} tracked skills</p>",
        stats.attempts,
        stats.correct,
        placement_label(q.placement_status(DEVICE, lang)),
        q.skill_states(DEVICE, lang).len()
    ));

    // The queue: next review date, interval, and how the card got there.
    let sched = q.review_schedule(DEVICE, lang, 50);
    if sched.is_empty() {
        h.push_str("<p style=\"margin:4px 0;opacity:.8\">The review queue is empty for this language.</p>");
    } else {
        let rows: String = sched
            .iter()
            .map(|s| {
                row(&[
                    s.word.clone(),
                    date_of(s.due_ms),
                    interval_label(s.due_ms, now_ms),
                    if s.on_fsrs { "fsrs".into() } else { "learning".into() },
                    format!("{} miss / {} rev / {} lapse", s.misses, s.reps, s.lapses),
                ])
            })
            .collect();
        h.push_str(&format!(
            "<table style=\"width:100%;border-collapse:collapse;font-size:12px;margin:8px 0;white-space:nowrap;text-align:left\"><tr><th>word</th><th>next review</th><th>interval</th><th>phase</th><th>history</th></tr>{rows}</table>"
        ));
    }

    // Skills, weakest first: the other half of what decides the next word.
    let skills = q.skill_states(DEVICE, lang);
    if !skills.is_empty() {
        let rows: String = skills
            .iter()
            .map(|k| {
                row(&[
                    k.id.clone(),
                    format!("{:.2}", k.mastery),
                    if k.overdue_days > 0 { format!("{} d overdue", k.overdue_days) } else { format!("due in {} d", -k.overdue_days) },
                    format!("{} rev", k.reps),
                ])
            })
            .collect();
        h.push_str(&format!(
            "<table style=\"width:100%;border-collapse:collapse;font-size:12px;margin:8px 0;white-space:nowrap;text-align:left\"><tr><th>skill</th><th>mastery</th><th>due</th><th>reviews</th></tr>{rows}</table>"
        ));
    }
    h
}

fn render(app: &App) {
    let (lang, word) = {
        let s = app.borrow();
        (s.cur_lang.clone(), s.word.clone())
    };
    let html = {
        let s = app.borrow();
        body(&learner_query::live(&s), &lang, &word, js_sys::Date::now())
    };
    dom::set_html(MARKER, &format!(
        "<div style=\"max-height:80vh;overflow:auto;padding:14px;background:var(--panel);border-radius:14px\">{html}<button class=\"ghost\" id=\"{MARKER}Close\">Close</button></div>"
    ));
    dom::add_class(MARKER, "show");
    dom::on_click(&format!("{MARKER}Close"), || dom::remove_class(MARKER, "show"));
}

/// Build the panel and the dev-menu entry, then wire them. Called once at
/// startup, only in a `dev_preview` build.
pub fn wire(app: &App) {
    let doc = dom::doc();
    // The panel: created here, so no markup for it exists in index.html.
    if doc.get_element_by_id(MARKER).is_none() {
        if let (Ok(panel), Some(body_el)) = (doc.create_element("div"), doc.body()) {
            panel.set_id(MARKER);
            let _ = panel.set_attribute("class", "scrim");
            let _ = body_el.append_child(&panel);
        }
    }
    // The dev-menu entry, next to the other dev doors.
    let btn_id = format!("{MARKER}Open");
    if let Some(menu) = doc.query_selector("#devMenu .modal").ok().flatten() {
        if doc.get_element_by_id(&btn_id).is_none() {
            if let Ok(btn) = doc.create_element("button") {
                btn.set_id(&btn_id);
                let _ = btn.set_attribute("class", "ghost");
                btn.set_text_content(Some("Learner inspector"));
                let _ = menu.append_child(&btn);
            }
        }
    }
    let a = app.clone();
    dom::on_click(&btn_id, move || {
        dom::remove_class("devMenu", "show");
        render(&a);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learner_query::Fake;

    #[test]
    fn the_panel_shows_the_queue_dates_intervals_and_the_reason() {
        let mut f = Fake::mid_progress();
        f.reason = Some(("receive".into(), NextWordReason::DueReview));
        let h = body(&f, "en", "receive", 0.0); // receive is due now, knight tomorrow
        assert!(h.contains("due review"), "the reason for the served word: {h}");
        assert!(h.contains("receive") && h.contains("knight"), "every queued word: {h}");
        assert!(h.contains("due now") && h.contains("in 24 h"), "intervals in the unit that fits: {h}");
        assert!(h.contains("1970-01-02"), "a next-review date: {h}");
        assert!(h.contains("fsrs") && h.contains("learning"), "which phase each card is in: {h}");
        assert!(h.contains("silent_letters") && h.contains("0.31"), "skills with mastery: {h}");
        assert!(h.contains("40 attempts, 29 correct") && h.contains("placement completed"), "summary: {h}");
    }

    #[test]
    fn an_empty_queue_and_an_unserved_word_read_plainly() {
        let h = body(&Fake::default(), "es", "", 0.0);
        assert!(h.contains("no word served yet"), "{h}");
        assert!(h.contains("review queue is empty"), "{h}");
    }

    #[test]
    fn intervals_read_in_the_unit_that_matters() {
        let h = 3_600_000.0;
        assert_eq!(interval_label(100.0 * h, 100.0 * h), "due now");
        assert_eq!(interval_label(100.0 * h + 600_000.0, 100.0 * h), "in 10 min");
        assert_eq!(interval_label(100.0 * h + 5.0 * h, 100.0 * h), "in 5 h");
        assert_eq!(interval_label(100.0 * h + 96.0 * h, 100.0 * h), "in 4 d");
        assert_eq!(interval_label(100.0 * h - 30.0 * 60_000.0, 100.0 * h), "due now", "just missed is not overdue");
        assert_eq!(interval_label(100.0 * h - 5.0 * h, 100.0 * h), "5 h overdue");
        assert_eq!(interval_label(100.0 * h - 72.0 * h, 100.0 * h), "3 d overdue");
    }

    #[test]
    fn the_inspector_never_writes() {
        // R4 is read-only: the module may not touch a write door.
        // Only the module body: this test's own list of names would match.
        let whole = include_str!("inspector.rs");
        let src = &whole[..whole.find("#[cfg(test)]").expect("test module marker")];
        for door in ["note_attempt", "note_placement", "finish_placement", "storage::set", "learner::reset", "promote_miss", "add_miss"] {
            assert!(!src.contains(door), "the inspector must not call {door}");
        }
    }
}
