//! Freehand drawing pad used to write a spelled word by hand, which then
//! gets OCR'd back into the answer box. Expanded beyond the original
//! Undo/Clear-only pad with: brush size, an eraser, a color palette, a
//! straight-line/ruler tool, a letter-guide (baseline/x-height) overlay,
//! single-pointer palm rejection, and smoothed/pressure-sensitive strokes.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, PointerEvent};

use crate::dom;

// CC-CJK-INK F2. The pad used to call a `window.spellOcr` global that no longer
// exists -- the retirement took the JS with it. It now goes to the SAME
// on-device Vision recognizer the photo word-list uses, through the native
// bridge, so no ink touches the network (Invariant 1).
//
// Reached the way every other native call is reached: Reflect over the global
// the JS layer actually publishes. The first attempt used a wasm_bindgen
// js_namespace import of `window.NativeLanguageKit`, which is the name of the
// FILE and not the name of the object -- the object is `SpellNativeLang`. That
// binding resolved to nothing and failed silently, so the pad drew perfectly
// and recognised nothing. It was also the only binding of its kind in the
// codebase, which was the clue.

/// Hand the current drawing to the recognizer and return its candidates,
/// best first. An empty list means "nothing legible", never a guess.
pub async fn recognize(lang: &str) -> Vec<(String, f64)> {
    // The CROPPED render, not the live pad: render_for_ocr tightens to the ink
    // and drops the UI-sized backing store, and Vision needs the glyph to fill
    // the frame -- the probe had to lower minimumTextHeight even on centred
    // renders.
    let canvas = render_for_ocr();
    let Ok(png) = canvas.to_data_url_with_type("image/png") else {
        return Vec::new();
    };
    // Show what was actually sent. Two rounds of this feature were debugged by
    // reasoning about an image nobody had seen -- first a bridge that resolved
    // to nothing, then a geometry guess. The picture settles it.
    if let Some(el) = dom::doc().get_element_by_id("inkSent") {
        let _ = el.set_attribute("src", &png);
        let _ = el.remove_attribute("hidden");
    }
    let Some(win) = web_sys::window() else { return Vec::new() };
    let Ok(kit) = js_sys::Reflect::get(&win, &JsValue::from_str("SpellNativeLang")) else {
        return Vec::new();
    };
    if kit.is_undefined() || kit.is_null() {
        return Vec::new();
    }
    let Ok(f) = js_sys::Reflect::get(&kit, &JsValue::from_str("recognizeInk")) else {
        return Vec::new();
    };
    let Ok(f) = f.dyn_into::<js_sys::Function>() else { return Vec::new() };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"png".into(), &png.into());
    let _ = js_sys::Reflect::set(&opts, &"lang".into(), &lang.into());
    let Ok(p) = f.call1(&kit, &opts) else { return Vec::new() };
    let Ok(p) = p.dyn_into::<js_sys::Promise>() else { return Vec::new() };
    let Ok(res) = JsFuture::from(p).await else { return Vec::new() };
    let Ok(list) = js_sys::Reflect::get(&res, &"candidates".into()) else {
        return Vec::new();
    };
    let arr = js_sys::Array::from(&list);
    let mut out = Vec::new();
    for v in arr.iter() {
        let t = js_sys::Reflect::get(&v, &"text".into())
            .ok()
            .and_then(|x| x.as_string())
            .unwrap_or_default();
        let c = js_sys::Reflect::get(&v, &"confidence".into())
            .ok()
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        if !t.is_empty() {
            out.push((t, c));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Pen,
    Eraser,
    Line,
}

#[derive(Clone)]
struct Stroke {
    color: String,
    width: f64,
    erase: bool,
    pts: Vec<(f64, f64)>,
    /// Parallel to `pts`; 0.0 for input that doesn't report pressure (mouse,
    /// most touchscreens), in which case width stays flat.
    pressures: Vec<f64>,
}

struct DrawState {
    dpr: f64,
    strokes: Vec<Stroke>,
    drawing_now: bool,
    /// The one pointer currently drawing, if any — a second simultaneous
    /// pointer (e.g. a palm resting on the screen next to a stylus) is
    /// ignored entirely rather than starting its own stroke.
    active_pointer: Option<i32>,
    tool: Tool,
    brush: f64,
    color: String,
    guide: bool,
    canvas_ready: bool,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            dpr: 1.0,
            strokes: Vec::new(),
            drawing_now: false,
            active_pointer: None,
            tool: Tool::Pen,
            brush: 4.0,
            color: "#ffb14d".to_string(),
            guide: false,
            canvas_ready: false,
        }
    }
}

