//! CC-SCAN-STACK module 2 — TonalKit, in Rust.
//!
//! Port of `ios/ScanStack/Sources/TonalKit/TonalKit.swift`, which was the
//! reference implementation and is deleted with this landing (module 6's
//! one-implementation rule: ported, verified behaviourally, removed — never
//! wrapped alongside).
//!
//! Deterministic tonal & flow math: photo + seed in, identical output out,
//! every run, every thread count, every architecture. Two disciplines carry
//! that promise:
//!
//! * **Standard ops go through kornia** (pinned by spike #9); everything
//!   that IS the determinism guarantee — posterize cuts, the palette
//!   sampler, orientation math — is hand-written here.
//! * **libm never enters the pipeline.** Spike #9's only red run traced to
//!   `f32::sin` in a corpus generator: the system math library differs per
//!   architecture. So orientation uses [`det_atan2`], a fixed polynomial
//!   built from IEEE-exact add/mul/div only. (`sqrt` stays: IEEE 754
//!   requires correct rounding for it, so it is bit-stable everywhere.)
//!
//! Two deliberate departures from the Swift reference, both improvements
//! and both visible in goldens rather than smuggled:
//!
//! * Sobel comes from kornia's `spatial_gradient_float_parallel_row` with
//!   its replicate border, then ×8 (exact: power of two) to undo its
//!   normalised kernel. The Swift loop left a zero border; replicate is
//!   simply better behaviour at the edge of the photo.
//! * `resize` is an integer box-average — exact by construction. Kornia's
//!   interpolating resize was NOT qualified by the spike, and an
//!   unqualified float path has no business in this crate.

use kornia_image::{Image, ImageSize};
use kornia_imgproc::filter::spatial_gradient_float_parallel_row;

pub const PI: f32 = std::f32::consts::PI;

// ---------------------------------------------------------------- gray

#[derive(Clone, Debug)]
pub struct Gray {
    pub width: usize,
    pub height: usize,
    /// Row-major luminance, 0..=255.
    pub px: Vec<u8>,
}

impl Gray {
    pub fn new(width: usize, height: usize, px: Vec<u8>) -> Self {
        assert_eq!(px.len(), width * height, "px must be width*height");
        Self { width, height, px }
    }

    #[inline]
    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.px[y * self.width + x]
    }

    /// Rec.601 luma with integer weights — colour conversion with no float
    /// path at all, so it cannot drift.
    pub fn from_rgb(width: usize, height: usize, rgb: &[(u8, u8, u8)]) -> Self {
        assert_eq!(rgb.len(), width * height);
        let px = rgb
            .iter()
            .map(|&(r, g, b)| ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8)
            .collect();
        Self { width, height, px }
    }

    /// Integer box-average downscale by whole factors. Exact: sums of u8 in
    /// u32, one integer division. See the module note for why this is not
    /// kornia's interpolating resize.
    pub fn downscale(&self, factor: usize) -> Gray {
        assert!(factor >= 1);
        let (w, h) = (self.width / factor, self.height / factor);
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0u32;
                for oy in 0..factor {
                    for ox in 0..factor {
                        sum += self.at(x * factor + ox, y * factor + oy) as u32;
                    }
                }
                px.push((sum / (factor * factor) as u32) as u8);
            }
        }
        Gray { width: w, height: h, px }
    }
}

// ------------------------------------------------------- posterization

#[derive(Clone, Debug)]
pub struct BandMap {
    pub width: usize,
    pub height: usize,
    /// Band index per pixel, 0 = darkest.
    pub band: Vec<u8>,
    pub count: usize,
    /// Upper luminance bound of each band (inclusive), ascending.
    pub bounds: Vec<u8>,
}

/// Posterize into `count` bands by EQUAL-POPULATION quantiles, not equal
/// luminance: a photo whose tones cluster (most portraits) otherwise
/// collapses into two used bands and eight empty ones. Deterministic — the
/// histogram fully determines the cuts. Integer throughout.
pub fn posterize(g: &Gray, count: usize) -> BandMap {
    assert!((2..=16).contains(&count), "band count out of range");
    let mut hist = [0usize; 256];
    for &v in &g.px {
        hist[v as usize] += 1;
    }
    let total = g.px.len();
    let mut bounds: Vec<u8> = Vec::new();
    let (mut acc, mut next) = (0usize, 1usize);
    for v in 0..256usize {
        acc += hist[v];
        while next < count && acc * count >= next * total {
            bounds.push(v as u8);
            next += 1;
        }
    }
    while bounds.len() < count {
        bounds.push(255);
    }
    let band = g
        .px
        .iter()
        .map(|&v| {
            let mut b = 0usize;
            while b < count - 1 && v > bounds[b] {
                b += 1;
            }
            b as u8
        })
        .collect();
    BandMap { width: g.width, height: g.height, band, count, bounds }
}

