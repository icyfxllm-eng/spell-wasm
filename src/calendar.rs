//! CC-CALENDAR v2 — plans, goals, cheers: the policy layer.
//!
//! The two governing laws live here as code shape:
//! 1. The planner steers, the engine drives — `planned_pick` can only
//!    return an index INTO the band window it is handed, and it stands
//!    aside entirely while anything in the window is overdue (the
//!    scheduler's due reviews are never displaced; I2's order is FSRS
//!    due -> band-legal planned -> engine fill).
//! 2. Goals are quests, never debts — unmet goals roll forward silently
//!    (`week_hand` simply re-deals), nothing here stores or renders a
//!    failure state, and no goal is ever time-denominated (I3).
//!
//! I7 (CI-enforced): goals originate on kid surfaces only. The ONLY
//! goal-writing door is `select_goal`; the parent's single verb is
//! `send_cheer`. The gate greps guardian code for the kid-only symbols.

const PLAN_CAP_PER_DAY: usize = 5;

fn plan_key(lang: &str) -> String {
    format!("spell_plan_{lang}")
}
fn goal_key(lang: &str) -> String {
    format!("spell_goal_{lang}")
}
fn cheer_key(lang: &str) -> String {
    format!("spell_cheer_{lang}")
}

// ---------------- weeks (Sunday ritual => Sunday start) ----------------

/// Day-of-week for an epoch day, 0 = Sunday (1970-01-01 was a Thursday).
pub fn dow(day: u32) -> u32 {
    (day + 4) % 7
}

pub fn week_start(day: u32) -> u32 {
    day - dow(day)
}

// ---------------- plans ----------------

pub fn plans(lang: &str) -> Vec<(u32, Vec<String>)> {
    crate::storage::get_json(&plan_key(lang)).unwrap_or_default()
}

pub fn planned_for(lang: &str, day: u32) -> Vec<String> {
    plans(lang).into_iter().find(|(d, _)| *d == day).map(|(_, w)| w).unwrap_or_default()
}

/// Add a word to a FUTURE day's plan. Returns false (a decline the UI
/// must explain with the audited reason) when the day is not in the
/// future, the day is full, or the word is already planned. Band
/// legality is enforced at SESSION time by construction (`planned_pick`
/// only ever picks within the handed band) — and at planning time the UI
/// only offers words drawn from the kid's own live pools.
pub fn plan_word(lang: &str, day: u32, today: u32, word: &str) -> bool {
    if day <= today {
        return false;
    }
    let mut all = plans(lang);
    let slot = match all.iter_mut().find(|(d, _)| *d == day) {
        Some((_, w)) => w,
        None => {
            all.push((day, Vec::new()));
            &mut all.last_mut().unwrap().1
        }
    };
    if slot.len() >= PLAN_CAP_PER_DAY || slot.iter().any(|w| w == word) {
        return false;
    }
    slot.push(word.to_string());
    all.retain(|(_, w)| !w.is_empty());
    all.sort_by_key(|(d, _)| *d);
    crate::storage::set_json(&plan_key(lang), &all);
    true
}

pub fn unplan_word(lang: &str, day: u32, word: &str) {
    let mut all = plans(lang);
    for (d, w) in all.iter_mut() {
        if *d == day {
            w.retain(|x| x != word);
        }
    }
    all.retain(|(_, w)| !w.is_empty());
    crate::storage::set_json(&plan_key(lang), &all);
}

/// LAW 1's teeth. Called inside the deck's promote window: returns an
/// index into `win` for a word planned for `day`, but ONLY when nothing
/// in the window is overdue (due reviews are never displaced). Purely a
/// function of its arguments — deterministic, D7-style.
pub fn planned_pick(
    st: &crate::learner::LearnerState,
    win: &[String],
    lang: &str,
    day: u32,
) -> Option<usize> {
    let planned = planned_for(lang, day);
    if planned.is_empty() {
        return None;
    }
    for word in win {
        for id in crate::learner::hazards(lang, word) {
            if let Some(s) = st.skills.iter().find(|s| s.id == id) {
                if s.fsrs.reps > 0 && day > s.fsrs.due_day {
                    return None; // an overdue review outranks every plan
                }
            }
        }
    }
    win.iter().position(|w| planned.contains(w))
}

// ---------------- goals: the dealt hand ----------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum GoalKind {
    LearnNew,
    Rematch,
    ConquerBoss,
    Dailies,
}

