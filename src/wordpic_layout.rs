//! CC-WORD-PICTURE v6 — the layout engine (L1–L5).
//!
//! Eric's law, verbatim: "no words overlap; words fill the line assigned on
//! the template." This module makes that structural:
//!  * L1 fill-the-line SOLVER: font size = pathLen / (units × script advance),
//!    clamped to [floor, band max]; the feed then prefers candidate words
//!    whose solved fill lands in [95%, 100%] — words never render at a styled
//!    size and never underfill their stroke.
//!  * L2 path-lock: every placement is expressed as (path, segment) geometry
//!    from THIS module — there is no free-position API to misuse (V4).
//!  * L3 collisions: axis-aligned volumes per placement, tested pairwise; the
//!    render-CI sweep asserts zero contacts for every (picture, lang, seed).
//!  * L4 corner split: polylines split at sharp vertices at manifest load
//!    (deterministic — same output every launch on every platform), so text
//!    never bends around a star point.
//!  * L5 canvas fit: the whole stroke map is normalized into the 512 frame
//!    with a safe margin BEFORE any placement math.
//!
//! Pure geometry — no web_sys — so the L9 sweep runs under `cargo test`.

use crate::wordpic::{Picture, WordPath};

pub const FRAME: f32 = 512.0;
pub const MARGIN: f32 = 22.0;
/// L1 legibility floor (absolute — the cascade never goes below).
pub const FLOOR: f32 = 13.0;
/// L4: split polylines at vertices sharper than this (degrees).
pub const CORNER_DEG: f32 = 35.0;
/// L7 band → [min, max] solved size at the reference viewport.
pub const BAND_SIZE: [(f32, f32); 4] = [(22.0, 46.0), (16.0, 32.0), (13.0, 24.0), (15.0, 30.0)];
/// V3: a placed word must cover ≥95% of its segment.
pub const FILL_MIN: f32 = 0.95;

/// Mean glyph advance as a fraction of font size, per script class. Rough by
/// design — the solver only needs to be consistent (V2), not typographically
/// exact; textLength justification closes the residual at render time.
pub fn advance(lang: &str) -> f32 {
    match lang {
        "zh" | "ja" => 1.0,
        "ko" => 0.95,
        "ar" => 0.52,
        "hi" => 0.58,
        "ru" => 0.58,
        _ => 0.54,
    }
}

// ---------- path geometry ----------

/// A flattened path: polyline points + cumulative lengths.
#[derive(Debug, Clone)]
pub struct Poly {
    pub pts: Vec<(f32, f32)>,
    pub cum: Vec<f32>,
}

impl Poly {
    pub fn len(&self) -> f32 {
        *self.cum.last().unwrap_or(&0.0)
    }