// ------------------------------------------------- deterministic atan2

/// atan2 from IEEE-exact arithmetic only (add/mul/div), no libm. Minimax
/// polynomial for atan on [0,1] (max error ≈ 1e-7 rad — far below any
/// band or stroke decision), plus the standard octant reduction. The same
/// bits on every architecture, which `f32::atan2` does not promise and
/// spike #9 showed `sin` does not deliver.
pub fn det_atan2(y: f32, x: f32) -> f32 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let (ay, ax) = (y.abs(), x.abs());
    // t in [0,1]; both branches divide the smaller by the larger.
    let swap = ay > ax;
    let t = if swap { ax / ay } else { ay / ax };
    // atan(t) ≈ t*(c0 + t²*(c1 + t²*(c2 + t²*c3))): odd minimax on [0,1].
    let s = t * t;
    let p = t * (0.99997726 + s * (-0.33262347 + s * (0.19354346 + s * (-0.11643287 + s * (0.05265332 + s * -0.01172120)))));
    let a = if swap { PI / 2.0 - p } else { p };
    let a = if x < 0.0 { PI - a } else { a };
    if y < 0.0 {
        -a
    } else {
        a
    }
}

// ------------------------------------------------------ flow field

#[derive(Clone, Debug)]
pub struct FlowField {
    pub width: usize,
    pub height: usize,
    /// Dominant orientation per pixel in radians, 0..<pi (undirected).
    pub angle: Vec<f32>,
    /// Anisotropy 0..=1 — how strongly that direction dominates.
    pub coherence: Vec<f32>,
}

/// Sobel gradients (kornia) → structure tensor → principal direction. The
/// stroke path follows the texture, which is what makes fur read as fur
/// rather than as hatching.
pub fn flow_field(g: &Gray, window: usize) -> FlowField {
    let (w, h) = (g.width, g.height);
    let img = Image::<f32, 1>::new(
        ImageSize { width: w, height: h },
        g.px.iter().map(|&v| v as f32).collect(),
    )
    .expect("gray dims");
    let mut dx = Image::from_size_val(img.size(), 0.0f32).unwrap();
    let mut dy = Image::from_size_val(img.size(), 0.0f32).unwrap();
    spatial_gradient_float_parallel_row(&img, &mut dx, &mut dy).expect("gradient");
    // kornia's kernel is the ×1/8 normalised Sobel; ×8 (a power of two,
    // exact in IEEE) restores the reference scale.
    let gx: Vec<f32> = dx.as_slice().iter().map(|v| v * 8.0).collect();
    let gy: Vec<f32> = dy.as_slice().iter().map(|v| v * 8.0).collect();

    let mut angle = vec![0.0f32; w * h];
    let mut coherence = vec![0.0f32; w * h];
    let r = (window / 2).max(1) as isize;
    for y in 0..h as isize {
        for x in 0..w as isize {
            let (mut jxx, mut jyy, mut jxy) = (0.0f32, 0.0f32, 0.0f32);
            for oy in -r..=r {
                for ox in -r..=r {
                    let (nx, ny) = (x + ox, y + oy);
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        continue;
                    }
                    let i = (ny * w as isize + nx) as usize;
                    let (a, b) = (gx[i], gy[i]);
                    jxx += a * a;
                    jyy += b * b;
                    jxy += a * b;
                }
            }
            let diff = jxx - jyy;
            let theta = 0.5 * det_atan2(2.0 * jxy, diff);
            // gradient direction → edge direction is perpendicular
            let mut e = theta + PI / 2.0;
            while e < 0.0 {
                e += PI;
            }
            while e >= PI {
                e -= PI;
            }
            let i = (y * w as isize + x) as usize;
            angle[i] = e;
            let trace = jxx + jyy;
            // sqrt is correctly rounded by IEEE 754 — bit-stable, unlike libm.
            let disc = (diff * diff + 4.0 * jxy * jxy).sqrt();
            coherence[i] = if trace > 0.0 { disc / trace } else { 0.0 };
        }
    }
    FlowField { width: w, height: h, angle, coherence }
}

// ------------------------------------------------- seeded palette

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub population: usize,
}

impl Swatch {
    /// Relative luminance 0..=1 (Rec. 709).
    pub fn luma(&self) -> f64 {
        (0.2126 * self.r as f64 + 0.7152 * self.g as f64 + 0.0722 * self.b as f64) / 255.0
    }
}

