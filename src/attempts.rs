//! CC-ATTEMPTS-SHIELDS / CC-CLIMB-SHIELDS — the cross-language attempts & shield
//! FORGE subsystem.
//!
//! THE CONTRACT: an "attempt" is ONE submitted answer that passed through the
//! single validation path (`game::submit_guess` -> `on_correct`/`on_wrong`).
//! Everything here is driven by exactly a handful of validated events — correct,
//! perfect-correct, a real miss, a shield-absorbed miss — and NEVER reads the
//! language, input method, entitlement level, or raw text. There is deliberately
//! not a single language / entitlement conditional in this file: one
//! implementation serves all languages, and the per-language / per-entitlement
//! test coverage is the SAME test code run over a matrix.
//!
//! Two features share this state machine:
//!   * Feature 1 — "Extra attempt on misses" (normal mode): one clean retry of
//!     a missed word, with the first miss still recorded for spaced repetition.
//!   * Feature 2 — shields (The Climb): FORGE a shield from segments earned by
//!     correct answers, spend one (by choice) to retry a missed word during a run.
//!
//! CC-CLIMB-SHIELDS replaces the old earning model ("1 shield per 5 consecutive
//! correct, cap 3") with the FORGE model:
//!   * A shield is forged from `SEGMENTS_PER_SHIELD` segments.
//!   * Each correct Climb word grants 1 segment; a PERFECT word (first attempt,
//!     no syllable replay) grants `PERFECT_SEGMENTS` instead.
//!   * Segments beyond a forge CARRY into the next shield — earned skill is never
//!     discarded (below the cap).
//!   * A real miss resets forge progress to 0 segments but NEVER removes a forged
//!     shield. A shield ABSORBING a miss does not reset progress.
//!   * Cap = `SHIELD_CAP` held shields; at the cap, forging FREEZES (correct
//!     words grant no segments) until a shield is spent, which re-opens forging.
//!
//! Earning is a PURE function of the run's event sequence — identical for every
//! player, language, and entitlement level, with zero randomness and zero
//! per-language constants.
//!
//! All logic is pure integer state (no DOM, no storage, no `js_sys`), so it runs
//! under `cargo test --lib` on a non-wasm target; `game.rs` wraps these calls
//! with the audio/animation/DOM around them.

use crate::model::AppState;

// ---------- Feature 1/5: the ONE forge config block ----------

/// The single source of the forge's earning constants (CC-CLIMB-SHIELDS feature
/// 1/5). Every literal that governs earning lives here — nothing is scattered as
/// a bare number across `game.rs` or the HUD.
///
/// The `heat` and `clutch` fields are **season-2 stubs, disabled by default**.
/// They are NOT implemented; a fixture that enables either must FAIL loudly (see
/// [`ForgeConfig::validate`]) rather than silently half-working.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForgeConfig {
    /// Segments needed to forge one shield.
    pub segments_per_shield: u8,
    /// Segments granted by a PERFECT word (first attempt, no syllable replay).
    pub perfect_segments: u8,
    /// The most shields a player can hold during a single Climb run.
    pub shield_cap: u8,
    /// Whether segments beyond a forge carry into the next shield (below the cap).
    pub overflow_carry: bool,
    /// SEASON-2 STUB — not implemented. Must stay `false`.
    pub heat: bool,
    /// SEASON-2 STUB — not implemented. Must stay `false`.
    pub clutch: bool,
}

/// The live forge configuration. CC-CLIMB-SHIELDS: 5 segments per shield, a
/// perfect word is worth 2, cap 2 held (CHANGED from the old cap of 3), overflow
/// carries. Season-2 features OFF.
pub const FORGE: ForgeConfig = ForgeConfig {
    segments_per_shield: 5,
    perfect_segments: 2,
    shield_cap: 2,
    overflow_carry: true,
    heat: false,
    clutch: false,
};

impl ForgeConfig {
    /// Guard against a fixture (or a future edit) that flips on a season-2 stub
    /// the forge doesn't implement. Panics with an explicit, greppable message so
    /// enabling `heat`/`clutch` FAILS CI instead of silently half-working. Called
    /// on every forge grant — cheap integer checks.
    pub fn validate(&self) {
        assert!(
            !self.heat && !self.clutch,
            "season-2 feature not implemented (forge `heat`/`clutch` are disabled stubs — \
             do NOT enable them; CC-CLIMB-SHIELDS ships season-1 only)"
        );
        assert!(self.segments_per_shield > 0, "segments_per_shield must be > 0");
        assert!(self.shield_cap > 0, "shield_cap must be > 0");
    }
}

