//! Daily Challenge (§4.3): a fixed set of words seeded by (date, locale), so
//! everyone playing the same language on the same day gets the exact same run —
//! one attempt per word, no retries, isolated from the adaptive selector, Misses,
//! word stats, chain streak, and The Climb. A per-day result + a consecutive-day
//! streak are persisted locally; the daily set itself is recomputed on the fly
//! (deterministic, so it never needs storing) and works offline once the audio
//! for those words is cached.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::consts::EN;
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

/// Which locale a daily run uses: the active built-in language, else English.
/// Coming-soon (and My Words / unknown) languages fall back to English so the
/// Daily Challenge only ever draws from audited, active languages (Feature 3).
pub fn locale_for(lang: &str) -> String {
    if crate::consts::is_active_lang(lang) {
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

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

// ---- deterministic cycle walk (no-repeat across days) -------------------------

/// Days elapsed since 2026-01-01 for a `YYYY-MM-DD` local date string (D1).
/// Howard Hinnant's `days_from_civil` — pure integer, platform-independent.
fn day_index(date: &str) -> i64 {
    let mut it = date.split('-');
    let y: i64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(2026);
    let m: i64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: i64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    days_from_civil(y, m, d) - days_from_civil(2026, 1, 1)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// FNV-1a of the sorted pool — part of the seed so a shipped word-list update
/// reshuffles the cycle exactly once (D4) instead of silently drifting indices.
fn pool_hash(pool: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for w in pool {
        for b in w.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0x00; // word separator
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Seed for one `(lang, tier, cycle)` shuffle, plus a retry salt for the
/// cycle-boundary guard. Deterministic FNV-1a of the descriptor string.
fn walk_seed(lang: &str, tier: &str, cycle: i64, ph: u64, salt: u32) -> u64 {
    let desc = format!("daily-v2|{lang}|{tier}|{cycle}|{ph}|{salt}");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in desc.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Full Fisher–Yates permutation of `0..n` from `seed` (deterministic).
fn permutation(n: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..n).collect();
    let mut rng = seed;
    for i in (1..n).rev() {
        let j = (next_u64(&mut rng) as usize) % (i + 1);
        idx.swap(i, j);
    }
    idx
}

/// One tier's daily slice via the cycle walk (Feature 1): the tier pool is
/// shuffled once per cycle and consumed `w` words per day, so no word recurs
/// until the whole pool is spent (`L = |pool|/w` days). `slice_for` returns the
/// day's words for an arbitrary `day_idx`, so the boundary guard can peek at
/// yesterday deterministically.
fn tier_slice(lang: &str, tier: &str, pool: &[&str], w: usize, day_idx: i64) -> Vec<String> {
    let n = pool.len();
    if n == 0 || w == 0 {
        return Vec::new();
    }
    let l = (n / w).max(1) as i64; // days per cycle
    let ph = pool_hash(pool);
    let take = w.min(n);
    let slice_for = |di: i64, salt: u32| -> Vec<usize> {
        let cycle = di.div_euclid(l);
        let day = di.rem_euclid(l) as usize;
        let perm = permutation(n, walk_seed(lang, tier, cycle, ph, salt));
        let start = day * w;
        perm[start..(start + take).min(n)].to_vec()
    };
    // Cycle-boundary guard: at day 0 a fresh shuffle could re-serve one of
    // yesterday's words; re-derive with a salt until disjoint (cap 8, then accept).
    let today = slice_for(day_idx, 0);
    let day = day_idx.rem_euclid(l);
    let chosen = if day == 0 && day_idx > 0 {
        let yesterday: std::collections::HashSet<usize> = slice_for(day_idx - 1, 0).into_iter().collect();
        let mut pick = today;
        let mut salt = 1u32;
        while salt <= 8 && pick.iter().any(|i| yesterday.contains(i)) {
            pick = slice_for(day_idx, salt);
            salt += 1;
        }
        pick
    } else {
        today
    };
    chosen.iter().map(|&i| pool[i].to_string()).collect()
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
    let arc: &[(&str, usize)] = if kid { &KID_ARC } else { &ARC };
    let di = day_index(date);
    let mut out = Vec::new();
    for (tier, count) in arc {
        let full = words::tier_for(&locale, tier);
        // Kid Mode "friendly words": filter age-inappropriate terms from the
        // daily pool too (deterministic — same list for everyone).
        let kept: Vec<&str> = if kid {
            full.iter().copied().filter(|w| crate::kid_filter::kid_allowed(&locale, w)).collect()
        } else {
            Vec::new()
        };
        let base: Vec<&str> = if kid && !kept.is_empty() { kept } else { full.to_vec() };
        // Stable input order (by word ID = the stored string) so the cycle walk
        // is identical on every device regardless of the source list's order.
        let mut pool = base;
        pool.sort_unstable();
        // Cycle walk: this tier's deck, W=count per day, no repeat for L days.
        let mut picked = tier_slice(&locale, tier, &pool, *count, di);
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

    fn date_for(day_idx: i64) -> String {
        // inverse of day_index for the test driver: civil date of 2026-01-01 + n
        let z = days_from_civil(2026, 1, 1) + day_idx + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{:04}-{:02}-{:02}", y, m, d)
    }

    /// I1: within each (lang, tier), no word repeats inside a window of L
    /// consecutive dailies, and yesterday never bleeds into today.
    #[test]
    fn daily_no_repeat() {
        let langs = ["en", "es", "fr", "de", "pt", "it", "nl", "pl", "sv", "nb", "tr", "vi", "ko", "ja", "th", "fil", "zh"];
        for &lang in &langs {
            for kid in [false, true] {
                let arc: &[(&str, usize)] = if kid { &KID_ARC } else { &ARC };
                let loc = locale_for(lang);
                for (tier, w) in arc {
                    let n = words::tier_for(&loc, tier).len();
                    if n == 0 {
                        continue;
                    }
                    let l = (n / w).max(1);
                    // Walk one full cycle: no word may appear twice within it.
                    let mut seen = std::collections::HashSet::new();
                    let mut prev: Vec<String> = Vec::new();
                    for day in 0..l as i64 {
                        let slice = tier_slice(&loc, tier, &{ let mut p: Vec<&str> = words::tier_for(&loc, tier).to_vec(); p.sort_unstable(); p }, *w, day);
                        for word in &slice {
                            assert!(seen.insert(word.clone()), "{lang}/{tier} kid={kid}: '{word}' repeated within cycle (day {day}, L={l})");
                        }
                        if day > 0 {
                            for word in &slice {
                                assert!(!prev.contains(word), "{lang}/{tier}: '{word}' repeated from yesterday (day {day})");
                            }
                        }
                        prev = slice;
                    }
                }
            }
        }
    }

    /// I2: 400-day run is byte-identical across two independent computations.
    #[test]
    fn daily_deterministic_400_days() {
        for &lang in &["en", "de", "th", "ja", "zh"] {
            for day in 0..400i64 {
                let date = date_for(day);
                let (_, a) = build_words(lang, &date, false);
                let (_, b) = build_words(lang, &date, false);
                assert_eq!(a, b, "{lang} {date}: non-deterministic");
            }
        }
    }
}
