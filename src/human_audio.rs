//! CC-HUMAN-AUDIO F5/F6 — verified human recordings, as one provider inside the
//! ONE audio router (`api::play_word_with`, I1). There is no second play path.
//!
//! What may play is decided at BUILD time, not here. `tools/human-audio/bundle.py`
//! writes `assets/human-audio/runtime.json` from the ingested auditor verdicts
//! (the only write path, I2): a clip is listed only if its accept verdict is
//! bound to its exact bytes, and only for a (language, tier) whose verified
//! coverage reached D3's 80%. Below that the whole tier stays on TTS, so a
//! player never hears the voice change every other word. The clips ship inside
//! the app and the site (D13); nothing is fetched from a third party (I9).
//!
//! At play time this module answers two questions: is the player's "Real voices
//! when available" switch on (F6, D4 default on), and is there a clip for this
//! word. Grading never reads anything here (I5).

use std::cell::Cell;
use std::collections::HashMap;

/// The build-time manifest. Compiled in, so the app and the site resolve the
/// same word to the same clip (D10) without a fetch.
#[cfg(not(feature = "testseam"))]
const RUNTIME: &str = include_str!("../assets/human-audio/runtime.json");
/// The e2e build compiles build.rs's fixture instead (every English bank word
/// -> one test clip), inert until a spec arms it; see `fixture_armed`.
#[cfg(feature = "testseam")]
const RUNTIME: &str = include_str!(concat!(env!("OUT_DIR"), "/human-audio-fixture.json"));

#[derive(serde::Deserialize, Default)]
struct Runtime {
    #[serde(default)]
    langs: HashMap<String, LangClips>,
}

#[derive(serde::Deserialize, Default)]
struct LangClips {
    /// Path of the clip folder relative to the app root, e.g. "human-audio/en/".
    base: String,
    /// entry -> file name within `base`.
    clips: HashMap<String, String>,
}

thread_local! {
    static MANIFEST: Runtime = serde_json::from_str(RUNTIME).unwrap_or_default();
    /// F6: mirrors the player's switch; set by `settings::apply_settings`.
    static ENABLED: Cell<bool> = const { Cell::new(true) };
}

/// F6 — "Real voices when available". Called from `apply_settings`, the one
/// place settings take effect, so every call site of the router obeys it.
pub fn set_enabled(on: bool) {
    ENABLED.with(|c| c.set(on));
}

pub fn enabled() -> bool {
    ENABLED.with(Cell::get)
}

/// The bundled clip for `word` in `lang`, as a URL relative to the app root —
/// or None when the switch is off (F6) or no verified clip exists.
pub fn clip_url(lang: &str, word: &str) -> Option<String> {
    MANIFEST.with(|m| resolve(m, lang, word))
}

/// The switch and the manifest, in that order: off means no clip at all.
fn resolve(m: &Runtime, lang: &str, word: &str) -> Option<String> {
    if !enabled() || !fixture_armed() {
        return None;
    }
    lookup(m, lang, word)
}

/// Production: always true, the manifest is the real one. Testseam: the fixture
/// maps EVERY English word, which would reroute every other spec's audio, so it
/// answers only when the human-audio spec sets `spell_test_human_audio`.
#[cfg(not(feature = "testseam"))]
fn fixture_armed() -> bool {
    true
}
#[cfg(feature = "testseam")]
fn fixture_armed() -> bool {
    cfg!(test) || crate::storage::get_raw("spell_test_human_audio").is_some()
}

fn lookup(m: &Runtime, lang: &str, word: &str) -> Option<String> {
    let l = m.langs.get(lang)?;
    l.clips.get(word).map(|f| format!("{}{}", l.base, f))
}

/// D8 — slow replay is the SAME clip at a pitch-preserving playback rate; no
/// separately stored slow clip. The server's slow render is ~0.6-0.7x of
/// normal, and the on-device voice uses 0.7, so the human clip matches it.
pub const SLOW_RATE: f64 = 0.7;

pub fn playback_rate(variant: &str, rate: f64) -> f64 {
    if variant == "slow" {
        rate * SLOW_RATE
    } else {
        rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Runtime {
        serde_json::from_str(
            r#"{"langs":{"en":{"base":"human-audio/en/","clips":{"apple":"aa11.m4a"}}}}"#,
        )
        .unwrap()
    }

    #[test]
    fn lookup_finds_only_listed_words_in_their_language() {
        let m = manifest();
        assert_eq!(lookup(&m, "en", "apple").as_deref(), Some("human-audio/en/aa11.m4a"));
        assert_eq!(lookup(&m, "en", "banana"), None);
        assert_eq!(lookup(&m, "fr", "apple"), None, "a clip never crosses languages");
    }

    #[test]
    #[cfg(not(feature = "testseam"))]
    fn committed_manifest_parses() {
        // A malformed manifest would silently disable every clip.
        let _: Runtime = serde_json::from_str(RUNTIME).expect("runtime.json must parse");
    }

    /// AUG6 settings-truth effect test for F6: with the switch off, no word
    /// resolves to the human provider, even one that has a verified clip.
    #[test]
    fn settings_effect_real_voices() {
        let m = manifest();
        set_enabled(true);
        assert!(resolve(&m, "en", "apple").is_some(), "switch on: the verified clip plays");
        set_enabled(false);
        assert!(resolve(&m, "en", "apple").is_none(), "switch off must mean no human clip");
        set_enabled(true);
    }

    #[test]
    fn slow_is_the_same_clip_slower() {
        assert_eq!(playback_rate("normal", 0.9), 0.9);
        assert!((playback_rate("slow", 1.0) - 0.7).abs() < 1e-9);
    }
}