impl GoalKind {
    pub fn card_key(self) -> &'static str {
        match self {
            GoalKind::LearnNew => "cal.card.learn",
            GoalKind::Rematch => "cal.card.rematch",
            GoalKind::ConquerBoss => "cal.card.conquer",
            GoalKind::Dailies => "cal.card.dailies",
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    pub kind: GoalKind,
    pub target: u32,
    pub week: u32, // week_start day
    #[serde(default)]
    pub done: bool,
}

pub fn active_goal(lang: &str) -> Option<Goal> {
    crate::storage::get_json(&goal_key(lang))
}

/// Deal this week's hand — deterministic from the kid's own live pools
/// (testable), never an impossible card, always at least one small win.
/// `conquer_pool`/`due_now` arrive from ReportsQuery at the call site so
/// this stays a pure function.
pub fn week_hand(conquer_pool: usize, due_now: usize) -> Vec<(GoalKind, u32)> {
    let mut hand = Vec::new();
    // D4 (signed): 5 new words per WEEK is the default quest.
    hand.push((GoalKind::LearnNew, 5));
    if due_now > 0 {
        hand.push((GoalKind::Rematch, due_now.min(5) as u32));
    }
    if conquer_pool >= 3 {
        hand.push((GoalKind::ConquerBoss, 3));
    }
    if hand.len() < 3 {
        // The guaranteed small win: three Daily Challenges.
        hand.push((GoalKind::Dailies, 3));
    }
    hand.truncate(3);
    hand
}

/// THE one goal-writing door (I7): a kid tap on a dealt card. One active
/// goal per language; re-picking replaces within the same week.
pub fn select_goal(lang: &str, kind: GoalKind, target: u32, today: u32) {
    let g = Goal { kind, target, week: week_start(today), done: false };
    crate::storage::set_json(&goal_key(lang), &g);
}

/// Progress is RECOMPUTED from the journal + daily history — no stored
/// counter to drift, nothing to migrate, quests can't corrupt.
pub fn goal_progress(lang: &str, goal: &Goal) -> u32 {
    let (lo, hi) = (goal.week, goal.week + 6);
    match goal.kind {
        GoalKind::LearnNew | GoalKind::ConquerBoss => {
            let mut n = 0u32;
            for (date, e) in crate::journal::read_all(lang) {
                if date_in_days(&date, lo, hi) {
                    n += e.mastered.len() as u32;
                }
            }
            n
        }
        GoalKind::Rematch => {
            let mut n = 0u32;
            for (date, e) in crate::journal::read_all(lang) {
                if date_in_days(&date, lo, hi) {
                    n += e.practiced.len() as u32;
                }
            }
            n
        }
        GoalKind::Dailies => crate::daily::load()
            .history
            .keys()
            .filter(|d| date_in_days(d, lo, hi))
            .count() as u32,
    }
}

/// Completion check; fires the EXISTING ceremony (D6 — no new economy)
/// exactly once per goal.
pub fn check_goal_done(lang: &str) -> bool {
    let Some(mut g) = active_goal(lang) else { return false };
    if g.done {
        return false;
    }
    if goal_progress(lang, &g) >= g.target {
        g.done = true;
        crate::storage::set_json(&goal_key(lang), &g);
        crate::audio_boost::chime();
        return true;
    }
    false
}

fn date_in_days(date: &str, lo: u32, hi: u32) -> bool {
    let mut it = date.split('-');
    let (Some(y), Some(m), Some(d)) = (
        it.next().and_then(|s| s.parse::<i64>().ok()),
        it.next().and_then(|s| s.parse::<u32>().ok()),
        it.next().and_then(|s| s.parse::<u32>().ok()),
    ) else {
        return false;
    };
    let day = crate::yearbook::day_of_pub(y, m, d);
    day >= lo && day <= hi
}

// ---------------- cheers (the parent's one verb) ----------------

pub const CHEER_POOL: usize = 5; // hard cap — cal.cheer.0..4, audited

pub fn active_cheer(lang: &str) -> Option<usize> {
    crate::storage::get_json(&cheer_key(lang))
}

/// Max one active cheer per goal; sending again replaces the line.
/// Canned index only — no free text exists in this feature, ever.
pub fn send_cheer(lang: &str, line: usize) {
    crate::storage::set_json(&cheer_key(lang), &(line % CHEER_POOL));
}

pub fn clear_cheer(lang: &str) {
    // storage has no remove; a JSON null deserializes to no usize, which
    // is exactly what active_cheer's Option contract wants.
    crate::storage::set_raw(&cheer_key(lang), "null");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week_starts_sunday() {
        // 1970-01-01 (day 0) was a Thursday; day 3 was the first Sunday.
        assert_eq!(dow(3), 0);
        assert_eq!(week_start(3), 3);
        assert_eq!(week_start(9), 3, "Friday belongs to Sunday-the-3rd's week");
        assert_eq!(week_start(10), 10, "the next Sunday opens a new week");
    }

    #[test]
    fn the_hand_always_deals_and_never_deals_impossible() {
        // Empty pools: LearnNew + the small win, never a boss card.
        let hand = week_hand(0, 0);
        assert!(hand.iter().any(|(k, _)| *k == GoalKind::LearnNew));
        assert!(hand.iter().any(|(k, _)| *k == GoalKind::Dailies));
        assert!(!hand.iter().any(|(k, _)| *k == GoalKind::ConquerBoss));
        // Rich pools: three cards, boss included, target capped.
        let hand = week_hand(8, 12);
        assert_eq!(hand.len(), 3);
        assert!(hand.iter().any(|(k, t)| *k == GoalKind::Rematch && *t == 5));
        assert!(hand.iter().any(|(k, _)| *k == GoalKind::ConquerBoss));
        // Determinism: same pools, same hand.
        assert_eq!(week_hand(8, 12), week_hand(8, 12));
    }

    #[test]
    fn learn_new_default_is_five_per_week() {
        // D4 as signed: the unit is the WEEK.
        assert!(week_hand(0, 0).iter().any(|(k, t)| *k == GoalKind::LearnNew && *t == 5));
    }
}
