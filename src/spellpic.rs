//! CC-WORDPICTURE-SCANLOCK v8.2 — the runtime side.
//!
//! The offline pipeline (tools/) and this module plan through the SAME
//! pure crate, `scanlock`, against the SAME shipped scan data, so a
//! layout proven legal in CI is the layout the device draws (v8 F1's
//! whole point). Nothing here chooses geometry: it loads pinned paths,
//! asks scanlock for a plan, and refuses to display an illegal one.

use std::sync::OnceLock;

use scanlock::{CapacityParams, MicroStroke, Placement, ScanPath};

pub const FLOOR: f32 = 13.0;
pub const AVG_ADVANCE: f32 = 0.5257; // measured, tools/measure_advance.py
pub const GAP_CHARS: f32 = 1.0;

#[derive(serde::Deserialize)]
pub struct RawPath {
    pub p: Vec<[f32; 2]>,
    pub s: Vec<f32>,
    pub f: u8,
}

#[derive(serde::Deserialize)]
pub struct RawReq {
    pub name: String,
    pub near: [f32; 2],
    pub tol: f32,
}

#[derive(serde::Deserialize)]
pub struct RawSubject {
    pub tier: String,
    #[serde(default)]
    pub req: Vec<RawReq>,
    pub paths: Vec<RawPath>,
}

#[derive(serde::Deserialize)]
pub struct ScanBundle {
    pub subjects: std::collections::HashMap<String, RawSubject>,
}

pub fn bundle() -> &'static ScanBundle {
    static B: OnceLock<ScanBundle> = OnceLock::new();
    B.get_or_init(|| {
        serde_json::from_str(include_str!("../config/wordpic/scans.json"))
            .expect("scans.json parses")
    })
}

pub fn has(subject: &str) -> bool {
    bundle().subjects.contains_key(subject)
}

fn band_max(tier: &str) -> f32 {
    match tier {
        "easy" => 40.0,
        "medium" => 32.0,
        "hard" => 24.0,
        _ => 30.0,
    }
}

/// Script-class advance (the offline eval uses the same table).
fn advance(lang: &str) -> f32 {
    match lang {
        "ja" => 1.0,
        "ko" => 0.95,
        "ar" => 0.52,
        "hi" => 0.58,
        "ru" => 0.58,
        _ => AVG_ADVANCE,
    }
}

pub fn params(tier: &str, lang: &str) -> CapacityParams {
    CapacityParams {
        floor: FLOOR,
        band_max: band_max(tier),
        avg_advance: advance(lang),
        min_word_chars: 3.0,
        gap_chars: GAP_CHARS,
        line_height_ratio: 1.0,
        step: 0.5,
        junction_radius: 10.0,
        keepout_arc_ratio: 0.75,
        curve_size_ratio: 0.9,
        min_words_per_path: 2.0,
        decorative_max_fraction: 0.20,
        micro_max_fraction: 0.10,
    }
}

pub fn scan_paths(subject: &str) -> Vec<ScanPath> {
    let Some(s) = bundle().subjects.get(subject) else { return Vec::new() };
    s.paths
        .iter()
        .map(|r| ScanPath {
            points: r.p.iter().map(|a| (a[0], a[1])).collect(),
            segments: r.s.clone(),
            sub_floor: r.f & 1 != 0,
            decorative_thin: r.f & 2 != 0,
            micro: r.f & 4 != 0,
        })
        .collect()
}

/// Tier pool as scanlock wants it: (typed word, RENDER unit count). The
/// adjacent-tier borrow is carried by appending the easy pool (D4).
pub fn pool(lang: &str, tier: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for t in [tier, "easy"] {
        for w in crate::words::tier_for(lang, t) {
            let typed = w.split('|').next().unwrap_or(w).to_string();
            if typed.contains(' ') || seen.contains(&typed) {
                continue;
            }
            let n = crate::wordpic_layout::render_units(&typed) as usize;
            seen.push(typed.clone());
            out.push((typed, n));
        }
    }
    out
}

pub struct Plan {
    pub size: f32,
    pub placements: Vec<Placement>,
    pub micro: Vec<MicroStroke>,
    pub words: Vec<String>,
}

