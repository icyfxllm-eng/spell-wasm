//! F2 -- the ring, and the board generator.

use super::{Board, Tier, Variant};
use crate::spelldoku::rng::Rng;

/// Where tile `i` sits on a `grid` x `grid` square (Full and Jr only; a Sprint ring
/// is not a square, see `boardgame_ring::tile_to_cell`), as (x, y) with y growing
/// DOWN the screen. Tile 0 is the bottom-left corner and tiles run clockwise:
/// up the left edge, across the top, down the right edge, back along the
/// bottom. The last tile is the one beside tile 0 (I1).
pub fn tile_to_grid(i: u32, grid: u32) -> (u32, u32) {
    let e = grid - 1;
    let ring = 4 * e;
    let i = i % ring;
    if i <= e {
        (0, e - i)
    } else if i <= 2 * e {
        (i - e, 0)
    } else if i <= 3 * e {
        (e, i - 2 * e)
    } else {
        (4 * e - i, e)
    }
}

/// F2: tiers drawn from the variant's weights; traps from its own rules.
pub fn generate(rng: &mut Rng, v: Variant) -> Board {
    let c = v.cfg();
    let total: u64 = c.weights.iter().map(|w| w.1).sum();
    let mut tiers: Vec<Option<Tier>> = Vec::with_capacity(c.tiles);
    tiers.push(None);
    for _ in 0..c.tiles - 2 {
        let mut r = rng.next_u64() % total;
        let mut pick = c.weights[0].0;
        for &(t, w) in c.weights {
            if r < w {
                pick = t;
                break;
            }
            r -= w;
        }
        tiers.push(Some(pick));
    }
    tiers.push(None);
    Board { tiers, traps: c.place_traps(rng), grid: c.grid }
}