/// Outcome of a forge grant, for the HUD.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Forge {
    /// Segments advanced; no shield completed this word.
    Progress,
    /// One (or more) shields completed this word — the forge MOMENT.
    Forged,
    /// At the cap; forging is frozen, nothing was granted ("shields full").
    Frozen,
}

/// Per-RUN aids state: lives in `AppState` next to the run's score/streak, is
/// session-only, and resets to all-zero at the start of every run. Shields and
/// forge progress do NOT persist between runs and can never be purchased.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct RunAids {
    /// Shields held right now (0..=`FORGE.shield_cap`). Only ever forged, never
    /// bought.
    pub shields: u8,
    /// Forge progress toward the NEXT shield, in segments (0..`SEGMENTS_PER_SHIELD`).
    /// Advances on every genuine correct; resets to 0 on a real miss; carries
    /// overflow across a forge; frozen at the cap.
    pub segments: u8,
    /// True once the current word has already consumed its ONE retry (an extra
    /// attempt or a shield). Enforces "at most one retry per word". Cleared when a
    /// new word starts.
    pub retry_used: bool,
    /// True if the player used SYLLABLE REPLAY on the current word. Defeats the
    /// "perfect word" bonus (but is never otherwise penalized). Cleared per word.
    pub syllable_replay_used: bool,
}

// ---------- run / word lifecycle ----------

/// Start of a Climb run (or any fresh chain): drop all aids to zero. Shields and
/// forge progress are per-run and never carry over (0 segments / 0 shields).
pub fn reset_run(state: &mut AppState) {
    state.aids = RunAids::default();
}

/// A new word is now on screen: the one-retry budget refreshes and the
/// syllable-replay flag clears. Forge progress and held shields are untouched
/// (they span the whole run).
pub fn start_word(state: &mut AppState) {
    state.aids.retry_used = false;
    state.aids.syllable_replay_used = false;
}

/// Record that the player used syllable replay on the current word. This only
/// costs the perfect-word bonus; it is never otherwise penalized.
pub fn note_syllable_replay(state: &mut AppState) {
    state.aids.syllable_replay_used = true;
}

/// Whether the CURRENT word still qualifies as "perfect" for the forge: no retry
/// used (first attempt) AND no syllable replay used. The caller only forges on a
/// genuine (non-rescued) correct, so this is the perfect test at that moment.
pub fn is_perfect_word(state: &AppState) -> bool {
    !state.aids.retry_used && !state.aids.syllable_replay_used
}

// ---------- Feature 1: extra attempts (normal mode) ----------

/// Gate + apply a validated-INCORRECT under the extra-attempts toggle.
///
/// When `enabled` and the word has not yet used its retry: record the FIRST miss
/// to spaced repetition exactly once (the data-integrity rule — recorded
/// REGARDLESS of what the retry then does), mark the retry as used, and return
/// `true` so the caller replays audio + clears the input. Otherwise return
/// `false` (toggle off, or the retry is already spent -> the caller runs the
/// normal miss consequence and records NOTHING further).
pub fn maybe_extra_attempt(
    enabled: bool,
    state: &mut AppState,
    word: &str,
    lang: &str,
    tier: &str,
    now: f64,
) -> bool {
    if !enabled || state.aids.retry_used {
        return false;
    }
    crate::misses::add_miss_at(state, word, lang, tier, now);
    state.aids.retry_used = true;
    true
}

/// Clear the in-progress answer so the retry starts empty — the wrong submission
/// is never pre-filled. Audio replay + focus are the caller's DOM concern.
pub fn prepare_retry(state: &mut AppState) {
    state.answer.clear();
}

// ---------- Feature 2: shields (The Climb) — the FORGE ----------

/// A validated-CORRECT in a Climb run FORGES toward the next shield. Grants
/// `PERFECT_SEGMENTS` for a perfect word, else 1 segment; carries any overflow
/// across a forge; caps at `SHIELD_CAP` held. At the cap, forging FREEZES
/// (nothing granted). Returns [`Forge`] describing what happened, for the HUD.
///
/// Must NOT be called for a word rescued by a shield/retry (that "correct" isn't
/// a genuine correct and forges nothing).
pub fn forge_on_correct(state: &mut AppState, perfect: bool) -> Forge {
    forge_grant(&mut state.aids, &FORGE, perfect)
}

