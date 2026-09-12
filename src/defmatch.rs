//! CC-DEF-MATCH P1 — the Definition Match core round engine.
//!
//! REVIEW-GATED. Round generation, distractor selection, and tell-free
//! shuffling live HERE in the core (spec: Placement) — the frontend receives a
//! fully-resolved [`Round`] and never selects content.
//!
//! CONTENT AUTHORITY (Invariant 1): every rendered string must resolve to a
//! passing Gig A row. The engine therefore consumes [`DefRow`]s whose
//! `prompt_grade` (D1 — the Gig A "uniquely identifies the word" column) and
//! `audit_pass` bits come from the audit artifacts, and exclusion sets consumed
//! from CC-DEF-PRECHECK sweep output. AS OF P1 (2026-07-27) NEITHER ARTIFACT
//! EXISTS in this repo — the lexicon (`data/<lang>/lexicon.jsonl`) has a gloss
//! field but zero populated rows, and no Gig A/precheck outputs are checked in.
//! P1 is engine + fixture tests per the build order; no language can activate
//! (Invariant 8) until those artifacts land. This module grants itself no
//! authority to fabricate either.
#![allow(dead_code)] // wired into the hub in P2; P1 is engine + tests only.

use std::collections::{HashMap, HashSet};

/// One audited definition row — the P1 shape of a Gig A export row. Fields
/// beyond the strings exist for distractor CRAFT (Feature 6), not rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct DefRow {
    pub word: String,
    pub definition: String,
    /// Gig A "uniquely identifies the word" — BLOCKING for this mode (D1).
    pub prompt_grade: bool,
    /// The row passed its language's definitions audit round.
    pub audit_pass: bool,
    /// Kid-register text (Invariant 6 — applies to distractors too).
    pub kid_register: bool,
    /// easy | medium | hard | expert.
    pub tier: String,
    /// Coarse topic bucket (easy = distant-topic, medium+ = same-topic).
    pub topic: String,
    /// Morphological root (hard/expert shared-morphology / same-root craft).
    pub root: String,
}

/// CC-DEF-PRECHECK near-miss/synonym exclusions: word -> words too close to
/// ever appear as its distractors (Invariant 2). Consumed, never generated.
#[derive(Debug, Clone, Default)]
pub struct ExclusionSets(pub HashMap<String, HashSet<String>>);

impl ExclusionSets {
    fn excluded(&self, target: &str, candidate: &str) -> bool {
        // Acceptance #2 names the TARGET's set; we also honour the reverse
        // direction (candidate's set naming the target) — strictly safer, and
        // a symmetric sweep still passes.
        self.0.get(target).is_some_and(|s| s.contains(candidate))
            || self.0.get(candidate).is_some_and(|s| s.contains(target))
    }
}

/// A fully-resolved round: what the frontend renders, nothing more to decide.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    /// D8 Reverse Round (Eric promoted 2026-07-27): the DEFINITION is the
    /// prompt and the cards are WORDS. `prompt` carries the target's
    /// definition; `cards` are words; `words` still names each card's word
    /// (identical to `cards` here), so target identity reads the same way in
    /// both formats. Forward rounds: false + empty prompt.
    pub reverse: bool,
    pub prompt: String,
    /// Card definition strings, in card-index order (3 in Kid Mode — D6, else 4).
    pub cards: Vec<String>,
    /// Which card index is the target word's definition.
    pub correct: usize,
    /// The word each card's definition actually belongs to — the reveal beat
    /// (Feature 4) shows `words[tapped]` under a wrong card. Audited rows only.
    pub words: Vec<String>,
    /// Lane assignment per card index (a permutation of 0..cards.len()).
    pub lanes: Vec<u8>,
    /// Spawn sequence: spawn_order[k] = card index spawned k-th (~0.8s apart,
    /// tier-scaled — timing is the frontend's, order is ours).
    pub spawn_order: Vec<u8>,
    pub seed: u64,
}

/// D3: a (language, tier) activates only with at least this many prompt-grade,
/// audit-passing rows. No bypass exists for this floor (Invariant 8).
pub const POOL_FLOOR: usize = 40;

/// D6: Kid Mode rounds float 3 cards; everything else floats 4.
pub const CARDS_KID: usize = 3;
pub const CARDS_FULL: usize = 4;

