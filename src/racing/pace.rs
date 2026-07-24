//! Pace ghosts (CC-SPELL-RACING Phase 4, spec F3).
//!
//! A pace ghost is a **synthetic opponent** — a `RaceGhost` (Phase 1 format) generated
//! from a difficulty *band* over a given track, with no recorded play behind it. This
//! is what gives a brand-new garage (or a fresh install) something to race against
//! before the player has recorded any ghost of their own.
//!
//! **Determinism (acceptance #2).** Generation is pure over `(band, track, seed)`:
//! same band + same track seed ⇒ **byte-identical** ghost on every device, so a pace
//! opponent is reproducible in CI and identical for everyone. The per-lap "human"
//! jitter is a hash of `(band.id, seed, lap_index)`, not a wall clock or RNG state.
//!
//! **Bands are language-independent** (`config/pace-bands.json`, one entry per band —
//! NOT per language). A lap's target time scales with the *word length* (chars), so
//! the same band is proportionally harder on longer words in any script.
//!
//! **Champion is GATED (gate G-C / decision D6).** Its parameters — and the whole
//! calibration method — are PROPOSED, not approved; `gated: true` keeps it out of
//! `active_bands()` (and therefore out of the shipped opponent list) until Eric signs
//! off with real tester data. Bronze–Platinum ship; Champion is generated only when a
//! caller explicitly asks for a gated band (tests / a future preview).
//!
//! Isolation (D2): no Climb/shield references.

use serde::Deserialize;

use crate::racing::format::{Identity, RaceGhost, WordEvent, SCHEMA_VERSION};
use crate::wordid;

/// One difficulty band. Language-independent; a lap's target time is
/// `base_ms + per_char_ms * chars`, then nudged ±`jitter_pct`% deterministically.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Band {
    /// Stable identifier + i18n key stem (`racing.pace.<id>`). Never free text.
    pub id: String,
    /// Fixed per-word overhead (ms).
    pub base_ms: u32,
    /// Added ms per character of the word.
    pub per_char_ms: u32,
    /// Deterministic per-lap jitter amplitude, percent of the lap's base time.
    pub jitter_pct: u32,
    /// Preset avatar/colour for this band's ghost (indices, no free text — D5).
    pub avatar_id: u8,
    pub color_id: u8,
    /// D6/G-C: a band whose parameters are not yet approved. Excluded from
    /// `active_bands()` so it never ships until sign-off.
    #[serde(default)]
    pub gated: bool,
}

const PACE_BANDS_JSON: &str = include_str!("../../config/pace-bands.json");

/// All bands as configured, including gated ones. Panics only if the bundled JSON is
/// malformed (a build-time authoring error, caught by tests).
pub fn bands() -> Vec<Band> {
    serde_json::from_str(PACE_BANDS_JSON).expect("bundled config/pace-bands.json must be valid JSON")
}

/// The bands that ship — gated bands (Champion, pending D6) excluded.
pub fn active_bands() -> Vec<Band> {
    bands().into_iter().filter(|b| !b.gated).collect()
}

/// Look up a band by id (active or gated).
pub fn band(id: &str) -> Option<Band> {
    bands().into_iter().find(|b| b.id == id)
}

// FNV-1a-64 — same primitive track.rs/wordid use, kept local for D2 isolation.
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// One lap's target duration (ms) for `band` on a word of `chars` characters, with
/// deterministic jitter keyed by `(band.id, seed, lap)`. Never below a 200ms floor.
fn lap_ms(band: &Band, chars: u32, seed: u64, lap: usize) -> u32 {
    let base = band.base_ms + band.per_char_ms * chars;
    // frac in [-1000, 1000] from a stable hash → jitter in ±jitter_pct% of base.
    let r = fnv1a(&format!("pace-v1|{}|{seed}|{lap}", band.id));
    let frac = (r % 2001) as i64 - 1000;
    let delta = base as i64 * band.jitter_pct as i64 * frac / 100_000;
    (base as i64 + delta).max(200) as u32
}

