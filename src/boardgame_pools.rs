//! CC-BOARD-GAME v1.1 D19 / D25 / F9 -- the word supply, built from the
//! device's bank and ledger. The engine never calls a bank; this is the screen
//! layer's half of that contract, kept free of the DOM so it can be tested.
//!
//! * `gate` (F9/I8): every tier of the mode's pool must supply the engine's
//!   minimum unique words in this language, queried from `words::tier_for`
//!   after the Spell Jr filter. The ledger does not lower the count: a held
//!   word is demoted behind fresh ones and relaxed back in only if needed
//!   (the ledger's own F-X3 rule), so it can reduce variety but never block a
//!   game.
//! * `supply` (D19): per tier, a candidate list of exactly the gate's size,
//!   fresh words first, drawn through `Ledger::select` under key
//!   `boardgame:{lang}:{tier}`.
//! * `long_band` (D25): the longest decile of the Expert pool by graphemes,
//!   floor of 20.
//! * `record` (D19): the words a game actually drew, written once.

use std::collections::BTreeSet;

use crate::boardgame::{BoardGameState, Tier, TierPools, Variant, LONG_WORD_FLOOR};
use crate::spelldoku::rng::Rng;
use crate::wordsearch::ledger::Ledger;
use crate::wordsearch::lexicon::graphemes;

/// Spell Jr is the Jr variant (F2) and has no size picker; everyone else plays Sprint or
/// Full (D-P4).
pub fn variant_for(kid: bool, sprint: bool) -> Variant {
    if kid {
        Variant::Jr
    } else if sprint {
        Variant::Sprint
    } else {
        Variant::Full
    }
}

/// The tiers a variant draws tile words from, lowest first.
pub fn tiers_of(v: Variant) -> Vec<Tier> {
    v.cfg().tiers().collect()
}

pub fn min_pool(v: Variant) -> usize {
    v.cfg().min_pool
}

pub fn tiles(v: Variant) -> usize {
    v.cfg().tiles
}

/// D19: the ledger key.
pub fn ledger_key(lang: &str, tier: Tier) -> String {
    format!("boardgame:{lang}:{}", tier.name())
}

/// The bank's unique entries for a tier, in bank order, with the Spell Jr
/// filter applied. Entries (not citations) so zh keeps its hanzi.
pub fn bank(lang: &str, tier: Tier, kid: bool) -> Vec<String> {
    raw_bank(lang, tier, kid)
}

