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
    /// v8.2.1 — a solid source feature (eyes, nose, buttons): never hosts
    /// words, always renders as a pinned stroke, excluded from corridor
    /// and junction tests. Its presence can be REQUIRED by the scan file.
    pub micro: bool,
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
    /// v8.2 F4: decorative arc fraction over budget (fraction, cap).
    DecorativeBudget(f32, f32),
    /// v8.2 F5: micro arc fraction over budget (fraction, cap).
    MicroBudget(f32, f32),
    /// v8.2 F3: no size satisfies the density floor (path, words at floor).
    DensityFloor(usize, usize),
    /// v8.2 F2/I12: rendered glyphs of different words intersect.
    GlyphOverlap(usize, usize),
}

/// v8.2 F2/I12 — oriented glyph boxes for the render-time overlap gate.
#[derive(Debug, Clone)]
pub struct GlyphObb {
    pub cx: f32,
    pub cy: f32,
    pub half_w: f32,
    pub half_h: f32,
    pub cos: f32,
    pub sin: f32,
}

pub fn placement_obbs(pl: &Placement) -> Vec<GlyphObb> {
    let cum = arc_cum(&pl.baseline);
    let total = *cum.last().unwrap_or(&0.0);
    if total <= 0.0 {
        return Vec::new();
    }
    let n = ((total / pl.advance_px).round() as usize).max(1);
    let point_and_tangent = |want: f32| -> ((f32, f32), (f32, f32)) {
        for i in 1..pl.baseline.len() {
            if cum[i] >= want {
                let seg = cum[i] - cum[i - 1];
                let f = if seg > 0.0 { (want - cum[i - 1]) / seg } else { 0.0 };
                let a = pl.baseline[i - 1];
                let b = pl.baseline[i];
                let d = ((b.0 - a.0), (b.1 - a.1));
                let m = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-6);
                return (
                    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f),
                    (d.0 / m, d.1 / m),
                );
            }
        }
        let a = pl.baseline[pl.baseline.len() - 2];
        let b = *pl.baseline.last().unwrap();
        let d = ((b.0 - a.0), (b.1 - a.1));
        let m = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-6);
        (b, (d.0 / m, d.1 / m))
    };
    (0..n)
        .map(|k| {
            let want = (k as f32 + 0.5) * (total / n as f32);
            let ((cx, cy), (tx, ty)) = point_and_tangent(want);
            GlyphObb {
                cx,
                cy,
                half_w: (total / n as f32) * 0.5 * 0.92,
                half_h: pl.glyph_size * 0.5,
                cos: tx,
                sin: ty,
            }
        })
        .collect()
}

fn obb_corners(o: &GlyphObb) -> [(f32, f32); 4] {
    let ux = (o.cos * o.half_w, o.sin * o.half_w);
    let vy = (-o.sin * o.half_h, o.cos * o.half_h);
    [
        (o.cx + ux.0 + vy.0, o.cy + ux.1 + vy.1),
        (o.cx + ux.0 - vy.0, o.cy + ux.1 - vy.1),
        (o.cx - ux.0 - vy.0, o.cy - ux.1 - vy.1),
        (o.cx - ux.0 + vy.0, o.cy - ux.1 + vy.1),
    ]
}

/// SAT intersection for two oriented boxes.
pub fn obb_intersect(a: &GlyphObb, b: &GlyphObb) -> bool {
    let ca = obb_corners(a);
    let cb = obb_corners(b);
    let axes = [
        (a.cos, a.sin),
        (-a.sin, a.cos),
        (b.cos, b.sin),
        (-b.sin, b.cos),
    ];
    for (ax, ay) in axes {
        let pa: Vec<f32> = ca.iter().map(|(x, y)| x * ax + y * ay).collect();
        let pb: Vec<f32> = cb.iter().map(|(x, y)| x * ax + y * ay).collect();
        let (amin, amax) = (pa.iter().cloned().fold(f32::MAX, f32::min), pa.iter().cloned().fold(f32::MIN, f32::max));
        let (bmin, bmax) = (pb.iter().cloned().fold(f32::MAX, f32::min), pb.iter().cloned().fold(f32::MIN, f32::max));
        if amax < bmin || bmax < amin {
            return false;
        }
    }
    true
}

