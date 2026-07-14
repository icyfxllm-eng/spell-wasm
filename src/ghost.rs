//! F6 — Ghost racing in The Climb (FP2, behind `flags::ghost_racing`, default ON).
//!
//! Single-player, LOCAL-ONLY. During a solo Climb run we record, per answered
//! word, the elapsed ms since the run started and whether it was correct. When a
//! run ends we keep the BEST run per study-language in localStorage
//! ("spell_ghost_v1"); during the next run we draw a live "ghost" pace marker on
//! the existing chain bar (#ghostPace) showing where that best run stood at the
//! same elapsed time, plus an ahead/behind delta.
//!
//! The ghost is a pure local replay. It NEVER talks to a server, the data NEVER
//! leaves the device, and it NEVER reads The Climb's leaderboard — `climb.rs` is
//! not touched or referenced here.
//!
//! Best-run metric: the run that reached the higher score (chain length = number
//! of correct answers, the same quantity the Climb surfaces as the current
//! chain) wins; on a tie, the run that reached it FASTER (smaller time-to-reach)
//! wins. I.e. "reached the furthest / highest score fastest."

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dom;
use crate::storage;

const GHOST_KEY: &str = "spell_ghost_v1";

/// One answered word in a run. The terminating miss that ends a chain is
/// recorded as `correct: false`; every earlier word in a chain is correct.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct GhostEvent {
    #[serde(rename = "t")]
    pub elapsed_ms: u32,
    #[serde(rename = "c")]
    pub correct: bool,
}

/// A stored best run for one language: its per-word event log.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GhostRun {
    pub events: Vec<GhostEvent>,
}

impl GhostRun {
    /// The run's score = number of correct answers (its final chain length).
    pub fn reached(&self) -> u32 {
        self.events.iter().filter(|e| e.correct).count() as u32
    }

    /// Elapsed ms at the last correct answer — how long it took to reach the
    /// score. Used as the speed tie-break and the marker's time axis.
    pub fn time_to_reach(&self) -> u32 {
        self.events.iter().filter(|e| e.correct).map(|e| e.elapsed_ms).max().unwrap_or(0)
    }

    /// Sorted elapsed-ms of each correct answer; position `i` is reached at
    /// `correct_times()[i-1]`. Basis for [`position_at`].
    fn correct_times(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.events.iter().filter(|e| e.correct).map(|e| e.elapsed_ms).collect();
        v.sort_unstable();
        v
    }

    /// Best-run comparison: higher score wins; on a tie, faster wins. Two empty
    /// runs are never "better" than each other.
    pub fn is_better_than(&self, other: &GhostRun) -> bool {
        let (a, b) = (self.reached(), other.reached());
        if a != b {
            return a > b;
        }
        if a == 0 {
            return false;
        }
        self.time_to_reach() < other.time_to_reach()
    }
}

/// Ghost fractional position at `elapsed_ms`, linearly interpolated between the
/// correct-answer timestamps. Position 0 at t=0, position `k` at `times[k-1]`,
/// clamped at `times.len()` once the ghost has finished (it can't exceed its own
/// reached score). Pure + deterministic — the unit-tested core of the marker.
pub fn position_at(times: &[u32], elapsed_ms: f64) -> f64 {
    let n = times.len();
    if n == 0 || elapsed_ms <= 0.0 {
        return 0.0;
    }
    let last = times[n - 1] as f64;
    if elapsed_ms >= last {
        return n as f64;
    }
    let mut prev = 0.0_f64;
    for (i, &t) in times.iter().enumerate() {
        let t = t as f64;
        if elapsed_ms < t {
            let frac = if t > prev { (elapsed_ms - prev) / (t - prev) } else { 0.0 };
            return i as f64 + frac;
        }
        prev = t;
    }
    n as f64
}

/// Pick the run to keep: the incumbent unless `candidate` is strictly better.
pub fn better_of(existing: Option<GhostRun>, candidate: GhostRun) -> GhostRun {
    match existing {
        Some(e) if !candidate.is_better_than(&e) => e,
        _ => candidate,
    }
}

// ---------- persistence (lang -> best run) ----------

