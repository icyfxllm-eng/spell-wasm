//! CC-BOARD-GAME-POLISH v1, Phase A -- the ring as a picture (Features 6, 7).
//!
//! Presentation only (I-P2, D-P19): screen position is a pure function of the
//! tile index and a shape, and no game state stores a coordinate. The engine's
//! own square `Board::grid` and its digest term are untouched; this file never
//! feeds back into a rule. It lives outside `src/boardgame/` on purpose: the
//! engine's source scan bans floats, and a pitch is a float.
//!
//! I-P10: the allowed shapes are the one config block below.

use crate::boardgame::Tier;

/// Allowed ring shapes as `(w, h)` in portrait (w <= h). Each satisfies
/// `2w + 2h - 4 == tiles`. Sprint is defined for Phase B and unused until then.
pub const FULL_SHAPES: &[(u32, u32)] = &[(22, 22), (21, 23), (20, 24), (19, 25)];
pub const SPRINT_SHAPES: &[(u32, u32)] = &[(11, 12), (10, 13)];
pub const JR_SHAPES: &[(u32, u32)] = &[(11, 11), (10, 12)];

/// The allowed shapes for a board of `tiles` tiles.
pub fn shapes_for(tiles: usize) -> &'static [(u32, u32)] {
    match tiles {
        84 => FULL_SHAPES,
        42 => SPRINT_SHAPES,
        _ => JR_SHAPES,
    }
}

/// Where tile `i` sits on a `w` x `h` ring, as (x, y) with y growing DOWN the
/// screen: tile 0 is the bottom-left corner and tiles run clockwise (up the
/// left edge, across the top, down the right edge, back along the bottom). The
/// last tile is the one beside tile 0.
pub fn tile_to_cell(i: u32, w: u32, h: u32) -> (u32, u32) {
    let ring = 2 * w + 2 * h - 4;
    let i = i % ring;
    let up = h - 1; // tiles 0..=up climb the left edge
    let across = up + (w - 1); // ..then across the top
    let down = across + (h - 1); // ..then down the right edge
    if i <= up {
        (0, h - 1 - i)
    } else if i <= across {
        (i - up, 0)
    } else if i <= down {
        (w - 1, i - across)
    } else {
        (w - 1 - (i - down), h - 1)
    }
}

/// Tile pitch (CSS px per cell) when a `w` x `h` ring is fitted to a box.
pub fn pitch(w: u32, h: u32, box_w: f64, box_h: f64) -> f64 {
    (box_w / w as f64).min(box_h / h as f64)
}

/// The shape with the largest pitch in the box; ties go to the squarer shape.
/// A box wider than tall takes the transposed (landscape) form of each shape.
pub fn choose(tiles: usize, box_w: f64, box_h: f64) -> (u32, u32) {
    let landscape = box_w > box_h;
    let mut best: Option<((u32, u32), f64)> = None;
    for &(a, b) in shapes_for(tiles) {
        let (w, h) = if landscape { (b, a) } else { (a, b) };
        let p = pitch(w, h, box_w, box_h);
        let better = match best {
            None => true,
            Some(((bw, bh), bp)) => p > bp + 1e-9 || ((p - bp).abs() <= 1e-9 && w.abs_diff(h) < bw.abs_diff(bh)),
        };
        if better {
            best = Some(((w, h), p));
        }
    }
    best.map(|b| b.0).unwrap_or((22, 22))
}

/// Feature 6: a piece is 1.6 x the tile, clamped to 24-40 pt.
pub fn piece_px(pitch: f64) -> f64 {
    (pitch * 1.6).clamp(24.0, 40.0)
}

/// Feature 6: the offset, in piece diameters, of the `k`-th of `n` pieces that
/// share a tile. Neighbours sit 0.8 of a diameter apart, so each piece keeps at
/// least 79% of its disc visible under the pieces drawn above it (the spec
/// asks for 60%).
pub fn fan(n: usize, k: usize) -> (f64, f64) {
    const TWO: [(f64, f64); 2] = [(-0.4, 0.0), (0.4, 0.0)];
    const QUAD: [(f64, f64); 4] = [(-0.4, -0.4), (0.4, -0.4), (-0.4, 0.4), (0.4, 0.4)];
    match n {
        0 | 1 => (0.0, 0.0),
        2 => TWO[k % 2],
        _ => QUAD[k % 4],
    }
}

