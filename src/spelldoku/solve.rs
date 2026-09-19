//! Uniqueness and difficulty (I1, I2, F8).
//!
//! Two solvers. `count_solutions` is exhaustive and only answers "how many
//! solutions, up to a cap" -- it proves uniqueness. `grade` is the human ladder:
//! it solves with named techniques only, cheapest first, and reports the
//! hardest one the solve NEEDED. A board it cannot finish needs guessing, which
//! no tier allows (I2). Difficulty is never graded by backtracking count.
//!
//! Both take `restrict`: a per-cell candidate mask, which is how fragment clues
//! (F2) enter as real constraints rather than decoration.

use super::geo::{bit, Geo};

/// The technique ladder, cheapest first (F8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tech {
    /// Naked and hidden singles.
    Single = 0,
    /// Naked and hidden pairs and triples.
    Subset = 1,
    /// Pointing and box-line reduction.
    Intersection = 2,
    /// X-wing and swordfish.
    Fish = 3,
}

fn popcount(m: u16) -> u32 {
    m.count_ones()
}

fn only(m: u16) -> u8 {
    m.trailing_zeros() as u8
}

/// Count solutions up to `cap`. `fixed[i] > 0` is a placed value; `restrict[i]`
/// narrows an empty cell's candidates (fragments). Deterministic: always
/// branches on the first cell with the fewest candidates, values low to high.
pub fn count_solutions(geo: &Geo, fixed: &[u8], restrict: &[u16], cap: usize) -> usize {
    let mut vals = fixed.to_vec();
    let mut count = 0;
    search(geo, &mut vals, restrict, cap, &mut count);
    count
}

fn candidates(geo: &Geo, vals: &[u8], restrict: &[u16], i: usize) -> u16 {
    let mut m = geo.full() & restrict[i];
    for &p in &geo.peers[i] {
        if vals[p] > 0 {
            m &= !bit(vals[p]);
        }
    }
    m
}

fn search(geo: &Geo, vals: &mut Vec<u8>, restrict: &[u16], cap: usize, count: &mut usize) {
    if *count >= cap {
        return;
    }
    let mut best: Option<(usize, u16)> = None;
    for i in 0..vals.len() {
        if vals[i] > 0 {
            continue;
        }
        let m = candidates(geo, vals, restrict, i);
        let c = popcount(m);
        if c == 0 {
            return;
        }
        if best.map_or(true, |(_, bm)| c < popcount(bm)) {
            best = Some((i, m));
            if c == 1 {
                break;
            }
        }
    }
    let Some((i, m)) = best else {
        *count += 1;
        return;
    };
    for v in 1..=geo.size.n as u8 {
        if m & bit(v) != 0 {
            vals[i] = v;
            search(geo, vals, restrict, cap, count);
            vals[i] = 0;
            if *count >= cap {
                return;
            }
        }
    }
}

/// The solver's working state.
struct State<'a> {
    geo: &'a Geo,
    vals: Vec<u8>,
    cands: Vec<u16>,
}

impl<'a> State<'a> {
    fn new(geo: &'a Geo, fixed: &[u8], restrict: &[u16]) -> Self {
        let mut s = State { geo, vals: vec![0; geo.cells()], cands: vec![0; geo.cells()] };
        for i in 0..geo.cells() {
            if fixed[i] == 0 {
                s.cands[i] = geo.full() & restrict[i];
            }
        }
        for i in 0..geo.cells() {
            if fixed[i] > 0 {
                s.place(i, fixed[i]);
            }
        }
        s
    }

    fn place(&mut self, i: usize, v: u8) {
        self.vals[i] = v;
        self.cands[i] = 0;
        for k in 0..self.geo.peers[i].len() {
            let p = self.geo.peers[i][k];
            self.cands[p] &= !bit(v);
        }
    }

    fn solved(&self) -> bool {
        self.vals.iter().all(|&v| v > 0)
    }

    fn broken(&self) -> bool {
        (0..self.vals.len()).any(|i| self.vals[i] == 0 && self.cands[i] == 0)
    }

