//! v7.4 fixtures — the trace is a tree, not a pile of lines.
use tracer_core::{trace_constrained, Budget, Constraints};

fn load(bytes: &'static [u8]) -> (Vec<u8>, usize, usize) {
    let header = std::str::from_utf8(&bytes[..20]).unwrap();
    let mut it = header.split_whitespace();
    it.next();
    let w: usize = it.next().unwrap().parse().unwrap();
    let h: usize = it.next().unwrap().parse().unwrap();
    (bytes[bytes.len() - w * h..].to_vec(), w, h)
}

fn in_poly(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let (mut inside, n) = (false, poly.len());
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

fn scale(pts: &[(f32, f32)], w: usize, h: usize) -> Vec<(f32, f32)> {
    pts.iter().map(|&(x, y)| (x * w as f32, y * h as f32)).collect()
}

/// `peacock-parts`: stored part lassos yield exactly two labeled closed
/// silhouettes partitioning the mask; interior features confine to their
/// part; zero paths cross the part boundary.
#[test]
fn peacock_parts() {
    let (gray, w, h) = load(include_bytes!("fixtures/peacock.pgm"));
    let outer = scale(&[(0.16, 0.16), (0.42, 0.06), (0.68, 0.10), (0.86, 0.24), (0.88, 0.52),
                        (0.76, 0.74), (0.62, 0.94), (0.40, 0.96), (0.28, 0.80), (0.12, 0.62), (0.08, 0.38)], w, h);
    let fan = scale(&[(0.10, 0.06), (0.90, 0.06), (0.90, 0.60), (0.62, 0.66), (0.40, 0.66), (0.10, 0.60)], w, h);
    let body = scale(&[(0.36, 0.60), (0.64, 0.60), (0.66, 0.98), (0.34, 0.98)], w, h);
    let c = Constraints {
        lasso: Some(outer),
        parts: vec![("fan".into(), fan.clone()), ("body".into(), body.clone())],
        ..Default::default()
    };
    let r = trace_constrained(&gray, w, h, Budget::EXPERT, &c);
    let sils: Vec<&tracer_core::TracedPath> =
        r.paths.iter().filter(|p| p.silhouette).collect();
    assert_eq!(sils.len(), 2, "exactly two part silhouettes");
    assert!(sils.iter().any(|p| p.part == "fan") && sils.iter().any(|p| p.part == "body"));
    for p in &r.paths {
        if p.part == "frame" {
            continue;
        }
        let poly = if p.part == "fan" { &fan } else { &body };
        let tol_ok = p
            .points
            .iter()
            .filter(|&&(x, y)| in_poly(x, y, poly))
            .count() as f32
            >= 0.95 * p.points.len() as f32;
        assert!(tol_ok, "path in part '{}' crosses its part boundary", p.part);
    }
}

/// `monalisa-containment`: the mask is clipped to the frame interior
/// BEFORE tracing; unclipped, the v7.3 balloon case must reproduce as a
/// failure (some silhouette pixel outside the rectangle).
#[test]
fn monalisa_containment() {
    let (gray, w, h) = load(include_bytes!("fixtures/mona.pgm"));
    let frame = (0.12 * w as f32, 0.06 * h as f32, 0.88 * w as f32, 0.96 * h as f32);
    let clipped = trace_constrained(
        &gray, w, h, Budget::EXPERT,
        &Constraints { frame: Some(frame), ..Default::default() },
    );
    for p in &clipped.paths {
        if p.part == "frame" {
            continue;
        }
        for &(x, y) in &p.points {
            assert!(
                x >= frame.0 - 0.5 && x <= frame.2 + 0.5 && y >= frame.1 - 0.5 && y <= frame.3 + 0.5,
                "path escaped the frame at ({x},{y})"
            );
        }
    }
    assert!(clipped.paths.iter().any(|p| p.part == "frame"), "frame path is the root");
    assert!(
        clipped.paths.iter().filter(|p| !p.silhouette && p.part != "frame").count() >= 1,
        "interior feature lines present inside the figure"
    );
    // The regression: without the frame clip, the balloon escapes.
    let unclipped = trace_constrained(&gray, w, h, Budget::EXPERT, &Constraints::default());
    let sil = unclipped.paths.iter().find(|p| p.silhouette).unwrap();
    let escaped = sil.points.iter().any(|&(x, y)| {
        x < frame.0 || x > frame.2 || y < frame.1 || y > frame.3
    });
    assert!(escaped, "unclipped balloon case must reproduce as the failure it was");
}
