//! v8 render harness. stdin (flat text):
//!   PARAMS floor band_max advance_frac max_justify seed
//!   PATH sub_floor decorative | t t t | x,y x,y ...
//!   WORD word chars
//! stdout: JSON {placements|error}. Baselines exported verbatim (F5.1).
use std::io::Read;

fn main() {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).unwrap();
    let mut paths: Vec<scanlock::ScanPath> = Vec::new();
    let mut pool: Vec<(String, usize)> = Vec::new();
    let mut params = scanlock::CapacityParams {
        floor: 13.0,
        band_max: 40.0,
        avg_advance: 0.5257,
        min_word_chars: 3.0,
        gap_chars: 1.0,
        line_height_ratio: 1.0,
        step: 0.5,
    };
    let mut seed = 1u64;
    for line in buf.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("PARAMS") => {
                params.floor = it.next().unwrap().parse().unwrap();
                params.band_max = it.next().unwrap().parse().unwrap();
                params.avg_advance = it.next().unwrap().parse().unwrap();
                params.gap_chars = it.next().unwrap().parse().unwrap();
                seed = it.next().unwrap().parse().unwrap();
            }
            Some("PATH") => {
                let rest: Vec<&str> = line.splitn(2, ' ').nth(1).unwrap().split('|').collect();
                let flags: Vec<&str> = rest[0].split_whitespace().collect();
                let segments: Vec<f32> =
                    rest[1].split_whitespace().filter_map(|t| t.parse().ok()).collect();
                let points: Vec<(f32, f32)> = rest[2]
                    .split_whitespace()
                    .filter_map(|p| {
                        let mut c = p.split(',');
                        Some((c.next()?.parse().ok()?, c.next()?.parse().ok()?))
                    })
                    .collect();
                paths.push(scanlock::ScanPath {
                    points,
                    segments,
                    sub_floor: flags[0] == "1",
                    decorative_thin: flags[1] == "1",
                });
            }
            Some("WORD") => {
                let w = it.next().unwrap().to_string();
                let n: usize = it.next().unwrap().parse().unwrap();
                pool.push((w, n));
            }
            _ => {}
        }
    }
    let inject_sparse = std::env::args().any(|a| a == "inject-sparse");
    match scanlock::plan_capacity(&paths, &pool, seed, &params) {
        Ok((size, mut pls)) => {
            if inject_sparse && !pls.is_empty() {
                pls.pop(); // simulate a renderer bug dropping a word
            }
            // v8.1 F5 / I10 — the coverage gate lives IN the render path
            // and has no off switch: every non-decorative path must be
            // >= 95% arc-covered by glyph runs or nothing displays.
            let mut uncovered: Vec<(usize, f32)> = Vec::new();
            for (pi, path) in paths.iter().enumerate() {
                if path.sub_floor || path.decorative_thin {
                    continue;
                }
                let mut cum = 0.0f32;
                let mut arc = 0.0f32;
                for w in path.points.windows(2) {
                    arc += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
                }
                // hostable arc: segments >= min_word_chars capacity at size
                let mut bounds = vec![0.0f32];
                bounds.extend(path.segments.iter().copied().filter(|t| *t > 0.0 && *t < 1.0));
                bounds.push(1.0);
                let mut hostable = 0.0f32;
                for w in bounds.windows(2) {
                    let seg = (w[1] - w[0]) * arc;
                    if seg / (size * params.avg_advance) >= params.min_word_chars {
                        hostable += seg;
                    }
                }
                // A packed SEGMENT is fully covered: its words + justified
                // spacing span it by construction (F3.3). Coverage counts
                // hostable segments that received words.
                for w in bounds.windows(2) {
                    let seg = (w[1] - w[0]) * arc;
                    if seg / (size * params.avg_advance) < params.min_word_chars {
                        continue;
                    }
                    let mid_has_word = pls.iter().any(|pl| {
                        pl.path_idx == pi && pl.t0 >= w[0] - 0.0001 && pl.t0 < w[1]
                    });
                    if mid_has_word {
                        cum += seg;
                    }
                }
                if hostable > 0.0 && cum / hostable < 0.95 {
                    uncovered.push((pi, cum / hostable));
                }
            }
            if !uncovered.is_empty() {
                let mut out = String::from("{\"render_blocked\":[");
                for (k, (pi, c)) in uncovered.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    out += &format!("[{pi},{c:.3}]");
                }
                out += "]}";
                println!("{out}");
                return;
            }
            let _ = size;
            let pls = pls;
            let mut out = String::from("{\"placements\":[");
            for (k, pl) in pls.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                out += &format!(
                    "{{\"path\":{},\"t0\":{:.4},\"t1\":{:.4},\"word\":\"{}\",\"size\":{:.2},\"advance\":{:.2},\"baseline\":[",
                    pl.path_idx, pl.t0, pl.t1, pl.word, pl.glyph_size, pl.advance_px
                );
                for (j, (x, y)) in pl.baseline.iter().enumerate() {
                    if j > 0 {
                        out.push(',');
                    }
                    out += &format!("[{x:.2},{y:.2}]");
                }
                out += "]}";
            }
            out += "]}";
            println!("{out}");
        }
        Err(e) => {
            println!("{{\"error\":\"{e:?}\"}}");
        }
    }
}
