//! CC-VISUAL-ECLIPSE F11 -- shareable result cards, behind `visual.cards`.
//!
//! Every shared card is free marketing, so it should look like an advert: a
//! 1080x1350 card in the player's own colours, with the orb, the score, the
//! mode, a language accent and the right script's face.
//!
//! [`layout`] turns a [`CardSpec`] into a list of drawing operations. It is
//! pure and deterministic for a given seed (AT-26 snapshots the operations as
//! well as the pixels), and [`child_safe`] is the AT-27 rule: a card carries no
//! name, no avatar and no free text -- only fixed labels and numbers. The card
//! never has a name or avatar field at all; the rule guards the text.
//!
//! [`render_png`] draws the operations on a canvas. With the flag off, sharing
//! is exactly as it was (share.rs).

/// The card's size (F11.1).
pub const W: f64 = 1080.0;
pub const H: f64 = 1350.0;

/// The colours a card is drawn in: the player's live theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub bg: String,
    pub orb: String,
    pub glow: String,
    pub accent: String,
    pub text: String,
    pub muted: String,
    pub orb_text: String,
}

/// What a card says. Every string is a fixed label or a number (AT-27).
#[derive(Clone, Debug, PartialEq)]
pub struct CardSpec {
    /// The mode's name, from the registry.
    pub mode: String,
    /// The big figure: a chain, a score, a place.
    pub hero: String,
    /// What the figure is.
    pub hero_label: String,
    /// An optional second line: a best, a streak.
    pub line: Option<String>,
    /// The study language, for the accent and the script face.
    pub lang: String,
    /// The footer line ("hear it · spell it").
    pub tagline: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// A vertical gradient over the whole card.
    Background { top: String, bottom: String },
    /// A filled circle.
    Circle { x: f64, y: f64, r: f64, fill: String, alpha: f64 },
    /// The orb: a radial gradient from a highlight to its colour.
    Orb { x: f64, y: f64, r: f64, light: String, body: String },
    /// A band of the language accent behind the mode name.
    Pill { x: f64, y: f64, w: f64, h: f64, fill: String, alpha: f64 },
    Text { text: String, x: f64, y: f64, size: f64, weight: u32, fill: String, face: &'static str, rtl: bool },
}

/// The face for `lang`'s script, falling back to the display face (F11.3).
pub fn face(lang: &str) -> &'static str {
    match lang {
        "ar" | "fa" => "'Noto Naskh Arabic','Bricolage Grotesque',sans-serif",
        "ur" => "'Noto Nastaliq Urdu','Bricolage Grotesque',sans-serif",
        "hi" => "'Noto Sans Devanagari','Bricolage Grotesque',sans-serif",
        "zh" => "'PingFang SC','Noto Sans SC','Bricolage Grotesque',sans-serif",
        "ja" => "'Hiragino Sans','Noto Sans JP','Bricolage Grotesque',sans-serif",
        "ko" => "'Apple SD Gothic Neo','Noto Sans KR','Bricolage Grotesque',sans-serif",
        _ => "'Bricolage Grotesque',sans-serif",
    }
}

pub fn rtl(lang: &str) -> bool {
    crate::consts::dir_attr(lang) == "rtl"
}

/// A small deterministic generator, so a seed always draws the same sky.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f64 / u32::MAX as f64
    }
}

