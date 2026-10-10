//! F15: Par boards. Six listenable words and a secret word, every word silent at load;
//! each Listen is a stroke, and the board knows the fewest it needs (par).
//!
//! The one-short rule: an unheard word can be worked out when at most one of its runes is
//! still undecoded and exactly one validity-list string fits the decoded units. Par is
//! the size of the smallest set of Listens from which every word can be finished by
//! one-short steps alone. The checker here shares no code with the generator's choice
//! of words; it works from the board's own runes (I17).

use std::collections::HashMap;

use crate::spelldoku::rng::{fnv, mix, Rng};

use super::gates::GateFail;
use super::lex::{pattern, Lexicon, Word};
use super::types::{Board, Slot, SlotKind, Tier, GEN_VERSION, MAX_RUNES};

/// Six listenable words plus the secret (last).
pub const NW: usize = 7;
const ALL: u8 = (1 << NW) - 1;
/// F9's cap: draws per board.
pub const ATTEMPT_CAP: u32 = 2000;

pub fn offered_tier(tier: Tier) -> bool {
    matches!(tier, Tier::Hard | Tier::Expert)
}

/// The par a tier must have (census C14, Eric 2026-10-10): Expert is par 3 only; Hard is 2 or 3.
fn par_ok(tier: Tier, par: usize) -> bool {
    match tier {
        Tier::Expert => par == 3,
        _ => par == 2 || par == 3,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParVerdict {
    pub par: usize,
    /// Every set of listens (bit i = word i) of par size that finishes the board.
    pub par_sets: Vec<u8>,
}

fn fail<T>(gate: &'static str, detail: &str) -> Result<T, GateFail> {
    Err(GateFail { gate, detail: detail.to_string() })
}

struct Stepper<'a> {
    board: &'a Board,
    lex: &'a Lexicon,
    rs: Vec<u16>,
    memo: HashMap<(u8, u16), bool>,
}

impl<'a> Stepper<'a> {
    fn new(board: &'a Board, lex: &'a Lexicon) -> Self {
        let rs = board.slots.iter().map(|s| s.runes.iter().fold(0u16, |m, &r| m | 1 << r)).collect();
        Stepper { board, lex, rs, memo: HashMap::new() }
    }

    /// Can word `w` be finished given the decoded runes in `known`?
    fn ok(&mut self, w: usize, known: u16) -> bool {
        let und = self.rs[w] & !known;
        if und == 0 {
            return true;
        }
        if und.count_ones() > 1 {
            return false;
        }
        if let Some(&v) = self.memo.get(&(w as u8, known)) {
            return v;
        }
        let b = self.board;
        let slot = &b.slots[w];
        let known_units: Vec<char> = (0..b.n_runes()).filter(|&r| known >> r & 1 == 1).map(|r| b.rune_unit[r]).collect();
        let pat = pattern(&slot.answer);
        let mut hits = 0;
        for cand in self.lex.valid_with_pattern(&pat) {
            let fit = slot.runes.iter().zip(cand).all(|(&r, &u)| if known >> r & 1 == 1 { b.rune_unit[r as usize] == u } else { !known_units.contains(&u) });
            if fit {
                hits += 1;
                if hits > 1 {
                    break;
                }
            }
        }
        let v = hits == 1;
        self.memo.insert((w as u8, known), v);
        v
    }

    fn works(&mut self, set: u8) -> bool {
        let mut known = (0..NW).filter(|&w| set >> w & 1 == 1).fold(0u16, |m, w| m | self.rs[w]);
        let mut done = set;
        loop {
            let mut progressed = false;
            for w in 0..NW {
                if done >> w & 1 == 0 && self.ok(w, known) {
                    done |= 1 << w;
                    known |= self.rs[w];
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        done == ALL
    }
}

/// Par and the par sets by trying every set of the six listenable words, smallest first.
pub fn par_of(board: &Board, lex: &Lexicon) -> ParVerdict {
    let mut st = Stepper::new(board, lex);
    let mut sets: Vec<u8> = (0u8..(1 << (NW - 1))).collect();
    sets.sort_by_key(|s| (s.count_ones(), *s));
    let mut par: Option<usize> = None;
    let mut par_sets = Vec::new();
    for s in sets {
        let k = s.count_ones() as usize;
        if par.is_some_and(|p| k > p) {
            break;
        }
        if st.works(s) {
            par = Some(k);
            par_sets.push(s);
        }
    }
    // Listening to all six always finishes, so a par always exists.
    ParVerdict { par: par.unwrap_or(NW - 1), par_sets }
}

/// With the words in `set` made spoken (the rest silent), how many full assignments (to 2)?
fn solutions_with(board: &Board, lex: &Lexicon, set: u8) -> u32 {
    let mut b = board.clone();
    for (i, s) in b.slots.iter_mut().enumerate() {
        s.kind = if i == NW - 1 {
            SlotKind::Secret
        } else if set >> i & 1 == 1 {
            SlotKind::Spoken
        } else {
            SlotKind::Silent
        };
    }
    super::gates::count_solutions(&b, lex, 2)
}

/// The independent checker: G1, G2, G4, G6 and P1 to P5. `all_p5` checks every par set
/// (the build step does); the generator checks the first.
pub fn check_par(board: &Board, lex: &Lexicon, all_p5: bool) -> Result<ParVerdict, GateFail> {
    let sh = board.tier.shape();
    let n = board.slots.len();
    if n != NW || !offered_tier(board.tier) {
        return fail("shape", "a Par board is Hard or Expert with six words and a secret");
    }
    let words: Vec<&Vec<char>> = board.slots.iter().map(|s| &s.answer).collect();
    for (i, w) in words.iter().enumerate() {
        if !w.iter().all(|c| c.is_alphabetic()) || !(sh.lo..=sh.hi).contains(&w.len()) || !lex.in_band(board.tier, w) {
            return fail("G1", "word not in band or length");
        }
        if words[i + 1..].contains(w) {
            return fail("G1", "duplicate word");
        }
    }
    if board.n_runes() > MAX_RUNES {
        return fail("G2", "more than 16 runes");
    }
    let rs: Vec<u16> = board.slots.iter().map(|s| s.runes.iter().fold(0u16, |m, &r| m | 1 << r)).collect();
    for i in 0..n {
        let rest = (0..n).filter(|&j| j != i).fold(0u16, |m, j| m | rs[j]);
        if (rs[i] & rest).count_ones() < 2 {
            return fail("G4", "a word shares fewer than two runes with the rest");
        }
    }
    let si = n - 1;
    for r in 0..board.n_runes() {
        if rs[si] >> r & 1 == 1 && !(0..si).any(|j| rs[j] >> r & 1 == 1) {
            return fail("G6a", "a secret rune appears nowhere else");
        }
    }
    for j in 0..si {
        let d = board.slots[si].runes.iter().filter(|&&r| rs[j] >> r & 1 == 1).count();
        if d * 10 > 6 * board.slots[si].runes.len() {
            return fail("G6b", "one word decodes more than 60% of the secret");
        }
    }
    if words.iter().any(|w| lex.has_homophone(w)) {
        return fail("P1", "a word has a sound-alike");
    }
    for r in 0..board.n_runes() {
        if rs.iter().filter(|m| **m >> r & 1 == 1).count() < 2 {
            return fail("P2", "a rune appears in only one word");
        }
    }
    let v = par_of(board, lex);
    if !par_ok(board.tier, v.par) {
        return fail("P3", "par is outside the tier's range");
    }
    let sized = if v.par == 2 { 15 } else { 20 };
    if v.par_sets.len() < 2 {
        return fail("P4", "only one par set: there is no choice to make");
    }
    if v.par_sets.len() * 10 > sized * 4 {
        return fail("P4", "most sets are par sets: the choice does not matter");
    }
    let check: &[u8] = if all_p5 { &v.par_sets } else { &v.par_sets[..1] };
    for &s in check {
        if solutions_with(board, lex, s) != 1 {
            return fail("P5", "a par set does not give one answer");
        }
    }
    Ok(v)
}

/// A Par board as the build step ships it: the seed (for the rune numbering), the par that
/// was proved under the large list, and the seven words as indexes into the tier's pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParRecord {
    pub seed: u64,
    pub par: u8,
    pub words: [u16; NW],
}

fn pool_of<'a>(lex: &'a Lexicon, tier: Tier) -> Vec<&'a Word> {
    lex.pool(tier)
}

/// Build the board a record names. Fails (None) if the pool no longer has those indexes.
pub fn board_from_record(lex: &Lexicon, tier: Tier, rec: &ParRecord) -> Option<Board> {
    let pool = pool_of(lex, tier);
    let words: Vec<&Vec<char>> = rec.words.iter().map(|&i| pool.get(i as usize).map(|w| &w.units)).collect::<Option<_>>()?;
    Some(assemble(rec.seed, tier, &lex.lang, &words, Some(rec.par)))
}

fn assemble(seed: u64, tier: Tier, lang: &str, words: &[&Vec<char>], par: Option<u8>) -> Board {
    let mut units: Vec<char> = words.iter().flat_map(|w| w.iter().copied()).collect();
    units.sort_unstable();
    units.dedup();
    // F1: which glyph stands for which unit is a seeded permutation, never alphabet order.
    Rng::new(mix(seed, 0x52_55_4e_45)).shuffle(&mut units);
    let rune_of = |c: char| units.iter().position(|&u| u == c).expect("unit has a rune") as u8;
    let slots: Vec<Slot> = words
        .iter()
        .enumerate()
        .map(|(i, w)| Slot { kind: if i == NW - 1 { SlotKind::Secret } else { SlotKind::Silent }, answer: (*w).clone(), runes: w.iter().map(|&c| rune_of(c)).collect() })
        .collect();
    Board { lang: lang.to_string(), tier, seed, gen_version: GEN_VERSION, slots, rune_unit: units, par }
}

fn pc(x: u64) -> u32 {
    x.count_ones()
}

/// Draw seven words: the secret last, closing every rune that is still held by one word.
fn construct(pool: &[&Word], rng: &mut Rng) -> Option<Vec<usize>> {
    let mut chosen: Vec<usize> = Vec::with_capacity(NW);
    let mut union = 0u64;
    let mut count = [0u8; 64];
    while chosen.len() < NW {
        let pick = if chosen.is_empty() {
            rng.below(pool.len())
        } else {
            let single: u64 = (0..64).filter(|&b| count[b] == 1).fold(0, |m, b| m | 1u64 << b);
            let last = chosen.len() + 1 == NW;
            let mut weights: Vec<(usize, u64)> = Vec::new();
            let mut total = 0u64;
            for (i, w) in pool.iter().enumerate() {
                if chosen.contains(&i) || pc(union | w.mask) as usize > MAX_RUNES || pc(union & w.mask) < 2 {
                    continue;
                }
                if last && (w.mask & !union != 0 || w.mask & single != single) {
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
        union |= pool[pick].mask;
        for b in 0..64 {
            if pool[pick].mask >> b & 1 == 1 {
                count[b] += 1;
            }
        }
        chosen.push(pick);
    }
    Some(chosen)
}

#[derive(Debug)]
pub struct ParGenerated {
    pub board: Board,
    pub record: ParRecord,
    pub attempts: u32,
}

/// Generate a Par board for `seed` against `lex`. Deterministic. The words are drawn from the
/// tier's pool without sound-alikes (P1); the pool index list in the record is over the
/// WHOLE pool so a record can be rebuilt without this filter.
pub fn generate_par(seed: u64, tier: Tier, lex: &Lexicon) -> Option<ParGenerated> {
    if !offered_tier(tier) {
        return None;
    }
    let whole: Vec<&Word> = lex.pool(tier);
    let idx: Vec<usize> = (0..whole.len()).filter(|&i| !lex.has_homophone(&whole[i].units)).collect();
    if idx.len() < NW {
        return None;
    }
    let pool: Vec<&Word> = idx.iter().map(|&i| whole[i]).collect();
    let mut rng = Rng::new(mix(seed ^ fnv(format!("{}:par:{}", lex.lang, tier.name()).as_bytes()), 1));
    for attempt in 1..=ATTEMPT_CAP {
        let Some(ch) = construct(&pool, &mut rng) else { continue };
        let words: Vec<&Vec<char>> = ch.iter().map(|&i| &pool[i].units).collect();
        let board = assemble(seed, tier, &lex.lang, &words, None);
        if let Ok(v) = check_par(&board, lex, false) {
            let mut b = board;
            b.par = Some(v.par as u8);
            let mut w = [0u16; NW];
            for (k, &i) in ch.iter().enumerate() {
                w[k] = idx[i] as u16;
            }
            return Some(ParGenerated { board: b, record: ParRecord { seed, par: v.par as u8, words: w }, attempts: attempt });
        }
    }
    None
}
