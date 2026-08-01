//! CC-FINALE feature 2 — the clean export renderer.
//!
//! The rule this module exists to enforce: an export is RE-RENDERED from
//! the piece's own data, never screen-captured. That is what lets it beat
//! a screenshot -- no HUD, no notch, resolution independent of the device.
//!
//! The awkward part is fonts. Rasterising an SVG through an <img> loads no
//! stylesheet and no webfont: external font references are ignored in that
//! context for security reasons. An export that merely said
//! `font-family: inherit` would silently fall back to a system face, every
//! glyph advance would shift, and the exported picture would not be the
//! picture. So the face travels inside the SVG as a data URI.
//!
//! We embed exactly the webfonts the picture actually uses, which is fewer
//! than it sounds. `.wp-word` is `font-family: inherit` with no `:lang()`
//! override anywhere, so every picture in every language renders in
//! Instrument Sans and falls back to the system font for scripts Instrument
//! does not cover -- CJK, Arabic, Devanagari. That fallback happens
//! identically on screen and in the export, because system fonts DO resolve
//! during rasterisation. Only the webfont needs carrying.
//!
//! Eric's call on failure (2026-07-31): fetch the face at export time and
//! REFUSE if it cannot be had. A keepsake in the wrong font is worse than
//! no keepsake, and it is the kind of wrong the player would not notice
//! until it is already in their camera roll.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::wordpic_screen::RenderMode;

/// The Instrument Sans subsets, with the unicode-ranges index.html declares.
/// Already split for the web, so there is no runtime subsetter here and no
/// need for one: latin + latinext together are ~61KB.
const FACES: &[(&str, &str)] = &[
    (
        "./fonts/instrument-latin.woff2",
        "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,\
         U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD",
    ),
    (
        "./fonts/instrument-latinext.woff2",
        "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,\
         U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,\
         U+A720-A7FF",
    ),
];

/// D3 (Eric, 2026-07-31). The canvas is 512pt square.
pub const KEEPSAKE_SCALE: u32 = 3;
pub const KEEPSAKE_CAP_PX: u32 = 4096;
pub const SHARE_CARD_PX: u32 = 2048;

/// What the player asked for. Both come from the same renderer; they differ
/// only in framing and resolution.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Product {
    /// Their art. No wordmark -- D2 is explicit that the keepsake is not an ad.
    Keepsake,
    /// Framed, titled, and carrying the wordmark.
    ShareCard,
}

impl Product {
    pub fn pixels(self, canvas_pt: u32) -> u32 {
        match self {
            Product::Keepsake => (canvas_pt * KEEPSAKE_SCALE).min(KEEPSAKE_CAP_PX),
            Product::ShareCard => SHARE_CARD_PX,
        }
    }
}

/// Why an export did not happen. Every arm is a refusal, never a
/// silently-degraded image.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportError {
    /// The face could not be fetched. Refusing is the point: see the module
    /// note. The player sees an audited string, not a wrong-font picture.
    FontUnavailable,
    /// The piece is not typesettable, so there is nothing legal to draw
    /// (the same I10 gate that governs play).
    NoPlan,
    /// The browser could not rasterise. Nothing partial is handed back.
    RasterFailed,
    /// No camera roll to save to -- a browser, or the site build. Share still
    /// works; the two are deliberately independent.
    NoPhotosAccess,
    /// Photos refused, or the write failed.
    SaveFailed,
}

impl ExportError {
    /// Audit-gated i18n key for what the player is told.
    pub fn i18n_key(self) -> &'static str {
        match self {
            ExportError::FontUnavailable => "finale.exportFontFailed",
            ExportError::NoPlan => "finale.exportNoPlan",
            ExportError::RasterFailed => "finale.exportFailed",
            ExportError::NoPhotosAccess => "finale.saveUnavailable",
            ExportError::SaveFailed => "finale.saveFailed",
        }
    }
}

/// Standard base64. Hand-rolled to keep the export path dependency-free and
/// unit-testable off-device -- it runs on font bytes, so a subtle bug here
/// would show up as a broken face rather than a crash.
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Assemble the `@font-face` block from already-fetched bytes.
pub fn face_css(faces: &[(String, &str)]) -> String {
    faces
        .iter()
        .map(|(b64, range)| {
            format!(
                "@font-face{{font-family:'SpellExport';font-style:normal;font-weight:400 700;\
                 src:url(data:font/woff2;base64,{b64}) format('woff2');unicode-range:{range};}}"
            )
        })
        .collect()
}

