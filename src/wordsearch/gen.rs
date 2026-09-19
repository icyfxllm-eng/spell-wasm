//! The Spell Search grid (F-S1, F-S2, F-S3, F-S5).
//!
//! A grid is a function of `(seed, Input)`. Every grid this returns has been
//! checked cell by cell: each target and decoy reads exactly once in all eight
//! directions (I1, I9), and no filler run spells a bank or blocklisted word
//! except inside a word placed on purpose (I3). A layout that cannot be made
//! clean is thrown away and the next seed is tried; nothing is ever served
//! unchecked.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::lexicon::{graphemes, Lexicon};
use crate::spelldoku::rng::{fnv, Rng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Jr,
    Easy,
    Medium,
    Hard,
    Expert,
}

/// → ↓ ↘ ↗, then the four backwards directions. A tier uses a prefix.
pub const DIRS: [(i32, i32); 8] = [(0, 1), (1, 0), (1, 1), (-1, 1), (0, -1), (-1, 0), (-1, -1), (1, -1)];

impl Tier {
    pub const ALL: [Tier; 5] = [Tier::Jr, Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert];

    pub fn id(self) -> &'static str {
        match self {
            Tier::Jr => "jr",
            Tier::Easy => "easy",
            Tier::Medium => "medium",
            Tier::Hard => "hard",
            Tier::Expert => "expert",
        }
    }

    pub fn from_id(s: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.id() == s)
    }

    /// Side of the square grid. Big enough for the tier's longest bank words
    /// and still a finger-sized cell on an SE-width phone.
    pub fn size(self) -> usize {
        match self {
            Tier::Jr => 7,
            Tier::Easy => 9,
            Tier::Medium => 10,
            Tier::Hard => 11,
            Tier::Expert => 12,
        }
    }

    /// Targets per puzzle. Hard and Expert carry fewer, because each target
    /// brings one or two decoys that need room too.
    pub fn targets(self) -> usize {
        match self {
            Tier::Jr => 5,
            Tier::Easy | Tier::Medium => 8,
            Tier::Hard => 6,
            Tier::Expert => 4,
        }
    }

    /// The longest bank word a tier serves. Expert stops short of its grid
    /// side: twelve words of twelve letters do not fit in 144 cells.
    pub fn max_word(self) -> usize {
        match self {
            Tier::Jr => 6,
            Tier::Expert => 10,
            t => t.size(),
        }
    }

    /// F-S3: the directions a word may run.
    pub fn dirs(self) -> &'static [(i32, i32)] {
        match self {
            Tier::Jr => &DIRS[..2],
            Tier::Easy => &DIRS[..3],
            Tier::Medium => &DIRS[..4],
            Tier::Hard | Tier::Expert => &DIRS,
        }
    }

    /// F-S3: decoys for `targets` targets -- none, 1 per 4, 1 per 2, 1 per
    /// target, 2 per target.
    pub fn decoy_count(self, targets: usize) -> usize {
        match self {
            Tier::Jr => 0,
            Tier::Easy => targets / 4,
            Tier::Medium => targets / 2,
            Tier::Hard => targets,
            Tier::Expert => targets * 2,
        }
    }

    /// The most decoys any one target carries.
    pub fn decoys_per_target(self) -> usize {
        match self {
            Tier::Expert => 2,
            Tier::Jr => 0,
            _ => 1,
        }
    }

    pub fn slow_replay(self) -> bool {
        self != Tier::Expert
    }

    /// F-X4: the paid definition hint is Easy and Medium only.
    pub fn definition_hint(self) -> bool {
        matches!(self, Tier::Easy | Tier::Medium)
    }

    /// F-S4: Lock It In is optional below Hard.
    pub fn lock_in_required(self) -> bool {
        matches!(self, Tier::Hard | Tier::Expert)
    }

    pub fn weighted_filler(self) -> bool {
        self == Tier::Expert
    }

    /// Which bank tier the targets come from. Jr is the Easy bank's short words.
    pub fn bank_tier(self) -> &'static str {
        match self {
            Tier::Jr | Tier::Easy => "easy",
            Tier::Medium => "medium",
            Tier::Hard => "hard",
            Tier::Expert => "expert",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    /// Folded (NFC, lowercase).
    pub word: String,
    /// Cells in reading order, row-major indices.
    pub cells: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub tier: Tier,
    pub size: usize,
    /// One grapheme per cell, row-major.
    pub grid: Vec<String>,
    /// In slot order, as the Input listed them.
    pub targets: Vec<Placed>,
    /// Each decoy with the index of the target it misspells.
    pub decoys: Vec<(Placed, usize)>,
}

