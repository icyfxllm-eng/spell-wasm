//! CC-WORDGRID Phase B — the criss-cross layout (D1, F-C2, F-C4; I5, I7).
//!
//! A freeform criss-cross of the list's own words: no filler words, no dense
//! American grid. The first word is laid down, and every later word must cross
//! one already placed, so the finished grid is one connected component by
//! construction (I7). A crossing cell holds one NFC grapheme that both words
//! share (I5, F-C4), so an accented letter never crosses its bare form.
//!
//! Where the crossings land is the hint system (F-C2): at Jr, Easy and Medium
//! the generator prefers crossings ON trap positions, so a hard letter is
//! settled by an easier word; at Hard and Expert it prefers them OFF, leaving
//! the trap to the player's ear.

use std::collections::HashMap;

use super::super::spelldoku::rng::Rng;
use super::super::wordsearch::lexicon::graphemes;

/// One placed word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub word: String,
    /// Cell indices in the finished grid, in reading order.
    pub cells: Vec<usize>,
    pub across: bool,
    /// Crossword number of its first cell.
    pub number: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// One grapheme per filled cell, None where the grid is blank.
    pub cells: Vec<Option<String>>,
    pub words: Vec<Placed>,
    /// F-C6: the cells whose letters spell the keystone, in order.
    pub shaded: Vec<usize>,
    pub keystone: Option<String>,
}

impl Grid {
    /// The crossings: (cell, the two words that share it).
    pub fn crossings(&self) -> Vec<(usize, usize, usize)> {
        let mut out = Vec::new();
        for (a, wa) in self.words.iter().enumerate() {
            for (b, wb) in self.words.iter().enumerate().skip(a + 1) {
                if wa.across == wb.across {
                    continue;
                }
                for &c in &wa.cells {
                    if wb.cells.contains(&c) {
                        out.push((c, a, b));
                    }
                }
            }
        }
        out
    }

    /// Where a crossing cell falls inside a word: its index in that word.
    pub fn index_in(&self, word: usize, cell: usize) -> Option<usize> {
        self.words[word].cells.iter().position(|&c| c == cell)
    }
}

/// A word being laid out, in unbounded coordinates.
#[derive(Clone)]
struct Lay {
    word: String,
    g: Vec<String>,
    r: i32,
    c: i32,
    across: bool,
}

impl Lay {
    fn cell(&self, k: usize) -> (i32, i32) {
        if self.across {
            (self.r, self.c + k as i32)
        } else {
            (self.r + k as i32, self.c)
        }
    }
}

/// Can this word sit here? Crossword rules: a cell either matches or is empty,
/// the ends stay clear, and a non-crossing cell has no neighbour beside it, so
/// no two words ever run alongside each other and spell something unintended.
fn fits(filled: &HashMap<(i32, i32), String>, lay: &Lay) -> Option<usize> {
    let mut crossings = 0;
    let (dr, dc) = if lay.across { (0, 1) } else { (1, 0) };
    let before = (lay.r - dr, lay.c - dc);
    let end = lay.cell(lay.g.len() - 1);
    let after = (end.0 + dr, end.1 + dc);
    if filled.contains_key(&before) || filled.contains_key(&after) {
        return None;
    }
    for (k, ch) in lay.g.iter().enumerate() {
        let p = lay.cell(k);
        match filled.get(&p) {
            Some(x) if x == ch => crossings += 1,
            Some(_) => return None,
            None => {
                // The two cells beside this one, across the word's direction.
                let (sr, sc) = if lay.across { (1, 0) } else { (0, 1) };
                if filled.contains_key(&(p.0 + sr, p.1 + sc)) || filled.contains_key(&(p.0 - sr, p.1 - sc)) {
                    return None;
                }
            }
        }
    }
    (crossings > 0).then_some(crossings)
}

/// The largest grid a phone shows comfortably.
pub const MAX_SIDE: usize = 13;

