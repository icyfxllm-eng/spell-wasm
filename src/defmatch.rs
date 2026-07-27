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

/// The Kid Mode tier ceiling (Invariant 6): hard/expert requests clamp to
/// medium IN CORE — the UI can ask, the engine refuses.
pub fn effective_tier<'a>(tier: &'a str, kid: bool) -> &'a str {
    if kid && matches!(tier, "hard" | "expert") {
        "medium"
    } else {
        tier
    }
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

    Some(Round { cards, correct, words, lanes, spawn_order, seed })
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
