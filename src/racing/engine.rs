//! The race engine (CC-SPELL-RACING Phase 5 core).
//!
//! A pure state machine that drives one race: given a track (word IDs) and an
//! opponent's per-lap finish times, it consumes the player's keystroke/submit events
//! (with caller-supplied elapsed-ms timestamps), reports the live standing vs the
//! opponent, and — on completion — produces a recorded `RaceGhost` (Phase 1 format)
//! ready for the garage (Phase 3).
//!
//! It is DOM-free and time-source-free: the frontend feeds it timestamps and renders
//! its state; tests feed synthetic timestamps. The opponent is just per-lap finish
//! times, so the engine is agnostic to where the opponent came from — a garage ghost
//! (Phase 3) or a synthetic pace ghost (Phase 4). No Climb/shield references (D2).

use crate::racing::format::{Identity, RaceGhost, WordEvent, SCHEMA_VERSION};

/// The live standing at a given elapsed time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Standing {
    /// Laps you have completed.
    pub your_lap: usize,
    /// Laps the opponent had completed by this elapsed time.
    pub opponent_lap: usize,
    /// `your_lap - opponent_lap`. Positive = ahead, negative = behind.
    pub delta: i64,
}

/// One race in progress. `finish()` yields the recorded ghost once every lap is done.
pub struct Race {
    track: Vec<u64>,             // word IDs, in race order (the laps)
    opponent_finish_ms: Vec<u32>, // opponent's finish elapsed-ms per lap (ascending)
    language: String,
    tier: String,
    word_list_hash: u64,
    identity: Identity, // YOUR identity, stamped into the recording

    events: Vec<WordEvent>,        // completed laps
    lap_start_ms: Option<u32>,     // when the current lap started (None until start_word)
    lap_keystrokes: Vec<u32>,      // keystroke elapsed-ms for the current lap
}

impl Race {
    /// Start a race over `track` against an opponent whose per-lap finish times are
    /// `opponent_finish_ms` (may be shorter/longer; standing clamps). `identity` is
    /// yours, recorded into the resulting ghost.
    pub fn new(
        track: Vec<u64>,
        opponent_finish_ms: Vec<u32>,
        language: impl Into<String>,
        tier: impl Into<String>,
        word_list_hash: u64,
        identity: Identity,
    ) -> Self {
        Race {
            track,
            opponent_finish_ms,
            language: language.into(),
            tier: tier.into(),
            word_list_hash,
            identity,
            events: Vec::new(),
            lap_start_ms: None,
            lap_keystrokes: Vec::new(),
        }
    }

    /// Total laps in this race.
    pub fn laps(&self) -> usize {
        self.track.len()
    }

    /// The lap currently being spelled (0-based); equals `laps()` once finished.
    pub fn current_lap(&self) -> usize {
        self.events.len()
    }

    pub fn is_finished(&self) -> bool {
        self.events.len() >= self.track.len()
    }

    /// Begin the current lap at `elapsed_ms`. Idempotent within a lap (a repeated
    /// call before a submit keeps the first start, so a re-render can't reset timing).
    pub fn start_word(&mut self, elapsed_ms: u32) {
        if self.lap_start_ms.is_none() && !self.is_finished() {
            self.lap_start_ms = Some(elapsed_ms);
            self.lap_keystrokes.clear();
        }
    }

    /// Record a keystroke at `elapsed_ms` for the current lap (ignored if the lap
    /// hasn't started or the race is done).
    pub fn keystroke(&mut self, elapsed_ms: u32) {
        if self.lap_start_ms.is_some() && !self.is_finished() {
            self.lap_keystrokes.push(elapsed_ms);
        }
    }

    /// Finish the current lap at `elapsed_ms` with `correct`. No-op if the lap wasn't
    /// started or the race is already done. Advances to the next lap.
    pub fn submit(&mut self, elapsed_ms: u32, correct: bool) {
        if self.is_finished() {
            return;
        }
        let Some(start) = self.lap_start_ms.take() else { return };
        let lap = self.events.len();
        self.events.push(WordEvent {
            word_id: self.track[lap],
            start_ms: start,
            finish_ms: elapsed_ms,
            keystrokes_ms: std::mem::take(&mut self.lap_keystrokes),
            correct,
        });
    }

