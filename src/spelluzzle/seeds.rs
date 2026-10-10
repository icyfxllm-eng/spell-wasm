//! Verified seeds for the tiers that have silent words (Medium, Hard, Expert).
//!
//! WHY. G8 and G9 prove a board has exactly one answer by checking it against a word
//! list larger than the bank. That list is a build-time input and never ships
//! (data/LICENSES.md, Eric 2026-10-09). So the proof is done offline: a build step
//! generates boards against the bank, keeps the seeds whose board still has one answer
//! under the large list, and ships only those seed numbers. The device regenerates a
//! board from a verified seed with the bank alone, and gets the same board (I7).
//!
//! A seed list is trusted only while the device's bank fingerprints the same as the one
//! it was built from. Otherwise the tier is simply absent (I11), never served unverified.

use serde::Deserialize;

use super::lex::Lexicon;
use super::types::{Tier, GEN_VERSION};

const FILE: &str = include_str!("../../assets/spelluzzle/seeds-en.json");

#[derive(Deserialize)]
pub struct SeedFile {
    pub gen_version: u32,
    pub lang: String,
    pub fingerprint: String,
    #[serde(default)]
    pub validity_source: String,
    #[serde(default)]
    pub validity_words: usize,
    pub tiers: std::collections::BTreeMap<String, Vec<u32>>,
    /// F15: verified Par boards per tier, each `[seed, par, w0..w6]` with the words as pool indexes.
    #[serde(default)]
    pub par: std::collections::BTreeMap<String, Vec<Vec<u32>>>,
}

pub fn file() -> Option<SeedFile> {
    serde_json::from_str(FILE).ok()
}

pub fn needs_seeds(tier: Tier) -> bool {
    !matches!(tier, Tier::Jr | Tier::Easy)
}

pub fn fingerprint_hex(lex: &Lexicon) -> String {
    format!("{:016x}", lex.fingerprint())
}

/// The verified seeds for `tier`, or none if the file is stale for this bank.
pub fn verified(lex: &Lexicon, tier: Tier) -> Vec<u32> {
    let Some(f) = file() else { return Vec::new() };
    if f.lang != lex.lang || f.gen_version != GEN_VERSION || f.fingerprint != fingerprint_hex(lex) {
        return Vec::new();
    }
    f.tiers.get(tier.name()).cloned().unwrap_or_default()
}

/// Is this tier playable? Jr and Easy always are; the others need verified seeds.
pub fn usable(lex: &Lexicon, tier: Tier) -> bool {
    !needs_seeds(tier) || !verified(lex, tier).is_empty()
}

/// The verified Par boards for `tier` (Hard, Expert), or none if the file is stale for this bank.
pub fn par_records(lex: &Lexicon, tier: Tier) -> Vec<super::par::ParRecord> {
    let Some(f) = file() else { return Vec::new() };
    if f.lang != lex.lang || f.gen_version != GEN_VERSION || f.fingerprint != fingerprint_hex(lex) {
        return Vec::new();
    }
    f.par
        .get(tier.name())
        .map(|v| {
            v.iter()
                .filter(|r| r.len() == 2 + super::par::NW)
                .map(|r| {
                    let mut w = [0u16; super::par::NW];
                    for (k, x) in r[2..].iter().enumerate() {
                        w[k] = *x as u16;
                    }
                    super::par::ParRecord { seed: r[0] as u64, par: r[1] as u8, words: w }
                })
                .collect()
        })
        .unwrap_or_default()
}
