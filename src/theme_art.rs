//! CC-THEMES-COSMOS F3 / F3a: the earned themes' art, drawn in code (T-D14:
//! SVG only, no photos, no Lottie or Rive). Every colour comes from the `art.*`
//! tokens (src/tokens.rs), never a literal here.
//!
//! Two pictures per look:
//! - the page art, one SVG the size of the viewport, set once as the background
//!   of the fixed #themeArt layer. The browser rasterises it once per size; it
//!   is never redrawn during a word (I-T13). Diamond's pavé field is this
//!   layer, with the scrims drawn above it by CSS (T-D22).
//! - the orb's face (--orb-art): a moon phase, a planet, an eclipsed moon or a
//!   brilliant-cut stone, drawn over the gem.
//!
//! The SVG builders are pure and seeded, so a look always draws the same
//! picture (result cards and snapshots are deterministic, AT-T17).

use std::fmt::Write;

use crate::astro::Band;
use crate::themes::Theme;
use crate::themes_cosmos::{Look, Planet};
use crate::tokens::*;

const SYNODIC: f64 = 29.530_588_853;

/// A tiny deterministic generator (xorshift), so the art needs no dependency
/// and draws the same stars every time.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        f64::from(x) / f64::from(u32::MAX)
    }
}

fn svg_open(w: f64, h: f64) -> String {
    format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w:.0}' height='{h:.0}' viewBox='0 0 {w:.0} {h:.0}' preserveAspectRatio='none'>")
}

/// One brilliant-cut round stone of radius 1, centred on the origin: girdle,
/// octagonal table, star and bezel facets, a few bright and dark facets for fire.
fn stone_symbol(id: &str) -> String {
    use std::f64::consts::PI;
    let pt = |r: f64, a: f64| (r * a.cos(), r * a.sin());
    let mut s = format!("<symbol id='{id}' viewBox='-1 -1 2 2' overflow='visible'>");
    let _ = write!(s, "<circle r='1' fill='{}' stroke='{}' stroke-width='.05'/>", ART_STONE_DEEP.value, ART_STONE_GIRDLE.value);
    let shades = [ART_STONE_BRIGHT.value, ART_STONE_MID.value, ART_STONE_DARK.value, ART_STONE_BRIGHT.value, ART_STONE_MID.value, ART_STONE_FIRE_1.value, ART_STONE_DEEP.value, ART_STONE_FIRE_2.value];
    for i in 0..8 {
        let a0 = f64::from(i) * PI / 4.0;
        let a1 = a0 + PI / 4.0;
        let (t0, t1) = (pt(0.52, a0), pt(0.52, a1));
        let c = pt(1.0, a0 + PI / 8.0);
        let (b0, b1) = (pt(1.0, a0), pt(1.0, a1));
        // Star facet (table edge to the girdle), then the two upper-girdle halves.
        let _ = write!(s, "<path d='M{:.3} {:.3}L{:.3} {:.3}L{:.3} {:.3}Z' fill='{}'/>", t0.0, t0.1, t1.0, t1.1, c.0, c.1, shades[i as usize]);
        let _ = write!(s, "<path d='M{:.3} {:.3}L{:.3} {:.3}L{:.3} {:.3}Z' fill='{}' opacity='.8'/>", t0.0, t0.1, b0.0, b0.1, c.0, c.1, shades[(i as usize + 3) % 8]);
        let _ = write!(s, "<path d='M{:.3} {:.3}L{:.3} {:.3}L{:.3} {:.3}Z' fill='{}' opacity='.8'/>", t1.0, t1.1, b1.0, b1.1, c.0, c.1, shades[(i as usize + 5) % 8]);
    }
    let table: Vec<String> = (0..8).map(|i| { let p = pt(0.52, f64::from(i) * PI / 4.0); format!("{:.3} {:.3}", p.0, p.1) }).collect();
    let _ = write!(s, "<path d='M{}Z' fill='{}'/>", table.join("L"), ART_STONE_TABLE.value);
    // A dark kite across the table gives the stone its depth.
    let _ = write!(s, "<path d='M0 -.52L.2 0L0 .52L-.2 0Z' fill='{}' opacity='.35'/>", ART_STONE_MID.value);
    s.push_str("</symbol>");
    s
}

