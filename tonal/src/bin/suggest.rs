//! CC-SCAN-STACK v1.2 module 12 — ML path suggestion, classical attempt #1.
//!
//! Tool-side, human-in-loop. This proposes; it never ships. A candidate
//! becomes a traced path only after Eric approves it in the review sidecar,
//! and `build_scan_library.py` refuses to export a subject while any of its
//! candidates is still `pending` — the export-block is the UX contract.
//!
//! Pipeline (D11: zero learned components until this demonstrably
//! under-suggests, and escalation is its own recorded decision):
//!   binarize → boundary contours (Moore trace, holes included) →
//!   Douglas-Peucker simplify → rank by prominence (arc × edge strength) →
//!   corridor-clearance pre-score.
//!
//! The clearance score is the v1.2 field lesson made mechanical: the layout
//! law refuses candidates a human eye reads as fine (the rhino's tail, the
//! armour creases — both blocked whole pictures). A suggester that ranks by
//! prominence alone keeps proposing paths the solver then refuses, so every
//! candidate carries its clearance and a `flagged` bit the review sheet
//! shows in red BEFORE Eric spends attention on it.
//!
//! Determinism: fixed seed, fixed iteration order, no libm (tonal's
//! discipline), no hash-map iteration anywhere. Same reference + seed →
//! byte-identical candidates.json, both runs, both arches.
//!
//! Usage: suggest <ref.png> <out-dir> [--seed N]

use std::fmt::Write as _;
use std::path::Path;

use tonal::Gray;

