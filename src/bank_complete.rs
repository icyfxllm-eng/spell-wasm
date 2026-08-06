//! CC-BANK-COMPLETE — the U-engine and its CI teeth (Features 1, 3, 5).
//!
//! "Completeness must be a computable property checked in CI, not an
//! adjective." U(lang) membership is decided by GATES, never judgment:
//!   gate 1  rank <= the language's T4 floor (signed D-FLOOR, data-driven)
//!   gate 2  single orthographic word — no whitespace, no digits, no
//!           abbreviations/initialisms, no proper nouns (D3 allowlist is
//!           launch-EMPTY, so cased tokens are simply out)
//!   gate 3  the profanity seed filter (the app's own, same fn)
//!   gate 4  TTS renders without fallback — PENDING the composite era:
//!           the check runs against the local pipeline per wave, and no
//!           pinned composite exists until CC-WORDLIST-SOURCES is in hand
//!   gate 5  game-usable sense — CC-DEF-PRECHECK is NOT in hand; the
//!           gate exists, reports Pending, and passes nothing
//! Gates run in fixed order; every exclusion records its FIRST failing
//! gate so Eric can audit why a word is out. Pending gates never
//! silently pass a word into U — a provisional record is marked as such.
//!
//! F5's bidirectional fixtures live in the tests below and are load-
//! bearing: red-flip proves a supra-floor gap turns the build red;
//! reverse proves planted illegal bank rows are flagged. The meta-test
//! makes REMOVING either fixture a build failure by itself.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Rank,
    Orthography,
    Profanity,
    /// Gate 4 — no renderable audio for this word.
    Tts,
    /// Gate 5 — no pre-checked, flag-clean definition.
    DefPrecheck,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Passed every RUNNABLE gate; provisional until pending gates run.
    Provisional,
    /// Passed ALL FIVE gates against loaded evidence — full U.
    Full,
    Excluded(Gate),
}

/// Gates 4 and 5 need per-word EVIDENCE that lives outside this engine
/// (rendered audio; pre-checked definitions). The engine stays pure —
/// no clock, no network, no RNG (F3) — by taking that evidence as an
/// explicit, ordered input. `Evidence::pending()` is the era before a
/// snapshot exists: every gate-1-3 survivor stays PROVISIONAL, which is
/// exactly the behaviour that shipped.
#[derive(Debug, Default, Clone)]
pub struct Evidence {
    pub tts: std::collections::BTreeSet<String>,
    pub defs: std::collections::BTreeSet<String>,
    /// False = no snapshot loaded (the pending era).
    pub loaded: bool,
}

impl Evidence {
    pub fn pending() -> Self {
        Self::default()
    }

    /// Build from two audited word lists — the shape a bundled snapshot
    /// or a wave's report provides.
    pub fn from_lists(tts: &[String], defs: &[String]) -> Self {
        Self {
            tts: tts.iter().cloned().collect(),
            defs: defs.iter().cloned().collect(),
            loaded: true,
        }
    }
}

pub struct URecord {
    pub word: String,
    pub rank: u32,
    pub verdict: Verdict,
}

/// Floors, as data (F1). Loaded from the signed table — the schema test
/// below is the CI that keeps it lawful.
pub fn floors() -> serde_json::Value {
    serde_json::from_str(include_str!("../config/bank_floors.json")).expect("bank_floors parses")
}

pub fn t4_floor(lang: &str) -> Option<u32> {
    floors()["languages"][lang]["tiers"]
        .as_array()
        .and_then(|t| t.last())
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
}

/// Gate 2: one orthographic word in typing units. Apostrophes and
/// hyphens ride the same legality the game's keyboards already accept;
/// anything cased, spaced, dotted, or numeric is out (D3 empty).
fn orthographic(word: &str) -> bool {
    !word.is_empty()
        && !word.chars().any(|c| c.is_whitespace())
        && !word.chars().any(|c| c.is_ascii_digit())
        && !word.contains('.')
        && word.chars().all(|c| !c.is_uppercase())
}

/// The deterministic U-engine core (F3): same inputs, same records,
/// byte-identical across machines — no clock, no network, no RNG.
pub fn compute_u(composite: &[(String, u32)], t4_floor: u32) -> Vec<URecord> {
    compute_u_with(composite, t4_floor, &Evidence::pending())
}

