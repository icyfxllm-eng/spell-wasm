//! CC-ONBOARD-JR F1 + F2 — one experience, one tier authority.
//!
//! WHY ONE FUNCTION. Spell Jr ended up doing less than its label promised
//! because every surface asked its own question. The base game capped Kid Mode
//! at Hard (`if s.kid && tier == "expert"`), Word Bee capped it at Medium
//! (`KID_CEILING`), and each comment called its own value "the" kid cap. The
//! Step 0 inventory also found the level selector unfiltered, so a Spell Jr
//! player could pick Expert and be quietly served Hard, and Letter Forge and
//! Practice iterating all four tiers with no kid rule at all. Scattered checks
//! are how a promise drifts. Every surface now asks one thing, `allowed_tiers`,
//! and nothing else decides which difficulty a player may see or be served.
//!
//! THE POLICY IS DATA. Each `config/modes.json` entry declares `juniorPolicy`.
//! A mode without one cannot ship: `scripts/modes-check.mjs` fails the build,
//! and serde refuses to parse the registry because the field has no default.
//! That makes "no Hard or Expert in Spell Jr, in any mode" true by
//! construction, including for modes that do not exist yet.
//!
//! NOTHING PERSISTED IS RENAMED. `AppState.kid` and `age_locked` keep their
//! names and their storage (`byear_prefs_v1`, `byear_agegate_v1`); this module
//! reads them. Renaming a persisted field needs a migration that buys a player
//! nothing (inventory §1).

use std::collections::HashMap;
use std::sync::OnceLock;

/// The difficulty ladder, easiest first. The one place a list of tiers is
/// written; I3's gate allowlists this file.
pub const TIERS: [&str; 4] = ["easy", "medium", "hard", "expert"];

/// Jr Climb serves Easy for this many words, then Medium (F3, D9).
pub const JR_CLIMB_EASY_WORDS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Experience {
    Junior,
    Standard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// An under-13 age-gate verdict. Locked: leaving needs the parent gate.
    AgeGate,
    /// A grown-up passed the parent gate. Recorded for completeness; the
    /// current flow re-runs the birthday prompt, which lands on AgeGate or
    /// Standard, so nothing yet produces this value.
    ParentGate,
    /// A 13+ player switched Spell Jr on themselves. Freely reversible.
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    pub experience: Experience,
    pub locked: bool,
    pub source: Source,
}

/// F1 — the one answer to "is this player junior or standard?".
pub fn resolve(kid: bool, age_locked: bool) -> Resolved {
    if age_locked {
        return Resolved { experience: Experience::Junior, locked: true, source: Source::AgeGate };
    }
    Resolved {
        experience: if kid { Experience::Junior } else { Experience::Standard },
        locked: false,
        source: Source::Toggle,
    }
}

/// Shorthand for callers holding only the flag.
pub fn of_kid(kid: bool) -> Experience {
    if kid { Experience::Junior } else { Experience::Standard }
}

fn policies() -> &'static HashMap<String, String> {
    static P: OnceLock<HashMap<String, String>> = OnceLock::new();
    P.get_or_init(|| crate::modes::all().into_iter().map(|m| (m.id, m.junior_policy)).collect())
}

/// F2 — the tiers `exp` may be offered or served in `mode_id`, easiest first.
///
/// An unknown mode fails CLOSED for a junior player: Easy + Medium. That is a
/// programming error (every rendered mode is registered), but the safe answer
/// to "what may a child be served here?" is never "anything".
pub fn allowed_tiers(exp: Experience, mode_id: &str) -> Vec<&'static str> {
    if exp == Experience::Standard {
        return TIERS.to_vec();
    }
    match policies().get(mode_id).map(String::as_str) {
        Some(p) if p.starts_with("ceiling:") => {
            let cap = &p["ceiling:".len()..];
            match TIERS.iter().position(|t| *t == cap) {
                Some(i) => TIERS[..=i].to_vec(),
                None => TIERS[..2].to_vec(),
            }
        }
        Some("hidden") => Vec::new(),
        // variant:<id>, ungated, and unknown all resolve to Easy + Medium.
        _ => TIERS[..2].to_vec(),
    }
}

