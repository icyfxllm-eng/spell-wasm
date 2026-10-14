//! CC-RANDOSPELL F6 -- Daily Randospell. The dice and the words are a pure
//! function of (date, language, tier), so every player on a day sees the same
//! game and the only variable is spelling (I3). Marathon rules, the Ward
//! available (D16), no cross-session history (it would make two players'
//! words differ). The words never depend on how a player answered: the word
//! stream advances once per press, and a press happens at the same point in
//! the dice sequence for everyone.

use super::bank::{Bank, INELIGIBLE};
use super::{Mode, Session};
use crate::spelldoku::rng::fnv;

/// The key a Daily attempt is stored under, and the seed's input.
pub fn key(date: &str, lang: &str, tier: usize) -> String {
    format!("{date}|{lang}|{tier}")
}

/// I6: no seed exists for a language the mode is off for.
pub fn daily_seed(date: &str, lang: &str, tier: usize, bank: &Bank) -> Option<u64> {
    if INELIGIBLE.contains(&lang) || !bank.cov.viable() {
        return None;
    }
    Some(fnv(format!("randospell-daily|{}", key(date, lang, tier.min(3))).as_bytes()))
}

impl Session {
    /// The Daily game for (date, lang, tier), or None when the language is off.
    pub fn new_daily(date: &str, lang: &str, tier: usize, bank: Bank) -> Option<Session> {
        let seed = daily_seed(date, lang, tier, &bank)?;
        Some(Session::new(seed, Mode::Marathon, tier, bank, Default::default()))
    }
}
