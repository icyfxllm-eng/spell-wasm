//! F20: Rune Duel. A territory game on the cipher: two players (or a player and the computer)
//! take turns spelling words from one shared board, and the runes a correct word newly decodes
//! are claimed by whoever spelled it.
//!
//! This file is the whole rule set and holds no DOM and no storage. The only verdicts a Duel
//! shows are the ones it computes here (D23, the one exception to v1 I6).
//!
//! Two rules the spec leaves open are read as follows, and noted here so they can be changed in
//! one place: a wrong Listen-and-spell leaves the slot heard and open for another try (by either
//! player), and a wrong Spell-unheard leaves the slot unheard and open.

use serde::{Deserialize, Serialize};

use crate::spelldoku::rng::{fnv, mix, Rng};

use super::lex::{pattern, Lexicon, Word};
use super::types::{Board, Slot, SlotKind, Tier, GEN_VERSION, MAX_RUNES};

/// Six words and the secret (last).
pub const WORDS: usize = 7;
/// Points per rune claimed (D41).
pub const HEARD_PTS: i64 = 1;
pub const UNHEARD_PTS: i64 = 2;
pub const SECRET_PTS: i64 = 3;
/// An NPC's chance of being right on an unheard word is this fraction of its heard accuracy.
const UNHEARD_FACTOR_NUM: i64 = 8;
const UNHEARD_FACTOR_DEN: i64 = 10;

pub fn offered_tier(tier: Tier) -> bool {
    matches!(tier, Tier::Easy | Tier::Medium | Tier::Hard | Tier::Expert)
}

// ------------------------------------------------------------------ board

fn pc(x: u64) -> u32 {
    x.count_ones()
}

/// The Duel gates: v1 G1, G2, G4, G6 and P1 (no sound-alikes).
pub fn check_duel_board(board: &Board, lex: &Lexicon) -> Result<(), &'static str> {
    let sh = board.tier.shape();
    let n = board.slots.len();
    if n != WORDS || !offered_tier(board.tier) {
        return Err("shape");
    }
    for (i, s) in board.slots.iter().enumerate() {
        let w = &s.answer;
        if !w.iter().all(|c| c.is_alphabetic()) || !(sh.lo..=sh.hi).contains(&w.len()) || !lex.in_band(board.tier, w) {
            return Err("G1");
        }
        if lex.has_homophone(w) {
            return Err("P1");
        }
        if board.slots[i + 1..].iter().any(|x| &x.answer == w) {
            return Err("G1");
        }
    }
    if board.n_runes() > MAX_RUNES {
        return Err("G2");
    }
    let rs: Vec<u16> = board.slots.iter().map(|s| s.runes.iter().fold(0u16, |m, &r| m | 1 << r)).collect();
    for i in 0..n {
        let rest = (0..n).filter(|&j| j != i).fold(0u16, |m, j| m | rs[j]);
        if (rs[i] & rest).count_ones() < 2 {
            return Err("G4");
        }
    }
    let si = n - 1;
    let others = (0..si).fold(0u16, |m, j| m | rs[j]);
    if rs[si] & !others != 0 {
        return Err("G6a");
    }
    for j in 0..si {
        let d = board.slots[si].runes.iter().filter(|&&r| rs[j] >> r & 1 == 1).count();
        if d * 10 > 6 * board.slots[si].runes.len() {
            return Err("G6b");
        }
    }
    Ok(())
}

fn assemble(seed: u64, tier: Tier, lang: &str, words: &[Vec<char>]) -> Board {
    let mut units: Vec<char> = words.iter().flatten().copied().collect();
    units.sort_unstable();
    units.dedup();
    Rng::new(mix(seed, 0x52_55_4e_45)).shuffle(&mut units);
    let rune_of = |c: char| units.iter().position(|&u| u == c).expect("unit has a rune") as u8;
    let slots = words
        .iter()
        .enumerate()
        .map(|(i, w)| Slot { kind: if i + 1 == words.len() { SlotKind::Secret } else { SlotKind::Silent }, answer: w.clone(), runes: w.iter().map(|&c| rune_of(c)).collect() })
        .collect();
    Board { lang: lang.to_string(), tier, seed, gen_version: GEN_VERSION, slots, rune_unit: units, par: None, run: None, list: None }
}