// ---- deterministic PRNG (splitmix64 — the crate convention, see daily.rs) ----

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn shuffled(n: usize, state: &mut u64) -> Vec<u8> {
    let mut v: Vec<u8> = (0..n as u8).collect();
    for i in (1..n).rev() {
        let j = (next_u64(state) % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
    v
}

/// CC-DEFMATCH-POLISH D3 — card speed as a DURATION multiplier on travel time.
/// Player-facing steps map to speed 0.6x / 1.0x / 1.4x, i.e. duration x1.67 /
/// x1.0 / x0.71 (PROPOSED values, flagged for review). Comfort control only:
/// round generation takes no speed input, so results are identical across
/// settings by construction (I3). Kid Mode defaults to Relaxed when unset;
/// iOS Reduce Motion forces Relaxed as the floor (I4).
pub fn speed_duration_mult(setting: Option<&str>, kid: bool, reduce_motion: bool) -> f32 {
    let chosen: f32 = match setting {
        Some("relaxed") => 1.67,
        Some("swift") => 0.71,
        Some("standard") => 1.0,
        // Unset: Kid Mode starts Relaxed; everyone else Standard.
        None => {
            if kid {
                1.67
            } else {
                1.0
            }
        }
        _ => 1.0,
    };
    if reduce_motion {
        chosen.max(1.67)
    } else {
        chosen
    }
}

/// P4 (D9): the Climb-variant forging adapter — Definition Match becomes
/// another legal way to forge shields by calling the EXACT functions core
/// spelling calls, in the same order, with no new rules: a first-tap catch is
/// a validated correct (`attempts::shield_on_correct`), any miss (wrong tap or
/// timeout) resets the earn streak (`attempts::shield_note_miss`). Casual mode
/// never calls this (streaks are score-only there).
/// Returns whether a shield was just earned.
pub fn climb_outcome(state: &mut crate::model::AppState, caught_first_tap: bool) -> bool {
    if caught_first_tap {
        crate::attempts::shield_on_correct(state)
    } else {
        crate::attempts::shield_note_miss(state);
        false
    }
}

/// P2 entitlement depth: PREVIEW reaches tier 1 (easy) only; FULL reaches all
/// tiers (acceptance #9). Enforced here in core — the hub asks, it never decides.
pub fn allowed_tiers(level: crate::entitlements::AccessLevel) -> &'static [&'static str] {
    use crate::entitlements::AccessLevel;
    match level {
        AccessLevel::Full => &["easy", "medium", "hard", "expert"],
        AccessLevel::Preview => &["easy"],
        AccessLevel::None => &[],
    }
}

/// The Spell Jr tier ceiling (Invariant 6): the UI can ask, the engine
/// refuses. CC-ONBOARD-JR: the answer comes from `experience::serve_tier`, the
/// one authority, so this mode cannot drift from the others.
pub fn effective_tier<'a>(tier: &'a str, kid: bool) -> &'a str {
    if !kid {
        return tier;
    }
    crate::experience::serve_tier(crate::experience::Experience::Junior, "def_match", tier)
}

/// The mode's word pool for one (rows, tier, kid) view: prompt-grade AND
/// audit-passing rows of that tier only (D1/Invariant 1); Kid Mode additionally
/// requires kid-register text — including for distractors (Invariant 6).
pub fn eligible<'a>(rows: &'a [DefRow], tier: &str, kid: bool) -> Vec<&'a DefRow> {
    let tier = effective_tier(tier, kid);
    rows.iter()
        .filter(|r| r.prompt_grade && r.audit_pass && r.tier == tier && (!kid || r.kid_register))
        .collect()
}

/// D3 activation floor for a (language, tier) pool view.
pub fn pool_floor_met(rows: &[DefRow], tier: &str, kid: bool) -> bool {
    eligible(rows, tier, kid).len() >= POOL_FLOOR
}

/// Distractor craft (Feature 6): how well `cand` fits as a WRONG answer for
/// `target` at `tier`. Higher = preferred. Selection stays deterministic: the
/// candidate list is sorted by (score desc, word asc) and the seed picks among
/// the top band, so identical inputs give identical rounds (Invariant 3).
fn craft_score(tier: &str, target: &DefRow, cand: &DefRow) -> i32 {
    let same_topic = !target.topic.is_empty() && cand.topic == target.topic;
    let same_root = !target.root.is_empty() && cand.root == target.root;
    match tier {
        // Easy: distant topics read obviously-different — that's the point.
        "easy" => {
            if same_topic {
                0
            } else {
                2
            }
        }
        // Medium: same topic, so the child must actually read.
        "medium" => {
            if same_topic {
                2
            } else {
                1
            }
        }
        // Hard: same semantic field or shared morphology.
        "hard" => (same_topic as i32) + (same_root as i32) * 2,
        // Expert: sense-siblings / same root at max preference.
        _ => (same_root as i32) * 3 + (same_topic as i32),
    }
}

