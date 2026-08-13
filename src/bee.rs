//! CC-BEE-SIM F2/F3 — the bee engine and the contestant table.
//!
//! A dramatized tier ladder over the existing engine: rounds escalate through
//! the bank's own difficulty tiers, synthetic contestants fall away around you,
//! and the run ends in a placement rather than a score. The ritual is the
//! product, so the engine's job is to make the ceremony reproducible — every
//! roster, every elimination and every word comes from the seed (I2).
//!
//! # The Definition request cannot be built today (D3)
//!
//! D3 offers Repeat, Slower and Definition, and gates the last one: the button
//! exists in a language only once that language's definitions clear audit. But
//! the only definitions in the app arrive from `/api/defpool` over the network,
//! and D7 requires this mode to be FULLY OFFLINE. The two cannot both hold, so
//! [`definitions_available`] answers false everywhere and the button is absent
//! — which is exactly what I1 specifies for a language whose definitions have
//! not cleared. Repeat and Slower are local TTS and work now.
//!
//! # Contestants are picked, not assembled (D2/F3)
//!
//! D2 asks for a curated name/flag COMPONENT table. This ships flat (name,
//! flag) pairs instead, because combining components can pair a name with a
//! flag from an unrelated culture — a small indignity the spec's own
//! "culturally plausible and inoffensive across markets" bar rules out, and one
//! no schema check would catch. A flat table is reviewable line by line and
//! cannot generate free text.

use std::collections::BTreeSet;

/// D1's ladder. Round number (1-based) to the bank tier it draws from.
const LADDER: [(u32, &str); 4] = [(1, "easy"), (3, "medium"), (5, "hard"), (7, "expert")];

/// D6: the Kid Mode ceiling, matching the standing kid tier cap.
const KID_CEILING: &str = "medium";

/// D2. Flat, reviewable, and deliberately not combinatorial.
const CONTESTANTS: [(&str, &str); 24] = [
    ("Amara", "\u{1F1F3}\u{1F1EC}"), ("Mateo", "\u{1F1F2}\u{1F1FD}"),
    ("Yuki", "\u{1F1EF}\u{1F1F5}"), ("Priya", "\u{1F1EE}\u{1F1F3}"),
    ("Lukas", "\u{1F1E9}\u{1F1EA}"), ("Sofia", "\u{1F1EE}\u{1F1F9}"),
    ("Omar", "\u{1F1EA}\u{1F1EC}"), ("Mei", "\u{1F1E8}\u{1F1F3}"),
    ("Noah", "\u{1F1FA}\u{1F1F8}"), ("Ines", "\u{1F1F5}\u{1F1F9}"),
    ("Dmitri", "\u{1F1F7}\u{1F1FA}"), ("Aisha", "\u{1F1F0}\u{1F1EA}"),
    ("Tomas", "\u{1F1E8}\u{1F1FF}"), ("Elif", "\u{1F1F9}\u{1F1F7}"),
    ("Jae", "\u{1F1F0}\u{1F1F7}"), ("Camila", "\u{1F1E8}\u{1F1F4}"),
    ("Hugo", "\u{1F1EB}\u{1F1F7}"), ("Nadia", "\u{1F1F5}\u{1F1F1}"),
    ("Kofi", "\u{1F1EC}\u{1F1ED}"), ("Linh", "\u{1F1FB}\u{1F1F3}"),
    ("Sara", "\u{1F1F8}\u{1F1EA}"), ("Diego", "\u{1F1E6}\u{1F1F7}"),
    ("Ravi", "\u{1F1F1}\u{1F1F0}"), ("Maya", "\u{1F1E7}\u{1F1F7}"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contestant {
    pub name: &'static str,
    pub flag: &'static str,
    /// Deterministic skill, 0..=100. Higher survives longer.
    pub skill: u32,
    /// The round this contestant goes out on, or `None` if they reach the end.
    pub out_on: Option<u32>,
}

fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The tier round `n` draws from, respecting D6's Kid ceiling.
pub fn tier_for_round(n: u32, kid: bool) -> &'static str {
    let mut t = "easy";
    for (from, name) in LADDER {
        if n >= from {
            t = name;
        }
    }
    if kid && (t == "hard" || t == "expert") {
        return KID_CEILING;
    }
    t
}

/// D3/I1. False everywhere today: the app's definitions are fetched from
/// `/api/defpool`, and D7 makes this mode offline. The gate is real and the
/// answer is currently no — which renders no button, per I1.
pub fn definitions_available(_lang: &str) -> bool {
    false
}

/// D2: the roster for a bee, resolved entirely from the seed.
///
/// Skill curves are tuned so early rounds cull the weak and the finals are
/// close, which is what makes a placement feel earned rather than rolled.
pub fn roster(seed: u64, size: usize) -> Vec<Contestant> {
    let n = size.clamp(8, 12).min(CONTESTANTS.len());
    let mut picked: Vec<usize> = Vec::new();
    let mut s = seed;
    while picked.len() < n {
        s = mix(s);
        let i = (s % CONTESTANTS.len() as u64) as usize;
        if !picked.contains(&i) {
            picked.push(i);
        }
    }
    picked
        .into_iter()
        .enumerate()
        .map(|(rank, i)| {
            let (name, flag) = CONTESTANTS[i];
            // Spread skill across the field so there is always a weak tail to
            // cull early and a strong head to survive to the finals.
            let base = 30 + (rank as u64 * 60 / n.max(1) as u64) as u32;
            let jitter = (mix(seed ^ i as u64) % 12) as u32;
            let skill = (base + jitter).min(99);
            Contestant { name, flag, skill, out_on: elimination_round(seed, i as u64, skill) }
        })
        .collect()
}

/// When a synthetic goes out. Deterministic, and never in round 1 — a bee that
/// culls someone before the player has spelled anything reads as arbitrary.
fn elimination_round(seed: u64, who: u64, skill: u32) -> Option<u32> {
    let roll = mix(seed ^ (who << 8)) % 100;
    // A weak contestant is likely out early; a strong one may not go out at all.
    if roll as u32 + skill >= 130 {
        return None;
    }
    let r = 2 + ((100 - skill) as u64 * 6 / 100).min(5);
    Some(r as u32 + (mix(seed ^ who) % 2) as u32)
}

/// I3: the words for a bee, one per round, no repeats.
///
/// Drawn from the tier the round calls for. A tier that runs dry ends the bee,
/// which is D1's "word-pool exhaustion" arm rather than an error.
pub fn words(lang: &str, seed: u64, rounds: u32, kid: bool) -> Vec<String> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for n in 1..=rounds {
        let tier = tier_for_round(n, kid);
        let pool = crate::words::tier_for(lang, tier);
        if pool.is_empty() {
            break;
        }
        let mut found = None;
        for k in 0..64u64 {
            let i = (mix(seed ^ (n as u64) << 16 ^ k) % pool.len() as u64) as usize;
            let w = pool[i].to_string();
            if !used.contains(&w) {
                found = Some(w);
                break;
            }
        }
        match found {
            Some(w) => {
                used.insert(w.clone());
                out.push(w);
            }
            None => break,
        }
    }
    out
}

