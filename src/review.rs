//! CC-LEARNING-ENGINE-L0 R2 — THE rule that decides when a missed word
//! comes back (I5: exactly one). The misses queue (`misses.rs`) and the zh
//! tone drill (`tone_drill.rs`) both schedule through it (C2); neither
//! computes a due time of its own. It replaced the Leitner boxes.
//!
//! Shape of a card's life:
//!
//! 1. **A miss** puts the word in learning: due NOW, then 10 minutes after
//!    a correct answer. These are the old boxes 1 and 2, kept so a missed
//!    word is still reviewable the same day. Same-day answers don't touch
//!    the FSRS memory state (FSRS-4.5 models whole days).
//! 2. **Learning done**, the card is on FSRS-4.5 (`learner::fsrs_review_
//!    graded`, the crate's one FSRS, C3) at whole-day granularity, graded
//!    Again / Hard / Good (D3). A miss sends it back to learning (a lapse).
//! 3. **Graduation**: a success whose next interval is over
//!    `GRADUATE_AFTER_DAYS` removes the word from the queue, as clearing the
//!    old top box did (redemption and the journal's "mastered" still fire).
//!
//! Defaults only (D4): the pinned FSRS-4.5 weights, no per-player fitting.
//! Pure functions of (state, grade, now): same inputs, same schedule (I7).

use serde::{Deserialize, Serialize};

pub use crate::learner::Grade;
use crate::learner::{fsrs_review_graded, FsrsState};

const DAY_MS: f64 = 86_400_000.0;

/// Same-day learning steps, as offsets from the answer: due now, then +10
/// minutes (the old Leitner boxes 1 and 2).
pub const LEARN_STEPS_MS: [f64; 2] = [0.0, 10.0 * 60.0 * 1000.0];

/// A success whose next interval exceeds this graduates the word. 7 days is
/// the old ladder's top rung, so "mastered" arrives at about the same
/// cadence as before: roughly 5 successes over about 9 days, against the
/// boxes' 5 over about 11.
pub const GRADUATE_AFTER_DAYS: u32 = 7;

/// A queued word's schedule. `step` < LEARN_STEPS_MS.len() means learning;
/// equal means on FSRS.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReviewState {
    pub step: u8,
    pub fsrs: FsrsState,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Comes back at this time (ms since the epoch).
    Due(f64),
    /// Remembered well enough to leave the queue.
    Graduated,
}

fn day_of(ms: f64) -> u32 {
    (ms / DAY_MS) as u32
}

fn learning(r: &ReviewState) -> bool {
    (r.step as usize) < LEARN_STEPS_MS.len()
}

fn fresh() -> ReviewState {
    ReviewState {
        step: 0,
        fsrs: FsrsState { stability: 0.0, difficulty: crate::learner::FSRS_D0, due_day: 0, reps: 0, lapses: 0 },
    }
}

/// A new miss: a fresh card, rated Again, in learning, due now.
pub fn start(now: f64) -> (ReviewState, f64) {
    let mut r = fresh();
    fsrs_review_graded(&mut r.fsrs, Grade::Again, day_of(now));
    (r, now + LEARN_STEPS_MS[0])
}

/// A miss on a word already queued. In learning: back to the first step,
/// FSRS untouched (same-day). On FSRS: a lapse, rated Again, back to learning.
pub fn on_miss(r: &mut ReviewState, now: f64) -> f64 {
    if !learning(r) {
        fsrs_review_graded(&mut r.fsrs, Grade::Again, day_of(now));
    }
    r.step = 0;
    now + LEARN_STEPS_MS[0]
}

/// A correct answer on a queued word, graded Hard or Good. (Again is a
/// miss: call `on_miss`.)
pub fn on_correct(r: &mut ReviewState, grade: Grade, now: f64) -> Outcome {
    debug_assert!(grade != Grade::Again);
    if learning(r) {
        r.step += 1;
        if learning(r) {
            return Outcome::Due(now + LEARN_STEPS_MS[r.step as usize]);
        }
        // Learning done: the card waits for the FSRS due day set by the miss.
        return Outcome::Due(r.fsrs.due_day.max(day_of(now) + 1) as f64 * DAY_MS);
    }
    let today = day_of(now);
    fsrs_review_graded(&mut r.fsrs, grade, today);
    if r.fsrs.due_day.saturating_sub(today) > GRADUATE_AFTER_DAYS {
        Outcome::Graduated
    } else {
        Outcome::Due(r.fsrs.due_day as f64 * DAY_MS)
    }
}

