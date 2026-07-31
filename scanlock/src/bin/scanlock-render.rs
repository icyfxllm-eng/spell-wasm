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
        junction_radius: 10.0,
        keepout_arc_ratio: 0.75,
        curve_size_ratio: 0.9,
        min_words_per_path: 2.0,
        decorative_max_fraction: 0.20,
        micro_max_fraction: 0.10,
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
                    micro: flags.len() > 2 && flags[2] == "1",
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
        Ok((size, mut pls, micro, coverage)) => {
            if inject_sparse && !pls.is_empty() {
                pls.pop(); // simulate a renderer bug dropping a word
            }
            // v8.1 F5 / I10 — the coverage gate lives IN the render path
            // and has no off switch: every non-decorative path must be
            // >= 95% arc-covered by glyph runs or nothing displays.
            // v8.2 F5/I10 — the coverage gate consumes the PACKER's own
            // hostable/covered numbers (one source of truth). Micro paths
            // count as covered by F5.
            let mut uncovered: Vec<(usize, f32)> = Vec::new();
            for c in &coverage {
                if micro.iter().any(|m| m.path_idx == c.path_idx) {
                    continue;
                }
                if c.hostable > 0.0 && c.covered / c.hostable < 0.95 {
                    uncovered.push((c.path_idx, c.covered / c.hostable));
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
            let micro = micro;
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
            out += "],\"micro\":[";
            for (k, m) in micro.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                out += &format!("{{\"path\":{},\"points\":[", m.path_idx);
                for (j, (x, y)) in m.points.iter().enumerate() {
                    if j > 0 {
                        out.push(',');
                    }
                    out += &format!("[{x:.2},{y:.2}]");
                }
                out += "]}";
            }
            out += "],\"coverage\":[";
            for (k, c) in coverage.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                out += &format!("{{\"path\":{},\"hostable\":{:.2},\"covered\":{:.2}}}",
                    c.path_idx, c.hostable, c.covered);
            }
            out += &format!("],\"size\":{size:.2}}}");
            println!("{out}");
        }
        Err(e) => {
            println!("{{\"error\":\"{e:?}\"}}");
        }
    }
}
