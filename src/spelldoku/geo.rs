//! Board geometry: sizes, units and peers (F7).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    /// Side length: 4, 6 or 9.
    pub n: usize,
    /// Box height and width.
    pub br: usize,
    pub bc: usize,
}

pub const S4: Size = Size { n: 4, br: 2, bc: 2 };
pub const S6: Size = Size { n: 6, br: 2, bc: 3 };
pub const S9: Size = Size { n: 9, br: 3, bc: 3 };

pub fn size_of(n: usize) -> Option<Size> {
    match n {
        4 => Some(S4),
        6 => Some(S6),
        9 => Some(S9),
        _ => None,
    }
}

/// Precomputed units and peers for one size.
pub struct Geo {
    pub size: Size,
    /// Rows `0..n`, then columns `n..2n`, then boxes `2n..3n`.
    pub units: Vec<Vec<usize>>,
    /// The three units each cell belongs to.
    pub cell_units: Vec<[usize; 3]>,
    pub peers: Vec<Vec<usize>>,
}

impl Geo {
    pub fn new(size: Size) -> Self {
        let n = size.n;
        let boxes_across = n / size.bc;
        let box_of = |i: usize| (i / n / size.br) * boxes_across + (i % n) / size.bc;
        let mut units = vec![Vec::new(); 3 * n];
        let mut cell_units = vec![[0usize; 3]; n * n];
        for i in 0..n * n {
            let (r, c, b) = (i / n, i % n, box_of(i));
            units[r].push(i);
            units[n + c].push(i);
            units[2 * n + b].push(i);
            cell_units[i] = [r, n + c, 2 * n + b];
        }
        let mut peers = vec![Vec::new(); n * n];
        for i in 0..n * n {
            let mut p: Vec<usize> = Vec::new();
            for u in cell_units[i] {
                for &j in &units[u] {
                    if j != i && !p.contains(&j) {
                        p.push(j);
                    }
                }
            }
            p.sort_unstable();
            peers[i] = p;
        }
        Geo { size, units, cell_units, peers }
    }

    pub fn cells(&self) -> usize {
        self.size.n * self.size.n
    }

    /// Candidate mask with every value `1..=n` set.
    pub fn full(&self) -> u16 {
        ((1u32 << (self.size.n + 1)) - 2) as u16
    }
}

pub fn bit(v: u8) -> u16 {
    1u16 << v
}