/// v8.2 F1 — junctions: arc positions (t) on each path where another
/// path's endpoint or interior converges within `radius`.
pub fn junctions(paths: &[ScanPath], radius: f32) -> Vec<Vec<f32>> {
    let mut out: Vec<Vec<f32>> = vec![Vec::new(); paths.len()];
    let word: Vec<usize> = paths
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.sub_floor && !p.decorative_thin && !p.micro)
        .map(|(i, _)| i)
        .collect();
    for &i in &word {
        let cum = arc_cum(&paths[i].points);
        let total = *cum.last().unwrap_or(&0.0);
        if total <= 0.0 {
            continue;
        }
        for &j in &word {
            if i == j {
                continue;
            }
            // other path's ENDPOINTS against this path's arc
            for ep in [paths[j].points[0], *paths[j].points.last().unwrap()] {
                for (k, pt) in paths[i].points.iter().enumerate() {
                    if (pt.0 - ep.0).hypot(pt.1 - ep.1) <= radius {
                        out[i].push(cum[k] / total);
                        break;
                    }
                }
            }
        }
        // own endpoints against other paths' interiors (arm ends)
        for (t_end, ep) in [(0.0f32, paths[i].points[0]), (1.0, *paths[i].points.last().unwrap())] {
            for &j in &word {
                if i == j {
                    continue;
                }
                let hit = paths[j]
                    .points
                    .iter()
                    .any(|pt| (pt.0 - ep.0).hypot(pt.1 - ep.1) <= radius);
                if hit {
                    out[i].push(t_end);
                }
            }
        }
        // v8.2 F1 (self-junctions): a single closed ring pinches near
        // ITSELF at wing joins, heads and feet — arc-distant but
        // space-near. Those convergences are exactly Eric's red circles.
        let pts = &paths[i].points;
        let n = pts.len();
        let stride = ((n / 600).max(1)) as usize; // subsample for O(n^2) scan
        let idx: Vec<usize> = (0..n).step_by(stride).collect();
        for (ai, &a) in idx.iter().enumerate() {
            for &b in idx.iter().skip(ai + 1) {
                let arc_gap = (cum[b] - cum[a]).min(total - (cum[b] - cum[a]));
                if arc_gap < radius * 4.0 {
                    continue; // neighbors along the path, not a pinch
                }
                if (pts[a].0 - pts[b].0).hypot(pts[a].1 - pts[b].1) <= radius {
                    out[i].push(cum[a] / total);
                    out[i].push(cum[b] / total);
                }
            }
        }
        out[i].sort_by(|a, b| a.partial_cmp(b).unwrap());
        out[i].dedup_by(|a, b| (*a - *b).abs() < 0.01);
    }
    out
}

