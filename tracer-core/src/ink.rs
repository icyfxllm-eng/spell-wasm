//! CC-TRACER-V7.5 — the ink branch. For clip-art references the drawn
//! black line IS the ground truth: extract it, skeletonize it, and grade
//! every trace against it. The eval's truth is NEVER derived from the
//! pipeline under test.

/// F1 — reference-class detection. Ink art = bimodal luminance + a high
/// thin-dark-structure ratio (dark pixels whose local stroke width is
/// under ~3% of the bbox diagonal). Returns (is_ink, bimodality, thin_ratio).
pub fn is_ink_art(gray: &[u8], w: usize, h: usize) -> (bool, f32, f32) {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = (w * h) as f32;
    let dark: u32 = hist[..64].iter().sum();
    let light: u32 = hist[192..].iter().sum();
    let mid: u32 = hist[64..192].iter().sum();
    let bimodality = (dark + light) as f32 / total - (mid as f32 / total);
    let mask: Vec<bool> = gray.iter().map(|&v| v < 128).collect();
    let diag = ((w * w + h * h) as f32).sqrt();
    let r = ((0.015 * diag) as usize).max(1);
    // A dark pixel is "thin" if background is reachable within r in one of
    // the two axis directions on both sides (stroke, not blob interior).
    let mut thin = 0usize;
    let mut darkn = 0usize;
    for y in 0..h {
        for x in 0..w {
            if !mask[y * w + x] {
                continue;
            }
            darkn += 1;
            let mut span_x = 0;
            for dx in 1..=r {
                if x >= dx && mask[y * w + x - dx] { span_x += 1 } else { break }
            }
            for dx in 1..=r {
                if x + dx < w && mask[y * w + x + dx] { span_x += 1 } else { break }
            }
            let mut span_y = 0;
            for dy in 1..=r {
                if y >= dy && mask[(y - dy) * w + x] { span_y += 1 } else { break }
            }
            for dy in 1..=r {
                if y + dy < h && mask[(y + dy) * w + x] { span_y += 1 } else { break }
            }
            if span_x.min(span_y) < r {
                thin += 1;
            }
        }
    }
    let thin_ratio = if darkn > 0 { thin as f32 / darkn as f32 } else { 0.0 };
    (bimodality > 0.35 && thin_ratio > 0.6 && darkn > 0, bimodality, thin_ratio)
}

/// F2 — ink mask (adaptive threshold) + Zhang-Suen skeleton.
pub fn ink_skeleton(gray: &[u8], w: usize, h: usize) -> Vec<bool> {
    let t = {
        // adaptive: Otsu clamped toward the dark mode (scans vary).
        let mut hist = [0u32; 256];
        for &v in gray {
            hist[v as usize] += 1;
        }
        let total: f64 = (w * h) as f64;
        let sum_all: f64 = hist.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
        let (mut sum_b, mut w_b, mut best, mut best_t) = (0f64, 0f64, 0f64, 127u8);
        for tt in 0..256 {
            w_b += hist[tt] as f64;
            if w_b == 0.0 { continue; }
            let w_f = total - w_b;
            if w_f == 0.0 { break; }
            sum_b += tt as f64 * hist[tt] as f64;
            let (m_b, m_f) = (sum_b / w_b, (sum_all - sum_b) / w_f);
            let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
            if between > best { best = between; best_t = tt as u8; }
        }
        best_t.min(160)
    };
    let mut img: Vec<bool> = gray.iter().map(|&v| v <= t).collect();
    // Zhang-Suen thinning.
    let idx = |x: usize, y: usize| y * w + x;
    loop {
        let mut changed = false;
        for phase in 0..2 {
            let mut kill = Vec::new();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    if !img[idx(x, y)] { continue; }
                    let p = [
                        img[idx(x, y - 1)], img[idx(x + 1, y - 1)], img[idx(x + 1, y)],
                        img[idx(x + 1, y + 1)], img[idx(x, y + 1)], img[idx(x - 1, y + 1)],
                        img[idx(x - 1, y)], img[idx(x - 1, y - 1)],
                    ];
                    let b: u8 = p.iter().map(|&v| v as u8).sum();
                    if !(2..=6).contains(&b) { continue; }
                    let mut a = 0;
                    for k in 0..8 {
                        if !p[k] && p[(k + 1) % 8] { a += 1; }
                    }
                    if a != 1 { continue; }
                    let (c1, c2) = if phase == 0 {
                        (!(p[0] && p[2] && p[4]), !(p[2] && p[4] && p[6]))
                    } else {
                        (!(p[0] && p[2] && p[6]), !(p[0] && p[4] && p[6]))
                    };
                    if c1 && c2 { kill.push(idx(x, y)); }
                }
            }
            if !kill.is_empty() { changed = true; }
            for i in kill { img[i] = false; }
        }
        if !changed { break; }
    }
    img
}

