//! CC-VISUAL-ECLIPSE F3 -- the orb as a living character: the pure core.
//!
//! Seven states and the table of moves between them, the speech envelope
//! (V-D13: RMS at 60 Hz, read against the playback clock), and the small timing
//! rules (the fetching grace, the idle rest). No DOM here; `orb_view` applies
//! all of it to `#orbWrap` behind `visual.orb`.
//!
//! The table is the contract (F3.2). A move it does not list is a wiring bug:
//! debug builds panic on it (AT-10), and release builds refuse it, log it and
//! count it, so an e2e run can assert that a whole session made none.

/// What the orb is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    /// Slow breathing.
    Idle,
    /// The word is on its way. Shown only if it takes longer than the grace.
    Fetching,
    /// The corona follows the voice.
    Speaking,
    /// Spoken; the glow leans toward the input.
    Awaiting,
    /// Flare.
    Correct,
    /// A brief dim.
    Wrong,
    /// Totality (F7).
    Milestone,
}

#[cfg(test)]
pub const ALL: [State; 7] =
    [State::Idle, State::Fetching, State::Speaking, State::Awaiting, State::Correct, State::Wrong, State::Milestone];

impl State {
    /// The value of `#orbWrap[data-orb]`.
    pub fn name(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Fetching => "fetching",
            State::Speaking => "speaking",
            State::Awaiting => "awaiting",
            State::Correct => "correct",
            State::Wrong => "wrong",
            State::Milestone => "milestone",
        }
    }

    #[cfg(test)]
    pub fn from_name(s: &str) -> Option<State> {
        ALL.into_iter().find(|st| st.name() == s)
    }
}

/// The allowed moves. Staying put is not a move (see `step`).
///
/// - Anything may return to `Idle`: leaving the screen, a new round.
/// - An answer can arrive at any point after the word was asked for, even
///   before the audio lands, so `Fetching`, `Speaking` and `Awaiting` all reach
///   `Correct` and `Wrong`.
/// - "Hear it again" can be tapped at any point after the word was asked for,
///   including after the verdict, so those reach `Fetching` and `Speaking`.
/// - `Wrong` reaches `Awaiting` for a retry (extra attempts, shields).
/// - `Milestone` follows a verdict or rest and hands back to a new word.
pub const TABLE: &[(State, &[State])] = {
    use State::*;
    &[
        (Idle, &[Fetching, Speaking, Awaiting, Milestone]),
        (Fetching, &[Speaking, Awaiting, Correct, Wrong, Idle]),
        (Speaking, &[Awaiting, Fetching, Correct, Wrong, Idle]),
        (Awaiting, &[Fetching, Speaking, Correct, Wrong, Idle]),
        (Correct, &[Fetching, Speaking, Milestone, Idle]),
        (Wrong, &[Fetching, Speaking, Awaiting, Idle]),
        (Milestone, &[Fetching, Idle]),
    ]
};

pub fn allowed(from: State, to: State) -> bool {
    TABLE.iter().any(|(f, tos)| *f == from && tos.contains(&to))
}

/// The result of asking for a move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The orb moves to the new state.
    Moved,
    /// Already there; nothing to do.
    Stayed,
    /// Not in the table. The orb stays where it was.
    Refused,
}

/// Ask for `from -> to`. An undefined move panics in debug builds (AT-10).
pub fn step(from: State, to: State) -> Step {
    if from == to {
        return Step::Stayed;
    }
    if allowed(from, to) {
        return Step::Moved;
    }
    #[cfg(debug_assertions)]
    panic!("orb: undefined transition {} -> {}", from.name(), to.name());
    #[cfg(not(debug_assertions))]
    Step::Refused
}

/// `step` without the debug panic, for tests that walk the refusals.
pub fn check(from: State, to: State) -> Step {
    if from == to {
        Step::Stayed
    } else if allowed(from, to) {
        Step::Moved
    } else {
        Step::Refused
    }
}

/// The envelope rate (V-D13).
pub const ENVELOPE_HZ: f64 = 60.0;

/// Below this the orb treats the voice as silent; AT-8 times the first frame
/// above it.
pub const ENVELOPE_THRESHOLD: f32 = 0.12;

/// RMS of each 1/60 s window of `samples`, normalised so the loudest window is
/// 1.0. Silence (or nothing) gives all zeros, never NaN.
pub fn envelope(samples: &[f32], sample_rate: f32) -> Vec<f32> {
    if samples.is_empty() || sample_rate <= 0.0 {
        return Vec::new();
    }
    let win = ((sample_rate as f64 / ENVELOPE_HZ).round() as usize).max(1);
    let mut out: Vec<f32> = samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let peak = out.iter().cloned().fold(0.0f32, f32::max);
    if peak > 0.0 {
        for v in &mut out {
            *v /= peak;
        }
    }
    out
}