/// Median-cut over sampled pixels. `seed` fixes the sampling stride offset
/// so the same photo + seed always yields the same palette; the cut itself
/// is deterministic given the sample. Hand-rolled: this sampler IS the
/// determinism guarantee (section A's own words).
pub fn median_cut(rgb: &[(u8, u8, u8)], count: usize, seed: u64) -> Vec<Swatch> {
    assert!(count > 0);
    if rgb.is_empty() {
        return Vec::new();
    }
    // splitmix64, same constants as the Swift reference.
    let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let stride = (rgb.len() / 20_000).max(1);
    let offset = (next() % stride as u64) as usize;
    let sample: Vec<(u8, u8, u8)> = rgb.iter().skip(offset).step_by(stride).copied().collect();
    let mut boxes: Vec<Vec<(u8, u8, u8)>> = vec![sample];
    while boxes.len() < count {
        // Largest box splits. On a tie Swift's max(by:) semantics were
        // unspecified; here it is explicitly the FIRST largest, so the
        // choice is part of the contract instead of an accident.
        let Some(i) = (0..boxes.len()).max_by_key(|&i| (boxes[i].len(), std::cmp::Reverse(i))) else {
            break;
        };
        if boxes[i].len() <= 1 {
            break;
        }
        let b = boxes.remove(i);
        let spread = |f: fn(&(u8, u8, u8)) -> u8| {
            let (mut lo, mut hi) = (255u8, 0u8);
            for p in &b {
                lo = lo.min(f(p));
                hi = hi.max(f(p));
            }
            hi as i32 - lo as i32
        };
        let spreads = [spread(|p| p.0), spread(|p| p.1), spread(|p| p.2)];
        let axis = (0..3).max_by_key(|&i| (spreads[i], std::cmp::Reverse(i))).unwrap();
        let mut sorted = b;
        // Stable sort: equal keys keep sample order, so the split point is
        // fully determined by the data.
        sorted.sort_by_key(|p| match axis {
            0 => p.0,
            1 => p.1,
            _ => p.2,
        });
        let mid = sorted.len() / 2;
        let hi = sorted.split_off(mid);
        boxes.push(sorted);
        boxes.push(hi);
    }
    let mut out: Vec<Swatch> = boxes
        .into_iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len();
            let sum = b.iter().fold((0usize, 0usize, 0usize), |a, p| {
                (a.0 + p.0 as usize, a.1 + p.1 as usize, a.2 + p.2 as usize)
            });
            Swatch { r: (sum.0 / n) as u8, g: (sum.1 / n) as u8, b: (sum.2 / n) as u8, population: n }
        })
        .collect();
    out.sort_by(|a, b| a.luma().partial_cmp(&b.luma()).unwrap());
    out
}

/// The contrast floor: drop swatches that cannot be told apart from the
/// page. Returns the kept swatches, never fewer than one.
pub fn enforce_contrast_floor(swatches: &[Swatch], background: f64, min_delta: f64) -> Vec<Swatch> {
    let kept: Vec<Swatch> = swatches
        .iter()
        .filter(|s| (s.luma() - background).abs() >= min_delta)
        .copied()
        .collect();
    if kept.is_empty() {
        return swatches
            .iter()
            .max_by(|a, b| {
                (a.luma() - background)
                    .abs()
                    .partial_cmp(&(b.luma() - background).abs())
                    .unwrap()
            })
            .map(|s| vec![*s])
            .unwrap_or_default();
    }
    kept
}

// ------------------------------------------- module 5: the backend seam

/// Module 5's contract, ported with the crate: acceleration is a
/// performance path, never a correctness dependency. Any future backend
/// (Metal was re-scoped to budget-contingent by section A) must agree with
/// the CPU reference exactly.
pub trait TonalBackend {
    fn name(&self) -> &'static str;
    fn posterize(&self, g: &Gray, count: usize) -> BandMap;
    fn flow(&self, g: &Gray, window: usize) -> FlowField;
}

pub struct CpuBackend;

impl TonalBackend for CpuBackend {
    fn name(&self) -> &'static str {
        "cpu"
    }
    fn posterize(&self, g: &Gray, count: usize) -> BandMap {
        posterize(g, count)
    }
    fn flow(&self, g: &Gray, window: usize) -> FlowField {
        flow_field(g, window)
    }
}

/// First disagreement between the registered backends, or None. Runs today
/// against the single CPU backend so the harness cannot rot before any
/// accelerated backend exists.
pub fn first_disagreement(g: &Gray, bands: usize) -> Option<String> {
    let backends: Vec<Box<dyn TonalBackend>> = vec![Box::new(CpuBackend)];
    let reference = &backends[0];
    let ref_bands = reference.posterize(g, bands);
    let ref_flow = reference.flow(g, 3);
    for b in backends.iter().skip(1) {
        if b.posterize(g, bands).band != ref_bands.band {
            return Some(format!("{} disagrees with {} on bands", b.name(), reference.name()));
        }
        let f = b.flow(g, 3);
        if f.angle.iter().zip(&ref_flow.angle).any(|(a, r)| a.to_bits() != r.to_bits()) {
            return Some(format!("{} disagrees with {} on flow", b.name(), reference.name()));
        }
    }
    None
}

#[cfg(test)]
mod tests;