/// v7.5: for FILLED silhouettes the drawn black outline is the shape's
/// BOUNDARY — the medial axis of a filled dog is a stick figure, but
/// Eric's criterion is "the black outline, every limb". Truth and trace
/// are therefore class-dependent: thin-stroke ink → centerline skeleton;
/// filled ink → boundary pixels (ink with a non-ink 4-neighbour).
pub fn ink_boundary(gray: &[u8], w: usize, h: usize) -> Vec<bool> {
    let t = {
        let mut hist = [0u32; 256];
        for &v in gray { hist[v as usize] += 1; }
        let total: f64 = (w * h) as f64;
        let sum_all: f64 = hist.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
        let (mut sum_b, mut w_b, mut best, mut best_t) = (0f64, 0f64, 0f64, 127u8);
        for tt in 0..256 {
            w_b += hist[tt] as f64;
            if w_b == 0.0 { continue; }
            let w_f = total - w_b;
            if w_f == 0.0 { break; }
            sum_b += tt as f64 * hist[tt] as f64;
            let (m_b, m_f) = (sum_b / w_b, (sum_all - sum_b) / w_f);
            let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
            if between > best { best = between; best_t = tt as u8; }
        }
        best_t.min(160)
    };
    let ink: Vec<bool> = gray.iter().map(|&v| v <= t).collect();
    (0..w * h)
        .map(|i| {
            if !ink[i] { return false; }
            let (x, y) = (i % w, i / w);
            x == 0 || y == 0 || x == w - 1 || y == h - 1
                || !ink[i - 1] || !ink[i + 1] || !ink[i - w] || !ink[i + w]
        })
        .collect()
}

/// Connected skeleton components (8-connected), each a pixel list.
pub fn skeleton_components(skel: &[bool], w: usize, h: usize) -> Vec<Vec<(f32, f32)>> {
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    for start in 0..w * h {
        if !skel[start] || seen[start] { continue; }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            comp.push((x as f32, y as f32));
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h { continue; }
                    let j = ny as usize * w + nx as usize;
                    if skel[j] && !seen[j] { seen[j] = true; stack.push(j); }
                }
            }
        }
        out.push(comp);
    }
    out
}