/// F3a: two packed layers of stones edge to edge on near-black. The back layer
/// is small and dim for depth; the front layer is larger and turned so rows
/// do not line up.
fn pave(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    s.push_str("<defs>");
    s.push_str(&stone_symbol("s"));
    s.push_str("</defs>");
    let mut rng = Rng(0x00d1_a3f5);
    let mut layer = |s: &mut String, r: f64, angle: f64, opacity: f64| {
        let dx = 2.0 * r * 0.97;
        let dy = dx * 0.866;
        let reach = (w * w + h * h).sqrt() / 2.0 + 2.0 * r;
        let (cx, cy) = (w / 2.0, h / 2.0);
        let _ = write!(s, "<g opacity='{opacity}' transform='rotate({angle} {cx:.0} {cy:.0})'>");
        let rows = (reach / dy).ceil() as i32;
        let cols = (reach / dx).ceil() as i32;
        for row in -rows..=rows {
            for col in -cols..=cols {
                let x = cx + f64::from(col) * dx + if row % 2 == 0 { 0.0 } else { dx / 2.0 };
                let y = cy + f64::from(row) * dy;
                // Rotated back into screen space: skip the stones the screen never shows.
                let (ox, oy) = (x - cx, y - cy);
                let (ca, sa) = (angle.to_radians().cos(), angle.to_radians().sin());
                let (sx, sy) = (cx + ox * ca - oy * sa, cy + ox * sa + oy * ca);
                if sx < -r || sx > w + r || sy < -r || sy > h + r {
                    continue;
                }
                let spin = (rng.next() * 45.0).round();
                let _ = write!(s, "<use href='#s' transform='translate({x:.1} {y:.1}) rotate({spin}) scale({r:.1})'/>");
            }
        }
        s.push_str("</g>");
    };
    layer(&mut s, 12.0, 0.0, 0.45);
    layer(&mut s, 23.0, 17.0, 1.0);
    s.push_str("</svg>");
    s
}

/// Brushed grain for the metals: fine hairlines, a few bright, a few dark.
/// The sheen itself is the page ground (--page-image).
fn brushed(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    let mut rng = Rng(0x0b2a_5e11);
    let mut y = 0.0;
    while y < h {
        let bright = rng.next() > 0.45;
        let o = 0.25 + rng.next() * 0.75;
        let c = if bright { ART_GRAIN_LIGHT.value } else { ART_GRAIN_DARK.value };
        let _ = write!(s, "<rect x='0' y='{y:.1}' width='{w:.0}' height='.7' fill='{c}' opacity='{o:.2}'/>");
        y += 1.4 + rng.next() * 2.6;
    }
    s.push_str("</svg>");
    s
}

fn stars(s: &mut String, w: f64, h: f64, n: u32, top_frac: f64, seed: u32) {
    let mut rng = Rng(seed);
    for _ in 0..n {
        let x = rng.next() * w;
        let y = rng.next() * h * top_frac;
        let r = 0.5 + rng.next() * 0.9;
        let c = if rng.next() > 0.7 { ART_STAR.value } else { ART_STAR_DIM.value };
        let _ = write!(s, "<circle cx='{x:.1}' cy='{y:.1}' r='{r:.2}' fill='{c}'/>");
    }
}

fn glow_at_bottom(w: f64, h: f64, colour: &str) -> String {
    let mut s = svg_open(w, h);
    let _ = write!(
        s,
        "<defs><radialGradient id='g' cx='50%' cy='100%' r='75%'><stop offset='0' stop-color='{colour}'/><stop offset='1' stop-color='{colour}' stop-opacity='0'/></radialGradient></defs><rect width='{w:.0}' height='{h:.0}' fill='url(#g)'/>"
    );
    s
}

