//! Circuits + seeded track generation (CC-SPELL-RACING Phase 2, spec F6 + D7).
//!
//! A track is a fixed-length list of words drawn from the player's `(language,
//! tier)`. Each word is a lap. Generation is **seeded and deterministic** — the same
//! `(lang, tier, circuit, seed)` yields the identical track on every device (F6: a
//! shared ghost's track reproduces exactly; acceptance tests #1/#2). Words are drawn
//! **without replacement**, so a track never repeats a word (D7); if a bank can't
//! supply enough unique words for a circuit, that circuit is simply **unavailable**
//! for that `(lang, tier)` — never padded with repeats.
//!
//! Self-contained deterministic PRNG (splitmix64) + Fisher–Yates, mirroring
//! `daily.rs` rather than depending on it, so `racing::` stays isolated (D2).

use crate::wordid;

/// Fixed-length race tracks (F6). A word is a lap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Circuit {
    Sprint,    // 10 words
    GrandPrix, // 20 words
    Endurance, // 40 words
}

impl Circuit {
    pub const ALL: [Circuit; 3] = [Circuit::Sprint, Circuit::GrandPrix, Circuit::Endurance];

    /// Number of laps (words) in this circuit.
    pub fn laps(self) -> usize {
        match self {
            Circuit::Sprint => 10,
            Circuit::GrandPrix => 20,
            Circuit::Endurance => 40,
        }
    }

    /// Stable identifier (registry/i18n key stem). Never free text.
    pub fn id(self) -> &'static str {
        match self {
            Circuit::Sprint => "sprint",
            Circuit::GrandPrix => "grand_prix",
            Circuit::Endurance => "endurance",
        }
    }

    /// Parse back from `id()` (e.g. a DOM `data-circuit` attribute).
    pub fn from_id(s: &str) -> Option<Circuit> {
        Circuit::ALL.iter().copied().find(|c| c.id() == s)
    }
}

// ---- deterministic PRNG (splitmix64) + FNV seed — same primitives as daily.rs ----

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Deterministic seed for one track. Includes the list hash so a shipped word-list
/// update reshuffles tracks exactly once (like daily's `pool_hash`) rather than
/// silently drifting which words a given seed picks.
fn track_seed(lang: &str, tier: &str, circuit: Circuit, seed: u64) -> u64 {
    let lh = wordid::list_hash(lang, tier);
    fnv1a(&format!("race-track-v1|{lang}|{tier}|{}|{seed}|{lh}", circuit.id()))
}

/// Draw `n` distinct indices from `0..pool_len`, seeded. `None` if the pool can't
/// supply `n` unique items (D7: never pad). Partial Fisher–Yates — O(n).
fn draw_indices(pool_len: usize, n: usize, seed: u64) -> Option<Vec<usize>> {
    if pool_len < n {
        return None;
    }
    let mut idx: Vec<usize> = (0..pool_len).collect();
    let mut rng = seed;
    for i in 0..n {
        // pick from the not-yet-drawn tail [i, pool_len)
        let j = i + (next_u64(&mut rng) as usize) % (pool_len - i);
        idx.swap(i, j);
    }
    idx.truncate(n);
    Some(idx)
}

/// Which circuits this `(lang, tier)` can run. A small bank caps at the largest
/// circuit that fits (Sprint ⊂ Grand Prix ⊂ Endurance by size), surfacing the cap
/// rather than padding (D7).
pub fn circuits_for_size(pool_len: usize) -> Vec<Circuit> {
    Circuit::ALL.iter().copied().filter(|c| c.laps() <= pool_len).collect()
}

/// Circuits available for the current production bank of `(lang, tier)`.
pub fn available(lang: &str, tier: &str) -> Vec<Circuit> {
    circuits_for_size(crate::words::tier_for(lang, tier).len())
}

/// Generate a track: the ordered words to race, or `None` if `(lang, tier)` can't
/// supply enough unique words for `circuit` (the circuit is unavailable there).
/// Deterministic in `(lang, tier, circuit, seed)`.
pub fn generate(lang: &str, tier: &str, circuit: Circuit, seed: u64) -> Option<Vec<&'static str>> {
    let pool = crate::words::tier_for(lang, tier);
    let s = track_seed(lang, tier, circuit, seed);
    draw_indices(pool.len(), circuit.laps(), s).map(|idx| idx.iter().map(|&i| pool[i]).collect())
}

// ---- sector (per-lap) timing vs a ghost -------------------------------------
// The display is Phase 5; these are the pure comparison primitives.

/// A ghost lap's duration (finish − start) in ms.
pub fn ghost_lap_ms(ev: &crate::racing::format::WordEvent) -> u32 {
    ev.finish_ms.saturating_sub(ev.start_ms)
}

