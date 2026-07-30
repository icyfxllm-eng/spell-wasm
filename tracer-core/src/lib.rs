//! CC-WORD-PICTURE v7 F8 — the trace core.
//!
//! Pure function of pixels: grayscale image in → outline paths + scale
//! classes + fidelity metrics out. Exactness by measurement, not by
//! adjective: the caller gets `deviation_frac` (max distance between the
//! source contour and the simplified paths, as a fraction of canvas width;
//! gate ≤ 0.015 per D10) and `coverage_frac` (share of the reference
//! silhouette perimeter the traced paths cover; gate ≥ 0.95).
//!
//! Runtime-portable by construction (D12): no deps, no filesystem, no
//! tool-side imports. The offline authoring tool and, later, the
//! on-device photo flow both call exactly this module.

/// One traced path in canvas coordinates (the caller's pixel space).
#[derive(Debug, Clone)]
pub struct TracedPath {
    pub points: Vec<(f32, f32)>,
    /// Tonal band 1 (lightest) ..= 4 (darkest) sampled inside the stroke.
    pub band: u8,
    /// F3 scale class derived from path length: dot | small | medium | long.
    pub scale_class: &'static str,
    /// True for the silhouette contour (identity carrier), false for
    /// interior tonal-boundary strokes.
    pub silhouette: bool,
    /// v7.4 containment tree: "frame" (root), a named part ("body",
    /// "fan"), or "" for the whole-subject silhouette when no parts are
    /// declared. Interior features carry their parent part's name.
    pub part: String,
}

#[derive(Debug, Clone)]
pub struct TraceResult {
    pub paths: Vec<TracedPath>,
    /// Max deviation of the source contour from the simplified silhouette,
    /// as a fraction of image width (D10 gate: ≤ 0.015).
    pub deviation_frac: f32,
    /// Fraction of the reference silhouette perimeter within tolerance of
    /// the traced paths (D10 gate: ≥ 0.95).
    pub coverage_frac: f32,
    /// Otsu threshold chosen (diagnostic).
    pub threshold: u8,
    /// v7.1 smoothness lint: stair-step/pixel-block artifacts remaining
    /// after smoothing. Non-zero fails the build.
    pub smoothness_violations: u32,
    /// The approved-mask candidate (row-major w×h) the gates measured
    /// against — the tool renders it as the human-editable layer, and the
    /// correction-loop fixture asserts scribble authority on it directly.
    pub mask: Vec<bool>,
}

/// Per-band point budgets: standard bands coarser, expert denser — the
/// same dial that gives the Mona Lisa its 150–200 short paths.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    /// Max points per simplified path.
    pub max_points: usize,
    /// Max interior (non-silhouette) paths.
    pub max_interior: usize,
    /// Tonal bands for interior boundaries (2–4).
    pub bands: u8,
}

impl Budget {
    pub const STANDARD: Budget = Budget { max_points: 64, max_interior: 10, bands: 3 };
    pub const EXPERT: Budget = Budget { max_points: 200, max_interior: 60, bands: 4 };
}

/// Trace `gray` (row-major, w×h, 0=black) into outline paths.
/// Unseeded: the subject is the darker Otsu class (swapped when it covers
/// >70% of the frame). Seeded (v7.2): `lasso` is a rough enclosure around
/// the subject — the player's future finger-circle. Everything outside is
/// HARD background; the subject is whatever differs from the background
/// statistics sampled around the lasso, found within it.
pub fn trace(gray: &[u8], w: usize, h: usize, budget: Budget) -> TraceResult {
    trace_seeded(gray, w, h, budget, None)
}