/// Rebuild a saved leg's board from its seed and words.
pub fn board_from_words(lang: &str, tier: Tier, seed: u64, words: &[String]) -> Option<Board> {
    if words.len() != WORDS {
        return None;
    }
    let ws: Vec<Vec<char>> = words.iter().map(|w| w.chars().flat_map(|c| c.to_lowercase()).collect()).collect();
    Some(assemble(seed, tier, lang, &ws))
}

fn construct(pool: &[&Word], rng: &mut Rng) -> Option<Vec<usize>> {
    let mut chosen: Vec<usize> = Vec::with_capacity(WORDS);
    let mut union = 0u64;
    while chosen.len() < WORDS {
        let last = chosen.len() + 1 == WORDS;
        let pick = if chosen.is_empty() {
            rng.below(pool.len())
        } else {
            let mut cands: Vec<(usize, u64)> = Vec::new();
            let mut total = 0u64;
            for (i, w) in pool.iter().enumerate() {
                if chosen.contains(&i) || pc(union | w.mask) as usize > MAX_RUNES || pc(union & w.mask) < 2 {
                    continue;
                }
                if last && w.mask & !union != 0 {
                    continue;
                }
                let sh = 1 + pc(union & w.mask) as u64;
                let wt = sh * sh * sh;
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
            pick
        };
        union |= pool[pick].mask;
        chosen.push(pick);
    }
    Some(chosen)
}

/// A Duel leg's board for `seed`: six words and a secret, none with a sound-alike, none in
/// `exclude` (the other leg's words). Deterministic. Generated on the device: a Duel needs no
/// validity list, because no entry is ever kept that could clash.
pub fn generate_board(seed: u64, tier: Tier, lex: &Lexicon, exclude: &[String]) -> Option<Board> {
    if !offered_tier(tier) {
        return None;
    }
    let skip: Vec<Vec<char>> = exclude.iter().map(|w| w.chars().collect()).collect();
    let pool: Vec<&Word> = lex.pool(tier).into_iter().filter(|w| !lex.has_homophone(&w.units) && !skip.contains(&w.units)).collect();
    if pool.len() < WORDS {
        return None;
    }
    let mut rng = Rng::new(mix(seed ^ fnv(format!("{}:duel:{}", lex.lang, tier.name()).as_bytes()), 1));
    for _ in 0..2000 {
        let Some(ch) = construct(&pool, &mut rng) else { continue };
        let words: Vec<Vec<char>> = ch.iter().map(|&i| pool[i].units.clone()).collect();
        let board = assemble(seed, tier, &lex.lang, &words);
        if check_duel_board(&board, lex).is_ok() {
            return Some(board);
        }
    }
    None
}

// ------------------------------------------------------------------ state

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opponent {
    Friend,
    /// The computer, at a Board Game difficulty: 0 Easy, 1 Normal, 2 Tough.
    Computer(u8),
}

/// What happened to a slot, as the table shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotState {
    Unheard,
    Heard,
    Solved,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegState {
    pub heard: Vec<bool>,
    pub solved: Vec<bool>,
    pub locked: Vec<bool>,
    /// Per rune: decoded by a solved word.
    pub decoded: Vec<bool>,
    /// Per rune: who claimed it (0 or 1).
    pub owner: Vec<Option<u8>>,
    pub points: [i64; 2],
    pub passes: u8,
    pub over: bool,
}

impl LegState {
    fn new(n_runes: usize) -> LegState {
        LegState { heard: vec![false; WORDS], solved: vec![false; WORDS], locked: vec![false; WORDS], decoded: vec![false; n_runes], owner: vec![None; n_runes], points: [0, 0], passes: 0, over: false }
    }
}

/// The result of one spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spelled {
    pub correct: bool,
    /// Runes newly claimed (empty on a miss).
    pub claimed: Vec<u8>,
    pub points: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// The player listened first: 1 point a rune.
    Heard,
    /// Nobody had listened: 2 a rune, 3 on the secret.
    Unheard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    Over,
    Solved,
    Locked,
    /// Spell unheard on a word that has been heard; or Listen on the secret.
    NotOffered,
    NotHeard,
}

pub struct Duel {
    pub tier: Tier,
    pub opponent: Opponent,
    pub boards: Vec<Board>,
    pub legs: Vec<LegState>,
    /// The leg on the table, 0 or 1.
    pub leg: usize,
    /// Who starts leg 1; the other player starts leg 2 (I22).
    pub starter: u8,
    pub turn: u8,
    /// Each player's spellings, for the computer's adaptive accuracy.
    pub hits: [i64; 2],
    pub attempts: [i64; 2],
}