/// Signed sector delta for one lap: `live − ghost` ms. Negative = the live racer is
/// faster on this lap (ahead), positive = slower (behind).
pub fn sector_delta_ms(live_lap_ms: u32, ghost_lap_ms: u32) -> i64 {
    live_lap_ms as i64 - ghost_lap_ms as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circuit_laps_are_fixed() {
        assert_eq!(Circuit::Sprint.laps(), 10);
        assert_eq!(Circuit::GrandPrix.laps(), 20);
        assert_eq!(Circuit::Endurance.laps(), 40);
    }

    #[test]
    fn draw_is_deterministic_and_unique() {
        let a = draw_indices(100, 40, 12345).unwrap();
        let b = draw_indices(100, 40, 12345).unwrap();
        assert_eq!(a, b, "same seed -> same draw");
        let c = draw_indices(100, 40, 999).unwrap();
        assert_ne!(a, c, "different seed -> different draw (overwhelmingly)");
        // D7: no duplicate indices.
        let mut sorted = a.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), a.len(), "no duplicate laps");
    }

    /// Acceptance #10: a small bank caps at Grand Prix (needs 20) and cannot run
    /// Endurance (needs 40) — surfaced, never padded.
    #[test]
    fn small_bank_caps_at_grand_prix() {
        let tiny = 25; // 25 unique words: fits Sprint(10) + Grand Prix(20), not Endurance(40)
        assert_eq!(circuits_for_size(tiny), vec![Circuit::Sprint, Circuit::GrandPrix]);
        assert!(draw_indices(tiny, 40, 1).is_none(), "Endurance impossible in a 25-word bank");
        assert!(draw_indices(tiny, 20, 1).is_some(), "Grand Prix still fits");

        // Even smaller: only Sprint.
        assert_eq!(circuits_for_size(10), vec![Circuit::Sprint]);
        // Too small for anything.
        assert_eq!(circuits_for_size(9), Vec::<Circuit>::new());
    }

    #[test]
    fn generate_on_a_real_bank_is_unique_and_deterministic() {
        // en/easy is a real production bank (≥ 40 words), so all circuits run.
        assert_eq!(available("en", "easy"), Circuit::ALL.to_vec());
        let t1 = generate("en", "easy", Circuit::Sprint, 7).unwrap();
        let t2 = generate("en", "easy", Circuit::Sprint, 7).unwrap();
        assert_eq!(t1, t2, "same inputs -> identical track");
        assert_eq!(t1.len(), 10);
        // D7 on real words.
        let mut u = t1.clone();
        u.sort_unstable();
        u.dedup();
        assert_eq!(u.len(), 10, "no repeated word in a track");
        // every word is a real member of the bank.
        let pool = crate::words::tier_for("en", "easy");
        for w in &t1 {
            assert!(pool.contains(w));
        }
    }

    #[test]
    fn empty_pool_offers_no_circuits() {
        // Config-independent: a zero-word pool supports nothing and never pads.
        assert_eq!(circuits_for_size(0), Vec::<Circuit>::new());
        assert!(draw_indices(0, 10, 1).is_none());
    }

    // Production only: under audit_preview, Arabic serves a DRAFT bank (non-empty),
    // so this "no production bank" property is exactly a not-audit_preview one.
    #[cfg(not(feature = "audit_preview"))]
    #[test]
    fn gated_language_with_no_production_bank_offers_no_circuits() {
        // ar is registered but rtl-gated: tier_for is empty, so generate returns None
        // (circuit unavailable) — never a padded or silently-English track.
        assert_eq!(available("ar", "easy"), Vec::<Circuit>::new());
        assert!(generate("ar", "easy", Circuit::Sprint, 1).is_none());
    }

    #[test]
    fn sector_delta_signs() {
        assert_eq!(sector_delta_ms(900, 1000), -100); // faster -> ahead
        assert_eq!(sector_delta_ms(1200, 1000), 200); // slower -> behind
    }

    /// D7 across the whole lineup: for EVERY registered language × tier × available
    /// circuit, a generated track repeats no word. Belt-and-suspenders over the
    /// draw-without-replacement guarantee — proves "no repeats in any language".
    #[test]
    fn no_track_repeats_a_word_in_any_language() {
        use crate::consts::{BUILTIN_LANGS, TIER_ORDER};
        let mut checked = 0;
        for (lang, _, _, _) in BUILTIN_LANGS {
            for tier in TIER_ORDER {
                for circuit in available(lang, tier) {
                    // a few seeds, so it's not a single-draw fluke
                    for seed in [1u64, 42, 9999] {
                        let track = generate(lang, tier, circuit, seed)
                            .expect("available circuit generates a track");
                        assert_eq!(track.len(), circuit.laps());
                        let mut u = track.clone();
                        u.sort_unstable();
                        u.dedup();
                        assert_eq!(
                            u.len(),
                            track.len(),
                            "{lang}/{tier}/{} seed {seed} repeated a word",
                            circuit.id()
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 0, "expected to check at least one language");
    }
}