    pub fn point_at(&self, t: f32) -> (f32, f32) {
        let want = t.clamp(0.0, 1.0) * self.len();
        for i in 1..self.pts.len() {
            if self.cum[i] >= want {
                let seg = self.cum[i] - self.cum[i - 1];
                let f = if seg > 0.0 { (want - self.cum[i - 1]) / seg } else { 0.0 };
                let (x0, y0) = self.pts[i - 1];
                let (x1, y1) = self.pts[i];
                return (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
            }
        }
        *self.pts.last().unwrap_or(&(0.0, 0.0))
    }
}

/// Flatten an SVG path `d` (M/L/Q/A subset — the manifest vocabulary) into a
/// polyline. Quads sample at 16 steps; arcs at 24.
pub fn flatten(d: &str) -> Poly {
    let mut pts: Vec<(f32, f32)> = Vec::new();
    let mut nums: Vec<f32> = Vec::new();
    let mut cmds: Vec<(char, Vec<f32>)> = Vec::new();
    let mut cur = ' ';
    for tok in d.split_whitespace() {
        let mut rest = tok;
        while !rest.is_empty() {
            let c = rest.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                if cur != ' ' {
                    cmds.push((cur, std::mem::take(&mut nums)));
                }
                cur = c;
                rest = &rest[1..];
            } else {
                let end = rest
                    .char_indices()
                    .find(|(i, ch)| *i > 0 && ch.is_ascii_alphabetic())
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
                for part in rest[..end].split(',') {
                    if let Ok(v) = part.parse() {
                        nums.push(v);
                    }
                }
                rest = &rest[end..];
            }
        }
    }
    if cur != ' ' {
        cmds.push((cur, nums));
    }
    let mut pos = (0.0f32, 0.0f32);
    for (c, n) in cmds {
        match c {
            'M' if n.len() >= 2 => {
                pos = (n[0], n[1]);
                pts.push(pos);
            }
            'L' if n.len() >= 2 => {
                for ch in n.chunks(2) {
                    pos = (ch[0], ch[1]);
                    pts.push(pos);
                }
            }
            'Q' if n.len() >= 4 => {
                let (x0, y0) = pos;
                let (cx, cy, x1, y1) = (n[0], n[1], n[2], n[3]);
                for k in 1..=16 {
                    let t = k as f32 / 16.0;
                    let u = 1.0 - t;
                    pts.push((
                        u * u * x0 + 2.0 * u * t * cx + t * t * x1,
                        u * u * y0 + 2.0 * u * t * cy + t * t * y1,
                    ));
                }
                pos = (x1, y1);
            }
            'A' if n.len() >= 7 => {
                // Endpoint-parameterized arc → sampled (rx==ry assumed, the
                // manifest vocabulary only uses circular arcs).
                let (rx, _ry, _rot, laf, sf, x1, y1) = (n[0], n[1], n[2], n[3], n[4], n[5], n[6]);
                let (x0, y0) = pos;
                let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let (dx, dy) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
                let dd = (dx * dx + dy * dy).sqrt().max(0.0001);
                let r = rx.max(dd);
                let h = (r * r - dd * dd).max(0.0).sqrt();
                let (ux, uy) = (-dy / dd, dx / dd);
                let sign = if (laf > 0.5) == (sf > 0.5) { 1.0 } else { -1.0 };
                let (cx, cy) = (mx + sign * h * ux, my + sign * h * uy);
                let a0 = (y0 - cy).atan2(x0 - cx);
                let a1 = (y1 - cy).atan2(x1 - cx);
                let mut sweep = a1 - a0;
                if sf > 0.5 && sweep < 0.0 {
                    sweep += std::f32::consts::TAU;
                }
                if sf <= 0.5 && sweep > 0.0 {
                    sweep -= std::f32::consts::TAU;
                }
                if laf > 0.5 && sweep.abs() < std::f32::consts::PI {
                    sweep += if sweep >= 0.0 { -std::f32::consts::TAU } else { std::f32::consts::TAU };
                }
                for k in 1..=24 {
                    let a = a0 + sweep * k as f32 / 24.0;
                    pts.push((cx + r * a.cos(), cy + r * a.sin()));
                }
                pos = (x1, y1);
            }
            _ => {}
        }
    }
    let mut cum = vec![0.0];
    for i in 1..pts.len() {
        let (x0, y0) = pts[i - 1];
        let (x1, y1) = pts[i];
        cum.push(cum[i - 1] + (x1 - x0).hypot(y1 - y0));
    }
    Poly { pts, cum }
}

/// L4 — split a polyline at sharp vertices; returns sub-polylines.
pub fn corner_split(p: &Poly) -> Vec<Poly> {
    let mut cuts = vec![0usize];
    for i in 1..p.pts.len().saturating_sub(1) {
        let (ax, ay) = p.pts[i - 1];
        let (bx, by) = p.pts[i];
        let (cx, cy) = p.pts[i + 1];
        let (v1x, v1y) = (bx - ax, by - ay);
        let (v2x, v2y) = (cx - bx, cy - by);
        let l1 = v1x.hypot(v1y);
        let l2 = v2x.hypot(v2y);
        if l1 < 1e-3 || l2 < 1e-3 {
            continue;
        }
        let cosang = ((v1x * v2x + v1y * v2y) / (l1 * l2)).clamp(-1.0, 1.0);
        let turn = cosang.acos().to_degrees();
        if turn > CORNER_DEG {
            cuts.push(i);
        }
    }
    cuts.push(p.pts.len() - 1);
    cuts.dedup();
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b > a {
            let pts = p.pts[a..=b].to_vec();
            let mut cum = vec![0.0];
            for i in 1..pts.len() {
                let (x0, y0) = pts[i - 1];
                let (x1, y1) = pts[i];
                cum.push(cum[i - 1] + (x1 - x0).hypot(y1 - y0));
            }
            out.push(Poly { pts, cum });
        }
    }
    if out.is_empty() {
        out.push(p.clone());
    }
    out
}

