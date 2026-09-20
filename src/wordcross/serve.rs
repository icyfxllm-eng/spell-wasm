//! Serving a Spell Cross: choosing the words (F-X1, F-X3), laying them out,
//! holding F-C3, hanging the keystone (F-C6), and the D2 / F-C7 fallback.

use super::layout::{build_best, shade_keystone, Grid, MAX_SIDE};
use super::traps::{distinguishing, trap_positions};
use super::super::spelldoku::rng::Rng;
use super::super::wordsearch::gen::Tier;
use super::super::wordsearch::ledger::{daily_pick, fits, seed, Ledger};
use super::super::wordsearch::lexicon::{daily_pool, eligible, graphemes, pool, Lexicon};
use super::super::wordsearch::serve::list_words;

/// I7 / D2: fewer than five connected words is not a crossword.
pub const MIN_WORDS: usize = 5;
/// How many words a tier tries to interlock (test 7 draws eight).
pub const DRAW: usize = 8;

/// F-C2 by tier, and F-C5's check timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    pub words: usize,
    /// Prefer crossings ON trap positions (Jr, Easy, Medium) or off them.
    pub on_traps: bool,
    /// A wrong letter may flash as it is typed; never at Hard or Expert.
    pub letter_check: bool,
}

pub fn params(tier: Tier) -> Params {
    match tier {
        Tier::Jr => Params { words: 6, on_traps: true, letter_check: true },
        Tier::Easy | Tier::Medium => Params { words: DRAW, on_traps: true, letter_check: true },
        Tier::Hard | Tier::Expert => Params { words: DRAW, on_traps: false, letter_check: false },
    }
}

pub struct Cross {
    pub lang: String,
    pub tier: Tier,
    pub grid: Grid,
    /// Words that could not interlock, or that F-C3 excluded.
    pub dropped: Vec<String>,
    /// Ledger key, `lang:tier`.
    pub key: String,
}

/// F-C3: a clue word in a collision group needs a crossing that tells the
/// spellings apart. Sense cues are the second choice, but every language's cue
/// mode is Off today (CC-SENSE-CUE D9 voids a claim without a named auditor),
/// so the third applies: the word leaves the grid.
fn unanswerable(lang: &str, grid: &Grid) -> Vec<String> {
    if crate::sense_cue::cue_mode(lang) != crate::sense_cue::CueMode::Off {
        return Vec::new();
    }
    let crossings = grid.crossings();
    grid.words
        .iter()
        .enumerate()
        .filter(|(i, p)| match distinguishing(lang, &p.word) {
            None => false,
            Some(pos) => !crossings
                .iter()
                .filter(|(_, a, b)| a == i || b == i)
                .any(|&(cell, _, _)| grid.index_in(*i, cell).is_some_and(|k| pos.contains(&k))),
        })
        .map(|(_, p)| p.word.clone())
        .collect()
}

/// Lay out these words, dropping any the grid cannot answer (F-C3), and hang a
/// keystone from `spare` (F-C6). None when fewer than five words interlock:
/// the caller then offers Spell Search (D2, F-C7).
pub fn lay(lang: &str, tier: Tier, words: &[String], spare: &[String], rng: &mut Rng) -> Option<Cross> {
    let p = params(tier);
    let traps = |w: &str| trap_positions(lang, w);
    let cues_on = crate::sense_cue::cue_mode(lang) != crate::sense_cue::CueMode::Off;
    let tells = |w: &str| if cues_on { None } else { distinguishing(lang, w) };
    let mut input: Vec<String> = words.to_vec();
    for _ in 0..4 {
        let (mut grid, left) = build_best(&input, rng, p.on_traps, &traps, &tells, 64)?;
        let bad = unanswerable(lang, &grid);
        if !bad.is_empty() && input.len() - bad.len() >= MIN_WORDS {
            input.retain(|w| !bad.contains(w));
            continue;
        }
        if grid.words.len() < MIN_WORDS || !bad.is_empty() {
            return None;
        }
        // F-C6: the keystone is a word of its own, spelled from shaded cells.
        for k in spare.iter().filter(|k| !grid.words.iter().any(|p| p.word == **k)) {
            if let Some(cells) = shade_keystone(&grid, k) {
                grid.shaded = cells;
                grid.keystone = Some(k.clone());
                break;
            }
        }
        let mut dropped = left;
        dropped.extend(words.iter().filter(|w| !input.contains(w)).cloned());
        return Some(Cross { lang: lang.to_string(), tier, grid, dropped, key: format!("{lang}:{}", tier.id()) });
    }
    None
}