fn load_all() -> HashMap<String, GhostRun> {
    storage::get_json(GHOST_KEY).unwrap_or_default()
}

fn save_all(map: &HashMap<String, GhostRun>) {
    storage::set_json(GHOST_KEY, map);
}

/// The stored best run for `lang`, if any.
pub fn best_for(lang: &str) -> Option<GhostRun> {
    load_all().remove(lang)
}

// ---------- live session ----------

#[derive(Default)]
struct Session {
    active: bool,
    lang: String,
    events: Vec<GhostEvent>,
    /// Correct-answer times of the best run being raced (empty if none).
    best_times: Vec<u32>,
    has_best: bool,
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

/// Outcome of finishing a run, for the caller's celebration decision.
pub enum Outcome {
    /// Beat a prior stored ghost — celebrate.
    Beat,
    /// First-ever run for this language stored (no prior ghost to beat).
    FirstRecord,
    /// Nothing stored (empty run, feature off, or didn't beat the ghost).
    NoChange,
}

/// Begin recording a solo Climb run for `lang` and load the best run to race.
/// No-op (zero DOM/storage effect) when the feature is off.
pub fn start_run(lang: &str) {
    if !crate::flags::ghost_racing() {
        return;
    }
    let best = best_for(lang);
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        s.active = true;
        s.lang = lang.to_string();
        s.events.clear();
        s.best_times = best.as_ref().map(|b| b.correct_times()).unwrap_or_default();
        s.has_best = best.is_some();
    });
    // Prime the marker (stays hidden until there's a ghost to show).
    update_pace(0, 0.0);
}

/// Record a correct answer at `elapsed_ms` and refresh the live pace marker.
pub fn note_correct(streak: u32, elapsed_ms: f64) {
    if !crate::flags::ghost_racing() {
        return;
    }
    let active = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if s.active {
            s.events.push(GhostEvent { elapsed_ms: elapsed_ms.max(0.0) as u32, correct: true });
        }
        s.active
    });
    if active {
        update_pace(streak, elapsed_ms);
    }
}

/// Record the terminating miss that ends a chain (kept for a complete per-word
/// log; it does not affect the best-run metric, which counts only correct words).
pub fn note_incorrect(elapsed_ms: f64) {
    if !crate::flags::ghost_racing() {
        return;
    }
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if s.active {
            s.events.push(GhostEvent { elapsed_ms: elapsed_ms.max(0.0) as u32, correct: false });
        }
    });
}

/// End the run: store it if it's a new best, and report whether it beat a prior
/// ghost so the caller can celebrate. Also hides the live marker.
pub fn finish_run() -> Outcome {
    if !crate::flags::ghost_racing() {
        return Outcome::NoChange;
    }
    hide_pace();
    let (active, lang, run) = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let active = s.active;
        s.active = false;
        (active, s.lang.clone(), GhostRun { events: s.events.clone() })
    });
    if !active || run.reached() == 0 {
        return Outcome::NoChange;
    }
    let mut all = load_all();
    let existing = all.get(&lang).cloned();
    let had_prior = existing.as_ref().map(|e| e.reached() > 0).unwrap_or(false);
    let beat = match &existing {
        Some(e) if e.reached() > 0 => run.is_better_than(e),
        _ => true,
    };
    if beat {
        all.insert(lang, better_of(existing, run));
        save_all(&all);
        if had_prior { Outcome::Beat } else { Outcome::FirstRecord }
    } else {
        Outcome::NoChange
    }
}

/// Hide the live pace marker. Idempotent and safe to call when the feature is
/// off (the element ships hidden), so it never introduces a behavioral diff.
pub fn hide_pace() {
    dom::toggle_class("ghostPace", "btn-hide", true);
    dom::toggle_class("ghostPace", "ahead", false);
    dom::toggle_class("ghostPace", "behind", false);
}