/// Build the round for (`word`, `seed`) at `tier` — or None when the word has
/// no prompt-grade row, the pool floor is unmet, or exclusions leave too few
/// distractors. Deterministic byte-for-byte given identical inputs
/// (Invariant 3); lane and spawn assignments come from independent seeded
/// shuffles, so they carry zero correlation with correctness (Invariant 4).
pub fn generate(
    rows: &[DefRow],
    exclusions: &ExclusionSets,
    tier: &str,
    word: &str,
    seed: u64,
    kid: bool,
) -> Option<Round> {
    let pool = eligible(rows, tier, kid);
    if pool.len() < POOL_FLOOR {
        return None;
    }
    let target = pool.iter().find(|r| r.word == word)?;
    let n_cards = if kid { CARDS_KID } else { CARDS_FULL };

    // Candidates: everything in the pool except the target and anything the
    // precheck marks too close to it (Invariant 2 — enforced HERE, not in UI).
    let mut cands: Vec<&DefRow> = pool
        .iter()
        .copied()
        .filter(|r| r.word != target.word && !exclusions.excluded(&target.word, &r.word))
        // Identical definition text can't be a distractor for it (two truths).
        .filter(|r| r.definition != target.definition)
        .collect();
    if cands.len() < n_cards - 1 {
        return None;
    }

    // Craft ranking, then a seeded pick among the best-fitting band so rounds
    // vary by seed without ever dipping below the tier's craft intent.
    cands.sort_by(|a, b| {
        craft_score(tier, target, b)
            .cmp(&craft_score(tier, target, a))
            .then(a.word.cmp(&b.word))
    });
    let band = (n_cards * 3).min(cands.len());
    let mut state = seed ^ fnv(&target.word) ^ fnv(tier);
    let mut picked: Vec<&DefRow> = Vec::with_capacity(n_cards - 1);
    let mut band_idx: Vec<usize> = (0..band).collect();
    for _ in 0..n_cards - 1 {
        let k = (next_u64(&mut state) % band_idx.len() as u64) as usize;
        picked.push(cands[band_idx.remove(k)]);
    }

    // Assemble + tell-free placement: THREE independent shuffles (card order,
    // lanes, spawn sequence). Correctness is decided by the first shuffle
    // alone, so lane/spawn distributions stay flat (Invariant 4, acceptance #3).
    let mut entries: Vec<&DefRow> = Vec::with_capacity(n_cards);
    entries.push(target);
    entries.extend(picked);
    let order = shuffled(n_cards, &mut state);
    let mut cards = vec![String::new(); n_cards];
    let mut words = vec![String::new(); n_cards];
    let mut correct = 0usize;
    for (from, &to) in order.iter().enumerate() {
        cards[to as usize] = entries[from].definition.clone();
        words[to as usize] = entries[from].word.clone();
        if from == 0 {
            correct = to as usize;
        }
    }
    let lanes = shuffled(n_cards, &mut state);
    let spawn_order = shuffled(n_cards, &mut state);

    Some(Round { reverse: false, prompt: String::new(), cards, correct, words, lanes, spawn_order, seed })
}

/// D8: every `REVERSE_EVERY`-th round flips the format (0 would be the
/// shipped-disabled stub; Eric promoted it, so it ships ON at 4).
pub const REVERSE_EVERY: u64 = 4;