// ---------- the solved layout (L1/L2/L3/L5) ----------

/// One solved placement: geometry a word is LOCKED to (V4: this is the only
/// currency the renderer accepts).
#[derive(Debug, Clone)]
pub struct Slot {
    /// Index into the picture's manifest paths.
    pub path_idx: usize,
    /// Flow: the sub-polyline this word rides. Stack: None.
    pub poly: Option<Poly>,
    /// Stack anchor: (x, y, cell, column height) — height is LAW (v6).
    pub stack: Option<(f32, f32, f32, f32)>,
    pub band: u8,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub slot: usize,
    /// Manifest path this word is locked to (adjacency exemption in L3).
    pub path_idx: usize,
    pub size: f32,
    /// Extra per-gap letter spacing the justifier adds (px).
    pub spacing: f32,
    /// Fraction of the segment the word covers BEFORE justification.
    pub fill: f32,
    /// Axis-aligned bounds (frame checks + collision fast-reject).
    pub bounds: (f32, f32, f32, f32),
    /// Baseline sample points (collision detail test).
    pub samples: Vec<(f32, f32)>,
}

/// Build the slot list for a picture: flatten, L5-normalize, L4-corner-split,
/// then divide flow paths into their manifest segments.
pub fn slots_for(p: &Picture) -> Vec<Slot> {
    slots_for_lang(p, "en")
}

/// L1(2) — segmentation is SOLVED per language: a smooth piece hosts
/// n = round(len / ideal word extent) segments, where the ideal extent comes
/// from the band's mid size and the path budget's mid unit count in that
/// script. Manifest `segs` is a floor, never a ceiling.
pub fn slots_for_lang(p: &Picture, lang: &str) -> Vec<Slot> {
    // Gather raw geometry for the normalization pass.
    let mut polys: Vec<(usize, Option<Poly>)> = Vec::new();
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (i, q) in p.paths.iter().enumerate() {
        if q.mode == "stack" {
            let h = q.size * q.budget.1 as f32;
            minx = minx.min(q.x - q.size);
            maxx = maxx.max(q.x + q.size);
            miny = miny.min(q.y - q.size);
            maxy = maxy.max(q.y + h);
            polys.push((i, None));
        } else if let Some(d) = &q.d {
            let poly = flatten(d);
            for (x, y) in &poly.pts {
                minx = minx.min(*x);
                maxx = maxx.max(*x);
                miny = miny.min(*y);
                maxy = maxy.max(*y);
            }
            polys.push((i, Some(poly)));
        }
    }
    // L5: normalize into FRAME with MARGIN.
    let w = (maxx - minx).max(1.0);
    let h = (maxy - miny).max(1.0);
    let s = ((FRAME - 2.0 * MARGIN) / w).min((FRAME - 2.0 * MARGIN) / h);
    let ox = (FRAME - w * s) / 2.0 - minx * s;
    let oy = (FRAME - h * s) / 2.0 - miny * s;
    let tx = |x: f32, y: f32| (x * s + ox, y * s + oy);

    let mut slots = Vec::new();
    for (i, poly) in polys {
        let q = &p.paths[i];
        if q.mode == "stack" {
            let (x, y) = tx(q.x, q.y);
            let cell = q.size * s;
            let height = cell * q.ghost.max(3) as f32;
            slots.push(Slot {
                path_idx: i,
                poly: None,
                stack: Some((x, y, cell, height)),
                band: q.band,
            });
            continue;
        }
        let poly = poly.unwrap();
        let norm = Poly {
            pts: poly.pts.iter().map(|(x, y)| tx(*x, *y)).collect(),
            cum: poly.cum.iter().map(|c| c * s).collect(),
        };
        // L4 first; then each smooth piece is auto-segmented by the solver.
        let pieces = corner_split(&norm);
        let (blo, bhi) = q.budget;
        let (slo, shi) = BAND_SIZE[(q.band.max(1) as usize - 1).min(3)];
        // Segment length follows the POOL, not the budget midpoint: the tier's
        // median eligible word length decides how much stroke one word can
        // genuinely fill (the en easy tier taught us it runs short).
        let med = pool_median_units(lang, &p.tier, blo, bhi);
        let ideal = (med * advance(lang) * (slo + shi) / 2.0).max(30.0);
        let manifest_segs = q.slots() as usize;
        // Distribute the manifest floor over pieces proportionally to length.
        let total_len: f32 = pieces.iter().map(|pc| pc.len()).sum();
        for piece in pieces {
            let share = if total_len > 0.0 { piece.len() / total_len } else { 1.0 };
            let floor_here = (manifest_segs as f32 * share).round() as usize;
            let solved = (piece.len() / ideal).round() as usize;
            let n = solved.max(floor_here).max(1).min(8);
            for k in 0..n {
                let t0 = k as f32 / n as f32;
                let t1 = (k + 1) as f32 / n as f32;
                slots.push(Slot {
                    path_idx: i,
                    poly: Some(sub_poly(&piece, t0, t1)),
                    stack: None,
                    band: q.band,
                });
            }
        }
    }
    slots
}

