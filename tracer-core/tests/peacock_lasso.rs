//! v7.2 `peacock-lasso` — the low-contrast regression. On this stored
//! reference the UNSEEDED auto-trace must FAIL the fidelity gates (the
//! background foliage merges with the translucent fan — this is the
//! regression that produced round 2's failed peacock). The tool's only
//! next step is routing to the lasso; the lasso-seeded trace (the stored
//! rough enclosure — sloppiness expected) must pass all gates with the
//! mask confined to the bird.

fn load() -> (Vec<u8>, usize, usize) {
    let s: &[u8] = include_bytes!("fixtures/peacock.pgm");
    let header = std::str::from_utf8(&s[..20]).unwrap();
    let mut it = header.split_whitespace();
    it.next();
    let w: usize = it.next().unwrap().parse().unwrap();
    let h: usize = it.next().unwrap().parse().unwrap();
    let start = s.len() - w * h;
    (s[start..].to_vec(), w, h)
}

/// Eric-style rough loop around the displaying bird (fan + body + legs).
fn lasso(w: usize, h: usize) -> Vec<(f32, f32)> {
    let (fw, fh) = (w as f32, h as f32);
    [
        (0.16, 0.16), (0.42, 0.06), (0.68, 0.10), (0.86, 0.24), (0.88, 0.52),
        (0.76, 0.74), (0.62, 0.94), (0.40, 0.96), (0.28, 0.80), (0.12, 0.62),
        (0.08, 0.38),
    ]
    .iter()
    .map(|&(x, y)| (x * fw, y * fh))
    .collect()
}

#[test]
fn peacock_lasso() {
    let (gray, w, h) = load();
    let unseeded = tracer_core::trace(&gray, w, h, tracer_core::Budget::STANDARD);
    let unseeded_pass = unseeded.deviation_frac <= 0.015 && unseeded.coverage_frac >= 0.95;
    assert!(!unseeded_pass, "unseeded auto-trace must FAIL on the low-contrast reference (routing event)");

    let poly = lasso(w, h);
    let seeded = tracer_core::trace_seeded(&gray, w, h, tracer_core::Budget::STANDARD, Some(&poly));
    assert!(seeded.deviation_frac <= 0.015, "seeded deviation {} > 1.5%", seeded.deviation_frac);
    assert!(seeded.coverage_frac >= 0.95, "seeded coverage {} < 95%", seeded.coverage_frac);
    assert_eq!(seeded.smoothness_violations, 0, "stair-step lint");
    // Mask confined to the bird: every silhouette point inside the lasso.
    let sil = seeded.paths.iter().find(|p| p.silhouette).expect("silhouette");
    let inside = sil
        .points
        .iter()
        .filter(|&&(x, y)| {
            let mut ins = false;
            let n = poly.len();
            let mut j = n - 1;
            for i in 0..n {
                let (xi, yi) = poly[i];
                let (xj, yj) = poly[j];
                if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                    ins = !ins;
                }
                j = i;
            }
            ins
        })
        .count();
    assert!(
        inside as f32 >= 0.97 * sil.points.len() as f32,
        "mask escaped the lasso: {inside}/{}",
        sil.points.len()
    );
}