fn sky(band: Band, w: f64, h: f64) -> String {
    match band {
        Band::Sunrise => {
            let mut s = glow_at_bottom(w, h, ART_SUN_GLOW.value);
            stars(&mut s, w, h, 14, 0.25, 0x5a11_0001);
            s.push_str("</svg>");
            s
        }
        Band::Sunset => {
            let mut s = glow_at_bottom(w, h, ART_DUSK_GLOW.value);
            s.push_str("</svg>");
            s
        }
        Band::Day => {
            let mut s = svg_open(w, h);
            s.push_str("<defs><filter id='b' x='-20%' y='-50%' width='140%' height='200%'><feGaussianBlur stdDeviation='9'/></filter></defs><g filter='url(#b)'>");
            let mut rng = Rng(0x0c10_0d5);
            for _ in 0..7 {
                let cx = rng.next() * w;
                let cy = h * (0.16 + rng.next() * 0.12);
                let rx = w * (0.12 + rng.next() * 0.14);
                let _ = write!(s, "<ellipse cx='{cx:.0}' cy='{cy:.0}' rx='{rx:.0}' ry='{:.0}' fill='{}'/>", rx * 0.32, ART_CLOUD.value);
            }
            s.push_str("</g></svg>");
            s
        }
        Band::Night => {
            let mut s = svg_open(w, h);
            stars(&mut s, w, h, 60, 0.85, 0x0517_a125);
            s.push_str("</svg>");
            s
        }
    }
}

fn beach(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    let sea = h * 0.12;
    let _ = write!(
        s,
        "<defs><linearGradient id='sea' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient></defs>",
        ART_SEA_DEEP.value, ART_SEA.value
    );
    let mut d = format!("M0 0H{w:.0}V{sea:.0}");
    let n = 8;
    for i in (0..n).rev() {
        let x0 = w * f64::from(i + 1) / f64::from(n);
        let x1 = w * f64::from(i) / f64::from(n);
        let _ = write!(d, "Q{:.0} {:.0} {x1:.0} {sea:.0}", (x0 + x1) / 2.0, sea + if i % 2 == 0 { 7.0 } else { -5.0 });
    }
    d.push('Z');
    let _ = write!(s, "<path d='{d}' fill='url(#sea)'/>");
    // The foam line traces the water's edge.
    let foam = d.split_once(&format!("V{sea:.0}")).map(|(_, rest)| format!("M{w:.0} {sea:.0}{}", rest.trim_end_matches('Z'))).unwrap_or_default();
    let _ = write!(s, "<path d='{foam}' fill='none' stroke='{}' stroke-width='3' stroke-linecap='round' opacity='.9'/>", ART_FOAM.value);
    let mut rng = Rng(0x5a0d_0042);
    for _ in 0..90 {
        let _ = write!(s, "<circle cx='{:.0}' cy='{:.0}' r='.9' fill='{}'/>", rng.next() * w, sea + 20.0 + rng.next() * (h - sea - 20.0), ART_SAND_FLECK.value);
    }
    s.push_str("</svg>");
    s
}

/// A five-petal phlox flower.
fn phlox(s: &mut String, x: f64, y: f64, r: f64, petal: &str) {
    for i in 0..5 {
        let a = f64::from(i) * 72.0;
        let _ = write!(s, "<ellipse cx='{x:.1}' cy='{:.1}' rx='{:.1}' ry='{:.1}' fill='{petal}' transform='rotate({a} {x:.1} {y:.1})'/>", y - r * 0.55, r * 0.34, r * 0.55);
    }
    let _ = write!(s, "<circle cx='{x:.1}' cy='{y:.1}' r='{:.1}' fill='{}'/>", r * 0.16, ART_PHLOX_EYE.value);
}

fn pink_moon(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    let mut rng = Rng(0x0f10_c5);
    for &(side, sign) in &[(0.0, 1.0), (w, -1.0)] {
        // Low along the side edges, below the play area's text (I-T4 is
        // measured on the rendered pixels, AT-T5b).
        for _ in 0..7 {
            let x = side + sign * rng.next() * w * 0.07;
            let y = h - rng.next() * h * 0.05;
            let _ = write!(s, "<path d='M{x:.0} {h:.0}Q{:.0} {:.0} {x:.0} {y:.0}' stroke='{}' stroke-width='1.6' fill='none'/>", x + sign * 6.0, (y + h) / 2.0, ART_LEAF.value);
            let petal = if rng.next() > 0.5 { ART_PHLOX_1.value } else { ART_PHLOX_2.value };
            phlox(&mut s, x, y, 4.0 + rng.next() * 3.5, petal);
        }
    }
    s.push_str("</svg>");
    s
}

