//! v7.5 F4/F6 — grade paths against the reference's ink skeleton.
//! stdin: PGM. argv[1]: paths file (JSON [[[x,y],...],...]). Prints metrics.
//! No verdict words until the F6 calibration gate holds (D5): the caller
//! compares against gates and says FAIL — this bin only reports numbers.
use std::io::Read;

fn main() {
    let paths_file = std::env::args().nth(1).expect("paths json arg");
    let ptxt = std::fs::read_to_string(paths_file).expect("paths file");
    let paths = parse_paths(&ptxt);
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("stdin");
    assert!(buf.starts_with(b"P5"));
    let hdr_end = buf.iter().position(|&b| b == b'\n').unwrap() + 1;
    let header = std::str::from_utf8(&buf[..hdr_end]).unwrap();
    let mut it = header.split_whitespace();
    it.next();
    let w: usize = it.next().unwrap().parse().unwrap();
    let h: usize = it.next().unwrap().parse().unwrap();
    let gray = &buf[buf.len() - w * h..];
    let (is_ink, bim, thin) = tracer_core::ink::is_ink_art(gray, w, h);
    let diag = ((w * w + h * h) as f32).sqrt();
    let tau = 0.01 * diag; // D1
    let l_min = (0.02 * diag) as usize; // D2
    let skel = if is_ink {
        tracer_core::ink::ink_skeleton(gray, w, h)
    } else {
        tracer_core::ink::ink_boundary(gray, w, h)
    };
    let comps = tracer_core::ink::skeleton_components(&skel, w, h);
    let e = tracer_core::ink::ink_eval(&comps, &paths, tau, l_min);
    if let Some(residual_file) = std::env::args().nth(2) {
        let near = |px: f32, py: f32| -> bool {
            paths.iter().any(|p| p.windows(2).any(|s2| {
                let (a, b) = (s2[0], s2[1]);
                let (vx, vy) = (b.0 - a.0, b.1 - a.1);
                let l2 = vx * vx + vy * vy;
                let t = if l2 == 0.0 { 0.0 } else { (((px - a.0) * vx + (py - a.1) * vy) / l2).clamp(0.0, 1.0) };
                (px - (a.0 + t * vx)).hypot(py - (a.1 + t * vy)) <= tau
            }))
        };
        let mut res = String::from("[");
        let mut first = true;
        for c in &comps {
            for &(x, y) in c {
                if !near(x, y) {
                    if !first { res += ","; }
                    res += &format!("[{x:.0},{y:.0}]");
                    first = false;
                }
            }
        }
        res += "]";
        let _ = std::fs::write(residual_file, res);
    }
    let unmatched = e.components.iter().filter(|(_, m)| !m).count();
    println!(
        "{{\"class\":\"{}\",\"bimodality\":{bim:.3},\"thin_ratio\":{thin:.3},\"ink_recall\":{:.4},\"path_precision\":{:.4},\"components\":{},\"unmatched\":{unmatched},\"component_coverage\":{}}}",
        if is_ink { "ink" } else { "photo" },
        e.ink_recall, e.path_precision, e.components.len(), e.component_coverage
    );
}

fn parse_paths(s: &str) -> Vec<Vec<(f32, f32)>> {
    // minimal JSON [[..],..] parser for our own emitted shape
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut nums: Vec<f32> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '[' => depth += 1,
            ']' | ',' => {
                if !cur.is_empty() {
                    if let Ok(v) = cur.parse::<f32>() {
                        nums.push(v);
                    }
                    cur.clear();
                }
                if c == ']' {
                    if depth == 2 {
                        let pts: Vec<(f32, f32)> =
                            nums.chunks(2).map(|c2| (c2[0], c2[1])).collect();
                        if pts.len() >= 2 {
                            out.push(pts);
                        }
                        nums.clear();
                    }
                    depth -= 1;
                }
            }
            _ if c.is_ascii_digit() || c == '.' || c == '-' => cur.push(c),
            _ => {}
        }
    }
    out
}
