//! The TonalKit laws, ported with the implementation. Behavioural tests are
//! verbatim from TonalKitTests.swift, and passing them against the same
//! synthetic fixtures is the "verified against the reference" step that
//! authorises deleting the Swift original (module 6).

use super::*;

/// The Swift fixture used `sin` — which spike #9 disqualified from
/// anything determinism-adjacent. Same visual character (a ramp with
/// diagonal texture), built from an integer triangle wave instead.
fn ramp_with_texture(w: usize, h: usize) -> Gray {
    let mut px = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let ramp = x as f64 / (w - 1) as f64 * 255.0;
            let phase = ((x + y) * 45) % 128; // triangle wave, period ~2.8px scaled
            let tri = if phase < 64 { phase as f64 } else { (128 - phase) as f64 };
            let tex = (tri - 32.0) * 18.0 / 32.0;
            px[y * w + x] = (ramp + tex).clamp(0.0, 255.0) as u8;
        }
    }
    Gray::new(w, h, px)
}

#[test]
fn posterize_is_deterministic() {
    let g = ramp_with_texture(96, 64);
    let a = posterize(&g, 10);
    let b = posterize(&g, 10);
    assert_eq!(a.band, b.band, "same photo must yield the same bands");
    assert_eq!(a.bounds, b.bounds);
}

#[test]
fn posterize_uses_every_band() {
    // The failure this guards: equal-luminance cuts leave most bands empty
    // on a clustered photo, and the render loses its shading.
    let g = ramp_with_texture(96, 64);
    let m = posterize(&g, 10);
    let used: std::collections::HashSet<u8> = m.band.iter().copied().collect();
    assert_eq!(used.len(), 10, "every band carries pixels; got {}", used.len());
}

#[test]
fn posterize_is_monotonic_in_luminance() {
    let g = ramp_with_texture(96, 64);
    let m = posterize(&g, 8);
    for i in 0..g.px.len() {
        for j in 0..g.px.len() {
            if g.px[i] < g.px[j] {
                assert!(m.band[i] <= m.band[j], "a darker pixel can never land in a lighter band");
                break;
            }
        }
    }
}

#[test]
fn flow_follows_texture_direction() {
    // Vertical stripes -> edges run vertically -> flow angle ~pi/2.
    let (w, h) = (64usize, 64usize);
    let mut px = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            px[y * w + x] = if (x / 4) % 2 == 0 { 30 } else { 220 };
        }
    }
    let f = flow_field(&Gray::new(w, h, px), 3);
    let (mut sum, mut n) = (0.0f64, 0usize);
    for y in 8..h - 8 {
        for x in 8..w - 8 {
            if f.coherence[y * w + x] > 0.5 {
                sum += f.angle[y * w + x] as f64;
                n += 1;
            }
        }
    }
    assert!(n > 100, "stripes must produce coherent flow (got {n})");
    let mean = sum / n as f64;
    assert!((mean - std::f64::consts::PI / 2.0).abs() < 0.2, "flow runs along the stripes: {mean}");
}

#[test]
fn flow_is_deterministic() {
    let g = ramp_with_texture(96, 64);
    let a = flow_field(&g, 3);
    let b = flow_field(&g, 3);
    assert!(a.angle.iter().zip(&b.angle).all(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(a.coherence.iter().zip(&b.coherence).all(|(x, y)| x.to_bits() == y.to_bits()));
}

#[test]
fn palette_is_seed_deterministic_and_ordered() {
    let rgb: Vec<(u8, u8, u8)> =
        (0..5000).map(|i| ((i % 256) as u8, ((i * 7) % 256) as u8, ((i * 13) % 256) as u8)).collect();
    let a = median_cut(&rgb, 6, 42);
    let b = median_cut(&rgb, 6, 42);
    assert_eq!(a, b, "same seed, same palette");
    assert_eq!(a.len(), 6);
    assert!(a.windows(2).all(|w| w[0].luma() <= w[1].luma()), "palette is ordered dark -> light");
}

#[test]
fn contrast_floor_drops_invisible_swatches_but_never_all() {
    let on_page = Swatch { r: 250, g: 250, b: 250, population: 10 }; // invisible on white
    let ink = Swatch { r: 20, g: 20, b: 20, population: 10 };
    let kept = enforce_contrast_floor(&[on_page, ink], 1.0, 0.18);
    assert_eq!(kept, vec![ink], "a swatch the page swallows is not a colour we can spell in");
    let all_invisible = enforce_contrast_floor(&[on_page], 1.0, 0.18);
    assert_eq!(all_invisible.len(), 1, "never return an empty palette");
}

#[test]
fn every_backend_agrees() {
    let mut px = vec![0u8; 64 * 48];
    for y in 0..48 {
        for x in 0..64 {
            px[y * 64 + x] = ((x * 4 + y * 2) % 256) as u8;
        }
    }
    let g = Gray::new(64, 48, px);
    assert!(first_disagreement(&g, 10).is_none(), "acceleration is a perf path, never a correctness dependency");
}

#[test]
fn det_atan2_matches_libm_within_tolerance_without_being_libm() {
    // The polynomial must be numerically RIGHT (close to true atan2) while
    // being bit-stable in a way libm never promises. 1e-5 rad is orders of
    // magnitude below any band or stroke decision.
    for i in 0..=360 {
        let th = (i as f64) * std::f64::consts::PI / 180.0 - std::f64::consts::PI;
        let (y, x) = (th.sin() as f32 * 3.0, th.cos() as f32 * 3.0);
        let got = det_atan2(y, x);
        let want = (y as f64).atan2(x as f64) as f32;
        assert!((got - want).abs() < 1e-5, "deg {i}: {got} vs {want}");
    }
}

#[test]
fn downscale_is_exact_box_average() {
    let g = Gray::new(4, 2, vec![0, 4, 8, 12, 2, 6, 10, 14]);
    let d = g.downscale(2);
    assert_eq!((d.width, d.height), (2, 1));
    assert_eq!(d.px, vec![3, 11], "integer box average, floor division");
}

#[test]
fn rgb_to_gray_is_integer_rec601() {
    let g = Gray::from_rgb(2, 1, &[(255, 0, 0), (0, 255, 0)]);
    assert_eq!(g.px, vec![76, 149], "no float path in colour conversion");
}
