//! F21: My Words boards. Five spoken words and the secret, built from the player's own list,
//! with bank top-ups where the list alone cannot make a fair board.
//!
//! The list words arrive already matched to the bank (the caller used `word_index::canonical`,
//! and applied the kid filter on a Jr profile): this module never matches or reads a list. A
//! board has no silent word, so its uniqueness needs no validity list beyond the bank (census
//! C6) and it is generated on the device.

use crate::spelldoku::rng::{fnv, mix, Rng};

use super::gates::check_board;
use super::lex::{Lexicon, Word};
use super::types::{Board, Slot, SlotKind, Tier, GEN_VERSION, MAX_RUNES};

/// Words on a board: 5 spoken + the secret.
pub const WORDS: usize = 6;
/// At least this many list words, or no board is served (D44).
pub const MIN_LIST_WORDS: usize = 3;
/// Draws per (list-word count, secret kind). The census used 150; this is a little more.
const BUDGET: u32 = 200;

#[derive(Debug, PartialEq, Eq)]
pub struct MyWordsBoard {
    pub board: Board,
    /// How many of the six words are the player's own.
    pub list_words: usize,
}

fn mask_of(lex: &Lexicon, w: &[char]) -> Option<u64> {
    w.iter().try_fold(0u64, |m, c| lex.alphabet.binary_search(c).ok().map(|i| m | 1u64 << i))
}

fn assemble(seed: u64, lex: &Lexicon, words: &[Vec<char>], from_list: Vec<bool>) -> Board {
    let mut units: Vec<char> = words.iter().flatten().copied().collect();
    units.sort_unstable();
    units.dedup();
    Rng::new(mix(seed, 0x52_55_4e_45)).shuffle(&mut units);
    let rune_of = |c: char| units.iter().position(|&u| u == c).expect("unit has a rune") as u8;
    let slots: Vec<Slot> = words
        .iter()
        .enumerate()
        .map(|(i, w)| Slot { kind: if i + 1 == words.len() { SlotKind::Secret } else { SlotKind::Spoken }, answer: w.clone(), runes: w.iter().map(|&c| rune_of(c)).collect() })
        .collect();
    Board { lang: lex.lang.clone(), tier: Tier::Easy, seed, gen_version: GEN_VERSION, slots, rune_unit: units, par: None, run: None, list: Some(from_list) }
}

/// Rebuild a saved board from its words (a list edited since cannot change it).
pub fn board_from_words(lex: &Lexicon, seed: u64, words: &[String], from_list: &[bool]) -> Option<Board> {
    if words.len() != WORDS || from_list.len() != WORDS {
        return None;
    }
    let ws: Vec<Vec<char>> = words.iter().map(|w| w.chars().flat_map(|c| c.to_lowercase()).collect()).collect();
    Some(assemble(seed, lex, &ws, from_list.to_vec()))
}

fn pc(x: u64) -> u32 {
    x.count_ones()
}

/// One draw: `plan[i]` says whether position `i` takes a list word; the last position is the secret.
fn construct(list: &[(Vec<char>, u64)], top: &[(Vec<char>, u64)], plan: &[bool; WORDS], rng: &mut Rng) -> Option<Vec<(usize, bool)>> {
    let mut chosen: Vec<(usize, bool)> = Vec::with_capacity(WORDS);
    let mut union = 0u64;
    let mut count = [0u8; 64];
    for pos in 0..WORDS {
        let from_list = plan[pos];
        let pool = if from_list { list } else { top };
        let single: u64 = (0..64).filter(|&b| count[b] == 1).fold(0, |m, b| m | 1u64 << b);
        let last = pos + 1 == WORDS;
        let mut cands: Vec<(usize, u64)> = Vec::new();
        let mut total = 0u64;
        for (i, (_, m)) in pool.iter().enumerate() {
            if chosen.contains(&(i, from_list)) {
                continue;
            }
            if pos > 0 && (pc(union | m) as usize > MAX_RUNES || pc(union & m) < 2) {
                continue;
            }
            if pos == 0 && pc(*m) as usize > MAX_RUNES {
                continue;
            }
            if last && (m & !union != 0 || m & single != single) {
                continue;
            }
            let k = 1 + pc(m & single) as u64;
            let wt = k * k * k;
            total += wt;
            cands.push((i, wt));
        }
        if cands.is_empty() {
            return None;
        }
        let mut roll = rng.next_u64() % total;
        let mut pick = cands[cands.len() - 1].0;
        for (i, wt) in &cands {
            if roll < *wt {
                pick = *i;
                break;
            }
            roll -= wt;
        }
        let m = pool[pick].1;
        union |= m;
        for b in 0..64 {
            if m >> b & 1 == 1 {
                count[b] += 1;
            }
        }
        chosen.push((pick, from_list));
    }
    Some(chosen)
}

