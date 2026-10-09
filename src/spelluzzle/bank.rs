//! Load a language's bank into a `Lexicon`, doing G1's filtering.
//!
//! A word is eligible when it is in the tier's bank band, is not blocked, has an
//! audio clip the app may serve (not withheld, CC-AUDIO-CLARITY I1), and, for
//! Spell Jr, passes the kid-safety list. Quarantined rows are already out of the
//! bank files. The validity list is the bank itself plus whatever the caller
//! adds: nothing larger ships to the device yet (census C6).

use super::lex::{LexInput, Lexicon};
use super::types::Tier;

fn bare(row: &str) -> &str {
    row.split('|').next().unwrap_or(row)
}

pub fn load(lang: &str, extra_validity: Vec<String>) -> Result<Lexicon, String> {
    let mut eligible = Vec::new();
    let mut all_bank: Vec<String> = Vec::new();
    let mut collisions: Vec<Vec<String>> = Vec::new();
    for tier in Tier::ALL {
        let mut words: Vec<String> = Vec::new();
        for band in tier.bands() {
            for row in crate::words::tier_for(lang, band) {
                let w = bare(row).to_lowercase();
                if crate::profanity::is_blocked(&w) || !crate::audio_verdict::servable(lang, &w, "normal") {
                    continue;
                }
                if tier == Tier::Jr && !crate::kid_filter::kid_allowed(lang, &w) {
                    continue;
                }
                words.push(w);
            }
        }
        eligible.push((tier, words));
    }
    for band in crate::experience::TIERS {
        for row in crate::words::tier_for(lang, band) {
            let w = bare(row).to_lowercase();
            let mut set = crate::homophones::group_members(lang, &w);
            if !set.is_empty() {
                set.push(w.clone());
                set.sort();
                collisions.push(set);
            }
            all_bank.push(w);
        }
    }
    collisions.sort();
    collisions.dedup();
    let mut validity = all_bank;
    validity.extend(extra_validity);
    Lexicon::new(LexInput { lang: lang.to_string(), eligible, validity, collisions })
}