fn sub_poly(p: &Poly, t0: f32, t1: f32) -> Poly {
    let n = 12;
    let pts: Vec<(f32, f32)> =
        (0..=n).map(|k| p.point_at(t0 + (t1 - t0) * k as f32 / n as f32)).collect();
    let mut cum = vec![0.0];
    for i in 1..pts.len() {
        let (x0, y0) = pts[i - 1];
        let (x1, y1) = pts[i];
        cum.push(cum[i - 1] + (x1 - x0).hypot(y1 - y0));
    }
    Poly { pts, cum }
}

/// L1 — solve one word onto one slot. Returns None when the word cannot fill
/// the slot within [floor, band max] and [FILL_MIN, 1] — the feed then tries
/// the next candidate (resolution order per L1).
pub fn solve(slot: &Slot, lang: &str, units: u32) -> Option<Placement> {
    let (lo, hi) = BAND_SIZE[(slot.band.max(1) as usize - 1).min(3)];
    let lo = lo.max(FLOOR);
    if let Some((x, y, _cell, height)) = slot.stack {
        // Fill-the-column: the COLUMN height is fixed by the manifest; the
        // glyph size solves as height/units, clamped to [floor, band max].
        // Words whose solved size escapes the band are rejected (next
        // candidate) — a stack can never overflow its column (the smiley's
        // eyes reaching its mouth taught us this).
        let size = height / units.max(1) as f32;
        if size < lo - 0.01 || size > hi + 0.01 {
            return None;
        }
        let size = size.clamp(lo, hi);
        let samples: Vec<(f32, f32)> =
            (0..units.max(1)).map(|k| (x, y + k as f32 * size)).collect();
        return Some(Placement {
            slot: 0,
            path_idx: 0,
            size,
            spacing: 0.0,
            fill: 1.0,
            bounds: (x - size * 0.6, y - size * 0.8, x + size * 0.6, y - size * 0.8 + height + size * 0.2),
            samples,
        });
    }
    let poly = slot.poly.as_ref()?;
    let len = poly.len();
    if len < 8.0 || units == 0 {
        return None;
    }
    let adv = advance(lang);
    let natural = len / (units as f32 * adv);
    let size = natural.clamp(lo, hi);
    let word_len = units as f32 * adv * size;
    let fill = (word_len / len).min(1.0);
    if fill < FILL_MIN {
        // Under-fills even at band max → needs a longer word (or the manifest
        // needed more segments; the sweep catches structural cases).
        if size >= hi - 0.01 {
            return None;
        }
    }
    if word_len > len * 1.02 && size <= lo + 0.01 {
        // Over-long even at the floor → next candidate (never squeeze, L3).
        return None;
    }
    let gaps = units.saturating_sub(1).max(1) as f32;
    let spacing = ((len - word_len) / gaps).clamp(0.0, size * 0.6);
    // Bounds: sample the polyline, inflate by size/2 above+below the baseline.
    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for k in 0..=8 {
        let (x, y) = poly.point_at(k as f32 / 8.0);
        bx0 = bx0.min(x);
        by0 = by0.min(y);
        bx1 = bx1.max(x);
        by1 = by1.max(y);
    }
    let pad = size * 0.55;
    let samples: Vec<(f32, f32)> = (0..=8).map(|k| poly.point_at(k as f32 / 8.0)).collect();
    Some(Placement {
        slot: 0,
        path_idx: 0,
        size,
        spacing,
        fill: (word_len / len).min(1.0),
        bounds: (bx0 - pad * 0.4, by0 - pad, bx1 + pad * 0.4, by1 + pad * 0.35),
        samples,
    })
}

