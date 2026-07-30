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
    pub const EXPERT: Budget = Budget { max_points: 128, max_interior: 60, bands: 4 };
}

/// Trace `gray` (row-major, w×h, 0=black) into outline paths.
/// The subject is taken as the darker Otsu class unless it covers >70% of
/// the frame (then the classes swap — dark backgrounds happen).
pub fn trace(gray: &[u8], w: usize, h: usize, budget: Budget) -> TraceResult {
    assert_eq!(gray.len(), w * h, "gray buffer must be w*h");
    let threshold = otsu(gray);
    let mut mask: Vec<bool> = gray.iter().map(|&v| v <= threshold).collect();
    let dark_share = mask.iter().filter(|&&m| m).count() as f32 / (w * h) as f32;
    if dark_share > 0.70 {
        for m in mask.iter_mut() {
            *m = !*m;
        }
    }
    keep_largest_component(&mut mask, w, h);
    let contour = moore_contour(&mask, w, h);
    let eps0 = 0.004 * w as f32;
    let silhouette = simplify_to_budget(&contour, eps0, budget.max_points);
    let deviation = max_deviation(&contour, &silhouette) / w as f32;
    let coverage = perimeter_coverage(&contour, &silhouette, 0.02 * w as f32);

    let mut paths = vec![path_of(silhouette, gray, w, budget.bands, true)];
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
            paths.push(path_of(p, gray, w, budget.bands, false));
        }
    }
    TraceResult { paths, deviation_frac: deviation, coverage_frac: coverage, threshold }
}

fn path_of(points: Vec<(f32, f32)>, gray: &[u8], w: usize, bands: u8, silhouette: bool) -> TracedPath {
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
    TracedPath { points, band, scale_class, silhouette }
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

