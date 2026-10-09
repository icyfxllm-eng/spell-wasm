//! The word data a board is built and checked against.
//!
//! Built once from plain strings so the core never reaches for the bank, the
//! audio verdicts or a file: whoever loads a language does its G1 filtering
//! (profanity, quarantine, withheld audio, Spell Jr resolution) and hands the
//! survivors in. Everything iterates in sorted order, so a board never depends
//! on hash-map ordering (I7).

use std::collections::{BTreeMap, HashMap, HashSet};

use super::types::Tier;

pub type Units = Vec<char>;

/// A word with the set of units it uses, as bits over the language alphabet.
#[derive(Clone, Debug)]
pub struct Word {
    pub units: Units,
    pub mask: u64,
}

pub struct LexInput {
    pub lang: String,
    /// Per tier, the words G1 lets onto a board: in the tier's bank band and
    /// already filtered. Order does not matter.
    pub eligible: Vec<(Tier, Vec<String>)>,
    /// The validity list for G8 and G9: every string that counts as a word.
    /// The eligible words and collision-set members are added automatically.
    pub validity: Vec<String>,
    /// Collision sets (CC-SENSE-CUE): words that sound the same.
    pub collisions: Vec<Vec<String>>,
}

pub struct Lexicon {
    pub lang: String,
    pub alphabet: Vec<char>,
    pool: BTreeMap<Tier, Vec<Word>>,
    band: BTreeMap<Tier, HashSet<Units>>,
    groups: HashMap<Units, Vec<Units>>,
    validity: Vec<Units>,
    by_pattern: HashMap<Vec<u8>, Vec<u32>>,
    validity_set: HashSet<Units>,
}

/// A word's isomorphism pattern: each distinct unit numbered by first appearance.
pub fn pattern(w: &[char]) -> Vec<u8> {
    let mut seen: Vec<char> = Vec::new();
    w.iter()
        .map(|c| match seen.iter().position(|s| s == c) {
            Some(i) => i as u8,
            None => {
                seen.push(*c);
                (seen.len() - 1) as u8
            }
        })
        .collect()
}

fn units(s: &str) -> Units {
    s.chars().flat_map(|c| c.to_lowercase()).collect()
}

fn is_word(u: &[char]) -> bool {
    !u.is_empty() && u.iter().all(|c| c.is_alphabetic())
}

impl Lexicon {
    pub fn new(input: LexInput) -> Result<Lexicon, String> {
        let mut band: BTreeMap<Tier, HashSet<Units>> = BTreeMap::new();
        let mut all: HashSet<Units> = HashSet::new();
        let mut elig: BTreeMap<Tier, Vec<Units>> = BTreeMap::new();
        for (t, ws) in &input.eligible {
            let e = elig.entry(*t).or_default();
            for w in ws {
                let u = units(w);
                if is_word(&u) {
                    band.entry(*t).or_default().insert(u.clone());
                    all.insert(u.clone());
                    e.push(u);
                }
            }
        }
        let mut groups: HashMap<Units, Vec<Units>> = HashMap::new();
        for set in &input.collisions {
            let members: Vec<Units> = set.iter().map(|s| units(s)).filter(|u| is_word(u)).collect();
            for m in &members {
                let g = groups.entry(m.clone()).or_default();
                g.extend(members.iter().cloned());
                all.insert(m.clone());
            }
        }
        for g in groups.values_mut() {
            g.sort();
            g.dedup();
        }
        for w in &input.validity {
            let u = units(w);
            if is_word(&u) {
                all.insert(u);
            }
        }
        let mut alphabet: Vec<char> = all.iter().flatten().copied().collect();
        alphabet.sort_unstable();
        alphabet.dedup();
        if alphabet.len() > 64 {
            return Err(format!("alphabet of {} units does not fit a 64-bit mask", alphabet.len()));
        }
        let mask_of = |u: &[char]| u.iter().fold(0u64, |m, c| m | 1u64 << alphabet.binary_search(c).expect("in alphabet"));
        let mut pool = BTreeMap::new();
        for (t, mut ws) in elig {
            ws.sort();
            ws.dedup();
            pool.insert(t, ws.into_iter().map(|u| Word { mask: mask_of(&u), units: u }).collect::<Vec<Word>>());
        }
        let mut validity: Vec<Units> = all.into_iter().collect();
        validity.sort();
        let mut by_pattern: HashMap<Vec<u8>, Vec<u32>> = HashMap::new();
        for (i, w) in validity.iter().enumerate() {
            by_pattern.entry(pattern(w)).or_default().push(i as u32);
        }
        let validity_set = validity.iter().cloned().collect();
        Ok(Lexicon { lang: input.lang, alphabet, pool, band, groups, validity, by_pattern, validity_set })
    }

    /// The words a board of `tier` may draw from, sorted, within the tier's length range.
    pub fn pool(&self, tier: Tier) -> Vec<&Word> {
        let s = tier.shape();
        self.pool.get(&tier).map(|p| p.iter().filter(|w| (s.lo..=s.hi).contains(&w.units.len())).collect()).unwrap_or_default()
    }

    pub fn in_band(&self, tier: Tier, w: &[char]) -> bool {
        self.band.get(&tier).is_some_and(|b| b.contains(w))
    }

    /// Everything that sounds like `w`, including `w`; empty if it is in no set.
    pub fn group(&self, w: &[char]) -> &[Units] {
        self.groups.get(w).map(|g| g.as_slice()).unwrap_or(&[])
    }

    pub fn has_homophone(&self, w: &[char]) -> bool {
        self.group(w).len() > 1
    }

    pub fn is_valid(&self, w: &[char]) -> bool {
        self.validity_set.contains(w)
    }

    /// Validity-list words with this isomorphism pattern.
    pub fn valid_with_pattern(&self, pat: &[u8]) -> impl Iterator<Item = &Units> {
        self.by_pattern.get(pat).into_iter().flatten().map(move |&i| &self.validity[i as usize])
    }

    /// A fingerprint of everything a board depends on: the eligible words per tier, the
    /// collision sets and the size of the validity list. A verified-seed file records
    /// it, and is used only when the device's own bank gives the same one.
    pub fn fingerprint(&self) -> u64 {
        let mut s = String::new();
        for t in super::types::Tier::ALL {
            s.push_str(t.name());
            for w in self.pool.get(&t).into_iter().flatten() {
                s.push(' ');
                s.extend(w.units.iter());
            }
            s.push('\n');
        }
        let mut groups: Vec<String> = self.groups.iter().map(|(k, g)| format!("{}:{}", k.iter().collect::<String>(), g.iter().map(|w| w.iter().collect::<String>()).collect::<Vec<_>>().join("+"))).collect();
        groups.sort();
        s.push_str(&groups.join(","));
        s.push_str(&format!("|{}", self.validity.len()));
        crate::spelldoku::rng::fnv(s.as_bytes())
    }

    pub fn validity_len(&self) -> usize {
        self.validity.len()
    }

    pub fn all_valid(&self) -> &[Units] {
        &self.validity
    }
}