/// I5 — the tier actually served. A request outside the allowed set is
/// REFUSED and replaced by the hardest allowed tier: the UI may ask for Hard,
/// the engine does not deliver it. Enforced at word selection, not only in the
/// selector.
pub fn serve_tier(exp: Experience, mode_id: &str, requested: &str) -> &'static str {
    let allowed = allowed_tiers(exp, mode_id);
    if let Some(t) = allowed.iter().find(|t| **t == requested) {
        return t;
    }
    allowed.last().copied().unwrap_or(TIERS[0])
}

/// F3 — the tier for the `word_number`-th word served in a Jr Climb run
/// (1-based). Counts words SERVED, so a miss still advances the count (D9).
pub fn jr_climb_tier(word_number: u32) -> &'static str {
    if word_number <= JR_CLIMB_EASY_WORDS { TIERS[0] } else { TIERS[1] }
}

/// F3 — true for exactly the word that crosses into Medium, so the level-up
/// beat fires once per run.
pub fn jr_climb_levels_up(word_number: u32) -> bool {
    word_number == JR_CLIMB_EASY_WORDS + 1
}

/// I6 — Jr Climb is unranked: zero leaderboard reads or writes.
pub fn leaderboard_allowed(exp: Experience) -> bool {
    exp == Experience::Standard
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Done 1 — allowed_tiers for every mode x {junior, standard} matches the
    /// golden table, which is derived from modes.json by a separate
    /// implementation. A policy edit or a resolver bug shows up here.
    #[test]
    fn allowed_tiers_matches_the_golden_table() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../config/allowed-tiers.golden.json")).unwrap();
        let rows = golden["modes"].as_object().expect("golden has modes");
        let registry: Vec<String> = crate::modes::all().into_iter().map(|m| m.id).collect();
        assert_eq!(rows.len(), registry.len(), "golden covers every registered mode");
        for id in &registry {
            for (exp, key) in [(Experience::Junior, "junior"), (Experience::Standard, "standard")] {
                let want: Vec<String> = rows[id][key].as_array().unwrap()
                    .iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let got: Vec<String> = allowed_tiers(exp, id).iter().map(|s| s.to_string()).collect();
                assert_eq!(got, want, "{id} / {key}");
            }
        }
    }

    /// I5 — no junior request for Hard or Expert is ever served, in any mode.
    #[test]
    fn a_junior_is_never_served_hard_or_expert() {
        for m in crate::modes::all() {
            for req in ["hard", "expert"] {
                let got = serve_tier(Experience::Junior, &m.id, req);
                assert!(got == "easy" || got == "medium",
                        "{}: junior asked for {req}, was served {got}", m.id);
            }
        }
        assert_eq!(serve_tier(Experience::Standard, "standard", "hard"), "hard");
    }

    /// I4 at runtime — every entry carries a policy the resolver understands.
    #[test]
    fn every_registered_mode_declares_a_known_policy() {
        for m in crate::modes::all() {
            let p = m.junior_policy.as_str();
            let known = p == "hidden" || p == "ungated" || p.starts_with("variant:")
                || p.strip_prefix("ceiling:").is_some_and(|t| TIERS.contains(&t));
            assert!(known, "{}: unknown juniorPolicy {p:?}", m.id);
        }
    }

    /// Done 4 — a 25-word Jr Climb run: 1-20 Easy, 21-25 Medium, one level-up
    /// beat, at word 21.
    #[test]
    fn jr_climb_steps_to_medium_at_word_21_exactly_once() {
        let tiers: Vec<&str> = (1..=25).map(jr_climb_tier).collect();
        assert!(tiers[..20].iter().all(|t| *t == "easy"), "words 1-20 are easy");
        assert!(tiers[20..].iter().all(|t| *t == "medium"), "words 21-25 are medium");
        let beats: Vec<u32> = (1..=25).filter(|n| jr_climb_levels_up(*n)).collect();
        assert_eq!(beats, vec![21]);
    }

    /// F1 — the three sources, and the lock only from the age gate.
    #[test]
    fn resolve_maps_the_stored_flags() {
        assert_eq!(resolve(true, true),
                   Resolved { experience: Experience::Junior, locked: true, source: Source::AgeGate });
        assert_eq!(resolve(true, false),
                   Resolved { experience: Experience::Junior, locked: false, source: Source::Toggle });
        assert_eq!(resolve(false, false).experience, Experience::Standard);
        // An age lock wins even if the prefs flag was somehow cleared.
        assert_eq!(resolve(false, true).experience, Experience::Junior);
    }

    /// Only the app build has Word Bee (the site build deletes the mode, D1),
    /// so this proof lives in the configuration that can run it. The site's
    /// junior surfaces are covered by the tests above, which need no mode.
    #[cfg(not(feature = "web"))]
    /// Done 10 (D8 + I5) — a sample of what a Spell Jr player is SERVED, per
    /// language, is 100% kid-safe and 100% Easy or Medium: the base game's
    /// deck, a fortnight of Jr Daily, and Word Bee. The other junior word
    /// sources are proved beside the code that deals them, because comparing a
    /// zh card against the bank means splitting "pinyin|hanzi" and the zh
    /// grading lint reads that shape here as a second grading path:
    /// Impostor in impostor.rs, Word Chains in chains.rs, Letter Forge in
    /// forge.rs.
    #[test]
    fn junior_served_words_are_kid_safe_and_easy_medium() {
        use crate::kid_filter::kid_allowed;
        use std::collections::{HashMap, HashSet};
        let mut banks: HashMap<String, HashSet<&'static str>> = HashMap::new();
        let mut total = 0usize;
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            let lang: &str = code;
            // (surface, locale, served entry)
            let mut served: Vec<(&str, String, String)> = Vec::new();
            let mut s = crate::model::AppState::default();
            s.kid = true;
            s.lang = lang.to_string();
            for tier in allowed_tiers(Experience::Junior, "standard") {
                let pool = crate::game::active_word_list(&s, tier);
                let stride = (pool.len() / 150).max(1);
                served.extend(pool.iter().step_by(stride).take(150).map(|w| ("base game", lang.to_string(), w.clone())));
            }
            for day in 1..=14 {
                let (loc, words) = crate::daily::build_words(lang, &format!("2026-09-{day:02}"), true);
                served.extend(words.into_iter().map(|w| ("Jr Daily", loc.clone(), w)));
            }
            for seed in 0..6u64 {
                served.extend(crate::bee::words(lang, seed, 8, true).into_iter().map(|w| ("Word Bee", lang.to_string(), w)));
            }
            for (surface, loc, w) in &served {
                let bank = banks.entry(loc.clone()).or_insert_with(|| {
                    TIERS[..2].iter().flat_map(|t| crate::words::tier_for(loc, t).iter().copied()).collect()
                });
                assert!(kid_allowed(loc, w), "{lang} {surface}: served {w:?}, which the kid filter refuses");
                assert!(bank.contains(w.as_str()), "{lang} {surface}: served {w:?}, which is not an Easy or Medium word");
            }
            total += served.len();
        }
        assert!(total >= 500 * 10, "the sample is too small to mean anything: {total} words");
    }

    /// I6 — Jr Climb never touches the leaderboard.
    #[test]
    fn junior_has_no_leaderboard() {
        assert!(!leaderboard_allowed(Experience::Junior));
        assert!(leaderboard_allowed(Experience::Standard));
    }
}
