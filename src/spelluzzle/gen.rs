//! Board generation. The generator proposes; `gates::check_board` disposes.
//!
//! Search method (free to change, per F9): draw words one at a time from the
//! tier's pool, keeping the union within 16 runes and each new word sharing at
//! least 2 runes with those already chosen, weighting toward words that cover
//! spoken runes still held by only one word. Without that weighting, G5
//! rejects dominate. Every candidate then goes through the checker.

use std::collections::HashSet;

use crate::spelldoku::rng::{fnv, mix, Rng};

use super::gates::{check_board, GateFail};
use super::lex::{Lexicon, Word};
use super::types::{Board, Tier, GEN_VERSION, MAX_RUNES};

/// F9: draws per board before giving up.
pub const ATTEMPT_CAP: u32 = 2000;

/// F11: a new board shares at most this many words with the previous one, when
/// the pool allows.
pub const MAX_SHARED: usize = 2;

#[derive(Debug)]
pub enum GenError {
    Cap { attempts: u32, last: Option<GateFail> },
    EmptyPool,
}

pub struct Generated {
    pub board: Board,
    pub attempts: u32,
    /// True when the word-overlap rule had to be relaxed to finish (F9).
    pub relaxed_overlap: bool,
}

pub fn generate(seed: u64, tier: Tier, lex: &Lexicon) -> Result<Generated, GenError> {
    generate_after(seed, tier, lex, &[])
}

/// Like `generate`, but keeps the new board to at most `MAX_SHARED` words from
/// `previous` (F11). If that cannot be met within the attempt cap, relax the
/// overlap rule first and try again; never a gate.
pub fn generate_after(seed: u64, tier: Tier, lex: &Lexicon, previous: &[String]) -> Result<Generated, GenError> {
    // F12: a script with hundreds of units cannot give runes that repeat across a board.
    assert!(!super::offer::NEVER.contains(&lex.lang.as_str()), "Spelluzzle is never built for {}", lex.lang);
    let pool = lex.pool(tier);
    if pool.len() < tier.shape().slots() {
        return Err(GenError::EmptyPool);
    }
    let prev: HashSet<Vec<char>> = previous.iter().map(|w| w.chars().collect()).collect();
    let flagged: Vec<bool> = pool.iter().map(|w| prev.contains(&w.units)).collect();
    let mut rng = Rng::new(mix(seed ^ fnv(format!("{}:{}", lex.lang, tier.name()).as_bytes()), 1));
    let mut total = 0;
    let mut last = None;
    for (relaxed, max_shared) in [(false, MAX_SHARED), (true, usize::MAX)] {
        if relaxed && prev.is_empty() {
            break;
        }
        for _ in 0..ATTEMPT_CAP {
            total += 1;
            let Some(chosen) = construct(tier, &pool, &flagged, max_shared, &mut rng) else { continue };
            let board = assemble(seed, tier, lex, &pool, &chosen);
            match check_board(&board, lex) {
                Ok(_) => return Ok(Generated { board, attempts: total, relaxed_overlap: relaxed }),
                Err(e) => last = Some(e),
            }
        }
    }
    Err(GenError::Cap { attempts: total, last })
}

fn pc(x: u64) -> u32 {
    x.count_ones()
}

/// Pick `slots` words: spoken first, then silent, the secret last.
fn construct(tier: Tier, pool: &[&Word], flagged: &[bool], max_shared: usize, rng: &mut Rng) -> Option<Vec<usize>> {
    let shape = tier.shape();
    let n = shape.slots();
    let mut chosen: Vec<usize> = Vec::with_capacity(n);
    let (mut union, mut spoken_mask) = (0u64, 0u64);
    let mut count = [0u8; 64];
    let mut shared = 0;
    while chosen.len() < n {
        let pick = if chosen.is_empty() {
            let mut i = rng.below(pool.len());
            for _ in 0..pool.len() {
                if shared_ok(flagged[i], shared, max_shared) {
                    break;
                }
                i = (i + 1) % pool.len();
            }
            i
        } else {
            let single: u64 = (0..64).filter(|&b| count[b] == 1 && spoken_mask >> b & 1 == 1).fold(0, |m, b| m | 1u64 << b);
            let mut weights: Vec<(usize, u64)> = Vec::new();
            let mut total = 0u64;
            for (i, w) in pool.iter().enumerate() {
                if chosen.contains(&i) || !shared_ok(flagged[i], shared, max_shared) {
                    continue;
                }
                if pc(union | w.mask) as usize > MAX_RUNES || pc(union & w.mask) < 2 {
                    continue;
                }
                // The secret comes last. G6 needs all its runes elsewhere on the board and
                // G5 needs every spoken rune held by two words, so it has to close both.
                if chosen.len() + 1 == n && (w.mask & !union != 0 || w.mask & single != single) {
                    continue;
                }
                let k = 1 + pc(w.mask & single) as u64;
                let wt = k * k * k;
                total += wt;
                weights.push((i, wt));
            }
            if weights.is_empty() {
                return None;
            }
            let mut roll = rng.next_u64() % total;
            let mut pick = weights[weights.len() - 1].0;
            for (i, wt) in &weights {
                if roll < *wt {
                    pick = *i;
                    break;
                }
                roll -= wt;
            }
            pick
        };
        if flagged[pick] {
            shared += 1;
        }
        let m = pool[pick].mask;
        if chosen.len() < shape.spoken {
            spoken_mask |= m;
        }
        union |= m;
        for b in 0..64 {
            if m >> b & 1 == 1 {
                count[b] += 1;
            }
        }
        chosen.push(pick);
    }
    Some(chosen)
}

fn shared_ok(flag: bool, shared: usize, max_shared: usize) -> bool {
    !flag || shared < max_shared
}

fn assemble(seed: u64, tier: Tier, lex: &Lexicon, pool: &[&Word], chosen: &[usize]) -> Board {
    let shape = tier.shape();
    let words: Vec<&Vec<char>> = chosen.iter().map(|&i| &pool[i].units).collect();
    let mut units: Vec<char> = words.iter().flat_map(|w| w.iter().copied()).collect();
    units.sort_unstable();
    units.dedup();
    // F1: which glyph stands for which unit is a seeded permutation, never alphabet order.
    Rng::new(mix(seed, 0x52_55_4e_45)).shuffle(&mut units);
    let spoken: Vec<Vec<char>> = words[..shape.spoken].iter().map(|w| (*w).clone()).collect();
    let silent: Vec<Vec<char>> = words[shape.spoken..words.len() - 1].iter().map(|w| (*w).clone()).collect();
    let secret = words[words.len() - 1].clone();
    let mut b = Board::from_words(&lex.lang, tier, seed, &spoken, &silent, &secret, units);
    b.gen_version = GEN_VERSION;
    b
}