/// Lay the words out. `traps` gives the positions of a word that its language's
/// confusion list edits; `on_traps` is whether this tier wants crossings there
/// (F-C2). Returns the grid and the words that did not fit.
pub fn build(
    words: &[String],
    rng: &mut Rng,
    on_traps: bool,
    traps: &dyn Fn(&str) -> Vec<usize>,
    tells: &dyn Fn(&str) -> Option<Vec<usize>>,
) -> Option<(Grid, Vec<String>)> {
    // Longest first -- they have the fewest places to go -- with the order
    // shuffled among words of equal length, so repeated attempts differ.
    let mut order: Vec<&String> = words.iter().collect();
    rng.shuffle(&mut order);
    order.sort_by_key(|w| std::cmp::Reverse(graphemes(w).len()));
    let first = order.first().copied()?;
    let mut filled: HashMap<(i32, i32), String> = HashMap::new();
    let mut placed: Vec<Lay> = Vec::new();
    let put = |filled: &mut HashMap<(i32, i32), String>, placed: &mut Vec<Lay>, lay: Lay| {
        for (k, ch) in lay.g.iter().enumerate() {
            filled.insert(lay.cell(k), ch.clone());
        }
        placed.push(lay);
    };
    put(
        &mut filled,
        &mut placed,
        Lay { word: first.clone(), g: graphemes(first), r: 0, c: 0, across: true },
    );
    // Two passes: a word that found no anchor may fit once later words have
    // widened the grid.
    let mut left = Vec::new();
    // (min row, max row, min col, max col) of what is placed so far.
    let mut bounds = (0i32, 0i32, 0i32, graphemes(first).len() as i32 - 1);
    let rest: Vec<&String> = order.into_iter().skip(1).collect();
    let mut queue: Vec<&String> = rest.clone();
    for pass in 0..3 {
        let round = std::mem::take(&mut queue);
        for w in round {
        let g = graphemes(w);
        let tw = traps(w);
        let mut best: Vec<(i32, Lay)> = Vec::new();
        for anchor in &placed {
            for (ai, ach) in anchor.g.iter().enumerate() {
                for (wi, wch) in g.iter().enumerate() {
                    if ach != wch {
                        continue;
                    }
                    let (ar, ac) = anchor.cell(ai);
                    let lay = if anchor.across {
                        Lay { word: w.clone(), g: g.clone(), r: ar - wi as i32, c: ac, across: false }
                    } else {
                        Lay { word: w.clone(), g: g.clone(), r: ar, c: ac - wi as i32, across: true }
                    };
                    let Some(crossings) = fits(&filled, &lay) else { continue };
                    // F-C2: a crossing on a trap position of either word is the
                    // preference the tier asks for; one more crossing is worth
                    // a little either way.
                    let trap = tw.contains(&wi) || traps(&anchor.word).contains(&ai);
                    let mut score = crossings as i32 * 2;
                    score += if trap == on_traps { 6 } else { -6 };
                    // Keep the grid compact: it has to fit MAX_SIDE, and a
                    // sprawling layout is the main reason a list fails to make
                    // a crossword at all.
                    let (er, ec) = lay.cell(lay.g.len() - 1);
                    let grow = (lay.r.min(er).min(bounds.0) - bounds.0).abs()
                        + (lay.r.max(er).max(bounds.1) - bounds.1).abs()
                        + (lay.c.min(ec).min(bounds.2) - bounds.2).abs()
                        + (lay.c.max(ec).max(bounds.3) - bounds.3).abs();
                    score -= grow.min(14);
                    // A crossing near the middle of either word interlocks the
                    // grid; crossing at the ends walks it off in a staircase.
                    let mid = |i: usize, len: usize| (len as i32 - 1 - (2 * i as i32 - len as i32 + 1).abs()) / 2;
                    score += mid(wi, g.len()) + mid(ai, anchor.g.len());
                    // F-C3, first choice: a word that sounds like another wants
                    // a crossing where the spellings differ.
                    for (w, i) in [(w.as_str(), wi), (anchor.word.as_str(), ai)] {
                        if let Some(pos) = tells(w) {
                            score += if pos.contains(&i) { 40 } else { -4 };
                        }
                    }
                    let side_r = (lay.r.min(er).min(bounds.0)..=lay.r.max(er).max(bounds.1)).count();
                    let side_c = (lay.c.min(ec).min(bounds.2)..=lay.c.max(ec).max(bounds.3)).count();
                    if side_r > MAX_SIDE || side_c > MAX_SIDE {
                        continue; // it would not fit the screen
                    }
                    best.push((score, lay));
                }
            }
        }
        if best.is_empty() {
            if pass < 2 {
                queue.push(w);
            } else {
                left.push(w.clone());
            }
            continue;
        }
        let top = best.iter().map(|(s, _)| *s).max().unwrap();
        let mut ties: Vec<Lay> = best.into_iter().filter(|(s, _)| *s == top).map(|(_, l)| l).collect();
        ties.sort_by_key(|l| (l.r, l.c, l.across));
        let lay = ties[rng.below(ties.len())].clone();
        let (er, ec) = lay.cell(lay.g.len() - 1);
        bounds = (
            bounds.0.min(lay.r).min(er),
            bounds.1.max(lay.r).max(er),
            bounds.2.min(lay.c).min(ec),
            bounds.3.max(lay.c).max(ec),
        );
        put(&mut filled, &mut placed, lay);
        }
    }

    // Normalize to a grid.
    let (r0, c0) = (filled.keys().map(|k| k.0).min()?, filled.keys().map(|k| k.1).min()?);
    let (r1, c1) = (filled.keys().map(|k| k.0).max()?, filled.keys().map(|k| k.1).max()?);
    let (h, w) = ((r1 - r0 + 1) as usize, (c1 - c0 + 1) as usize);
    if h > MAX_SIDE || w > MAX_SIDE {
        return None;
    }
    let at = |r: i32, c: i32| ((r - r0) as usize) * w + (c - c0) as usize;
    let mut cells = vec![None; w * h];
    for (&(r, c), ch) in &filled {
        cells[at(r, c)] = Some(ch.clone());
    }
    // Crossword numbering: word starts in reading order share a number.
    let mut starts: Vec<(usize, usize)> = placed.iter().enumerate().map(|(i, l)| (at(l.r, l.c), i)).collect();
    starts.sort();
    let mut numbers = vec![0usize; placed.len()];
    let mut n = 0;
    let mut last = usize::MAX;
    for (cell, i) in starts {
        if cell != last {
            n += 1;
            last = cell;
        }
        numbers[i] = n;
    }
    let words_out: Vec<Placed> = placed
        .iter()
        .enumerate()
        .map(|(i, l)| Placed {
            word: l.word.clone(),
            cells: (0..l.g.len()).map(|k| { let (r, c) = l.cell(k); at(r, c) }).collect(),
            across: l.across,
            number: numbers[i],
        })
        .collect();
    Some((Grid { w, h, cells, words: words_out, shaded: Vec::new(), keystone: None }, left))
}