fn aurora(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    s.push_str("<defs><filter id='b' x='-10%' y='-50%' width='120%' height='200%'><feGaussianBlur stdDeviation='14'/></filter></defs><g filter='url(#b)' fill='none' stroke-linecap='round'>");
    let ribbons = [(0.10, ART_AURORA_GREEN.value, 46.0), (0.20, ART_AURORA_VIOLET.value, 38.0), (0.30, ART_AURORA_GREEN.value, 30.0)];
    for (i, (y, c, sw)) in ribbons.iter().enumerate() {
        let y0 = h * y;
        let lift = if i % 2 == 0 { -40.0 } else { 34.0 };
        let _ = write!(s, "<path d='M{:.0} {y0:.0}C{:.0} {:.0} {:.0} {:.0} {:.0} {y0:.0}' stroke='{c}' stroke-width='{sw}'/>", -w * 0.1, w * 0.3, y0 + lift, w * 0.7, y0 - lift, w * 1.1);
    }
    s.push_str("</g>");
    stars(&mut s, w, h, 26, 0.6, 0xa0a0_0001);
    s.push_str("</svg>");
    s
}

fn great_wall(w: f64, h: f64) -> String {
    let mut s = svg_open(w, h);
    stars(&mut s, w, h, 24, 0.35, 0x6e7a_11a1);
    let ridge = |base: f64, amp: f64, seed: u32| {
        let mut rng = Rng(seed);
        let mut pts = Vec::new();
        let n = 9;
        for i in 0..=n {
            pts.push((w * f64::from(i) / f64::from(n), base - rng.next() * amp));
        }
        pts
    };
    let far = ridge(h * 0.86, h * 0.07, 0x00fa_0001);
    let near = ridge(h * 0.93, h * 0.06, 0x00e4_0002);
    let path = |pts: &[(f64, f64)]| {
        let mut d = format!("M0 {h:.0}");
        for (x, y) in pts {
            let _ = write!(d, "L{x:.0} {y:.0}");
        }
        let _ = write!(d, "L{w:.0} {h:.0}Z");
        d
    };
    let _ = write!(s, "<path d='{}' fill='{}'/>", path(&far), ART_RIDGE_FAR.value);
    // The wall rides the near ridge: a band with crenellations and two towers.
    let mut top = String::new();
    for (i, (x, y)) in near.iter().enumerate() {
        let _ = write!(top, "{}{x:.0} {:.0}", if i == 0 { "M" } else { "L" }, y - 8.0);
    }
    let _ = write!(s, "<path d='{top}' stroke='{}' stroke-width='9' fill='none' stroke-linejoin='round'/>", ART_WALL.value);
    for win in near.windows(2) {
        let ((x0, y0), (x1, y1)) = (win[0], win[1]);
        let steps = ((x1 - x0) / 9.0).floor() as i32;
        for k in 0..steps {
            let t = f64::from(k) / f64::from(steps);
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            if k % 2 == 0 {
                let _ = write!(s, "<rect x='{:.1}' y='{:.1}' width='4.5' height='5' fill='{}'/>", x, y - 17.0, ART_WALL_CRENEL.value);
            }
        }
    }
    for &i in &[2usize, 6] {
        let (x, y) = near[i];
        let _ = write!(s, "<rect x='{:.0}' y='{:.0}' width='18' height='22' fill='{}'/>", x - 9.0, y - 30.0, ART_WALL_CRENEL.value);
    }
    let _ = write!(s, "<path d='{}' fill='{}'/>", path(&near), ART_RIDGE_NEAR.value);
    s.push_str("</svg>");
    s
}

/// The page art for a look at `w` x `h` CSS pixels; None when the theme draws
/// none (the Classic five, Lunar, Blood Moon).
pub fn page_svg(look: &Look, w: f64, h: f64) -> Option<String> {
    let (w, h) = (w.max(1.0).round(), h.max(1.0).round());
    Some(match look.theme {
        Theme::Bronze | Theme::Silver | Theme::Gold => brushed(w, h),
        Theme::Diamond => pave(w, h),
        Theme::Sky => sky(look.band.unwrap_or(Band::Day), w, h),
        Theme::Beach => beach(w, h),
        Theme::PinkMoon => pink_moon(w, h),
        Theme::Solar => {
            let mut s = svg_open(w, h);
            stars(&mut s, w, h, 110, 1.0, 0x50a1_a2);
            s.push_str("</svg>");
            s
        }
        Theme::Aurora => aurora(w, h),
        Theme::GreatWall => great_wall(w, h),
        _ => return None,
    })
}