/// The device plan. Returns None when the subject cannot be typeset
/// legally — the caller must then display NOTHING for it (F5/I10: no
/// sparse or illegal frame ever reaches the screen).
pub fn plan(subject: &str, lang: &str, seed: u64) -> Option<Plan> {
    let s = bundle().subjects.get(subject)?;
    let paths = scan_paths(subject);
    if paths.is_empty() {
        return None;
    }
    let p = params(&s.tier, lang);
    let (size, placements, micro, coverage) =
        scanlock::plan_capacity(&paths, &pool(lang, &s.tier), seed, &p).ok()?;
    if placements.is_empty() {
        return None; // a plan with zero words is not a picture
    }
    // F5 coverage gate — the packer's own numbers, no off switch (I10).
    for c in &coverage {
        if micro.iter().any(|m| m.path_idx == c.path_idx) {
            continue;
        }
        if c.hostable > 0.0 && c.covered / c.hostable < 0.95 {
            return None;
        }
    }
    // required-micro gate: a named feature missing = nothing displays.
    for r in &s.req {
        let present = micro.iter().any(|m| {
            let n = m.points.len().max(1) as f32;
            let cx = m.points.iter().map(|p| p.0).sum::<f32>() / n;
            let cy = m.points.iter().map(|p| p.1).sum::<f32>() / n;
            (cx - r.near[0]).hypot(cy - r.near[1]) <= r.tol
        });
        if !present {
            return None;
        }
    }
    let words = placements.iter().map(|pl| pl.word.clone()).collect();
    Some(Plan { size, placements, micro, words })
}

/// Arc length of a polyline (shared by the renderer for textLength).
pub fn poly_len(pts: &[(f32, f32)]) -> f32 {
    pts.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_loads_and_every_subject_plans_in_english() {
        let b = bundle();
        assert!(b.subjects.len() >= 12, "twelve subjects shipped");
        for (name, _) in b.subjects.iter() {
            let p = plan(name, "en", 1);
            assert!(p.is_some(), "{name} must plan legally on device");
            let p = p.unwrap();
            assert!(p.size >= FLOOR);
            assert!(!p.words.is_empty());
        }
    }

    /// The device draws exactly what CI proved: baselines are verbatim
    /// sub-polylines of the shipped scan (residual 0 by construction).
    #[test]
    fn baselines_lie_on_the_pinned_scan() {
        let paths = scan_paths("dog");
        let p = plan("dog", "en", 1).unwrap();
        for pl in &p.placements {
            let src = &paths[pl.path_idx].points;
            for pt in pl.baseline.iter().skip(1).take(pl.baseline.len().saturating_sub(2)) {
                let on = src.windows(2).any(|w| {
                    let (a, b) = (w[0], w[1]);
                    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
                    let l2 = vx * vx + vy * vy;
                    let t = if l2 == 0.0 { 0.0 } else { (((pt.0 - a.0) * vx + (pt.1 - a.1) * vy) / l2).clamp(0.0, 1.0) };
                    (pt.0 - (a.0 + t * vx)).hypot(pt.1 - (a.1 + t * vy)) < 0.05
                });
                assert!(on, "baseline point off the pinned scan");
            }
        }
    }

    /// Determinism: same (subject, lang, seed) → same plan, so a resumed
    /// campaign redraws identically (I3 carried).
    /// Eric: no repeated words inside a picture. The packer never reuses
    /// a pool INDEX, so duplicates can only arrive from the pool itself
    /// (the tier list plus the borrowed easy list). Checked across every
    /// shipped subject and every language.
    #[test]
    fn no_repeated_words_within_a_picture() {
        for name in bundle().subjects.keys() {
            for (lang, _, _, _) in crate::consts::BUILTIN_LANGS.iter() {
                let Some(p) = plan(name, lang, 3) else { continue };
                let mut seen: Vec<&String> = Vec::new();
                for w in &p.words {
                    assert!(
                        !seen.contains(&w),
                        "{name}/{lang}: word {w:?} appears twice in one picture"
                    );
                    seen.push(w);
                }
            }
        }
    }

    #[test]
    fn plans_are_deterministic() {
        let a = plan("snowman", "en", 7).unwrap();
        let b = plan("snowman", "en", 7).unwrap();
        assert_eq!(a.words, b.words);
        assert_eq!(a.size, b.size);
    }
}

#[cfg(test)]
mod live_probe {
    /// Dev probe: print the first N planned words for a (subject, lang,
    /// seed) so a live browser run can be driven and verified.
    /// WP_SEED must be passed as the EXACT u64 — reading it through JS
    /// rounds past 2^53 and you will chase a plan that never existed.
    #[test]
    #[ignore]
    fn first_words() {
        let sub = std::env::var("SP_SUB").unwrap_or_else(|_| "dog".into());
        let lang = std::env::var("SP_LANG").unwrap_or_else(|_| "en".into());
        let seed: u64 = std::env::var("SP_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let p = super::plan(&sub, &lang, seed).expect("plans");
        eprintln!("SIZE {} WORDS {:?}", p.size, &p.words[..8.min(p.words.len())]);
    }
}
