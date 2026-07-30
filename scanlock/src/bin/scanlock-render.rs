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
    let mut params = scanlock::Params {
        floor: 13.0,
        band_max: 40.0,
        advance_frac: 0.54,
        max_justify: 2.0,
    };
    let mut seed = 1u64;
    for line in buf.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("PARAMS") => {
                params.floor = it.next().unwrap().parse().unwrap();
                params.band_max = it.next().unwrap().parse().unwrap();
                params.advance_frac = it.next().unwrap().parse().unwrap();
                params.max_justify = it.next().unwrap().parse().unwrap();
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
    match scanlock::typeset(&paths, &pool, seed, &params) {
        Ok(pls) => {
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
