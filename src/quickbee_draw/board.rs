//! CC-QUICKBEE-DRAW F3 / I1 / A6 -- the drawn leaderboard.
//!
//! LOCAL ONLY (Eric, 2026-10-15): a per-language list of this device's best
//! drawn runs, held under its own storage key. It is never merged with typed
//! Quick Bee, which keeps its own records and which this file cannot reach, and
//! a query for one language returns that language's rows and nothing else.
//!
//! Pure data and rules, so the surface is testable; the screen only draws it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The device's own key. Nothing else writes it and this mode writes nothing
/// else for scores.
pub const STORE_KEY: &str = "spell_quickbee_draw_v1";
/// Rows kept and shown per language.
pub const TOP_N: usize = 5;
/// Recent scores kept per language (for a trend; not shown as a board).
const RECENT_N: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub score: u32,
    pub words: u32,
    /// The tier the run was played at ("easy" .. "expert").
    pub tier: String,
    /// Days since the epoch, for tie-breaks and display.
    pub day: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub best: BTreeMap<String, u32>,
    #[serde(default)]
    pub runs: BTreeMap<String, u32>,
    #[serde(default)]
    pub recent: BTreeMap<String, Vec<u32>>,
    /// The board: per language, best first, at most `TOP_N`.
    #[serde(default)]
    pub top: BTreeMap<String, Vec<Entry>>,
}

/// What recording a run did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    /// 1-based place on this language's board, or None when it did not place.
    pub rank: Option<usize>,
    pub new_best: bool,
    pub best: u32,
}

impl Store {
    /// Record a finished run for `lang`. A score of 0 is counted as a run and is
    /// never placed on the board.
    pub fn record(&mut self, lang: &str, e: Entry) -> Recorded {
        let score = e.score;
        *self.runs.entry(lang.to_string()).or_default() += 1;
        let recent = self.recent.entry(lang.to_string()).or_default();
        recent.push(score);
        if recent.len() > RECENT_N {
            recent.remove(0);
        }
        let prev_best = self.best.get(lang).copied().unwrap_or(0);
        let new_best = score > prev_best;
        let best = self.best.entry(lang.to_string()).or_default();
        *best = (*best).max(score);
        let best_now = *best;
        let mut rank = None;
        if score > 0 {
            let list = self.top.entry(lang.to_string()).or_default();
            // Higher score first; among equals the newer run ranks above the older.
            let at = list.iter().position(|x| score >= x.score).unwrap_or(list.len());
            list.insert(at, e);
            if at < TOP_N {
                rank = Some(at + 1);
            }
            list.truncate(TOP_N);
        }
        Recorded { rank, new_best, best: best_now }
    }

    /// F3 / A6: this language's board and no other.
    pub fn board(&self, lang: &str) -> &[Entry] {
        self.top.get(lang).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn best_of(&self, lang: &str) -> u32 {
        self.best.get(lang).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(score: u32, day: u32) -> Entry {
        Entry { score, words: score / 2, tier: "easy".into(), day }
    }

    #[test]
    fn a6_a_language_query_returns_only_that_languages_rows() {
        let mut s = Store::default();
        s.record("zh", e(12, 1));
        s.record("zh", e(30, 2));
        s.record("ja", e(99, 2));
        assert_eq!(s.board("zh").iter().map(|x| x.score).collect::<Vec<_>>(), vec![30, 12]);
        assert_eq!(s.board("ja").len(), 1);
        assert!(s.board("ko").is_empty() && s.board("en").is_empty());
        assert_eq!(s.best_of("zh"), 30, "ja's 99 never reaches zh");
    }

    #[test]
    fn the_board_keeps_five_best_first_and_reports_the_place() {
        let mut s = Store::default();
        for (i, sc) in [10, 40, 20, 30, 50, 25].iter().enumerate() {
            s.record("zh", e(*sc, i as u32));
        }
        assert_eq!(s.board("zh").iter().map(|x| x.score).collect::<Vec<_>>(), vec![50, 40, 30, 25, 20]);
        let r = s.record("zh", e(45, 9));
        assert_eq!(r.rank, Some(2));
        let r = s.record("zh", e(5, 10));
        assert_eq!(r.rank, None, "a run below the board does not place");
        assert_eq!(s.board("zh").len(), TOP_N);
    }

    #[test]
    fn a_zero_run_counts_but_never_places_and_a_tie_ranks_the_newer_run_first() {
        let mut s = Store::default();
        let z = s.record("ja", e(0, 1));
        assert_eq!((z.rank, z.new_best), (None, false));
        assert_eq!(s.runs.get("ja"), Some(&1));
        s.record("ja", e(8, 1));
        let r = s.record("ja", e(8, 2));
        assert_eq!(r.rank, Some(1), "the newer of two equal scores sits first");
        assert_eq!(s.board("ja")[0].day, 2);
    }

    #[test]
    fn new_best_is_reported_once_and_recent_is_capped() {
        let mut s = Store::default();
        assert!(s.record("zh", e(5, 1)).new_best);
        assert!(!s.record("zh", e(5, 2)).new_best, "equal is not a new best");
        assert!(s.record("zh", e(6, 3)).new_best);
        for i in 0..20 {
            s.record("zh", e(1, i));
        }
        assert_eq!(s.recent["zh"].len(), 10);
    }

    #[test]
    fn a_store_saved_before_the_board_existed_still_loads() {
        let old = r#"{"best":{"zh":9},"runs":{"zh":3},"recent":{"zh":[4,9,2]}}"#;
        let s: Store = serde_json::from_str(old).unwrap();
        assert_eq!(s.best_of("zh"), 9);
        assert!(s.board("zh").is_empty());
        let back: Store = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn i1_the_board_module_reaches_no_other_modes_store() {
        // It owns one key and names no other mode's records, so a typed Quick Bee
        // score cannot rank against a drawn one.
        let src = include_str!("board.rs");
        let code = src.split("#[cfg(test)]").next().unwrap();
        for banned in ["spell_climb", "climb", "leaderboard", "typed", "wordgrid", "daily", "shield", "storage::"] {
            let hits: Vec<&str> = code.lines().filter(|l| !l.trim_start().starts_with("//")).filter(|l| l.contains(banned)).collect();
            assert!(hits.is_empty(), "board.rs mentions {banned}: {hits:?}");
        }
        assert_eq!(STORE_KEY, "spell_quickbee_draw_v1");
    }
}