/// Words a crossword can hold: playable, short enough for the grid, and
/// spelled in this language's own letters.
pub fn usable(lang: &str, words: &[String]) -> Vec<String> {
    let lex = Lexicon::get(lang);
    list_words(lang, words).into_iter().filter(|w| graphemes(w).len() <= MAX_SIDE && lex.spells(w)).collect()
}

/// F-X1: a bank-tier Spell Cross for this player, seeded by the puzzle counter.
pub fn bank(lang: &str, tier: Tier, ledger: &Ledger, day: u32) -> Option<Cross> {
    if !eligible(lang) {
        return None;
    }
    let words = pool(lang, tier);
    let k = format!("{lang}:{}", tier.id());
    let base = seed(&format!("cross:{k}"), ledger.counter, 0);
    let fit = |p: &[String], w: &str| fits(lang, MAX_SIDE, p, w);
    // F-C2: the tiers that want crossings ON trap positions need words that
    // have some. In English Easy only 12% of letter positions are trap
    // positions and most words have none, so a random draw cannot put 60% of
    // crossings there however well it is laid out: the draw has to prefer
    // trap-rich words first, the way Spell Search prefers words with decoys.
    let trappy: Vec<String> = if params(tier).on_traps {
        words.iter().filter(|w| !trap_positions(lang, w).is_empty()).cloned().collect()
    } else {
        Vec::new()
    };
    for round in 0..8u64 {
        let mut rng = Rng::new(base.wrapping_add(round));
        let mut picked = Vec::new();
        if !trappy.is_empty() {
            ledger.select(&k, &trappy, &mut picked, params(tier).words, day, &mut rng, &fit);
        }
        ledger.select(&k, &words, &mut picked, params(tier).words + 4, day, &mut rng, &fit);
        if picked.len() < MIN_WORDS + 1 {
            return None;
        }
        let spare = picked.split_off(params(tier).words.min(picked.len() - 1));
        if let Some(c) = lay(lang, tier, &picked, &spare, &mut rng) {
            return Some(c);
        }
    }
    None
}

/// F-X1 / Phase C: the Daily Spell Cross -- the same crossword for everyone
/// playing this language on this date. It draws from the whole bank, so a word
/// stays away for the 90 days D9 asks (`daily_gap`), and `ymd` seeds the layout.
pub fn daily(lang: &str, tier: Tier, ymd: u32, day: u32) -> Option<Cross> {
    if !eligible(lang) {
        return None;
    }
    let k = format!("{lang}:{}", tier.id());
    let fit = |p: &[String], w: &str| fits(lang, MAX_SIDE, p, w);
    // A word set that will not interlock is redrawn, the same way on every
    // device, so a date always has its crossword.
    // One draw from this date's block, so the 90-day window holds; the retries
    // rearrange those same words rather than reaching into another day's.
    let words = daily_pick(&daily_pool(lang, tier), params(tier).words + 6, day, &format!("cross:{k}"), &fit);
    if words.len() < MIN_WORDS + 1 {
        return None;
    }
    for r in 0..8u64 {
        let mut rng = Rng::new(seed(&format!("cross-daily:{k}"), r, ymd));
        let mut order = words.clone();
        if r > 0 {
            rng.shuffle(&mut order);
        }
        let (picked, spare) = order.split_at(params(tier).words.min(order.len() - 1));
        if let Some(c) = lay(lang, tier, picked, spare, &mut rng) {
            return Some(c);
        }
    }
    None
}

/// F-X1: "Make a puzzle" on a My Words list, as a crossword.
pub fn from_list(lang: &str, tier: Tier, list_id: &str, words: &[String], counter: u64) -> Option<Cross> {
    if !eligible(lang) {
        return None;
    }
    let mut rng = Rng::new(seed(&format!("cross-list:{list_id}"), counter, 0));
    let mut all = usable(lang, words);
    rng.shuffle(&mut all);
    if all.len() < MIN_WORDS {
        return None;
    }
    let cut = params(tier).words.min(all.len());
    let spare = all[cut..].to_vec();
    lay(lang, tier, &all[..cut], &spare, &mut rng)
}
