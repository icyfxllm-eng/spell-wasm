//! CC-SENSE-CUE v1.1 F7 -- the ONE read API for sound-alike tables (Rust side).
//!
//! Every registry language has a generated table at
//! `assets/words/<lang>/soundalikes.json` (tools/build_collisions.py). Nothing
//! else in the app opens those files; `crate::homophones` is a view over this
//! module, and scripts/soundalike-api-check.mjs fails CI on any other reader.
//!
//! `status` says whether a table was checked (F2): "verified",
//! "none-by-construction" or "missing". A language with no table reads as
//! "missing" with no sets, which is distinct from "verified" with zero sets
//! (I4). H1: status gates cues, the sound-alike bonus and Impossible only --
//! Normal grading reads the sets whatever the status, exactly as it did when
//! these were plain-text homophone files.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

// Fields the Phase A consumers do not read yet (wordId, setId, mergerRules,
// confirmed, generator) are the API the bonus, Impossible and the report use.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    #[serde(rename = "memberId")]
    pub member_id: String,
    /// The bank row this member is, or `None` for a real word outside the bank.
    #[serde(rename = "wordId")]
    pub word_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SoundalikeSet {
    #[serde(rename = "setId")]
    pub set_id: String,
    /// "true" (collide in the active voice) or "accent" (collide only under a
    /// named merger rule, F4).
    pub kind: String,
    #[serde(rename = "mergerRules", default)]
    pub merger_rules: Vec<String>,
    #[serde(default)]
    pub confirmed: bool,
    pub members: Vec<Member>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    pub lang: String,
    pub status: String,
    pub voice: Option<String>,
    pub generator: Option<String>,
    pub sets: Vec<SoundalikeSet>,
}

/// Per-language table source. One arm per registry language, so a language
/// without data still has a table that SAYS so.
fn source(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => include_str!("../assets/words/en/soundalikes.json"),
        "es" => include_str!("../assets/words/es/soundalikes.json"),
        "fr" => include_str!("../assets/words/fr/soundalikes.json"),
        "de" => include_str!("../assets/words/de/soundalikes.json"),
        "pt" => include_str!("../assets/words/pt/soundalikes.json"),
        "pl" => include_str!("../assets/words/pl/soundalikes.json"),
        "vi" => include_str!("../assets/words/vi/soundalikes.json"),
        "ko" => include_str!("../assets/words/ko/soundalikes.json"),
        "ja" => include_str!("../assets/words/ja/soundalikes.json"),
        "fil" => include_str!("../assets/words/fil/soundalikes.json"),
        "zh" => include_str!("../assets/words/zh/soundalikes.json"),
        "ru" => include_str!("../assets/words/ru/soundalikes.json"),
        "ar" => include_str!("../assets/words/ar/soundalikes.json"),
        "sw" => include_str!("../assets/words/sw/soundalikes.json"),
        "hi" => include_str!("../assets/words/hi/soundalikes.json"),
        _ => return None,
    })
}

fn missing(lang: &str) -> Table {
    Table { lang: lang.to_string(), status: "missing".into(), voice: None, generator: None, sets: Vec::new() }
}

/// The table for `lang`, parsed once. Unknown languages read as "missing".
pub fn soundalikes(lang: &str) -> &'static Table {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static Table>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(t) = cache.lock().unwrap().get(lang) {
        return t;
    }
    let table = match source(lang) {
        // The gate regenerates and checks these files on every push, so a parse
        // failure is a build bug; fall back to "missing" rather than panic in play.
        Some(json) => serde_json::from_str(json).unwrap_or_else(|_| missing(lang)),
        None => missing(lang),
    };
    let leaked: &'static Table = Box::leak(Box::new(table));
    cache.lock().unwrap().insert(lang.to_string(), leaked);
    leaked
}

/// I6: a table read against a voice it was not built for reads as "missing".
/// The client does not choose voices, so it calls this only where a voice is
/// known; the gate separately fails if a table's voice is not the backend's.
#[allow(dead_code)]
pub fn status_for_voice(lang: &str, voice: &str) -> &'static str {
    let t = soundalikes(lang);
    if t.voice.as_deref() == Some(voice) {
        match t.status.as_str() {
            "verified" => "verified",
            "none-by-construction" => "none-by-construction",
            _ => "missing",
        }
    } else {
        "missing"
    }
}

/// Member lists of every set whose kind is in `kinds`. SENSE-CUE grading reads
/// `&["true"]`; CC-IMPOSSIBLE reads `&["true", "accent"]`.
pub fn sets_of_kind(lang: &str, kinds: &[&str]) -> Vec<&'static SoundalikeSet> {
    soundalikes(lang).sets.iter().filter(|s| kinds.contains(&s.kind.as_str())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: [&str; 15] = ["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "zh", "ru", "ar", "sw", "hi"];

    /// I4: every registry language has a parseable table, so "checked and empty"
    /// can be told from "never built".
    #[test]
    fn every_registry_language_has_a_table() {
        for lang in REGISTRY {
            let t = soundalikes(lang);
            assert_eq!(t.lang, lang, "{lang}: table failed to parse or names the wrong language");
            assert!(["verified", "none-by-construction", "missing"].contains(&t.status.as_str()), "{lang}: bad status");
        }
    }

    /// Phase A migrates today's data only: en/es/ru keep their sets, nothing is
    /// verified yet (the F5 loopback has not run), and every set is kind "true".
    #[test]
    fn phase_a_carries_existing_sets_unverified() {
        for lang in ["en", "es", "ru"] {
            let t = soundalikes(lang);
            assert!(!t.sets.is_empty(), "{lang}: lost its sets in migration");
            assert_eq!(t.status, "missing", "{lang}: cannot be verified before the loopback runs");
            assert!(t.sets.iter().all(|s| s.kind == "true"), "{lang}: Phase A adds no kinds");
        }
    }

    /// S1: one set identity. Every setId is unique and namespaced by language.
    #[test]
    fn set_ids_are_unique_and_namespaced() {
        let mut seen = std::collections::HashSet::new();
        for lang in REGISTRY {
            for s in &soundalikes(lang).sets {
                assert!(s.set_id.starts_with(&format!("{lang}:")), "{}: not namespaced", s.set_id);
                assert!(seen.insert(s.set_id.clone()), "{}: duplicate setId", s.set_id);
            }
        }
    }

    /// I6: a voice mismatch reads as missing.
    #[test]
    fn voice_mismatch_reads_missing() {
        assert_eq!(status_for_voice("en", "en-US-Not-A-Voice"), "missing");
    }

    #[test]
    fn unknown_language_is_missing() {
        assert_eq!(soundalikes("xx").status, "missing");
        assert!(soundalikes("xx").sets.is_empty());
    }
}
