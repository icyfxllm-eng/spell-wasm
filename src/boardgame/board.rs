//! F2 -- the ring, and the board generator.

use super::rules::Ruleset;
use super::{Board, Tier};
use crate::spelldoku::rng::Rng;

/// Where tile `i` sits on a `grid` x `grid` square, as (x, y) with y growing
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

/// F2: tiers drawn from the ruleset's weights; traps from its own table.
pub fn generate<R: Ruleset>(rng: &mut Rng) -> Board {
    let total: u64 = R::WEIGHTS.iter().map(|w| w.1).sum();
    let mut tiers: Vec<Option<Tier>> = Vec::with_capacity(R::TILES);
    tiers.push(None);
    for _ in 0..R::TILES - 2 {
        let mut r = rng.next_u64() % total;
        let mut pick = R::WEIGHTS[0].0;
        for &(t, w) in R::WEIGHTS {
            if r < w {
                pick = t;
                break;
            }
            r -= w;
        }
        tiers.push(Some(pick));
    }
    tiers.push(None);
    Board { tiers, traps: R::place_traps(rng), grid: R::GRID }
}
