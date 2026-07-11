//! Daily Challenge (§4.3): a fixed set of words seeded by (date, locale), so
//! everyone playing the same language on the same day gets the exact same run —
//! one attempt per word, no retries, isolated from the adaptive selector, Misses,
//! word stats, chain streak, and The Climb. A per-day result + a consecutive-day
//! streak are persisted locally; the daily set itself is recomputed on the fly
//! (deterministic, so it never needs storing) and works offline once the audio
//! for those words is cached.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::consts::{is_builtin_lang, EN};
use crate::{storage, words};

/// How many words a daily run contains, and its difficulty arc.
const ARC: [(&str, usize); 4] = [("easy", 2), ("medium", 3), ("hard", 3), ("expert", 2)];
const KID_ARC: [(&str, usize); 2] = [("easy", 5), ("medium", 5)];

const KEY: &str = "spell_daily_v1";

/// Session-only run state (the persisted streak/history lives in `Record`).
#[derive(Default)]
pub struct DailyState {
    pub active: bool,
    pub locale: String,
    pub date: String,
    pub words: Vec<String>,
    /// How many words have been served so far (0..=words.len()).
    pub idx: usize,
    pub correct: u32,
}

/// Persisted daily progress.
#[derive(Serialize, Deserialize, Default)]
pub struct Record {
    #[serde(default)]
    pub last_completed: String,
    #[serde(default)]
    pub streak: u32,
    #[serde(default)]
    pub best_streak: u32,
    #[serde(default)]
    pub history: HashMap<String, u32>, // date -> correct count
}

pub fn load() -> Record {
    storage::get_json(KEY).unwrap_or_default()
}

fn save(r: &Record) {
    storage::set_json(KEY, r);
}

fn ymd(d: &js_sys::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

/// Local calendar date, YYYY-MM-DD.
pub fn today() -> String {
    ymd(&js_sys::Date::new_0())
}

fn yesterday() -> String {
    let d = js_sys::Date::new_0();
    d.set_time(d.get_time() - 86_400_000.0);
    ymd(&d)
}

/// Which locale a daily run uses: the active built-in language, else English
/// (My Words / unknown langs don't have a shared daily set).
pub fn locale_for(lang: &str) -> String {
    if is_builtin_lang(lang) {
        lang.to_string()
    } else {
        EN.to_string()
    }
}

pub fn is_done_today() -> bool {
    load().history.contains_key(&today())
}

/// Today's score if already played, else None.
pub fn today_score() -> Option<u32> {
    load().history.get(&today()).copied()
}

// ---- deterministic PRNG (splitmix64 over an FNV-1a seed of "date:locale") ----

fn seed(date: &str, locale: &str, kid: bool) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in date.bytes().chain(b":".iter().copied()).chain(locale.bytes()).chain(if kid { b":kid".iter().copied() } else { b"".iter().copied() }) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Deterministically pick `count` distinct words from `pool` using `rng`
/// (partial Fisher–Yates), preserving order of selection.
fn pick(pool: &[&str], count: usize, rng: &mut u64) -> Vec<String> {
    let n = pool.len();
    if n == 0 {
        return Vec::new();
    }
    let take = count.min(n);
    let mut idx: Vec<usize> = (0..n).collect();
    for i in 0..take {
        let j = i + (next_u64(rng) as usize) % (n - i);
        idx.swap(i, j);
    }
    idx[..take].iter().map(|&i| pool[i].to_string()).collect()
}

/// Arc position (0-based) where the Expert finale begins: slots 9 and 10 (idx 8,
/// 9) are the "boss words" that get the Expert badge.
pub const FINALE_START: usize = 8;

/// Build today's word set for a language. Deterministic for a given
/// (date, locale, kid) triple. Words within each tier are ordered easiest→
/// hardest (length proxy until the scoring pipeline ships per-word scores) so
/// slots 1-8 ramp smoothly; slots 9-10 come from the Expert tier as the finale.
///
/// The spec's 90/180-day rolling exclusion is intentionally NOT applied yet: the
/// current ~40-word tiers are too small to honor it (the Expert pool would
/// exhaust in ~20 days), and a wall-clock-based exclusion would break
/// determinism. It lands once the difficulty pipeline expands the pools.
pub fn build_words(lang: &str, date: &str, kid: bool) -> (String, Vec<String>) {
    let locale = locale_for(lang);
    let mut rng = seed(date, &locale, kid);
    let arc: &[(&str, usize)] = if kid { &KID_ARC } else { &ARC };
    let mut out = Vec::new();
    for (tier, count) in arc {
        let mut picked = pick(words::tier_for(&locale, tier), *count, &mut rng);
        // Ramp within the tier: easiest first (shorter = easier proxy for now).
        picked.sort_by_key(|w| w.chars().count());
        out.extend(picked);
    }
    (locale, out)
}

/// Record a finished run, update the consecutive-day streak, and return
/// `(streak, best_streak)`. Idempotent for a given day (re-finishing keeps the
/// first score/streak).
pub fn record_result(date: &str, correct: u32) -> (u32, u32) {
    let mut r = load();
    if r.history.contains_key(date) {
        return (r.streak, r.best_streak);
    }
    r.streak = if r.last_completed == yesterday() { r.streak + 1 } else { 1 };
    r.best_streak = r.best_streak.max(r.streak);
    r.last_completed = date.to_string();
    r.history.insert(date.to_string(), correct);
    save(&r);
    (r.streak, r.best_streak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_per_date_locale() {
        let (_, a) = build_words("en", "2026-07-10", false);
        let (_, b) = build_words("en", "2026-07-10", false);
        assert_eq!(a, b, "same date+locale must yield identical words");
        assert_eq!(a.len(), 10);
    }

    #[test]
    fn locale_changes_the_set() {
        let (_, en) = build_words("en", "2026-07-10", false);
        let (_, es) = build_words("es", "2026-07-10", false);
        assert_ne!(en, es, "different locale should differ");
    }

    #[test]
    fn date_changes_the_set() {
        let (_, d1) = build_words("en", "2026-07-10", false);
        let (_, d2) = build_words("en", "2026-07-11", false);
        assert_ne!(d1, d2, "different date should differ");
    }

    #[test]
    fn kid_arc_is_ten_easy_medium() {
        let (_, k) = build_words("en", "2026-07-10", true);
        assert_eq!(k.len(), 10);
    }
}