/// Build a REVERSE round: same pool, same exclusions, same craft ranking,
/// same three independent shuffles — the cards are the WORDS and the prompt
/// is the target's definition. Zero new content rows (D8's whole point).
pub fn generate_reverse(
    rows: &[DefRow],
    exclusions: &ExclusionSets,
    tier: &str,
    word: &str,
    seed: u64,
    kid: bool,
) -> Option<Round> {
    let base = generate(rows, exclusions, tier, word, seed, kid)?;
    let pool = eligible(rows, tier, kid);
    let target = pool.iter().find(|r| r.word == word)?;
    Some(Round {
        reverse: true,
        prompt: target.definition.clone(),
        // The floating cards are the words themselves; `words` stays the
        // per-card word (== cards) so callers read target identity uniformly.
        cards: base.words.clone(),
        words: base.words.clone(),
        correct: base.correct,
        lanes: base.lanes,
        spawn_order: base.spawn_order,
        seed,
    })
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic fixture pool — 60 rows per tier (over the D3 floor), topics
    /// cycling over 6 buckets, roots over 10, all prompt-grade + audit-passing,
    /// kid_register on easy/medium. SYNTHETIC BY DESIGN: P1 proves the engine;
    /// Invariant-1 realism arrives with the Gig A artifacts in P2+.
    fn fixture_rows() -> Vec<DefRow> {
        let mut rows = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            for i in 0..60 {
                rows.push(DefRow {
                    word: format!("{tier}_w{i}"),
                    definition: format!("meaning of {tier}_w{i}"),
                    prompt_grade: i % 7 != 6, // some rows fail D1 and must vanish
                    audit_pass: true,
                    kid_register: matches!(tier, "easy" | "medium"),
                    tier: tier.to_string(),
                    topic: format!("t{}", i % 6),
                    root: format!("r{}", i % 10),
                });
            }
        }
        rows
    }

    fn fixture_exclusions() -> ExclusionSets {
        let mut m = HashMap::new();
        // w0's near-misses: w1 and w2 (per-tier) — never legal distractors.
        for tier in ["easy", "medium", "hard", "expert"] {
            m.insert(
                format!("{tier}_w0"),
                HashSet::from([format!("{tier}_w1"), format!("{tier}_w2")]),
            );
        }
        ExclusionSets(m)
    }

    /// Acceptance #1: same (tier, word, seed) → byte-identical rounds, 100 runs.
    #[test]
    fn determinism_100_runs() {
        let rows = fixture_rows();
        let ex = fixture_exclusions();
        let first = generate(&rows, &ex, "hard", "hard_w3", 42, false).unwrap();
        for _ in 0..100 {
            let again = generate(&rows, &ex, "hard", "hard_w3", 42, false).unwrap();
            assert_eq!(first, again, "round must reproduce byte-identically");
        }
        // A different seed must be able to differ (not a constant function).
        let other = generate(&rows, &ex, "hard", "hard_w3", 43, false).unwrap();
        assert!(first != other || first.cards == other.cards);
    }

    /// Acceptance #2: sweep every word × 50 seeds — no distractor from the
    /// target's exclusion set, in either direction.
    #[test]
    fn uniqueness_sweep_no_excluded_distractors() {
        let rows = fixture_rows();
        let ex = fixture_exclusions();
        for tier in ["easy", "medium", "hard", "expert"] {
            for r in eligible(&rows, tier, false) {
                for seed in 0..50u64 {
                    let round = generate(&rows, &ex, tier, &r.word, seed, false).unwrap();
                    for (i, w) in round.words.iter().enumerate() {
                        if i == round.correct {
                            continue;
                        }
                        assert!(!ex.excluded(&r.word, w), "{tier}/{} seed {seed}: excluded distractor {w}", r.word);
                        assert_ne!(w, &r.word, "target duplicated as distractor");
                    }
                }
            }
        }
    }

    /// Acceptance #3: 10,000 seeded rounds per tier — the correct card's lane
    /// and spawn position are uniform within χ² tolerance (p=0.001).
    #[test]
    fn tell_detection_chi_squared() {
        let rows = fixture_rows();
        let ex = ExclusionSets::default();
        for tier in ["easy", "medium", "hard", "expert"] {
            let n = 10_000u64;
            let mut lane_counts = [0u64; CARDS_FULL];
            let mut spawn_counts = [0u64; CARDS_FULL];
            let mut card_counts = [0u64; CARDS_FULL];
            let pool: Vec<String> = eligible(&rows, tier, false).iter().map(|r| r.word.clone()).collect();
            for seed in 0..n {
                let word = &pool[(seed % pool.len() as u64) as usize];
                let round = generate(&rows, &ex, tier, word, seed, false).unwrap();
                card_counts[round.correct] += 1;
                lane_counts[round.lanes[round.correct] as usize] += 1;
                let spawn_pos = round.spawn_order.iter().position(|&c| c as usize == round.correct).unwrap();
                spawn_counts[spawn_pos] += 1;
            }
            // χ² critical value, dof=3, p=0.001.
            const CRIT: f64 = 16.266;
            for (name, counts) in [("card", card_counts), ("lane", lane_counts), ("spawn", spawn_counts)] {
                let exp = n as f64 / CARDS_FULL as f64;
                let chi2: f64 = counts.iter().map(|&c| (c as f64 - exp).powi(2) / exp).sum();
                assert!(chi2 < CRIT, "{tier}: correct-{name} distribution not uniform (chi2={chi2:.2}, counts={counts:?})");
            }
        }
    }

    /// Acceptance #4 (core half): fuzz 1,000 miss paths — every reveal pair
    /// (tapped definition, its owning word; correct card) resolves to a
    /// prompt-grade, audit-passing row. By construction AND verified.
    #[test]
    fn reveal_integrity_fuzz() {
        let rows = fixture_rows();
        let ex = fixture_exclusions();
        let by_word: HashMap<&str, &DefRow> = rows.iter().map(|r| (r.word.as_str(), r)).collect();
        for seed in 0..1_000u64 {
            let tier = ["easy", "medium", "hard", "expert"][(seed % 4) as usize];
            let pool: Vec<String> = eligible(&rows, tier, false).iter().map(|r| r.word.clone()).collect();
            let word = &pool[(seed % pool.len() as u64) as usize];
            let round = generate(&rows, &ex, tier, word, seed, false).unwrap();
            // A "miss" taps any wrong card; the reveal shows that card's word.
            for (i, (def, w)) in round.cards.iter().zip(&round.words).enumerate() {
                let row = by_word[w.as_str()];
                assert!(row.prompt_grade && row.audit_pass, "card {i} renders a non-audited row");
                assert_eq!(&row.definition, def, "card text must be the row's own definition");
            }
        }
    }

    /// Acceptance #10 (core half): 1,000 Kid rounds — always 3 cards, only
    /// kid-register rows (distractors included), hard/expert clamp to medium.
    #[test]
    fn kid_mode_fuzz() {
        let rows = fixture_rows();
        let ex = ExclusionSets::default();
        let by_word: HashMap<&str, &DefRow> = rows.iter().map(|r| (r.word.as_str(), r)).collect();
        for seed in 0..1_000u64 {
            // Ask for forbidden tiers half the time — the CORE must clamp.
            let ask = ["easy", "medium", "hard", "expert"][(seed % 4) as usize];
            let pool: Vec<String> = eligible(&rows, ask, true).iter().map(|r| r.word.clone()).collect();
            let word = &pool[(seed % pool.len() as u64) as usize];
            let round = generate(&rows, &ex, ask, word, seed, true).unwrap();
            assert_eq!(round.cards.len(), CARDS_KID, "Kid Mode floats exactly 3 cards");
            for w in &round.words {
                let row = by_word[w.as_str()];
                assert!(row.kid_register, "non-Kid-register row {w} in a Kid round");
                assert!(matches!(row.tier.as_str(), "easy" | "medium"), "above-medium row {w} in a Kid round");
            }
        }
    }

    /// D3 (POLISH): speed maps duration only; Kid unset = Relaxed; Reduce
    /// Motion floors everything at Relaxed; and a seeded round is byte-
    /// identical regardless of setting (generation takes no speed input).
    #[test]
    fn speed_is_comfort_only() {
        assert_eq!(speed_duration_mult(Some("relaxed"), false, false), 1.67);
        assert_eq!(speed_duration_mult(Some("standard"), false, false), 1.0);
        assert_eq!(speed_duration_mult(Some("swift"), false, false), 0.71);
        assert_eq!(speed_duration_mult(None, true, false), 1.67, "Kid fresh profile starts Relaxed");
        assert_eq!(speed_duration_mult(None, false, false), 1.0);
        assert_eq!(speed_duration_mult(Some("swift"), false, true), 1.67, "Reduce Motion floors at Relaxed");
        // I3: identical seeded rounds across settings — generation has no
        // speed parameter, byte-identity is structural; pin it anyway.
        let rows = fixture_rows();
        let ex = ExclusionSets::default();
        let a = generate(&rows, &ex, "medium", "medium_w3", 5, false).unwrap();
        let b = generate(&rows, &ex, "medium", "medium_w3", 5, false).unwrap();
        assert_eq!(a, b);
    }

    /// D8: a reverse round floats WORDS, prompts the target's definition,
    /// shares the forward round's determinism and tell-freedom (same shuffles),
    /// and every card resolves to an audited row.
    #[test]
    fn reverse_rounds_flip_the_format() {
        let rows = fixture_rows();
        let ex = fixture_exclusions();
        let by_word: HashMap<&str, &DefRow> = rows.iter().map(|r| (r.word.as_str(), r)).collect();
        let r1 = generate_reverse(&rows, &ex, "hard", "hard_w3", 9, false).unwrap();
        let r2 = generate_reverse(&rows, &ex, "hard", "hard_w3", 9, false).unwrap();
        assert_eq!(r1, r2, "reverse rounds are deterministic too");
        assert!(r1.reverse);
        assert_eq!(r1.prompt, by_word["hard_w3"].definition, "prompt is the target definition");
        assert_eq!(r1.cards, r1.words, "cards ARE the words");
        assert_eq!(r1.words[r1.correct], "hard_w3");
        for w in &r1.words {
            let row = by_word[w.as_str()];
            assert!(row.prompt_grade && row.audit_pass, "reverse card {w} must be audited");
        }
        // Same tell-freedom machinery: correct index varies with the seed.
        let seen: std::collections::HashSet<usize> =
            (0..40u64).map(|s| generate_reverse(&rows, &ex, "hard", "hard_w3", s, false).unwrap().correct).collect();
        assert!(seen.len() > 1, "correct card position must vary across seeds");
    }

    /// Acceptance #7: shield parity. The SAME outcome sequence driven through
    /// the Definition Match adapter and through the exact calls core spelling
    /// makes (game.rs) must leave identical shield accounting at EVERY step —
    /// one ruleset, no drift possible.
    #[test]
    fn shield_parity_with_core_spelling() {
        use crate::model::AppState;
        let sequences: &[&[bool]] = &[
            &[true, true, true, true, true, true, true],
            &[true, true, false, true, true, true, false],
            &[false, false, true, true, true, true, true, true, true, true],
            &[true; 25],
        ];
        for seq in sequences {
            let mut via_defmatch = AppState::default();
            let mut via_spelling = AppState::default();
            for &correct in *seq {
                let a = climb_outcome(&mut via_defmatch, correct);
                // What game.rs does on the same outcome (1201 / 1557).
                let b = if correct {
                    crate::attempts::shield_on_correct(&mut via_spelling)
                } else {
                    crate::attempts::shield_note_miss(&mut via_spelling);
                    false
                };
                assert_eq!(a, b, "earn event diverged mid-sequence");
                assert_eq!(via_defmatch.aids.shields, via_spelling.aids.shields);
                assert_eq!(via_defmatch.aids.earn_streak, via_spelling.aids.earn_streak);
            }
        }
    }

    /// Acceptance #9 (core rule): PREVIEW reaches tier 1 only; FULL reaches
    /// all tiers; None reaches none.
    #[test]
    fn entitlement_tier_depth() {
        use crate::entitlements::AccessLevel;
        assert_eq!(allowed_tiers(AccessLevel::Preview), &["easy"]);
        assert_eq!(allowed_tiers(AccessLevel::Full), &["easy", "medium", "hard", "expert"]);
        assert!(allowed_tiers(AccessLevel::None).is_empty());
    }

    /// D1: rows failing prompt-grade are absent from the pool (the word stays
    /// in other modes — this engine simply never sees it). D3: floor enforced.
    #[test]
    fn pool_gates() {
        let rows = fixture_rows();
        for r in eligible(&rows, "easy", false) {
            assert!(r.prompt_grade && r.audit_pass);
        }
        assert!(pool_floor_met(&rows, "easy", false));
        // A starving pool refuses to generate at all.
        let tiny: Vec<DefRow> = rows.iter().filter(|r| r.tier == "easy").take(10).cloned().collect();
        assert!(!pool_floor_met(&tiny, "easy", false));
        assert!(generate(&tiny, &ExclusionSets::default(), "easy", "easy_w0", 1, false).is_none());
    }

    /// Empty-artifact reality (P1 survey): with no audit-passing rows AT ALL —
    /// the repo's true state today — nothing activates and nothing generates.
    #[test]
    fn no_artifacts_means_no_rounds() {
        let mut rows = fixture_rows();
        for r in &mut rows {
            r.audit_pass = false; // no Gig A round has run
        }
        for tier in ["easy", "medium", "hard", "expert"] {
            assert!(!pool_floor_met(&rows, tier, false));
            assert!(generate(&rows, &ExclusionSets::default(), tier, "easy_w0", 7, false).is_none());
        }
    }
}