/// Fetch every face the export needs and return the CSS to embed.
/// `Err(FontUnavailable)` if ANY of them fails -- a partial face set means
/// some words render in a fallback and others do not, which is a subtler
/// wrong than losing the lot.
pub async fn fetch_face_css() -> Result<String, ExportError> {
    let win = web_sys::window().ok_or(ExportError::FontUnavailable)?;
    let mut out = Vec::new();
    for (path, range) in FACES {
        let resp = JsFuture::from(win.fetch_with_str(path))
            .await
            .map_err(|_| ExportError::FontUnavailable)?
            .dyn_into::<web_sys::Response>()
            .map_err(|_| ExportError::FontUnavailable)?;
        if !resp.ok() {
            return Err(ExportError::FontUnavailable);
        }
        let buf = JsFuture::from(resp.array_buffer().map_err(|_| ExportError::FontUnavailable)?)
            .await
            .map_err(|_| ExportError::FontUnavailable)?;
        let bytes = js_sys::Uint8Array::new(&buf).to_vec();
        if bytes.is_empty() {
            return Err(ExportError::FontUnavailable);
        }
        out.push((base64(&bytes), *range));
    }
    Ok(face_css(&out))
}

/// What the share card prints under the piece. Everything here is data the
/// app already owns -- no new strings pass the audit gate because none are
/// composed: the icon is the picture's, the endonym is the registry's, the
/// wordmark is the brand, the attribution is provenance.
pub struct CardMeta<'a> {
    pub icon: &'a str,
    pub lang_endonym: &'a str,
    /// "after Leonardo da Vinci" for PD-Art; empty otherwise.
    pub attribution: &'a str,
    /// 1..=4 dots (easy..expert) -- a tier MARK, not copy to translate.
    pub tier_dots: u8,
}

/// CC-FINALE feature 2, the share card: the piece plus a tasteful frame.
/// D2: the wordmark lives HERE and only here -- the keepsake is their art,
/// not an ad, and a test pins that difference. The card is taller than the
/// piece (512x600): art untouched on top, one quiet caption band below.
pub fn card_svg(piece_svg: &str, meta: &CardMeta) -> String {
    // The piece arrives as a complete <svg>; embed it verbatim via nesting,
    // which preserves its geometry byte-for-byte (and the parity test's
    // guarantees with it).
    let inner = piece_svg;
    let dots: String = (0..4)
        .map(|i| {
            let fill = if i < meta.tier_dots { "#ffb14d" } else { "#3a4258" };
            format!("<circle cx=\"{}\" cy=\"557\" r=\"5\" fill=\"{fill}\"/>", 258 + i as i32 * 18)
        })
        .collect();
    let attribution = if meta.attribution.is_empty() {
        String::new()
    } else {
        format!(
            "<text x=\"256\" y=\"592\" text-anchor=\"middle\" font-size=\"12\" fill=\"#95a0bb\" font-style=\"italic\">{}</text>",
            crate::dom::escape_html(meta.attribution)
        )
    };
    format!(
        "<svg viewBox=\"0 0 512 600\" xmlns=\"http://www.w3.org/2000/svg\">         <rect width=\"512\" height=\"600\" fill=\"#0e1420\"/>         <svg x=\"0\" y=\"0\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">{inner}</svg>         <rect x=\"0\" y=\"512\" width=\"512\" height=\"88\" fill=\"#121a2b\"/>         <text x=\"22\" y=\"563\" font-size=\"30\">{icon}</text>         {dots}         <text x=\"490\" y=\"560\" text-anchor=\"end\" font-size=\"15\" fill=\"#eef1f8\" font-weight=\"600\">SpellGame</text>         <text x=\"70\" y=\"560\" font-size=\"14\" fill=\"#95a0bb\">{lang}</text>         {attribution}         </svg>",
        icon = crate::dom::escape_html(meta.icon),
        lang = crate::dom::escape_html(meta.lang_endonym),
    )
}

/// The export SVG for a finished piece: same geometry as the final play
/// frame, carrying its own styles, background and face.
pub fn export_svg(
    plan: &crate::spellpic::Plan,
    lang: &str,
    words: &[String],
    face_css: &str,
) -> String {
    crate::wordpic_screen::scanlock_svg(
        plan,
        lang,
        words,
        RenderMode::Export { font_data_uri: face_css },
    )
}