/// F2 — vectorize the skeleton into junction-preserving centerline
/// paths: spur-prune, then walk degree-2 chains between endpoints and
/// junctions (a leg meeting the body stays connected at the junction
/// pixel); isolated loops walk their cycle. Simplified with eps kept
/// inside the D1 tolerance so the path IS the ink.
pub fn vectorize_skeleton(skel: &[bool], w: usize, h: usize, eps: f32) -> Vec<Vec<(f32, f32)>> {
    let mut sk = skel.to_vec();
    let nbrs = |sk: &[bool], i: usize| -> Vec<usize> {
        let (x, y) = (i % w, i / w);
        let mut out = Vec::new();
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    let j = ny as usize * w + nx as usize;
                    if sk[j] { out.push(j); }
                }
            }
        }
        out
    };
    // Spur pruning: erode endpoint chains shorter than 5px (thinning noise).
    for _ in 0..5 {
        let ends: Vec<usize> =
            (0..w * h).filter(|&i| sk[i] && nbrs(&sk, i).len() == 1).collect();
        let mut removed = false;
        for e in ends {
            // measure the chain from this endpoint
            let (mut chain, mut cur, mut prev) = (vec![e], e, usize::MAX);
            loop {
                let n: Vec<usize> = nbrs(&sk, cur).into_iter().filter(|&j| j != prev).collect();
                if n.len() != 1 || chain.len() > 3 { break; }
                prev = cur;
                cur = n[0];
                chain.push(cur);
            }
            if chain.len() <= 2 && nbrs(&sk, *chain.last().unwrap()).len() > 2 {
                for &i in &chain[..chain.len() - 1] { sk[i] = false; }
                removed = true;
            }
        }
        if !removed { break; }
    }
    let deg = |sk: &[bool], i: usize| nbrs(sk, i).len();
    let mut visited = vec![false; w * h]; // edge-visited marker per pixel
    let mut paths: Vec<Vec<(f32, f32)>> = Vec::new();
    let pt = |i: usize| ((i % w) as f32, (i / w) as f32);
    // Walk chains starting at every endpoint or junction.
    let starts: Vec<usize> =
        (0..w * h).filter(|&i| sk[i] && deg(&sk, i) != 2).collect();
    for &s0 in &starts {
        for n0 in nbrs(&sk, s0) {
            if visited[n0] && deg(&sk, n0) == 2 { continue; }
            let mut chain = vec![pt(s0)];
            let (mut prev, mut cur) = (s0, n0);
            loop {
                chain.push(pt(cur));
                if deg(&sk, cur) != 2 { break; }
                visited[cur] = true;
                let next: Vec<usize> =
                    nbrs(&sk, cur).into_iter().filter(|&j| j != prev).collect();
                if next.is_empty() { break; }
                prev = cur;
                cur = next[0];
            }
            if chain.len() >= 2 {
                paths.push(chain);
            }
        }
    }
    // Isolated cycles: all remaining unvisited degree-2 pixels.
    for i in 0..w * h {
        if !sk[i] || visited[i] || deg(&sk, i) != 2 { continue; }
        let mut chain = vec![pt(i)];
        visited[i] = true;
        let (mut prev, mut cur) = (i, nbrs(&sk, i)[0]);
        while cur != i {
            chain.push(pt(cur));
            visited[cur] = true;
            let next: Vec<usize> = nbrs(&sk, cur).into_iter().filter(|&j| j != prev).collect();
            if next.is_empty() { break; }
            prev = cur;
            cur = next[0];
        }
        chain.push(pt(i));
        if chain.len() >= 6 { paths.push(chain); }
    }
    // Simplify inside the D1 budget; dedupe reverse-direction duplicates.
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    for p in paths {
        let sp = crate::simplify_public(&p, eps);
        if sp.len() < 2 { continue; }
        let key = |v: &Vec<(f32, f32)>| {
            let a = v[0];
            let b = *v.last().unwrap();
            let mid = v[v.len() / 2];
            ((a.0 + b.0 + mid.0) as i32, (a.1 + b.1 + mid.1) as i32, v.len())
        };
        if !out.iter().any(|q| key(q) == key(&sp) || {
            let mut r = sp.clone();
            r.reverse();
            key(q) == key(&r)
        }) {
            out.push(sp);
        }
    }
    out
}

pub struct InkEval {
    pub ink_recall: f32,
    pub path_precision: f32,
    /// (component pixel count, matched) per QUALIFYING component.
    pub components: Vec<(usize, bool)>,
    pub component_coverage: bool,
}