/// Pure forge-grant core (config-injected so tests can exercise variants). See
/// [`forge_on_correct`].
pub fn forge_grant(a: &mut RunAids, cfg: &ForgeConfig, perfect: bool) -> Forge {
    cfg.validate();
    // At the cap, forging is FROZEN — a correct word grants no segments.
    if a.shields >= cfg.shield_cap {
        a.segments = 0; // hold nothing partial while full ("shields full")
        return Forge::Frozen;
    }
    let gain = if perfect { cfg.perfect_segments } else { 1 };
    a.segments = a.segments.saturating_add(gain);
    let mut forged = false;
    while a.segments >= cfg.segments_per_shield && a.shields < cfg.shield_cap {
        a.shields += 1;
        forged = true;
        if cfg.overflow_carry {
            a.segments -= cfg.segments_per_shield; // carry the remainder
        } else {
            a.segments = 0; // non-carry stub: discard remainder
            break;
        }
    }
    // Reaching the cap freezes forging: hold no partial (spending re-opens at 0).
    if a.shields >= cfg.shield_cap {
        a.segments = 0;
    }
    if forged {
        Forge::Forged
    } else {
        Forge::Progress
    }
}

/// A REAL miss resets forge progress to 0 segments — but NEVER removes an
/// already-forged shield. Applies to an UNSHIELDED miss (no shield, or the player
/// declined) and to a FAILED second attempt after a shield. A shield ABSORBING a
/// miss must NOT call this (the absorbed miss isn't a miss for forge purposes).
pub fn forge_reset(state: &mut AppState) {
    state.aids.segments = 0;
}

/// Whether forging is currently frozen because the player is at the shield cap.
pub fn forge_frozen(state: &AppState) -> bool {
    state.aids.shields >= FORGE.shield_cap
}

/// Whether a Climb miss may be shielded: at least one shield held AND none spent
/// on this word yet (a second shield can never rescue the same word).
pub fn shield_available(state: &AppState) -> bool {
    state.aids.shields > 0 && !state.aids.retry_used
}

/// Player ACCEPTED the "Use a shield?" prompt: consume exactly one shield and
/// spend the word's one retry. Returns `false` (no-op) if none is available or
/// one was already spent — accounting stays exact and never goes negative.
///
/// Spending does NOT reset forge progress (the absorbed miss isn't a forge miss);
/// because the shield count drops below the cap, forging re-opens at whatever
/// segment count stood before (0 when the freeze had cleared it).
pub fn spend_shield(state: &mut AppState) -> bool {
    if state.aids.shields == 0 || state.aids.retry_used {
        return false;
    }
    state.aids.shields -= 1;
    state.aids.retry_used = true;
    true
}

#[cfg(test)]
mod tests {
    //! Forge-math unit tests + the property/matrix invariants. The
    //! language/entitlement-matrix tests run the SAME closure over a matrix,
    //! proving one implementation serves every language and entitlement level
    //! with zero branching.
    use super::*;
    use crate::misses;
    use crate::model::AppState;

    fn st() -> AppState {
        AppState::default()
    }

    fn miss_count(state: &AppState, word: &str, lang: &str) -> u32 {
        let key = misses::miss_key(word, lang);
        state
            .misses
            .iter()
            .find(|m| misses::miss_key(&m.word, &m.lang) == key)
            .map(|m| m.misses)
            .unwrap_or(0)
    }

    // ----- Feature 1: extra attempts -----