fn raw_bank(lang: &str, tier: Tier, kid: bool) -> Vec<String> {
    let mut seen = BTreeSet::new();
    crate::words::tier_for(lang, tier.name())
        .iter()
        .filter(|w| !kid || crate::kid_filter::kid_allowed(lang, w))
        .filter(|w| seen.insert(**w))
        .map(|w| w.to_string())
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Short {
    pub tier: Tier,
    pub have: usize,
    pub need: usize,
}

/// F9 / I8 / A8. `Err` means "coming soon" for this language and audience.
pub fn gate(lang: &str, v: Variant) -> Result<(), Short> {
    let kid = v == Variant::Jr;
    for tier in tiers_of(v) {
        let have = bank(lang, tier, kid).len();
        if have < min_pool(v) {
            return Err(Short { tier, have, need: min_pool(v) });
        }
    }
    if v.cfg().has_traps() {
        // The Long Word band is a slice of Expert, so it exists whenever Expert
        // does; the floor is what could still fail.
        let have = long_band(&bank(lang, Tier::Expert, kid)).len();
        if have < LONG_WORD_FLOOR {
            return Err(Short { tier: Tier::Expert, have, need: LONG_WORD_FLOOR });
        }
    }
    Ok(())
}

/// D25: the longest decile of `expert` by graphemes (never `chars().count()`),
/// at least `LONG_WORD_FLOOR` words (or all of them if there are fewer). Zh
/// entries are measured on the typed pinyin. Ties break by the word itself so
/// the slice is the same on every device.
pub fn long_band(expert: &[String]) -> Vec<String> {
    let mut v: Vec<(usize, &String)> = expert.iter().map(|w| (graphemes(crate::boardgame_grade::citation(w)).len(), w)).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let take = (v.len().div_ceil(10)).max(LONG_WORD_FLOOR).min(v.len());
    v.into_iter().take(take).map(|(_, w)| w.clone()).collect()
}

/// What `supply` returns besides the pools: how many held words had to be
/// relaxed back in (kept on the device, like the ledger's own counter).
pub struct Supply {
    pub pools: TierPools,
    pub relaxed: usize,
}

/// D19: build the engine's `TierPools` for one new game. `day` is the ledger's
/// day number; `seed` only orders the fresh words, the engine reshuffles from
/// its own seed.
pub fn supply(lang: &str, v: Variant, ledger: &Ledger, day: u32, seed: u64) -> Result<Supply, Short> {
    gate(lang, v)?;
    let kid = v == Variant::Jr;
    let need = min_pool(v);
    let mut rng = Rng::new(seed ^ 0xB0A4_D5B1);
    let mut pools = TierPools::default();
    let mut relaxed = 0;
    for tier in tiers_of(v) {
        let all = bank(lang, tier, kid);
        let mut picked: Vec<String> = Vec::new();
        relaxed += ledger.select(&ledger_key(lang, tier), &all, &mut picked, need, day, &mut rng, &|_, _| true);
        pools.tiers[tier.ix()] = picked;
    }
    if v.cfg().has_traps() {
        let band = long_band(&bank(lang, Tier::Expert, kid));
        let key = ledger_key(lang, Tier::Expert);
        let fresh: Vec<String> = band.iter().filter(|w| !ledger.holds(&key, w, day)).cloned().collect();
        // Prefer words not served lately, but never shrink the band below its floor.
        pools.long_word = if fresh.len() >= LONG_WORD_FLOOR { fresh } else { band };
    }
    Ok(Supply { pools, relaxed })
}

/// D19: write the game's drawn words to the ledger, once, for a finished or an
/// abandoned game alike. Only words the engine actually drew are recorded.
pub fn record(ledger: &mut Ledger, lang: &str, s: &BoardGameState, day: u32, relaxed: usize) {
    if s.drawn.is_empty() {
        return;
    }
    let entries: Vec<(String, String)> = s.drawn.iter().map(|(t, w)| (ledger_key(lang, *t), w.clone())).collect();
    ledger.record_many(&entries, day, relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boardgame::{apply, new_game, Action, Difficulty, GameConfig, Phase};
    use crate::consts::BUILTIN_LANGS;

    /// A8 -- the real `words::tier_for` query, for every language the app
    /// serves, for both audiences. Either the counts are met or the language
    /// is "coming soon"; and today every one is met.
    #[test]
    fn a8_every_builtin_language_either_meets_the_counts_or_is_coming_soon() {
        for lang in BUILTIN_LANGS.iter().map(|l| l.0) {
            for v in Variant::ALL {
                let kid = v == Variant::Jr;
                let verdict = gate(lang, v);
                let counts: Vec<usize> = tiers_of(v).iter().map(|&t| bank(lang, t, kid).len()).collect();
                let meets = counts.iter().all(|&c| c >= min_pool(v))
                    && (!v.cfg().has_traps() || long_band(&bank(lang, Tier::Expert, kid)).len() >= LONG_WORD_FLOOR);
                assert_eq!(verdict.is_ok(), meets, "{lang} {v:?}: gate disagrees with the counts {counts:?}");
                if let Err(s) = verdict {
                    assert!(s.have < s.need);
                }
                eprintln!("A8 {lang} {v:?}: {counts:?} -> {}", if meets { "playable" } else { "coming soon" });
            }
        }
    }

    #[test]
    fn a8_on_current_banks_every_language_passes() {
        for lang in BUILTIN_LANGS.iter().map(|l| l.0) {
            for v in Variant::ALL {
                assert!(gate(lang, v).is_ok(), "{lang} {v:?}: {:?}", gate(lang, v));
            }
        }
    }

    #[test]
    fn d25_long_band_is_the_longest_decile_with_a_floor_of_twenty() {
        let words: Vec<String> = (1..=300).map(|i| "a".repeat(3 + i % 11) + &format!("{i}")).collect();
        let band = long_band(&words);
        assert_eq!(band.len(), 30);
        let min_len = band.iter().map(|w| graphemes(w).len()).min().unwrap();
        let outside_max = words.iter().filter(|w| !band.contains(w)).map(|w| graphemes(w).len()).max().unwrap();
        assert!(min_len >= outside_max, "the band is not the longest slice");
        // Floor: a small pool still yields 20 (or all of it if smaller).
        assert_eq!(long_band(&words[..100]).len(), 20);
        assert_eq!(long_band(&words[..12]).len(), 12);
        // Graphemes, not chars: a combining mark does not lengthen a word.
        let combining = vec!["e\u{301}e\u{301}e\u{301}".to_string(), "abcd".to_string()];
        assert_eq!(graphemes(&combining[0]).len(), 3);
    }

    #[test]
    fn d25_the_real_long_band_is_long_for_english() {
        let ex = bank("en", Tier::Expert, false);
        let band = long_band(&ex);
        assert!(band.len() >= LONG_WORD_FLOOR && band.len() <= ex.len());
        let mean = |v: &[String]| v.iter().map(|w| graphemes(w).len()).sum::<usize>() as f64 / v.len() as f64;
        assert!(mean(&band) > mean(&ex));
    }

    fn solo_game(lang: &str, kid: bool, led: &Ledger, day: u32, seed: u64) -> (BoardGameState, usize) {
        let v = variant_for(kid, false);
        let sup = supply(lang, v, led, day, seed).unwrap();
        let cfg = GameConfig::solo(v, Difficulty::Normal, 3, 0, lang, kid, crate::boardgame_grade::grade);
        (new_game(seed, cfg, sup.pools).unwrap(), sup.relaxed)
    }

    /// Play `turns` human turns, answering right, and return the state.
    fn play_some(mut s: BoardGameState, max_landings: usize) -> BoardGameState {
        let mut landings = 0;
        while s.phase != Phase::Finished && landings < max_landings {
            if s.is_npc_turn() {
                apply(&mut s, Action::AdvanceNpc).unwrap();
                continue;
            }
            match s.phase {
                Phase::AwaitRoll => apply(&mut s, Action::Roll).unwrap(),
                Phase::AwaitSpelling => {
                    landings += 1;
                    let w = crate::boardgame_grade::citation(&s.pending.as_ref().unwrap().word).to_string();
                    apply(&mut s, Action::SubmitSpelling(w)).unwrap()
                }
                _ => apply(&mut s, Action::ChooseSwitchTarget(None)).unwrap(),
            }
        }
        s
    }

    /// A14 -- consecutive games in one language share nothing while fresh
    /// words last; an abandoned game records only what it drew.
    #[test]
    fn a14_consecutive_games_do_not_share_words_and_abandoned_games_record_only_drawn() {
        let mut led = Ledger::default();
        let (mut prev, mut shared_total): (BTreeSet<String>, usize) = (BTreeSet::new(), 0);
        for g in 0..6u64 {
            let (s, relaxed) = solo_game("en", false, &led, 100, 40 + g);
            let s = play_some(s, 12); // abandoned part-way: a dozen landings
            assert_eq!(s.recycled, 0);
            let drawn: BTreeSet<String> = s.drawn.iter().map(|d| d.1.clone()).collect();
            assert!(!drawn.is_empty() && drawn.len() == s.drawn.len());
            let shared = drawn.intersection(&prev).count();
            assert!(shared <= relaxed, "game {g}: {shared} shared words but only {relaxed} relaxed");
            shared_total += shared;
            let before = led.seen.len();
            record(&mut led, "en", &s, 100, relaxed);
            assert_eq!(led.seen.len() - before, drawn.len(), "the ledger took more than the drawn words");
            for (t, w) in &s.drawn {
                assert!(led.seen.iter().any(|x| x.k == ledger_key("en", *t) && &x.w == w));
            }
            prev = drawn;
        }
        assert_eq!(shared_total, 0, "English has thousands of words; nothing should repeat");
    }

    #[test]
    fn a14_a_game_nobody_played_records_nothing() {
        let (s, _) = solo_game("en", false, &Ledger::default(), 1, 9);
        let mut led = Ledger::default();
        record(&mut led, "en", &s, 1, 0);
        assert!(led.seen.is_empty() && led.counter == 0);
    }

    #[test]
    fn a14_a_dry_pool_relaxes_instead_of_blocking() {
        // Hold every Jr kid word; supply must still hand back a full list.
        let mut led = Ledger::default();
        let mut entries = Vec::new();
        for t in [Tier::Easy, Tier::Medium] {
            for w in bank("en", t, true) {
                entries.push((ledger_key("en", t), w));
            }
        }
        led.record_many(&entries, 50, 0);
        let sup = supply("en", Variant::Jr, &led, 50, 3).unwrap();
        assert!(sup.relaxed > 0);
        assert!(sup.pools.tiers[Tier::Easy.ix()].len() >= min_pool(Variant::Jr));
    }

    #[test]
    fn jr_pools_are_kid_safe_and_easy_medium_only() {
        let sup = supply("en", Variant::Jr, &Ledger::default(), 1, 1).unwrap();
        assert!(sup.pools.tiers[Tier::Hard.ix()].is_empty() && sup.pools.tiers[Tier::Expert.ix()].is_empty());
        for t in [Tier::Easy, Tier::Medium] {
            for w in &sup.pools.tiers[t.ix()] {
                assert!(crate::kid_filter::kid_allowed("en", w), "{w} is not kid-safe");
            }
        }
        assert!(sup.pools.long_word.is_empty());
    }
}