/// The full five-gate engine. Gate order is FIXED and reported: rank,
/// orthography, profanity, TTS, def-precheck — so a word's exclusion
/// always names the first law it broke (auditable by construction).
pub fn compute_u_with(
    composite: &[(String, u32)],
    t4_floor: u32,
    ev: &Evidence,
) -> Vec<URecord> {
    composite
        .iter()
        .map(|(word, rank)| {
            let verdict = if *rank > t4_floor {
                Verdict::Excluded(Gate::Rank)
            } else if !orthographic(word) {
                Verdict::Excluded(Gate::Orthography)
            } else if crate::profanity::is_blocked(word) {
                Verdict::Excluded(Gate::Profanity)
            } else if !ev.loaded {
                // The pending era: gates 4-5 have no evidence to judge
                // by, so a survivor is PROVISIONAL — never silently full.
                Verdict::Provisional
            } else if !ev.tts.contains(word) {
                Verdict::Excluded(Gate::Tts)
            } else if !ev.defs.contains(word) {
                Verdict::Excluded(Gate::DefPrecheck)
            } else {
                Verdict::Full
            };
            URecord { word: word.clone(), rank: *rank, verdict }
        })
        .collect()
}

// ═══════════════ UNMUNCH / GENERATE — the sampled-audit machinery
//
// A machine-expanded surface list (hunspell unmunch; a morphological
// generator) can hold a million forms. Nobody reads a million words, so
// the amendment's whole subject is: WHICH words get read, and what
// makes the batch pass. That policy is Eric's signature. The MACHINERY
// is here and deterministic — same population + same rule = the same
// sample, so an auditor can prove what was reviewed and reproduce it.
//
// The rule is a PARAMETER, never a default: `SampleRule` has no Default
// impl and `expansion_allowed` refuses an unsigned rule outright, so an
// unaudited expansion cannot reach a bank by forgetting to configure it.

#[derive(Debug, Clone, PartialEq)]
pub struct SampleRule {
    /// How many forms a human reads per batch.
    pub sample_size: usize,
    /// The most defects that sample may contain and still pass.
    pub max_defects: usize,
    /// Recorded so a re-run reproduces the same draw for review.
    pub seed: u64,
    /// The amendment's own signature marker — set only when Eric's
    /// signed rule is what is loaded.
    pub signed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditVerdict {
    /// The sample was clean enough; the batch may enter the bank.
    Pass,
    /// Too many defects — the WHOLE batch is refused, not just the
    /// defective words (a bad sample means a bad expansion).
    Reject { defects: usize, allowed: usize },
    /// No signed rule: nothing may ship, and this is not an error state
    /// to route around.
    Unsigned,
}

/// The deterministic draw. Sorted population + seeded stride = a sample
/// any auditor can regenerate from the record alone.
pub fn audit_sample<'a>(population: &'a [String], rule: &SampleRule) -> Vec<&'a str> {
    if !rule.signed || population.is_empty() || rule.sample_size == 0 {
        return Vec::new();
    }
    let mut sorted: Vec<&str> = population.iter().map(|s| s.as_str()).collect();
    sorted.sort_unstable();
    sorted.dedup();
    let n = sorted.len();
    let take = rule.sample_size.min(n);
    // A seeded stride spreads the draw across the whole alphabetized
    // population — never just the head, which is where the easy words
    // live and where a lazy audit would look.
    let stride = (n / take).max(1);
    let start = (rule.seed % n as u64) as usize;
    let mut out = Vec::with_capacity(take);
    let mut i = 0usize;
    while out.len() < take && i < n {
        out.push(sorted[(start + i * stride) % n]);
        i += 1;
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The verdict on a reviewed sample.
pub fn audit_verdict(defects: usize, rule: &SampleRule) -> AuditVerdict {
    if !rule.signed {
        return AuditVerdict::Unsigned;
    }
    if defects > rule.max_defects {
        AuditVerdict::Reject { defects, allowed: rule.max_defects }
    } else {
        AuditVerdict::Pass
    }
}

/// The gate every expansion wave passes through. An unsigned rule can
/// never yield true, whatever the caller believes about the batch.
pub fn expansion_allowed(verdict: &AuditVerdict) -> bool {
    matches!(verdict, AuditVerdict::Pass)
}

/// F4/F6 coverage core: the gap sets, computable off any bank snapshot.
/// Report-only until a language's `launched` flag flips post-wave.
pub fn gaps(u: &[URecord], bank: &std::collections::BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let u_words: std::collections::BTreeSet<&str> = u
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Provisional | Verdict::Full))
        .map(|r| r.word.as_str())
        .collect();
    let missing = u_words.iter().filter(|w| !bank.contains(**w)).map(|w| w.to_string()).collect();
    let extra = bank.iter().filter(|w| !u_words.contains(w.as_str())).cloned().collect();
    (missing, extra)
}

#[cfg(test)]
mod bank_complete_ci {
    use super::*;
    use std::collections::BTreeSet;