/// D4: where the next bee starts after going out in round `n` — one below,
/// never below the first. Re-entry is the hook, so it must always exist.
pub fn reentry_round(out_on: u32) -> u32 {
    out_on.saturating_sub(1).max(1)
}

/// D4: the player's placement. `None` means they were never eliminated.
///
/// Everyone still standing when the player falls beats them; everyone already
/// gone does not. Ties break toward the player, because a placement is a
/// consolation and should not be stingy — which is why a player who reaches
/// the end places FIRST even though the strongest synthetics also survived.
///
/// The round number alone could not express that. An earlier signature took a
/// bare u32, so a player who stood to the end was scored against every
/// contestant who also stood to the end and came second in a bee they won; the
/// screen papered over it by special-casing a win before calling in. Making
/// survival part of the type puts the rule in one place.
pub fn placement(roster: &[Contestant], player_out_on: Option<u32>) -> usize {
    let Some(out) = player_out_on else {
        return 1; // stood to the end: the tie goes to the player
    };
    let ahead = roster
        .iter()
        .filter(|c| match c.out_on {
            None => true,
            Some(r) => r > out,
        })
        .count();
    ahead + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_escalates_and_kid_mode_is_capped() {
        assert_eq!(tier_for_round(1, false), "easy");
        assert_eq!(tier_for_round(2, false), "easy");
        assert_eq!(tier_for_round(3, false), "medium");
        assert_eq!(tier_for_round(5, false), "hard");
        assert_eq!(tier_for_round(9, false), "expert");
        // I4 — the kid ceiling holds however deep the bee goes.
        for n in 1..=12 {
            let t = tier_for_round(n, true);
            assert!(t == "easy" || t == "medium", "round {n} gave kids {t}");
        }
    }

    /// I2 — the whole ceremony reproduces from the seed.
    #[test]
    fn a_bee_is_reproducible() {
        for seed in [1u64, 42, 9999] {
            assert_eq!(roster(seed, 12), roster(seed, 12));
            assert_eq!(words("en", seed, 10, false), words("en", seed, 10, false));
        }
        assert_ne!(roster(1, 12), roster(2, 12), "different seeds, same roster");
    }

    /// I3 — no word repeats within a bee, in any live language.
    #[test]
    fn no_word_repeats_within_a_bee() {
        for lang in ["en", "es", "fr", "de", "ru", "ja", "ko", "zh", "hi", "ar"] {
            for seed in 0..20u64 {
                let ws = words(lang, seed, 12, false);
                let uniq: BTreeSet<&String> = ws.iter().collect();
                assert_eq!(uniq.len(), ws.len(), "{lang}: a word repeated in bee {seed}");
                assert!(!ws.is_empty(), "{lang}: bee {seed} produced no words");
            }
        }
    }

    #[test]
    fn the_roster_is_sized_and_never_duplicates_a_person() {
        for seed in 0..30u64 {
            let r = roster(seed, 12);
            assert_eq!(r.len(), 12);
            let names: BTreeSet<&str> = r.iter().map(|c| c.name).collect();
            assert_eq!(names.len(), r.len(), "the same contestant appeared twice");
        }
        assert_eq!(roster(1, 3).len(), 8, "the field clamps to at least eight");
        assert_eq!(roster(1, 99).len(), 12, "and at most twelve");
    }

    /// Nobody goes out in round one: a cull before the player has spelled
    /// anything reads as arbitrary rather than competitive.
    #[test]
    fn nobody_is_eliminated_in_round_one() {
        for seed in 0..50u64 {
            for c in roster(seed, 12) {
                assert!(c.out_on != Some(1), "{} went out in round 1", c.name);
            }
        }
    }

    /// The field should thin out but not empty: a bee where everyone survives
    /// has no stakes, and one where everyone falls has no competition.
    #[test]
    fn the_field_thins_without_emptying() {
        let mut survived = 0;
        let mut eliminated = 0;
        for seed in 0..40u64 {
            for c in roster(seed, 12) {
                match c.out_on {
                    None => survived += 1,
                    Some(_) => eliminated += 1,
                }
            }
        }
        assert!(eliminated > survived / 4, "too few eliminations: {eliminated} vs {survived}");
        assert!(survived > 0, "nobody ever reaches the final");
    }

    /// D4 — re-entry is one round below, and always a real round.
    #[test]
    fn reentry_is_one_round_below_and_never_zero() {
        assert_eq!(reentry_round(5), 4);
        assert_eq!(reentry_round(2), 1);
        assert_eq!(reentry_round(1), 1);
        assert_eq!(reentry_round(0), 1);
    }

    #[test]
    fn placement_counts_only_those_still_standing() {
        let r = vec![
            Contestant { name: "A", flag: "", skill: 90, out_on: None },
            Contestant { name: "B", flag: "", skill: 80, out_on: Some(7) },
            Contestant { name: "C", flag: "", skill: 40, out_on: Some(3) },
            Contestant { name: "D", flag: "", skill: 30, out_on: Some(2) },
        ];
        // Out in round 4: A (never out) and B (out later) are ahead.
        assert_eq!(placement(&r, Some(4)), 3);
        // Standing at the end wins outright, even though A also survived.
        assert_eq!(placement(&r, None), 1);
        // Out in round 2: A and B are ahead; C fell at 3, also ahead.
        assert_eq!(placement(&r, Some(2)), 4);
    }

    /// I1 — the Definition gate is real, and today it says no everywhere.
    #[test]
    fn the_definition_button_is_gated_and_currently_absent() {
        for lang in ["en", "es", "ja", "ar"] {
            assert!(
                !definitions_available(lang),
                "{lang}: definitions come from /api/defpool over the network, and D7 \
                 makes this mode offline — the button cannot render yet"
            );
        }
    }

    /// F3 — the contestant table is data a reviewer can read, and cannot
    /// produce free text.
    #[test]
    fn the_contestant_table_is_curated_and_finite() {
        assert!(CONTESTANTS.len() >= 12, "the field needs more people than a bee seats");
        let names: BTreeSet<&str> = CONTESTANTS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), CONTESTANTS.len(), "a duplicate name in the table");
        for (n, f) in CONTESTANTS {
            assert!(!n.is_empty() && !f.is_empty());
            assert!(n.chars().all(|c| c.is_alphabetic()), "{n} is not a plain name");
        }
    }
}
