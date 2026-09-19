//! Repeat avoidance (F8, I6): a hash that is the same for every board a player
//! would recognise as "the same puzzle" -- digit relabelings and the standard
//! symmetry group (rows within bands, bands, columns within stacks, stacks, and
//! transposition where the boxes are square).
//!
//! It is an INVARIANT, not a full canonical form: two relabeled or permuted
//! copies always hash equal, which is the guarantee I6 needs. Two genuinely
//! different boards can very rarely share a hash too; that only makes the
//! generator skip a board it could have served, never serve a repeat.

use super::gen::{Clue, Puzzle};
use super::rng::fnv;

fn kind(c: &Clue) -> u8 {
    match c {
        Clue::Empty => 0,
        Clue::Given(_) | Clue::Spelled(_) => 1,
        Clue::Fragment(_) => 2,
    }
}

fn value(c: &Clue) -> Option<u8> {
    match c {
        Clue::Given(v) | Clue::Spelled(v) => Some(*v),
        _ => None,
    }
}

/// Band profile: each band's rows reduced to a sorted list of per-row
/// signatures, then the bands themselves sorted -- invariant under row
/// permutations within a band and band permutations.
fn profile(lines: &[Vec<u8>], band: usize) -> Vec<Vec<Vec<u8>>> {
    let mut bands: Vec<Vec<Vec<u8>>> = lines
        .chunks(band)
        .map(|chunk| {
            let mut rows: Vec<Vec<u8>> = chunk.to_vec();
            rows.sort();
            rows
        })
        .collect();
    bands.sort();
    bands
}

fn signature(p: &Puzzle, br: usize, bc: usize, transpose: bool) -> String {
    let n = p.n;
    let at = |r: usize, c: usize| if transpose { &p.clues[c * n + r] } else { &p.clues[r * n + c] };
    // Per line: sorted multiset of clue kinds with the counts per crossing block.
    let row_sig = |r: usize| -> Vec<u8> {
        let mut per_stack: Vec<u8> = (0..n / bc)
            .map(|s| (0..bc).map(|k| kind(at(r, s * bc + k))).filter(|&k| k > 0).count() as u8)
            .collect();
        per_stack.sort_unstable();
        let frags = (0..n).filter(|&c| kind(at(r, c)) == 2).count() as u8;
        per_stack.push(frags);
        per_stack
    };
    let col_sig = |c: usize| -> Vec<u8> {
        let mut per_band: Vec<u8> = (0..n / br)
            .map(|b| (0..br).map(|k| kind(at(b * br + k, c))).filter(|&k| k > 0).count() as u8)
            .collect();
        per_band.sort_unstable();
        let frags = (0..n).filter(|&r| kind(at(r, c)) == 2).count() as u8;
        per_band.push(frags);
        per_band
    };
    let rows: Vec<Vec<u8>> = (0..n).map(row_sig).collect();
    let cols: Vec<Vec<u8>> = (0..n).map(col_sig).collect();
    // Relabel-invariant value structure: for each digit, where its givens sit
    // by band and by stack, as sorted counts; then the digits' shapes sorted.
    let mut shapes: Vec<(Vec<u8>, Vec<u8>)> = (1..=n as u8)
        .map(|v| {
            let mut by_band = vec![0u8; n / br];
            let mut by_stack = vec![0u8; n / bc];
            for r in 0..n {
                for c in 0..n {
                    if value(at(r, c)) == Some(v) {
                        by_band[r / br] += 1;
                        by_stack[c / bc] += 1;
                    }
                }
            }
            by_band.sort_unstable();
            by_stack.sort_unstable();
            (by_band, by_stack)
        })
        .collect();
    shapes.sort();
    format!("{n}|{:?}|{:?}|{:?}", profile(&rows, br), profile(&cols, bc), shapes)
}

pub fn hash(p: &Puzzle) -> u64 {
    let size = super::geo::size_of(p.n).expect("size");
    let a = signature(p, size.br, size.bc, false);
    // Transposition is a symmetry only when the boxes are square.
    let s = if size.br == size.bc {
        let b = signature(p, size.bc, size.br, true);
        if b < a { b } else { a }
    } else {
        a
    };
    fnv(s.as_bytes())
}
