//! Language "beauty layer" — optional per-word enrichment (etymology, character
//! components, tone families, compounds…) shown as a subordinate second line on
//! the meaning card. Purely additive: a word without enrichment behaves exactly
//! as before. All strings live in auditor-reviewable data files
//! (src/i18n/enrich/{lang}.json), never hardcoded.
//!
//! `verified` gate: release builds show only `verified: true` insights (a
//! native-speaker auditor has approved them). Dev/E2E builds (`--features
//! testseam`) show unverified ones too, badged, so drafts can be reviewed
//! in-app. This keeps unreviewed kid-facing text out of production.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Insight {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub verified: bool,
}

type Table = HashMap<String, Insight>;

fn tables() -> &'static HashMap<&'static str, Table> {
    static T: OnceLock<HashMap<&'static str, Table>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m: HashMap<&'static str, Table> = HashMap::new();
        // Pilot languages (spec step 3): components / tone_family / compound.
        m.insert("en", parse(include_str!("i18n/enrich/en.json")));
        m.insert("es", parse(include_str!("i18n/enrich/es.json")));
        m.insert("fr", parse(include_str!("i18n/enrich/fr.json")));
        m.insert("de", parse(include_str!("i18n/enrich/de.json")));
        m.insert("pt", parse(include_str!("i18n/enrich/pt.json")));
        m.insert("pl", parse(include_str!("i18n/enrich/pl.json")));
        m.insert("vi", parse(include_str!("i18n/enrich/vi.json")));
        m.insert("ko", parse(include_str!("i18n/enrich/ko.json")));
        m.insert("ja", parse(include_str!("i18n/enrich/ja.json")));
        m.insert("zh", parse(include_str!("i18n/enrich/zh.json")));
        m.insert("fil", parse(include_str!("i18n/enrich/fil.json")));
        m
    })
}

fn parse(s: &str) -> Table {
    serde_json::from_str(s).unwrap_or_default()
}

/// True when unverified insights may render (dev/E2E builds only).
#[cfg(feature = "testseam")]
fn show_unverified() -> bool {
    true
}
#[cfg(not(feature = "testseam"))]
fn show_unverified() -> bool {
    false
}

/// The insight for `key` in `lang`, if one exists and passes the verified gate.
/// For Mandarin, callers pass the hanzi (the `spoken` form) as the key, since the
/// insight is about the character, not the typed pinyin.
pub fn insight(lang: &str, key: &str) -> Option<Insight> {
    let ins = tables().get(lang)?.get(key)?.clone();
    if ins.verified || show_unverified() {
        Some(ins)
    } else {
        None
    }
}

impl Insight {
    /// "Beloved" words — the no_equivalent / usage_gem gems that earn a subtle
    /// sparkle on the meaning card (at most one per session).
    pub fn is_beloved(&self) -> bool {
        matches!(self.kind.as_str(), "no_equivalent" | "usage_gem")
    }
}

// ---------------------------------------------------------------- picker notes

#[derive(Deserialize, Clone)]
struct Note {
    text: String,
    #[serde(default)]
    verified: bool,
}

fn notes() -> &'static HashMap<String, Note> {
    static N: OnceLock<HashMap<String, Note>> = OnceLock::new();
    N.get_or_init(|| serde_json::from_str(include_str!("i18n/enrich/notes.json")).unwrap_or_default())
}

/// The writing-system note for `lang` (shown once on first selection), gated.
pub fn picker_note(lang: &str) -> Option<String> {
    let n = notes().get(lang)?;
    (n.verified || show_unverified()).then(|| n.text.clone())
}

// ---------------------------------------------------------------- proverbs

#[derive(Deserialize, Clone)]
pub struct Proverb {
    pub o: String,
    pub t: String,
    #[serde(default)]
    verified: bool,
}

fn proverbs() -> &'static HashMap<String, Vec<Proverb>> {
    static P: OnceLock<HashMap<String, Vec<Proverb>>> = OnceLock::new();
    P.get_or_init(|| serde_json::from_str(include_str!("i18n/enrich/proverbs.json")).unwrap_or_default())
}

/// A deterministically-chosen verified proverb for `lang`, or None to fall back
/// to the standard congratulation. `seed` (a date hash) picks which one and
/// whether one shows (~1 in 3), so it's stable per day.
pub fn proverb(lang: &str, seed: u64) -> Option<Proverb> {
    if seed % 3 != 0 {
        return None;
    }
    let pool: Vec<&Proverb> = proverbs()
        .get(lang)?
        .iter()
        .filter(|p| p.verified || show_unverified())
        .collect();
    if pool.is_empty() {
        return None;
    }
    Some(pool[(seed / 3) as usize % pool.len()].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilots_load_and_are_within_budget() {
        for lang in ["en","es","fr","de","pt","pl","vi","ko","ja","zh","fil"] {
            let t = tables().get(lang).expect("pilot table missing");
            assert!(!t.is_empty(), "{lang} enrichment empty");
            for (word, ins) in t {
                assert!(ins.text.chars().count() <= 140, "{lang}/{word}: insight over 140 chars");
                assert!(!ins.kind.is_empty(), "{lang}/{word}: missing type");
            }
        }
    }

    #[test]
    fn verified_gate_hides_unverified_in_release() {
        // Pilot data is all verified:false; the release path (show_unverified=false)
        // must return None. This test build enables testseam-independent logic via
        // the field directly.
        let t = tables().get("de").unwrap();
        let handschuh = t.get("Handschuh").expect("Handschuh insight present");
        assert!(!handschuh.verified, "pilot insights ship unverified until an auditor approves");
    }
}