thread_local! {
    static STATE: RefCell<DrawState> = RefCell::new(DrawState::default());
}

fn ctx_2d(canvas: &HtmlCanvasElement) -> CanvasRenderingContext2d {
    canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap()
}

pub fn set_tool(tool: Tool) {
    STATE.with(|s| s.borrow_mut().tool = tool);
}

pub fn set_brush(size: f64) {
    STATE.with(|s| s.borrow_mut().brush = size);
}

pub fn set_color(color: &str) {
    STATE.with(|s| s.borrow_mut().color = color.to_string());
}

pub fn set_guide(on: bool) {
    STATE.with(|s| s.borrow_mut().guide = on);
    redraw_all();
}

pub fn has_strokes() -> bool {
    STATE.with(|s| !s.borrow().strokes.is_empty())
}

pub fn size_canvas() {
    let canvas = dom::canvas("canvas");
    let dpr = dom::window().device_pixel_ratio();
    let css_w = canvas.client_width().max(1) as f64;
    // Reads the actual rendered height (driven by CSS, which sizes the pad
    // taller on small viewports) instead of assuming a fixed value, so the
    // backing-store math stays correct however the canvas is styled.
    let css_h = canvas.client_height().max(1) as f64;
    canvas.set_width((css_w * dpr).round() as u32);
    canvas.set_height((css_h * dpr).round() as u32);
    let ctx = ctx_2d(&canvas);
    let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.dpr = dpr;
        st.canvas_ready = true;
    });
    redraw_all();
}

fn draw_guides(ctx: &CanvasRenderingContext2d, css_w: f64, css_h: f64) {
    ctx.save();
    ctx.set_stroke_style_str("rgba(255,177,77,.45)");
    ctx.set_line_width(1.0);
    let _ = ctx.set_line_dash(&js_sys::Array::of2(&JsValue::from_f64(6.0), &JsValue::from_f64(5.0)));
    // baseline (where letters sit) and x-height guide (top of lowercase letters)
    for frac in [0.72_f64, 0.32_f64] {
        let y = (css_h * frac).round() + 0.5;
        ctx.begin_path();
        ctx.move_to(0.0, y);
        ctx.line_to(css_w, y);
        let _ = ctx.stroke();
    }
    ctx.restore();
}

/// Widens (or leaves flat) a base line width by per-point pressure. Only
/// input that actually reports pressure (real styluses) changes the width;
/// mouse/touch (pressure 0) stay at a flat `base`.
fn width_at(base: f64, pressure: f64) -> f64 {
    if pressure > 0.0 {
        base * (0.5 + pressure).min(1.6)
    } else {
        base
    }
}

/// Draws one segment of a stroke ending at `pts[i]`, smoothed via a
/// quadratic curve through the midpoints on either side of `pts[i-1]` (the
/// standard technique for turning raw polyline samples into a natural
/// curve). The first segment (`i == 1`) has no earlier point to smooth
/// against, so it's a plain line.
fn draw_segment(ctx: &CanvasRenderingContext2d, pts: &[(f64, f64)], pressures: &[f64], i: usize, base_width: f64) {
    ctx.set_line_width(width_at(base_width, pressures[i]));
    ctx.begin_path();
    if i == 1 {
        ctx.move_to(pts[0].0, pts[0].1);
        ctx.line_to(pts[1].0, pts[1].1);
    } else {
        let (p0, p1, p2) = (pts[i - 2], pts[i - 1], pts[i]);
        let mid_prev = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
        let mid_cur = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        ctx.move_to(mid_prev.0, mid_prev.1);
        ctx.quadratic_curve_to(p1.0, p1.1, mid_cur.0, mid_cur.1);
    }
    let _ = ctx.stroke();
}

