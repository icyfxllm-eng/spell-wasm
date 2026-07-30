//! CC-WORDPICTURE-SCANLOCK v8 core.
//!
//! F1: every word is text-on-path along the pinned path's arc length;
//! the baseline IS the path (erase the letters, recover the scan).
//! F2: size = arc/chars clamped [floor, band max]; spacing justifies to
//! fill the segment exactly; split only at pre-marked boundaries; else
//! INVALID and redraw. F3: collisions resolve in word choice and size,
//! never geometry; 3 consecutive seed failures on one pair = subject
//! fails typesettability. I2: geometry in, placements out — no API here
//! can write, resample, or transform a path.

/// A pinned scan path (verbatim coordinates) + its scan-time marks.
#[derive(Debug, Clone)]
pub struct ScanPath {
    pub points: Vec<(f32, f32)>,
    /// Pre-marked segment boundaries as arc fractions (F2's ONLY splits).
    pub segments: Vec<f32>,
    pub sub_floor: bool,
    pub decorative_thin: bool,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub path_idx: usize,
    /// Arc-fraction range of the segment this word fills exactly.
    pub t0: f32,
    pub t1: f32,
    pub word: String,
    pub glyph_size: f32,
    /// Justified advance per glyph along the arc (fills segment exactly).
    pub advance_px: f32,
    /// The baseline: verbatim sub-polyline of the pinned path.
    pub baseline: Vec<(f32, f32)>,
}

#[derive(Debug)]
pub enum TypesetError {
    /// (path a, path b): every legal word collided at floor across the
    /// re-seed budget — subject fails typesettability (route to F4).
    Untypesettable(usize, usize),
    /// A segment no pool word can legally fill (post-gate this should
    /// not happen; reported, never squeezed).
    NoLegalWord(usize),
}

pub struct Params {
    pub floor: f32,
    pub band_max: f32,
    /// Mean glyph advance as a fraction of size (script class).
    pub advance_frac: f32,
    /// D4 (pending Eric): justified spacing may stretch to at most this
    /// multiple of natural tracking; sparser segments must split/redraw.
    pub max_justify: f32,
}

fn arc_cum(points: &[(f32, f32)]) -> Vec<f32> {
    let mut cum = vec![0.0f32];
    for w in points.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        cum.push(cum.last().unwrap() + d);
    }
    cum
}

/// Verbatim sub-polyline for [t0,t1]: original interior points untouched;
/// endpoints interpolated ON the polyline (residual 0 by construction).
pub fn sub_polyline(points: &[(f32, f32)], cum: &[f32], t0: f32, t1: f32) -> Vec<(f32, f32)> {
    let total = *cum.last().unwrap_or(&0.0);
    let (a, b) = (t0 * total, t1 * total);
    let mut out = Vec::new();
    let point_at = |want: f32| -> (f32, f32) {
        for i in 1..points.len() {
            if cum[i] >= want {
                let seg = cum[i] - cum[i - 1];
                let f = if seg > 0.0 { (want - cum[i - 1]) / seg } else { 0.0 };
                return (
                    points[i - 1].0 + (points[i].0 - points[i - 1].0) * f,
                    points[i - 1].1 + (points[i].1 - points[i - 1].1) * f,
                );
            }
        }
        *points.last().unwrap()
    };
    out.push(point_at(a));
    for i in 0..points.len() {
        if cum[i] > a && cum[i] < b {
            out.push(points[i]);
        }
    }
    out.push(point_at(b));
    out
}

/// Glyph anchor boxes along a baseline for collision testing (word choice
/// and size are the ONLY remedies — geometry never moves).
fn glyph_boxes(pl: &Placement) -> Vec<(f32, f32, f32)> {
    // (x, y, half) sampled per glyph center along the baseline
    let cum = arc_cum(&pl.baseline);
    let total = *cum.last().unwrap_or(&0.0);
    let n = ((total / pl.advance_px).round() as usize).max(1);
    (0..n)
        .map(|k| {
            let want = (k as f32 + 0.5) * pl.advance_px;
            let mut pt = *pl.baseline.last().unwrap();
            for i in 1..pl.baseline.len() {
                if cum[i] >= want {
                    let seg = cum[i] - cum[i - 1];
                    let f = if seg > 0.0 { (want - cum[i - 1]) / seg } else { 0.0 };
                    pt = (
                        pl.baseline[i - 1].0 + (pl.baseline[i].0 - pl.baseline[i - 1].0) * f,
                        pl.baseline[i - 1].1 + (pl.baseline[i].1 - pl.baseline[i - 1].1) * f,
                    );
                    break;
                }
            }
            (pt.0, pt.1, pl.glyph_size * 0.55)
        })
        .collect()
}