impl Duel {
    pub fn new(boards: Vec<Board>, tier: Tier, opponent: Opponent, starter: u8) -> Duel {
        let legs = boards.iter().map(|b| LegState::new(b.n_runes())).collect();
        Duel { tier, opponent, boards, legs, leg: 0, starter: starter & 1, turn: starter & 1, hits: [0, 0], attempts: [0, 0] }
    }

    pub fn is_computer(&self, player: u8) -> bool {
        matches!(self.opponent, Opponent::Computer(_)) && player == 1
    }

    pub fn board(&self) -> &Board {
        &self.boards[self.leg]
    }

    pub fn state(&self) -> &LegState {
        &self.legs[self.leg]
    }

    pub fn slot_state(&self, slot: usize) -> SlotState {
        let l = self.state();
        if l.solved[slot] {
            SlotState::Solved
        } else if l.locked[slot] {
            SlotState::Locked
        } else if l.heard[slot] {
            SlotState::Heard
        } else {
            SlotState::Unheard
        }
    }

    /// Both legs played.
    pub fn match_over(&self) -> bool {
        self.legs.iter().all(|l| l.over)
    }

    pub fn totals(&self) -> [i64; 2] {
        self.legs.iter().fold([0, 0], |t, l| [t[0] + l.points[0], t[1] + l.points[1]])
    }

    /// The winner (0 or 1), or None for a draw. Meaningful once the match is over.
    pub fn winner(&self) -> Option<u8> {
        let t = self.totals();
        match t[0].cmp(&t[1]) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// Whether the slot can be spelled at all this turn.
    fn open(&self, slot: usize) -> Result<(), Refused> {
        if self.match_over() || self.state().over {
            return Err(Refused::Over);
        }
        match self.slot_state(slot) {
            SlotState::Solved => Err(Refused::Solved),
            SlotState::Locked => Err(Refused::Locked),
            _ => Ok(()),
        }
    }

    /// "Listen and spell" begins: the orb plays the word, so the slot is heard from now on.
    /// Not offered on the secret word. Returns the word to play.
    pub fn listen(&mut self, slot: usize) -> Result<String, Refused> {
        self.open(slot)?;
        if slot + 1 == WORDS {
            return Err(Refused::NotOffered);
        }
        self.legs[self.leg].heard[slot] = true;
        Ok(self.boards[self.leg].slots[slot].answer.iter().collect())
    }

    /// Commit an entry. `how` says which action it was. A correct entry claims; a wrong one is
    /// cleared and changes nothing else. The turn then passes.
    pub fn submit(&mut self, slot: usize, typed: &[char], how: How) -> Result<Spelled, Refused> {
        self.open(slot)?;
        match how {
            How::Unheard if self.legs[self.leg].heard[slot] => return Err(Refused::NotOffered),
            How::Heard if !self.legs[self.leg].heard[slot] => return Err(Refused::NotHeard),
            _ => {}
        }
        let correct = typed == self.boards[self.leg].slots[slot].answer.as_slice();
        Ok(self.resolve(slot, how, correct))
    }

    /// Apply a spelling whose result is already known (the computer's draw).
    pub fn resolve(&mut self, slot: usize, how: How, correct: bool) -> Spelled {
        let p = self.turn as usize;
        self.attempts[p] += 1;
        let mut out = Spelled { correct, claimed: Vec::new(), points: 0 };
        let per = match how {
            How::Heard => HEARD_PTS,
            How::Unheard if slot + 1 == WORDS => SECRET_PTS,
            How::Unheard => UNHEARD_PTS,
        };
        if correct {
            self.hits[p] += 1;
            let runes = self.boards[self.leg].slots[slot].runes.clone();
            let leg = &mut self.legs[self.leg];
            leg.solved[slot] = true;
            for r in runes {
                let r = r as usize;
                if !leg.decoded[r] {
                    leg.decoded[r] = true;
                    leg.owner[r] = Some(self.turn);
                    out.claimed.push(r as u8);
                }
            }
            out.points = per * out.claimed.len() as i64;
            leg.points[p] += out.points;
            // Words with nothing left to claim lock, unscored.
            for s in 0..WORDS {
                if !leg.solved[s] && !leg.locked[s] && self.boards[self.leg].slots[s].runes.iter().all(|&r| leg.decoded[r as usize]) {
                    leg.locked[s] = true;
                }
            }
        }
        self.legs[self.leg].passes = 0;
        self.end_turn();
        out
    }

    pub fn pass(&mut self) {
        if self.match_over() || self.state().over {
            return;
        }
        self.legs[self.leg].passes += 1;
        self.end_turn();
    }

    fn end_turn(&mut self) {
        let l = &mut self.legs[self.leg];
        let done = (0..WORDS).all(|s| l.solved[s] || l.locked[s]);
        if done || l.passes >= 2 {
            l.over = true;
            // The next leg is started by the other player (I22).
            if self.leg + 1 < self.legs.len() {
                self.leg += 1;
                self.turn = 1 - self.starter;
            }
            return;
        }
        self.turn = 1 - self.turn;
    }

    // -------------------------------------------------------------- the computer

    /// What the computer's choice may look at: the table as both players see it. No answers.
    fn visible(&self) -> Visible<'_> {
        let b = self.board();
        let l = self.state();
        Visible {
            runes: b.slots.iter().map(|s| s.runes.as_slice()).collect(),
            decoded_unit: (0..b.n_runes()).map(|r| l.decoded[r].then(|| b.rune_unit[r])).collect(),
            heard: &l.heard,
            solved: &l.solved,
            locked: &l.locked,
        }
    }