// ---------- the orb's face (viewBox 0 0 100 100) ----------

fn orb_open() -> String {
    "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>".to_string()
}

/// The lit part of the Moon at `age` days: a semicircle on the lit side plus
/// the terminator, an ellipse whose half-width is |cos| of the phase angle.
/// Waxing lights the right side (as seen from the northern hemisphere).
pub fn lit_path(age: f64) -> String {
    let k = (age / SYNODIC).rem_euclid(1.0);
    let c = (k * std::f64::consts::TAU).cos();
    let rx = 50.0 * c.abs();
    if k < 0.5 {
        format!("M50 0A50 50 0 0 1 50 100A{rx:.2} 50 0 0 {} 50 0Z", if c > 0.0 { 0 } else { 1 })
    } else {
        format!("M50 0A50 50 0 0 0 50 100A{rx:.2} 50 0 0 {} 50 0Z", if c > 0.0 { 1 } else { 0 })
    }
}

fn moon(age: f64) -> String {
    let mut s = orb_open();
    let lit = lit_path(age);
    let _ = write!(
        s,
        "<defs><clipPath id='l'><path d='{lit}'/></clipPath></defs><circle cx='50' cy='50' r='50' fill='{}'/><path d='{lit}' fill='{}'/><g clip-path='url(#l)' fill='{}'><circle cx='38' cy='36' r='11'/><circle cx='60' cy='30' r='7'/><circle cx='58' cy='58' r='13'/><circle cx='34' cy='66' r='6'/></g></svg>",
        ART_MOON_SHADOW.value, ART_MOON_LIT.value, ART_MOON_MARE.value
    );
    s
}

fn blood_moon() -> String {
    let mut s = orb_open();
    let _ = write!(
        s,
        "<defs><radialGradient id='r' cx='42%' cy='38%' r='62%'><stop offset='0' stop-color='{}'/><stop offset='1' stop-color='{}'/></radialGradient></defs><circle cx='50' cy='50' r='50' fill='url(#r)'/><g fill='{}' opacity='.6'><circle cx='38' cy='36' r='11'/><circle cx='58' cy='58' r='13'/></g></svg>",
        ART_MOON_BLOOD_RIM.value, ART_MOON_BLOOD.value, ART_MOON_BLOOD.value
    );
    s
}

