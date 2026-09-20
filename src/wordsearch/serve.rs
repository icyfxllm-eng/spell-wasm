//! F-X1 — the three sources of a Spell Search puzzle: a bank tier, the Daily
//! Puzzle, and a dated My Words list. Each returns a checked grid, or nothing;
//! none mutates the ledger (the caller records what it actually served).

use super::confusion::{decoys, has_decoys};
use super::gen::{generate, Input, Puzzle, Tier};
use super::ledger::{daily_pick, fits, seed, Ledger};
use super::lexicon::{daily_pool, eligible, fold, graphemes, playable_word, pool, Lexicon};
use crate::spelldoku::rng::Rng;

pub struct Served {
    pub puzzle: Puzzle,
    /// Ledger key, `lang:tier`.
    pub key: String,
    pub relaxed: usize,
}

/// The largest grid; a My Words list's long words grow the grid up to this.
pub const MAX_SIZE: usize = 12;

pub fn key(lang: &str, tier: Tier) -> String {
    format!("{lang}:{}", tier.id())
}

fn input<'a>(lang: &str, tier: Tier, size: usize, targets: Vec<String>, lex: &'a Lexicon) -> Input<'a> {
    let with = has_decoys(lang) && tier.decoy_count(targets.len()) > 0;
    let decoy_lists = targets.iter().map(|t| if with { decoys(lang, t) } else { Vec::new() }).collect();
    Input { tier, size, targets, decoys: decoy_lists, lex }
}

/// A bank-tier puzzle for this player: unseen words first (F-X3), seeded by the
/// player's puzzle counter (F-X2).
pub fn bank(lang: &str, tier: Tier, ledger: &Ledger, day: u32) -> Option<Served> {
    if !eligible(lang) {
        return None; // I6
    }
    let lex = Lexicon::get(lang);
    let words = pool(lang, tier);
    let k = key(lang, tier);
    let size = tier.size();
    let base = seed(&format!("bank:{k}"), ledger.counter, 0);
    let fit = |p: &[String], w: &str| fits(lang, size, p, w);
    // A target set that will not lay out cleanly is swapped for another. If
    // eight sets in a row will not (a crowded Expert grid, well under 1 in 1,000),
    // the puzzle carries one target fewer -- with the tier's full decoys per
    // target -- rather than serving nothing.
    for round in 0..16u64 {
        let n = if round < 8 { tier.targets() } else { tier.targets() - 1 };
        let mut rng = Rng::new(base.wrapping_add(round));
        let mut picked = Vec::new();
        let mut relaxed = 0;
        // Tiers with decoys need targets that have them: pick those first.
        let want = tier.decoy_count(n);
        if has_decoys(lang) && want > 0 {
            let per = tier.decoys_per_target();
            let need = want.div_ceil(per);
            let able: Vec<String> = words.iter().filter(|w| decoys(lang, w).len() >= per).cloned().collect();
            relaxed += ledger.select(&k, &able, &mut picked, need, day, &mut rng, &fit);
        }
        relaxed += ledger.select(&k, &words, &mut picked, n, day, &mut rng, &fit);
        if picked.len() < 3 {
            return None;
        }
        if let Some(puzzle) = generate(rng.next_u64(), &input(lang, tier, size, picked, lex)) {
            return Some(Served { puzzle, key: k, relaxed });
        }
    }
    None
}

/// The Daily Puzzle: the same grid for every player of this language and tier
/// on this date (F-X1). `ymd` seeds the layout; `day` picks the words.
pub fn daily(lang: &str, tier: Tier, ymd: u32, day: u32) -> Option<Served> {
    if !eligible(lang) {
        return None;
    }
    let lex = Lexicon::get(lang);
    let k = key(lang, tier);
    let size = tier.size();
    let fit = |p: &[String], w: &str| fits(lang, size, p, w);
    let picked = daily_pick(&daily_pool(lang, tier), tier.targets(), day, &k, &fit);
    let base = seed(&format!("daily:{k}"), 0, ymd);
    // A word set that will not lay out cleanly loses one word at a time, the
    // same way on every device, rather than leaving the day without a Daily.
    for r in 0..picked.len() as u64 {
        let mut words = picked.clone();
        if r > 0 {
            words.remove(r as usize - 1);
        }
        if words.len() < 3 {
            break;
        }
        if let Some(puzzle) = generate(base.wrapping_add(r), &input(lang, tier, size, words, lex)) {
            return Some(Served { puzzle, key: k, relaxed: 0 });
        }
    }
    None
}

/// The words of a My Words list that a grid can hold, folded, in list order.
pub fn list_words(lang: &str, words: &[String]) -> Vec<String> {
    let lex = Lexicon::get(lang);
    let mut out: Vec<String> = Vec::new();
    for w in words {
        let f = fold(w);
        if playable_word(&f) && lex.spells(&f) && graphemes(&f).len() <= MAX_SIZE && !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

/// "Make a puzzle" on a My Words list. Repeating the player's own words is the
/// point, so the ledger's window does not apply; the counter in the seed makes
/// each puzzle from the same list lay out differently (F-X3).
pub fn from_list(lang: &str, tier: Tier, list_id: &str, words: &[String], counter: u64) -> Option<Served> {
    if !eligible(lang) {
        return None;
    }
    let lex = Lexicon::get(lang);
    let mut rng = Rng::new(seed(&format!("list:{list_id}"), counter, 0));
    let mut cands = list_words(lang, words);
    rng.shuffle(&mut cands);
    let longest = cands.iter().map(|w| graphemes(w).len()).max().unwrap_or(0);
    let size = tier.size().max(longest).min(MAX_SIZE);
    let mut picked: Vec<String> = Vec::new();
    for w in cands {
        if picked.len() == tier.targets() {
            break;
        }
        if fits(lang, size, &picked, &w) {
            picked.push(w);
        }
    }
    if picked.len() < 3 {
        return None;
    }
    let puzzle = generate(rng.next_u64(), &input(lang, tier, size, picked, lex))?;
    Some(Served { puzzle, key: key(lang, tier), relaxed: 0 })
}

/// I8: one number for 500 seeds across five configurations. The host test pins
/// it; the e2e recomputes it in the app's WebAssembly and must get the same.
pub fn golden_digest(count: u64) -> u64 {
    const MIX: [(&str, Tier); 5] = [("en", Tier::Easy), ("en", Tier::Hard), ("es", Tier::Easy), ("ru", Tier::Easy), ("en", Tier::Jr)];
    let mut s = String::new();
    for i in 0..count {
        let (lang, tier) = MIX[(i % 5) as usize];
        let ledger = Ledger { counter: i, ..Default::default() };
        let h = bank(lang, tier, &ledger, 0).map_or(0, |p| super::gen::hash(&p.puzzle));
        s.push_str(&format!("{h:x},"));
    }
    crate::spelldoku::rng::fnv(s.as_bytes())
}