fn draw_full_stroke(ctx: &CanvasRenderingContext2d, st: &Stroke) {
    ctx.set_global_composite_operation(if st.erase { "destination-out" } else { "source-over" }).ok();
    ctx.set_stroke_style_str(&st.color);
    for i in 1..st.pts.len() {
        draw_segment(ctx, &st.pts, &st.pressures, i, st.width);
    }
    ctx.set_global_composite_operation("source-over").ok();
}

pub fn redraw_all() {
    let canvas = dom::canvas("canvas");
    let ctx = ctx_2d(&canvas);
    let dpr = STATE.with(|s| s.borrow().dpr).max(0.0001);
    let css_w = canvas.width() as f64 / dpr;
    let css_h = canvas.height() as f64 / dpr;
    ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    let guide_on = STATE.with(|s| s.borrow().guide);
    if guide_on {
        draw_guides(&ctx, css_w, css_h);
    }

    STATE.with(|s| {
        for st in &s.borrow().strokes {
            draw_full_stroke(&ctx, st);
        }
    });

    let has = STATE.with(|s| !s.borrow().strokes.is_empty());
    dom::toggle_class("drawHint", "gone", has);
}

fn pt_from(canvas: &HtmlCanvasElement, e: &PointerEvent) -> (f64, f64) {
    let r = canvas.get_bounding_client_rect();
    (e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top())
}

pub fn start_stroke(e: &PointerEvent) {
    let ready = STATE.with(|s| s.borrow().canvas_ready);
    if !ready {
        return;
    }
    // Palm rejection: only the first pointer down gets to draw until it's
    // lifted, and only "primary" pointers (ignores e.g. a secondary
    // simultaneous touch) start a stroke at all.
    if !e.is_primary() {
        e.prevent_default();
        return;
    }
    let already_active = STATE.with(|s| s.borrow().active_pointer.is_some());
    if already_active {
        e.prevent_default();
        return;
    }

    let canvas = dom::canvas("canvas");
    let p = pt_from(&canvas, e);
    let pressure = e.pressure() as f64;
    let _ = canvas.set_pointer_capture(e.pointer_id());

    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.active_pointer = Some(e.pointer_id());
        let (color, width, erase) = match st.tool {
            Tool::Pen => (st.color.clone(), st.brush, false),
            Tool::Eraser => (String::new(), st.brush * 2.2, true),
            Tool::Line => (st.color.clone(), st.brush, false),
        };
        st.strokes.push(Stroke { color, width, erase, pts: vec![p, p], pressures: vec![pressure, pressure] });
        st.drawing_now = true;
    });
    dom::add_class("drawHint", "gone");
    e.prevent_default();
}

/// Feeds one already-resolved (point, pressure) sample into the current
/// stroke and incrementally draws just the new segment (for the Pen/Eraser
/// tools — cheap per-sample, unlike replaying the whole stroke).
fn add_pen_sample(ctx: &CanvasRenderingContext2d, p: (f64, f64), pressure: f64) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if let Some(last) = st.strokes.last_mut() {
            last.pts.push(p);
            last.pressures.push(pressure);
            let i = last.pts.len() - 1;
            ctx.set_global_composite_operation(if last.erase { "destination-out" } else { "source-over" }).ok();
            ctx.set_stroke_style_str(&last.color);
            draw_segment(ctx, &last.pts, &last.pressures, i, last.width);
            ctx.set_global_composite_operation("source-over").ok();
        }
    });
}

