//! Offline authoring harness: PGM(P5) on stdin -> JSON paths on stdout.
//! std-only shell around the pure core (the core itself stays fs-free).
use std::io::Read;

fn main() {
    let budget = match std::env::args().nth(1).as_deref() {
        Some("expert") => tracer_core::Budget::EXPERT,
        _ => tracer_core::Budget::STANDARD,
    };
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
    let r = tracer_core::trace(gray, w, h, budget);
    let mut out = String::from("{");
    out += &format!("\"deviation\":{:.5},\"coverage\":{:.4},\"threshold\":{},\"w\":{w},\"h\":{h},\"paths\":[",
        r.deviation_frac, r.coverage_frac, r.threshold);
    for (k, p) in r.paths.iter().enumerate() {
        if k > 0 { out += ","; }
        out += &format!("{{\"band\":{},\"scale\":\"{}\",\"silhouette\":{},\"points\":[",
            p.band, p.scale_class, p.silhouette);
        for (j, (x, y)) in p.points.iter().enumerate() {
            if j > 0 { out += ","; }
            out += &format!("[{x:.1},{y:.1}]");
        }
        out += "]}";
    }
    out += "]}";
    println!("{out}");
}