/// Build a board from `list_words` (already matched to the bank). `kid` keeps every word free of
/// sound-alikes (v1 I12). The generator tries as many list words as the gates allow, up to six,
/// and tries a bank top-up as the secret word before a list word. Fewer than three list words, or
/// no fair board inside the budget, is None.
pub fn generate(seed: u64, lex: &Lexicon, list_words: &[Vec<char>], kid: bool) -> Option<MyWordsBoard> {
    let usable = |w: &Vec<char>| (3..=9).contains(&w.len()) && w.iter().all(|c| c.is_alphabetic()) && (!kid || !lex.has_homophone(w));
    let mut list: Vec<(Vec<char>, u64)> = Vec::new();
    for w in list_words {
        if usable(w) && !list.iter().any(|(x, _)| x == w) {
            if let Some(m) = mask_of(lex, w) {
                list.push((w.clone(), m));
            }
        }
    }
    // Words that sound alike cannot share a board (G3): keep the first of each collision set.
    let mut keep: Vec<(Vec<char>, u64)> = Vec::new();
    for (w, m) in list {
        if !keep.iter().any(|(x, _)| lex.group(x).contains(&w)) {
            keep.push((w, m));
        }
    }
    let list = keep;
    if list.len() < MIN_LIST_WORDS {
        return None;
    }
    // Top-ups: Easy and Medium bands (Jr: what the Jr resolver serves), without sound-alikes when kid.
    let mut top: Vec<(Vec<char>, u64)> = Vec::new();
    let tiers: &[Tier] = if kid { &[Tier::Jr] } else { &[Tier::Easy, Tier::Medium] };
    for t in tiers {
        let pool: Vec<&Word> = lex.pool(*t);
        for w in pool {
            if (!kid || !lex.has_homophone(&w.units)) && !list.iter().any(|(x, _)| x == &w.units) && !top.iter().any(|(x, _)| x == &w.units) {
                top.push((w.units.clone(), w.mask));
            }
        }
    }
    let mut rng = Rng::new(mix(seed ^ fnv(format!("{}:mywords", lex.lang).as_bytes()), 1));
    let max_k = list.len().min(WORDS);
    for k in (MIN_LIST_WORDS..=max_k).rev() {
        // Secret as a top-up first (so list words are spoken), then as a list word.
        for secret_from_list in [false, true] {
            if k == WORDS && !secret_from_list {
                continue; // six list words need the secret to be one
            }
            if secret_from_list && k == 0 {
                continue;
            }
            for _ in 0..BUDGET {
                // A random placement of k list words among the six positions, with the secret fixed.
                let mut plan = [false; WORDS];
                let spoken_lists = if secret_from_list { k - 1 } else { k };
                let mut slots: Vec<usize> = (0..WORDS - 1).collect();
                rng.shuffle(&mut slots);
                for &s in slots.iter().take(spoken_lists) {
                    plan[s] = true;
                }
                plan[WORDS - 1] = secret_from_list;
                let Some(ch) = construct(&list, &top, &plan, &mut rng) else { continue };
                let words: Vec<Vec<char>> = ch.iter().map(|&(i, l)| if l { list[i].0.clone() } else { top[i].0.clone() }).collect();
                let from_list: Vec<bool> = ch.iter().map(|&(_, l)| l).collect();
                let board = assemble(seed, lex, &words, from_list);
                if check_board(&board, lex).is_ok() {
                    return Some(MyWordsBoard { board, list_words: k });
                }
            }
        }
    }
    None
}