    /// Your lap vs the opponent's lap at `elapsed_ms`. The opponent's lap is how many
    /// of its finish times are ≤ `elapsed_ms` (clamped to its recorded length).
    pub fn standing(&self, elapsed_ms: u32) -> Standing {
        let your_lap = self.events.len();
        let opponent_lap = self.opponent_finish_ms.iter().filter(|&&t| t <= elapsed_ms).count();
        Standing {
            your_lap,
            opponent_lap,
            delta: your_lap as i64 - opponent_lap as i64,
        }
    }

    /// The recorded ghost — only once every lap is finished. Returns `None` mid-race,
    /// so a partial run can never be stored as a completion.
    pub fn finish(&self) -> Option<RaceGhost> {
        if !self.is_finished() {
            return None;
        }
        Some(RaceGhost {
            schema_version: SCHEMA_VERSION,
            language: self.language.clone(),
            tier: self.tier.clone(),
            word_list_hash: self.word_list_hash,
            identity: self.identity,
            events: self.events.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::racing::format::{self, Identity};

    fn me() -> Identity {
        Identity { avatar_id: 1, color_id: 2 }
    }

    // Play a full 3-lap race, all correct, finishing each lap at the given times.
    fn run_full() -> Race {
        let mut r = Race::new(vec![11, 22, 33], vec![1000, 2000, 3000], "en", "easy", 42, me());
        let finishes = [900u32, 1900, 3200];
        let mut t = 0;
        for (lap, &fin) in finishes.iter().enumerate() {
            r.start_word(t);
            r.keystroke(t + 100);
            r.keystroke(t + 200);
            r.submit(fin, true);
            assert_eq!(r.current_lap(), lap + 1);
            t = fin;
        }
        r
    }

    #[test]
    fn a_full_race_produces_a_valid_recordable_ghost() {
        let r = run_full();
        assert!(r.is_finished());
        let g = r.finish().expect("finished race yields a ghost");
        assert_eq!(g.events.len(), 3);
        assert_eq!(g.identity, me());
        assert_eq!(g.language, "en");
        assert_eq!(g.word_list_hash, 42);
        // the recorded ghost is a well-formed v1 ghost (round-trips through encode).
        let round: RaceGhost = serde_json::from_str(&format::encode(&g)).unwrap();
        assert_eq!(round, g);
        // laps carry the track's word IDs in order.
        assert_eq!(g.events.iter().map(|e| e.word_id).collect::<Vec<_>>(), vec![11, 22, 33]);
    }

    #[test]
    fn no_ghost_until_finished() {
        let mut r = Race::new(vec![1, 2], vec![100, 200], "en", "easy", 0, me());
        assert!(r.finish().is_none());
        r.start_word(0);
        r.submit(90, true);
        assert!(r.finish().is_none(), "one lap left -> no completion");
        r.start_word(90);
        r.submit(180, true);
        assert!(r.finish().is_some());
    }

    #[test]
    fn standing_tracks_ahead_and_behind() {
        let r = Race::new(vec![1, 2, 3], vec![1000, 2000, 3000], "en", "easy", 0, me());
        // at t=1500 the opponent has finished lap 1 only; you've finished 0.
        let s = r.standing(1500);
        assert_eq!(s.opponent_lap, 1);
        assert_eq!(s.your_lap, 0);
        assert_eq!(s.delta, -1, "behind by one");
    }

    #[test]
    fn ahead_when_you_have_more_laps() {
        let mut r = Race::new(vec![1, 2, 3], vec![5000, 6000, 7000], "en", "easy", 0, me());
        r.start_word(0);
        r.submit(400, true);
        r.start_word(400);
        r.submit(800, true); // you've done 2 laps by t=800; opponent 0
        let s = r.standing(800);
        assert_eq!(s.your_lap, 2);
        assert_eq!(s.opponent_lap, 0);
        assert_eq!(s.delta, 2, "ahead by two");
    }

    #[test]
    fn start_word_is_idempotent_and_submit_needs_a_start() {
        let mut r = Race::new(vec![1], vec![100], "en", "easy", 0, me());
        // submit before start -> no-op
        r.submit(50, true);
        assert_eq!(r.current_lap(), 0);
        r.start_word(10);
        r.start_word(999); // ignored; first start wins
        r.keystroke(20);
        r.submit(80, true);
        let g = r.finish().unwrap();
        assert_eq!(g.events[0].start_ms, 10, "first start_word timestamp kept");
        assert_eq!(g.events[0].finish_ms, 80);
    }
}