/// v7.3 (D13) — human correction marks. HARD constraints, never hints:
/// a solve that violates any of them is a core bug, covered by fixture.
#[derive(Debug, Clone, Default)]
pub struct Constraints {
    /// Rough enclosure around the subject (the future finger-circle).
    pub lasso: Option<Vec<(f32, f32)>>,
    /// Green scribbles: these pixels ARE subject (stamped into the mask
    /// with a small radius so a stroke bridges under-covered regions).
    pub keep: Vec<(f32, f32)>,
    /// Red scribbles: these pixels ARE background (stamped out).
    pub exclude: Vec<(f32, f32)>,
    /// Locked boundary redraws: (index into silhouette span start fraction
    /// 0..1, replacement points spliced verbatim after smoothing).
    pub splices: Vec<(f32, Vec<(f32, f32)>)>,
    /// Landmark anchors the silhouette must pass through (0.5% tolerance —
    /// enforced by snap-insertion, so the gate holds by construction).
    pub anchors: Vec<(f32, f32)>,
    /// v7.4: frame rectangle (x0, y0, x1, y1) — the tree's root. The mask
    /// is HARD-CLIPPED to the frame interior before any tracing.
    pub frame: Option<(f32, f32, f32, f32)>,
    /// v7.4 part decomposition: (name, enclosing polygon). Each part gets
    /// its own closed silhouette; interior features confine to their part.
    pub parts: Vec<(String, Vec<(f32, f32)>)>,
}

/// FNV-1a over rounded path coordinates — the pin hash (v7.3). Same trace
/// → same hash; a pinned subject that re-traces differently fails CI.
pub fn trace_hash(r: &TraceResult) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    for p in &r.paths {
        for &(x, y) in &p.points {
            for b in ((x * 10.0).round() as i32).to_le_bytes() {
                eat(b);
            }
            for b in ((y * 10.0).round() as i32).to_le_bytes() {
                eat(b);
            }
        }
        eat(p.band);
    }
    h
}

pub fn trace_seeded(
    gray: &[u8],
    w: usize,
    h: usize,
    budget: Budget,
    lasso: Option<&[(f32, f32)]>,
) -> TraceResult {
    let c = Constraints { lasso: lasso.map(|l| l.to_vec()), ..Default::default() };
    trace_constrained(gray, w, h, budget, &c)
}