pub struct Input<'a> {
    pub tier: Tier,
    pub size: usize,
    /// Folded target words, in slot order.
    pub targets: Vec<String>,
    /// Decoy candidates per target, index-aligned. Empty where there are none.
    pub decoys: Vec<Vec<String>>,
    pub lex: &'a Lexicon,
}

const ATTEMPTS: u64 = 48;
const REPAIR_ROUNDS: usize = 80;

pub fn generate(seed: u64, input: &Input) -> Option<Puzzle> {
    // The blocklist check is the costly part of a scan (a microsecond or two
    // a run, thousands of runs a scan), and repair rescans mostly-unchanged
    // lines: remember every answer for the whole generation.
    let mut memo: HashMap<String, bool> = HashMap::new();
    (0..ATTEMPTS).find_map(|a| build(&mut Rng::new(seed.wrapping_add(a.wrapping_mul(0x9E37_79B9_7F4A_7C15))), input, &mut memo))
}

/// How many decoys this input will carry: the tier's count, or fewer when the
/// targets do not have that many between them (a My Words or Daily puzzle).
pub fn decoys_wanted(input: &Input) -> usize {
    let n = input.targets.len();
    if n == 0 {
        return 0;
    }
    let cap = input.tier.decoys_per_target();
    let available: usize = input.decoys.iter().map(|d| d.len().min(cap)).sum();
    input.tier.decoy_count(n).min(available)
}

fn spots(grid: &[Option<String>], n: usize, g: &[String], dirs: &[(i32, i32)]) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let len = g.len() as i32;
    for r in 0..n as i32 {
        for c in 0..n as i32 {
            for &(dr, dc) in dirs {
                let (er, ec) = (r + dr * (len - 1), c + dc * (len - 1));
                if er < 0 || ec < 0 || er >= n as i32 || ec >= n as i32 {
                    continue;
                }
                let cells: Vec<usize> = (0..len).map(|k| ((r + dr * k) * n as i32 + c + dc * k) as usize).collect();
                if cells.iter().zip(g).all(|(&i, ch)| grid[i].as_ref().map_or(true, |x| x == ch)) {
                    out.push(cells);
                }
            }
        }
    }
    out
}

fn place(grid: &mut [Option<String>], cells: &[usize], g: &[String]) {
    for (&i, ch) in cells.iter().zip(g) {
        grid[i] = Some(ch.clone());
    }
}

/// F-S5: filler weights. Bank letter frequency; at Expert, half the weight moves
/// to the targets' own letters.
fn filler_weights(input: &Input) -> Vec<(String, u64)> {
    let freq = &input.lex.freq;
    if !input.tier.weighted_filler() {
        return freq.clone();
    }
    let mut own: BTreeMap<&str, u64> = BTreeMap::new();
    for t in &input.targets {
        for g in graphemes(t) {
            if let Some((k, _)) = freq.iter().find(|(k, _)| *k == g) {
                *own.entry(k.as_str()).or_default() += 1;
            }
        }
    }
    let f_sum: u64 = freq.iter().map(|(_, w)| w).sum();
    let t_sum: u64 = own.values().sum::<u64>().max(1);
    freq.iter().map(|(k, w)| (k.clone(), w * t_sum + own.get(k.as_str()).copied().unwrap_or(0) * f_sum)).collect()
}

fn sample(rng: &mut Rng, weights: &[(String, u64)], total: u64) -> String {
    let mut x = rng.next_u64() % total;
    for (k, w) in weights {
        if x < *w {
            return k.clone();
        }
        x -= w;
    }
    weights.last().map(|(k, _)| k.clone()).unwrap_or_default()
}

