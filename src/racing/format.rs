//! Ghost format v1 (CC-SPELL-RACING Phase 1, spec F1 + D4).
//!
//! A ghost is *just data*: which words (by stable ID from `crate::wordid`), when
//! each was started/finished, each keystroke's time, and correctness. That is why a
//! ghost works across all languages with zero authored content — and it is the
//! foundation for recording, garage, sharing, and pace ghosts.
//!
//! **Invariants (enforced by tests below):**
//! - **Versioned.** `schema_version` is present; a reader REJECTS a ghost from a
//!   newer major version rather than misracing (`LoadError::NewerVersion`).
//! - **No free text.** The only string fields are `language` and `tier`, both
//!   *enumerated identifiers* validated on load. No names, no messages, no custom
//!   strings — so a ghost needs no profanity filter and is COPPA-inert (spec F1/D5).
//! - **Never substitutes.** An unresolvable word aborts the load
//!   (`LoadError::UnresolvableWord`); a word is never silently swapped.
//! - **Deterministic.** serde preserves field order, so encode→decode→encode is
//!   byte-identical (acceptance test #1's basis).

use serde::{Deserialize, Serialize};

use crate::wordid;

/// The current ghost schema version. Bump on a breaking change.
pub const SCHEMA_VERSION: u32 = 1;

/// Preset roster sizes (assets/racing/avatars/manifest.json, gate G-A). Identity is
/// two indices into these — nothing else.
pub const AVATAR_COUNT: u8 = 16;
pub const COLOR_COUNT: u8 = 6;

/// The ONLY identity a ghost carries (D5): an avatar + a colour, both preset
/// indices. No free text of any kind.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Identity {
    pub avatar_id: u8, // 0..AVATAR_COUNT
    pub color_id: u8,  // 0..COLOR_COUNT
}

/// One raced word: the word (by stable ID), when it started/finished within the run
/// (ms from run start), each keystroke's elapsed-ms, and whether it was correct.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WordEvent {
    #[serde(rename = "w")]
    pub word_id: u64,
    #[serde(rename = "s")]
    pub start_ms: u32,
    #[serde(rename = "f")]
    pub finish_ms: u32,
    #[serde(rename = "k")]
    pub keystrokes_ms: Vec<u32>,
    #[serde(rename = "c")]
    pub correct: bool,
}

/// A recorded race. The ordered `events` ARE the track (word IDs in race order);
/// `word_list_hash` lets a reader tell whether its local `(language, tier)` list is
/// byte-identical to the one this ghost was recorded against.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct RaceGhost {
    #[serde(rename = "v")]
    pub schema_version: u32,
    /// Enumerated language code (validated on load), never free text.
    pub language: String,
    /// Enumerated tier (validated on load), never free text.
    pub tier: String,
    #[serde(rename = "h")]
    pub word_list_hash: u64,
    #[serde(rename = "id")]
    pub identity: Identity,
    #[serde(rename = "e")]
    pub events: Vec<WordEvent>,
}

/// Why a ghost could not be loaded. Each maps to a specific human message (F1); the
/// localized strings are wired in Phase 5 via `message_key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// JSON did not parse.
    Malformed,
    /// Recorded by a newer major version → "made in a newer version of Spell."
    NewerVersion,
    /// avatar/color index out of the preset range.
    BadIdentity,
    /// Language not registered/playable on this device.
    UnknownLanguage,
    /// Tier not one of easy/medium/hard/expert.
    UnknownTier,
    /// A word ID isn't in the current list → abort (never substitute).
    UnresolvableWord,
}

impl LoadError {
    /// i18n key for the user-facing message. Strings added to the locales in Phase 5
    /// (this keeps Phase 1 self-contained; no locale changes yet).
    pub fn message_key(self) -> &'static str {
        match self {
            LoadError::Malformed => "ghost.err.malformed",
            LoadError::NewerVersion => "ghost.err.newerVersion",
            LoadError::BadIdentity => "ghost.err.badIdentity",
            LoadError::UnknownLanguage => "ghost.err.unknownLanguage",
            LoadError::UnknownTier => "ghost.err.unknownTier",
            LoadError::UnresolvableWord => "ghost.err.unresolvableWord",
        }
    }
}

/// A validated, fully-resolved ghost ready to race.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LoadedGhost {
    pub ghost: RaceGhost,
    /// The words to race, aligned 1:1 with `ghost.events`.
    pub words: Vec<&'static str>,
    /// True when the local `(language, tier)` list hash matches — exact reproduction.
    /// False means the list changed since recording and we resolved by ID instead.
    pub list_matches: bool,
}

/// Encode a ghost to its wire form. Deterministic (serde field order is fixed).
pub fn encode(g: &RaceGhost) -> String {
    serde_json::to_string(g).expect("RaceGhost always serializes")
}

