use crate::consts::ACHIEVEMENTS;
use crate::dom::{self, escape_html};
use crate::model::{AppState, ACH_KEY};
use crate::storage;

pub fn load(state: &mut AppState) {
    if let Some(a) = storage::get_json(ACH_KEY) {
        state.achievements = a;
    }
}

fn save(state: &AppState) {
    storage::set_json(ACH_KEY, &state.achievements);
}

pub fn has(state: &AppState, id: &str) -> bool {
    state.achievements.unlocked.iter().any(|x| x == id)
}

/// Unlocks the achievement if not already unlocked; shows a toast and
/// re-renders the grid when it's newly earned.
pub fn unlock(state: &mut AppState, id: &str) {
    if has(state, id) {
        return;
    }
    state.achievements.unlocked.push(id.to_string());
    save(state);
    render(state);
    if let Some(a) = ACHIEVEMENTS.iter().find(|a| a.id == id) {
        dom::show_toast(&crate::i18n::t(&format!("ach.{}.nm", a.id)));
    }
}

pub fn check_streak(state: &mut AppState) {
    unlock(state, "first");
    if state.streak >= 5 {
        unlock(state, "chain5");
    }
    if state.streak >= 10 {
        unlock(state, "chain10");
    }
    if state.streak >= 25 {
        unlock(state, "chain25");
    }
    if state.timed && state.streak >= 10 {
        unlock(state, "timed10");
    }
}

pub fn render(state: &AppState) {
    dom::set_text("achCount", &format!("{}/{}", state.achievements.unlocked.len(), ACHIEVEMENTS.len()));
    let html: String = ACHIEVEMENTS
        .iter()
        .map(|a| {
            let got = has(state, a.id);
            let nm = crate::i18n::t(&format!("ach.{}.nm", a.id));
            let desc = crate::i18n::t(&format!("ach.{}.desc", a.id));
            format!(
                "<div class=\"ach-badge {}\" title=\"{}\"><span class=\"ic\">{}</span><span class=\"nm\">{}</span></div>",
                if got { "got" } else { "" },
                escape_html(&desc),
                a.ic,
                escape_html(&nm)
            )
        })
        .collect();
    dom::set_html("achGrid", &html);
}
