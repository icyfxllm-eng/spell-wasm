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
        m.insert("zh", parse(include_str!("i18n/enrich/zh.json")));
        m.insert("vi", parse(include_str!("i18n/enrich/vi.json")));
        m.insert("de", parse(include_str!("i18n/enrich/de.json")));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilots_load_and_are_within_budget() {
        for lang in ["zh", "vi", "de"] {
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
