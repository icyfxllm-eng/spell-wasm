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
    /// F6 override: the set is fair only when one of THESE members is the prompt
    /// (the voice says a lone "read" as "reed", so "read" typed for "red" is fair,
    /// "red" typed for "read" is not). Empty = symmetric.
    #[serde(rename = "targetOnly", default)]
    pub target_only: Vec<String>,
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

/// English's second table: the current bank plus the re-tier rows
/// (CC-EN-EXPERT-GROWTH). Served only while `words::retier_active()`, so with
/// en.retier off every reader -- grading, Spelluzzle, Spell Cross, Search --
/// sees exactly the table it saw before.
fn retier_source(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some(include_str!("../assets/words/en/soundalikes-retier.json")),
        _ => None,
    }
}

/// Which table `lang` reads right now: (cache key, source).
fn active(lang: &str) -> (String, Option<&'static str>) {
    if let Some(src) = retier_source(lang) {
        if crate::words::retier_active() {
            return (format!("{lang}#retier"), Some(src));
        }
    }
    (lang.to_string(), source(lang))
}

/// The cache key of the table `lang` reads right now. `crate::homophones`
/// keys its own derived cache on this, so a flag flip cannot serve a stale map.
pub fn active_key(lang: &str) -> String {
    active(lang).0
}

fn missing(lang: &str) -> Table {
    Table { lang: lang.to_string(), status: "missing".into(), voice: None, generator: None, sets: Vec::new() }
}

/// The table for `lang`, parsed once. Unknown languages read as "missing".
pub fn soundalikes(lang: &str) -> &'static Table {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static Table>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let (key, src) = active(lang);
    if let Some(t) = cache.lock().unwrap().get(&key) {
        return t;
    }
    let table = match src {
        // The gate regenerates and checks these files on every push, so a parse
        // failure is a build bug; fall back to "missing" rather than panic in play.
        Some(json) => serde_json::from_str(json).unwrap_or_else(|_| missing(lang)),
        None => missing(lang),
    };
    let leaked: &'static Table = Box::leak(Box::new(table));
    cache.lock().unwrap().insert(key, leaked);
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

    /// Phase B: en/es/ru are verified (generator + flag-only loopback), and the
    /// loopback never changed a kind (I9). English's accent sets come only from a
    /// named merger rule (I7), and grading never reads them.
    #[test]
    fn phase_b_tables_are_verified_and_accent_sets_name_a_rule() {
        for lang in ["en", "es", "ru"] {
            let t = soundalikes(lang);
            assert!(!t.sets.is_empty(), "{lang}: lost its sets");
            assert_eq!(t.status, "verified", "{lang}: generator and loopback both ran");
            for s in &t.sets {
                assert!(s.kind == "true" || s.kind == "accent", "{lang}: bad kind {}", s.kind);
                if s.kind == "accent" {
                    assert!(!s.merger_rules.is_empty(), "{}: an accent set must name its rule", s.set_id);
                }
            }
        }
        // pin-pen is an accent merge for the US voice: grading must not accept it.
        assert!(!crate::homophones::accepts("en", "in", "en"), "accent sets are not accept-all");
        assert!(crate::homophones::accepts("en", "pair", "pear"), "true sets still are");
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

    /// en.retier picks English's table: off -> today's table (so Spelluzzle and
    /// grading are unchanged); on -> the re-tier table; a Spell Jr / Kid Mode
    /// profile keeps today's table even with the flag on.
    #[test]
    fn english_table_follows_en_retier() {
        let count = |on: bool, junior: bool| {
            crate::flags::set_test_override(Some(if on { "on" } else { "off" }));
            crate::words::set_junior_for_test(junior);
            let n = soundalikes("en").sets.len();
            let key = active_key("en");
            crate::flags::set_test_override(None);
            crate::words::set_junior_for_test(false);
            (n, key)
        };
        let (off, off_key) = count(false, false);
        let (on, on_key) = count(true, false);
        let (jr, jr_key) = count(true, true);
        assert_eq!(off_key, "en");
        assert_eq!(on_key, "en#retier");
        assert_eq!(jr_key, "en");
        assert!(on > off, "the re-tier table covers more rows ({on} vs {off})");
        assert_eq!(jr, off, "a junior profile keeps today's table");
        // other languages never switch
        crate::flags::set_test_override(Some("on"));
        assert_eq!(active_key("es"), "es");
        crate::flags::set_test_override(None);
    }

    #[test]
    fn unknown_language_is_missing() {
        assert_eq!(soundalikes("xx").status, "missing");
        assert!(soundalikes("xx").sets.is_empty());
    }
}