/// Generate a pace ghost for `band` racing `words` (the track, in order) of
/// `(language, tier)`. Laps run back-to-back from t=0; `keystrokes_ms` is empty (a
/// pace ghost is a time target, not a replayed keystream) and every lap is `correct`.
/// Deterministic in `(band, words, seed)` — same inputs ⇒ byte-identical ghost.
pub fn generate(
    band: &Band,
    words: &[&str],
    language: &str,
    tier: &str,
    word_list_hash: u64,
    seed: u64,
) -> RaceGhost {
    let mut clock: u32 = 0;
    let events = words
        .iter()
        .enumerate()
        .map(|(lap, w)| {
            let start = clock;
            let dur = lap_ms(band, w.chars().count() as u32, seed, lap);
            clock = start.saturating_add(dur);
            WordEvent {
                word_id: wordid::word_id(w),
                start_ms: start,
                finish_ms: clock,
                keystrokes_ms: Vec::new(),
                correct: true,
            }
        })
        .collect();
    RaceGhost {
        schema_version: SCHEMA_VERSION,
        language: language.to_string(),
        tier: tier.to_string(),
        word_list_hash,
        identity: Identity { avatar_id: band.avatar_id, color_id: band.color_id },
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::racing::format::{AVATAR_COUNT, COLOR_COUNT};

    #[test]
    fn config_parses_and_has_the_expected_bands() {
        let all = bands();
        let ids: Vec<&str> = all.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["bronze", "silver", "gold", "platinum", "champion"]);
        // Champion is the only gated band; the other four ship.
        assert_eq!(active_bands().iter().map(|b| b.id.clone()).collect::<Vec<_>>(),
                   vec!["bronze", "silver", "gold", "platinum"]);
        assert!(band("champion").unwrap().gated, "Champion stays gated until D6 sign-off");
        // Every band's preset identity is in range (D5).
        for b in &all {
            assert!(b.avatar_id < AVATAR_COUNT && b.color_id < COLOR_COUNT, "{} identity in range", b.id);
        }
    }

    #[test]
    fn faster_bands_finish_sooner() {
        // Monotonic difficulty: on the same word, each band down the list is quicker.
        let words = ["spelling"];
        let times: Vec<u32> = bands()
            .iter()
            .map(|b| garage_total(&generate(b, &words, "en", "easy", 0, 7)))
            .collect();
        for w in times.windows(2) {
            assert!(w[0] > w[1], "each band should be faster than the one before: {times:?}");
        }
    }

    /// Acceptance #2: same band + same track seed ⇒ byte-identical ghost.
    #[test]
    fn generation_is_byte_identical_for_same_band_and_seed() {
        let words = ["alpha", "bravo", "charlie", "delta"];
        let b = band("gold").unwrap();
        let g1 = generate(&b, &words, "en", "easy", 42, 7);
        let g2 = generate(&b, &words, "en", "easy", 42, 7);
        assert_eq!(
            serde_json::to_string(&g1).unwrap(),
            serde_json::to_string(&g2).unwrap(),
            "same (band, track, seed) must serialize byte-for-byte identically"
        );
        // A different seed almost surely differs (jitter changes).
        let g3 = generate(&b, &words, "en", "easy", 99, 7);
        assert_ne!(g1, g3, "a different track seed changes the pace ghost");
    }

    #[test]
    fn laps_are_contiguous_and_scale_with_word_length() {
        let b = band("silver").unwrap();
        let short = generate(&b, &["at"], "en", "easy", 0, 1);
        let long = generate(&b, &["extraordinary"], "en", "easy", 0, 1);
        assert!(garage_total(&long) > garage_total(&short), "longer word takes longer");

        // laps run back-to-back: each start == previous finish, first start == 0.
        let g = generate(&b, &["one", "two", "three"], "en", "easy", 0, 1);
        assert_eq!(g.events[0].start_ms, 0);
        for pair in g.events.windows(2) {
            assert_eq!(pair[1].start_ms, pair[0].finish_ms, "no gap between laps");
        }
        // pace ghosts carry no keystream and are all correct.
        assert!(g.events.iter().all(|e| e.keystrokes_ms.is_empty() && e.correct));
    }

    #[test]
    fn empty_track_yields_an_empty_ghost() {
        let b = band("bronze").unwrap();
        let g = generate(&b, &[], "en", "easy", 0, 1);
        assert!(g.events.is_empty());
    }

    // total_time_ms lives in garage; inline the same "last finish" here to avoid a dep.
    fn garage_total(g: &RaceGhost) -> u32 {
        g.events.last().map(|e| e.finish_ms).unwrap_or(0)
    }
}
