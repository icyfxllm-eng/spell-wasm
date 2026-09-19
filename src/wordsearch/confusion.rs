//! F-S2 — trap decoys, from the hand-authored confusion list only (E5, D6).
//!
//! A player's own miss history is never read here. The shipped decoys were
//! proven not to be words at build time (tools/wordsearch/build_decoys.py, E6);
//! `edits` re-derives them from the same rules so a test can hold the table to
//! "exactly one confusion edit from its target" (I2).

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use serde::Deserialize;

const EN_RULES: &str = include_str!("../../config/confusions/en.json");
const EN_DECOYS: &str = include_str!("../../assets/wordsearch/en-decoys.txt");

#[derive(Deserialize)]
struct Rules {
    swaps: Vec<serde_json::Value>,
    double: String,
    single: String,
}

/// E6: only a language with a dictionary to prove decoys are not words gets
/// them. Spanish and Russian have none yet, so they play without (D11).
pub fn has_decoys(lang: &str) -> bool {
    lang == "en"
}

fn rules(lang: &str) -> Option<&'static Rules> {
    static EN: OnceLock<Rules> = OnceLock::new();
    (lang == "en").then(|| EN.get_or_init(|| serde_json::from_str(EN_RULES).expect("config/confusions/en.json")))
}

/// Every string exactly one confusion edit from `word` (ASCII English rules).
/// Mirrors `edits` in tools/wordsearch/build_decoys.py. Spell Search ships the
/// decoy table this produced; SpellDoku Word Mode asks it whether two bank
/// words are one edit apart (CC-SPELLDOKU v1.2 R4).
pub fn edits(lang: &str, word: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(r) = rules(lang) else { return out };
    let b = word.as_bytes();
    for rule in &r.swaps {
        let frm = rule[0].as_str().unwrap_or("");
        let to = rule[1].as_str().unwrap_or("");
        let at = rule[2].as_str().unwrap_or("any");
        if let Some(min) = rule.get(3).and_then(|v| v.as_u64()) {
            if (word.len() as u64) < min {
                continue;
            }
        }
        if frm.is_empty() {
            continue;
        }
        let mut start = 0;
        while let Some(off) = word.get(start..).and_then(|s| s.find(frm)) {
            let i = start + off;
            start = i + 1;
            let ok = match at {
                "start" => i == 0,
                "mid" => i != 0,
                "end" => i + frm.len() == word.len(),
                _ => true,
            };
            if ok {
                out.insert(format!("{}{}{}", &word[..i], to, &word[i + frm.len()..]));
            }
        }
    }
    let vowel = |c: u8| b"aeiouy".contains(&c);
    for i in 1..b.len() {
        let c = b[i];
        let doubled = i + 1 < b.len() && b[i + 1] == c;
        if r.single.as_bytes().contains(&c) && doubled {
            out.insert(format!("{}{}", &word[..i], &word[i + 1..]));
        }
        let between = vowel(b[i - 1]) && i + 1 < b.len() && vowel(b[i + 1]);
        if r.double.as_bytes().contains(&c) && between {
            out.insert(format!("{}{}{}", &word[..i], c as char, &word[i..]));
        }
    }
    out.remove(word);
    out
}

fn table(lang: &str) -> Option<&'static HashMap<String, Vec<String>>> {
    static EN: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    (lang == "en").then(|| {
        EN.get_or_init(|| {
            EN_DECOYS
                .lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .filter_map(|l| l.split_once('\t'))
                .map(|(w, ds)| (w.to_string(), ds.split(' ').map(str::to_string).collect()))
                .collect()
        })
    })
}

/// The shipped decoys for one target, re-checked against the blocklist here
/// too, since the build tool can only read the plain lists.
pub fn decoys(lang: &str, word: &str) -> Vec<String> {
    table(lang)
        .and_then(|t| t.get(word))
        .map(|v| v.iter().filter(|d| !crate::profanity::is_blocked(d)).cloned().collect())
        .unwrap_or_default()
}

/// Test view of the whole table.
#[cfg(test)]
pub fn all(lang: &str) -> Vec<(String, Vec<String>)> {
    let mut v: Vec<_> = table(lang).map(|t| t.iter().map(|(k, d)| (k.clone(), d.clone())).collect()).unwrap_or_default();
    v.sort();
    v
}