    fn singles(&mut self) -> bool {
        for i in 0..self.vals.len() {
            if self.vals[i] == 0 && popcount(self.cands[i]) == 1 {
                let v = only(self.cands[i]);
                self.place(i, v);
                return true;
            }
        }
        for u in 0..self.geo.units.len() {
            for v in 1..=self.geo.size.n as u8 {
                let mut spot = None;
                let mut n = 0;
                for &i in &self.geo.units[u] {
                    if self.vals[i] == v {
                        n = 2; // already placed in this unit
                        break;
                    }
                    if self.vals[i] == 0 && self.cands[i] & bit(v) != 0 {
                        n += 1;
                        spot = Some(i);
                    }
                }
                if n == 1 {
                    self.place(spot.unwrap(), v);
                    return true;
                }
            }
        }
        false
    }

    fn subsets(&mut self) -> bool {
        let n = self.geo.size.n;
        for k in 2..=3usize {
            for u in 0..self.geo.units.len() {
                let empty: Vec<usize> =
                    self.geo.units[u].iter().copied().filter(|&i| self.vals[i] == 0).collect();
                if empty.len() <= k {
                    continue;
                }
                // Naked: k cells whose candidates together are k values.
                for combo in combos(&empty, k) {
                    let union = combo.iter().fold(0u16, |m, &i| m | self.cands[i]);
                    if popcount(union) as usize == k {
                        let mut changed = false;
                        for &j in &empty {
                            if !combo.contains(&j) && self.cands[j] & union != 0 {
                                self.cands[j] &= !union;
                                changed = true;
                            }
                        }
                        if changed {
                            return true;
                        }
                    }
                }
                // Hidden: k values that fit only k cells.
                let values: Vec<u8> = (1..=n as u8)
                    .filter(|&v| empty.iter().any(|&i| self.cands[i] & bit(v) != 0))
                    .collect();
                if values.len() <= k {
                    continue;
                }
                for vs in combos(&values, k) {
                    let mask = vs.iter().fold(0u16, |m, &v| m | bit(v));
                    let cells: Vec<usize> =
                        empty.iter().copied().filter(|&i| self.cands[i] & mask != 0).collect();
                    if cells.len() == k {
                        let mut changed = false;
                        for &i in &cells {
                            if self.cands[i] & !mask != 0 {
                                self.cands[i] &= mask;
                                changed = true;
                            }
                        }
                        if changed {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn intersections(&mut self) -> bool {
        let n = self.geo.size.n;
        let boxes = 2 * n..3 * n;
        for b in boxes {
            for v in 1..=n as u8 {
                let cells: Vec<usize> = self.geo.units[b]
                    .iter()
                    .copied()
                    .filter(|&i| self.vals[i] == 0 && self.cands[i] & bit(v) != 0)
                    .collect();
                if cells.len() < 2 {
                    continue;
                }
                // Pointing: all in one row or column -> clear the rest of that line.
                for line in [0usize, 1] {
                    let l = self.geo.cell_units[cells[0]][line];
                    if cells.iter().all(|&i| self.geo.cell_units[i][line] == l) {
                        let mut changed = false;
                        for &j in &self.geo.units[l] {
                            if self.geo.cell_units[j][2] != b && self.vals[j] == 0 && self.cands[j] & bit(v) != 0 {
                                self.cands[j] &= !bit(v);
                                changed = true;
                            }
                        }
                        if changed {
                            return true;
                        }
                    }
                }
            }
        }
        // Box-line: all of a line's candidates in one box -> clear the rest of the box.
        for l in 0..2 * n {
            for v in 1..=n as u8 {
                let cells: Vec<usize> = self.geo.units[l]
                    .iter()
                    .copied()
                    .filter(|&i| self.vals[i] == 0 && self.cands[i] & bit(v) != 0)
                    .collect();
                if cells.len() < 2 {
                    continue;
                }
                let b = self.geo.cell_units[cells[0]][2];
                if cells.iter().all(|&i| self.geo.cell_units[i][2] == b) {
                    let mut changed = false;
                    for &j in &self.geo.units[b] {
                        if !self.geo.units[l].contains(&j) && self.vals[j] == 0 && self.cands[j] & bit(v) != 0 {
                            self.cands[j] &= !bit(v);
                            changed = true;
                        }
                    }
                    if changed {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn fish(&mut self) -> bool {
        let n = self.geo.size.n;
        for k in 2..=3usize {
            for (base, cover) in [(0usize, n), (n, 0usize)] {
                // base lines are rows (or columns); cover lines the other way.
                for v in 1..=n as u8 {
                    let lines: Vec<(usize, Vec<usize>)> = (0..n)
                        .filter_map(|l| {
                            let pos: Vec<usize> = self.geo.units[base + l]
                                .iter()
                                .copied()
                                .filter(|&i| self.vals[i] == 0 && self.cands[i] & bit(v) != 0)
                                .map(|i| if base == 0 { i % n } else { i / n })
                                .collect();
                            (pos.len() >= 2 && pos.len() <= k).then_some((l, pos))
                        })
                        .collect();
                    if lines.len() < k {
                        continue;
                    }
                    let idx: Vec<usize> = (0..lines.len()).collect();
                    for combo in combos(&idx, k) {
                        let mut covers: Vec<usize> = Vec::new();
                        for &c in &combo {
                            for &p in &lines[c].1 {
                                if !covers.contains(&p) {
                                    covers.push(p);
                                }
                            }
                        }
                        if covers.len() != k {
                            continue;
                        }
                        let chosen: Vec<usize> = combo.iter().map(|&c| lines[c].0).collect();
                        let mut changed = false;
                        for &cl in &covers {
                            for &j in &self.geo.units[cover + cl] {
                                let line = if base == 0 { j / n } else { j % n };
                                if !chosen.contains(&line) && self.vals[j] == 0 && self.cands[j] & bit(v) != 0 {
                                    self.cands[j] &= !bit(v);
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

fn combos<T: Copy>(items: &[T], k: usize) -> Vec<Vec<T>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    fn go<T: Copy>(items: &[T], k: usize, start: usize, cur: &mut Vec<T>, out: &mut Vec<Vec<T>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..items.len() {
            cur.push(items[i]);
            go(items, k, i + 1, cur, out);
            cur.pop();
        }
    }
    go(items, k, 0, &mut cur, &mut out);
    out
}

/// The hardest technique the solve needs, or None when the ladder cannot finish
/// it (the board would need guessing, which no tier allows).
pub fn grade(geo: &Geo, fixed: &[u8], restrict: &[u16]) -> Option<Tech> {
    let mut s = State::new(geo, fixed, restrict);
    let mut hardest = Tech::Single;
    loop {
        if s.broken() {
            return None;
        }
        if s.solved() {
            return Some(hardest);
        }
        if s.singles() {
            continue;
        }
        if s.subsets() {
            hardest = hardest.max(Tech::Subset);
            continue;
        }
        if s.intersections() {
            hardest = hardest.max(Tech::Intersection);
            continue;
        }
        if s.fish() {
            hardest = hardest.max(Tech::Fish);
            continue;
        }
        return None;
    }
}

/// The first cell the cheapest technique can place, and that technique -- the
/// hint ladder's levels 1 and 2 (F9). Never a value, never a letter.
pub fn next_step(geo: &Geo, fixed: &[u8], restrict: &[u16]) -> Option<(usize, Tech)> {
    let mut s = State::new(geo, fixed, restrict);
    let mut hardest = Tech::Single;
    for _ in 0..(geo.cells() * geo.size.n * 4) {
        let before = s.vals.clone();
        if s.singles() {
            let placed = (0..s.vals.len()).find(|&i| before[i] == 0 && s.vals[i] > 0)?;
            return Some((placed, hardest));
        }
        if s.subsets() {
            hardest = hardest.max(Tech::Subset);
        } else if s.intersections() {
            hardest = hardest.max(Tech::Intersection);
        } else if s.fish() {
            hardest = hardest.max(Tech::Fish);
        } else {
            return None;
        }
    }
    None
}