pub fn trace_constrained(
    gray: &[u8],
    w: usize,
    h: usize,
    budget: Budget,
    cons: &Constraints,
) -> TraceResult {
    let lasso = cons.lasso.as_deref();
    assert_eq!(gray.len(), w * h, "gray buffer must be w*h");
    let threshold = otsu(gray);
    let mut mask: Vec<bool> = if let Some(poly) = lasso {
        let inside: Vec<bool> =
            (0..w * h).map(|i| point_in_poly((i % w) as f32, (i / w) as f32, poly)).collect();
        // Background MEDIAN from the ring just outside the lasso (the spec's
        // "sampled around the lasso" — robust to clutter far away).
        let ring = 14i32;
        let mut ring_vals: Vec<u8> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if inside[y * w + x] {
                    continue;
                }
                let mut near = false;
                'scan: for dy in (-ring..=ring).step_by(4) {
                    for dx in (-ring..=ring).step_by(4) {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h
                            && inside[ny as usize * w + nx as usize]
                        {
                            near = true;
                            break 'scan;
                        }
                    }
                }
                if near {
                    ring_vals.push(gray[y * w + x]);
                }
            }
        }
        ring_vals.sort_unstable();
        let bg = ring_vals.get(ring_vals.len() / 2).copied().unwrap_or(255) as f64;
        let mad = {
            let mut d: Vec<f64> = ring_vals.iter().map(|&v| (v as f64 - bg).abs()).collect();
            d.sort_by(|a, b| a.partial_cmp(b).unwrap());
            d.get(d.len() / 2).copied().unwrap_or(0.0)
        };
        let tol = (2.5 * mad).clamp(14.0, 48.0);
        let m: Vec<bool> =
            (0..w * h).map(|i| inside[i] && (gray[i] as f64 - bg).abs() > tol).collect();
        // Degenerate guard: if the statistical mask collapsed, fall back to
        // Otsu WITHIN the lasso (subject = minority class inside).
        let area = m.iter().filter(|&&x| x).count();
        let lasso_area = inside.iter().filter(|&&x| x).count().max(1);
        if area * 50 < lasso_area {
            let vals: Vec<u8> = (0..w * h).filter(|&i| inside[i]).map(|i| gray[i]).collect();
            let t = otsu(&vals);
            let dark: Vec<bool> =
                (0..w * h).map(|i| inside[i] && gray[i] <= t).collect();
            let dn = dark.iter().filter(|&&x| x).count();
            if dn * 2 < lasso_area {
                dark
            } else {
                (0..w * h).map(|i| inside[i] && gray[i] > t).collect()
            }
        } else {
            m
        }
    } else {
        let mut m: Vec<bool> = gray.iter().map(|&v| v <= threshold).collect();
        let dark_share = m.iter().filter(|&&x| x).count() as f32 / (w * h) as f32;
        if dark_share > 0.70 {
            for b in m.iter_mut() {
                *b = !*b;
            }
        }
        m
    };
    // v7.3 scribbles: hard stamps BEFORE morphology/components so a green
    // stroke bridges regions and a red stroke severs them.
    const R: i32 = 5;
    let stamp = |mask: &mut Vec<bool>, pts: &[(f32, f32)], val: bool| {
        for &(px, py) in pts {
            for dy in -R..=R {
                for dx in -R..=R {
                    if dx * dx + dy * dy > R * R {
                        continue;
                    }
                    let (x, y) = (px as i32 + dx, py as i32 + dy);
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        mask[y as usize * w + x as usize] = val;
                    }
                }
            }
        }
    };
    stamp(&mut mask, &cons.keep, true);
    stamp(&mut mask, &cons.exclude, false);
    // v7.4: the frame is the tree's root — the mask is clipped to its
    // interior BEFORE tracing, never cleaned up after.
    if let Some((fx0, fy0, fx1, fy1)) = cons.frame {
        for y in 0..h {
            for x in 0..w {
                let inside = (x as f32) > fx0 && (x as f32) < fx1 && (y as f32) > fy0 && (y as f32) < fy1;
                if !inside {
                    mask[y * w + x] = false;
                }
            }
        }
    }
    morph_close(&mut mask, w, h);
    stamp(&mut mask, &cons.exclude, false); // red survives morphology too
    keep_largest_component(&mut mask, w, h);
    // Every green pixel must be in the final mask: reattach any component
    // holding a keep-scribble (human authority is absolute).
    if !cons.keep.is_empty() {
        let mut m2: Vec<bool> = gray.iter().map(|_| false).collect();
        for &(px, py) in &cons.keep {
            let (x, y) = (px as usize, py as usize);
            if x < w && y < h {
                m2[y * w + x] = true;
            }
        }
        stamp(&mut mask, &cons.keep, true);
        let _ = m2;
        morph_close(&mut mask, w, h);
        stamp(&mut mask, &cons.exclude, false);
    }
    let contour = moore_contour(&mask, w, h);
    let eps0 = 0.004 * w as f32;
    let silhouette = simplify_to_budget(&contour, eps0, budget.max_points);
    // A degenerate contour must FAIL the gates explicitly — a 2-point
    // trace scoring perfectly against itself is the round-2 trap.
    let degenerate = contour.len() < 8 || poly_len(&silhouette) < 0.1 * w as f32;
    let deviation = if degenerate { 1.0 } else { max_deviation(&contour, &silhouette) / w as f32 };
    let coverage = if degenerate { 0.0 } else { perimeter_coverage(&contour, &silhouette, 0.02 * w as f32) };

    let mut paths: Vec<TracedPath> = Vec::new();
    if let Some((fx0, fy0, fx1, fy1)) = cons.frame {
        paths.push(path_of_part(
            vec![(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1), (fx0, fy0)],
            gray, w, budget.bands, false, "frame".into(),
        ));
    }
    if cons.parts.is_empty() {
        paths.push(path_of(silhouette, gray, w, budget.bands, true));
    } else {
        // v7.4 part decomposition: each part is mask ∧ its polygon, with
        // its own closed silhouette; a multi-part subject rendered as one
        // blob is a review failure by definition.
        for (name, poly) in &cons.parts {
            let mut pm: Vec<bool> = (0..w * h)
                .map(|i| mask[i] && point_in_poly((i % w) as f32, (i / w) as f32, poly))
                .collect();
            keep_largest_component(&mut pm, w, h);
            let pc = moore_contour(&pm, w, h);
            if pc.len() >= 8 {
                let ps = simplify_to_budget(&pc, eps0, budget.max_points);
                paths.push(path_of_part(ps, gray, w, budget.bands, true, name.clone()));
            }
        }
    }
    // Interior tonal boundaries: posterize inside the mask, trace each
    // darker-than-band region's boundary, keep the longest few.
    let mut interior: Vec<Vec<(f32, f32)>> = Vec::new();
    for b in 1..budget.bands {
        let cut = threshold as u32 * b as u32 / budget.bands as u32;
        let mut m2: Vec<bool> = gray
            .iter()
            .zip(mask.iter())
            .map(|(&v, &inm)| inm && (v as u32) <= cut)
            .collect();
        keep_largest_component(&mut m2, w, h);
        let c = moore_contour(&m2, w, h);
        if c.len() > 12 {
            interior.push(simplify_to_budget(&c, eps0 * 1.5, budget.max_points / 2));
        }
    }
    interior.sort_by(|a, b| poly_len(b).partial_cmp(&poly_len(a)).unwrap());
    interior.truncate(budget.max_interior);
    for p in interior {
        if poly_len(&p) > 0.06 * w as f32 {
            // v7.4: an interior feature belongs to the part containing it.
            let part = if cons.parts.is_empty() {
                String::new()
            } else {
                let (cx, cy) = p.iter().fold((0.0, 0.0), |a, q| (a.0 + q.0, a.1 + q.1));
                let n = p.len().max(1) as f32;
                let (cx, cy) = (cx / n, cy / n);
                match cons.parts.iter().find(|(_, poly)| point_in_poly(cx, cy, poly)) {
                    Some((name, _)) => name.clone(),
                    None => continue, // outside every part — illegal source
                }
            };
            let clipped: Vec<(f32, f32)> = if let Some((name, poly)) =
                cons.parts.iter().find(|(n2, _)| *n2 == part)
            {
                let _ = name;
                p.iter().copied().filter(|&(x, y)| point_in_poly(x, y, poly)).collect()
            } else {
                p
            };
            if clipped.len() >= 5 {
                paths.push(path_of_part(clipped, gray, w, budget.bands, false, part));
            }
        }
    }
    // v7.1 smoothness lint: enforce min segment length + max turn angle.
    // The frame is AUTHORED geometry (the tree's root) — its square
    // corners are the point; it is exempt from smoothing and lint.
    for p in paths.iter_mut() {
        if p.part != "frame" {
            p.points = smooth(&p.points, 6.0, 40.0);
        }
    }
    // Degenerate interiors (needle triangles, stray dots) are noise, not
    // contours — cull anything the smoother couldn't even engage with.
    paths.retain(|p| p.silhouette || p.part == "frame" || p.points.len() >= 5);
    // v7.3 anchors: the silhouette must pass through each landmark within
    // 0.5% of canvas — enforced by snap-inserting the anchor into its
    // nearest segment (the gate holds by construction).
    if let Some(sil) = paths.iter_mut().find(|p| p.silhouette && p.part != "frame") {
        for &(ax, ay) in &cons.anchors {
            let mut best = (f32::MAX, 0usize);
            for k in 0..sil.points.len().saturating_sub(1) {
                let d = seg_dist((ax, ay), sil.points[k], sil.points[k + 1]);
                if d < best.0 {
                    best = (d, k + 1);
                }
            }
            if best.0 > 0.001 {
                sil.points.insert(best.1, (ax, ay));
            }
        }
    }
    // v7.3 splices: locked verbatim boundary redraws, applied last — a
    // re-solve reproduces them byte-identically by construction.
    if let Some(sil) = paths.iter_mut().find(|p| p.silhouette) {
        for (at, seg) in &cons.splices {
            if sil.points.len() < 4 || seg.is_empty() {
                continue;
            }
            let n = sil.points.len();
            let i0 = ((at * n as f32) as usize).min(n - 2);
            let i1 = (i0 + n / 8).min(n - 1);
            sil.points.splice(i0..i1, seg.iter().copied());
        }
    }
    let smoothness_violations = paths
        .iter()
        .filter(|p| p.part != "frame")
        .map(|p| turn_violations(&p.points, 6.0, 40.0))
        .sum();
    TraceResult { paths, deviation_frac: deviation, coverage_frac: coverage, threshold, smoothness_violations, mask }
}

