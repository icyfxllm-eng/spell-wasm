//! CC-SCAN-STACK v1.1 #9 — does kornia-rs earn its place?
//!
//! Criteria, verbatim from the file: implement posterize + Sobel + structure
//! tensor via kornia-rs; run the 50-photo corpus on x86 and ARM; assert
//! byte-identical bands and flow across arch, runs, and thread counts; pin
//! the crate; record unsupported ops.
//!
//! Division of labour mirrors section A: posterize (equal-population bands)
//! is OUR algorithm and stays hand-written -- hand-rolled IS the determinism
//! guarantee -- while the STANDARD ops (Sobel gradients) go through kornia.
//! The structure tensor is composed from kornia's gradients by hand, and
//! that composition is recorded as a finding, not hidden.
//!
//! Output: one JSON line per image (band hash, flow hash), then a corpus
//! hash. Byte-identical = equal corpus hash, where "bytes" means the f32 BIT
//! PATTERNS of the flow field -- an epsilon comparison would be conceding
//! nondeterminism while pretending not to.

use std::fmt::Write as _;

use kornia_image::{Image, ImageSize};
use kornia_imgproc::filter::spatial_gradient_float_parallel_row;

const BANDS: usize = 10;

/// FNV-1a over raw bytes. A tiny, dependency-free, stable hash.
fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Equal-population posterize — the TonalKit algorithm, ported verbatim from
/// the Swift reference implementation. Ours, so hand-written (section A).
fn posterize(px: &[f32], bands: usize) -> Vec<u8> {
    let mut sorted: Vec<f32> = px.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    let cuts: Vec<f32> = (1..bands).map(|k| sorted[k * n / bands]).collect();
    px.iter()
        .map(|v| cuts.iter().take_while(|c| v >= c).count() as u8)
        .collect()
}

/// Sobel gradients via kornia, structure tensor + flow angle composed from
/// them by hand (kornia has no structure-tensor op at this version — a
/// recorded gap, not a silent one).
/// Split diagnostics: hash kornia's raw gradients separately from the flow
/// angles composed on top of them. If the arches disagree, this says WHOSE
/// bytes moved -- kornia's convolution, or our atan2, which goes to the
/// system libm and is a known per-arch divergence. Striking section A on
/// libm's behaviour would be blaming the crate for the OS.
fn flow_with_grad_hash(img: &Image<f32, 1>, window: usize) -> (Vec<f32>, u64) {
    let size = img.size();
    let mut dx = Image::from_size_val(size, 0.0f32).unwrap();
    let mut dy = Image::from_size_val(size, 0.0f32).unwrap();
    spatial_gradient_float_parallel_row(img, &mut dx, &mut dy).unwrap();
    let (w, h) = (size.width, size.height);
    let (gx, gy) = (dx.as_slice(), dy.as_slice());
    let gh = fnv(gx.iter().chain(gy.iter()).flat_map(|f| f.to_bits().to_le_bytes()));
    let mut out = vec![0.0f32; w * h];
    let r = window as isize;
    for y in 0..h as isize {
        for x in 0..w as isize {
            let (mut jxx, mut jxy, mut jyy) = (0.0f64, 0.0f64, 0.0f64);
            for oy in -r..=r {
                for ox in -r..=r {
                    let (sx, sy) = ((x + ox).clamp(0, w as isize - 1), (y + oy).clamp(0, h as isize - 1));
                    let i = (sy * w as isize + sx) as usize;
                    let (a, b) = (gx[i] as f64, gy[i] as f64);
                    jxx += a * a;
                    jxy += a * b;
                    jyy += b * b;
                }
            }
            // dominant orientation of the local tensor; the flow runs along it
            out[(y * w as isize + x) as usize] = (0.5 * (2.0 * jxy).atan2(jxx - jyy)) as f32;
        }
    }
    (out, gh)
}