const CANVAS: f32 = 512.0;
const MARGIN: f32 = 22.0;
/// Below this many px of corridor (512-canvas units) the solver has
/// historically refused the picture; the sheet shows these red.
const CLEARANCE_FLOOR: f32 = 14.0;
const MIN_ARC_PX: f32 = 40.0;
/// Matches build_scan_library's SMALL_FEATURE_MAX: a short CLOSED contour
/// becomes a micro feature downstream — drawn filled, never hosting a word,
/// EXEMPT from corridor keep-outs (the snowman's eyes, the cactus's
/// thorns). Flagging those for tight clearance would be a false alarm
/// about a law that does not apply to them.
const MICRO_ARC_MAX: f32 = 130.0;
const DP_EPSILON: f32 = 1.6;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: suggest <ref.png> <out-dir> [--seed N]");
        std::process::exit(2);
    }
    let seed: u64 = args
        .iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let gray = load_png_gray(Path::new(&args[1]));
    let name = Path::new(&args[1]).file_stem().unwrap().to_string_lossy().into_owned();

    // Fixed threshold, not equal-population: the references are near-binary
    // line art, and an adaptive cut would make the candidate set depend on
    // how much empty page surrounds the subject.
    let mask: Vec<bool> = gray.px.iter().map(|&v| v < 128).collect();

    let contours = trace_all_boundaries(&mask, gray.width, gray.height);
    let (sx, sy, scale) = fit_512(&contours);

    // Edge strength from the same flow field the render stack uses.
    let flow = tonal::flow_field(&gray, 3);

    let mut cands: Vec<Candidate> = contours
        .into_iter()
        .map(|c| {
            let pts: Vec<(f32, f32)> = c
                .iter()
                .map(|&(x, y)| ((x as f32 - sx) * scale + MARGIN, (y as f32 - sy) * scale + MARGIN))
                .collect();
            let simplified = douglas_peucker(&pts, DP_EPSILON);
            let arc = poly_len(&simplified);
            let strength = c
                .iter()
                .map(|&(x, y)| flow.coherence[y * gray.width + x])
                .sum::<f32>()
                / c.len().max(1) as f32;
            Candidate {
                points: simplified,
                arc,
                prominence: arc * (0.25 + strength),
                clearance: f32::MAX,
                flagged: false,
                micro: false,
            }
        })
        .filter(|c| c.arc >= MIN_ARC_PX)
        .collect();

    // Prominence order, stably: ties break on point data, never on memory
    // address or arrival order.
    cands.sort_by(|a, b| {
        b.prominence
            .partial_cmp(&a.prominence)
            .unwrap()
            .then_with(|| a.points.first().partial_cmp(&b.points.first()).unwrap())
    });

    // Pairwise corridor clearance — the pre-score that keeps the solver's
    // refusals off Eric's review sheet.
    for i in 0..cands.len() {
        cands[i].micro = cands[i].arc < MICRO_ARC_MAX && is_closed(&cands[i].points);
    }
    for i in 0..cands.len() {
        let mut min_d = f32::MAX;
        let mut min_word = f32::MAX;
        for j in 0..cands.len() {
            if i == j {
                continue;
            }
            let d = poly_distance(&cands[i].points, &cands[j].points);
            min_d = min_d.min(d);
            if !cands[j].micro {
                min_word = min_word.min(d);
            }
        }
        cands[i].clearance = min_d;
        // Corridors exist between WORD-HOSTING strokes. Micro features are
        // exempt downstream on both sides of the pair: a thorn near the
        // trunk is the design, not a refusal -- so neither the thorn nor
        // the trunk flags for it. False alarms are how flags die.
        cands[i].flagged = min_word < CLEARANCE_FLOOR && !cands[i].micro;
    }

    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();

    // candidates.json — every candidate born `pending`. Approval happens in
    // this file, by a human, and the scan build reads it back.
    let mut json = String::from("{\n");
    let _ = writeln!(json, " \"subject\": \"{name}\",\n \"seed\": {seed},\n \"clearance_floor\": {CLEARANCE_FLOOR},");
    let _ = writeln!(json, " \"candidates\": [");
    for (k, c) in cands.iter().enumerate() {
        let pts: Vec<String> = c.points.iter().map(|(x, y)| format!("[{x:.1},{y:.1}]")).collect();
        let _ = writeln!(
            json,
            "  {{\"id\": {k}, \"status\": \"pending\", \"arc\": {:.1}, \"clearance\": {:.1}, \"flagged\": {}, \"micro\": {}, \"points\": [{}]}}{}",
            c.arc,
            if c.clearance == f32::MAX { 9999.0 } else { c.clearance },
            c.flagged,
            c.micro,
            pts.join(","),
            if k + 1 < cands.len() { "," } else { "" }
        );
    }
    json.push_str(" ]\n}\n");
    std::fs::write(out.join(format!("{name}.json")), &json).unwrap();

    // Review sheet: green = clear, red = the solver will refuse this
    // corridor; numbers so approve/reject can name candidates.
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\" width=\"768\">\
         <rect width=\"512\" height=\"512\" fill=\"#101623\"/>",
    );
    for (k, c) in cands.iter().enumerate() {
        let d: String = c
            .points
            .iter()
            .enumerate()
            .map(|(i, (x, y))| format!("{}{x:.1} {y:.1} ", if i == 0 { "M" } else { "L" }))
            .collect();
        let color = if c.flagged { "#ff5d5d" } else if c.micro { "#ffb14d" } else { "#7ee08c" };
        let _ = write!(
            svg,
            "<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.6\"/>\
             <text x=\"{:.0}\" y=\"{:.0}\" fill=\"{color}\" font-size=\"11\">{k}</text>",
            c.points[0].0 + 3.0,
            c.points[0].1 - 3.0
        );
    }
    svg.push_str("</svg>");
    std::fs::write(out.join(format!("{name}-review.svg")), svg).unwrap();

    println!(
        "{name}: {} candidates ({} flagged below the {CLEARANCE_FLOOR}px corridor floor)",
        cands.len(),
        cands.iter().filter(|c| c.flagged).count()
    );
}

struct Candidate {
    points: Vec<(f32, f32)>,
    arc: f32,
    prominence: f32,
    clearance: f32,
    flagged: bool,
    micro: bool,
}

fn is_closed(p: &[(f32, f32)]) -> bool {
    p.len() > 2 && dist(p[0], *p.last().unwrap()) < 3.0
}

