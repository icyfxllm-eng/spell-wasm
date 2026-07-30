//! CC-WORDPICTURE-SCANLOCK v8 core.
//!
//! F1: every word is text-on-path along the pinned path's arc length;
//! the baseline IS the path (erase the letters, recover the scan).
//! F2: size = arc/chars clamped [floor, band max]; spacing justifies to
//! fill the segment exactly; split only at pre-marked boundaries; else
//! INVALID and redraw. F3: collisions resolve in word choice and size,
//! never geometry; 3 consecutive seed failures on one pair = subject
//! fails typesettability. I2: geometry in, placements out — no API here
//! can write, resample, or transform a path.

/// A pinned scan path (verbatim coordinates) + its scan-time marks.
#[derive(Debug, Clone)]
pub struct ScanPath {
    pub points: Vec<(f32, f32)>,
    /// Pre-marked segment boundaries as arc fractions (F2's ONLY splits).
    pub segments: Vec<f32>,
    pub sub_floor: bool,
    pub decorative_thin: bool,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub path_idx: usize,
    /// Arc-fraction range of the segment this word fills exactly.
    pub t0: f32,
    pub t1: f32,
    pub word: String,
    pub glyph_size: f32,
    /// Justified advance per glyph along the arc (fills segment exactly).
    pub advance_px: f32,
    /// The baseline: verbatim sub-polyline of the pinned path.
    pub baseline: Vec<(f32, f32)>,
}

#[derive(Debug)]
pub enum TypesetError {
    /// (path a, path b): every legal word collided at floor across the
    /// re-seed budget — subject fails typesettability (route to F4).
    Untypesettable(usize, usize),
    /// A segment no pool word can legally fill (post-gate this should
    /// not happen; reported, never squeezed).
    NoLegalWord(usize),
}

pub struct Params {
    pub floor: f32,
    pub band_max: f32,
    /// Mean glyph advance as a fraction of size (script class).
    pub advance_frac: f32,
    /// D4 (pending Eric): justified spacing may stretch to at most this
    /// multiple of natural tracking; sparser segments must split/redraw.
    pub max_justify: f32,
}

fn arc_cum(points: &[(f32, f32)]) -> Vec<f32> {
    let mut cum = vec![0.0f32];
    for w in points.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        cum.push(cum.last().unwrap() + d);
    }
    cum
}

/// Verbatim sub-polyline for [t0,t1]: original interior points untouched;
/// endpoints interpolated ON the polyline (residual 0 by construction).
pub fn sub_polyline(points: &[(f32, f32)], cum: &[f32], t0: f32, t1: f32) -> Vec<(f32, f32)> {
    let total = *cum.last().unwrap_or(&0.0);
    let (a, b) = (t0 * total, t1 * total);
    let mut out = Vec::new();
    let point_at = |want: f32| -> (f32, f32) {
        for i in 1..points.len() {
            if cum[i] >= want {
                let seg = cum[i] - cum[i - 1];
                let f = if seg > 0.0 { (want - cum[i - 1]) / seg } else { 0.0 };
                return (
                    points[i - 1].0 + (points[i].0 - points[i - 1].0) * f,
                    points[i - 1].1 + (points[i].1 - points[i - 1].1) * f,
                );
            }
        }
        *points.last().unwrap()
    };
    out.push(point_at(a));
    for i in 0..points.len() {
        if cum[i] > a && cum[i] < b {
            out.push(points[i]);
        }
    }
    out.push(point_at(b));
    out
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// v8.1 F2 — the size-descent report when a subject cannot be planned.
#[derive(Debug)]
pub enum PlanError {
    /// (path a, path b, interior distance): corridors intersect at floor.
    CorridorConflict(usize, usize, f32),
    /// (path, cap at floor): starved below MIN_WORD_CHARS at floor.
    Starved(usize, f32),
    /// (path, budget left): pool exhausted mid-pack — BLOCKED, never sparse.
    PoolExhausted(usize, f32),
}

pub struct CapacityParams {
    pub floor: f32,
    pub band_max: f32,
    /// Measured mean advance in em (tools/measure_advance.py) or the
    /// script-class value for non-Latin.
    pub avg_advance: f32,
    pub min_word_chars: f32,
    pub gap_chars: f32,
    pub line_height_ratio: f32,
    pub step: f32,
}

/// Interior point set (junction zones excluded — connected strokes touch
/// at junctions by structure).
fn interior(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    const J: f32 = 18.0;
    let e0 = points[0];
    let e1 = *points.last().unwrap();
    points
        .iter()
        .copied()
        .filter(|pt| {
            (pt.0 - e0.0).hypot(pt.1 - e0.1) > J && (pt.0 - e1.0).hypot(pt.1 - e1.1) > J
        })
        .collect()
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0);
    (p.0 - (a.0 + t * vx)).hypot(p.1 - (a.1 + t * vy))
}

/// Measurement-only densify (baselines are never derived from this).
fn densify(points: &[(f32, f32)], step: f32) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for w in points.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        let n = (d / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f32 / n as f32;
            out.push((w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t));
        }
    }
    out.push(*points.last().unwrap());
    out
}