/// Rasterise a finished piece to a PNG data URL at `px` square.
///
/// Re-rendered from the piece's own data at export resolution -- never a
/// screen capture, which feature 2 forbids and which would cap quality at
/// whatever device happened to be in the player's hand.
///
/// The SVG travels as a data URI into an <img>, which is the step that makes
/// the embedded font non-negotiable: that context loads no stylesheet and no
/// webfont, so anything not carried inside the markup falls back silently.
pub async fn rasterize(svg: &str, px: u32) -> Result<String, ExportError> {
    rasterize_wh(svg, px, px).await
}

/// Non-square rasterize for the share card.
pub async fn rasterize_wh(svg: &str, w_px: u32, h_px: u32) -> Result<String, ExportError> {
    let doc = web_sys::window().and_then(|w| w.document()).ok_or(ExportError::RasterFailed)?;
    let img: web_sys::HtmlImageElement = doc
        .create_element("img")
        .map_err(|_| ExportError::RasterFailed)?
        .dyn_into()
        .map_err(|_| ExportError::RasterFailed)?;

    // percent-encode rather than base64: the SVG is text, and keeping it
    // readable in a data URI makes an export bug inspectable in devtools.
    let mut enc = String::with_capacity(svg.len() * 2);
    for b in svg.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b' ' | b'=' | b':'
            | b'/' | b',' | b'(' | b')' | b';' | b'{' | b'}' | b'\'' => enc.push(b as char),
            _ => enc.push_str(&format!("%{b:02X}")),
        }
    }
    img.set_src(&format!("data:image/svg+xml;charset=utf-8,{enc}"));

    // decode() resolves when the image is actually ready to draw. Waiting on
    // onload alone can hand back a blank canvas on WebKit.
    JsFuture::from(img.decode()).await.map_err(|_| ExportError::RasterFailed)?;

    let canvas: web_sys::HtmlCanvasElement = doc
        .create_element("canvas")
        .map_err(|_| ExportError::RasterFailed)?
        .dyn_into()
        .map_err(|_| ExportError::RasterFailed)?;
    canvas.set_width(w_px);
    canvas.set_height(h_px);
    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| ExportError::RasterFailed)?
        .ok_or(ExportError::RasterFailed)?
        .dyn_into()
        .map_err(|_| ExportError::RasterFailed)?;
    ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, w_px as f64, h_px as f64)
        .map_err(|_| ExportError::RasterFailed)?;
    canvas.to_data_url_with_type("image/png").map_err(|_| ExportError::RasterFailed)
}

/// The whole export path for a finished piece: fetch the face, render,
/// rasterise. Any failure is a refusal -- never a wrong-font keepsake.
pub async fn export_png(
    plan: &crate::spellpic::Plan,
    lang: &str,
    words: &[String],
    product: Product,
    meta: Option<CardMeta<'_>>,
) -> Result<String, ExportError> {
    if plan.placements.is_empty() {
        return Err(ExportError::NoPlan);
    }
    let face = fetch_face_css().await?;
    let piece = export_svg(plan, lang, words, &face);
    match (product, meta) {
        (Product::ShareCard, Some(m)) => {
            // Card is 512x600: rasterize at the card's aspect so nothing
            // squashes; D3's 2048 applies to the longest side.
            let svg = card_svg(&piece, &m);
            rasterize_wh(&svg, 2048 * 512 / 600, 2048).await
        }
        _ => rasterize(&piece, product.pixels(512)).await,
    }
}