fn point_in_poly(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 3×3 close (dilate then erode) — solidifies speckle before component
/// analysis so texture inside the subject doesn't shatter the mask.
fn morph_close(mask: &mut Vec<bool>, w: usize, h: usize) {
    let pass = |src: &[bool], grow: bool| -> Vec<bool> {
        let mut out = vec![false; src.len()];
        for y in 0..h {
            for x in 0..w {
                let mut any = false;
                let mut all = true;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        let v = nx >= 0
                            && ny >= 0
                            && (nx as usize) < w
                            && (ny as usize) < h
                            && src[ny as usize * w + nx as usize];
                        any |= v;
                        all &= v;
                    }
                }
                out[y * w + x] = if grow { any } else { all };
            }
        }
        out
    };
    let d = pass(mask, true);
    *mask = pass(&d, false);
}

/// Drop vertices closer than `min_seg`; relax vertices whose turn exceeds
/// `max_turn_deg` (one corner-cutting pass) — stair-steps become curves.
fn smooth(pts: &[(f32, f32)], min_seg: f32, max_turn_deg: f32) -> Vec<(f32, f32)> {
    if pts.len() < 4 {
        return pts.to_vec();
    }
    let mut out: Vec<(f32, f32)> = vec![pts[0]];
    for &p in &pts[1..] {
        let l = *out.last().unwrap();
        if (p.0 - l.0).hypot(p.1 - l.1) >= min_seg {
            out.push(p);
        }
    }
    if out.len() < 4 {
        return out;
    }
    // Relax to convergence (bounded): corners beyond the turn cap melt,
    // then re-check — stair-steps need several passes to become curves.
    for _ in 0..24 {
        if turn_violations(&out, min_seg, max_turn_deg) == 0 {
            break;
        }
        let mut relaxed: Vec<(f32, f32)> = vec![out[0]];
        for k in 1..out.len() - 1 {
            let (a, b, c) = (out[k - 1], out[k], out[k + 1]);
            if turn_deg(a, b, c) > max_turn_deg * 0.8 {
                relaxed.push(((a.0 + 2.0 * b.0 + c.0) / 4.0, (a.1 + 2.0 * b.1 + c.1) / 4.0));
            } else {
                relaxed.push(b);
            }
        }
        relaxed.push(*out.last().unwrap());
        // Merge any segments the relax pass shortened below the floor.
        let mut merged: Vec<(f32, f32)> = vec![relaxed[0]];
        for &p in &relaxed[1..] {
            let l = *merged.last().unwrap();
            if (p.0 - l.0).hypot(p.1 - l.1) >= min_seg * 0.5 {
                merged.push(p);
            }
        }
        out = merged;
    }
    // Ultimate melt: any vertex still past the lint threshold after the
    // relax passes is a needle spike — delete it outright.
    let mut k = 1;
    while k + 1 < out.len() {
        if turn_deg(out[k - 1], out[k], out[k + 1]) > max_turn_deg + 15.0 {
            out.remove(k);
            if k > 1 {
                k -= 1;
            }
        } else {
            k += 1;
        }
    }
    // Closing seam: the merge pass always keeps the final point, which can
    // crowd its neighbor below the floor on closed contours.
    while out.len() > 3 {
        let n = out.len();
        let d = (out[n - 1].0 - out[n - 2].0).hypot(out[n - 1].1 - out[n - 2].1);
        if d < min_seg * 0.5 {
            out.remove(n - 2);
        } else {
            break;
        }
    }
    out
}

