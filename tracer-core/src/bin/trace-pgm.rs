//! Offline authoring harness: PGM(P5) on stdin -> JSON paths on stdout.
//! std-only shell around the pure core (the core itself stays fs-free).
use std::io::Read;

fn main() {
    let budget = match std::env::args().nth(1).as_deref() {
        Some("expert") => tracer_core::Budget::EXPERT,
        _ => tracer_core::Budget::STANDARD,
    };
    // args: budget [lasso=x,y;x,y...] [frame=x0,y0,x1,y1] [part=name:x,y;x,y...]*
    let mut cons = tracer_core::Constraints::default();
    let parse_poly = |arg: &str| -> Vec<(f32, f32)> {
        arg.split(';')
            .filter_map(|p| {
                let mut it = p.split(',');
                Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
            })
            .collect()
    };
    for arg in std::env::args().skip(2) {
        if let Some(v) = arg.strip_prefix("frame=") {
            let n: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect();
            if n.len() == 4 {
                cons.frame = Some((n[0], n[1], n[2], n[3]));
            }
        } else if let Some(v) = arg.strip_prefix("part=") {
            if let Some((name, poly)) = v.split_once(':') {
                cons.parts.push((name.to_string(), parse_poly(poly)));
            }
        } else if let Some(v) = arg.strip_prefix("lasso=") {
            cons.lasso = Some(parse_poly(v));
        } else if let Some(v) = arg.strip_prefix("keep=") {
            cons.keep = parse_poly(v);
        } else if let Some(v) = arg.strip_prefix("exclude=") {
            cons.exclude = parse_poly(v);
        }
    }
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("stdin");
    let s = &buf;
    assert!(s.starts_with(b"P5"), "PGM P5 required");
    // header: P5 <w> <h> <max>\n then raw bytes
    let mut nums = Vec::new();
    let mut i = 2;
    while nums.len() < 3 {
        while i < s.len() && (s[i] as char).is_whitespace() { i += 1; }
        if s[i] == b'#' { while s[i] != b'\n' { i += 1; } continue; }
        let mut v = 0usize;
        while i < s.len() && (s[i] as char).is_ascii_digit() {
            v = v * 10 + (s[i] - b'0') as usize;
            i += 1;
        }
        nums.push(v);
    }
    i += 1;
    let (w, h) = (nums[0], nums[1]);
    let gray = &s[i..i + w * h];
    let mode = std::env::args().nth(1).unwrap_or_default();
    if mode == "ink" || mode == "ink-boundary" {
        // v7.5 F2: the trace IS the ink — skeleton centerline paths.
        let diag = ((w * w + h * h) as f32).sqrt();
        let (thin_stroke, _, _) = tracer_core::ink::is_ink_art(gray, w, h);
        let skel = if thin_stroke && mode != "ink-boundary" {
            tracer_core::ink::ink_skeleton(gray, w, h)
        } else {
            tracer_core::ink::ink_boundary(gray, w, h)
        };
        let paths = tracer_core::ink::vectorize_skeleton(&skel, w, h, 0.006 * diag);
        let mut out = String::from("{\"paths\":[");
        for (k, p) in paths.iter().enumerate() {
            if k > 0 { out += ","; }
            out += "[";
            for (j, (x, y)) in p.iter().enumerate() {
                if j > 0 { out += ","; }
                out += &format!("[{x:.1},{y:.1}]");
            }
            out += "]";
        }
        out += "]}";
        println!("{out}");
        return;
    }
    let r = tracer_core::trace_constrained(gray, w, h, budget, &cons);
    let mut out = String::from("{");
    out += &format!("\"deviation\":{:.5},\"coverage\":{:.4},\"threshold\":{},\"smooth_viol\":{},\"w\":{w},\"h\":{h},\"paths\":[",
        r.deviation_frac, r.coverage_frac, r.threshold, r.smoothness_violations);
    for (k, p) in r.paths.iter().enumerate() {
        if k > 0 { out += ","; }
        out += &format!("{{\"band\":{},\"scale\":\"{}\",\"silhouette\":{},\"part\":\"{}\",\"points\":[",
            p.band, p.scale_class, p.silhouette, p.part);
        for (j, (x, y)) in p.points.iter().enumerate() {
            if j > 0 { out += ","; }
            out += &format!("[{x:.1},{y:.1}]");
        }
        out += "]}";
    }
    out += "]}";
    println!("{out}");
}