    /// The computer's accuracy now, in thousandths: the Board Game's own formula (D43), fed the
    /// human's running record in this match.
    pub fn npc_accuracy_milli(&self, difficulty: u8) -> i64 {
        let cfg = crate::boardgame::Variant::Full.cfg();
        let d = [crate::boardgame::Difficulty::Easy, crate::boardgame::Difficulty::Normal, crate::boardgame::Difficulty::Tough][(difficulty as usize).min(2)];
        let human = crate::boardgame::human_accuracy_milli(cfg.prior(), self.hits[0], self.attempts[0]);
        crate::boardgame::engine::npc_accuracy_milli(human, cfg.delta(d))
    }

    /// The computer's move for this turn: the action with the most expected points from what is
    /// visible, or a pass when none is worth taking.
    pub fn npc_choice(&self, lex: &Lexicon) -> Option<NpcMove> {
        let Opponent::Computer(diff) = self.opponent else { return None };
        self.npc_choice_at(lex, self.npc_accuracy_milli(diff))
    }

    pub fn npc_choice_at(&self, lex: &Lexicon, acc: i64) -> Option<NpcMove> {
        let v = self.visible();
        let mut best: Option<(i64, NpcMove)> = None;
        let mut consider = |ev: i64, m: NpcMove| {
            if best.as_ref().map_or(true, |(b, _)| ev > *b) {
                best = Some((ev, m));
            }
        };
        for s in 0..WORDS {
            if v.solved[s] || v.locked[s] {
                continue;
            }
            let new = v.runes[s].iter().filter(|&&r| v.decoded_unit[r as usize].is_none()).map(|&r| r).collect::<std::collections::BTreeSet<u8>>().len() as i64;
            if new == 0 {
                continue;
            }
            if s + 1 < WORDS {
                consider(acc * new * HEARD_PTS, NpcMove { slot: s, how: How::Heard });
            }
            // Unheard only where the word can be worked out: at most two runes undecoded and
            // exactly one validity-list word fits.
            if !v.heard[s] && new <= 2 && v.one_fit(lex, s) {
                let per = if s + 1 == WORDS { SECRET_PTS } else { UNHEARD_PTS };
                consider(acc * UNHEARD_FACTOR_NUM / UNHEARD_FACTOR_DEN * new * per, NpcMove { slot: s, how: How::Unheard });
            }
        }
        best.map(|(_, m)| m)
    }

    /// Play the computer's whole turn: choose, then draw whether it is right at its accuracy
    /// (0.8 times it on an unheard word). Returns what it did.
    pub fn npc_turn(&mut self, lex: &Lexicon, rng: &mut Rng) -> NpcDone {
        let Opponent::Computer(diff) = self.opponent else { return NpcDone::Passed };
        let acc = self.npc_accuracy_milli(diff);
        self.npc_turn_at(lex, rng, acc)
    }