fn load_png_gray(path: &Path) -> Gray {
    let dec = png::Decoder::new(std::fs::File::open(path).expect("open png"));
    let mut reader = dec.read_info().expect("png info");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    let (w, h) = (info.width as usize, info.height as usize);
    let step = buf.len() / (w * h);
    // Composite alpha over white the way every tracer in this repo does —
    // a transparent page IS the page.
    let px: Vec<u8> = (0..w * h)
        .map(|i| {
            let o = i * step;
            let (r, g, b, a) = match step {
                4 => (buf[o], buf[o + 1], buf[o + 2], buf[o + 3]),
                3 => (buf[o], buf[o + 1], buf[o + 2], 255),
                2 => (buf[o], buf[o], buf[o], buf[o + 1]),
                _ => (buf[o], buf[o], buf[o], 255),
            };
            let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
            ((lum * a as u32 + 255 * (255 - a as u32)) / 255) as u8
        })
        .collect();
    Gray::new(w, h, px)
}

/// Every dark/light boundary as a closed pixel chain: outer contours AND
/// holes, via Moore-neighbour tracing with a visited set per starting edge.
fn trace_all_boundaries(mask: &[bool], w: usize, h: usize) -> Vec<Vec<(usize, usize)>> {
    let at = |x: isize, y: isize| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && mask[y as usize * w + x as usize]
    };
    let mut visited = vec![false; w * h];
    let mut out = Vec::new();
    // Scan order fixes discovery order; discovery order fixes output order.
    for sy in 0..h {
        for sx in 0..w {
            if !mask[sy * w + sx] || visited[sy * w + sx] {
                continue;
            }
            let (ix, iy) = (sx as isize, sy as isize);
            // boundary pixel: some 4-neighbour is light
            if at(ix - 1, iy) && at(ix + 1, iy) && at(ix, iy - 1) && at(ix, iy + 1) {
                continue;
            }
            const DIRS: [(isize, isize); 8] =
                [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
            let mut chain = Vec::new();
            let (mut cx, mut cy) = (ix, iy);
            let mut dir = 6usize; // entered heading up
            loop {
                chain.push((cx as usize, cy as usize));
                visited[cy as usize * w + cx as usize] = true;
                let mut found = false;
                for k in 0..8 {
                    let d = (dir + 6 + k) % 8; // start behind-left: textbook Moore
                    let (nx, ny) = (cx + DIRS[d].0, cy + DIRS[d].1);
                    if at(nx, ny) {
                        cx = nx;
                        cy = ny;
                        dir = d;
                        found = true;
                        break;
                    }
                }
                if !found || ((cx, cy) == (ix, iy) && chain.len() > 2) {
                    break;
                }
                if chain.len() > w * h {
                    break; // pathological; never silent-loop
                }
            }
            if chain.len() >= 8 {
                out.push(chain);
            }
        }
    }
    out
}

fn fit_512(contours: &[Vec<(usize, usize)>]) -> (f32, f32, f32) {
    let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (f32::MAX, f32::MAX, 0.0f32, 0.0f32);
    for c in contours {
        for &(x, y) in c {
            lo_x = lo_x.min(x as f32);
            lo_y = lo_y.min(y as f32);
            hi_x = hi_x.max(x as f32);
            hi_y = hi_y.max(y as f32);
        }
    }
    let span = (hi_x - lo_x).max(hi_y - lo_y).max(1.0);
    (lo_x, lo_y, (CANVAS - 2.0 * MARGIN) / span)
}

fn poly_len(p: &[(f32, f32)]) -> f32 {
    p.windows(2).map(|w| dist(w[0], w[1])).sum()
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).sqrt() // sqrt is IEEE-correctly-rounded: allowed
}

fn douglas_peucker(pts: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (mut worst, mut wi) = (0.0f32, 0usize);
    for (i, &p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
        let d = seg_dist(p, a, b);
        if d > worst {
            worst = d;
            wi = i;
        }
    }
    if worst <= eps {
        return vec![a, b];
    }
    let mut left = douglas_peucker(&pts[..=wi], eps);
    let right = douglas_peucker(&pts[wi..], eps);
    left.pop();
    left.extend(right);
    left
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0);
    dist(p, (a.0 + t * vx, a.1 + t * vy))
}

/// Min distance between two polylines, sampled — the corridor the solver
/// will see.
fn poly_distance(a: &[(f32, f32)], b: &[(f32, f32)]) -> f32 {
    let mut min = f32::MAX;
    for &p in a.iter().step_by(2) {
        for w in b.windows(2) {
            min = min.min(seg_dist(p, w[0], w[1]));
        }
    }
    min
}
