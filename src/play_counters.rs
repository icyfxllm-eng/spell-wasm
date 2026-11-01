//! Three lifetime counters the cosmos unlocks need and nothing else records
//! (CC-THEMES-COSMOS census C4): Impossible correct words (Diamond), sessions
//! (Beach) and days played (Aurora). All local, all never reset, none touching
//! Daily, the streak, The Climb or shields. Written at the seams that already
//! see every answer and every finished round; read by the unlock rules.

use crate::storage;

pub const IMPOSSIBLE_KEY: &str = "spell_impossible_correct_v1";
pub const SESSIONS_KEY: &str = "spell_sessions_v1";
pub const DAYS_KEY: &str = "spell_days_played_v1";

fn bump(key: &str) {
    let n = storage::get_raw(key).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    storage::set_raw(key, &n.saturating_add(1).to_string());
}

fn read(key: &str) -> u32 {
    storage::get_raw(key).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0)
}

/// A base-game correct answer at the Impossible level (Impossible or Mix),
/// not rescued by a retry. Review runs are practice and do not count.
pub fn note_impossible_correct() {
    bump(IMPOSSIBLE_KEY);
}

#[allow(dead_code)] // read by the Diamond unlock once that spec is built
pub fn impossible_correct() -> u32 {
    read(IMPOSSIBLE_KEY)
}

/// A session is one completed round: a solo run that ended with at least one
/// correct word, or a finished Daily (Eric's call on "session" for Beach is
/// pending; this is the recommended definition).
pub fn note_session() {
    bump(SESSIONS_KEY);
}

#[allow(dead_code)] // read by the Beach unlock once that spec is built
pub fn sessions() -> u32 {
    read(SESSIONS_KEY)
}

/// `date` (YYYY-MM-DD, local) added to a sorted, de-duplicated day list.
/// Returns whether the list changed.
pub fn add_day(days: &mut Vec<String>, date: &str) -> bool {
    match days.binary_search_by(|d| d.as_str().cmp(date)) {
        Ok(_) => false,
        Err(i) => {
            days.insert(i, date.to_string());
            true
        }
    }
}

/// The first-read seed: every day the Daily history already holds.
pub fn seed_days<'a>(daily_history_dates: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut days = Vec::new();
    for d in daily_history_dates {
        add_day(&mut days, d);
    }
    days
}

fn load_days() -> Vec<String> {
    if let Some(d) = storage::get_json::<Vec<String>>(DAYS_KEY) {
        return d;
    }
    let seeded = seed_days(crate::daily::load().history.keys());
    storage::set_json(DAYS_KEY, &seeded);
    seeded
}

/// A correct answer in any mode just happened today.
pub fn note_day_played() {
    let mut days = load_days();
    if add_day(&mut days, &crate::daily::today()) {
        storage::set_json(DAYS_KEY, &days);
    }
}

#[allow(dead_code)] // read by the Aurora unlock once that spec is built
pub fn days_played() -> u32 {
    load_days().len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_counted_once() {
        let mut d = Vec::new();
        assert!(add_day(&mut d, "2026-10-31"));
        assert!(!add_day(&mut d, "2026-10-31"));
        assert!(add_day(&mut d, "2026-10-30"));
        assert_eq!(d, ["2026-10-30", "2026-10-31"]);
    }

    #[test]
    fn the_seed_is_every_daily_history_date_once() {
        let h = vec!["2026-10-02".to_string(), "2026-10-01".to_string(), "2026-10-02".to_string()];
        assert_eq!(seed_days(h.iter()), ["2026-10-01", "2026-10-02"]);
        assert!(seed_days(Vec::<String>::new().iter()).is_empty());
    }

    #[test]
    fn a_seeded_day_is_not_counted_again_today() {
        let mut d = seed_days(["2026-10-31".to_string()].iter());
        assert!(!add_day(&mut d, "2026-10-31"));
        assert_eq!(d.len(), 1);
    }
}
