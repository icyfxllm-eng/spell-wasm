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

pub fn render(state: &AppState) {
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
        let pct = if d.seen > 0 { (100 * d.correct) / d.seen } else { 0 };
        let num = if d.seen > 0 {
            format!("<b>{}</b>/{} \u{b7} {}%", d.correct, d.seen, pct)
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
        dom::set_text("statsTotal", &format!("{}/{} correct overall \u{b7} {}%", total_correct, total_seen, pct));
    }
}

pub fn reset_current_lang(state: &mut AppState) {
    state.stats.remove(&state.lang);
    save(state);
    render(state);
}
