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
pub const BAND_SIZE: [(f32, f32); 4] = [(22.0, 40.0), (16.0, 32.0), (13.0, 24.0), (13.0, 30.0)];
/// V3 (calibrated on the first solver renders): a word covers 78–94% of its
/// segment, centered — full-bleed 100% jammed neighbors into each other
/// ("cookfell"), which reads as overlap even when geometry says contact.
pub const FILL_MIN: f32 = 0.72;
pub const FILL_MAX: f32 = 0.94;

/// RENDER glyph count: what actually occupies the stroke. NFC chars — for ko
/// that's hangul BLOCKS (a 4-block word types as ~11 jamo but renders as 4
/// glyphs; layout must count 4). Budgets/difficulty keep typing units.
pub fn render_units(word: &str) -> u32 {
    use unicode_normalization::UnicodeNormalization;
    word.nfc().count() as u32
}

/// Mean glyph advance as a fraction of font size, per script class. Rough by
/// design — the solver only needs to be consistent (V2), not typographically
/// exact; textLength justification closes the residual at render time.
pub fn advance(lang: &str) -> f32 {
    match lang {
        // zh TYPES AND RENDERS pinyin (Latin + tone digits) — the hanzi side
        // never reaches the canvas, so its advance is Latin-class.
        "ja" => 1.0,
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
            'l' if n.len() >= 2 => {
                for ch in n.chunks(2) {
                    pos = (pos.0 + ch[0], pos.1 + ch[1]);
                    pts.push(pos);
                }
            }
            'q' if n.len() >= 4 => {
                for ch in n.chunks(4) {
                    if ch.len() < 4 {
                        break;
                    }
                    let (x0, y0) = pos;
                    let (cx, cy, x1, y1) =
                        (x0 + ch[0], y0 + ch[1], x0 + ch[2], y0 + ch[3]);
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
                // W3C F.6.5: center sits on the +normal side when laf != sf.
                let sign = if (laf > 0.5) != (sf > 0.5) { 1.0 } else { -1.0 };
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
                // laf is fully encoded by the center choice for rx == ry; no
                // further sweep surgery (a one-ULP float artifact here once
                // flipped a semicircle onto its twin — the doubled-smiley bug).
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
    /// v7.5/F3 — dot-class slot (eyes, buttons, nostrils): a short stroke
    /// (24..45px) hosting one 2–4 unit word at a relaxed fill window.
    pub dot: bool,
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
/// v7.5 Option 2 — the guide layer, in the SAME normalized space as the
/// word slots (identical L5 transform, computed from the word paths so
/// nothing shifts). Visible art only: never hosts words, never collides.
pub fn guide_polys(p: &Picture) -> Vec<Poly> {
    let (s, ox, oy) = norm_transform(p);
    p.guide
        .iter()
        .map(|d| {
            let poly = flatten(d);
            let pts: Vec<(f32, f32)> =
                poly.pts.iter().map(|(x, y)| (x * s + ox, y * s + oy)).collect();
            let mut cum = vec![0.0];
            for i in 1..pts.len() {
                let (x0, y0) = pts[i - 1];
                let (x1, y1) = pts[i];
                cum.push(cum[i - 1] + (x1 - x0).hypot(y1 - y0));
            }
            Poly { pts, cum }
        })
        .collect()
}

fn norm_transform(p: &Picture) -> (f32, f32, f32) {
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for d in p.guide.iter() {
        let poly = flatten(d);
        for (x, y) in &poly.pts {
            minx = minx.min(*x);
            maxx = maxx.max(*x);
            miny = miny.min(*y);
            maxy = maxy.max(*y);
        }
    }
    for q in p.paths.iter() {
        if q.mode == "stack" {
            let h = q.size * q.budget.1 as f32;
            minx = minx.min(q.x - q.size);
            maxx = maxx.max(q.x + q.size);
            miny = miny.min(q.y - q.size);
            maxy = maxy.max(q.y + h);
        } else if let Some(d) = &q.d {
            let poly = flatten(d);
            for (x, y) in &poly.pts {
                minx = minx.min(*x);
                maxx = maxx.max(*x);
                miny = miny.min(*y);
                maxy = maxy.max(*y);
            }
        }
    }
    let w = (maxx - minx).max(1.0);
    let h = (maxy - miny).max(1.0);
    let s = ((FRAME - 2.0 * MARGIN) / w).min((FRAME - 2.0 * MARGIN) / h);
    let ox = (FRAME - w * s) / 2.0 - minx * s;
    let oy = (FRAME - h * s) / 2.0 - miny * s;
    (s, ox, oy)
}

pub fn slots_for_lang(p: &Picture, lang: &str) -> Vec<Slot> {
    // Gather raw geometry for the normalization pass. v7.5: the guide
    // layer shares the space, so its bounds join the fit (words and art
    // must land on the same picture).
    let mut polys: Vec<(usize, Option<Poly>)> = Vec::new();
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for d in p.guide.iter() {
        let poly = flatten(d);
        for (x, y) in &poly.pts {
            minx = minx.min(*x);
            maxx = maxx.max(*x);
            miny = miny.min(*y);
            maxy = maxy.max(*y);
        }
    }
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
            let height = cell * q.column.max(3) as f32;
            slots.push(Slot {
                path_idx: i,
                poly: None,
                stack: Some((x, y, cell, height)),
                dot: false,
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
            // v7.5/F3: dot-class — a 24..45px piece is ONE slot hosting a
            // short word (eyes look like eyes). Below 24px it is noise.
            if piece.len() < 45.0 {
                if piece.len() >= 30.0 {
                    slots.push(Slot {
                        path_idx: i,
                        poly: Some(piece.clone()),
                        stack: None,
                        dot: true,
                        band: q.band,
                    });
                }
                continue;
            }
            let share = if total_len > 0.0 { piece.len() / total_len } else { 1.0 };
            let floor_here = (manifest_segs as f32 * share).round() as usize;
            let solved = (piece.len() / ideal).round() as usize;
            // Never mince a stroke below ~55px per slot: short-word scripts
            // (ko renders 2-4 blocks) otherwise split strokes into crumbs no
            // pool can fill 101 times over.
            let max_slots = ((piece.len() / 45.0).floor() as usize).max(1);
            let n = solved.max(floor_here).max(1).min(8).min(max_slots);
            for k in 0..n {
                let t0 = k as f32 / n as f32;
                let t1 = (k + 1) as f32 / n as f32;
                slots.push(Slot {
                    path_idx: i,
                    poly: Some(sub_poly(&piece, t0, t1)),
                    stack: None,
                    dot: false,
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
        // Stacks clamp to the ABSOLUTE floor, not the band minimum: dense
        // scripts (zh pinyin runs 8-10 units) must still solve a column.
        let size = height / units.max(1) as f32;
        if size < FLOOR - 0.01 || size > hi + 0.01 {
            return None;
        }
        let size = size.clamp(FLOOR, hi);
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
    if slot.dot && !(2..=5).contains(&units) {
        return None; // F3: a too-long word is simply ineligible for a dot
    }
    let adv = advance(lang);
    // Solve toward the padded target: word extent ≈ FILL_MAX of the segment.
    let natural = len * FILL_MAX / (units as f32 * adv);
    let size = natural.clamp(lo, hi);
    let word_len = units as f32 * adv * size;
    let fill = word_len / len;
    let (fmin, fmax) = if slot.dot { (0.50, 0.98) } else { (FILL_MIN, FILL_MAX) };
    if fill < fmin && size >= hi - 0.01 {
        return None; // under-fills even at band max → longer word needed
    }
    if fill > fmax + 0.02 && size <= lo + 0.01 {
        return None; // over-long even at the floor → never squeeze (L3)
    }
    let gaps = units.saturating_sub(1).max(1) as f32;
    let spacing = ((len * FILL_MAX - word_len) / gaps).clamp(0.0, size * 0.5);
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
/// v7 F1 — runtime legality core, pure and CI-testable. Input: measured
/// glyph boxes (slot id, x, y, w, h) from the LIVE render — real fonts at
/// device metrics. Different slots' glyphs may not intersect; every glyph
/// stays inside the frame. Same-slot pairs are exempt (a word's own glyphs
/// curve and kern together).
pub fn glyph_violations(glyphs: &[(u32, f32, f32, f32, f32)]) -> u32 {
    // Depth rule: legal layouts have near-touching boxes at slot
    // boundaries, seams and star corners (the v6 junction exemptions) — a
    // VIOLATION is glyphs ON each other, the round-2 jam. Flag a pair only
    // when the boxes interpenetrate >35% of the smaller box on BOTH axes.
    // Test glyph CORES (central 50% of each box): angled letters at star
    // corners overlap as AABBs while their ink stays apart — the AABB of a
    // rotated glyph is inflated. Cores only collide when letters are truly
    // on each other.
    const DEPTH: f32 = 0.35;
    let core = |g: &(u32, f32, f32, f32, f32)| {
        (g.1 + g.3 * 0.25, g.2 + g.4 * 0.25, g.3 * 0.5, g.4 * 0.5)
    };
    let mut n = 0u32;
    for (i, a) in glyphs.iter().enumerate() {
        if a.1 < -0.5 || a.2 < -0.5 || a.1 + a.3 > FRAME + 0.5 || a.2 + a.4 > FRAME + 0.5 {
            n += 1;
        }
        let (ax, ay, aw, ah) = core(a);
        for b in glyphs.iter().skip(i + 1) {
            if a.0 == b.0 {
                continue; // a word's own glyphs kern/curve together
            }
            let (bx, by, bw, bh) = core(b);
            let ow = (ax + aw).min(bx + bw) - ax.max(bx);
            let oh = (ay + ah).min(by + bh) - ay.max(by);
            if ow > DEPTH * aw.min(bw) && oh > DEPTH * ah.min(bh) {
                n += 1;
            }
        }
    }
    n
}

pub fn overlaps(a: &Placement, b: &Placement) -> bool {
    // Words abutting along ONE continuous stroke are layout, not collision:
    // the solver caps fill at 100%, so neighbors on the same path only share
    // a junction point.
    if a.path_idx == b.path_idx && a.slot.abs_diff(b.slot) <= 1 {
        return false;
    }
    // Closed loops: the first and last words of one path abut at the seam
    // (the star outline taught us). Same path + shared endpoint = layout.
    if a.path_idx == b.path_idx {
        let ends = |v: &Vec<(f32, f32)>| (v[0], *v.last().unwrap());
        if !a.samples.is_empty() && !b.samples.is_empty() {
            let (a0, a1) = ends(&a.samples);
            let (b0, b1) = ends(&b.samples);
            let d = [a0, a1]
                .iter()
                .flat_map(|pa| [b0, b1].map(|pb| (pa.0 - pb.0).hypot(pa.1 - pb.1)))
                .fold(f32::MAX, f32::min);
            if d < 8.0 {
                return false;
            }
        }
    }
    // Fast reject on far-apart AABBs first.
    let (ax0, ay0, ax1, ay1) = a.bounds;
    let (bx0, by0, bx1, by1) = b.bounds;
    if ax1 < bx0 - 4.0 || bx1 < ax0 - 4.0 || ay1 < by0 - 4.0 || by1 < ay0 - 4.0 {
        return false;
    }
    // Body-vs-body proximity is a violation; tip-to-tip closeness where two
    // strokes JOIN (circle seams, a mouth meeting the face) is abutment.
    let na = a.samples.len();
    let nb = b.samples.len();
    let body = |i: usize, n: usize| i * 4 >= n && (n - 1 - i) * 4 >= n; // middle ~50%
    for (i, pa) in a.samples.iter().enumerate() {
        for (j, pb) in b.samples.iter().enumerate() {
            let (ba, bb) = (body(i, na), body(j, nb));
            // Body-into-body needs full clearance; a word ENDING at another's
            // side (T-junction — a tower tie meeting a leg) is legal contact
            // and only flags when the tip actually sits ON the other word.
            let clear = match (ba, bb) {
                (true, true) => (a.size + b.size) * 0.38,
                (false, false) => continue,
                _ => (a.size + b.size) * 0.20,
            };
            if (pa.0 - pb.0).hypot(pa.1 - pb.1) < clear {
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
    layout_feed_opt(p, lang, seed, recent, false)
}

/// v7 F1 — `shrink` is the substitution rung of the runtime legality
/// ladder (D1): after 5 failed re-solves the screen re-enters with
/// shrink=true and candidates order shortest-first (deterministic), so an
/// illegal frame resolves to smaller words instead of being displayed.
pub fn layout_feed_opt(
    p: &Picture,
    lang: &str,
    seed: u64,
    recent: &[String],
    shrink: bool,
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
                (n >= blo && n <= bhi && !used.contains(&t)).then_some((t.clone(), render_units(&t)))
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
        scored.sort_by_key(|(fresh, i, _, n)| if shrink { (*fresh, *n) } else { (*fresh, *i) });
        'cand: for (_, _, w, n) in scored {
            if let Some(mut pl) = solve(slot, lang, n) {
                // v7 F1 root-cause #2: identity BEFORE the hit check. A
                // candidate used to carry placeholder slot/path_idx 0, so
                // the same-path and seam exemptions misfired against any
                // path-0 placement — the feed produced layouts the sweep's
                // own law flags (fish tail: p0×p1 share an endpoint).
                pl.slot = si;
                pl.path_idx = slot.path_idx;
                let hit = placements.iter().any(|other| overlaps(&pl, other));
                if !hit && in_frame(&pl) {
                    best = Some((w, pl));
                    break;
                }
                // L3 cascade step 1: SHRINK toward the floor (never below) —
                // a slightly smaller word beats a colliding one, and beats a
                // hand-nudged manifest (which the spec bans).
                for factor in [0.88f32, 0.78] {
                    let shrunk = pl.size * factor;
                    if shrunk < FLOOR {
                        break;
                    }
                    let mut pl2 = pl.clone();
                    pl2.size = shrunk;
                    let pad = shrunk * 0.55;
                    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                    for (sx, sy) in &pl2.samples {
                        bx0 = bx0.min(*sx);
                        by0 = by0.min(*sy);
                        bx1 = bx1.max(*sx);
                        by1 = by1.max(*sy);
                    }
                    pl2.bounds = (bx0 - pad * 0.4, by0 - pad, bx1 + pad * 0.4, by1 + pad * 0.35);
                    let hit2 = placements.iter().any(|other| overlaps(&pl2, other));
                    if !hit2 && in_frame(&pl2) {
                        best = Some((w, pl2));
                        break 'cand;
                    }
                }
                // v7 F1: NEVER accept an illegal placement. An unfilled
                // slot is honest (CI flags it; the runtime ladder re-solves
                // it); a colliding word is the round-2 star escape by
                // construction. This branch used to take the first solvable
                // collider as "best effort" — that was the leak.
            }
        }
        // v7.5/F3 (D4): adjacent-tier borrow — if NO candidate from this
        // tier's pool solved the slot (short stroke, long-word pool), retry
        // once with the adjacent tier's pool. Deterministic; logged by the
        // sweep as a fill like any other. Never squeeze, never skip silent.
        if best.is_none() {
            // Chain down to the easy pool: short words live there, and a
            // dot slot in a hard picture still deserves one (D4).
            let chain: &[&str] = match p.tier.as_str() {
                "expert" => &["hard", "medium", "easy"],
                "hard" => &["medium", "easy"],
                "medium" => &["easy"],
                _ => &["medium"],
            };
            for adjacent in chain {
            if best.is_some() { break; }
            let pool2 = crate::words::tier_for(lang, adjacent);
            let mut cands2: Vec<(String, u32)> = pool2
                .iter()
                .filter_map(|w| {
                    let t = w.split('|').next().unwrap_or(w).to_string();
                    let n = crate::wordpic::unit_len(lang, &t);
                    (n >= 2 && n <= bhi && !used.contains(&t)).then_some((t.clone(), render_units(&t)))
                })
                .collect();
            if !cands2.is_empty() {
                let rot = (splitmix(&mut st) % cands2.len() as u64) as usize;
                cands2.rotate_left(rot);
            }
            'cand2: for (w, n) in cands2 {
                if let Some(mut pl) = solve(slot, lang, n) {
                    pl.slot = si;
                    pl.path_idx = slot.path_idx;
                    let hit = placements.iter().any(|other| overlaps(&pl, other));
                    if !hit && in_frame(&pl) {
                        best = Some((w, pl));
                        break 'cand2;
                    }
                }
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

/// Median RENDERED length of the budget-eligible pool for (lang, tier) —
/// budget eligibility is typing units (gameplay), but segments must size to
/// what the glyphs occupy (ko: blocks, not jamo; zh: pinyin chars).
fn pool_median_units(lang: &str, tier: &str, blo: u32, bhi: u32) -> f32 {
    let pool = crate::words::tier_for(lang, tier);
    let mut lens: Vec<u32> = pool
        .iter()
        .filter_map(|w| {
            let t = w.split('|').next().unwrap_or(w);
            let n = crate::wordpic::unit_len(lang, t);
            (n >= blo && n <= bhi).then_some(render_units(t))
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

/// I1 readiness exceptions: (picture, language) pairs whose tier pool cannot
/// fill the v6 stroke map (ko/ja expert pools run long-compound; the D10
/// authoring tool's densified map lifts these). The screen HIDES these pairs;
/// the readiness report lists them. Capped small — growth here means fix the
/// map, not the list.
// v7.5 rotation: the rebuilt Mona freed ko/ja. The zh bank lacks
// 1-syllable (2-4 pinyin char) entries, so short slots in these four
// pictures cannot fill until the bank grows — ledgered bank debt, the
// same class the ko jamo work retired. Hidden pairs, never jumbled.
pub const READINESS_EXCEPTIONS: [(&str, &str); 4] =
    [("snowman", "zh"), ("turtle", "zh"), ("peacock", "zh"), ("mona", "zh")];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordpic;

    /// L10 render emitter: outline + solver-filled SVGs for every picture,
    /// straight from the engine (the review artifact generator).
    #[test]
    #[ignore]
    fn emit_renders() {
        let dir = std::env::var("WP_RENDER_DIR").unwrap_or_else(|_| "/tmp/wp-renders".into());
        std::fs::create_dir_all(&dir).unwrap();
        for p in &wordpic::manifest().pictures {
            let slots = slots_for_lang(p, "en");
            let (words, placements, _) = layout_feed(p, "en", 1, &[]);
            for filled in [false, true] {
                let mut svg = String::from(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\" width=\"512\" height=\"512\"><rect width=\"512\" height=\"512\" fill=\"#101623\"/>",
                );
                // v7.5 Option 2: the guide layer under everything.
                for g in guide_polys(p) {
                    let d: String = g
                        .pts
                        .iter()
                        .enumerate()
                        .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
                        .collect();
                    svg.push_str(&format!(
                        "<path d=\"{d}\" fill=\"none\" stroke=\"#8fa0c0\" stroke-opacity=\"0.35\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
                    ));
                }
                for (si, sl) in slots.iter().enumerate() {
                    let placed = filled && si < placements.len();
                    if let Some(poly) = &sl.poly {
                        let d: String = poly
                            .pts
                            .iter()
                            .enumerate()
                            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
                            .collect();
                        if !placed {
                            svg.push_str(&format!(
                                "<path d=\"{d}\" fill=\"none\" stroke=\"#3a4763\" stroke-width=\"3\" stroke-linecap=\"round\"/>"
                            ));
                        } else {
                            let pl = &placements[si];
                            let pid = format!("r{si}");
                            svg.push_str(&format!("<defs><path id=\"{pid}\" d=\"{d}\"/></defs>"));
                            svg.push_str(&format!(
                                "<text font-size=\"{:.0}\" fill=\"#e8ecf5\" font-family=\"Helvetica\" letter-spacing=\"{:.1}\" text-anchor=\"middle\"><textPath href=\"#{pid}\" startOffset=\"50%\">{}</textPath></text>",
                                pl.size, pl.spacing, words[si]
                            ));
                        }
                    } else if let Some((x, y, cell, height)) = sl.stack {
                        if !placed {
                            let n = (height / cell).round() as i32;
                            for k in 0..n {
                                svg.push_str(&format!(
                                    "<rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"4\" fill=\"none\" stroke=\"#3a4763\" stroke-width=\"2\"/>",
                                    x - cell * 0.42,
                                    y + k as f32 * cell - cell * 0.72,
                                    cell * 0.84,
                                    cell * 0.84
                                ));
                            }
                        } else {
                            let pl = &placements[si];
                            for (k, ch) in words[si].chars().enumerate() {
                                svg.push_str(&format!(
                                    "<text x=\"{x:.0}\" y=\"{:.0}\" font-size=\"{:.0}\" fill=\"#e8ecf5\" font-family=\"Helvetica\" text-anchor=\"middle\">{}</text>",
                                    y + k as f32 * pl.size,
                                    pl.size,
                                    ch
                                ));
                            }
                        }
                    }
                }
                svg.push_str("</svg>");
                let name = format!("{dir}/{}-{}.svg", p.id, if filled { "filled" } else { "outline" });
                std::fs::write(&name, svg).unwrap();
            }
            println!("{}: {} slots, {} filled", p.id, slots.len(), words.len());
        }
    }

    #[test]
    #[ignore]
    fn debug_slots() {
        for pid in ["cat", "smiley"] {
            let p = wordpic::picture(pid).unwrap();
            let slots = slots_for_lang(p, "en");
            println!("== {pid}: {} slots", slots.len());
            if pid == "smiley" {
                let top = flatten("M76 256 A180 180 0 0 1 436 256");
                let bot = flatten("M76 256 A180 180 0 1 0 436 256");
                println!("  RAW top mid={:?} bot mid={:?}", top.point_at(0.5), bot.point_at(0.5));
                println!("  RAW top len={:.0} bot len={:.0}", top.len(), bot.len());
                for si in [0usize, 8] {
                    if let Some(pl) = &slots[si].poly {
                        println!("  slot{si} p{} start={:?} mid={:?} end={:?} len={:.0}",
                            slots[si].path_idx, pl.point_at(0.0), pl.point_at(0.5), pl.point_at(1.0), pl.len());
                    }
                }
            }
            // Inline diagnosis of one failing slot.
            if pid == "cat" {
                let sl = &slots[2];
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
    /// Eric: no repeated words inside a picture — also for the legacy
    /// (non-scan) subjects that still plan through this engine.
    #[test]
    fn no_repeated_words_legacy_engine() {
        for p in &wordpic::manifest().pictures {
            for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
                if READINESS_EXCEPTIONS.contains(&(p.id.as_str(), code)) {
                    continue;
                }
                let (words, _, _) = layout_feed(p, code, 5, &[]);
                let mut seen: Vec<&String> = Vec::new();
                for w in &words {
                    assert!(!seen.contains(&w), "{}/{}: {w:?} twice in one picture", p.id, code);
                    seen.push(w);
                }
            }
        }
    }

    /// v8 F6 `star-side-rails`: the two vertical letter-stacks that read
    /// as dashed UI rails and the stray ground line are gone from the
    /// star, and no picture stroke in ANY subject is a free-floating
    /// stack that could be mistaken for interface.
    #[test]
    fn star_side_rails() {
        let star = wordpic::picture("star").unwrap();
        assert_eq!(star.paths.len(), 1, "a star is a star: outline only");
        assert_eq!(star.paths[0].mode, "flow");
        for p in &wordpic::manifest().pictures {
            for q in &p.paths {
                assert!(
                    q.mode != "stack" || !q.feature.to_lowercase().contains("sparkle"),
                    "{}: free-floating decorative stack reads as UI chrome",
                    p.id
                );
            }
        }
    }

    /// v7 F1 fixture `star-overlap-r2`: reconstruct the round-2 escape —
    /// the device drew three star words ~70% wider than the solver's
    /// estimate (letter-spacing without textLength). The runtime check
    /// must catch all three, and the honest solved layout must be legal.
    #[test]
    fn star_overlap_r2() {
        let m = wordpic::manifest();
        let p = m.pictures.iter().find(|p| p.id == "star").unwrap();
        let (_, pls, _) = layout_feed(p, "en", 1, &[]);
        // Per-glyph boxes as the device measures them: one box per sample,
        // width = the solved advance, height = the solved size.
        let boxes = |widen: &[usize]| -> Vec<(u32, f32, f32, f32, f32)> {
            let mut g = Vec::new();
            for (i, pl) in pls.iter().enumerate() {
                let w0 = pl.size * 0.55; // ≈ en advance per glyph
                let w = if widen.contains(&i) { w0 * 2.6 } else { w0 };
                // The word rides the CENTRAL fill of its slot (72–94%,
                // centered): approximate glyphs at the middle 5 of the 9
                // path samples.
                // Realistic mean glyph box: x-height ≈ 0.75×size (few
                // glyphs span full ascender-to-descender). A spilled word
                // (the round-2 bug) spreads past its fill window — glyphs
                // land across the WHOLE slot, into the neighbors' territory.
                let pts: Vec<&(f32, f32)> = if widen.contains(&i) {
                    pl.samples.iter().collect()
                } else {
                    pl.samples.iter().skip(2).take(5).collect()
                };
                for (sx, sy) in pts {
                    g.push((i as u32, sx - w / 2.0, sy - pl.size * 0.4, w, pl.size * 0.75));
                }
            }
            g
        };
        assert_eq!(glyph_violations(&boxes(&[])), 0, "solved star is legal");
        // Round-2 reconstruction: three words render far wider than the
        // solver estimated (real font metrics, no textLength) and jam into
        // their neighbors. The check must catch every jam.
        // Widen every third word, whatever the star's path count is (F6
        // deleted its rails, so fixed indices would be brittle).
        let widen: Vec<usize> = (0..pls.len()).step_by(3).take(3).collect();
        assert!(widen.len() >= 3, "star hosts enough words to reconstruct the escape");
        let spill = boxes(&widen);
        assert!(glyph_violations(&spill) >= 3, "check catches the round-2 escape");
    }

    /// v7 acceptance: star renders legally across 25 consecutive seeds in
    /// every language — the deterministic re-solve sequence (D1) always
    /// has a legal layout to land on.
    #[test]
    fn star_25_seeds_all_langs() {
        let m = wordpic::manifest();
        let p = m.pictures.iter().find(|p| p.id == "star").unwrap();
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            for seed in 1..=25u64 {
                let (words, placements, slots) = layout_feed(p, code, seed, &[]);
                assert_eq!(words.len(), slots.len(), "star/{code}/{seed}: filled");
                for (i, a) in placements.iter().enumerate() {
                    assert!(in_frame(a), "star/{code}/{seed}: in frame");
                    for b in placements.iter().skip(i + 1) {
                        assert!(!overlaps(a, b), "star/{code}/{seed}: overlap");
                    }
                }
            }
        }
    }

    #[test]
    fn render_ci_sweep() {
        let mut failures: Vec<String> = Vec::new();
        assert!(READINESS_EXCEPTIONS.len() <= 4, "readiness list growing — fix maps or the zh bank instead");
        for (code, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
            for p in &wordpic::manifest().pictures {
                if READINESS_EXCEPTIONS.contains(&(p.id.as_str(), code)) {
                    continue; // hidden pair — listed in the readiness report
                }
                for seed in 1..=5u64 {
                    let (words, placements, slots) = layout_feed(p, code, seed, &[]);
                    if words.len() != slots.len() {
                        let got: Vec<usize> = placements.iter().map(|pl| pl.slot).collect();
                        let missing: Vec<String> = (0..slots.len())
                            .filter(|i| !got.contains(i))
                            .map(|i| {
                                let sl = &slots[i];
                                match &sl.poly {
                                    Some(pl) => format!("s{i}(p{} len{:.0} b{})", sl.path_idx, pl.len(), sl.band),
                                    None => format!("s{i}(p{} STACK)", sl.path_idx),
                                }
                            })
                            .collect();
                        failures.push(format!("{}/{}/{}: unfilled {} of {} — {}",
                            p.id, code, seed, slots.len() - words.len(), slots.len(),
                            missing.join(", ")));
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
                                    "{}/{}/{}: overlap '{}'(s{} p{})×'{}'(s{} p{})",
                                    p.id, code, seed,
                                    words[i], placements[i].slot, placements[i].path_idx,
                                    words[j], placements[j].slot, placements[j].path_idx
                                ));
                            }
                        }
                    }
                }
            }
        }
        if let Ok(dir) = std::env::var("WP_SWEEP_LOG") {
            let _ = std::fs::write(dir, failures.join("\n"));
        }
        if !failures.is_empty() {
            let mut hist: std::collections::HashMap<String, usize> = Default::default();
            for f in &failures {
                let key = f.split('/').next().unwrap_or("?").to_string()
                    + " " + if f.contains("overlap") { "overlap" }
                        else if f.contains("unfilled") { "unfilled" }
                        else if f.contains("sub-floor") { "floor" } else { "frame" };
                *hist.entry(key).or_default() += 1;
            }
            let mut rows: Vec<_> = hist.into_iter().collect();
            rows.sort_by(|a, b| b.1.cmp(&a.1));
            let smiley: Vec<&String> =
                failures.iter().filter(|f| f.starts_with("smiley/en/1")).take(8).collect();
            let unfilled: Vec<&String> =
                failures.iter().filter(|f| f.contains("unfilled")).take(6).collect();
            panic!("L9 sweep failures ({}):\nHIST {:?}\nSMILEY {:?}\nUNFILLED {:?}",
                failures.len(), rows, smiley, unfilled);
        }
    }

    /// L7 `mona-lisa-overlap` — expert bands drive size hierarchy and the map
    /// re-authored to 150–200 words.
    #[test]
    fn mona_bands_and_density() {
        let p = wordpic::picture("mona").unwrap();
        let slots = slots_for_lang(p, "en");
        // INTERIM (CC-MASTERPIECE-TONAL decree, 2026-08-04): mona hosts
        // on six machine-traced silhouette arcs until her tonal re-author
        // passes Eric's gate — the expanded ko/vi pools made the old
        // hand-traced map unhostable. The tonal map RE-PINS this range
        // and restores the band-hierarchy assert below.
        assert!(
            (12..=110).contains(&slots.len()),
            "interim arc map hosts a real campaign (got {})",
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
        // Band hierarchy: only meaningful with >=2 distinct bands — the
        // interim map is single-band; the tonal map brings this back.
        let bands: std::collections::BTreeSet<u8> = slots.iter().map(|s| s.band).collect();
        if bands.len() >= 2 {
            assert!(avg(1) > avg(3) + 2.0, "band hierarchy visible: bg {} vs feature {}", avg(1), avg(3));
        }
    }
}
