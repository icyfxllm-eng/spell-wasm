//! CC-AUDIO-CLARITY v1.1 F2 — the intelligibility verdict, as the app reads it.
//!
//! The verdicts themselves are produced offline by the harness
//! (`scripts/audio-verdicts.mjs`), which transcribes every served clip with two
//! recognizers and never shows either of them the bank text (I3). This module
//! is only the consumer: it answers one question, "may this clip be served",
//! and it is the mechanism behind I1 — a FAIL clip is never heard.
//!
//! It does NOT choose a fallback. When a clip is withheld the resolver simply
//! moves to its next source, exactly as it does for a clip that will not load
//! (CC-BUILD219-FIXES owns that order, and I5 keeps it one resolver).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Verdict {
    /// Both recognizers returned the word.
    Pass,
    /// Exactly one did. Served, and listed for review (D7).
    Weak,
    /// Neither did. Never served (I1).
    Fail,
    /// In the CC-SENSE-CUE collision table: not recognizable by design, so a
    /// recognizer disagreeing proves nothing.
    ExemptHomophone,
    /// No comparison key or no recognizer for this language (census C8).
    /// Served, and listed -- withholding audio from a whole language because we
    /// cannot yet score it would be a worse outcome than the risk.
    Unscorable,
}

impl Verdict {
    /// The one rule this file exists for.
    pub fn servable(self) -> bool {
        !matches!(self, Verdict::Fail)
    }
}

#[derive(Deserialize)]
struct Entry {
    v: Verdict,
    /// What each recognizer heard, kept so a verdict can be argued with. The
    /// app never reads it; the report and a human do.
    #[serde(default)]
    #[allow(dead_code)]
    heard: Vec<String>,
}

#[derive(Deserialize, Default)]
struct Store {
    /// Languages whose clip set has been measured end to end. A language is
    /// added here only when its verdicts are complete, which is what lets F7's
    /// CI gate be strict about measured languages without bricking the gate for
    /// the ones whose measurement has not run yet.
    #[serde(default)]
    measured: Vec<String>,
    #[serde(default)]
    clips: HashMap<String, Entry>,
}

fn store() -> &'static Store {
    static S: OnceLock<Store> = OnceLock::new();
    S.get_or_init(|| serde_json::from_str(include_str!("../config/audio-verdicts.json")).unwrap_or_default())
}

/// The key a verdict is filed under. The voice is not in it on purpose: the
/// server picks the voice per language, so a voice change regenerates the clips
/// AND the verdicts together (I8), and the app never has to know which voice it
/// is hearing.
fn key(lang: &str, word: &str, variant: &str) -> String {
    format!("{lang}|{word}|{variant}")
}

pub fn verdict(lang: &str, word: &str, variant: &str) -> Option<Verdict> {
    store().clips.get(&key(lang, word, variant)).map(|e| e.v)
}

/// Whether this language's clips have been measured at all.
pub fn measured(lang: &str) -> bool {
    store().measured.iter().any(|l| l == lang)
}

/// May this clip be served?
///
/// An unmeasured language is served: the alternative is silence everywhere the
/// harness has not reached yet, which is worse for a player than an unverified
/// clip and is not what I1 is for. F7 reports the gap instead.
pub fn servable(lang: &str, word: &str, variant: &str) -> bool {
    match verdict(lang, word, variant) {
        Some(v) => v.servable(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fail_is_withheld() {
        assert!(!Verdict::Fail.servable(), "I1: a FAIL clip is never heard");
        for v in [Verdict::Pass, Verdict::Weak, Verdict::ExemptHomophone, Verdict::Unscorable] {
            assert!(v.servable(), "{v:?} is served");
        }
    }

    #[test]
    fn the_shipped_store_parses_and_withholds_what_it_says() {
        // The file ships with the app; a malformed one would silently serve
        // everything, so parsing it is part of the build.
        let s = store();
        for (k, e) in &s.clips {
            let parts: Vec<&str> = k.split('|').collect();
            assert_eq!(parts.len(), 3, "key {k} is lang|word|variant");
            assert_eq!(servable(parts[0], parts[1], parts[2]), e.v.servable(), "{k}");
        }
    }

    /// I1 is only true while the resolver actually asks. This is the scan that
    /// notices if the precondition is ever removed or moved below the point
    /// where a source starts playing.
    #[test]
    fn the_resolver_still_asks_before_it_plays() {
        let api = include_str!("api.rs");
        let i = api.find("fn play_chain(").expect("play_chain moved");
        let body = &api[i..];
        let end = body.find("\n}\n").unwrap_or(body.len());
        let body = &body[..end];
        let asks = body.find("audio_verdict::servable").expect("play_chain no longer consults the verdict (I1)");
        let plays = body.find("Source::Human => play_human").expect("the source match moved");
        assert!(asks < plays, "the verdict must be consulted BEFORE a source plays");
        assert!(body.contains("next();"), "a withheld clip must advance the router, not end the chain");
    }

    #[test]
    fn an_unmeasured_clip_is_served_and_says_so() {
        assert!(servable("xx", "nosuchword", "normal"), "an unmeasured clip still plays");
        assert_eq!(verdict("xx", "nosuchword", "normal"), None);
        assert!(!measured("xx"), "and the language is not claimed as measured");
    }
}