fn update_pace(streak: u32, elapsed_ms: f64) {
    let (active, has_best, ghost_pos) = SESSION.with(|s| {
        let s = s.borrow();
        (s.active, s.has_best, position_at(&s.best_times, elapsed_ms))
    });
    if !active || !has_best {
        // No prior ghost for this language yet — nothing to race, stay hidden.
        hide_pace();
        return;
    }
    let ghost_words = ghost_pos.floor() as i64;
    let delta = streak as i64 - ghost_words;
    let text = if delta > 0 {
        crate::i18n::tp("ghost.ahead", &[("n", &delta.to_string())])
    } else if delta < 0 {
        crate::i18n::tp("ghost.behind", &[("n", &(-delta).to_string())])
    } else {
        crate::i18n::t("ghost.even")
    };
    dom::set_text("ghostPace", &text);
    dom::toggle_class("ghostPace", "ahead", delta > 0);
    dom::toggle_class("ghostPace", "behind", delta < 0);
    dom::toggle_class("ghostPace", "btn-hide", false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(t: u32, c: bool) -> GhostEvent {
        GhostEvent { elapsed_ms: t, correct: c }
    }

    fn run(correct_times: &[u32]) -> GhostRun {
        GhostRun { events: correct_times.iter().map(|&t| ev(t, true)).collect() }
    }

    #[test]
    fn reached_and_time_ignore_the_terminating_miss() {
        let r = GhostRun { events: vec![ev(1000, true), ev(2200, true), ev(3000, false)] };
        assert_eq!(r.reached(), 2);
        assert_eq!(r.time_to_reach(), 2200);
    }

    #[test]
    fn better_run_prefers_higher_score_then_faster() {
        let base = run(&[1000, 2000, 3000, 4000, 5000]); // reached 5 in 5000ms
        // Higher score wins even if slower.
        assert!(run(&[1000, 2000, 3000, 4000, 5000, 9000]).is_better_than(&base));
        // Same score, faster wins.
        assert!(run(&[500, 1000, 1500, 2000, 2500]).is_better_than(&base));
        // Same score, slower loses.
        assert!(!run(&[1000, 2000, 3000, 4000, 6000]).is_better_than(&base));
        // Lower score loses.
        assert!(!run(&[100, 200, 300]).is_better_than(&base));
        // Empty vs empty: neither better.
        assert!(!GhostRun::default().is_better_than(&GhostRun::default()));
    }

    #[test]
    fn better_of_replaces_only_when_strictly_better() {
        let existing = run(&[1000, 2000, 3000]);
        // No prior best → candidate is kept.
        assert_eq!(better_of(None, run(&[500])).reached(), 1);
        // Strictly better (higher) → replaced.
        assert_eq!(better_of(Some(existing.clone()), run(&[1, 2, 3, 4])).reached(), 4);
        // Not better → incumbent kept (same reached, slower candidate).
        let kept = better_of(Some(existing.clone()), run(&[1000, 2000, 9000]));
        assert_eq!(kept.time_to_reach(), 3000);
    }

    #[test]
    fn position_interpolates_between_correct_times() {
        let times = [1000u32, 2000, 3000];
        assert_eq!(position_at(&times, 0.0), 0.0);
        assert_eq!(position_at(&times, -50.0), 0.0);
        assert!((position_at(&times, 500.0) - 0.5).abs() < 1e-9);
        assert!((position_at(&times, 1000.0) - 1.0).abs() < 1e-9);
        assert!((position_at(&times, 1500.0) - 1.5).abs() < 1e-9);
        assert!((position_at(&times, 2500.0) - 2.5).abs() < 1e-9);
        assert_eq!(position_at(&times, 3000.0), 3.0);
        // Past the ghost's final time → clamped at its reached score.
        assert_eq!(position_at(&times, 9000.0), 3.0);
        // No ghost → always 0.
        assert_eq!(position_at(&[], 1234.0), 0.0);
    }

    #[test]
    fn flag_off_is_a_total_no_op() {
        crate::flags::set_test_override(Some("off"));
        SESSION.with(|s| *s.borrow_mut() = Session::default());
        start_run("en");
        note_correct(1, 100.0);
        note_incorrect(200.0);
        let recorded = SESSION.with(|s| s.borrow().events.len());
        assert_eq!(recorded, 0, "no events recorded when the flag is off");
        assert!(matches!(finish_run(), Outcome::NoChange));
        crate::flags::set_test_override(None);
    }
}