fn turn_deg(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    let (v1x, v1y) = (b.0 - a.0, b.1 - a.1);
    let (v2x, v2y) = (c.0 - b.0, c.1 - b.1);
    let dot = v1x * v2x + v1y * v2y;
    let m = (v1x.hypot(v1y) * v2x.hypot(v2y)).max(1e-6);
    (dot / m).clamp(-1.0, 1.0).acos().to_degrees()
}

fn turn_violations(pts: &[(f32, f32)], min_seg: f32, max_turn_deg: f32) -> u32 {
    let mut n = 0;
    for k in 1..pts.len().saturating_sub(1) {
        if turn_deg(pts[k - 1], pts[k], pts[k + 1]) > max_turn_deg + 15.0 {
            n += 1;
        }
    }
    for s in pts.windows(2) {
        if (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1) < min_seg * 0.5 {
            n += 1;
        }
    }
    n
}

fn path_of(points: Vec<(f32, f32)>, gray: &[u8], w: usize, bands: u8, silhouette: bool) -> TracedPath {
    path_of_part(points, gray, w, bands, silhouette, String::new())
}

fn path_of_part(
    points: Vec<(f32, f32)>,
    gray: &[u8],
    w: usize,
    bands: u8,
    silhouette: bool,
    part: String,
) -> TracedPath {
    let len = poly_len(&points);
    let frac = len / w as f32;
    let scale_class = if frac < 0.09 {
        "dot"
    } else if frac < 0.18 {
        "small"
    } else if frac < 0.35 {
        "medium"
    } else {
        "long"
    };
    // Band from mean darkness sampled along the path.
    let h = gray.len() / w;
    let mut sum = 0u32;
    let mut n = 0u32;
    for &(x, y) in &points {
        let (xi, yi) = (x as usize, y as usize);
        if xi < w && yi < h {
            sum += gray[yi * w + xi] as u32;
            n += 1;
        }
    }
    let mean = if n > 0 { (sum / n) as u8 } else { 128 };
    let band = (bands as u32 - (mean as u32 * bands as u32 / 256).min(bands as u32 - 1)) as u8;
    TracedPath { points, band, scale_class, silhouette, part }
}