fn build(rng: &mut Rng, input: &Input, memo: &mut HashMap<String, bool>) -> Option<Puzzle> {
    let n = input.size;
    let dirs = input.tier.dirs();
    let mut grid: Vec<Option<String>> = vec![None; n * n];
    let tg: Vec<Vec<String>> = input.targets.iter().map(|t| graphemes(t)).collect();
    if tg.is_empty() || tg.iter().any(|g| g.len() < 3 || g.len() > n) {
        return None;
    }

    // Longest first: they have the fewest places to go.
    let mut order: Vec<usize> = (0..tg.len()).collect();
    order.sort_by_key(|&i| (Reverse(tg[i].len()), i));
    let mut target_cells: Vec<Vec<usize>> = vec![Vec::new(); tg.len()];
    for i in order {
        let s = spots(&grid, n, &tg[i], dirs);
        if s.is_empty() {
            return None;
        }
        let cells = s[rng.below(s.len())].clone();
        place(&mut grid, &cells, &tg[i]);
        target_cells[i] = cells;
    }

    // F-S2: decoys, at most `cap` per target, never the same one twice, never a
    // target or a target reversed (I2, I9).
    let want = decoys_wanted(input);
    let cap = input.tier.decoys_per_target().max(1);
    let mut used: HashSet<String> = input.targets.iter().cloned().collect();
    let reversed: HashSet<String> = tg.iter().map(|g| g.iter().rev().cloned().collect::<String>()).collect();
    let mut decoys: Vec<(Placed, usize)> = Vec::new();
    let mut who: Vec<usize> = (0..tg.len()).collect();
    rng.shuffle(&mut who);
    for &t in &who {
        let mut got = 0;
        let mut cands = input.decoys.get(t).cloned().unwrap_or_default();
        rng.shuffle(&mut cands);
        for d in cands {
            if got == cap || decoys.len() == want {
                break;
            }
            let g = graphemes(&d);
            let rev: String = g.iter().rev().cloned().collect();
            if used.contains(&d) || used.contains(&rev) || reversed.contains(&d) || g.len() > n {
                continue;
            }
            // Inside another word, or around one, either way: it would read twice.
            let tangled = used.iter().any(|u| u.contains(d.as_str()) || d.contains(u.as_str()) || u.contains(&rev) || rev.contains(u.as_str()));
            if tangled {
                continue;
            }
            let s = spots(&grid, n, &g, dirs);
            if s.is_empty() {
                continue;
            }
            let cells = s[rng.below(s.len())].clone();
            place(&mut grid, &cells, &g);
            used.insert(d.clone());
            decoys.push((Placed { word: d, cells }, t));
            got += 1;
        }
    }
    if decoys.len() < want {
        return None;
    }

    // F-S5: filler, then repair until I1, I3 and I9 hold.
    let weights = filler_weights(input);
    let total: u64 = weights.iter().map(|(_, w)| w).sum();
    if total == 0 {
        return None;
    }
    let fixed: Vec<bool> = grid.iter().map(Option::is_some).collect();
    let mut cells: Vec<String> = grid.into_iter().map(|c| c.unwrap_or_else(|| sample(rng, &weights, total))).collect();
    let targets: Vec<Placed> =
        input.targets.iter().zip(target_cells).map(|(w, c)| Placed { word: w.clone(), cells: c }).collect();
    for _ in 0..REPAIR_ROUNDS {
        let bad = violations(&cells, n, &targets, &decoys, input.lex, memo);
        if bad.is_empty() {
            return Some(Puzzle { tier: input.tier, size: n, grid: cells, targets, decoys });
        }
        for span in bad {
            let free: Vec<usize> = span.into_iter().filter(|&i| !fixed[i]).collect();
            if free.is_empty() {
                return None; // placed words alone break the rules: new layout
            }
            let i = free[rng.below(free.len())];
            cells[i] = sample(rng, &weights, total);
        }
    }
    None
}

/// Every straight line of 3 or more cells, along the four axes.
pub fn lines(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let n_i = n as i32;
    let mut walk = |mut r: i32, mut c: i32, dr: i32, dc: i32| {
        let mut v = Vec::new();
        while r >= 0 && c >= 0 && r < n_i && c < n_i {
            v.push((r * n_i + c) as usize);
            r += dr;
            c += dc;
        }
        if v.len() >= 3 {
            out.push(v);
        }
    };
    for i in 0..n_i {
        walk(i, 0, 0, 1);
        walk(0, i, 1, 0);
        walk(i, 0, 1, 1);
        walk(0, i, 1, -1);
    }
    for i in 1..n_i {
        walk(0, i, 1, 1);
        walk(i, n_i - 1, 1, -1);
    }
    out
}