fn collide(a: &Placement, b: &Placement) -> bool {
    if a.path_idx == b.path_idx {
        return false; // same pinned path: neighbors abut by construction
    }
    let (ba, bb) = (glyph_boxes(a), glyph_boxes(b));
    for &(ax, ay, ah) in &ba {
        for &(bx, by, bh) in &bb {
            let need = (ah + bh) * 0.8;
            if (ax - bx).hypot(ay - by) < need {
                return true;
            }
        }
    }
    false
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// F2 fit for one word on one segment. Returns glyph size + advance, or
/// None (INVALID — caller redraws). Never squeezes, never overflows.
fn fit(word_chars: usize, seg_len: f32, p: &Params) -> Option<(f32, f32)> {
    if word_chars == 0 {
        return None;
    }
    let n = word_chars as f32;
    // size such that the word's natural extent fills the segment
    let natural_size = seg_len / (n * p.advance_frac);
    let size = natural_size.clamp(p.floor, p.band_max);
    let natural_ext = n * p.advance_frac * size;
    if natural_ext > seg_len + 0.5 {
        return None; // would overflow past the endpoint at floor — INVALID
    }
    // justify: spacing stretches glyph advance to fill exactly
    let advance_px = seg_len / n;
    if advance_px > p.advance_frac * size * p.max_justify {
        return None; // sparser than the D4 cap — split/redraw instead
    }
    Some((size, advance_px))
}

/// Typeset a subject: one word per pre-marked segment of every
/// typesettable path. Deterministic in (seed). F3 ladder inside;
/// re-seeding is the caller's loop (cap 3 per spec).
pub fn typeset(
    paths: &[ScanPath],
    pool: &[(String, usize)], // (word, char_count)
    seed: u64,
    p: &Params,
) -> Result<Vec<Placement>, TypesetError> {
    let mut st = seed ^ 0x5343414e; // "SCAN"
    let mut placements: Vec<Placement> = Vec::new();
    let mut used: Vec<usize> = Vec::new();
    for (pi, path) in paths.iter().enumerate() {
        if path.sub_floor || path.decorative_thin {
            continue;
        }
        let cum = arc_cum(&path.points);
        let total = *cum.last().unwrap_or(&0.0);
        let mut bounds = vec![0.0f32];
        bounds.extend(path.segments.iter().copied().filter(|t| *t > 0.0 && *t < 1.0));
        bounds.push(1.0);
        bounds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bounds.dedup();
        for w2 in bounds.windows(2) {
            let (t0, t1) = (w2[0], w2[1]);
            let seg_len = (t1 - t0) * total;
            if seg_len < p.floor * 2.0 {
                continue; // sub-floor sliver between marks: D-A territory
            }
            let rot = (splitmix(&mut st) % pool.len().max(1) as u64) as usize;
            let mut placed = false;
            let mut last_err = TypesetError::NoLegalWord(pi);
            for k in 0..pool.len() {
                let idx = (rot + k) % pool.len();
                if used.contains(&idx) {
                    continue;
                }
                let (word, chars) = &pool[idx];
                let Some((size, adv)) = fit(*chars, seg_len, p) else { continue };
                let mut pl = Placement {
                    path_idx: pi,
                    t0,
                    t1,
                    word: word.clone(),
                    glyph_size: size,
                    advance_px: adv,
                    baseline: sub_polyline(&path.points, &cum, t0, t1),
                };
                // F3: shrink the colliding word(S) toward floor — the new
                // word first, then the placed neighbor (word size is a
                // legal remedy on both sides; geometry never is).
                let mut ok = !placements.iter().any(|o| collide(&pl, o));
                if !ok && size > p.floor {
                    pl.glyph_size = p.floor;
                    if pl.advance_px <= p.advance_frac * p.floor * p.max_justify {
                        ok = !placements.iter().any(|o| collide(&pl, o));
                    }
                    if !ok {
                        pl.glyph_size = size;
                    }
                }
                if !ok {
                    let colliders: Vec<usize> = placements
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| collide(&pl, o))
                        .map(|(k, _)| k)
                        .collect();
                    let shrinkable = colliders.iter().all(|&k| {
                        let o = &placements[k];
                        o.glyph_size > p.floor
                            && o.advance_px <= p.advance_frac * p.floor * p.max_justify
                    });
                    // F3 step 2 — REDRAW a colliding neighbor: swap its
                    // word for a longer one (smaller justified advance),
                    // which legalizes shrinking it to floor. Word choice
                    // is a legal remedy on both sides; geometry never is.
                    if !shrinkable && colliders.len() == 1 {
                        let k = colliders[0];
                        let o_seg = {
                            let o = &placements[k];
                            let cum = arc_cum(&o.baseline);
                            *cum.last().unwrap_or(&0.0)
                        };
                        let need_chars =
                            (o_seg / (p.advance_frac * p.floor * p.max_justify)).ceil() as usize;
                        let saved_word = placements[k].word.clone();
                        let saved_size = placements[k].glyph_size;
                        let saved_adv = placements[k].advance_px;
                        let mut swapped = false;
                        for (wi, (w2, c2)) in pool.iter().enumerate() {
                            if used.contains(&wi) || *c2 < need_chars {
                                continue;
                            }
                            if let Some((s2, a2)) = fit(*c2, o_seg, p) {
                                placements[k].word = w2.clone();
                                placements[k].glyph_size = s2.min(p.floor.max(s2)).min(s2);
                                placements[k].glyph_size = p.floor.max(p.floor); // floor it
                                placements[k].glyph_size = p.floor;
                                placements[k].advance_px = a2;
                                if a2 <= p.advance_frac * p.floor * p.max_justify {
                                    used.push(wi);
                                    swapped = true;
                                    break;
                                } else {
                                    placements[k].word = saved_word.clone();
                                    placements[k].glyph_size = saved_size;
                                    placements[k].advance_px = saved_adv;
                                }
                            }
                        }
                        if swapped {
                            pl.glyph_size = p.floor;
                            if pl.advance_px <= p.advance_frac * p.floor * p.max_justify
                                && !placements.iter().any(|o| collide(&pl, o))
                            {
                                ok = true;
                            } else {
                                pl.glyph_size = size;
                            }
                        }
                    }
                    if !ok && shrinkable && !colliders.is_empty() {
                        let saved: Vec<(usize, f32)> =
                            colliders.iter().map(|&k| (k, placements[k].glyph_size)).collect();
                        for &k in &colliders {
                            placements[k].glyph_size = p.floor;
                        }
                        pl.glyph_size = p.floor;
                        if pl.advance_px <= p.advance_frac * p.floor * p.max_justify
                            && !placements.iter().any(|o| collide(&pl, o))
                        {
                            ok = true;
                        } else {
                            for (k, v) in saved {
                                placements[k].glyph_size = v;
                            }
                            pl.glyph_size = size;
                        }
                    }
                }
                if ok {
                    used.push(idx);
                    placements.push(pl);
                    placed = true;
                    break;
                } else if let Some(o) = placements.iter().find(|o| collide(&pl, o)) {
                    last_err = TypesetError::Untypesettable(pi, o.path_idx);
                }
            }
            if !placed {
                return Err(last_err);
            }
        }
    }
    Ok(placements)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(len: f32) -> ScanPath {
        ScanPath {
            points: vec![(0.0, 0.0), (len / 2.0, 0.0), (len, 0.0)],
            segments: vec![],
            sub_floor: false,
            decorative_thin: false,
        }
    }

    #[test]
    fn baseline_is_the_path_verbatim() {
        let p = ScanPath {
            points: vec![(0.0, 0.0), (37.5, 12.2), (81.0, 9.9), (140.0, 44.0)],
            segments: vec![],
            sub_floor: false,
            decorative_thin: false,
        };
        let pool = vec![("hello".to_string(), 5)];
        let params = Params { floor: 13.0, band_max: 40.0, advance_frac: 0.54, max_justify: 2.0 };
        let out = typeset(&[p.clone()], &pool, 1, &params).unwrap();
        assert_eq!(out.len(), 1);
        // every interior point of the baseline is literally a scan point
        for pt in &out[0].baseline[1..out[0].baseline.len() - 1] {
            assert!(p.points.contains(pt), "baseline resampled the scan: {pt:?}");
        }
        // residual 0 by construction: endpoints lie on scan segments
        let cum = arc_cum(&p.points);
        let total = *cum.last().unwrap();
        assert!((arc_cum(&out[0].baseline).last().unwrap() - total).abs() < 0.01);
    }

    #[test]
    fn fit_never_squeezes_or_overflows() {
        let params = Params { floor: 13.0, band_max: 40.0, advance_frac: 0.54, max_justify: 2.0 };
        // 10 chars on a 50px segment: needs 70px at floor -> INVALID
        assert!(fit(10, 50.0, &params).is_none());
        // 3 chars on 50px: size = 50/(3*0.54)=30.8, fits exactly
        let (size, adv) = fit(3, 50.0, &params).unwrap();
        assert!((size - 30.86).abs() < 0.1);
        assert!((adv * 3.0 - 50.0).abs() < 0.01, "justified to fill exactly");
        // sparse: 2 chars on 200px at band max 40 -> advance 100 > 2x cap
        assert!(fit(2, 200.0, &params).is_none(), "D4 cap routes to split");
    }

    #[test]
    fn collision_resolves_in_words_never_geometry() {
        // two parallel strokes 12px apart: floor glyphs (13px) must collide
        let a = straight(120.0);
        let mut b = straight(120.0);
        for pt in b.points.iter_mut() {
            pt.1 = 12.0;
        }
        let pool: Vec<(String, usize)> =
            (0..8).map(|k| (format!("w{k}wordy"), 7)).collect();
        let params = Params { floor: 13.0, band_max: 24.0, advance_frac: 0.54, max_justify: 2.0 };
        let before = [a.clone(), b.clone()];
        let r = typeset(&before, &pool, 1, &params);
        assert!(matches!(r, Err(TypesetError::Untypesettable(_, _))));
        // geometry unchanged by the attempt (I2: read-only)
        assert_eq!(before[0].points, a.points);
        assert_eq!(before[1].points, b.points);
    }
}