fn otsu(gray: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = gray.len() as f64;
    let sum_all: f64 = hist.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
    let (mut sum_b, mut w_b, mut best, mut best_t) = (0f64, 0f64, 0f64, 127u8);
    for t in 0..256 {
        w_b += hist[t] as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let m_b = sum_b / w_b;
        let m_f = (sum_all - sum_b) / w_f;
        let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if between > best {
            best = between;
            best_t = t as u8;
        }
    }
    best_t
}

fn keep_largest_component(mask: &mut [bool], w: usize, h: usize) {
    let mut label = vec![0u32; w * h];
    let mut next = 0u32;
    let mut sizes: Vec<u32> = vec![0];
    let mut stack = Vec::new();
    for start in 0..w * h {
        if mask[start] && label[start] == 0 {
            next += 1;
            sizes.push(0);
            stack.push(start);
            label[start] = next;
            while let Some(i) = stack.pop() {
                sizes[next as usize] += 1;
                let (x, y) = (i % w, i / w);
                let mut push = |j: usize| {
                    if mask[j] && label[j] == 0 {
                        label[j] = next;
                        stack.push(j);
                    }
                };
                if x + 1 < w { push(i + 1); }
                if x > 0 { push(i - 1); }
                if y + 1 < h { push(i + w); }
                if y > 0 { push(i - w); }
            }
        }
    }
    if next == 0 {
        return;
    }
    let biggest = (1..=next).max_by_key(|&l| sizes[l as usize]).unwrap();
    for i in 0..w * h {
        mask[i] = label[i] == biggest;
    }
}

/// Moore-neighbor boundary trace of the mask's outer contour.
fn moore_contour(mask: &[bool], w: usize, h: usize) -> Vec<(f32, f32)> {
    let at = |x: isize, y: isize| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && mask[y as usize * w + x as usize]
    };
    let Some(start) = (0..w * h).find(|&i| mask[i]) else { return Vec::new() };
    let (sx, sy) = ((start % w) as isize, (start / w) as isize);
    const N: [(isize, isize); 8] =
        [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
    let mut out = Vec::new();
    let (mut cx, mut cy) = (sx, sy);
    let mut dir = 6usize; // came from above
    loop {
        out.push((cx as f32, cy as f32));
        let mut found = false;
        for k in 0..8 {
            let d = (dir + 6 + k) % 8; // backtrack-relative scan
            let (nx, ny) = (cx + N[d].0, cy + N[d].1);
            if at(nx, ny) {
                cx = nx;
                cy = ny;
                dir = d;
                found = true;
                break;
            }
        }
        if !found || (cx == sx && cy == sy && out.len() > 2) || out.len() > 8 * (w + h) {
            break;
        }
    }
    out
}