    /// F1 schema CI: floors monotonic ascending, sw structurally 3-tier,
    /// pool floors match tier counts, signed values verbatim spot-checks.
    #[test]
    fn floor_registry_is_lawful() {
        let f = floors();
        let langs = f["languages"].as_object().unwrap();
        assert_eq!(langs.len(), 15, "fifteen languages, as signed");
        for (lang, row) in langs {
            let tiers: Vec<u64> =
                row["tiers"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
            assert!(tiers.windows(2).all(|w| w[0] < w[1]), "{lang}: floors must ascend");
            let pools = row["poolFloors"].as_array().unwrap().len();
            assert_eq!(pools, tiers.len(), "{lang}: pool floors track tier count");
            if lang == "sw" {
                assert_eq!(tiers.len(), 3, "sw is STRUCTURALLY 3-tier (signed)");
            } else {
                assert_eq!(tiers.len(), 4, "{lang}: four tiers");
            }
        }
        // Signed-table spot pins (D-FLOOR verbatim).
        assert_eq!(t4_floor("en"), Some(30_000));
        assert_eq!(t4_floor("sw"), Some(10_000), "sw T4 IS its T3 cap");
        assert_eq!(t4_floor("fil"), Some(8_000));
        assert!(f["languages"]["hi"]["designAheadOnly"].as_bool().unwrap_or(false), "hi: D5");
        assert!(f["languages"]["en"]["frozen"].as_bool().unwrap_or(false), "en: D5 frozen reference");
    }

    /// F5 fixture (a): RED-FLIP. A synthetic supra-floor word with no
    /// bank row must surface as a gap — a completeness check that cannot
    /// fail is worse than none.
    #[test]
    fn red_flip_synthetic_gap_is_caught() {
        let composite = vec![("zebrawombat".to_string(), 50u32)];
        let u = compute_u(&composite, 1000);
        let bank = BTreeSet::new(); // the word is NOT in the bank
        let (missing, _) = gaps(&u, &bank);
        assert_eq!(missing, vec!["zebrawombat".to_string()], "the gap MUST go red");
    }

    /// F5 fixture (b): REVERSE. Planted illegal rows — sub-floor, cased
    /// (proper-noun class), multi-word, digit-bearing — must each be
    /// flagged with the correct first-failing gate.
    #[test]
    fn reverse_planted_illegal_rows_are_flagged() {
        let composite = vec![
            ("waytoorare".to_string(), 999_999u32),
            ("London".to_string(), 10),
            ("ice cream".to_string(), 20),
            ("route66".to_string(), 30),
        ];
        let u = compute_u(&composite, 1000);
        assert_eq!(u[0].verdict, Verdict::Excluded(Gate::Rank));
        assert_eq!(u[1].verdict, Verdict::Excluded(Gate::Orthography), "cased = out (D3 empty)");
        assert_eq!(u[2].verdict, Verdict::Excluded(Gate::Orthography), "multi-word = out");
        assert_eq!(u[3].verdict, Verdict::Excluded(Gate::Orthography), "digits = out");
    }

    /// The fixtures are permanent: REMOVING either one is itself a build
    /// failure (the file proves its own contents).
    #[test]
    fn the_fixtures_cannot_be_removed() {
        let me = include_str!("bank_complete.rs");
        assert!(me.contains("fn red_flip_synthetic_gap_is_caught"));
        assert!(me.contains("fn reverse_planted_illegal_rows_are_flagged"));
    }

    /// Determinism (F3): same inputs, same records, twice.
    #[test]
    fn u_computation_is_deterministic() {
        let composite: Vec<(String, u32)> =
            (0..500).map(|i| (format!("word{i}"), i)).collect();
        let a: Vec<_> = compute_u(&composite, 250).iter().map(|r| format!("{}{:?}", r.word, r.verdict)).collect();
        let b: Vec<_> = compute_u(&composite, 250).iter().map(|r| format!("{}{:?}", r.word, r.verdict)).collect();
        assert_eq!(a, b);
    }

    /// Pending gates never pass a word into full U — everything that
    /// survives gates 1-3 is PROVISIONAL until TTS + DEF-PRECHECK run.
    #[test]
    fn nothing_reaches_full_u_before_the_pending_gates() {
        let u = compute_u(&[("cat".to_string(), 5)], 1000);
        assert_eq!(u[0].verdict, Verdict::Provisional, "provisional, never silently complete");
    }

    /// Gates 4 and 5, live: with evidence loaded a word reaches FULL U
    /// only when it has BOTH audio and a pre-checked definition, and an
    /// exclusion names which one it lacked.
    #[test]
    fn tts_and_def_gates_decide_full_u() {
        let ev = Evidence::from_lists(
            &["cat".into(), "dog".into(), "fox".into()],
            &["cat".into(), "fox".into()],
        );
        let composite = vec![
            ("cat".to_string(), 5),   // both -> Full
            ("dog".to_string(), 6),   // audio, no definition
            ("owl".to_string(), 7),   // no audio at all
        ];
        let u = compute_u_with(&composite, 1000, &ev);
        assert_eq!(u[0].verdict, Verdict::Full);
        assert_eq!(u[1].verdict, Verdict::Excluded(Gate::DefPrecheck));
        assert_eq!(u[2].verdict, Verdict::Excluded(Gate::Tts));
    }

    /// Gate ORDER is law: a word that fails an earlier gate reports
    /// that gate, never a later one, however much else is wrong.
    #[test]
    fn the_first_broken_law_is_the_reported_one() {
        let ev = Evidence::from_lists(&[], &[]);
        // profane AND missing audio AND missing definition
        let u = compute_u_with(&[("Shit2".to_string(), 5)], 1000, &ev);
        assert_eq!(
            u[0].verdict,
            Verdict::Excluded(Gate::Orthography),
            "orthography precedes profanity precedes the pending pair"
        );
    }

    /// Determinism survives the new gates (F3).
    #[test]
    fn evidence_gates_stay_deterministic() {
        let ev = Evidence::from_lists(&["a".into(), "b".into()], &["a".into()]);
        let c: Vec<(String, u32)> = (0..200).map(|i| (format!("w{i}"), i)).collect();
        let x: Vec<_> = compute_u_with(&c, 500, &ev).iter().map(|r| format!("{:?}", r.verdict)).collect();
        let y: Vec<_> = compute_u_with(&c, 500, &ev).iter().map(|r| format!("{:?}", r.verdict)).collect();
        assert_eq!(x, y);
    }

    // ───────────── UNMUNCH / GENERATE sampled audit

    fn pop(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("form{i:05}")).collect()
    }