/// F-C6: shade cells that spell `word`, in reading order, one cell each. None
/// when this grid cannot spell it.
pub fn shade_keystone(grid: &Grid, word: &str) -> Option<Vec<usize>> {
    let want = graphemes(word);
    let mut out = Vec::new();
    let mut from = 0;
    for ch in &want {
        let next = (from..grid.cells.len()).find(|&i| grid.cells[i].as_deref() == Some(ch.as_str()))?;
        out.push(next);
        from = next + 1;
    }
    Some(out)
}

/// How well a finished grid matches the tier's crossing preference (F-C2):
/// words interlocked first, then crossings on (or off) trap positions.
pub fn score(
    grid: &Grid,
    on_traps: bool,
    traps: &dyn Fn(&str) -> Vec<usize>,
    tells: &dyn Fn(&str) -> Option<Vec<usize>>,
) -> i64 {
    let mut s = grid.words.len() as i64 * 1000;
    // Among layouts that hold the same words, the tightest one wins: a
    // crossword that walks off in a staircase wastes the screen.
    s -= (grid.w * grid.h) as i64;
    let crossings = grid.crossings();
    s += crossings.len() as i64 * 30;
    for &(cell, a, b) in &crossings {
        let hit = [a, b].iter().any(|&w| grid.index_in(w, cell).is_some_and(|k| traps(&grid.words[w].word).contains(&k)));
        s += if hit == on_traps { 20 } else { -20 };
    }
    // F-C3: a collision word that no crossing tells apart has to leave the
    // grid, which costs more than any crossing preference.
    for (i, p) in grid.words.iter().enumerate() {
        let Some(pos) = tells(&p.word) else { continue };
        let answered = crossings
            .iter()
            .filter(|(_, a, b)| *a == i || *b == i)
            .any(|&(cell, _, _)| grid.index_in(i, cell).is_some_and(|k| pos.contains(&k)));
        if !answered {
            s -= 900;
        }
    }
    s
}

/// Lay the words out `tries` times and keep the best grid (`score`). One pass
/// is greedy; the spread of attempts is what makes a crossword out of a list
/// that the first ordering could not interlock.
pub fn build_best(
    words: &[String],
    rng: &mut Rng,
    on_traps: bool,
    traps: &dyn Fn(&str) -> Vec<usize>,
    tells: &dyn Fn(&str) -> Option<Vec<usize>>,
    tries: usize,
) -> Option<(Grid, Vec<String>)> {
    let mut best: Option<(i64, Grid, Vec<String>)> = None;
    for _ in 0..tries {
        let Some((g, left)) = build(words, rng, on_traps, traps, tells) else { continue };
        let s = score(&g, on_traps, traps, tells);
        if best.as_ref().is_none_or(|(bs, _, _)| s > *bs) {
            best = Some((s, g, left));
        }
    }
    best.map(|(_, g, l)| (g, l))
}