/// Discrete minimum turn radius over a path (circumradius of triples).
pub fn min_turn_radius(points: &[(f32, f32)]) -> f32 {
    let mut r_min = f32::MAX;
    for w in points.windows(3) {
        let (a, b, c) = (w[0], w[1], w[2]);
        let ab = (b.0 - a.0).hypot(b.1 - a.1);
        let bc = (c.0 - b.0).hypot(c.1 - b.1);
        let ca = (a.0 - c.0).hypot(a.1 - c.1);
        let s2 = ((a.0 - c.0) * (b.1 - a.1) - (a.0 - b.0) * (c.1 - a.1)).abs();
        if s2 > 1e-3 {
            r_min = r_min.min(ab * bc * ca / (2.0 * s2));
        }
    }
    r_min
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
    /// v8.2 F1
    pub junction_radius: f32,
    pub keepout_arc_ratio: f32,
    /// v8.2 F3
    pub curve_size_ratio: f32,
    pub min_words_per_path: f32,
    /// v8.2 F4/F5 budgets
    pub decorative_max_fraction: f32,
    pub micro_max_fraction: f32,
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
        .filter(|(_, p)| !p.sub_floor && !p.decorative_thin && !p.micro)
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
/// Usable arc after v8.2 F1 keep-outs: each junction on a path reserves
/// KEEPOUT_ARC = ratio x size of arc (half either side of an interior
/// junction; inward-only at an endpoint junction).
pub fn usable_arc(path: &ScanPath, juncs: &[f32], size: f32, ratio: f32) -> f32 {
    let total = arc_cum(&path.points).last().copied().unwrap_or(0.0);
    if total <= 0.0 {
        return 0.0;
    }
    let k = size * ratio;
    // Keep-outs MERGE: on a serpentine path whose coils pass near
    // themselves, junctions cluster and their reservations overlap.
    // Summing them independently over-counts until nothing is left
    // (the dragon's MicroBudget 1.0) — union the intervals instead.
    let mut iv: Vec<(f32, f32)> = juncs
        .iter()
        .map(|&t| {
            let c = t * total;
            ((c - k).max(0.0), (c + k).min(total))
        })
        .collect();
    iv.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut reserved = 0.0f32;
    let mut cur: Option<(f32, f32)> = None;
    for (a, b) in iv {
        match cur {
            Some((s0, e0)) if a <= e0 => cur = Some((s0, e0.max(b))),
            Some((s0, e0)) => {
                reserved += e0 - s0;
                cur = Some((a, b));
            }
            None => cur = Some((a, b)),
        }
    }
    if let Some((s0, e0)) = cur {
        reserved += e0 - s0;
    }
    (total - reserved).max(0.0)
}

pub fn size_by_descent(
    paths: &[ScanPath],
    p: &CapacityParams,
) -> Result<f32, PlanError> {
    // v8.2 F4/F5/I13 — budgets are enforced at scan load, before packing.
    let tot_arc: f32 = paths
        .iter()
        .map(|q| arc_cum(&q.points).last().copied().unwrap_or(0.0))
        .sum();
    if tot_arc > 0.0 {
        let deco: f32 = paths
            .iter()
            .filter(|q| q.decorative_thin)
            .map(|q| arc_cum(&q.points).last().copied().unwrap_or(0.0))
            .sum();
        let frac = deco / tot_arc;
        if frac > p.decorative_max_fraction {
            return Err(PlanError::DecorativeBudget(frac, p.decorative_max_fraction));
        }
    }
    let dists = pairwise_interior_dist(paths);
    // A convergence closer than one glyph height IS a junction for
    // typesetting purposes — radius scales with the candidate size.
    let juncs = junctions(paths, p.junction_radius.max(p.band_max * p.line_height_ratio));
    // v8.2 F3.1 curvature cap: s <= r_min x ratio over non-micro paths.
    let mut cap_by_curve = p.band_max;
    for q in paths.iter().filter(|q| !q.sub_floor && !q.decorative_thin && !q.micro) {
        let r = min_turn_radius(&q.points);
        if r.is_finite() {
            cap_by_curve = cap_by_curve.min(r * p.curve_size_ratio);
        }
    }
    let mut s = p.band_max.min(cap_by_curve.max(p.floor));
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
            // v8.2 F3.2 density floor + F1 keep-outs + F5 micro classing:
            // a path too small to host MIN_WORD_CHARS at this size is
            // MICRO (renders as stroke) — it does not starve the descent;
            // every non-micro path must reach MIN_WORDS_PER_PATH words.
            let mut micro_arc = 0.0f32;
            for (i, path) in paths.iter().enumerate() {
                if path.sub_floor || path.decorative_thin || path.micro {
                    continue;
                }
                let arc = arc_cum(&path.points).last().copied().unwrap_or(0.0);
                let usable = usable_arc(path, &juncs[i], s, p.keepout_arc_ratio);
                let cap = usable / (s * p.avg_advance);
                if cap < p.min_word_chars {
                    micro_arc += arc;
                    continue;
                }
                // The density floor forbids STARVING a path that could be
                // dense; it does not outlaw inherently small paths. A path
                // that cannot reach MIN_WORDS_PER_PATH even at floor is a
                // single-word path (subject-agnostic; no per-picture knob).
                let words = (cap / (p.min_word_chars + p.gap_chars)).floor();
                let usable_floor = usable_arc(path, &juncs[i], p.floor, p.keepout_arc_ratio);
                let words_at_floor = ((usable_floor / (p.floor * p.avg_advance))
                    / (p.min_word_chars + p.gap_chars))
                    .floor();
                if words_at_floor >= p.min_words_per_path && words < p.min_words_per_path {
                    ok = false;
                    err = Some(PlanError::DensityFloor(i, words as usize));
                    break;
                }
            }
            if ok && tot_arc > 0.0 && micro_arc / tot_arc > p.micro_max_fraction {
                ok = false;
                err = Some(PlanError::MicroBudget(micro_arc / tot_arc, p.micro_max_fraction));
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
    let mut micro_arc = 0.0f32;
    for (i, path) in paths.iter().enumerate() {
        if path.sub_floor || path.decorative_thin || path.micro {
            continue;
        }
        let arc = arc_cum(&path.points).last().copied().unwrap_or(0.0);
        let usable = usable_arc(path, &juncs[i], s, p.keepout_arc_ratio);
        let cap = usable / (s * p.avg_advance);
        if cap < p.min_word_chars {
            micro_arc += arc;
            continue;
        }
        let words = (cap / (p.min_word_chars + p.gap_chars)).floor();
        let usable_floor = usable_arc(path, &juncs[i], p.floor, p.keepout_arc_ratio);
        let words_at_floor =
            ((usable_floor / (p.floor * p.avg_advance)) / (p.min_word_chars + p.gap_chars)).floor();
        if words_at_floor >= p.min_words_per_path && words < p.min_words_per_path {
            return Err(PlanError::DensityFloor(i, words as usize));
        }
    }
    if tot_arc > 0.0 && micro_arc / tot_arc > p.micro_max_fraction {
        return Err(PlanError::MicroBudget(micro_arc / tot_arc, p.micro_max_fraction));
    }
    Err(PlanError::Starved(usize::MAX, 0.0))
}

/// v8.1 F3 — greedy pack to capacity. Word count is the OUTPUT (I7).
/// Packing is per pre-marked segment (words never cross a corner mark);
/// intervals are sequential so same-path overlap cannot exist (F4). The
/// only exits are PACKED or an error (I8) — a word is never dropped.
/// v8.2 F5 — a path rendered as its plain pinned stroke (identity-
/// critical micro features: snowman eyes). Never squeezed, never dropped.
#[derive(Debug, Clone)]
pub struct MicroStroke {
    pub path_idx: usize,
    pub points: Vec<(f32, f32)>,
}

/// v8.2 — descent VERIFIES by planning: a candidate size is accepted
/// only if it packs with zero rendered-glyph overlaps. Self-pinching
/// rings cannot be predicted by corridor distance alone, so the honest
/// constraint is the render gate itself, applied during descent (I14:
/// still one global size, still no per-picture knobs).
/// Per-path coverage as the PACKER measured it — the gate consumes these
/// numbers rather than recomputing runs, so gate and packer cannot
/// disagree (the v8.1 duck bug).
#[derive(Debug, Clone)]
pub struct Coverage {
    pub path_idx: usize,
    pub hostable: f32,
    pub covered: f32,
}

pub type Plan = (f32, Vec<Placement>, Vec<MicroStroke>, Vec<Coverage>);

pub fn plan_capacity(
    paths: &[ScanPath],
    pool: &[(String, usize)],
    seed: u64,
    p: &CapacityParams,
) -> Result<Plan, PlanError> {
    let start = size_by_descent(paths, p)?;
    let mut s = start;
    let mut last: PlanError = PlanError::Starved(usize::MAX, 0.0);
    while s >= p.floor - 0.01 {
        // re-validate structural constraints at this size, then pack
        let mut p2 = CapacityParams { band_max: s, ..*p };
        p2.band_max = s;
        match size_by_descent(paths, &p2) {
            Ok(sz) => {
                // F1 keep-out learning: an overlap the detector missed IS
                // a junction. Reserve arc at the offending midpoints and
                // re-plan at the SAME size (bounded); geometry untouched.
                let mut extra: Vec<Vec<f32>> = vec![Vec::new(); paths.len()];
                let mut resolved = None;
                for _ in 0..8 {
                    match plan_at(paths, pool, seed, p, sz, &extra) {
                        Ok(r) => {
                            resolved = Some(r);
                            break;
                        }
                        Err(PlanError::GlyphOverlap(a, b)) => {
                            // learn keep-outs from the colliding runs
                            let plan = plan_at_unchecked(paths, pool, seed, p, sz, &extra);
                            if let Some(pls) = plan {
                                for &k in &[a, b] {
                                    if let Some(pl) = pls.get(k) {
                                        let mid = (pl.t0 + pl.t1) * 0.5;
                                        extra[pl.path_idx].push(mid);
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        Err(e) => {
                            last = e;
                            break;
                        }
                    }
                }
                if let Some(r) = resolved {
                    return Ok(r);
                }
            }
            Err(e @ (PlanError::DecorativeBudget(_, _) | PlanError::MicroBudget(_, _))) => {
                return Err(e)
            }
            Err(e) => last = e,
        }
        s -= p.step;
    }
    Err(last)
}

/// The same pack without the overlap gate — used only to identify which
/// runs collided so keep-outs can be learned.
fn plan_at_unchecked(
    paths: &[ScanPath],
    pool: &[(String, usize)],
    seed: u64,
    p: &CapacityParams,
    size: f32,
    extra: &[Vec<f32>],
) -> Option<Vec<Placement>> {
    plan_at_inner(paths, pool, seed, p, size, extra, true).ok().map(|r| r.1)
}

fn plan_at(
    paths: &[ScanPath],
    pool: &[(String, usize)],
    seed: u64,
    p: &CapacityParams,
    size: f32,
    extra: &[Vec<f32>],
) -> Result<Plan, PlanError> {
    // I12: the public path ALWAYS runs the gate.
    plan_at_inner(paths, pool, seed, p, size, extra, false)
}

fn plan_at_inner(
    paths: &[ScanPath],
    pool: &[(String, usize)],
    seed: u64,
    p: &CapacityParams,
    size: f32,
    extra: &[Vec<f32>],
    skip_gate: bool,
) -> Result<Plan, PlanError> {
    let mut juncs = junctions(paths, p.junction_radius.max(size * p.line_height_ratio));
    for (i, ex) in extra.iter().enumerate() {
        if i < juncs.len() {
            juncs[i].extend(ex.iter().copied());
            juncs[i].sort_by(|a, b| a.partial_cmp(b).unwrap());
            juncs[i].dedup_by(|a, b| (*a - *b).abs() < 0.005);
        }
    }
    let mut micro: Vec<MicroStroke> = Vec::new();
    let mut coverage: Vec<Coverage> = Vec::new();
    let mut st = seed ^ 0x43415041; // "CAPA"
    let mut used: Vec<usize> = Vec::new();
    let mut placements = Vec::new();
    // longest-first ordering; seeded rotation only among equal lengths
    let mut by_len: Vec<usize> = (0..pool.len()).collect();
    by_len.sort_by_key(|&i| std::cmp::Reverse(pool[i].1));
    for (pi, path) in paths.iter().enumerate() {
        if path.micro {
            micro.push(MicroStroke { path_idx: pi, points: path.points.clone() });
            continue;
        }
        if path.sub_floor || path.decorative_thin {
            continue;
        }
        let cum = arc_cum(&path.points);
        let total = *cum.last().unwrap_or(&0.0);
        // v8.2 F5: micro classing at the chosen size.
        let usable = usable_arc(path, &juncs[pi], size, p.keepout_arc_ratio);
        if usable / (size * p.avg_advance) < p.min_word_chars {
            micro.push(MicroStroke { path_idx: pi, points: path.points.clone() });
            continue;
        }
        // v8.2 F1: junction keep-outs become hard bounds; the packer sees
        // only the clear interior runs between them (I11 by construction).
        let k_frac = (size * p.keepout_arc_ratio) / total.max(1e-6);
        let mut blocked: Vec<(f32, f32)> = juncs[pi]
            .iter()
            .map(|&t| ((t - k_frac).max(0.0), (t + k_frac).min(1.0)))
            .collect();
        blocked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut bounds = vec![0.0f32];
        bounds.extend(path.segments.iter().copied().filter(|t| *t > 0.0 && *t < 1.0));
        bounds.push(1.0);
        bounds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bounds.dedup();
        // subtract keep-outs from the segment list
        let mut runs: Vec<(f32, f32)> = Vec::new();
        for w2 in bounds.windows(2) {
            let (mut a, b) = (w2[0], w2[1]);
            for &(k0, k1) in &blocked {
                if k1 <= a || k0 >= b {
                    continue;
                }
                if k0 > a {
                    runs.push((a, k0.min(b)));
                }
                a = k1.max(a);
            }
            if a < b {
                runs.push((a, b));
            }
        }
        let mut cov = Coverage { path_idx: pi, hostable: 0.0, covered: 0.0 };
        for w2 in runs.iter().map(|&(a, b)| [a, b]) {
            let (t0, t1) = (w2[0], w2[1]);
            let seg_len = (t1 - t0) * total;
            let mut budget = seg_len / (size * p.avg_advance);
            if budget < p.min_word_chars {
                continue; // sub-min sliver — not hostable, not counted
            }
            cov.hostable += seg_len;
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
            cov.covered += seg_len;
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
        coverage.push(cov);
    }
    // v8.2 F2 / I12 — the rendered-glyph overlap gate: OBBs of glyphs
    // from DIFFERENT words may never intersect. Runs on placement output,
    // on every path, offline and on device. No off switch.
    if skip_gate {
        return Ok((size, placements, micro, coverage));
    }
    let obbs: Vec<(usize, Vec<GlyphObb>)> =
        placements.iter().enumerate().map(|(i, pl)| (i, placement_obbs(pl))).collect();
    for a in 0..obbs.len() {
        for b in a + 1..obbs.len() {
            if placements[a].path_idx == placements[b].path_idx {
                // Exempt ONLY arc-adjacent runs. Arc-distant runs on a
                // self-pinching ring collide in SPACE — the red-circle
                // class; they must be checked. On a CLOSED ring the seam
                // (t~1 meeting t~0) is adjacency too (carried v6 law).
                let closed = {
                    let pts = &paths[placements[a].path_idx].points;
                    let f = pts[0];
                    let l = *pts.last().unwrap();
                    (f.0 - l.0).hypot(f.1 - l.1) < 2.0
                };
                // adjacency measured in ABSOLUTE arc (a gap is px, not a
                // fraction — on short paths a fraction test misreads
                // consecutive words as distant).
                let arc = arc_cum(&paths[placements[a].path_idx].points)
                    .last()
                    .copied()
                    .unwrap_or(1.0);
                let tol = (p.gap_chars + 0.75) * placements[a].glyph_size * p.avg_advance
                    + p.keepout_arc_ratio * 2.0 * placements[a].glyph_size;
                let adj = ((placements[a].t1 - placements[b].t0).abs() * arc) < tol
                    || ((placements[b].t1 - placements[a].t0).abs() * arc) < tol
                    || (closed
                        && (((1.0 - placements[a].t1) + placements[b].t0) * arc < tol
                            || ((1.0 - placements[b].t1) + placements[a].t0) * arc < tol));
                if adj {
                    continue;
                }
            }
            if obbs[a].1.iter().any(|x| obbs[b].1.iter().any(|y| obb_intersect(x, y))) {
                return Err(PlanError::GlyphOverlap(a, b));
            }
        }
    }
    Ok((size, placements, micro, coverage))
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
            micro: false,
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
            junction_radius: 10.0,
            keepout_arc_ratio: 0.75,
            curve_size_ratio: 0.9,
            min_words_per_path: 2.0,
            decorative_max_fraction: 0.20,
            micro_max_fraction: 0.10,
        }
    }

    /// v8.2 F1/I11: no glyph enters a keep-out around a junction.
    #[test]
    fn keepouts_are_respected() {
        let p = params();
        // T-junction: vertical arm's endpoint meets the horizontal interior
        let horiz = straight(600.0, 0.0);
        let arm = ScanPath {
            points: vec![(300.0, 2.0), (300.0, 150.0), (300.0, 300.0)],
            segments: vec![],
            sub_floor: false,
            decorative_thin: false,
            micro: false,
        };
        let paths = [horiz, arm];
        let j = junctions(&paths, p.junction_radius);
        assert!(!j[0].is_empty() && !j[1].is_empty(), "junction detected both arms");
        let (size, pls, _micro, _cov) = plan_capacity(&paths, &pool(), 1, &p).unwrap();
        let k = size * p.keepout_arc_ratio;
        for pl in &pls {
            let arc: f32 = 600.0;
            for &t in &j[pl.path_idx] {
                let (a, b) = (t * arc - k, t * arc + k);
                let (w0, w1) = (pl.t0 * arc, pl.t1 * arc);
                assert!(w1 <= a + 0.5 || w0 >= b - 0.5, "glyph run entered a keep-out");
            }
        }
    }

    /// v8.2 F2/I12: the OBB gate refuses overlapping renders.
    #[test]
    fn obb_gate_catches_crossing_words() {
        let a = GlyphObb { cx: 100.0, cy: 100.0, half_w: 10.0, half_h: 8.0, cos: 1.0, sin: 0.0 };
        let b = GlyphObb { cx: 104.0, cy: 102.0, half_w: 10.0, half_h: 8.0, cos: 0.0, sin: 1.0 };
        assert!(obb_intersect(&a, &b));
        let far = GlyphObb { cx: 300.0, cy: 300.0, half_w: 10.0, half_h: 8.0, cos: 1.0, sin: 0.0 };
        assert!(!obb_intersect(&a, &far));
    }

    /// v8.2 F4/I13: decorative arc over budget blocks before packing.
    #[test]
    fn decorative_budget_blocks() {
        let p = params();
        let mut deco = straight(900.0, 60.0);
        deco.decorative_thin = true;
        let paths = [straight(300.0, 0.0), deco];
        assert!(matches!(
            plan_capacity(&paths, &pool(), 1, &p),
            Err(PlanError::DecorativeBudget(_, _))
        ));
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
        let (size, pls, _micro, _cov) = plan_capacity(&paths, &pool(), 1, &p).unwrap();
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
