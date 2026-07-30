//! v7.3 `correction-loop` — human authority is absolute (D13). A re-solve
//! that violates a scribble, moves a spliced segment, or misses an anchor
//! tolerance is a core bug. Pinned means pinned: identical inputs must
//! reproduce the identical trace hash.
use tracer_core::{trace_constrained, trace_hash, Budget, Constraints};

fn load(bytes: &'static [u8]) -> (Vec<u8>, usize, usize) {
    let header = std::str::from_utf8(&bytes[..20]).unwrap();
    let mut it = header.split_whitespace();
    it.next();
    let w: usize = it.next().unwrap().parse().unwrap();
    let h: usize = it.next().unwrap().parse().unwrap();
    (bytes[bytes.len() - w * h..].to_vec(), w, h)
}

fn lasso(w: usize, h: usize) -> Vec<(f32, f32)> {
    [(0.16, 0.16), (0.42, 0.06), (0.68, 0.10), (0.86, 0.24), (0.88, 0.52),
     (0.76, 0.74), (0.62, 0.94), (0.40, 0.96), (0.28, 0.80), (0.12, 0.62), (0.08, 0.38)]
        .iter().map(|&(x, y)| (x * w as f32, y * h as f32)).collect()
}

#[test]
fn green_scribble_is_law() {
    let (gray, w, h) = load(include_bytes!("fixtures/peacock.pgm"));
    // A stroke across the under-covered fan (one gesture, per the spec).
    let keep: Vec<(f32, f32)> = (0..12)
        .map(|k| (w as f32 * (0.30 + 0.035 * k as f32), h as f32 * 0.30))
        .collect();
    let c = Constraints { lasso: Some(lasso(w, h)), keep: keep.clone(), ..Default::default() };
    let r = trace_constrained(&gray, w, h, Budget::STANDARD, &c);
    for &(x, y) in &keep {
        assert!(r.mask[y as usize * w + x as usize], "green pixel ({x},{y}) escaped the mask");
    }
}

#[test]
fn red_scribble_is_law() {
    let (gray, w, h) = load(include_bytes!("fixtures/mona.pgm"));
    let base = trace_constrained(&gray, w, h, Budget::STANDARD, &Constraints::default());
    let sil0 = base.paths.iter().find(|p| p.silhouette).unwrap();
    // Red-scribble the crown-shadow spike region: just above the topmost
    // silhouette run.
    let top = sil0.points.iter().cloned().fold((0f32, f32::MAX), |a, p| {
        if p.1 < a.1 { (p.0, p.1) } else { a }
    });
    let exclude: Vec<(f32, f32)> = (-3..=3)
        .map(|k| (top.0 + k as f32 * 4.0, (top.1 - 2.0).max(1.0)))
        .collect();
    let c = Constraints { exclude: exclude.clone(), ..Default::default() };
    let r = trace_constrained(&gray, w, h, Budget::STANDARD, &c);
    let sil = r.paths.iter().find(|p| p.silhouette).unwrap();
    for &(ex, ey) in &exclude {
        for &(sx, sy) in &sil.points {
            assert!((sx - ex).hypot(sy - ey) > 1.5, "silhouette still touches red scribble at ({ex},{ey})");
        }
    }
}

#[test]
fn splice_locks_and_anchor_holds_and_pin_reproduces() {
    let (gray, w, h) = load(include_bytes!("fixtures/peacock.pgm"));
    let seg: Vec<(f32, f32)> = vec![(60.0, 60.0), (80.0, 58.0), (100.0, 60.0)];
    let base = trace_constrained(
        &gray, w, h, Budget::STANDARD,
        &Constraints { lasso: Some(lasso(w, h)), ..Default::default() },
    );
    let sil0 = base.paths.iter().find(|p| p.silhouette).unwrap();
    let anchor = sil0.points[sil0.points.len() * 7 / 10]; // outside the spliced span
    let c = Constraints {
        lasso: Some(lasso(w, h)),
        splices: vec![(0.25, seg.clone())],
        anchors: vec![anchor],
        ..Default::default()
    };
    let r1 = trace_constrained(&gray, w, h, Budget::STANDARD, &c);
    let r2 = trace_constrained(&gray, w, h, Budget::STANDARD, &c);
    let s1 = r1.paths.iter().find(|p| p.silhouette).unwrap();
    let s2 = r2.paths.iter().find(|p| p.silhouette).unwrap();
    assert_eq!(s1.points, s2.points, "re-solve moved a locked splice");
    let spliced = s1.points.windows(seg.len()).any(|win| win == seg.as_slice());
    assert!(spliced, "spliced segment not present verbatim");
    let tol = 0.005 * w as f32;
    let ok = s1.points.iter().any(|&(x, y)| (x - anchor.0).hypot(y - anchor.1) <= tol);
    assert!(ok, "anchor missed its 0.5% tolerance");
    assert_eq!(trace_hash(&r1), trace_hash(&r2), "pin hash not reproducible");
}
