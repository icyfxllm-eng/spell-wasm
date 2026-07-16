//! CC-ATTEMPTS-SHIELDS — the cross-language attempts & shields subsystem.
//!
//! THE CONTRACT: an "attempt" is ONE submitted answer that passed through the
//! single validation path (`game::submit_guess` -> `on_correct`/`on_wrong`).
//! Everything here is driven by exactly two events — validated-correct and
//! validated-incorrect — and NEVER reads the language, input method, or raw
//! text. There is deliberately not a single language conditional in this file:
//! one implementation serves all 17 languages, and the per-language test
//! coverage is the SAME test code run over a language matrix.
//!
//! Two features share this state machine:
//!   * Feature 1 — "Extra attempt on misses" (normal mode): one clean retry of
//!     a missed word, with the first miss still recorded for spaced repetition.
//!   * Feature 2 — shields (The Climb): earn a shield every few correct answers,
//!     spend one (by choice) to retry a missed word during a run.
//!
//! All logic is pure integer state (no DOM, no storage, no `js_sys`), so it runs
//! under `cargo test --lib` on a non-wasm target; `game.rs` wraps these calls
//! with the audio/animation/DOM around them.

use crate::model::AppState;

/// A shield is earned every this-many consecutive validated-correct answers.
pub const SHIELD_EARN_STREAK: u32 = 5;
/// The most shields a player can hold during a single Climb run.
pub const SHIELD_CAP: u8 = 3;

/// Per-RUN aids state (I7): lives in `AppState` next to the run's score/streak,
/// is session-only, and resets to all-zero at the start of every run. Shields
/// do NOT persist between runs and can never be purchased.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct RunAids {
    /// Shields held right now (0..=`SHIELD_CAP`). Only ever earned, never bought.
    pub shields: u8,
    /// Consecutive validated-correct toward the next shield. Resets on ANY miss.
    pub earn_streak: u32,
    /// True once the current word has already consumed its ONE retry (an extra
    /// attempt or a shield). Enforces "at most one retry per word" (I4 / PD5 no
    /// chaining). Cleared when a new word starts.
    pub retry_used: bool,
}

// ---------- run / word lifecycle ----------

/// Start of a Climb run (or any fresh chain): drop all aids to zero. Shields are
/// per-run and never carry over.
pub fn reset_run(state: &mut AppState) {
    state.aids = RunAids::default();
}

/// A new word is now on screen: the one-retry budget refreshes. Earn streak and
/// held shields are untouched (they span the whole run).
pub fn start_word(state: &mut AppState) {
    state.aids.retry_used = false;
}

// ---------- Feature 1: extra attempts (normal mode) ----------

/// Gate + apply a validated-INCORRECT under the extra-attempts toggle.
///
/// When `enabled` and the word has not yet used its retry: record the FIRST miss
/// to spaced repetition exactly once (the data-integrity rule — recorded
/// REGARDLESS of what the retry then does), mark the retry as used, and return
/// `true` so the caller replays audio + clears the input. Otherwise return
/// `false` (toggle off, or the retry is already spent -> the caller runs the
/// normal miss consequence and records NOTHING further: the first submission
/// alone decides the spaced-rep record — I2).
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
/// is never pre-filled (A5). Audio replay + focus are the caller's DOM concern.
pub fn prepare_retry(state: &mut AppState) {
    state.answer.clear();
}

// ---------- Feature 2: shields (The Climb) ----------

/// Validated-CORRECT in a Climb run: advance the earn streak; every
/// `SHIELD_EARN_STREAK` grant one shield up to `SHIELD_CAP` (earning at the cap
/// is a no-op). Returns `true` iff a shield was just earned (HUD "earned"
/// feedback). Must NOT be called for a word that was rescued by a shield/retry
/// (PD4: that "correct" doesn't extend anything).
pub fn shield_on_correct(state: &mut AppState) -> bool {
    let a = &mut state.aids;
    a.earn_streak += 1;
    if a.earn_streak >= SHIELD_EARN_STREAK {
        a.earn_streak = 0;
        if a.shields < SHIELD_CAP {
            a.shields += 1;
            return true;
        }
    }
    false
}

/// Whether a Climb miss may be shielded: at least one shield held AND none spent
/// on this word yet (PD5 — a second shield can never rescue the same word).
pub fn shield_available(state: &AppState) -> bool {
    state.aids.shields > 0 && !state.aids.retry_used
}