    /// The same turn at an explicit accuracy (thousandths). The census and the fairness test
    /// drive both seats with this, at 0.60, 0.75 and 0.90.
    pub fn npc_turn_at(&mut self, lex: &Lexicon, rng: &mut Rng, acc: i64) -> NpcDone {
        let Some(m) = self.npc_choice_at(lex, acc) else {
            self.pass();
            return NpcDone::Passed;
        };
        let chance = match m.how {
            How::Heard => acc,
            How::Unheard => acc * UNHEARD_FACTOR_NUM / UNHEARD_FACTOR_DEN,
        };
        let word = if m.how == How::Heard { self.listen(m.slot).ok() } else { None };
        let correct = (rng.next_u64() % 1000) as i64 <= chance;
        let spelled = self.resolve(m.slot, m.how, correct);
        NpcDone::Spelled { slot: m.slot, how: m.how, heard_word: word, spelled }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NpcMove {
    pub slot: usize,
    pub how: How,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NpcDone {
    Passed,
    Spelled { slot: usize, how: How, heard_word: Option<String>, spelled: Spelled },
}

/// The table as the computer may see it. It holds rune numbers and the units of runes that are
/// already decoded, and nothing from which an undecoded word could be read (A37).
struct Visible<'a> {
    runes: Vec<&'a [u8]>,
    decoded_unit: Vec<Option<char>>,
    heard: &'a [bool],
    solved: &'a [bool],
    locked: &'a [bool],
}

impl Visible<'_> {
    /// Exactly one validity-list word fits slot `s`: same isomorphism pattern as its runes, the
    /// decoded runes equal their units, and no undecoded rune takes a unit already decoded.
    fn one_fit(&self, lex: &Lexicon, s: usize) -> bool {
        let pat = pattern_of(self.runes[s]);
        let used: Vec<char> = self.decoded_unit.iter().flatten().copied().collect();
        let mut hits = 0;
        for w in lex.valid_with_pattern(&pat) {
            let ok = self.runes[s].iter().zip(w).all(|(&r, &u)| match self.decoded_unit[r as usize] {
                Some(d) => d == u,
                None => !used.contains(&u),
            });
            if ok {
                hits += 1;
                if hits > 1 {
                    return false;
                }
            }
        }
        hits == 1
    }
}

fn pattern_of(runes: &[u8]) -> Vec<u8> {
    let mut seen: Vec<u8> = Vec::new();
    runes
        .iter()
        .map(|r| match seen.iter().position(|s| s == r) {
            Some(i) => i as u8,
            None => {
                seen.push(*r);
                (seen.len() - 1) as u8
            }
        })
        .collect()
}

// ------------------------------------------------------------------ saving

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuelSave {
    pub tier: String,
    pub opponent: Opponent,
    pub starter: u8,
    pub turn: u8,
    pub leg: usize,
    pub seeds: Vec<u64>,
    pub words: Vec<Vec<String>>,
    pub legs: Vec<LegState>,
    pub hits: [i64; 2],
    pub attempts: [i64; 2],
}

impl Duel {
    pub fn save(&self) -> DuelSave {
        DuelSave {
            tier: self.tier.name().to_string(),
            opponent: self.opponent.clone(),
            starter: self.starter,
            turn: self.turn,
            leg: self.leg,
            seeds: self.boards.iter().map(|b| b.seed).collect(),
            words: self.boards.iter().map(|b| b.words()).collect(),
            legs: self.legs.clone(),
            hits: self.hits,
            attempts: self.attempts,
        }
    }

    pub fn restore(lang: &str, s: &DuelSave) -> Option<Duel> {
        let tier = Tier::from_name(&s.tier)?;
        if s.seeds.len() != 2 || s.words.len() != 2 || s.legs.len() != 2 {
            return None;
        }
        let boards: Vec<Board> = s.seeds.iter().zip(&s.words).map(|(seed, w)| board_from_words(lang, tier, *seed, w)).collect::<Option<_>>()?;
        if s.legs.iter().zip(&boards).any(|(l, b)| l.decoded.len() != b.n_runes() || l.heard.len() != WORDS) {
            return None;
        }
        Some(Duel { tier, opponent: s.opponent.clone(), boards, legs: s.legs.clone(), leg: s.leg.min(1), starter: s.starter & 1, turn: s.turn & 1, hits: s.hits, attempts: s.attempts })
    }
}
