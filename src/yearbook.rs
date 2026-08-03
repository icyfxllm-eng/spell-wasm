//! BD-5 CC-YEARBOOK — the compiler and its surfaces.
//!
//! I1: read/re-render ONLY. The compiler is a pure function over the
//! stores the app already keeps; the sole new persistence is the parent's
//! cover choice + the D5 ping opt-in (both sanctioned by the readme).
//! HONESTY CONSEQUENCE, recorded in the ledger: completed pictures carry
//! no completion date (Run never stored one), and I1 forbids starting to
//! keep them — so the gallery spread is ALL-TIME, labeled as such, while
//! the dated spreads (journey, milestones, firsts) honor the period.
//! Climb has no dated season store yet either: the period menu ships
//! calendar year (BD-D5 signed default) / school year / all-time.

use crate::yearbook_pdf::{assemble, PageJpeg, Paper};

const COVER_KEY: &str = "spell_yb_cover_v1"; // parent's cover pic id
const PING_KEY: &str = "spell_yb_ping_v1"; // D5 opt-in ("1")

// ---------------- periods ----------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Period {
    CalendarYear, // BD-D5: Eric's signed default
    SchoolYear,   // Sep 1 – Jun 30
    AllTime,
}

impl Period {
    pub fn key(self) -> &'static str {
        match self {
            Period::CalendarYear => "yb.period.cal",
            Period::SchoolYear => "yb.period.school",
            Period::AllTime => "yb.period.all",
        }
    }

    /// Inclusive epoch-day bounds for the period containing `today`
    /// (an epoch-day, learner convention). Pure — golden-testable.
    pub fn day_bounds(self, today: u32) -> (u32, u32) {
        match self {
            Period::AllTime => (0, u32::MAX),
            Period::CalendarYear => {
                let (y, _, _) = ymd(today);
                (day_of(y, 1, 1), day_of(y, 12, 31))
            }
            Period::SchoolYear => {
                let (y, m, _) = ymd(today);
                let start_y = if m >= 9 { y } else { y - 1 };
                (day_of(start_y, 9, 1), day_of(start_y + 1, 6, 30))
            }
        }
    }
}

/// Civil-from-days / days-from-civil (Howard Hinnant's algorithms) — the
/// no-Date route to period boundaries, so the compiler stays pure.
fn ymd(day: u32) -> (i64, u32, u32) {
    let z = day as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn day_of(y: i64, m: u32, d: u32) -> u32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as u32
}

// ---------------- the compiled book ----------------

pub struct GalleryItem {
    pub pic: String,
    pub lang: String,
    pub words: usize,
}

pub struct Book {
    pub period: Period,
    pub cover_pic: Option<String>,
    pub gallery: Vec<GalleryItem>, // all-time (see module doc)
    pub mastered_in_period: usize,
    pub hardest_word: Option<String>,
    pub redemptions_in_period: usize,
    pub days_played: usize,
    pub best_streak: u32,
    pub shields: u32,
    pub daily_total: u32,
    pub first_words: Vec<(String, String)>, // (lang, word), period-scoped
    pub favorite_pic: Option<String>,       // most-replayed
}

/// Sparsity floor (readme feature 4): fewer than this many real content
/// atoms and the book is an honest "not enough yet", never filler.
pub const FLOOR: usize = 3;

impl Book {
    pub fn atoms(&self) -> usize {
        self.gallery.len()
            + self.mastered_in_period
            + self.days_played
            + usize::from(self.favorite_pic.is_some())
    }
    pub fn enough(&self) -> bool {
        self.atoms() >= FLOOR
    }
}