pub fn move_stroke(e: &PointerEvent) {
    let (drawing, active) = STATE.with(|s| {
        let st = s.borrow();
        (st.drawing_now, st.active_pointer)
    });
    if !drawing || active != Some(e.pointer_id()) {
        return;
    }
    let canvas = dom::canvas("canvas");
    let tool = STATE.with(|s| s.borrow().tool);

    match tool {
        Tool::Line => {
            let p = pt_from(&canvas, e);
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                if let Some(last) = st.strokes.last_mut() {
                    let start = last.pts[0];
                    last.pts = vec![start, p];
                    last.pressures = vec![0.0, 0.0];
                }
            });
            redraw_all();
        }
        Tool::Pen | Tool::Eraser => {
            let ctx = ctx_2d(&canvas);
            // High-frequency samples the browser batched since the last
            // event (fast strokes on a capable device) — falls back to
            // just this event when unsupported/empty.
            let coalesced = e.get_coalesced_events();
            if coalesced.length() == 0 {
                add_pen_sample(&ctx, pt_from(&canvas, e), e.pressure() as f64);
            } else {
                for sub in coalesced.iter() {
                    if let Ok(sub) = sub.dyn_into::<PointerEvent>() {
                        add_pen_sample(&ctx, pt_from(&canvas, &sub), sub.pressure() as f64);
                    }
                }
            }
        }
    }
    e.prevent_default();
}

pub fn end_stroke(e: &PointerEvent) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.active_pointer == Some(e.pointer_id()) {
            st.drawing_now = false;
            st.active_pointer = None;
        }
    });
}

pub fn undo_stroke() {
    STATE.with(|s| {
        s.borrow_mut().strokes.pop();
    });
    redraw_all();
}

pub fn clear_canvas() {
    STATE.with(|s| s.borrow_mut().strokes.clear());
    let canvas = dom::canvas("canvas");
    if canvas.width() > 0 {
        let ctx = ctx_2d(&canvas);
        ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    }
    dom::remove_class("drawHint", "gone");
}

