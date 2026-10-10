//! F9: the independent checker. A board is served only if `check_board` accepts
//! all of G1 to G11.
//!
//! This file shares no code path with `gen`. Ratios use integers and u128
//! rationals so every platform agrees (F9 arithmetic rule).

use super::lex::{pattern, Lexicon};
use super::types::{Board, SlotKind, MAX_RUNES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateFail {
    /// "shape", or "G1" to "G11" (G6 is split "G6a" and "G6b").
    pub gate: &'static str,
    pub detail: String,
}

fn fail<T>(gate: &'static str, detail: impl Into<String>) -> Result<T, GateFail> {
    Err(GateFail { gate, detail: detail.into() })
}

/// A non-negative rational.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frac {
    pub n: u128,
    pub d: u128,
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

impl Frac {
    pub fn new(n: u128, d: u128) -> Frac {
        let g = gcd(n, d).max(1);
        Frac { n: n / g, d: d / g }
    }
    fn add(self, o: Frac) -> Frac {
        Frac::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
    fn div(self, k: u128) -> Frac {
        Frac::new(self.n, self.d * k)
    }
    /// self >= n/d
    pub fn at_least(self, n: u128, d: u128) -> bool {
        self.n * d >= n * self.d
    }
    pub fn as_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub runes: usize,
    pub cells: usize,
    pub earned: Frac,
    pub cascade: Frac,
}

fn rune_sets(board: &Board) -> Vec<u16> {
    board.slots.iter().map(|s| s.runes.iter().fold(0u16, |m, &r| m | 1 << r)).collect()
}

pub fn check_board(board: &Board, lex: &Lexicon) -> Result<Metrics, GateFail> {
    // F15: a Par board has its own gates (G1, G2, G4, G6 and P1 to P5); v1's G3, G5 and G7 to G11
    // describe a fixed spoken set, which it does not have.
    if board.par.is_some() {
        super::par::check_par(board, lex, false)?;
        let z = Frac::new(0, 1);
        return Ok(Metrics { runes: board.n_runes(), cells: board.cells(), earned: z, cascade: z });
    }
    let shape = board.tier.shape();
    let ns = shape.spoken;
    let slots = &board.slots;
    let n = slots.len();

    // Shape: the tier's slot kinds, in order, with exactly one secret last.
    if n != shape.slots() {
        return fail("shape", format!("{n} slots, want {}", shape.slots()));
    }
    for (i, s) in slots.iter().enumerate() {
        let want = if i < ns {
            SlotKind::Spoken
        } else if i + 1 < n {
            SlotKind::Silent
        } else {
            SlotKind::Secret
        };
        if s.kind != want {
            return fail("shape", format!("slot {i} is {:?}", s.kind));
        }
        if s.runes.len() != s.answer.len() || s.runes.iter().zip(&s.answer).any(|(&r, &u)| board.rune_unit.get(r as usize) != Some(&u)) {
            return fail("shape", format!("slot {i} runes do not spell its answer"));
        }
    }

    // G1 words.
    for s in slots {
        let w: String = s.answer.iter().collect();
        if !s.answer.iter().all(|c| c.is_alphabetic()) {
            return fail("G1", format!("not a single token of units: {w}"));
        }
        if !(shape.lo..=shape.hi).contains(&s.answer.len()) {
            return fail("G1", format!("length {} of {w}", s.answer.len()));
        }
        if !lex.in_band(board.tier, &s.answer) {
            return fail("G1", format!("not in the {} band: {w}", board.tier.name()));
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            if slots[i].answer == slots[j].answer {
                return fail("G1", "duplicate word");
            }
        }
    }

    // G2 runes.
    let runes = board.n_runes();
    if runes > MAX_RUNES {
        return fail("G2", format!("{runes} runes"));
    }

    // G3 one sound each.
    for i in 0..n {
        for j in i + 1..n {
            if lex.group(&slots[i].answer).contains(&slots[j].answer) {
                return fail("G3", format!("{} and {} sound alike", slots[i].answer.iter().collect::<String>(), slots[j].answer.iter().collect::<String>()));
            }
        }
    }
    if board.tier == super::types::Tier::Jr {
        for s in slots {
            if lex.has_homophone(&s.answer) {
                return fail("G3", format!("Spell Jr word {} has a sound-alike", s.answer.iter().collect::<String>()));
            }
        }
    }

    let rs = rune_sets(board);
    let name = |i: usize| slots[i].answer.iter().collect::<String>();
    // G4 linked: every word shares at least 2 distinct runes with the rest.
    for i in 0..n {
        let rest = (0..n).filter(|&j| j != i).fold(0u16, |m, j| m | rs[j]);
        if (rs[i] & rest).count_ones() < 2 {
            return fail("G4", name(i));
        }
    }
    // G5 cross-checked: every rune in a spoken word is in at least 2 words.
    for i in 0..ns {
        for r in 0..runes {
            if rs[i] >> r & 1 == 1 && rs.iter().filter(|m| **m >> r & 1 == 1).count() < 2 {
                return fail("G5", format!("{}: rune {r}", name(i)));
            }
        }
    }
    // G6 secret reachable, and never mostly decoded by one word.
    let si = n - 1;
    for r in 0..runes {
        if rs[si] >> r & 1 == 1 && !(0..si).any(|j| rs[j] >> r & 1 == 1) {
            return fail("G6a", format!("rune {r} of the secret word appears nowhere else"));
        }
    }
    let sec = &slots[si];
    for j in 0..si {
        let d = sec.runes.iter().filter(|&&r| rs[j] >> r & 1 == 1).count();
        if d * 10 > 6 * sec.runes.len() {
            return fail("G6b", format!("{} decodes {d} of {} secret cells", name(j), sec.runes.len()));
        }
    }
    // G7 silent coverage.
    let spoken_runes = (0..ns).fold(0u16, |m, i| m | rs[i]);
    if let Some((num, den)) = shape.silent_share {
        for i in ns..si {
            let d = slots[i].runes.iter().filter(|&&r| spoken_runes >> r & 1 == 1).count() as u64;
            let len = slots[i].runes.len() as u64;
            if d * (den as u64) < (num as u64) * len || d >= len {
                return fail("G7", format!("{}: {d} of {len} cells decoded by the spoken words", name(i)));
            }
        }
    }
    // G10 earned share: sum over runes of (cells in spoken words / spoken words holding it) / spoken cells.
    let spc: u128 = slots[..ns].iter().map(|s| s.runes.len() as u128).sum();
    let mut earned = Frac::new(0, 1);
    for r in 0..runes {
        let cnt: u128 = slots[..ns].iter().map(|s| s.runes.iter().filter(|&&x| x as usize == r).count() as u128).sum();
        let holders = (0..ns).filter(|&i| rs[i] >> r & 1 == 1).count() as u128;
        if holders > 0 {
            earned = earned.add(Frac::new(cnt, holders));
        }
    }
    let earned = earned.div(spc);
    if !earned.at_least(2, 5) {
        return fail("G10", format!("earned share {:.3}", earned.as_f64()));
    }
    // G11 cascade: committing one spoken word decodes at least a quarter of the other words' cells, on average.
    let mut cascade = Frac::new(0, 1);
    for i in 0..ns {
        let (mut dd, mut oc) = (0u128, 0u128);
        for j in 0..n {
            if j != i {
                oc += slots[j].runes.len() as u128;
                dd += slots[j].runes.iter().filter(|&&r| rs[i] >> r & 1 == 1).count() as u128;
            }
        }
        cascade = cascade.add(Frac::new(dd, oc));
    }
    let cascade = cascade.div(ns as u128);
    if !cascade.at_least(1, 4) {
        return fail("G11", format!("cascade {:.3}", cascade.as_f64()));
    }
    // G8 one solution.
    let sols = count_solutions(board, lex, 2);
    if sols != 1 {
        return fail("G8", format!("{} full assignments (stopped counting at 2)", if sols >= 2 { "2 or more".to_string() } else { sols.to_string() }));
    }
    // G9 step by step.
    let mut known = spoken_runes;
    let mut left: Vec<usize> = (ns..si).collect();
    let mut progress = true;
    while !left.is_empty() && progress {
        progress = false;
        for i in left.clone() {
            if solvable_given(board, lex, i, known) {
                known |= rs[i];
                left.retain(|&k| k != i);
                progress = true;
            }
        }
    }
    if !left.is_empty() {
        return fail("G9", left.iter().map(|&i| name(i)).collect::<Vec<_>>().join(","));
    }
    Ok(Metrics { runes, cells: board.cells(), earned, cascade })
}

/// G9: given the runes in `known`, exactly one validity-list word fits slot `i`.
fn solvable_given(board: &Board, lex: &Lexicon, i: usize, known: u16) -> bool {
    let slot = &board.slots[i];
    let pat = pattern(&slot.answer);
    let known_units: Vec<char> = (0..board.n_runes()).filter(|&r| known >> r & 1 == 1).map(|r| board.rune_unit[r]).collect();
    let mut hits = 0;
    for w in lex.valid_with_pattern(&pat) {
        let ok = slot.runes.iter().zip(w).all(|(&r, &u)| {
            if known >> r & 1 == 1 {
                board.rune_unit[r as usize] == u
            } else {
                !known_units.contains(&u)
            }
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

/// G8: how many full assignments exist, counting no further than `limit`.
///
/// An assignment gives every rune one unit and no two runes the same unit, so
/// that each spoken slot reads as a word in its answer's collision group and
/// each silent and secret slot reads as a validity-list word.
pub fn count_solutions(board: &Board, lex: &Lexicon, limit: u32) -> u32 {
    struct Slot<'a> {
        runes: &'a [u8],
        cands: Vec<&'a [char]>,
        spoken: bool,
    }
    let mut slots: Vec<Slot> = Vec::new();
    for s in &board.slots {
        let pat = pattern(&s.answer);
        let cands: Vec<&[char]> = if s.kind == SlotKind::Spoken {
            let g = lex.group(&s.answer);
            if g.is_empty() {
                vec![s.answer.as_slice()]
            } else {
                g.iter().filter(|w| w.len() == s.answer.len() && pattern(w) == pat).map(|w| w.as_slice()).collect()
            }
        } else {
            lex.valid_with_pattern(&pat).map(|w| w.as_slice()).collect()
        };
        slots.push(Slot { runes: &s.runes, cands, spoken: s.kind == SlotKind::Spoken });
    }
    slots.sort_by_key(|s| (!s.spoken, s.cands.len()));

    fn go(slots: &[Slot], k: usize, r2u: &mut [Option<char>; MAX_RUNES], used: &mut Vec<char>, found: &mut u32, limit: u32) {
        if *found >= limit {
            return;
        }
        if k == slots.len() {
            *found += 1;
            return;
        }
        let s = &slots[k];
        'cand: for w in &s.cands {
            // Try the word: record what it newly binds so we can undo exactly that.
            let mut bound: Vec<u8> = Vec::new();
            for (&r, &u) in s.runes.iter().zip(w.iter()) {
                match r2u[r as usize] {
                    Some(prev) if prev == u => {}
                    Some(_) => {
                        for b in &bound {
                            if let Some(c) = r2u[*b as usize].take() {
                                used.retain(|x| *x != c);
                            }
                        }
                        continue 'cand;
                    }
                    None => {
                        if used.contains(&u) {
                            for b in &bound {
                                if let Some(c) = r2u[*b as usize].take() {
                                    used.retain(|x| *x != c);
                                }
                            }
                            continue 'cand;
                        }
                        r2u[r as usize] = Some(u);
                        used.push(u);
                        bound.push(r);
                    }
                }
            }
            go(slots, k + 1, r2u, used, found, limit);
            for b in &bound {
                if let Some(c) = r2u[*b as usize].take() {
                    used.retain(|x| *x != c);
                }
            }
            if *found >= limit {
                return;
            }
        }
    }
    let mut found = 0;
    go(&slots, 0, &mut [None; MAX_RUNES], &mut Vec::new(), &mut found, limit);
    found
}
