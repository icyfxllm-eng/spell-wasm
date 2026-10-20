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
