//! CC-IMPOSSIBLE-LAUNCH: English spelling varieties that are BOTH right (British / American).
//!
//! Eric, 2026-10-19: both spellings are accepted, for every player, whatever their region. The list lives in
//! `assets/words/en/spelling-variants.tsv` (one British/American pair per line). This is a grading list and
//! nothing else: it never makes a sound-alike set, a cue or a bonus pair, and the two spellings need not sound
//! alike (learnt / learned). English only; no other language has such a list.
//!
//! [`accepts`] answers one question: is `typed` the other spelling of `word`? A spelling that is the same after
//! the usual fold is not "another spelling", so it is left to the normal comparison.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::norm::fold_strict;

const PAIRS: &str = include_str!("../assets/words/en/spelling-variants.tsv");

/// folded spelling -> the folded spellings that are its other form(s).
fn map() -> &'static HashMap<String, Vec<String>> {
    static MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for line in PAIRS.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            let mut it = line.split('\t');
            let (Some(a), Some(b)) = (it.next(), it.next()) else { continue };
            let (a, b) = (fold_strict(a), fold_strict(b));
            if a != b {
                m.entry(a.clone()).or_default().push(b.clone());
                m.entry(b).or_default().push(a);
            }
        }
        m
    })
}

/// Is `typed` the other national spelling of the English prompt `word`?
pub fn accepts(word: &str, typed: &str) -> bool {
    let (w, t) = (fold_strict(word), fold_strict(typed));
    w != t && map().get(&w).is_some_and(|others| others.contains(&t))
}


// ---- Display: which form a player is SHOWN. Grading above never depends on this. ----

/// Regions whose own spelling is the British form. Canada is left out on purpose: its usage is mixed, and the
/// setting below is there for anyone the guess gets wrong.
const BRITISH_REGIONS: &[&str] = &["GB", "UK", "IE", "AU", "NZ", "ZA", "IN", "SG", "HK", "MY", "PK", "KE", "NG", "GH", "MT", "CY"];

/// Does the first English tag in the device's language list name a British-spelling region? `tags` are BCP-47
/// tags as the browser gives them ("en-GB", "fr-FR"). A bare "en" says nothing, so it is American (the default).
pub fn british_region(tags: &[String]) -> bool {
    tags.iter()
        .find(|t| t.to_ascii_lowercase().starts_with("en"))
        .and_then(|t| t.split(['-', '_']).skip(1).find(|p| p.len() == 2))
        .is_some_and(|r| BRITISH_REGIONS.contains(&r.to_ascii_uppercase().as_str()))
}

#[cfg(target_arch = "wasm32")]
fn device_tags() -> Vec<String> {
    let Some(nav) = web_sys::window().map(|w| w.navigator()) else { return Vec::new() };
    nav.languages().iter().filter_map(|v| v.as_string()).collect()
}
#[cfg(not(target_arch = "wasm32"))]
fn device_tags() -> Vec<String> {
    Vec::new()
}

const PREF_KEY: &str = "spell_british";

/// The player's choice if they have made one (the Settings switch), else what the device region suggests.
pub fn prefers_british() -> bool {
    match crate::storage::get_raw(PREF_KEY).as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => british_region(&device_tags()),
    }
}

pub fn set_prefers_british(on: bool) {
    crate::storage::set_raw(PREF_KEY, if on { "1" } else { "0" });
}

/// `word` in the form the player prefers: the British or the American member of its pair, or `word` unchanged
/// when it is in no pair. A leading capital is kept.
pub fn display(word: &str, british: bool) -> String {
    let w = fold_strict(word);
    let mut hit: Option<&str> = None;
    for line in PAIRS.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let mut it = line.split('\t');
        let (Some(gb), Some(us)) = (it.next(), it.next()) else { continue };
        let (gb, us) = (gb.trim(), us.trim());
        if fold_strict(gb) == w || fold_strict(us) == w {
            hit = Some(if british { gb } else { us });
            break;
        }
    }
    match hit {
        Some(form) if word.chars().next().is_some_and(char::is_uppercase) => {
            let mut c = form.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        }
        Some(form) => form.to_string(),
        None => word.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_spelling_is_accepted_for_the_other() {
        for (a, b) in [("colour", "color"), ("centre", "center"), ("organise", "organize"), ("travelled", "traveled"), ("grey", "gray"), ("learnt", "learned"), ("judgement", "judgment"), ("anaemia", "anemia")] {
            assert!(accepts(a, b) && accepts(b, a), "{a} / {b}");
            assert!(accepts(&a.to_uppercase(), b), "case is folded");
        }
    }


    #[test]
    fn display_follows_the_player_and_never_changes_other_words() {
        assert_eq!(display("color", true), "colour");
        assert_eq!(display("colour", false), "color");
        assert_eq!(display("colour", true), "colour");
        assert_eq!(display("Color", true), "Colour");
        assert_eq!(display("learned", true), "learnt");
        assert_eq!(display("cat", true), "cat");
        assert_eq!(display("cat", false), "cat");
    }

    #[test]
    fn region_guess_reads_the_first_english_tag() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(british_region(&v(&["en-GB"])));
        assert!(british_region(&v(&["fr-FR", "en-AU", "en-US"])), "first English tag decides");
        assert!(british_region(&v(&["en_ie"])));
        assert!(!british_region(&v(&["en-US"])));
        assert!(!british_region(&v(&["en"])), "no region -> American default");
        assert!(!british_region(&v(&["en-CA"])), "Canada is mixed -> default");
        assert!(!british_region(&v(&["de-DE"])), "no English tag");
        assert!(!british_region(&[]));
    }

    #[test]
    fn it_is_not_a_general_leniency() {
        assert!(!accepts("colour", "colr"));
        assert!(!accepts("color", "color"), "the same spelling is the normal comparison's job");
        assert!(!accepts("whilst", "while"), "a different word, not a spelling");
        assert!(!accepts("amongst", "among"));
        assert!(!accepts("cat", "dog"));
    }

    #[test]
    fn the_list_is_well_formed() {
        let mut seen = std::collections::HashSet::new();
        let mut n = 0;
        for line in PAIRS.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            assert_eq!(f.len(), 2, "not a pair: {line:?}");
            assert!(f.iter().all(|w| w.chars().all(|c| c.is_ascii_lowercase() || c == '-')), "plain lowercase only: {line:?}");
            assert_ne!(f[0], f[1], "{line}");
            let mut k = [f[0], f[1]];
            k.sort();
            assert!(seen.insert((k[0], k[1])), "duplicate pair {line:?}");
            n += 1;
        }
        assert!(n > 150, "{n} pairs");
    }
}