/// Any Climb miss resets the shield EARN streak (PD2 — the miss was real, even
/// if it is about to be shielded). Call on every validated-incorrect in a run,
/// before the accept/decline prompt.
pub fn shield_note_miss(state: &mut AppState) {
    state.aids.earn_streak = 0;
}

/// Player ACCEPTED the "Use a shield?" prompt: consume exactly one shield and
/// spend the word's one retry. Returns `false` (no-op) if none is available or
/// one was already spent — accounting stays exact and never goes negative (I8).
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
    //! A1–A13 (the unit-testable ones). The language-matrix tests (A4, A11) run
    //! the SAME closure over `en`, `es`, and a composition-input language (`ja`)
    //! — proving one implementation serves every language with zero per-language
    //! branching.
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
        // First (wrong) submission: retry granted, miss recorded.
        let retry = maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0);
        assert!(retry, "extra attempt should be granted on the first miss");
        assert_eq!(miss_count(&s, "apple", "en"), 1, "first miss recorded once");
        // Retry submission is CORRECT: caller records nothing more (no add_miss),
        // does not bump the streak. The spaced-rep record is unchanged.
        assert!(s.aids.retry_used);
        assert_eq!(miss_count(&s, "apple", "en"), 1, "missed-words store still CONTAINS the word exactly once");
    }

    #[test]
    fn a2_on_miss_then_miss_marks_word_missed_exactly_once() {
        let mut s = st();
        start_word(&mut s);
        assert!(maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0));
        // Retry submission is ALSO wrong: no second retry, no second record.
        let again = maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 2000.0);
        assert!(!again, "no second retry for the same word (I4)");
        assert_eq!(miss_count(&s, "apple", "en"), 1, "word marked missed ONCE, not twice");
    }

    #[test]
    fn a3_toggle_off_is_production_parity() {
        let mut s = st();
        start_word(&mut s);
        // Toggle OFF: the new path is never entered — no retry, nothing recorded
        // here (the legacy path owns miss recording unchanged).
        let retry = maybe_extra_attempt(false, &mut s, "apple", "en", "easy", 1000.0);
        assert!(!retry);
        assert!(!s.aids.retry_used);
        assert_eq!(miss_count(&s, "apple", "en"), 0);
    }

    #[test]
    fn a4_language_matrix_identical_code() {
        // Same closure, three languages incl. a composition-input one (ja).
        for lang in ["en", "es", "ja"] {
            let mut s = st();
            start_word(&mut s);
            let retry = maybe_extra_attempt(true, &mut s, "word", lang, "easy", 1000.0);
            assert!(retry, "{lang}: retry granted");
            assert_eq!(miss_count(&s, "word", lang), 1, "{lang}: recorded once");
            let again = maybe_extra_attempt(true, &mut s, "word", lang, "easy", 2000.0);
            assert!(!again, "{lang}: only one retry");
            assert_eq!(miss_count(&s, "word", lang), 1, "{lang}: still once");
        }
    }

    #[test]
    fn a5_retry_starts_empty_and_fires_once_per_word() {
        let mut s = st();
        start_word(&mut s);
        s.answer = "wrng".to_string();
        // Exactly one retry (== exactly one audio replay + one input clear).
        assert!(maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 1000.0));
        prepare_retry(&mut s);
        assert_eq!(s.answer, "", "retry field starts empty (not pre-filled)");
        assert!(!maybe_extra_attempt(true, &mut s, "apple", "en", "easy", 2000.0), "audio replay fires exactly once");
    }

    #[test]
    fn i2_spaced_rep_identical_regardless_of_retry_outcome() {
        // Miss -> correct.
        let mut a = st();
        start_word(&mut a);
        maybe_extra_attempt(true, &mut a, "apple", "en", "easy", 1000.0);
        // (retry correct: nothing more recorded)
        // Miss -> miss.
        let mut b = st();
        start_word(&mut b);
        maybe_extra_attempt(true, &mut b, "apple", "en", "easy", 1000.0);
        maybe_extra_attempt(true, &mut b, "apple", "en", "easy", 2000.0); // retry wrong
        assert_eq!(miss_count(&a, "apple", "en"), miss_count(&b, "apple", "en"), "byte-for-byte identical spaced-rep input");
    }

    // ----- Feature 2: shields -----

    #[test]
    fn a6_earn_curve_5_10_15_20_cap() {
        let mut s = st();
        for i in 1..=20u32 {
            shield_on_correct(&mut s);
            match i {
                5 => assert_eq!(s.aids.shields, 1, "5 -> 1"),
                10 => assert_eq!(s.aids.shields, 2, "10 -> 2"),
                15 => assert_eq!(s.aids.shields, 3, "15 -> 3"),
                20 => assert_eq!(s.aids.shields, 3, "20 -> 3 (cap)"),
                _ => {}
            }
        }
        assert!(s.aids.shields <= SHIELD_CAP, "never above cap (I8)");
    }

    #[test]
    fn a7_miss_resets_earn_streak_no_shield_from_four_plus_four() {
        let mut s = st();
        for _ in 0..4 {
            assert!(!shield_on_correct(&mut s));
        }
        assert_eq!(s.aids.earn_streak, 4);
        shield_note_miss(&mut s); // any miss resets the earning streak
        assert_eq!(s.aids.earn_streak, 0);
        for _ in 0..4 {
            assert!(!shield_on_correct(&mut s), "4 more correct != a shield");
        }
        assert_eq!(s.aids.shields, 0);
        assert!(shield_on_correct(&mut s), "the 5th does");
        assert_eq!(s.aids.shields, 1);
    }

    #[test]
    fn a8_accept_prompt_consumes_one_and_correct_retry_continues() {
        let mut s = st();
        s.aids.shields = 1;
        start_word(&mut s);
        // Miss during the run.
        shield_note_miss(&mut s);
        assert!(shield_available(&s), "prompt offered");
        assert!(spend_shield(&mut s), "accept consumes the shield");
        assert_eq!(s.aids.shields, 0, "count 0 after spend");
        assert!(s.aids.retry_used);
        // No chaining: a second shield can't rescue the same word even if held.
        s.aids.shields = 2;
        assert!(!shield_available(&s), "no second shield on the same word (PD5)");
        assert!(!spend_shield(&mut s));
        // Correct on retry -> run continues; earn streak was reset, does NOT
        // extend (caller skips shield_on_correct for a rescued word).
        assert_eq!(s.aids.earn_streak, 0);
    }

    #[test]
    fn a9_accept_then_miss_again_is_normal_consequence_one_consumed() {
        let mut s = st();
        s.aids.shields = 1;
        start_word(&mut s);
        shield_note_miss(&mut s);
        assert!(spend_shield(&mut s));
        assert_eq!(s.aids.shields, 0);
        // Retry also wrong: normal miss consequence, nothing more consumed.
        assert!(!shield_available(&s));
        assert!(!spend_shield(&mut s), "exactly one shield consumed");
        assert_eq!(s.aids.shields, 0);
    }

    #[test]
    fn a9_decline_keeps_the_shield() {
        let mut s = st();
        s.aids.shields = 1;
        start_word(&mut s);
        shield_note_miss(&mut s);
        assert!(shield_available(&s));
        // DECLINE: do not spend -> normal miss consequence, shield still held.
        assert_eq!(s.aids.shields, 1, "declining keeps the shield");
        assert!(!s.aids.retry_used);
    }

    #[test]
    fn a11_language_matrix_earn_and_spend_identical_code() {
        // Shields never read language: the identical sequence yields identical
        // state for every language.
        for _lang in ["en", "es", "ja"] {
            let mut s = st();
            for _ in 0..5 {
                shield_on_correct(&mut s);
            }
            assert_eq!(s.aids.shields, 1);
            start_word(&mut s);
            shield_note_miss(&mut s);
            assert!(spend_shield(&mut s));
            assert_eq!(s.aids.shields, 0);
        }
    }

    #[test]
    fn a12_no_validated_event_no_earn_or_spend() {
        // Keystrokes / IME composition never reach this subsystem: with zero
        // validated events, aids stay at their default.
        let mut s = st();
        start_word(&mut s);
        assert_eq!(s.aids, RunAids::default());
    }

    #[test]
    fn i8_accounting_shields_equal_earns_minus_spends_never_negative() {
        let mut s = st();
        let mut earns = 0u32;
        let mut spends = 0u32;
        for round in 0..40 {
            if shield_on_correct(&mut s) {
                earns += 1;
            }
            if round % 7 == 0 {
                start_word(&mut s);
                shield_note_miss(&mut s);
                if spend_shield(&mut s) {
                    spends += 1;
                }
            }
            assert!(s.aids.shields <= SHIELD_CAP);
        }
        assert_eq!(s.aids.shields as u32, earns - spends, "shields == earns - spends");
    }

    #[test]
    fn reset_run_clears_everything() {
        let mut s = st();
        s.aids.shields = 3;
        s.aids.earn_streak = 4;
        s.aids.retry_used = true;
        reset_run(&mut s);
        assert_eq!(s.aids, RunAids::default());
    }
}