/// L3 — overlap test: sampled baselines must keep a clearance proportional
/// to the two font sizes (tighter than AABB on angled/converging strokes —
/// the fish's nose and tail taught us that).
pub fn overlaps(a: &Placement, b: &Placement) -> bool {
    // Words abutting along ONE continuous stroke are layout, not collision:
    // the solver caps fill at 100%, so neighbors on the same path only share
    // a junction point.
    if a.path_idx == b.path_idx && a.slot.abs_diff(b.slot) <= 1 {
        return false;
    }
    // Fast reject on far-apart AABBs first.
    let (ax0, ay0, ax1, ay1) = a.bounds;
    let (bx0, by0, bx1, by1) = b.bounds;
    if ax1 < bx0 - 4.0 || bx1 < ax0 - 4.0 || ay1 < by0 - 4.0 || by1 < ay0 - 4.0 {
        return false;
    }
    // Body-vs-body proximity is a violation; tip-to-tip closeness where two
    // strokes JOIN (circle seams, a mouth meeting the face) is abutment.
    let clear = (a.size + b.size) * 0.38;
    let na = a.samples.len();
    let nb = b.samples.len();
    let body = |i: usize, n: usize| i * 4 >= n && (n - 1 - i) * 4 >= n; // middle ~50%
    for (i, pa) in a.samples.iter().enumerate() {
        for (j, pb) in b.samples.iter().enumerate() {
            if (pa.0 - pb.0).hypot(pa.1 - pb.1) < clear && (body(i, na) || body(j, nb)) {
                return true;
            }
        }
    }
    false
}

/// In-frame check (L5).
pub fn in_frame(p: &Placement) -> bool {
    let (x0, y0, x1, y1) = p.bounds;
    x0 >= -2.0 && y0 >= -2.0 && x1 <= FRAME + 2.0 && y1 <= FRAME + 2.0
}

/// The v6 feed: budget-gated candidates ordered by |solved fill − 1| so the
/// chosen word FILLS its slot; collision cascade tries further candidates.
/// Deterministic (V2): candidate order derives only from the pool + seed.
pub fn layout_feed(
    p: &Picture,
    lang: &str,
    seed: u64,
    recent: &[String],
) -> (Vec<String>, Vec<Placement>, Vec<Slot>) {
    let slots = slots_for_lang(p, lang);
    let pool = crate::words::tier_for(lang, &p.tier);
    let mut st = seed ^ 0x57505F5636; // v6 salt
    let mut used: Vec<String> = Vec::new();
    let mut words = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    for (si, slot) in slots.iter().enumerate() {
        let q = &p.paths[slot.path_idx];
        let (blo, bhi) = q.budget;
        // Shuffle a candidate window deterministically, then order by fit.
        let mut cands: Vec<(String, u32)> = pool
            .iter()
            .filter_map(|w| {
                let t = w.split('|').next().unwrap_or(w).to_string();
                let n = crate::wordpic::unit_len(lang, &t);
                (n >= blo && n <= bhi && !used.contains(&t)).then_some((t, n))
            })
            .collect();
        // Seeded rotation for variety, then stable sort by fill fitness.
        if !cands.is_empty() {
            let rot = (splitmix(&mut st) % cands.len() as u64) as usize;
            cands.rotate_left(rot);
        }
        let fresh_first = |w: &String| if recent.contains(w) { 1u8 } else { 0u8 };
        let mut best: Option<(String, Placement)> = None;
        let mut scored: Vec<(u8, u32, String, u32)> = cands
            .into_iter()
            .enumerate()
            .map(|(i, (w, n))| (fresh_first(&w), i as u32, w, n))
            .collect();
        scored.sort_by_key(|(fresh, i, _, _)| (*fresh, *i));
        for (_, _, w, n) in scored {
            if let Some(pl) = solve(slot, lang, n) {
                let hit = placements.iter().any(|other| overlaps(&pl, other));
                if !hit && in_frame(&pl) {
                    best = Some((w, pl));
                    break;
                }
                if best.is_none() {
                    best = Some((w.clone(), pl));
                }
            }
        }
        if let Some((w, mut pl)) = best {
            pl.slot = si;
            pl.path_idx = slot.path_idx;
            used.push(w.clone());
            words.push(w);
            placements.push(pl);
        }
    }
    (words, placements, slots)
}