fn pairwise_interior_dist(paths: &[ScanPath]) -> Vec<(usize, usize, f32)> {
    let word: Vec<usize> = paths
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.sub_floor && !p.decorative_thin)
        .map(|(i, _)| i)
        .collect();
    let mut out = Vec::new();
    for a in 0..word.len() {
        for b in a + 1..word.len() {
            let (i, j) = (word[a], word[b]);
            let di = densify(&paths[i].points, 6.0);
            let dj = densify(&paths[j].points, 6.0);
            let pi = interior(&di);
            let pj = interior(&dj);
            if pi.len() < 2 || pj.len() < 2 {
                continue;
            }
            let mut m = f32::MAX;
            for pt in pi.iter().step_by(3) {
                for w in pj.windows(2) {
                    m = m.min(seg_dist(*pt, w[0], w[1]));
                }
            }
            out.push((i, j, m));
        }
    }
    out
}

/// v8.1 F2 — global size by descent: one size per picture (I9).
pub fn size_by_descent(
    paths: &[ScanPath],
    p: &CapacityParams,
) -> Result<f32, PlanError> {
    let dists = pairwise_interior_dist(paths);
    let mut s = p.band_max;
    while s >= p.floor - 0.01 {
        let mut ok = true;
        let mut err: Option<PlanError> = None;
        for &(i, j, d) in &dists {
            if d < s * p.line_height_ratio {
                ok = false;
                err = Some(PlanError::CorridorConflict(i, j, d));
                break;
            }
        }
        if ok {
            for (i, path) in paths.iter().enumerate() {
                if path.sub_floor || path.decorative_thin {
                    continue;
                }
                let arc = arc_cum(&path.points).last().copied().unwrap_or(0.0);
                let cap = arc / (s * p.avg_advance);
                if cap < p.min_word_chars {
                    ok = false;
                    err = Some(PlanError::Starved(i, cap));
                    break;
                }
            }
        }
        if ok {
            return Ok(s);
        }
        s -= p.step;
        let _ = err;
    }
    // report the blocker measured at floor
    let s = p.floor;
    for &(i, j, d) in &dists {
        if d < s * p.line_height_ratio {
            return Err(PlanError::CorridorConflict(i, j, d));
        }
    }
    for (i, path) in paths.iter().enumerate() {
        if path.sub_floor || path.decorative_thin {
            continue;
        }
        let arc = arc_cum(&path.points).last().copied().unwrap_or(0.0);
        let cap = arc / (s * p.avg_advance);
        if cap < p.min_word_chars {
            return Err(PlanError::Starved(i, cap));
        }
    }
    Err(PlanError::Starved(usize::MAX, 0.0))
}