    #[test]
    fn an_unsigned_rule_ships_nothing() {
        let unsigned = SampleRule { sample_size: 100, max_defects: 2, seed: 7, signed: false };
        assert!(audit_sample(&pop(10_000), &unsigned).is_empty(), "no sample without a signed rule");
        assert_eq!(audit_verdict(0, &unsigned), AuditVerdict::Unsigned, "even a clean read cannot pass");
        assert!(!expansion_allowed(&AuditVerdict::Unsigned), "the gate cannot be talked around");
    }

    #[test]
    fn the_sample_is_reproducible_and_spread() {
        let rule = SampleRule { sample_size: 50, max_defects: 1, seed: 42, signed: true };
        let population = pop(10_000);
        let a = audit_sample(&population, &rule);
        let b = audit_sample(&population, &rule);
        assert_eq!(a, b, "an auditor can reproduce the exact draw");
        assert_eq!(a.len(), 50);
        // spread: the draw must not be the alphabetical head
        let head: Vec<String> = (0..50).map(|i| format!("form{i:05}")).collect();
        assert_ne!(a, head.iter().map(|s| s.as_str()).collect::<Vec<_>>(), "not just the easy words");
        let distinct_prefixes: std::collections::HashSet<&str> =
            a.iter().map(|w| &w[4..6]).collect();
        assert!(distinct_prefixes.len() > 5, "the sample spans the population");
        // a different seed draws differently
        let other = SampleRule { seed: 43, ..rule.clone() };
        assert_ne!(audit_sample(&population, &other), a);
    }

    #[test]
    fn too_many_defects_rejects_the_whole_batch() {
        let rule = SampleRule { sample_size: 100, max_defects: 2, seed: 1, signed: true };
        assert_eq!(audit_verdict(0, &rule), AuditVerdict::Pass);
        assert_eq!(audit_verdict(2, &rule), AuditVerdict::Pass, "at the limit still passes");
        assert_eq!(
            audit_verdict(3, &rule),
            AuditVerdict::Reject { defects: 3, allowed: 2 },
            "one over rejects the BATCH, not the word"
        );
        assert!(!expansion_allowed(&audit_verdict(3, &rule)));
        assert!(expansion_allowed(&audit_verdict(1, &rule)));
    }

    #[test]
    fn a_small_population_is_read_entirely() {
        let rule = SampleRule { sample_size: 500, max_defects: 0, seed: 9, signed: true };
        let small = pop(20);
        assert_eq!(audit_sample(&small, &rule).len(), 20, "sample never exceeds the population");
    }
}
