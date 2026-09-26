//! CC-FEEDBACK F1 — the one place an outcome becomes a feedback state.
//!
//! # Why this is not a new outcome enum
//!
//! The §0 census found six grading paths with six unrelated shapes and no
//! shared type, and F1 asks for an exhaustive match with no wildcard arm. The
//! obvious reading is that every mode must be converted to one outcome enum —
//! but that would mean editing every grading path, and the file's own header
//! says grading logic is not touched.
//!
//! It does not have to. Each mode keeps ITS OWN outcome type and converts into
//! the shared [`State`] here. Rust already enforces exhaustiveness on a match
//! over a local enum, so adding a variant to any mode's outcome breaks the
//! build at its conversion until someone decides what the player should feel.
//! That is I1's guarantee, per mode, without a single verdict changing.
//!
//! Every conversion lives in THIS file. A mode that maps its own outcome to a
//! colour at the call site is the thing F1 exists to end.
//!
//! # The one divergence, deliberately left in place
//!
//! [`State`] is what the player SHOULD feel. [`crate::game::Outcome::css_class`]
//! is what the renderer draws TODAY. They already disagree in one case:
//! `OtherSense` is a `Close` state that currently renders green, because an
//! accepted other-sense answer goes through `on_correct`.
//!
//! That disagreement is not an oversight and must not be "fixed" here. D3 signs
//! `valid_other_sense` as `close`, and making the amber actually appear is F2's
//! job, in Phase B, where it is a visible change a player will notice. This
//! phase types the plumbing and changes nothing on screen.

/// The four states of the feedback grammar (F1).
///
/// Four, not two, because SpellGame has more than two outcomes and a
/// green/red binary would lie about near-misses, other-sense answers and
/// voided rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Success,
    Close,
    Miss,
    Neutral,
}

// ── the base game: practice, Daily, Climb ───────────────────────────────────
impl From<crate::game::Outcome> for State {
    fn from(o: crate::game::Outcome) -> Self {
        use crate::game::Outcome as O;
        match o {
            O::Correct => State::Success,
            // D3, signed: a real other sense of the prompt is not a miss, and
            // it is not a plain success either -- the player spelled a word the
            // audio genuinely supports, just not the one on the card.
            O::OtherSense => State::Close,
            O::Misspelled => State::Miss,
            O::TimedOut => State::Miss,
            // Giving up is not a failure of spelling, and a voided round is not
            // the player's failure at all (AUDIO-CLARITY D9: they lose nothing
            // by saying they could not hear it).
            O::GaveUp => State::Neutral,
            O::Voided => State::Neutral,
        }
    }
}

// ── SpellDoku ───────────────────────────────────────────────────────────────
#[cfg(not(feature = "web"))]
impl From<crate::spelldoku::play::Verdict> for State {
    fn from(v: crate::spelldoku::play::Verdict) -> Self {
        use crate::spelldoku::play::Verdict as V;
        match v {
            V::Correct => State::Success,
            V::Misspelled => State::Miss,
            // These two had no row in the spec's table, and the gap mattered:
            // in both the player SPELLED CORRECTLY and placed wrong. Calling
            // that a miss tells them their spelling was wrong when it was not,
            // which is the exact lie the four-state grammar exists to prevent.
            // Claude's recommendation, applied on Eric's "do it" of 2026-09-25;
            // one line each to reverse.
            V::WrongValue => State::Close,
            V::WrongSystem => State::Close,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Outcome as O;

    /// The mapping table from the spec, pinned. If a row changes, it changes
    /// here and in the spec together, not by accident.
    #[test]
    fn f1_table() {
        for (outcome, want) in [
            (O::Correct, State::Success),
            (O::OtherSense, State::Close),
            (O::Misspelled, State::Miss),
            (O::TimedOut, State::Miss),
            (O::GaveUp, State::Neutral),
            (O::Voided, State::Neutral),
        ] {
            assert_eq!(State::from(outcome), want, "{outcome:?} maps to the wrong state");
        }
    }

    /// I3-adjacent, and the reason this phase is safe to ship on its own: the
    /// class the renderer sets is byte-for-byte what it set before the refactor.
    /// These strings were read out of game.rs, not invented.
    #[test]
    fn the_rendered_class_is_unchanged() {
        assert_eq!(O::Correct.css_class(), "feedback good");
        assert_eq!(O::OtherSense.css_class(), "feedback good");
        assert_eq!(O::Misspelled.css_class(), "feedback bad");
        assert_eq!(O::TimedOut.css_class(), "feedback bad");
        assert_eq!(O::GaveUp.css_class(), "feedback");
    }

    /// The documented divergence, asserted so that removing it is a deliberate
    /// act with a failing test attached, not a quiet drift.
    #[test]
    fn other_sense_is_close_but_still_renders_green_until_f2() {
        assert_eq!(State::from(O::OtherSense), State::Close);
        assert_eq!(O::OtherSense.css_class(), O::Correct.css_class());
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn spelldoku_placement_errors_are_not_spelling_errors() {
        use crate::spelldoku::play::Verdict as V;
        assert_eq!(State::from(V::Correct), State::Success);
        assert_eq!(State::from(V::Misspelled), State::Miss);
        assert_eq!(State::from(V::WrongValue), State::Close);
        assert_eq!(State::from(V::WrongSystem), State::Close);
    }
}
