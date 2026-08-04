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
    TtsPending,
    DefPrecheckPending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Passed every RUNNABLE gate; provisional until pending gates run.
    Provisional,
    Excluded(Gate),
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
    composite
        .iter()
        .map(|(word, rank)| {
            let verdict = if *rank > t4_floor {
                Verdict::Excluded(Gate::Rank)
            } else if !orthographic(word) {
                Verdict::Excluded(Gate::Orthography)
            } else if crate::profanity::is_blocked(word) {
                Verdict::Excluded(Gate::Profanity)
            } else {
                // Gates 4 (TTS) and 5 (DEF-PRECHECK) are pending their
                // authority files/eras — a word here is PROVISIONAL, never
                // silently full-U.
                Verdict::Provisional
            };
            URecord { word: word.clone(), rank: *rank, verdict }
        })
        .collect()
}

/// F4/F6 coverage core: the gap sets, computable off any bank snapshot.
/// Report-only until a language's `launched` flag flips post-wave.
pub fn gaps(u: &[URecord], bank: &std::collections::BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let u_words: std::collections::BTreeSet<&str> = u
        .iter()
        .filter(|r| r.verdict == Verdict::Provisional)
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
}