/// Carry-over from the Leitner boxes (signed 2026-09-19): the entry keeps
/// its due time. Boxes 1-2 were the same-day steps; boxes 3, 4 and 5 had
/// 1, 3 and 7-day gaps, which become the FSRS stability. Every queued word
/// was missed at least once, so difficulty starts at D0(Again).
pub fn carry_over(box_: u32, due: f64) -> ReviewState {
    let mut r = fresh();
    fsrs_review_graded(&mut r.fsrs, Grade::Again, day_of(due));
    match box_ {
        0 | 1 => r.step = 0,
        2 => r.step = 1,
        b => {
            r.step = LEARN_STEPS_MS.len() as u8;
            r.fsrs.stability = match b {
                3 => 1.0,
                4 => 3.0,
                _ => 7.0,
            };
            r.fsrs.due_day = day_of(due);
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: f64 = 20_000.0 * DAY_MS + 3_600_000.0; // 01:00 on day 20000

    fn due(o: Outcome) -> f64 {
        match o {
            Outcome::Due(t) => t,
            Outcome::Graduated => panic!("graduated early"),
        }
    }

    #[test]
    fn a_miss_is_due_now_then_ten_minutes_later() {
        let (mut r, d) = start(T0);
        assert_eq!(d, T0, "a fresh miss is reviewable immediately, as before");
        let d2 = due(on_correct(&mut r, Grade::Good, T0 + 1000.0));
        assert_eq!(d2, T0 + 1000.0 + 600_000.0, "second step: +10 minutes");
        assert_eq!(r.fsrs.reps, 1, "same-day steps don't touch the FSRS state");
    }

    #[test]
    fn learning_done_waits_for_a_later_day() {
        let (mut r, _) = start(T0);
        on_correct(&mut r, Grade::Good, T0);
        let d = due(on_correct(&mut r, Grade::Good, T0 + 700_000.0));
        assert!(d >= (day_of(T0) + 1) as f64 * DAY_MS, "never due again the same day once learned");
        assert!(!learning(&r));
    }

    #[test]
    fn good_reviews_graduate_in_about_the_old_cadence() {
        // miss, two same-day steps, then Good on each due day.
        let (mut r, _) = start(T0);
        on_correct(&mut r, Grade::Good, T0);
        let mut t = due(on_correct(&mut r, Grade::Good, T0 + 700_000.0));
        let mut successes = 2;
        loop {
            successes += 1;
            match on_correct(&mut r, Grade::Good, t) {
                Outcome::Due(n) => {
                    assert!(n > t);
                    t = n;
                }
                Outcome::Graduated => break,
            }
            assert!(successes < 12, "never graduated");
        }
        let days = (t - T0) / DAY_MS;
        assert!((4..=7).contains(&successes), "graduated after {successes} successes");
        assert!((5.0..=16.0).contains(&days), "graduated after {days:.1} days");
    }

    #[test]
    fn hard_schedules_sooner_than_good() {
        let mut a = carry_over(4, T0);
        let mut b = a.clone();
        let t = T0 + 3.0 * DAY_MS;
        let (ga, gb) = (on_correct(&mut a, Grade::Hard, t), on_correct(&mut b, Grade::Good, t));
        let (da, db) = (due(ga), match gb { Outcome::Due(x) => x, Outcome::Graduated => f64::INFINITY });
        assert!(da < db, "Hard {da} must come back before Good {db}");
    }

    #[test]
    fn a_miss_on_a_learned_card_is_a_lapse_back_to_learning() {
        let mut r = carry_over(5, T0);
        let s = r.fsrs.stability;
        let d = on_miss(&mut r, T0 + 7.0 * DAY_MS);
        assert_eq!(d, T0 + 7.0 * DAY_MS);
        assert!(learning(&r) && r.fsrs.lapses == 1 && r.fsrs.stability < s);
    }

    #[test]
    fn carry_over_keeps_every_box_meaningful() {
        assert_eq!(carry_over(1, T0).step, 0);
        assert_eq!(carry_over(2, T0).step, 1);
        for (b, s) in [(3, 1.0), (4, 3.0), (5, 7.0)] {
            let r = carry_over(b, T0);
            assert!(!learning(&r));
            assert_eq!((r.fsrs.stability, r.fsrs.due_day), (s, day_of(T0)), "box {b}");
        }
    }

    #[test]
    fn same_inputs_same_schedule() {
        let run = || {
            let (mut r, _) = start(T0);
            let mut out = vec![];
            for (i, g) in [Grade::Good, Grade::Hard, Grade::Good, Grade::Good].into_iter().enumerate() {
                out.push(format!("{:?}", on_correct(&mut r, g, T0 + (i as f64 * 2.0 + 0.5) * DAY_MS)));
            }
            (out, serde_json::to_string(&r).unwrap())
        };
        assert_eq!(run(), run());
    }

    // -- I5: exactly one review rule ---------------------------------------------

    /// Lines that would be a second rule: a due time computed outside this
    /// file, or the Leitner ladder coming back.
    fn second_rule(file: &str, src: &str) -> Vec<String> {
        if file.ends_with("review.rs") {
            return Vec::new();
        }
        // A due assignment only counts in files that handle queue entries;
        // other modules have unrelated `due` fields (telemetry's send time).
        let queue_file = src.contains("MissEntry") || src.contains(".misses") || src.contains("tone_drill");
        let mut out = Vec::new();
        for (i, line) in src.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let assigns_due = queue_file && code.contains(".due =") && !code.contains("==");
            let from_review = code.contains("review::") || code.ends_with("= due;") || code.contains(".due = due");
            if (assigns_due && !from_review) || code.contains("SR_INT") || code.contains("SR_MAXBOX") || code.contains("box_ +=") {
                out.push(format!("{file}:{}: {}", i + 1, code));
            }
        }
        out
    }

    fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    #[test]
    fn i5_one_rule_decides_when_a_missed_word_returns() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rs_files(&root, &mut files);
        assert!(files.len() > 50);
        let bad: Vec<String> = files
            .iter()
            .flat_map(|f| second_rule(&f.to_string_lossy(), &std::fs::read_to_string(f).unwrap()))
            .collect();
        assert!(bad.is_empty(), "a second review rule (L0 I5):\n{}", bad.join("\n"));
    }

    #[test]
    fn i5_scan_catches_a_planted_second_rule() {
        for planted in [
            "        let e: &mut MissEntry = x; e.due = now + 600_000.0;",
            "    state.misses[i].due = now_ms() + SR_INT[b] as f64;",
            "    e.box_ += 1;",
            "    if b > SR_MAXBOX {",
        ] {
            assert!(!second_rule("src/misses.rs", planted).is_empty(), "missed: {planted}");
        }
        assert!(second_rule("src/telemetry/mod.rs", "    a.due = now + rand01() * DAY_MS;").is_empty(), "unrelated due fields");
        assert!(second_rule("src/misses.rs", "        e.due = crate::review::on_miss(r, now);").is_empty());
        assert!(second_rule("src/misses.rs", "        state.misses[idx].due = due;").is_empty());
        assert!(second_rule("src/review.rs", "e.due = now + 1.0;").is_empty(), "the rule itself may");
    }

    // -- I4: identity-keyed --------------------------------------------------------

    #[test]
    fn stress_homographs_and_word_forms_schedule_independently() {
        use crate::model::AppState;
        let mut st = AppState::default();
        for w in ["за́мок", "замо́к", "рука", "ру́ку"] {
            crate::misses::add_miss_at(&mut st, w, "ru", "easy", T0);
        }
        assert_eq!(st.misses.len(), 4, "each form is its own card");
        crate::misses::promote_miss_at(&mut st, "за́мок", "ru", Grade::Good, T0 + 1000.0);
        let step = |w: &str| st.misses.iter().find(|m| m.word == w).and_then(|m| m.review.as_ref()).map(|r| r.step);
        assert_eq!(step("за́мок"), Some(1));
        assert_eq!(step("замо́к"), Some(0), "reviewing one homograph must not move the other");
        assert_eq!(step("рука"), Some(0));
        assert_eq!(step("ру́ку"), Some(0));
    }
}
