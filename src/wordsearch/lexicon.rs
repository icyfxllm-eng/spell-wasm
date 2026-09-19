//! Which languages play Spell Search, and the words and letters each one uses.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use super::gen::Tier;

/// The v1 launch set (census rulings, 2026-09-18). ko and zh are excluded
/// whatever any gate says (D4); sw has no blocklist (E7); vi, hi, ja and ar fail
/// E1/E2.
pub const LANGS: [&str; 8] = ["en", "es", "ru", "fr", "de", "pt", "pl", "fil"];

/// E3 (signed): a word counts as audio-served when its language has a server
/// voice. Every launch language has one; a test holds this list to the
/// backend's `LANG_VOICES`, and I6 is enforced here, per language.
pub const SERVER_VOICE: [&str; 8] = LANGS;

pub fn eligible(lang: &str) -> bool {
    LANGS.contains(&lang) && SERVER_VOICE.contains(&lang)
}

/// The tiers a player may be offered. Under-13 players only ever get Jr (F-X5).
/// A language without decoys plays Easy only (D11).
pub fn tiers_for(lang: &str, kid: bool) -> Vec<Tier> {
    if !eligible(lang) {
        return Vec::new();
    }
    if kid {
        return vec![Tier::Jr];
    }
    if super::confusion::has_decoys(lang) {
        vec![Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert]
    } else {
        vec![Tier::Easy]
    }
}

/// NFC, lowercase, split into grapheme clusters: the unit a cell holds.
pub fn graphemes(word: &str) -> Vec<String> {
    let w: String = word.to_lowercase().nfc().collect();
    w.graphemes(true).map(str::to_string).collect()
}

pub fn fold(word: &str) -> String {
    graphemes(word).concat()
}

/// A word a grid can hold: letters only, 3 or more cells, not blocklisted.
pub fn playable_word(word: &str) -> bool {
    let g = graphemes(word);
    g.len() >= 3
        && g.iter().all(|c| c.chars().all(|ch| ch.is_alphabetic() || unicode_normalization::char::is_combining_mark(ch)))
        && !crate::profanity::is_blocked(word)
}

fn bank_words(lang: &str, tier: &str) -> impl Iterator<Item = String> {
    crate::words::tier_for(lang, tier).iter().map(|w| w.split('|').next().unwrap_or(w).to_string())
}

pub struct Lexicon {
    pub lang: String,
    /// Every bank word of 4 or more cells, folded. I3 checks filler against it.
    pub words: HashSet<String>,
    /// Filler letter weights: how often each cell value occurs in the bank (F-S5).
    pub freq: Vec<(String, u64)>,
}

impl Lexicon {
    fn build(lang: &str) -> Lexicon {
        let mut words = HashSet::new();
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            for w in bank_words(lang, tier) {
                if !playable_word(&w) {
                    continue;
                }
                let g = graphemes(&w);
                for c in &g {
                    *counts.entry(c.clone()).or_default() += 1;
                }
                if g.len() >= 4 {
                    words.insert(g.concat());
                }
            }
        }
        Lexicon { lang: lang.to_string(), words, freq: counts.into_iter().collect() }
    }

    /// One lexicon per language, built once and kept for the process.
    pub fn get(lang: &str) -> &'static Lexicon {
        static CACHE: OnceLock<Mutex<HashMap<String, &'static Lexicon>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(l) = cache.lock().unwrap().get(lang) {
            return l;
        }
        let built: &'static Lexicon = Box::leak(Box::new(Lexicon::build(lang)));
        cache.lock().unwrap().insert(lang.to_string(), built);
        built
    }

    /// Can every cell of this word be drawn from this language's letters?
    pub fn spells(&self, word: &str) -> bool {
        graphemes(word).iter().all(|c| self.freq.iter().any(|(g, _)| g == c))
    }
}

/// The bank words a tier draws targets from, in bank order. Jr is the Easy
/// bank's short words: the tier CC-ONBOARD-JR resolves an under-13 player to.
pub fn pool(lang: &str, tier: Tier) -> Vec<String> {
    let (min_len, max_len) = (3, tier.max_word());
    let mut seen = HashSet::new();
    bank_words(lang, tier.bank_tier())
        .filter(|w| playable_word(w))
        .map(|w| fold(&w))
        .filter(|w| (min_len..=max_len).contains(&graphemes(w).len()))
        .filter(|w| seen.insert(w.clone()))
        .collect()
}

/// E4 (signed): the pool a tier needs for the no-repeat window to hold.
pub fn pool_floor(tier: Tier) -> usize {
    tier.targets() * super::ledger::WINDOW_PUZZLES as usize * 3 / 2
}