    #[test]
    fn a1_on_miss_then_correct_records_word_and_grants_one_retry() {
        let mut s = st();
        start_word(&mut s);
        let retry = maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0);
        assert!(retry, "extra attempt should be granted on the first miss");
        assert_eq!(miss_count(&s, "apple", "en"), 1, "first miss recorded once");
        assert!(s.aids.retry_used);
        assert_eq!(miss_count(&s, "apple", "en"), 1, "word still recorded exactly once");
    }

    #[test]
    fn a2_on_miss_then_miss_marks_word_missed_exactly_once() {
        let mut s = st();
        start_word(&mut s);
        assert!(maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0));
        let again = maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 2000.0);
        assert!(!again, "no second retry for the same word");
        assert_eq!(miss_count(&s, "apple", "en"), 1, "word marked missed ONCE, not twice");
    }

    #[test]
    fn a3_toggle_off_is_production_parity() {
        let mut s = st();
        start_word(&mut s);
        let retry = maybe_extra_attempt(false, &mut s, "apple", "en", "easy", 1000.0);
        assert!(!retry);
        assert!(!s.aids.retry_used);
        assert_eq!(miss_count(&s, "apple", "en"), 0);
    }

    #[test]
    fn a5_retry_starts_empty_and_fires_once_per_word() {
        let mut s = st();
        start_word(&mut s);
        s.answer = "wrng".to_string();
        assert!(maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0));
        prepare_retry(&mut s);
        assert_eq!(s.answer, "", "retry field starts empty (not pre-filled)");
        assert!(!maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 2000.0), "audio replay fires exactly once");
    }

    // ----- Feature 2: the FORGE — math unit tests -----

    #[test]
    fn forge_basic_five_assisted_forges_on_the_fifth() {
        // "5 assisted forge on the 5th": 1 segment/word, forge on word 5.
        let mut s = st();
        for i in 1..=4 {
            assert_eq!(forge_on_correct(&mut s, false), Forge::Progress, "word {i} no forge");
            assert_eq!(s.aids.segments, i as u8);
            assert_eq!(s.aids.shields, 0);
        }
        assert_eq!(forge_on_correct(&mut s, false), Forge::Forged, "the 5th assisted word forges");
        assert_eq!(s.aids.shields, 1);
        assert_eq!(s.aids.segments, 0, "no overflow from exactly 5");
    }

    #[test]
    fn forge_perfect_word_grants_two_segments() {
        let mut s = st();
        assert_eq!(forge_on_correct(&mut s, true), Forge::Progress);
        assert_eq!(s.aids.segments, 2, "a perfect word is worth 2 segments");
        // Using syllable replay never penalizes: a plain correct still grants 1.
        assert_eq!(forge_on_correct(&mut s, false), Forge::Progress);
        assert_eq!(s.aids.segments, 3);
    }

    #[test]
    fn forge_overflow_carries_2plus2plus2_forges_on_the_third_perfect() {
        // "2+2+2 forges on the 3rd perfect word", with 1 segment overflow.
        let mut s = st();
        assert_eq!(forge_on_correct(&mut s, true), Forge::Progress); // 2
        assert_eq!(s.aids.segments, 2);
        assert_eq!(forge_on_correct(&mut s, true), Forge::Progress); // 4
        assert_eq!(s.aids.segments, 4);
        assert_eq!(forge_on_correct(&mut s, true), Forge::Forged, "3rd perfect forges"); // 6 -> forge
        assert_eq!(s.aids.shields, 1);
        assert_eq!(s.aids.segments, 1, "the 6th segment carries as 1/5 of the next shield");
    }

    #[test]
    fn forge_unshielded_miss_resets_segments_only() {
        let mut s = st();
        forge_on_correct(&mut s, true); // 2
        forge_on_correct(&mut s, false); // 3
        assert_eq!(s.aids.segments, 3);
        forge_reset(&mut s); // unshielded miss
        assert_eq!(s.aids.segments, 0, "miss resets forge progress");
        assert_eq!(s.aids.shields, 0);
    }

    #[test]
    fn forge_miss_never_removes_a_forged_shield() {
        let mut s = st();
        for _ in 0..5 {
            forge_on_correct(&mut s, false);
        }
        assert_eq!(s.aids.shields, 1);
        forge_on_correct(&mut s, false); // segment 1 toward next
        assert_eq!(s.aids.segments, 1);
        forge_reset(&mut s);
        assert_eq!(s.aids.segments, 0, "progress wiped");
        assert_eq!(s.aids.shields, 1, "the forged shield is never removed");
    }

    #[test]
    fn forge_cap_is_two_and_freezes() {
        let mut s = st();
        // Forge 2 shields (10 assisted corrects).
        for _ in 0..10 {
            forge_on_correct(&mut s, false);
        }
        assert_eq!(s.aids.shields, 2, "cap is 2 (changed from the old cap of 3)");
        assert_eq!(s.aids.segments, 0);
        // At the cap forging FREEZES: further corrects grant nothing.
        assert_eq!(forge_on_correct(&mut s, true), Forge::Frozen, "frozen at cap");
        assert_eq!(s.aids.segments, 0, "no segments while full");
        assert_eq!(s.aids.shields, 2, "still capped at 2");
        assert!(forge_frozen(&s));
    }

    #[test]
    fn forge_spend_reopens_forging_at_zero() {
        let mut s = st();
        for _ in 0..10 {
            forge_on_correct(&mut s, false);
        }
        assert_eq!(s.aids.shields, 2);
        assert!(forge_frozen(&s));
        // A miss on a new word: spend one shield to absorb it.
        start_word(&mut s);
        assert!(shield_available(&s));
        assert!(spend_shield(&mut s));
        assert_eq!(s.aids.shields, 1, "one shield spent");
        assert!(!forge_frozen(&s), "forging re-opens once below the cap");
        assert_eq!(s.aids.segments, 0, "re-opens at 0 segments");
        // Forging resumes normally.
        start_word(&mut s);
        assert_eq!(forge_on_correct(&mut s, false), Forge::Progress);
        assert_eq!(s.aids.segments, 1);
    }

    #[test]
    fn forge_shield_absorbed_miss_preserves_segments() {
        let mut s = st();
        // Hold 1 shield with 3 segments of progress toward the next.
        for _ in 0..5 {
            forge_on_correct(&mut s, false);
        }
        assert_eq!(s.aids.shields, 1);
        forge_on_correct(&mut s, false);
        forge_on_correct(&mut s, false);
        forge_on_correct(&mut s, false);
        assert_eq!(s.aids.segments, 3);
        // Miss on a new word, absorbed by the shield: segments are PRESERVED.
        start_word(&mut s);
        assert!(spend_shield(&mut s));
        assert_eq!(s.aids.shields, 0);
        assert_eq!(s.aids.segments, 3, "a shield-absorbed miss does NOT reset forge progress");
    }

    #[test]
    fn forge_failed_second_attempt_after_shield_resets() {
        let mut s = st();
        // 1 shield, 3 segments.
        for _ in 0..5 {
            forge_on_correct(&mut s, false);
        }
        forge_on_correct(&mut s, false);
        forge_on_correct(&mut s, false);
        forge_on_correct(&mut s, false);
        assert_eq!(s.aids.segments, 3);
        // Miss -> spend shield (preserve) -> retry ALSO wrong -> real miss resets.
        start_word(&mut s);
        assert!(spend_shield(&mut s));
        assert_eq!(s.aids.segments, 3, "absorbed miss preserves");
        // The retry is wrong -> a real miss: reset now.
        forge_reset(&mut s);
        assert_eq!(s.aids.segments, 0, "failed second attempt after a shield resets");
    }

    #[test]
    fn forge_perfect_defeated_by_retry_or_syllable_replay() {
        let mut s = st();
        start_word(&mut s);
        assert!(is_perfect_word(&s), "fresh word is perfect");
        note_syllable_replay(&mut s);
        assert!(!is_perfect_word(&s), "syllable replay defeats perfect");
        start_word(&mut s);
        s.aids.retry_used = true;
        assert!(!is_perfect_word(&s), "a used retry defeats perfect");
    }

    #[test]
    fn a11_language_matrix_forge_identical_code() {
        // Shields never read language: the identical sequence yields identical
        // state for every language (incl. a composition-input one).
        let mut ref_aids = None;
        for _lang in ["en", "ko", "zh", "es", "ja"] {
            let mut s = st();
            for i in 0..7 {
                forge_on_correct(&mut s, i % 2 == 0); // mixed perfect/assisted
            }
            start_word(&mut s);
            spend_shield(&mut s);
            match &ref_aids {
                None => ref_aids = Some(s.aids.clone()),
                Some(r) => assert_eq!(&s.aids, r, "identical forge state across languages"),
            }
        }
    }

    #[test]
    fn i8_accounting_shields_never_negative_never_above_cap() {
        let mut s = st();
        for round in 0..60 {
            forge_on_correct(&mut s, round % 3 == 0);
            if round % 7 == 0 {
                start_word(&mut s);
                spend_shield(&mut s);
            }
            assert!(s.aids.shields <= FORGE.shield_cap, "never above cap");
            assert!(s.aids.segments < FORGE.segments_per_shield, "segments always < a full shield");
        }
    }

    #[test]
    fn a12_no_validated_event_no_forge_or_spend() {
        let mut s = st();
        start_word(&mut s);
        assert_eq!(s.aids, RunAids::default());
    }

    #[test]
    fn reset_run_clears_everything() {
        let mut s = st();
        s.aids.shields = 2;
        s.aids.segments = 4;
        s.aids.retry_used = true;
        s.aids.syllable_replay_used = true;
        reset_run(&mut s);
        assert_eq!(s.aids, RunAids::default());
    }

    // ----- Determinism / replay property test -----

    /// A recorded in-run event sequence replayed through the forge produces the
    /// SAME forge/shield state at every step — no randomness, no hidden inputs.
    #[test]
    fn determinism_same_sequence_same_timeline() {
        #[derive(Clone, Copy)]
        enum Ev {
            Perfect,
            Correct,
            Miss,          // real (unshielded / failed-retry) miss
            ShieldedMiss,  // a miss absorbed by a spent shield
        }
        use Ev::*;
        let seq = [
            Perfect, Correct, Perfect, Miss, Correct, Correct, Perfect, Correct, Correct,
            ShieldedMiss, Correct, Perfect, Perfect, Perfect, Miss, Correct, Correct, Correct,
        ];
        let run = |seq: &[Ev]| -> Vec<(u8, u8)> {
            let mut s = st();
            let mut timeline = Vec::new();
            for ev in seq {
                start_word(&mut s);
                match ev {
                    Perfect => {
                        forge_on_correct(&mut s, true);
                    }
                    Correct => {
                        forge_on_correct(&mut s, false);
                    }
                    Miss => forge_reset(&mut s),
                    ShieldedMiss => {
                        // Only meaningful when a shield is held; otherwise a no-op
                        // spend (accounting never goes negative).
                        spend_shield(&mut s);
                    }
                }
                timeline.push((s.aids.shields, s.aids.segments));
            }
            timeline
        };
        let a = run(&seq);
        let b = run(&seq);
        assert_eq!(a, b, "identical event sequence -> identical forge/shield timeline");
        // And the timeline is non-trivial (it actually forged and spent).
        assert!(a.iter().any(|&(sh, _)| sh >= 1), "the sequence forged at least one shield");
    }

    /// A recorded Ghost-Racing track (its per-word correct/incorrect log) replayed
    /// through the forge reproduces a shield timeline exactly on every replay. The
    /// ghost log only carries correct/incorrect, so each correct is treated as a
    /// (non-perfect) forge grant and each incorrect as a real miss — a pure,
    /// deterministic mapping.
    #[test]
    fn determinism_ghost_track_replays_identically() {
        use crate::ghost::{GhostEvent, GhostRun};
        let track = GhostRun {
            events: vec![
                GhostEvent { elapsed_ms: 1000, correct: true },
                GhostEvent { elapsed_ms: 2000, correct: true },
                GhostEvent { elapsed_ms: 3200, correct: true },
                GhostEvent { elapsed_ms: 4500, correct: true },
                GhostEvent { elapsed_ms: 6000, correct: true },
                GhostEvent { elapsed_ms: 7000, correct: true },
                GhostEvent { elapsed_ms: 9000, correct: false },
            ],
        };
        let replay = |t: &GhostRun| -> Vec<(u8, u8)> {
            let mut s = st();
            let mut out = Vec::new();
            for e in &t.events {
                start_word(&mut s);
                if e.correct {
                    forge_on_correct(&mut s, false);
                } else {
                    forge_reset(&mut s);
                }
                out.push((s.aids.shields, s.aids.segments));
            }
            out
        };
        let first = replay(&track);
        let second = replay(&track);
        assert_eq!(first, second, "same ghost track -> same forge timeline every replay");
        // 6 corrects -> 1 shield forged (at the 5th) + 1 carry segment, then a
        // miss wipes the carry but keeps the shield.
        assert_eq!(*first.last().unwrap(), (1, 0), "final state: 1 shield held, forge reset by the miss");
    }

    // ----- Config stubs: enabling a season-2 feature FAILS -----

    #[test]
    #[should_panic(expected = "season-2 feature not implemented")]
    fn season2_heat_stub_enabled_fails_ci() {
        let mut cfg = FORGE;
        cfg.heat = true;
        let mut a = RunAids::default();
        forge_grant(&mut a, &cfg, false); // must panic on validate()
    }

    #[test]
    #[should_panic(expected = "season-2 feature not implemented")]
    fn season2_clutch_stub_enabled_fails_ci() {
        let mut cfg = FORGE;
        cfg.clutch = true;
        let mut a = RunAids::default();
        forge_grant(&mut a, &cfg, false); // must panic on validate()
    }

    #[test]
    fn ship_config_is_season1_only() {
        // The shipped constants are exactly the CC-CLIMB-SHIELDS season-1 values.
        assert_eq!(FORGE.segments_per_shield, 5);
        assert_eq!(FORGE.perfect_segments, 2);
        assert_eq!(FORGE.shield_cap, 2, "cap CHANGED 3 -> 2");
        assert!(FORGE.overflow_carry);
        assert!(!FORGE.heat && !FORGE.clutch, "season-2 stubs disabled");
        FORGE.validate(); // the shipped config is valid
    }
}