/// Parse, validate, and resolve a ghost against the CURRENT device state. Returns a
/// `LoadedGhost` or the specific `LoadError` (never a partial/substituted race).
pub fn load(json: &str) -> Result<LoadedGhost, LoadError> {
    let ghost: RaceGhost = serde_json::from_str(json).map_err(|_| LoadError::Malformed)?;
    validate(&ghost)?;

    let mut words = Vec::with_capacity(ghost.events.len());
    for ev in &ghost.events {
        match wordid::resolve(&ghost.language, &ghost.tier, ev.word_id) {
            Some(w) => words.push(w),
            None => return Err(LoadError::UnresolvableWord),
        }
    }
    let list_matches = wordid::list_hash(&ghost.language, &ghost.tier) == ghost.word_list_hash;
    Ok(LoadedGhost { ghost, words, list_matches })
}

fn validate(g: &RaceGhost) -> Result<(), LoadError> {
    // Major-version guard first: a newer ghost must be refused, not misread.
    if g.schema_version > SCHEMA_VERSION {
        return Err(LoadError::NewerVersion);
    }
    if g.identity.avatar_id >= AVATAR_COUNT || g.identity.color_id >= COLOR_COUNT {
        return Err(LoadError::BadIdentity);
    }
    if !crate::consts::is_active_lang(&g.language) {
        return Err(LoadError::UnknownLanguage);
    }
    if !crate::consts::TIER_ORDER.contains(&g.tier.as_str()) {
        return Err(LoadError::UnknownTier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A ghost built from real en/easy words, so it resolves and matches the list.
    fn sample() -> RaceGhost {
        let words = crate::words::tier_for("en", "easy");
        let events = words
            .iter()
            .take(3)
            .enumerate()
            .map(|(i, w)| WordEvent {
                word_id: wordid::word_id(w),
                start_ms: (i as u32) * 1000,
                finish_ms: (i as u32) * 1000 + 800,
                keystrokes_ms: vec![100, 250, 400],
                correct: true,
            })
            .collect();
        RaceGhost {
            schema_version: SCHEMA_VERSION,
            language: "en".to_string(),
            tier: "easy".to_string(),
            word_list_hash: wordid::list_hash("en", "easy"),
            identity: Identity { avatar_id: 3, color_id: 2 },
            events,
        }
    }

    #[test]
    fn encode_decode_is_byte_stable() {
        let g = sample();
        let once = encode(&g);
        let round: RaceGhost = serde_json::from_str(&once).unwrap();
        let twice = encode(&round);
        assert_eq!(once, twice, "encode->decode->encode must be byte-identical");
        assert_eq!(g, round, "value round-trips");
    }

    /// Acceptance #4: the schema carries NO free text. Serialize a fully-populated
    /// ghost and assert every string VALUE in the JSON is one of the two enumerated
    /// identifiers (language, tier). Adding a `name`/`message` field would introduce
    /// a new string value and fail here.
    #[test]
    fn no_free_text_only_enumerated_identifiers() {
        let g = sample();
        let v: serde_json::Value = serde_json::from_str(&encode(&g)).unwrap();
        let mut strings = Vec::new();
        collect_strings(&v, &mut strings);
        let allowed = ["en", "easy"]; // g.language, g.tier
        for s in &strings {
            assert!(
                allowed.contains(&s.as_str()),
                "ghost schema leaked a free-text string value: {s:?}"
            );
        }
        assert_eq!(strings.len(), 2, "exactly language + tier are strings");
    }

    fn collect_strings(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| collect_strings(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| collect_strings(x, out)),
            _ => {}
        }
    }

    #[test]
    fn resolves_when_the_list_matches() {
        let loaded = load(&encode(&sample())).expect("valid ghost loads");
        assert!(loaded.list_matches, "same list -> exact reproduction");
        assert_eq!(loaded.words.len(), 3);
        // words align with events and are real en/easy words.
        for (w, ev) in loaded.words.iter().zip(&loaded.ghost.events) {
            assert_eq!(wordid::word_id(w), ev.word_id);
        }
    }

    #[test]
    fn newer_version_is_refused() {
        let mut g = sample();
        g.schema_version = SCHEMA_VERSION + 1;
        assert_eq!(load(&encode(&g)), Err(LoadError::NewerVersion));
    }

    #[test]
    fn bad_identity_is_refused() {
        let mut g = sample();
        g.identity.avatar_id = 99;
        assert_eq!(load(&encode(&g)), Err(LoadError::BadIdentity));
        let mut g2 = sample();
        g2.identity.color_id = 6;
        assert_eq!(load(&encode(&g2)), Err(LoadError::BadIdentity));
    }

    #[test]
    fn unknown_language_or_tier_is_refused() {
        let mut g = sample();
        g.language = "qqq".to_string();
        assert_eq!(load(&encode(&g)), Err(LoadError::UnknownLanguage));
        let mut g2 = sample();
        g2.tier = "insane".to_string();
        assert_eq!(load(&encode(&g2)), Err(LoadError::UnknownTier));
    }

    #[test]
    fn unresolvable_word_aborts_never_substitutes() {
        let mut g = sample();
        // 0 is not a real word ID in en/easy → must abort, not swap in another word.
        g.events[1].word_id = 0;
        assert_eq!(load(&encode(&g)), Err(LoadError::UnresolvableWord));
    }

    #[test]
    fn malformed_json_is_refused() {
        assert_eq!(load("{not json"), Err(LoadError::Malformed));
    }
}