/// Pure compile over already-kept stores. `today` is an epoch-day.
pub fn compile(state: &crate::model::AppState, period: Period, today: u32) -> Book {
    let (lo, hi) = period.day_bounds(today);
    // Picture data arrives through the I3 bridge — the compiler never
    // imports the Spell Picture subtree. No bridge (picture feature off,
    // or the web build) => an honest picture-free book.
    let runs = crate::surface_hooks::picture_bridge().gallery.map(|f| f()).unwrap_or_default();

    let mut gallery: Vec<GalleryItem> = runs
        .iter()
        .map(|(pic, lang, words, _, _)| GalleryItem { pic: pic.clone(), lang: lang.clone(), words: *words })
        .collect();
    gallery.sort_by(|a, b| a.pic.cmp(&b.pic)); // deterministic order (I3 of THIS file)

    let favorite_pic = runs
        .iter()
        .filter(|(_, _, _, replayed, _)| *replayed)
        .max_by_key(|(_, _, _, _, touched)| *touched)
        .map(|(pic, _, _, _, _)| pic.clone());

    let st = crate::learner::load_for(&state.lang);
    let in_period =
        |day: u32| day >= lo && day <= hi;
    let mastered_in_period = st
        .log
        .iter()
        .filter(|a| a.correct && in_period(a.day))
        .map(|a| a.word.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let hardest_word = st
        .log
        .iter()
        .filter(|a| a.correct && in_period(a.day))
        .max_by_key(|a| a.word.chars().count())
        .map(|a| a.word.clone());
    let mut first_words: Vec<(String, String)> = Vec::new();
    if let Some(first) = st.log.iter().find(|a| a.correct && in_period(a.day)) {
        first_words.push((state.lang.clone(), first.word.clone()));
    }

    let redemptions_in_period = crate::reports::redemptions(&state.lang)
        .iter()
        .filter(|r| in_period((r.mastered_ts / 86_400_000.0) as u32))
        .count();

    let rec = crate::daily::load();
    let days_played = rec
        .history
        .keys()
        .filter(|d| date_key_in(d, lo, hi))
        .count();
    let daily_total: u32 = rec
        .history
        .iter()
        .filter(|(d, _)| date_key_in(d, lo, hi))
        .map(|(_, n)| *n)
        .sum();

    Book {
        period,
        cover_pic: crate::storage::get_raw(COVER_KEY),
        gallery,
        mastered_in_period,
        hardest_word,
        redemptions_in_period,
        days_played,
        best_streak: rec.best_streak,
        shields: state.aids.shields as u32,
        daily_total,
        first_words,
        favorite_pic,
    }
}

/// Clean re-render via the bridge (acceptance #3 by construction).
pub fn render_piece(pic: &str, lang: &str) -> Option<String> {
    crate::surface_hooks::picture_bridge().render_piece.and_then(|f| f(pic, lang))
}

/// Daily history keys are "YYYY-MM-DD"; parse without Date.
fn date_key_in(key: &str, lo: u32, hi: u32) -> bool {
    let mut it = key.split('-');
    let (Some(y), Some(m), Some(d)) =
        (it.next().and_then(|s| s.parse::<i64>().ok()),
         it.next().and_then(|s| s.parse::<u32>().ok()),
         it.next().and_then(|s| s.parse::<u32>().ok()))
    else {
        return false;
    };
    let day = day_of(y, m, d);
    day >= lo && day <= hi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_year_bounds_are_jan1_dec31() {
        // 2026-08-03 is epoch day 20668.
        let (lo, hi) = Period::CalendarYear.day_bounds(20668);
        assert_eq!(ymd(lo), (2026, 1, 1));
        assert_eq!(ymd(hi), (2026, 12, 31));
    }

    #[test]
    fn school_year_straddles_september() {
        let (lo, hi) = Period::SchoolYear.day_bounds(20668); // Aug 2026
        assert_eq!(ymd(lo), (2025, 9, 1), "August belongs to the 25/26 school year");
        assert_eq!(ymd(hi), (2026, 6, 30));
        let (lo2, _) = Period::SchoolYear.day_bounds(day_of(2026, 10, 1));
        assert_eq!(ymd(lo2), (2026, 9, 1), "October opens the 26/27 year");
    }

    #[test]
    fn civil_roundtrip() {
        for day in [0u32, 719, 20668, 25000] {
            let (y, m, d) = ymd(day);
            assert_eq!(day_of(y, m, d), day);
        }
    }

    #[test]
    fn date_keys_filter_honestly() {
        let (lo, hi) = Period::CalendarYear.day_bounds(20668);
        assert!(date_key_in("2026-06-15", lo, hi));
        assert!(!date_key_in("2025-12-31", lo, hi));
        assert!(!date_key_in("garbage", lo, hi));
    }
}