/// CC-FINALE feature 3 — hand the keepsake to Photos.
///
/// Add-only: the native side asks for `.addOnly` authorization and can never
/// enumerate the library. Permission denied is a refusal with an audited
/// string, and Share keeps working either way -- sharing needs no Photos
/// access at all, which is exactly why the spec keeps them independent.
///
/// With no native plugin (the web build, or a browser) there is no camera
/// roll to save to, so say so rather than pretending.
pub fn save_to_photos(data_url: &str) -> Result<(), ExportError> {
    use wasm_bindgen::JsValue;
    let Some(b64) = data_url.split(',').nth(1) else {
        return Err(ExportError::RasterFailed);
    };
    let Some(plugin) = crate::share::cap_plugin("NativeLanguageKit") else {
        return Err(ExportError::NoPhotosAccess);
    };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &JsValue::from_str("data"), &JsValue::from_str(b64));
    let f = js_sys::Reflect::get(&plugin, &JsValue::from_str("savePicture"))
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .ok_or(ExportError::NoPhotosAccess)?;
    let promise = f
        .call1(&plugin, &opts)
        .ok()
        .and_then(|p| p.dyn_into::<js_sys::Promise>().ok())
        .ok_or(ExportError::SaveFailed)?;
    wasm_bindgen_futures::spawn_local(async move {
        let key = match JsFuture::from(promise).await {
            Ok(_) => "finale.saved",
            // Every rejection reads the same to the player: it did not save.
            // Distinguishing "denied" from "failed" in copy would be guessing
            // at the native reason string.
            Err(_) => ExportError::SaveFailed.i18n_key(),
        };
        crate::dom::set_text("wpRevealNote", &crate::i18n::t(key));
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_reference_vectors() {
        // RFC 4648 §10 -- if the padding is wrong the font silently fails to
        // decode and every word falls back, which is exactly the failure
        // this module exists to prevent.
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_handles_high_bytes() {
        // woff2 is binary; a sign-extension bug would corrupt the face.
        assert_eq!(base64(&[0xFF, 0xFE, 0xFD]), "//79");
        assert_eq!(base64(&[0x00, 0x80, 0xFF]), "AID/");
    }

    #[test]
    fn every_face_carries_a_unicode_range() {
        // Two faces with no ranges would both claim every codepoint and the
        // second would win, dropping half the Latin coverage.
        for (path, range) in FACES {
            assert!(path.ends_with(".woff2"), "{path} is not a woff2");
            assert!(range.starts_with("U+"), "{path} has no unicode-range");
        }
        let css = face_css(&FACES.iter().map(|(_, r)| ("AAAA".to_string(), *r)).collect::<Vec<_>>());
        assert_eq!(css.matches("@font-face").count(), FACES.len());
        assert_eq!(css.matches("unicode-range:").count(), FACES.len());
    }

    #[test]
    fn the_wordmark_lives_on_the_card_and_never_the_keepsake() {
        // D2 verbatim: "on share cards only -- never on the saved keepsake
        // (their art, not an ad)". The keepsake path IS export_svg, which
        // this asserts stays brand-free.
        let piece = "<svg viewBox=\"0 0 512 512\"><path d=\"M1 1\"/></svg>";
        let meta = CardMeta { icon: "🐺", lang_endonym: "Español", attribution: "", tier_dots: 3 };
        let card = card_svg(piece, &meta);
        assert!(card.contains("SpellGame"), "the card carries the wordmark");
        assert!(!piece.contains("SpellGame"), "the keepsake never does");
        // The piece is embedded VERBATIM: geometry untouched by framing.
        assert!(card.contains(piece), "framing must not touch the art");
        assert!(card.contains("Español"), "language endonym on the card");
        assert_eq!(card.matches("#ffb14d").count(), 3, "three tier dots lit for hard");
    }

    #[test]
    fn masterpiece_cards_carry_the_attribution_and_others_do_not() {
        let piece = "<svg viewBox=\"0 0 512 512\"></svg>";
        let with = card_svg(piece, &CardMeta { icon: "🖼", lang_endonym: "English",
            attribution: "after Leonardo da Vinci", tier_dots: 4 });
        assert!(with.contains("after Leonardo da Vinci"), "honest and classy");
        let without = card_svg(piece, &CardMeta { icon: "⭐", lang_endonym: "English",
            attribution: "", tier_dots: 1 });
        assert!(!without.contains("font-style=\"italic\""), "no empty attribution row");
    }

    #[test]
    fn d3_resolutions() {
        // Keepsake is 3x the 512pt canvas, well inside the 4096 cap.
        assert_eq!(Product::Keepsake.pixels(512), 1536);
        // ...and the cap actually binds on a hypothetical larger canvas.
        assert_eq!(Product::Keepsake.pixels(2048), KEEPSAKE_CAP_PX);
        assert_eq!(Product::ShareCard.pixels(512), 2048);
    }

    #[test]
    fn every_failure_is_a_refusal_with_an_audited_string() {
        for e in [ExportError::FontUnavailable, ExportError::NoPlan, ExportError::RasterFailed,
                  ExportError::NoPhotosAccess, ExportError::SaveFailed] {
            assert!(e.i18n_key().starts_with("finale."), "{e:?} has no audited string");
        }
    }
}
