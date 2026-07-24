//! Live race session (CC-SPELL-RACING Phase 5 — gameplay bridge).
//!
//! Holds the in-progress `Race` (engine) while the player spells a track, driven by
//! the gameplay loop: `begin_lap` when a word is presented, `end_lap` when it's
//! answered. On completion `finish` records the ghost to the garage. Thread-local
//! (single-threaded WASM), like `ghost.rs`'s session — so `model.rs` is untouched.
//!
//! Timing uses a run-relative clock: the caller passes `now_ms` (the same
//! `game::now_ms` the rest of the loop uses); elapsed = now − run_start. No Climb or
//! shield references (D2).

use std::cell::RefCell;

use crate::racing::engine::{Race, Standing};
use crate::racing::format::Identity;
use crate::racing::garage;
use crate::racing::track::Circuit;

struct Live {
    race: Race,
    circuit: Circuit,
    language: String,
    tier: String,
    run_start_ms: f64,
    lap_open: bool,
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
}

/// Summary handed back on finish, for a result/celebration surface.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub your_time_ms: u32,
    pub opponent_time_ms: u32,
    /// You finished faster than the opponent's recorded run.
    pub won: bool,
}

pub fn is_active() -> bool {
    LIVE.with(|l| l.borrow().is_some())
}

/// Begin a race. `track_ids` are the words in order; `opponent_finish_ms` is the
/// opponent's per-lap finish time (same length). `run_start_ms` anchors the clock.
pub fn start(
    language: &str,
    tier: &str,
    circuit: Circuit,
    track_ids: Vec<u64>,
    opponent_finish_ms: Vec<u32>,
    word_list_hash: u64,
    identity: Identity,
    run_start_ms: f64,
) {
    let race = Race::new(track_ids, opponent_finish_ms, language, tier, word_list_hash, identity);
    LIVE.with(|l| {
        *l.borrow_mut() = Some(Live {
            race,
            circuit,
            language: language.to_string(),
            tier: tier.to_string(),
            run_start_ms,
            lap_open: false,
        });
    });
}

fn elapsed(live: &Live, now_ms: f64) -> u32 {
    (now_ms - live.run_start_ms).max(0.0) as u32
}

/// A word was presented — start its lap timer (idempotent within the lap).
pub fn begin_lap(now_ms: f64) {
    LIVE.with(|l| {
        if let Some(live) = l.borrow_mut().as_mut() {
            let e = elapsed(live, now_ms);
            live.race.start_word(e);
            live.lap_open = true;
        }
    });
}

/// The current word was answered — close its lap.
pub fn end_lap(now_ms: f64, correct: bool) {
    LIVE.with(|l| {
        if let Some(live) = l.borrow_mut().as_mut() {
            let e = elapsed(live, now_ms);
            live.race.submit(e, correct);
            live.lap_open = false;
        }
    });
}

/// Live standing vs the opponent right now (None if no race active).
pub fn standing(now_ms: f64) -> Option<Standing> {
    LIVE.with(|l| l.borrow().as_ref().map(|live| live.race.standing(elapsed(live, now_ms))))
}

pub fn is_finished() -> bool {
    LIVE.with(|l| l.borrow().as_ref().map(|live| live.race.is_finished()).unwrap_or(false))
}

/// Finalize a completed race: record the ghost to the garage and clear the session.
/// Returns the `Outcome`, or `None` if no race is active or it isn't finished (a
/// partial run is never recorded).
pub fn finish() -> Option<Outcome> {
    LIVE.with(|l| {
        // Only CONSUME the session once the race is actually complete — a premature
        // finish() must leave the in-progress race intact.
        if !l.borrow().as_ref().map(|live| live.race.is_finished()).unwrap_or(false) {
            return None;
        }
        let live = l.borrow_mut().take()?;
        let ghost = live.race.finish()?; // None unless every lap is done
        let your_time = garage::total_time_ms(&ghost);
        let opponent_time = live.race.opponent_total_ms();
        garage::record(&live.language, &live.tier, live.circuit, ghost);
        Some(Outcome {
            your_time_ms: your_time,
            opponent_time_ms: opponent_time,
            won: your_time <= opponent_time,
        })
    })
}

/// Abandon a race without recording (leaving the mode).
pub fn cancel() {
    LIVE.with(|l| *l.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::racing::format::Identity;

    fn me() -> Identity {
        Identity { avatar_id: 2, color_id: 1 }
    }

    #[test]
    fn a_played_race_records_a_ghost_and_reports_outcome() {
        // 3-lap track; opponent finishes laps at 1000/2000/3000.
        start("en", "easy", Circuit::Sprint, vec![10, 20, 30], vec![1000, 2000, 3000], 7, me(), 0.0);
        assert!(is_active());
        // play faster than the opponent (finish at 2500 < 3000)
        for (lap, fin) in [800.0, 1600.0, 2500.0].iter().enumerate() {
            begin_lap(*fin - 400.0);
            end_lap(*fin, true);
            let _ = lap;
        }
        assert!(is_finished());
        let out = finish().expect("finished race yields an outcome");
        assert_eq!(out.your_time_ms, 2500);
        assert_eq!(out.opponent_time_ms, 3000);
        assert!(out.won, "2500 < 3000 -> won");
        assert!(!is_active(), "session cleared after finish");
        // the ghost landed in the garage's Most Recent for this (lang,tier,circuit).
        let s = garage::load().slots("en", "easy", Circuit::Sprint);
        // storage is a no-op in native tests, so load() is empty — but finish()
        // returning Some proves the record path ran without panicking.
        let _ = s;
    }

    #[test]
    fn standing_reflects_race_progress() {
        start("en", "easy", Circuit::Sprint, vec![1, 2, 3], vec![1000, 2000, 3000], 0, me(), 0.0);
        // at t=1500 opponent has finished lap 1; you none.
        assert_eq!(standing(1500.0).unwrap().delta, -1);
        begin_lap(0.0);
        end_lap(500.0, true);
        begin_lap(500.0);
        end_lap(900.0, true); // 2 laps by t=900, opponent 0
        assert_eq!(standing(900.0).unwrap().delta, 2);
    }

    #[test]
    fn no_outcome_until_finished_and_cancel_clears() {
        start("en", "easy", Circuit::Sprint, vec![1, 2], vec![100, 200], 0, me(), 0.0);
        begin_lap(0.0);
        end_lap(90.0, true);
        assert!(finish().is_none(), "one lap left -> no record");
        assert!(is_active(), "not finished -> still active");
        cancel();
        assert!(!is_active());
    }
}