/// Spans breaking I1, I3 or I9. Empty means the grid is clean.
///
/// - Each target and decoy reads exactly once, in all 8 directions (I1, I9).
/// - No run of 3+ cells spells a bank word of 4+ cells or a blocklisted word,
///   unless it lies wholly inside one placed word (I3).
pub fn violations(
    cells: &[String],
    n: usize,
    targets: &[Placed],
    decoys: &[(Placed, usize)],
    lex: &Lexicon,
    memo: &mut HashMap<String, bool>,
) -> Vec<Vec<usize>> {
    let placed: Vec<&Placed> = targets.iter().chain(decoys.iter().map(|(p, _)| p)).collect();
    let index: HashMap<&str, usize> = placed.iter().enumerate().map(|(i, p)| (p.word.as_str(), i)).collect();
    let mut seen: Vec<Vec<Vec<usize>>> = vec![Vec::new(); placed.len()];
    let mut bad: Vec<Vec<usize>> = Vec::new();
    let inside = |span: &[usize]| placed.iter().any(|p| span.iter().all(|c| p.cells.contains(c)));
    for line in lines(n) {
        for rev in [false, true] {
            let seq: Vec<usize> = if rev { line.iter().rev().copied().collect() } else { line.clone() };
            for i in 0..seq.len() {
                let mut s = String::new();
                for j in i..seq.len() {
                    s.push_str(&cells[seq[j]]);
                    let k = j - i + 1;
                    if k < 3 {
                        continue;
                    }
                    let span = &seq[i..=j];
                    if let Some(&w) = index.get(s.as_str()) {
                        let mut key = span.to_vec();
                        key.sort_unstable();
                        if !seen[w].contains(&key) {
                            seen[w].push(key);
                        }
                        continue;
                    }
                    let flagged = (k >= 4 && lex.words.contains(&s))
                        || *memo.entry(s.clone()).or_insert_with(|| crate::profanity::is_blocked(&s));
                    if flagged && !inside(span) {
                        bad.push(span.to_vec());
                    }
                }
            }
        }
    }
    for (w, found) in seen.iter().enumerate() {
        let mut own = placed[w].cells.clone();
        own.sort_unstable();
        for f in found {
            if *f != own {
                bad.push(f.clone());
            }
        }
    }
    bad
}

/// What a straight drag from cell `a` to cell `b` covers, in order. `None` when
/// the two cells are not on one of the eight lines.
pub fn path(n: usize, a: usize, b: usize) -> Option<Vec<usize>> {
    let (ar, ac, br, bc) = ((a / n) as i32, (a % n) as i32, (b / n) as i32, (b % n) as i32);
    let (dr, dc) = (br - ar, bc - ac);
    if dr != 0 && dc != 0 && dr.abs() != dc.abs() {
        return None;
    }
    let steps = dr.abs().max(dc.abs());
    let (sr, sc) = (dr.signum(), dc.signum());
    Some((0..=steps).map(|k| ((ar + sr * k) * n as i32 + ac + sc * k) as usize).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Target(usize),
    Decoy(usize),
    Miss,
}

/// A drag lands on a word when it covers exactly that word's cells, either way.
pub fn judge(p: &Puzzle, drag: &[usize]) -> Hit {
    let same = |cells: &[usize]| cells.len() == drag.len() && (cells == drag || cells.iter().rev().eq(drag.iter()));
    if let Some(i) = p.targets.iter().position(|t| same(&t.cells)) {
        return Hit::Target(i);
    }
    if let Some(j) = p.decoys.iter().position(|(d, _)| same(&d.cells)) {
        return Hit::Decoy(j);
    }
    Hit::Miss
}

/// The canonical form of a grid, hashed: tier, size, cells and every placement.
/// Equal on every platform for equal inputs (I8).
pub fn hash(p: &Puzzle) -> u64 {
    let mut s = format!("{}|{}|{}", p.tier.id(), p.size, p.grid.join("\u{1f}"));
    for t in &p.targets {
        s.push_str(&format!("|t:{}:{:?}", t.word, t.cells));
    }
    for (d, t) in &p.decoys {
        s.push_str(&format!("|d:{}:{}:{:?}", d.word, t, d.cells));
    }
    fnv(s.as_bytes())
}