/// The envelope at `t` seconds of playback (already scaled by the playback
/// rate). Linear between frames; 0 before the start and after the end.
pub fn level_at(env: &[f32], t: f64) -> f32 {
    if env.is_empty() || t < 0.0 {
        return 0.0;
    }
    let x = t * ENVELOPE_HZ;
    let i = x.floor() as usize;
    if i + 1 >= env.len() {
        return if i < env.len() { env[i] } else { 0.0 };
    }
    let f = (x - i as f64) as f32;
    env[i] * (1.0 - f) + env[i + 1] * f
}

/// How long a clip lasts, from its envelope, in seconds of playback.
pub fn duration(env: &[f32]) -> f64 {
    env.len() as f64 / ENVELOPE_HZ
}

/// The first frame time (s) at or above the threshold, if the voice ever gets
/// there. AT-8's "first orb frame above the envelope threshold".
pub fn first_above(env: &[f32]) -> Option<f64> {
    env.iter().position(|v| *v >= ENVELOPE_THRESHOLD).map(|i| i as f64 / ENVELOPE_HZ)
}

/// Parse a motion token such as "120ms" or "30s" into milliseconds.
pub fn token_ms(v: &str) -> f64 {
    let v = v.trim();
    if let Some(ms) = v.strip_suffix("ms") {
        ms.trim().parse().unwrap_or(0.0)
    } else if let Some(s) = v.strip_suffix('s') {
        s.trim().parse::<f64>().map(|s| s * 1000.0).unwrap_or(0.0)
    } else {
        0.0
    }
}

/// The fetching look is skipped when the clip answers inside this (F3: under
/// 120 ms there is no flicker).
pub fn fetch_grace_ms() -> f64 {
    token_ms(crate::tokens::MOTION_ANIM_ORB_FETCH_GRACE.value)
}