fn planet(p: Planet) -> String {
    let (a, b) = match p {
        Planet::Mercury => (ART_MERCURY_1.value, ART_MERCURY_2.value),
        Planet::Venus => (ART_VENUS_1.value, ART_VENUS_2.value),
        Planet::Earth => (ART_EARTH_SEA.value, ART_EARTH_SEA.value),
        Planet::Mars => (ART_MARS_1.value, ART_MARS_2.value),
        Planet::Jupiter => (ART_JUPITER_1.value, ART_JUPITER_2.value),
        Planet::Saturn => (ART_SATURN_1.value, ART_SATURN_2.value),
        Planet::Uranus => (ART_URANUS_1.value, ART_URANUS_2.value),
        Planet::Neptune => (ART_NEPTUNE_1.value, ART_NEPTUNE_2.value),
    };
    let mut s = orb_open();
    let _ = write!(s, "<defs><radialGradient id='p' cx='38%' cy='34%' r='70%'><stop offset='0' stop-color='{a}'/><stop offset='1' stop-color='{b}'/></radialGradient><clipPath id='c'><circle cx='50' cy='50' r='50'/></clipPath></defs>");
    // Saturn and Uranus sit smaller inside the orb so their rings fit.
    let r = if matches!(p, Planet::Saturn | Planet::Uranus) { 40 } else { 50 };
    let _ = write!(s, "<g clip-path='url(#c)'><circle cx='50' cy='50' r='50' fill='{}'/>", ART_MOON_SHADOW.value);
    match p {
        Planet::Saturn | Planet::Uranus => {
            let tilt = if p == Planet::Saturn { -18 } else { 82 };
            let ring = if p == Planet::Saturn { ART_SATURN_RING.value } else { ART_URANUS_1.value };
            let _ = write!(s, "<ellipse cx='50' cy='50' rx='48' ry='12' fill='none' stroke='{ring}' stroke-width='5' transform='rotate({tilt} 50 50)' opacity='.55'/>");
            let _ = write!(s, "<circle cx='50' cy='50' r='{r}' fill='url(#p)'/>");
            // The near half of the ring passes in front of the planet.
            let _ = write!(s, "<path d='M2 50A48 12 0 0 0 98 50' fill='none' stroke='{ring}' stroke-width='5' transform='rotate({tilt} 50 50)'/>");
        }
        _ => {
            let _ = write!(s, "<circle cx='50' cy='50' r='{r}' fill='url(#p)'/>");
        }
    }
    match p {
        Planet::Mercury => {
            let _ = write!(s, "<g fill='{b}' opacity='.55'><circle cx='30' cy='40' r='7'/><circle cx='62' cy='28' r='5'/><circle cx='58' cy='66' r='9'/><circle cx='36' cy='72' r='4'/></g>");
        }
        Planet::Earth => {
            let _ = write!(s, "<g fill='{}'><path d='M22 30Q36 18 48 30T44 56Q30 60 24 48Z'/><path d='M58 52Q76 46 80 62T60 84Q52 70 58 52Z'/></g>", ART_EARTH_LAND.value);
            let _ = write!(s, "<g fill='none' stroke='{}' stroke-width='4' stroke-linecap='round'><path d='M14 62Q40 54 62 66'/><path d='M40 16Q62 12 82 26'/></g>", ART_EARTH_CLOUD.value);
        }
        Planet::Mars => {
            let _ = write!(s, "<g fill='{b}' opacity='.5'><path d='M20 44Q40 34 58 46T84 44Q70 60 46 58T20 44Z'/></g><ellipse cx='50' cy='6' rx='18' ry='6' fill='{}' opacity='.8'/>", ART_STAR.value);
        }
        Planet::Jupiter => {
            for (y, hgt) in [(22, 7), (36, 9), (54, 8), (70, 6)] {
                let _ = write!(s, "<rect x='0' y='{y}' width='100' height='{hgt}' fill='{b}' opacity='.55'/>");
            }
            let _ = write!(s, "<ellipse cx='64' cy='62' rx='9' ry='5' fill='{}'/>", ART_JUPITER_SPOT.value);
        }
        Planet::Venus => {
            let _ = write!(s, "<g fill='none' stroke='{b}' stroke-width='6' opacity='.4'><path d='M8 40Q50 30 92 44'/><path d='M10 64Q50 56 90 70'/></g>");
        }
        Planet::Neptune => {
            let _ = write!(s, "<ellipse cx='40' cy='58' rx='9' ry='5' fill='{b}'/><path d='M14 36Q50 30 86 38' stroke='{a}' stroke-width='3' fill='none' opacity='.6'/>");
        }
        Planet::Saturn | Planet::Uranus => {}
    }
    // The night side, so the world reads as a sphere lit from the upper left.
    let _ = write!(s, "<circle cx='50' cy='50' r='{r}' fill='none' stroke='{}' stroke-width='{}' opacity='.35' transform='translate(6 6)'/>", ART_MOON_SHADOW.value, r / 3);
    s.push_str("</g></svg>");
    s
}

fn stone_orb() -> String {
    let mut s = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='-1.05 -1.05 2.1 2.1'><defs>".to_string();
    s.push_str(&stone_symbol("s"));
    s.push_str("</defs><use href='#s' x='-1' y='-1' width='2' height='2'/></svg>");
    s
}