/// F4 — the three-gate eval. Truth = the skeleton; tau in pixels.
pub fn ink_eval(
    skel_comps: &[Vec<(f32, f32)>],
    paths: &[Vec<(f32, f32)>],
    tau: f32,
    l_min: usize,
) -> InkEval {
    let near_path = |px: f32, py: f32| -> bool {
        paths.iter().any(|p| {
            p.windows(2).any(|s| seg_dist((px, py), s[0], s[1]) <= tau)
        })
    };
    let (mut hit, mut tot) = (0usize, 0usize);
    let mut components = Vec::new();
    for comp in skel_comps {
        let ch = comp.iter().filter(|&&(x, y)| near_path(x, y)).count();
        if comp.len() >= l_min {
            components.push((comp.len(), ch as f32 >= 0.9 * comp.len() as f32));
        }
        hit += ch;
        tot += comp.len();
    }
    let ink_recall = if tot > 0 { hit as f32 / tot as f32 } else { 0.0 };
    // precision: sample paths every ~2px; near any skeleton pixel?
    let all_skel: Vec<(f32, f32)> = skel_comps.iter().flatten().copied().collect();
    let (mut phit, mut ptot) = (0usize, 0usize);
    for p in paths {
        for s in p.windows(2) {
            let len = (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1);
            let steps = (len / 2.0).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let t = k as f32 / steps as f32;
                let (x, y) = (s[0].0 + t * (s[1].0 - s[0].0), s[0].1 + t * (s[1].1 - s[0].1));
                ptot += 1;
                if all_skel.iter().any(|&(sx, sy)| (sx - x).hypot(sy - y) <= tau) {
                    phit += 1;
                }
            }
        }
    }
    let path_precision = if ptot > 0 { phit as f32 / ptot as f32 } else { 0.0 };
    let component_coverage = components.iter().all(|&(_, m)| m) && !components.is_empty();
    InkEval { ink_recall, path_precision, components, component_coverage }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic stick horse: body bar + 4 legs + tail, drawn 3px thick.
    fn stick_horse(w: usize, h: usize) -> (Vec<u8>, Vec<Vec<(f32, f32)>>) {
        let mut g = vec![245u8; w * h];
        let mut strokes: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut line = |g: &mut Vec<u8>, x0: i32, y0: i32, x1: i32, y1: i32| {
            let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
            for k in 0..=n {
                let x = x0 + (x1 - x0) * k / n;
                let y = y0 + (y1 - y0) * k / n;
                for dy in -1..=1i32 {
                    for dx in -1..=1i32 {
                        let (px, py) = (x + dx, y + dy);
                        if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                            g[py as usize * w + px as usize] = 15;
                        }
                    }
                }
            }
        };
        let segs: Vec<((i32, i32), (i32, i32))> = vec![
            ((40, 60), (160, 60)),   // body
            ((50, 60), (50, 120)),   // legs
            ((80, 60), (80, 120)),
            ((120, 60), (120, 120)),
            ((150, 60), (150, 120)),
            ((40, 60), (16, 96)),    // tail
        ];
        for &((x0, y0), (x1, y1)) in &segs {
            line(&mut g, x0, y0, x1, y1);
            strokes.push(vec![(x0 as f32, y0 as f32), (x1 as f32, y1 as f32)]);
        }
        (g, strokes)
    }

    #[test]
    fn ink_class_detects_clip_art() {
        let (g, _) = stick_horse(200, 150);
        let (is_ink, _, thin) = is_ink_art(&g, 200, 150);
        assert!(is_ink, "stick figure is ink art (thin ratio {thin})");
    }

    #[test]
    fn full_trace_passes_all_three_gates() {
        let (g, strokes) = stick_horse(200, 150);
        let skel = ink_skeleton(&g, 200, 150);
        let comps = skeleton_components(&skel, 200, 150);
        let e = ink_eval(&comps, &strokes, 2.5, 5);
        assert!(e.ink_recall >= 0.97, "recall {}", e.ink_recall);
        assert!(e.path_precision >= 0.97, "precision {}", e.path_precision);
        assert!(e.component_coverage, "coverage");
    }

    /// F2: vectorized skeleton paths EARN all three gates on the stick
    /// horse — the trace IS the ink.
    #[test]
    fn vectorization_earns_the_gates() {
        let (g, _) = stick_horse(200, 150);
        let skel = ink_skeleton(&g, 200, 150);
        let paths = vectorize_skeleton(&skel, 200, 150, 1.2);
        let comps = skeleton_components(&skel, 200, 150);
        let e = ink_eval(&comps, &paths, 2.5, 5);
        assert!(e.ink_recall >= 0.97, "recall {}", e.ink_recall);
        assert!(e.path_precision >= 0.97, "precision {}", e.path_precision);
        assert!(e.component_coverage, "coverage");
        assert!(paths.len() >= 5, "junction-preserving: limbs are separate paths, got {}", paths.len());
    }

    /// Done-when #2: deleting the tail path flips component_coverage FAIL.
    #[test]
    fn missing_tail_fails_component_coverage() {
        let (g, mut strokes) = stick_horse(200, 150);
        strokes.pop(); // the tail
        let skel = ink_skeleton(&g, 200, 150);
        let comps = skeleton_components(&skel, 200, 150);
        let e = ink_eval(&comps, &strokes, 2.5, 5);
        assert!(e.ink_recall < 0.97 || !e.component_coverage, "tail deletion must FAIL");
    }

    /// Done-when #3: a stray path in empty canvas drops precision.
    #[test]
    fn stray_path_fails_precision() {
        let (g, mut strokes) = stick_horse(200, 150);
        strokes.push(vec![(180.0, 130.0), (198.0, 148.0)]); // empty corner
        let skel = ink_skeleton(&g, 200, 150);
        let comps = skeleton_components(&skel, 200, 150);
        let e = ink_eval(&comps, &strokes, 2.5, 5);
        assert!(e.path_precision < 0.97, "stray path must FAIL precision, got {}", e.path_precision);
    }
}