/// The idle breath pauses after this long without input (V-D23).
pub fn rest_after_ms() -> f64 {
    token_ms(crate::tokens::MOTION_ANIM_ORB_REST_AFTER.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use State::*;

    #[test]
    fn the_table_covers_every_state_once() {
        for s in ALL {
            assert_eq!(TABLE.iter().filter(|(f, _)| *f == s).count(), 1, "{s:?}");
        }
    }

    #[test]
    fn no_state_lists_itself() {
        for (f, tos) in TABLE {
            assert!(!tos.contains(f), "{f:?} lists itself");
        }
    }

    #[test]
    fn every_state_reaches_idle_and_idle_reaches_the_word() {
        for s in ALL {
            if s != Idle {
                assert!(allowed(s, Idle), "{s:?} cannot return to idle");
            }
        }
        assert!(allowed(Idle, Fetching));
    }

    // AT-10, half one: every defined transition, named.
    #[test]
    fn the_word_cycle() {
        for (f, t) in [(Idle, Fetching), (Fetching, Speaking), (Speaking, Awaiting), (Awaiting, Correct), (Correct, Fetching)] {
            assert_eq!(step(f, t), Step::Moved, "{f:?} -> {t:?}");
        }
    }

    #[test]
    fn a_cache_hit_skips_fetching() {
        assert_eq!(step(Idle, Speaking), Step::Moved);
        assert_eq!(step(Correct, Speaking), Step::Moved);
        assert_eq!(step(Wrong, Speaking), Step::Moved);
        assert_eq!(step(Awaiting, Speaking), Step::Moved);
    }

    #[test]
    fn no_audio_goes_straight_to_awaiting() {
        assert_eq!(step(Fetching, Awaiting), Step::Moved);
        assert_eq!(step(Idle, Awaiting), Step::Moved);
    }

    #[test]
    fn an_answer_can_beat_the_audio() {
        for f in [Fetching, Speaking, Awaiting] {
            assert_eq!(step(f, Correct), Step::Moved);
            assert_eq!(step(f, Wrong), Step::Moved);
        }
    }

    #[test]
    fn hear_it_again_from_anywhere_after_the_ask() {
        for f in [Speaking, Awaiting, Correct, Wrong] {
            assert_eq!(step(f, Fetching), Step::Moved, "{f:?}");
        }
    }

    #[test]
    fn a_retry_returns_to_awaiting() {
        assert_eq!(step(Wrong, Awaiting), Step::Moved);
    }

    #[test]
    fn milestone_in_and_out() {
        assert_eq!(step(Idle, Milestone), Step::Moved);
        assert_eq!(step(Correct, Milestone), Step::Moved);
        assert_eq!(step(Milestone, Fetching), Step::Moved);
        assert_eq!(step(Milestone, Idle), Step::Moved);
    }

    #[test]
    fn every_listed_move_is_moved() {
        // The table and `step` agree, entry by entry: nothing listed is refused.
        let mut n = 0;
        for (f, tos) in TABLE {
            for t in *tos {
                assert_eq!(step(*f, *t), Step::Moved, "{f:?} -> {t:?}");
                n += 1;
            }
        }
        assert_eq!(n, 29, "a move was added or removed; name it in a test above");
    }

    #[test]
    fn staying_put_is_not_a_move() {
        for s in ALL {
            assert_eq!(step(s, s), Step::Stayed);
        }
    }

    // AT-10, half two: every undefined transition throws in debug.
    #[test]
    fn every_undefined_move_is_refused() {
        let mut refused = 0;
        for f in ALL {
            for t in ALL {
                if f != t && !allowed(f, t) {
                    assert_eq!(check(f, t), Step::Refused);
                    refused += 1;
                }
            }
        }
        assert_eq!(refused, 7 * 6 - 29);
    }

    #[test]
    fn every_undefined_move_panics_in_debug() {
        for f in ALL {
            for t in ALL {
                if f != t && !allowed(f, t) {
                    let r = std::panic::catch_unwind(|| step(f, t));
                    assert!(r.is_err(), "{f:?} -> {t:?} did not panic");
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "undefined transition milestone -> wrong")]
    fn a_named_undefined_move_panics() {
        step(Milestone, Wrong);
    }

    #[test]
    fn names_round_trip() {
        for s in ALL {
            assert_eq!(State::from_name(s.name()), Some(s));
        }
        assert_eq!(State::from_name("asleep"), None);
    }

    // --- the envelope ---

    #[test]
    fn envelope_is_one_value_per_sixtieth() {
        let sr = 48_000.0;
        let samples = vec![0.5f32; 48_000];
        let env = envelope(&samples, sr);
        assert_eq!(env.len(), 60);
        assert!(env.iter().all(|v| (*v - 1.0).abs() < 1e-6));
        assert!((duration(&env) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn envelope_follows_loudness_and_peaks_at_one() {
        let sr = 6000.0; // 100 samples per frame
        let mut s = vec![0.0f32; 100];
        s.extend(vec![0.2f32; 100]);
        s.extend(vec![0.8f32; 100]);
        let env = envelope(&s, sr);
        assert_eq!(env.len(), 3);
        assert_eq!(env[0], 0.0);
        assert!((env[1] - 0.25).abs() < 1e-6);
        assert!((env[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn silence_is_zero_not_nan() {
        let env = envelope(&[0.0; 1000], 44_100.0);
        assert!(env.iter().all(|v| *v == 0.0));
        assert!(envelope(&[], 44_100.0).is_empty());
        assert!(envelope(&[1.0], 0.0).is_empty());
    }

    #[test]
    fn a_negative_wave_is_as_loud_as_a_positive_one() {
        let a = envelope(&[0.5; 800], 48_000.0);
        let b = envelope(&[-0.5; 800], 48_000.0);
        assert_eq!(a, b);
    }

    #[test]
    fn level_interpolates_and_clamps() {
        let env = vec![0.0, 1.0, 0.0];
        assert_eq!(level_at(&env, -0.1), 0.0);
        assert!((level_at(&env, 0.5 / 60.0) - 0.5).abs() < 1e-6);
        assert!((level_at(&env, 1.0 / 60.0) - 1.0).abs() < 1e-6);
        assert_eq!(level_at(&env, 10.0), 0.0);
        assert_eq!(level_at(&[], 0.2), 0.0);
    }

    #[test]
    fn first_above_finds_the_onset() {
        let env = vec![0.0, 0.05, 0.3, 1.0];
        assert!((first_above(&env).unwrap() - 2.0 / 60.0).abs() < 1e-9);
        assert_eq!(first_above(&[0.0, 0.01]), None);
    }

    #[test]
    fn timing_tokens_parse() {
        assert_eq!(token_ms("120ms"), 120.0);
        assert_eq!(token_ms("30s"), 30_000.0);
        assert_eq!(token_ms("soon"), 0.0);
        assert_eq!(fetch_grace_ms(), 120.0, "F3: skipped under 120 ms");
        assert_eq!(rest_after_ms(), 30_000.0, "V-D23: rest after 30 s");
    }
}
