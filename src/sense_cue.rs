//! CC-SENSE-CUE F6 — per-language cue mode.
//!
//! One asset class, two very different exercises, and a third state where
//! neither runs. Making that a registry field rather than a code branch is what
//! lets a language roll out without a build, and what keeps the decision
//! inspectable instead of scattered through grading.
//!
//! D9 IS ENFORCED HERE, NOT MERELY DOCUMENTED. A language may claim `support`
//! or `sole`, but without a named native auditor this resolves to `off`. The
//! failure mode that matters is a forgotten audit: someone flips a mode, the
//! auditor never materialises, and unaudited images ship to children. Making
//! the claim self-voiding means the safe state is the default state.
//!
//! This mirrors `gloss_audited` in src/translate.rs, where `audited` is a human
//! claim no tool sets — the same reasoning, applied to a different asset class.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueMode {
    /// No cues. Collision sets fall back to accept-all (F2a) — Phase B.
    Off,
    /// Audio plays normally; the cue appears only via Second Look.
    Support,
    /// The picture is the entire prompt; audio is suppressed.
    Sole,
}

#[derive(Deserialize)]
struct Reg {
    languages: HashMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "cueMode", default)]
    cue_mode: String,
    #[serde(default)]
    auditor: Option<String>,
}

fn registry() -> &'static Reg {
    use std::sync::OnceLock;
    static R: OnceLock<Reg> = OnceLock::new();
    R.get_or_init(|| {
        serde_json::from_str(include_str!("../config/sense-cue.json"))
            .expect("sense-cue.json parses")
    })
}

/// The effective mode for a language. Unknown languages are `Off`, and so is
/// any language whose claim outruns its auditor.
pub fn cue_mode(lang: &str) -> CueMode {
    let Some(e) = registry().languages.get(lang) else {
        return CueMode::Off;
    };
    let named = e.auditor.as_deref().is_some_and(|a| !a.trim().is_empty());
    match e.cue_mode.as_str() {
        "support" if named => CueMode::Support,
        "sole" if named => CueMode::Sole,
        _ => CueMode::Off,
    }
}

/// Is any language currently serving cues? Used by the Invariant 4 sweep, so
/// that "no sole-mode screen has audio" is checked against reality rather than
/// assumed because nothing is enabled yet.
pub fn any_sole_language() -> bool {
    registry().languages.keys().any(|l| cue_mode(l) == CueMode::Sole)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase B's state: every language off, so collision sets stay accept-all
    /// and nothing a player sees changes.
    #[test]
    fn every_language_starts_off() {
        for lang in registry().languages.keys() {
            assert_eq!(cue_mode(lang), CueMode::Off, "{lang} must ship off");
        }
        assert!(!any_sole_language());
    }

    /// D9 AS LAW. The claim is self-voiding without a named auditor — this is
    /// the test that stops a forgotten audit from shipping unaudited images to
    /// a child. Parsed from a fixture so it holds independently of what the
    /// real registry currently says.
    #[test]
    fn a_mode_without_a_named_auditor_resolves_to_off() {
        let r: Reg = serde_json::from_str(
            r#"{"languages":{
                 "a":{"cueMode":"support","auditor":null},
                 "b":{"cueMode":"sole","auditor":"  "},
                 "c":{"cueMode":"support","auditor":"Paul"},
                 "d":{"cueMode":"sole","auditor":"Paul"}}}"#,
        )
        .unwrap();
        let eff = |k: &str| {
            let e = &r.languages[k];
            let named = e.auditor.as_deref().is_some_and(|a| !a.trim().is_empty());
            match e.cue_mode.as_str() {
                "support" if named => CueMode::Support,
                "sole" if named => CueMode::Sole,
                _ => CueMode::Off,
            }
        };
        assert_eq!(eff("a"), CueMode::Off, "no auditor -> off");
        assert_eq!(eff("b"), CueMode::Off, "blank auditor -> off");
        assert_eq!(eff("c"), CueMode::Support, "named auditor -> claim honoured");
        assert_eq!(eff("d"), CueMode::Sole);
    }

    /// An unknown language is off, not a panic: the registry is data and may
    /// lag the language list.
    #[test]
    fn an_unregistered_language_is_off() {
        assert_eq!(cue_mode("xx"), CueMode::Off);
        assert_eq!(cue_mode(""), CueMode::Off);
    }

    /// Invariant 4 — no sole-mode screen has a reachable audio path. There is
    /// no sole language yet, so this asserts the PRECONDITION that makes that
    /// true, and will start doing real work the day one is enabled.
    #[test]
    fn invariant_4_holds_because_no_language_is_sole() {
        assert!(!any_sole_language(),
                "a sole language exists — Invariant 4 now needs a real audio sweep, \
                 not this precondition check");
    }
}