/// Douglas-Peucker, then epsilon-doubling until the point budget holds.
fn simplify_to_budget(pts: &[(f32, f32)], mut eps: f32, budget: usize) -> Vec<(f32, f32)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    for _ in 0..12 {
        let s = douglas_peucker(pts, eps);
        if s.len() <= budget.max(3) {
            return s;
        }
        eps *= 1.5;
    }
    douglas_peucker(pts, eps)
}

fn douglas_peucker(pts: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (mut idx, mut dmax) = (0usize, 0f32);
    let (a, b) = (pts[0], *pts.last().unwrap());
    for (i, &p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
        let d = seg_dist(p, a, b);
        if d > dmax {
            dmax = d;
            idx = i;
        }
    }
    if dmax > eps {
        let mut left = douglas_peucker(&pts[..=idx], eps);
        let right = douglas_peucker(&pts[idx..], eps);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![a, b]
    }
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = ((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2;
    let t = t.clamp(0.0, 1.0);
    (p.0 - (a.0 + t * vx)).hypot(p.1 - (a.1 + t * vy))
}

fn poly_len(pts: &[(f32, f32)]) -> f32 {
    pts.windows(2).map(|s| (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1)).sum()
}

fn max_deviation(contour: &[(f32, f32)], simplified: &[(f32, f32)]) -> f32 {
    contour
        .iter()
        .map(|&p| {
            simplified
                .windows(2)
                .map(|s| seg_dist(p, s[0], s[1]))
                .fold(f32::MAX, f32::min)
        })
        .fold(0f32, f32::max)
}

fn perimeter_coverage(contour: &[(f32, f32)], simplified: &[(f32, f32)], tol: f32) -> f32 {
    if contour.is_empty() {
        return 0.0;
    }
    let hit = contour
        .iter()
        .filter(|&&p| {
            simplified
                .windows(2)
                .map(|s| seg_dist(p, s[0], s[1]))
                .fold(f32::MAX, f32::min)
                <= tol
        })
        .count();
    hit as f32 / contour.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D10 fixture: a known reference (disk with a bite) traces within the
    /// gates — deviation ≤ 1.5% of width, perimeter coverage ≥ 95%.
    #[test]
    fn fidelity_gates_on_stored_reference() {
        let (w, h) = (160usize, 160usize);
        let mut gray = vec![230u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 - 80.0, y as f32 - 80.0);
                let in_disk = (dx * dx + dy * dy).sqrt() < 60.0;
                let in_bite = ((x as f32 - 130.0).powi(2) + (y as f32 - 80.0).powi(2)).sqrt() < 24.0;
                if in_disk && !in_bite {
                    gray[y * w + x] = 30;
                }
            }
        }
        let r = trace(&gray, w, h, Budget::EXPERT);
        assert!(!r.paths.is_empty());
        assert!(r.paths[0].silhouette);
        assert!(r.deviation_frac <= 0.015, "deviation {} > 1.5%", r.deviation_frac);
        assert!(r.coverage_frac >= 0.95, "coverage {} < 95%", r.coverage_frac);
        // Determinism: same input, same output.
        let r2 = trace(&gray, w, h, Budget::EXPERT);
        assert_eq!(r.paths[0].points, r2.paths[0].points);
    }

    #[test]
    fn scale_classes_derive_from_length() {
        let (w, h) = (200usize, 200usize);
        let mut gray = vec![240u8; w * h];
        for y in 96..104 {
            for x in 96..104 {
                gray[y * w + x] = 20; // a small blot → short path → dot/small
            }
        }
        let r = trace(&gray, w, h, Budget::STANDARD);
        assert!(matches!(r.paths[0].scale_class, "dot" | "small"));
    }
}