/// The drawing, as data. Same spec, palette and seed: same operations.
pub fn layout(spec: &CardSpec, p: &Palette, seed: u32) -> Vec<Op> {
    let cx = W / 2.0;
    let r = rtl(&spec.lang);
    let f = face(&spec.lang);
    let mut ops = vec![Op::Background { top: p.bg.clone(), bottom: crate::tokens::COLOR_BG.value.into() }];
    // A sparse star field, seeded.
    let mut rng = Rng(seed | 1);
    for _ in 0..42 {
        let (x, y, rad) = (rng.next() * W, rng.next() * H * 0.62, 1.0 + rng.next() * 2.6);
        ops.push(Op::Circle { x, y, r: rad, fill: p.text.clone(), alpha: 0.18 + rng.next() * 0.4 });
    }
    // The orb, with its corona.
    for (i, a) in [0.10, 0.16, 0.26].iter().enumerate() {
        ops.push(Op::Circle { x: cx, y: 360.0, r: 230.0 - i as f64 * 34.0, fill: p.glow.clone(), alpha: *a });
    }
    ops.push(Op::Orb { x: cx, y: 360.0, r: 150.0, light: p.text.clone(), body: p.orb.clone() });
    ops.push(Op::Text { text: "S".into(), x: cx, y: 368.0, size: 160.0, weight: 800, fill: p.orb_text.clone(), face: face("en"), rtl: false });
    // The mode, on a band of the language accent.
    ops.push(Op::Pill { x: cx - 300.0, y: 600.0, w: 600.0, h: 86.0, fill: p.accent.clone(), alpha: 0.22 });
    ops.push(Op::Text { text: spec.mode.clone(), x: cx, y: 646.0, size: 50.0, weight: 700, fill: p.text.clone(), face: f, rtl: r });
    // The figure and what it is.
    ops.push(Op::Text { text: spec.hero.clone(), x: cx, y: 860.0, size: 260.0, weight: 800, fill: p.text.clone(), face: face("en"), rtl: false });
    ops.push(Op::Text { text: spec.hero_label.clone(), x: cx, y: 1030.0, size: 56.0, weight: 700, fill: p.accent.clone(), face: f, rtl: r });
    if let Some(line) = &spec.line {
        ops.push(Op::Text { text: line.clone(), x: cx, y: 1110.0, size: 44.0, weight: 500, fill: p.muted.clone(), face: f, rtl: r });
    }
    ops.push(Op::Text { text: spec.tagline.clone(), x: cx, y: 1230.0, size: 42.0, weight: 700, fill: p.text.clone(), face: f, rtl: r });
    ops.push(Op::Text { text: "spellgame.net".into(), x: cx, y: 1292.0, size: 36.0, weight: 500, fill: p.muted.clone(), face: face("en"), rtl: false });
    ops
}

/// AT-27: every text on the card is one of `labels` (the fixed strings the
/// caller passed in from i18n and the registry), a number, or the brand. A unit
/// test, because at runtime a child profile cannot share at all (share.rs).
#[cfg(test)]
pub fn child_safe(ops: &[Op], labels: &[&str]) -> bool {
    ops.iter().all(|op| match op {
        Op::Text { text, .. } => {
            text == "S"
                || text == "spellgame.net"
                || labels.contains(&text.as_str())
                || text.chars().all(|c| c.is_ascii_digit() || "/#·: ".contains(c))
        }
        _ => true,
    })
}

// ---------- the web side ----------

/// The live theme, from the stylesheet's custom properties.
pub fn live_palette() -> Palette {
    use crate::tokens::*;
    let doc = web_sys::window().and_then(|w| w.document());
    let style = doc
        .as_ref()
        .and_then(|d| d.document_element())
        .and_then(|el| web_sys::window().and_then(|w| w.get_computed_style(&el).ok().flatten()));
    let get = |t: Token| {
        style
            .as_ref()
            .and_then(|s| s.get_property_value(t.css).ok())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| t.value.to_string())
    };
    Palette {
        bg: get(COLOR_BG_USER),
        orb: get(COLOR_ORB_CORE),
        glow: get(COLOR_GLOW),
        accent: get(COLOR_ACCENT),
        text: SHARE_CARD_TEXT.value.into(),
        muted: SHARE_CARD_MUTED.value.into(),
        orb_text: get(COLOR_ORB_TEXT),
    }
}

/// Draw a card to a PNG data URL.
pub fn render_png(spec: &CardSpec, palette: &Palette, seed: u32) -> Option<String> {
    use wasm_bindgen::JsCast;
    let doc = web_sys::window()?.document()?;
    let canvas: web_sys::HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(W as u32);
    canvas.set_height(H as u32);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    for op in layout(spec, palette, seed) {
        match op {
            Op::Background { top, bottom } => {
                let g = ctx.create_linear_gradient(0.0, 0.0, 0.0, H);
                let _ = g.add_color_stop(0.0, &top);
                let _ = g.add_color_stop(1.0, &bottom);
                ctx.set_fill_style_canvas_gradient(&g);
                ctx.fill_rect(0.0, 0.0, W, H);
            }
            Op::Circle { x, y, r, fill, alpha } => {
                ctx.set_global_alpha(alpha);
                ctx.begin_path();
                let _ = ctx.arc(x, y, r, 0.0, std::f64::consts::TAU);
                ctx.set_fill_style_str(&fill);
                ctx.fill();
                ctx.set_global_alpha(1.0);
            }
            Op::Orb { x, y, r, light, body } => {
                if let Ok(g) = ctx.create_radial_gradient(x - r * 0.3, y - r * 0.35, r * 0.1, x, y, r) {
                    let _ = g.add_color_stop(0.0, &light);
                    let _ = g.add_color_stop(0.35, &body);
                    let _ = g.add_color_stop(1.0, &body);
                    ctx.set_fill_style_canvas_gradient(&g);
                }
                ctx.begin_path();
                let _ = ctx.arc(x, y, r, 0.0, std::f64::consts::TAU);
                ctx.fill();
            }
            Op::Pill { x, y, w, h, fill, alpha } => {
                ctx.set_global_alpha(alpha);
                ctx.set_fill_style_str(&fill);
                ctx.begin_path();
                let _ = ctx.round_rect_with_f64(x, y, w, h, h / 2.0);
                ctx.fill();
                ctx.set_global_alpha(1.0);
            }
            Op::Text { text, x, y, size, weight, fill, face, rtl } => {
                // `direction` has no typed setter in this web-sys; set the property.
                let _ = js_sys::Reflect::set(&ctx, &"direction".into(), &(if rtl { "rtl" } else { "ltr" }).into());
                ctx.set_font(&format!("{weight} {size}px {face}"));
                ctx.set_fill_style_str(&fill);
                let _ = ctx.fill_text_with_max_width(&text, x, y, W - 120.0);
            }
        }
    }
    canvas.to_data_url_with_type("image/png").ok()
}

