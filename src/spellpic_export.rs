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
}

impl ExportError {
    /// Audit-gated i18n key for what the player is told.
    pub fn i18n_key(self) -> &'static str {
        match self {
            ExportError::FontUnavailable => "finale.exportFontFailed",
            ExportError::NoPlan => "finale.exportNoPlan",
            ExportError::RasterFailed => "finale.exportFailed",
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
    fn d3_resolutions() {
        // Keepsake is 3x the 512pt canvas, well inside the 4096 cap.
        assert_eq!(Product::Keepsake.pixels(512), 1536);
        // ...and the cap actually binds on a hypothetical larger canvas.
        assert_eq!(Product::Keepsake.pixels(2048), KEEPSAKE_CAP_PX);
        assert_eq!(Product::ShareCard.pixels(512), 2048);
    }

    #[test]
    fn every_failure_is_a_refusal_with_an_audited_string() {
        for e in [ExportError::FontUnavailable, ExportError::NoPlan, ExportError::RasterFailed] {
            assert!(e.i18n_key().starts_with("finale."), "{e:?} has no audited string");
        }
    }
}