fn load_corpus() -> Vec<(String, usize, usize, Vec<f32>)> {
    let mut out = Vec::new();
    // Real references first (PNG only; decode is part of what is under test).
    let ref_dir = std::path::Path::new("../../content-pipeline/wordpic/ref");
    let mut names: Vec<_> = std::fs::read_dir(ref_dir)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|x| x == "png").unwrap_or(false)).collect())
        .unwrap_or_default();
    names.sort();
    for p in names {
        let dec = png::Decoder::new(std::fs::File::open(&p).unwrap());
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let (w, h) = (info.width as usize, info.height as usize);
        let step = buf.len() / (w * h);
        let px: Vec<f32> = (0..w * h)
            .map(|i| {
                let o = i * step;
                // Rec.601 luma, integer weights: no float path in the decode.
                let (r, g, b) = (buf[o] as u32, buf[o + step.min(2) - 1] as u32, buf[o + step - 1] as u32);
                ((r * 299 + g * 587 + b * 114) / 1000) as f32 / 255.0
            })
            .collect();
        out.push((p.file_stem().unwrap().to_string_lossy().into_owned(), w, h, px));
    }
    // Seeded synthetic to 50: gradients, rings, and xorshift noise. Chosen
    // BECAUSE they are adversarial for float determinism (long runs of equal
    // values, high-frequency noise), and honestly labelled synthetic.
    let mut seed: u64 = 0x5EED_CAFE;
    let mut rng = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let n_real = out.len();
    for k in 0..(50 - n_real) {
        let (w, h) = (256usize, 256usize);
        let px: Vec<f32> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f32, (i / w) as f32);
                match k % 3 {
                    0 => (x + y * 0.5) % 256.0 / 255.0,
                    // Rings via an INTEGER triangle wave of r^2. The first
                    // version used f32::sin -- libm, which differs per arch --
                    // and every cross-arch divergence in the spike's first
                    // run traced to exactly those images. A determinism
                    // corpus whose generator is itself nondeterministic
                    // frames whatever it tests.
                    1 => {
                        let (ix, iy) = ((x as i64) - 128, (y as i64) - 128);
                        let r2 = (ix * ix + iy * iy) as u32;
                        let tri = (r2 % 1024).min(1024 - r2 % 1024);
                        tri as f32 / 512.0
                    }
                    _ => (rng() & 0xFF) as f32 / 255.0,
                }
            })
            .collect();
        out.push((format!("synthetic-{k:02}"), w, h, px));
    }
    out
}

fn main() {
    let corpus = load_corpus();
    let mut report = String::new();
    let mut corpus_hash: u64 = 0xcbf29ce484222325;
    let mut grad_hash: u64 = 0xcbf29ce484222325;
    let mut band_hash: u64 = 0xcbf29ce484222325;
    for (name, w, h, px) in &corpus {
        let img = Image::<f32, 1>::new(ImageSize { width: *w, height: *h }, px.clone()).unwrap();
        let bands = posterize(px, BANDS);
        let (fl, gh) = flow_with_grad_hash(&img, 3);
        let bh = fnv(bands.iter().copied());
        let fh = fnv(fl.iter().flat_map(|f| f.to_bits().to_le_bytes()));
        corpus_hash ^= bh.rotate_left(17) ^ fh;
        grad_hash ^= gh.rotate_left(29);
        band_hash ^= bh.rotate_left(17);
        let _ = writeln!(report, "{{\"img\":\"{name}\",\"bands\":\"{bh:016x}\",\"grad\":\"{gh:016x}\",\"flow\":\"{fh:016x}\"}}");
    }
    print!("{report}");
    println!(
        "{{\"corpus\":\"{corpus_hash:016x}\",\"grads\":\"{grad_hash:016x}\",\"bands\":\"{band_hash:016x}\",\"arch\":\"{}\",\"n\":{}}}",
        std::env::consts::ARCH,
        corpus.len()
    );
}