/// Today's date and the mode as a seed, so a card for the same result on the
/// same day is the same card.
pub fn seed_for(mode: &str, day: &str) -> u32 {
    // FNV-1a, 32-bit.
    format!("{mode}|{day}").bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Palette {
        Palette {
            bg: "#1c1830".into(),
            orb: "#ffb14d".into(),
            glow: "#ffb14d".into(),
            accent: "#ffb14d".into(),
            text: "#ffffff".into(),
            muted: "#b9b3d0".into(),
            orb_text: "#3a230a".into(),
        }
    }
    fn spec(lang: &str) -> CardSpec {
        CardSpec { mode: "The Climb".into(), hero: "12".into(), hero_label: "word chain".into(), line: Some("best: 15".into()), lang: lang.into(), tagline: "hear it · spell it".into() }
    }

    // AT-26: deterministic for a given seed.
    #[test]
    fn the_same_seed_draws_the_same_card() {
        assert_eq!(layout(&spec("en"), &pal(), 7), layout(&spec("en"), &pal(), 7));
        assert_ne!(layout(&spec("en"), &pal(), 7), layout(&spec("en"), &pal(), 8), "a different seed moves the stars");
    }

    #[test]
    fn the_card_is_1080_by_1350_and_everything_is_on_it() {
        assert_eq!((W, H), (1080.0, 1350.0));
        for op in layout(&spec("en"), &pal(), 3) {
            if let Op::Text { y, .. } | Op::Circle { y, .. } = op {
                assert!((0.0..=H).contains(&y), "{op:?}");
            }
        }
    }

    #[test]
    fn it_carries_the_orb_score_mode_and_accent() {
        let ops = layout(&spec("en"), &pal(), 1);
        assert!(ops.iter().any(|o| matches!(o, Op::Orb { .. })));
        let texts: Vec<&str> = ops.iter().filter_map(|o| if let Op::Text { text, .. } = o { Some(text.as_str()) } else { None }).collect();
        for want in ["The Climb", "12", "word chain", "best: 15"] {
            assert!(texts.contains(&want), "{want} missing from {texts:?}");
        }
        assert!(ops.iter().any(|o| matches!(o, Op::Pill { fill, .. } if fill == "#ffb14d")));
    }

    #[test]
    fn an_arabic_card_runs_right_to_left_in_the_arabic_face() {
        let ops = layout(&spec("ar"), &pal(), 1);
        let mode = ops.iter().find_map(|o| if let Op::Text { text, rtl, face, .. } = o { (text == "The Climb").then_some((*rtl, *face)) } else { None }).unwrap();
        assert!(mode.0 && mode.1.contains("Noto Naskh Arabic"));
        // The figure stays in the Latin face, left to right.
        let hero = ops.iter().find_map(|o| if let Op::Text { text, rtl, .. } = o { (text == "12").then_some(*rtl) } else { None }).unwrap();
        assert!(!hero);
    }

    #[test]
    fn script_faces() {
        assert!(face("hi").contains("Devanagari"));
        assert!(face("ja").contains("Hiragino"));
        assert!(face("en").starts_with("'Bricolage"));
    }

    // AT-27: no name, no avatar, no free text.
    #[test]
    fn a_card_holds_only_fixed_labels_and_numbers() {
        let s = spec("en");
        let ops = layout(&s, &pal(), 1);
        assert!(child_safe(&ops, &["The Climb", "word chain", "best: 15", "hear it · spell it"]));
        // A player's own text slipped in as the mode name is caught.
        let mut leaked = s.clone();
        leaked.mode = "Emma's list".into();
        assert!(!child_safe(&layout(&leaked, &pal(), 1), &["word chain", "best: 15", "hear it · spell it"]));
    }
}