/// v8.1 F3 — greedy pack to capacity. Word count is the OUTPUT (I7).
/// Packing is per pre-marked segment (words never cross a corner mark);
/// intervals are sequential so same-path overlap cannot exist (F4). The
/// only exits are PACKED or an error (I8) — a word is never dropped.
pub fn plan_capacity(
    paths: &[ScanPath],
    pool: &[(String, usize)],
    seed: u64,
    p: &CapacityParams,
) -> Result<(f32, Vec<Placement>), PlanError> {
    let size = size_by_descent(paths, p)?;
    let mut st = seed ^ 0x43415041; // "CAPA"
    let mut used: Vec<usize> = Vec::new();
    let mut placements = Vec::new();
    // longest-first ordering; seeded rotation only among equal lengths
    let mut by_len: Vec<usize> = (0..pool.len()).collect();
    by_len.sort_by_key(|&i| std::cmp::Reverse(pool[i].1));
    for (pi, path) in paths.iter().enumerate() {
        if path.sub_floor || path.decorative_thin {
            continue;
        }
        let cum = arc_cum(&path.points);
        let total = *cum.last().unwrap_or(&0.0);
        let mut bounds = vec![0.0f32];
        bounds.extend(path.segments.iter().copied().filter(|t| *t > 0.0 && *t < 1.0));
        bounds.push(1.0);
        bounds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bounds.dedup();
        for w2 in bounds.windows(2) {
            let (t0, t1) = (w2[0], w2[1]);
            let seg_len = (t1 - t0) * total;
            let mut budget = seg_len / (size * p.avg_advance);
            if budget < p.min_word_chars {
                continue; // sub-min sliver between corner marks (D-A)
            }
            let mut words: Vec<usize> = Vec::new();
            let mut packed_chars = 0.0f32;
            loop {
                let need = p.min_word_chars + if words.is_empty() { 0.0 } else { p.gap_chars };
                if budget < need {
                    break;
                }
                let limit = budget - if words.is_empty() { 0.0 } else { p.gap_chars };
                // longest word with len <= limit; seeded rotation on ties
                let mut best_len = 0usize;
                for &wi in &by_len {
                    if used.contains(&wi) {
                        continue;
                    }
                    if (pool[wi].1 as f32) <= limit {
                        best_len = pool[wi].1;
                        break;
                    }
                }
                if best_len == 0 {
                    return Err(PlanError::PoolExhausted(pi, budget));
                }
                let ties: Vec<usize> = by_len
                    .iter()
                    .copied()
                    .filter(|&wi| !used.contains(&wi) && pool[wi].1 == best_len)
                    .collect();
                let pick = ties[(splitmix(&mut st) % ties.len() as u64) as usize];
                used.push(pick);
                if !words.is_empty() {
                    budget -= p.gap_chars;
                }
                budget -= pool[pick].1 as f32;
                packed_chars += pool[pick].1 as f32;
                words.push(pick);
            }
            if words.is_empty() {
                continue;
            }
            // F3.3: remainder becomes justified spacing across the run —
            // the run spans the FULL segment; glyphs never scale.
            let n_gaps = (words.len() - 1) as f32;
            let slack = seg_len - packed_chars * size * p.avg_advance;
            let gap_px = if n_gaps > 0.0 {
                (slack / n_gaps).max(0.0)
            } else {
                0.0
            };
            let mut cursor = t0 * total;
            for (k, &wi) in words.iter().enumerate() {
                let chars = pool[wi].1 as f32;
                let mut ext = chars * size * p.avg_advance;
                if n_gaps == 0.0 {
                    ext = seg_len; // single word justifies across the segment
                }
                let a = cursor / total;
                let b = ((cursor + ext) / total).min(t1);
                placements.push(Placement {
                    path_idx: pi,
                    t0: a,
                    t1: b,
                    word: pool[wi].0.clone(),
                    glyph_size: size,
                    advance_px: ext / chars,
                    baseline: sub_polyline(&path.points, &cum, a, b),
                });
                cursor += ext;
                if (k as f32) < n_gaps {
                    cursor += gap_px;
                }
            }
        }
    }
    Ok((size, placements))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(len: f32, y: f32) -> ScanPath {
        ScanPath {
            points: vec![(0.0, y), (len / 2.0, y), (len, y)],
            segments: vec![],
            sub_floor: false,
            decorative_thin: false,
        }
    }

    fn params() -> CapacityParams {
        CapacityParams {
            floor: 13.0,
            band_max: 40.0,
            avg_advance: 0.5257,
            min_word_chars: 3.0,
            gap_chars: 1.0,
            line_height_ratio: 1.0,
            step: 0.5,
        }
    }

    fn pool() -> Vec<(String, usize)> {
        (0..200)
            .map(|k| {
                let n = 3 + (k % 9);
                (format!("{}{}", "abcdefghijkl".chars().take(n).collect::<String>(), k), n + k.to_string().len())
            })
            .map(|(w, _)| {
                let n = w.chars().count();
                (w, n)
            })
            .collect()
    }

    #[test]
    fn capacity_is_arc_over_size_advance() {
        // 400px path at size 20: cap = 400/(20*0.5257) = 38.04 chars
        let p = params();
        let cap = 400.0 / (20.0 * p.avg_advance);
        assert!((cap - 38.04).abs() < 0.1);
    }

    #[test]
    fn emergent_counts_and_baselines_verbatim() {
        let p = params();
        let paths = [straight(600.0, 0.0), straight(200.0, 100.0)];
        let (size, pls) = plan_capacity(&paths, &pool(), 1, &p).unwrap();
        assert!(size >= p.floor && size <= p.band_max);
        let long: usize = pls.iter().filter(|x| x.path_idx == 0).count();
        let short: usize = pls.iter().filter(|x| x.path_idx == 1).count();
        assert!(long > short, "capacity scales with arc: {long} vs {short}");
        for pl in &pls {
            let src = &paths[pl.path_idx].points;
            for pt in &pl.baseline[1..pl.baseline.len().saturating_sub(1)] {
                let on = src.windows(2).any(|w| seg_dist(*pt, w[0], w[1]) < 0.01);
                assert!(on, "baseline off scan");
            }
        }
    }

    #[test]
    fn corridor_conflict_descends_then_blocks() {
        let p = params();
        // two parallel paths 18px apart: descent must land size <= 18
        let paths = [straight(400.0, 0.0), straight(400.0, 18.0)];
        let s = size_by_descent(&paths, &p).unwrap();
        assert!(s <= 18.0 && s >= p.floor);
        // 9px apart: below floor corridor -> BLOCKED loudly
        let tight = [straight(400.0, 0.0), straight(400.0, 9.0)];
        assert!(matches!(
            size_by_descent(&tight, &p),
            Err(PlanError::CorridorConflict(_, _, _))
        ));
    }

    #[test]
    fn packer_exits_are_packed_or_blocked() {
        let p = params();
        let paths = [straight(2000.0, 0.0)];
        // starved pool: 3 tiny words for a 2000px path -> PoolExhausted
        let tiny: Vec<(String, usize)> =
            vec![("abc".into(), 3), ("def".into(), 3), ("ghi".into(), 3)];
        assert!(matches!(
            plan_capacity(&paths, &tiny, 1, &p),
            Err(PlanError::PoolExhausted(_, _))
        ));
    }
}