/// The orb's face for a look; None keeps the gem.
pub fn orb_svg(look: &Look) -> Option<String> {
    Some(match look.theme {
        Theme::Lunar => moon(look.phase.map(|p| p.1).unwrap_or(SYNODIC / 2.0)),
        Theme::PinkMoon | Theme::GreatWall => moon(SYNODIC / 2.0),
        Theme::Sky if look.band == Some(Band::Night) => moon(SYNODIC / 2.0),
        Theme::BloodMoon => blood_moon(),
        Theme::Solar => planet(look.planet.unwrap_or(Planet::Mercury)),
        Theme::Diamond => stone_orb(),
        _ => return None,
    })
}

/// A CSS url() for an SVG, percent-encoded so it survives as a custom property.
pub fn css_url(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len() + 32);
    out.push_str("url(\"data:image/svg+xml,");
    for c in svg.chars() {
        match c {
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '"' => out.push_str("%22"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out.push_str("\")");
    out
}

// ---------- the web side ----------

thread_local! {
    /// What #themeArt holds now: (look key, width, height). Same key, no redraw.
    static DRAWN: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn key(look: &Look, w: f64, h: f64) -> String {
    format!("{}|{:?}|{:?}|{:?}|{w:.0}x{h:.0}", look.theme.id(), look.band, look.phase.map(|p| p.0), look.planet)
}

fn root_style() -> Option<web_sys::CssStyleDeclaration> {
    use wasm_bindgen::JsCast;
    let root = web_sys::window()?.document()?.document_element()?;
    Some(root.dyn_into::<web_sys::HtmlElement>().ok()?.style())
}

fn art_el() -> Option<web_sys::HtmlElement> {
    use wasm_bindgen::JsCast;
    let doc = web_sys::window()?.document()?;
    if let Some(el) = doc.get_element_by_id("themeArt") {
        return el.dyn_into().ok();
    }
    let el = doc.create_element("div").ok()?;
    el.set_id("themeArt");
    let _ = el.set_attribute("aria-hidden", "true");
    let body = doc.body()?;
    let _ = body.prepend_with_node_1(&el);
    install_resize();
    el.dyn_into().ok()
}

/// Draw `look` (None: take the art away). Called from themes::apply, which runs
/// at boot, on a theme change and at word boundaries, never mid-word.
pub fn render(look: Option<&Look>) {
    let Some(win) = web_sys::window() else { return };
    let Some(style) = root_style() else { return };
    let Some(look) = look.filter(|l| crate::themes_cosmos::is_cosmos(l.theme)) else {
        let _ = style.remove_property("--orb-art");
        if let Some(el) = win.document().and_then(|d| d.get_element_by_id("themeArt")) {
            el.remove();
        }
        DRAWN.with(|d| d.borrow_mut().clear());
        return;
    };
    let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(390.0);
    let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(844.0);
    let k = key(look, w, h);
    let Some(el) = art_el() else { return };
    place_scrims(&el);
    if DRAWN.with(|d| *d.borrow() == k) {
        return;
    }
    DRAWN.with(|d| *d.borrow_mut() = k);
    let es = el.style();
    match page_svg(look, w, h) {
        Some(svg) => {
            let _ = es.set_property("background-image", &css_url(&svg));
        }
        None => {
            let _ = es.remove_property("background-image");
        }
    }
    match orb_svg(look) {
        Some(svg) => {
            let _ = style.set_property("--orb-art", &css_url(&svg));
        }
        None => {
            let _ = style.remove_property("--orb-art");
        }
    }
}

/// F3a.3: the scrims follow the screen's own header, orb and keyboard. They
/// are transparent in every theme but Diamond, so this only places them.
fn place_scrims(el: &web_sys::HtmlElement) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let es = el.style();
    if let Some(orb) = doc.get_element_by_id("orbWrap") {
        let r = orb.get_bounding_client_rect();
        if r.width() > 0.0 {
            let _ = es.set_property("--scrim-orb-x", &format!("{:.0}px", r.left() + r.width() / 2.0));
            let _ = es.set_property("--scrim-orb-y", &format!("{:.0}px", r.top() + r.height() / 2.0));
            let _ = es.set_property("--scrim-orb-r", &format!("{:.0}px", r.width() * 0.62));
        }
    }
    if let Some(kb) = doc.get_element_by_id("gameKeyboard") {
        let r = kb.get_bounding_client_rect();
        if r.height() > 0.0 {
            let _ = es.set_property("--scrim-kb", &format!("{:.0}px", r.top() - 12.0));
        }
    }
}

fn install_resize() {
    use wasm_bindgen::{closure::Closure, JsCast};
    let cb = Closure::<dyn FnMut()>::new(|| crate::themes::apply());
    if let Some(w) = web_sys::window() {
        let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
    }
    cb.forget();
}

/// Between words: Diamond's one glint (T-D21), Aurora's drift. Neither runs
/// under Reduce Motion (the CSS neutralises both, and this does not start them).
pub fn between_words(theme: Theme, reduce_motion: bool) {
    if reduce_motion {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Some(el) = doc.get_element_by_id("themeArt") else { return };
    match theme {
        Theme::Diamond => {
            let _ = el.class_list().remove_1("glint");
            // Restart the animation: read layout once, then add the class back.
            let _ = el.get_bounding_client_rect();
            let _ = el.class_list().add_1("glint");
        }
        Theme::Aurora => {
            use wasm_bindgen::JsCast;
            if let Ok(h) = el.dyn_into::<web_sys::HtmlElement>() {
                let now = h.style().get_property_value("--art-shift").unwrap_or_default();
                let _ = h.style().set_property("--art-shift", if now.trim() == "14px" { "-14px" } else { "14px" });
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes_cosmos::{look, LocalNow, Pins, PLANETS};

    const NOON: LocalNow = LocalNow { y: 2026, m: 11, d: 2, h: 12, min: 0 };

    #[test]
    fn every_cosmos_theme_draws_something_and_the_classics_draw_nothing() {
        for t in crate::themes::REGISTRY {
            let l = look(t, NOON, Pins::default());
            let drawn = page_svg(&l, 390.0, 844.0).is_some() || orb_svg(&l).is_some();
            assert_eq!(drawn, crate::themes_cosmos::is_cosmos(t), "{}", t.id());
        }
    }

    #[test]
    fn art_is_deterministic() {
        let l = look(Theme::Diamond, NOON, Pins::default());
        assert_eq!(page_svg(&l, 390.0, 844.0), page_svg(&l, 390.0, 844.0));
    }

    #[test]
    fn the_pave_field_covers_the_screen_edge_to_edge() {
        let svg = page_svg(&look(Theme::Diamond, NOON, Pins::default()), 390.0, 844.0).unwrap();
        let stones = svg.matches("<use ").count();
        assert!(stones > 300, "{stones} stones");
        // Data-URI size stays modest (it is rasterised once).
        assert!(svg.len() < 200_000, "{} bytes", svg.len());
    }

    #[test]
    fn no_literal_colour_reaches_the_art() {
        // Every paint is a token value; the builders hold none of their own.
        let src = include_str!("theme_art.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        assert!(!body.contains("fill='#") && !body.contains("stroke='#") && !body.contains("stop-color='#"));
    }

    #[test]
    fn the_terminator_matches_the_phase() {
        // New: the right semicircle is cancelled by a full-width ellipse
        // bulging right; full: both halves lit.
        assert!(lit_path(0.0).contains("A50.00 50 0 0 0 50 0"));
        assert!(lit_path(SYNODIC / 2.0).contains("A50.00 50"));
        // Quarters: a straight terminator (zero-width ellipse).
        assert!(lit_path(SYNODIC / 4.0).contains("A0.00 50"));
        assert!(lit_path(SYNODIC * 0.75).contains("A0.00 50"));
        // Waxing lights the right side, waning the left.
        assert!(lit_path(5.0).starts_with("M50 0A50 50 0 0 1"));
        assert!(lit_path(20.0).starts_with("M50 0A50 50 0 0 0"));
    }

    #[test]
    fn every_planet_has_a_face() {
        for p in PLANETS {
            let svg = orb_svg(&look(Theme::Solar, NOON, Pins { planet: Some(p), ..Pins::default() })).unwrap();
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"), "{}", p.id());
        }
    }

    #[test]
    fn css_url_escapes_what_breaks_a_custom_property() {
        let u = css_url("<svg fill='#fff'>\"%</svg>");
        assert!(!u[5..u.len() - 2].contains('"') && !u.contains('#') && u.starts_with("url(\"data:image/svg+xml,"));
    }
}