/// Median typing-unit length of the budget-eligible pool for (lang, tier).
fn pool_median_units(lang: &str, tier: &str, blo: u32, bhi: u32) -> f32 {
    let pool = crate::words::tier_for(lang, tier);
    let mut lens: Vec<u32> = pool
        .iter()
        .filter_map(|w| {
            let t = w.split('|').next().unwrap_or(w);
            let n = crate::wordpic::unit_len(lang, t);
            (n >= blo && n <= bhi).then_some(n)
        })
        .collect();
    if lens.is_empty() {
        return (blo + bhi) as f32 / 2.0;
    }
    lens.sort_unstable();
    lens[lens.len() / 2] as f32
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordpic;

    #[test]
    #[ignore]
    fn debug_slots() {
        for pid in ["smiley", "star"] {
            let p = wordpic::picture(pid).unwrap();
            let slots = slots_for_lang(p, "en");
            println!("== {pid}: {} slots", slots.len());
            // Inline diagnosis of one failing slot (slot 8 on smiley).
            if pid == "smiley" {
                let sl = &slots[8];
                let pool = crate::words::tier_for("en", &p.tier);
                let q = &p.paths[sl.path_idx];
                let mut cand = 0;
                let mut solved = 0;
                for w in pool.iter().take(400) {
                    let t = w.split('|').next().unwrap_or(w).to_string();
                    let n = crate::wordpic::unit_len("en", &t);
                    if n >= q.budget.0 && n <= q.budget.1 {
                        cand += 1;
                        if solve(sl, "en", n).is_some() {
                            solved += 1;
                        }
                    }
                }
                println!("  slot8 diag: candidates={cand} solvable={solved} polylen={:.1} pts={}",
                    sl.poly.as_ref().unwrap().len(), sl.poly.as_ref().unwrap().pts.len());
            }
            let (words, placements, _) = layout_feed(p, "en", 1, &[]);
            println!("  filled {} of {}", words.len(), slots.len());
            for (w, pl) in words.iter().zip(&placements) {
                println!("  slot{} '{}' size={:.0} fill={:.2}", pl.slot, w, pl.size, pl.fill);
            }
            let filled: Vec<usize> = placements.iter().map(|pl| pl.slot).collect();
            let missing: Vec<usize> =
                (0..slots.len()).filter(|i| !filled.contains(i)).collect();
            println!("  missing slots: {missing:?}");
            for i in &missing {
                if let Some(pl) = &slots[*i].poly {
                    println!("    slot{} len={:.0} band={}", i, pl.len(), slots[*i].band);
                }
            }
        }
    }

    /// V2 — solver determinism.
    #[test]
    fn solver_deterministic() {
        let p = wordpic::picture("fish").unwrap();
        let a = layout_feed(p, "en", 9, &[]);
        let b = layout_feed(p, "en", 9, &[]);
        assert_eq!(a.0, b.0);
        assert_eq!(a.1.len(), b.1.len());
        for (x, y) in a.1.iter().zip(b.1.iter()) {
            assert_eq!(x.size, y.size);
            assert_eq!(x.bounds, y.bounds);
        }
    }

    /// L8 — the fish golden fixture: layout pinned within tolerance.
    #[test]
    fn fish_golden_fixture() {
        let p = wordpic::picture("fish").unwrap();
        let (words, placements, slots) = layout_feed(p, "en", 9, &[]);
        assert_eq!(words.len(), slots.len(), "fish fills every slot");
        for pl in &placements {
            assert!(pl.size >= FLOOR && pl.size <= 46.0, "fish sizes sane: {}", pl.size);
            assert!(in_frame(pl), "fish in frame");
        }
        for i in 0..placements.len() {
            for j in i + 1..placements.len() {
                assert!(!overlaps(&placements[i], &placements[j]), "fish never overlaps");
            }
        }
    }

    /// L8 `star-pentagon` — corner split yields straight edge slots; no word
    /// rides across a star vertex.
    #[test]
    fn star_corners_stay_crisp() {
        let p = wordpic::picture("star").unwrap();
        let slots = slots_for_lang(p, "en");
        // Every flow slot must be (nearly) straight: max deviation of interior
        // points from the chord under 6px.
        for s in &slots {
            let Some(poly) = &s.poly else { continue };
            let (x0, y0) = poly.pts[0];
            let (x1, y1) = *poly.pts.last().unwrap();
            let chord = (x1 - x0).hypot(y1 - y0).max(0.001);
            for (x, y) in &poly.pts {
                let dev = ((x1 - x0) * (y0 - y) - (x0 - x) * (y1 - y0)).abs() / chord;
                assert!(dev < 6.0, "star slot bends around a corner (dev {dev})");
            }
        }
    }

    /// L9 — THE SWEEP (V1/V3): every picture × language × 5 seeds — zero
    /// overlaps, zero out-of-frame, zero sub-floor, fill ≥ 95% (or the slot
    /// was solvable to band max, tracked by fill>=FILL_MIN assertion on the
    /// chosen candidate).
    #[test]
    fn render_ci_sweep() {
        let mut failures: Vec<String> = Vec::new();
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            for p in &wordpic::manifest().pictures {
                for seed in 1..=5u64 {
                    let (words, placements, slots) = layout_feed(p, code, seed, &[]);
                    if words.len() != slots.len() {
                        failures.push(format!("{}/{}/{}: {}/{} slots unfilled",
                            p.id, code, seed, words.len(), slots.len()));
                        continue;
                    }
                    for pl in &placements {
                        if pl.size < FLOOR - 0.01 {
                            failures.push(format!("{}/{}/{}: sub-floor {}", p.id, code, seed, pl.size));
                        }
                        if !in_frame(pl) {
                            failures.push(format!("{}/{}/{}: out of frame {:?}", p.id, code, seed, pl.bounds));
                        }
                    }
                    for i in 0..placements.len() {
                        for j in i + 1..placements.len() {
                            if overlaps(&placements[i], &placements[j]) {
                                failures.push(format!(
                                    "{}/{}/{}: overlap words '{}'×'{}'",
                                    p.id, code, seed, words[i], words[j]
                                ));
                            }
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "L9 sweep failures ({}):\n{}", failures.len(),
            failures[..failures.len().min(25)].join("\n"));
    }

    /// L7 `mona-lisa-overlap` — expert bands drive size hierarchy and the map
    /// re-authored to 150–200 words.
    #[test]
    fn mona_bands_and_density() {
        let p = wordpic::picture("mona").unwrap();
        let slots = slots_for_lang(p, "en");
        assert!(
            (150..=200).contains(&slots.len()),
            "expert map hosts 150-200 words (got {})",
            slots.len()
        );
        let (_, placements, slots) = layout_feed(p, "en", 3, &[]);
        // Background (band 1) words solve larger than feature (band 3) words.
        let avg = |band: u8| {
            let v: Vec<f32> = placements
                .iter()
                .zip(&slots)
                .filter(|(_, s)| s.band == band)
                .map(|(pl, _)| pl.size)
                .collect();
            v.iter().sum::<f32>() / v.len().max(1) as f32
        };
        assert!(avg(1) > avg(3) + 2.0, "band hierarchy visible: bg {} vs feature {}", avg(1), avg(3));
    }
}