/// Renders the strokes onto a tight, high-contrast off-screen canvas
/// suitable for OCR (white background, black ink), mirroring the original
/// `renderForOCR`.
fn render_for_ocr() -> HtmlCanvasElement {
    let (min_x, min_y, max_x, max_y) = STATE.with(|s| {
        let st = s.borrow();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for stroke in &st.strokes {
            for (x, y) in &stroke.pts {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
        }
        (min_x, min_y, max_x, max_y)
    });

    let w = (max_x - min_x).max(40.0);
    let h = (max_y - min_y).max(40.0);
    // Two geometries, because two different things are being read.
    //
    // A Latin word is a wide ribbon, so it goes into a 900x300 box. A CJK
    // character is a square, and F0 measured the recogniser on SQUARE images
    // with the glyph filling about 78% of the frame -- 3/3 there against 2/3
    // once the same characters were put in a bigger frame with more margin.
    // Sending a shape the recogniser was never measured on is guessing, so a
    // single character gets the geometry that was validated.
    let square = crate::ink_probe::wants_square();
    let (box_w, box_h, pad) = if square { (256.0, 256.0, 28.0) } else { (900.0, 300.0, 24.0) };
    let scale = (box_w / w).min(box_h / h);

    let doc = dom::doc();
    let out: HtmlCanvasElement = doc.create_element("canvas").unwrap().dyn_into().unwrap();
    out.set_width((w * scale + pad * 2.0).round() as u32);
    out.set_height((h * scale + pad * 2.0).round() as u32);
    let o = ctx_2d(&out);
    o.set_fill_style_str("#fff");
    o.fill_rect(0.0, 0.0, out.width() as f64, out.height() as f64);
    o.set_stroke_style_str("#000");
    o.set_line_cap("round");
    o.set_line_join("round");
    // A pen line, not a hairline. Thin strokes survived the local test, but a
    // heavier line is closer to the filled glyphs F0 read at full confidence
    // and costs nothing on a character this size.
    o.set_line_width(if square { (10.0_f64).max(w * scale * 0.055) } else { (6.0_f64).max(8.0 * scale * 0.5) });

    STATE.with(|s| {
        for stroke in &s.borrow().strokes {
            if stroke.erase {
                continue;
            }
            o.begin_path();
            for (i, (x, y)) in stroke.pts.iter().enumerate() {
                let px = (x - min_x) * scale + pad;
                let py = (y - min_y) * scale + pad;
                if i == 0 {
                    o.move_to(px, py);
                } else {
                    o.line_to(px, py);
                }
            }
            let _ = o.stroke();
        }
    });
    out
}

/// Below this mean-confidence score (Tesseract's own 0-100 estimate for the
/// whole read), a recognized word is shown to the player for confirmation
/// instead of being trusted automatically — a wrong OCR read on a correctly
/// spelled word would otherwise silently score as a miss that's the OCR's
/// fault, not the player's.
const OCR_CONFIDENCE_THRESHOLD: f64 = 65.0;

pub enum OcrOutcome {
    /// Confidently read — safe to treat as the player's actual answer.
    Confident(String),
    /// Recognized *something*, but not confidently enough to trust as-is.
    Unsure(String),
    Empty,
    Failed,
}

pub async fn read_writing() -> OcrOutcome {
    read_writing_in("en").await
}

/// Read the pad for a given study language.
///
/// Latin keeps its old shape: letters only, thresholded, because an English
/// answer is a word and stray marks are noise.
///
/// CJK does NOT filter to letters. The retired code kept only
/// `is_ascii_alphabetic`, which would strip every character this feature exists
/// to read -- a filter that was correct for the feature it was written for and
/// silently fatal for this one.
pub async fn read_writing_in(lang: &str) -> OcrOutcome {
    if !has_strokes() {
        return OcrOutcome::Empty;
    }
    let cands = recognize(lang).await;
    let Some((text, confidence)) = cands.into_iter().next() else {
        return OcrOutcome::Empty;
    };
    let cjk = lang.starts_with("zh") || lang.starts_with("ja");
    let kept: String = if cjk {
        text.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        text.chars().filter(|c| c.is_ascii_alphabetic()).collect()
    };
    if kept.is_empty() {
        OcrOutcome::Empty
    } else if confidence >= OCR_CONFIDENCE_THRESHOLD {
        OcrOutcome::Confident(kept)
    } else {
        OcrOutcome::Unsure(kept)
    }
}

#[cfg(test)]
mod bridge_tests {
    /// The ink path must reach the global the JS layer ACTUALLY publishes.
    ///
    /// This exists because the first version bound to `window.NativeLanguageKit`
    /// -- the name of the FILE -- while the object is `SpellNativeLang`. Nothing
    /// failed loudly: the pad captured strokes perfectly and recognised nothing,
    /// and the only symptom was a human saying "I don't know if it registered".
    /// A silent bridge is worse than a broken one.
    #[test]
    fn the_ink_bridge_names_the_global_the_js_layer_publishes() {
        // The SOURCE, not ios/App/App/public — that directory is a build
        // OUTPUT, regenerated by `npm run build`. The first version of this
        // feature edited the output, so the JS method was wiped before it ever
        // shipped: the plugin existed, the Rust called for it, and the method
        // it called was not in the bundle. Checking the output would have
        // agreed with itself right up until the next build.
        let js = include_str!("../native-language-kit.js");
        let rs = include_str!("drawing.rs");
        // Whatever the JS assigns to window is the only name that can work.
        let published = js
            .lines()
            .find_map(|l| l.trim().strip_prefix("window.").and_then(|r| r.split(&[' ', '=']).next()))
            .expect("the JS layer publishes no window global");
        assert!(
            rs.contains(&format!("\"{published}\"")),
            "drawing.rs does not reference `{published}`, the global the JS layer \
             publishes — the bridge would resolve to nothing and fail silently"
        );
        // And the method has to exist on that object.
        assert!(
            js.contains("recognizeInk:"),
            "the JS layer no longer exposes recognizeInk"
        );
    }
}