/// Feature 7: 1, 2 or 3 pips for the variant's lowest, middle and highest tier.
/// `present` is the set of tiers the board uses (Jr: two, Standard: three), so
/// the variant difference comes from the board, not an `if`.
pub fn pips(tier: Tier, present: &[Tier]) -> u8 {
    let mut order: Vec<Tier> = present.to_vec();
    order.sort();
    order.dedup();
    order.iter().position(|&t| t == tier).map(|p| p as u8 + 1).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A-P10: every allowed shape has 2w + 2h - 4 tiles, the index -> cell map
    /// is a bijection onto the perimeter, tile 0 is the bottom-left corner,
    /// indices run clockwise, and the last tile sits beside tile 0.
    #[test]
    fn a_p10_shapes_are_perimeters_and_the_map_is_a_clockwise_bijection() {
        for (tiles, shapes) in [(84usize, FULL_SHAPES), (42, SPRINT_SHAPES), (40, JR_SHAPES)] {
            assert!(!shapes.is_empty());
            for &(a, b) in shapes {
                for (w, h) in [(a, b), (b, a)] {
                    assert_eq!((2 * w + 2 * h - 4) as usize, tiles, "{w}x{h} must hold {tiles} tiles");
                    let cells: Vec<(u32, u32)> = (0..tiles as u32).map(|i| tile_to_cell(i, w, h)).collect();
                    let set: HashSet<_> = cells.iter().copied().collect();
                    assert_eq!(set.len(), tiles, "{w}x{h}: a cell was used twice");
                    for &(x, y) in &cells {
                        assert!(x < w && y < h);
                        assert!(x == 0 || y == 0 || x == w - 1 || y == h - 1, "{w}x{h}: ({x},{y}) is not on the perimeter");
                    }
                    assert_eq!(cells[0], (0, h - 1), "{w}x{h}: tile 0 is the bottom-left corner");
                    // Every step is one cell along the edge.
                    for i in 1..tiles {
                        let (p, q) = (cells[i - 1], cells[i]);
                        assert_eq!(p.0.abs_diff(q.0) + p.1.abs_diff(q.1), 1, "{w}x{h}: tile {i} is not adjacent to tile {}", i - 1);
                    }
                    // Clockwise: up the left edge first, then across the top.
                    assert_eq!(cells[1], (0, h - 2));
                    assert_eq!(cells[h as usize - 1], (0, 0), "{w}x{h}: the climb ends at the top-left corner");
                    assert_eq!(cells[h as usize], (1, 0), "{w}x{h}: then it runs left to right");
                    // The finish is beside tile 0, along the bottom edge.
                    assert_eq!(cells[tiles - 1], (1, h - 1), "{w}x{h}: the last tile is beside tile 0");
                }
            }
        }
    }

    /// The square shapes agree with the engine's own `tile_to_grid`, so the
    /// presentation is the same ring the engine was tested on.
    #[test]
    fn square_shapes_match_the_engines_grid() {
        for g in [22u32, 11] {
            for i in 0..(4 * (g - 1)) {
                assert_eq!(tile_to_cell(i, g, g), crate::boardgame::board::tile_to_grid(i, g));
            }
        }
    }

    #[test]
    fn the_chooser_takes_the_largest_pitch_and_ties_go_to_the_squarer_shape() {
        // A square box: every full shape is limited by the long side; the
        // squarest one wins on the tie... but 22x22 has the largest pitch.
        assert_eq!(choose(84, 400.0, 400.0), (22, 22));
        // A tall box takes the tall shape.
        assert_eq!(choose(84, 350.0, 600.0), (19, 25));
        // A wide box takes it transposed.
        assert_eq!(choose(84, 600.0, 350.0), (25, 19));
        // Equal pitch (width-limited for both): the squarer one.
        let p = choose(84, 220.0, 5000.0);
        assert_eq!(p, (19, 25), "width-limited: 19 columns give the largest pitch");
        // D-P14: a box where 19x25 would shrink the tiles picks 21x23.
        let (w, h) = choose(84, 350.0, 375.0);
        let (pw, ph) = (pitch(w, h, 350.0, 375.0), pitch(19, 25, 350.0, 375.0));
        assert!(pw >= ph);
        assert_ne!((w, h), (19, 25));
        // Jr and the unused Sprint shapes.
        assert_eq!(choose(40, 300.0, 500.0), (10, 12));
        assert_eq!(choose(42, 300.0, 500.0), (10, 13));
    }

    #[test]
    fn pieces_are_one_point_six_tiles_clamped_and_fan_out_apart() {
        assert_eq!(piece_px(10.0), 24.0);
        assert!((piece_px(20.0) - 32.0).abs() < 1e-9);
        assert_eq!(piece_px(40.0), 40.0);
        // Four pieces on a tile: every pair is at least 0.8 of a diameter apart.
        for n in 2..=4usize {
            for a in 0..n {
                for b in a + 1..n {
                    let (pa, pb) = (fan(n, a), fan(n, b));
                    let d = ((pa.0 - pb.0).powi(2) + (pa.1 - pb.1).powi(2)).sqrt();
                    assert!(d >= 0.8 - 1e-9, "n={n}: pieces {a} and {b} are only {d} apart");
                }
            }
        }
    }

    #[test]
    fn pips_rank_the_tiers_a_variant_uses() {
        let std = [Tier::Medium, Tier::Hard, Tier::Expert];
        assert_eq!([pips(Tier::Medium, &std), pips(Tier::Hard, &std), pips(Tier::Expert, &std)], [1, 2, 3]);
        let jr = [Tier::Easy, Tier::Medium];
        assert_eq!([pips(Tier::Easy, &jr), pips(Tier::Medium, &jr)], [1, 2]);
    }
}
