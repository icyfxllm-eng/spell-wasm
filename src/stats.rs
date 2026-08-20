use crate::consts::TIER_ORDER;
use crate::dom;
use crate::game::name_for;
use crate::model::{AppState, TierStat, STATS_KEY};
use crate::storage;

pub fn load(state: &mut AppState) {
    state.stats = storage::get_json(STATS_KEY).unwrap_or_default();
}

fn save(state: &AppState) {
    storage::set_json(STATS_KEY, &state.stats);
}

pub fn record(state: &mut AppState, lang: &str, tier: &str, correct: bool) {
    let lang_stats = state.stats.entry(lang.to_string()).or_default();
    let t = lang_stats.entry(tier.to_string()).or_default();
    t.seen += 1;
    if correct {
        t.correct += 1;
    }
    save(state);
    render(state);
}

/// CC-ZH-TONE F3: a tone-only miss is an attempt like any other, but it is
/// neither correct nor a plain miss. Separate from `record` so `seen` is
/// incremented exactly once and `correct` is never inflated.
pub fn record_tone_only(state: &mut AppState, lang: &str, tier: &str) {
    let lang_stats = state.stats.entry(lang.to_string()).or_default();
    let t = lang_stats.entry(tier.to_string()).or_default();
    t.seen += 1;
    t.tone_partial += 1;
    save(state);
    render(state);
}

pub fn render(state: &AppState) {
    // CC-LEARNING-ENGINE L2 — the guardian section, view-only (sharing
    // would be an explicit affordance; v1 has none). Dark by default.
    if crate::dom::exists("statsGuardian") && crate::flags::learner_surfaces() {
        let st = crate::learner::load_for(&state.lang);
        let r = crate::learner::guardian_report(&st, crate::learner::current_day());
        dom::set_html("statsGuardian", &crate::learner::guardian_report_html(&r, &state.lang));
        dom::remove_class("statsGuardian", "btn-hide");
    } else if crate::dom::exists("statsGuardian") {
        dom::add_class("statsGuardian", "btn-hide");
    }
    // Achievements live on the stats screen and must re-render here so a UI
    // language change (which calls stats::render) refreshes their titles too —
    // otherwise they keep the boot locale while the headers switch (the
    // mixed-language bug).
    crate::achievements::render(state);
    dom::set_text("statsLang", &name_for(state, &state.lang));
    let empty = TierStat::default();
    let by_tier = state.stats.get(&state.lang);
    let mut total_seen = 0u32;
    let mut total_correct = 0u32;
    let mut rows = String::new();
    for tier in TIER_ORDER {
        let d = by_tier.and_then(|m| m.get(tier)).unwrap_or(&empty);
        total_seen += d.seen;
        total_correct += d.correct;
        // Credited, not raw: a tone-only miss earns a fraction (F3). The
        // numerator shown stays the honest whole-correct count, and the tone
        // credit is named next to it rather than silently inflating the pair.
        let pct = if d.seen > 0 {
            (100.0 * d.credited() / d.seen as f64).round() as u32
        } else {
            0
        };
        let num = if d.seen > 0 {
            {
                let mut cell = format!("<b>{}</b>/{} \u{b7} {}%", d.correct, d.seen, pct);
                if d.tone_partial > 0 {
                    cell.push_str(&format!(
                        " <span class=\"tone-credit\">({})</span>",
                        crate::i18n::tp("stats.toneCredit", &[("n", &d.tone_partial.to_string())])
                    ));
                }
                cell
            }
        } else {
            "\u{2014}".to_string()
        };
        rows.push_str(&format!(
            "<div class=\"srow\"><span class=\"stier\">{}</span><div class=\"sbar\"><div class=\"sfill\" style=\"width:{}%\"></div></div><span class=\"snum\">{}</span></div>",
            tier,
            if d.seen > 0 { pct } else { 0 },
            num
        ));
    }
    if total_seen == 0 {
        dom::set_html("statsBody", &format!("<div class=\"stats-empty\">{}</div>", crate::i18n::t("card.accuracyEmpty")));
        dom::set_text("statsTotal", "");
    } else {
        dom::set_html("statsBody", &rows);
        let pct = (100 * total_correct) / total_seen;
        dom::set_text(
            "statsTotal",
            &crate::i18n::tp(
                "card.overall",
                &[
                    ("c", &total_correct.to_string()),
                    ("s", &total_seen.to_string()),
                    ("p", &pct.to_string()),
                ],
            ),
        );
    }
}

pub fn reset_current_lang(state: &mut AppState) {
    state.stats.remove(&state.lang);
    save(state);
    render(state);
}
