//! CC-RANDOSPELL v1.1 F18 -- the one canvas particle layer of the Eclipse
//! table. Everything persistent (stones, dial, corona, heat notches, lanterns)
//! is CSS and SVG in `randospell_screen`; this file draws only the two things
//! that need many short-lived points: a Bank's stones streaming into the
//! lantern, and a miss's stones cracking into dust.
//!
//! The law (D-R22, A33): at most `MAX_PARTICLES` alive, the canvas torn down
//! the moment the burst ends so none exists between turns, and nothing at all
//! under Reduce Motion (the state is carried by numerals, pips and levels).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub const MAX_PARTICLES: usize = 200;
const CANVAS_ID: &str = "rsFxCanvas";
const BURST_MS: f64 = 900.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Burst {
    /// A Bank: cool white-blue points stream toward the lantern.
    Stream,
    /// A miss: the stones crack and fall as dust.
    Shatter,
}

thread_local! {
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
    static RUN: Cell<u32> = const { Cell::new(0) };
}

pub fn reduce_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

pub fn live() -> usize {
    LIVE.with(Cell::get)
}

pub fn peak() -> usize {
    PEAK.with(Cell::get)
}

pub fn canvas_present() -> bool {
    crate::dom::doc().get_element_by_id(CANVAS_ID).is_some()
}

struct P {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r: f64,
}

fn teardown() {
    if let Some(c) = crate::dom::doc().get_element_by_id(CANVAS_ID) {
        c.remove();
    }
    LIVE.with(|l| l.set(0));
}

/// Start a burst of `n` particles (capped) inside `#rsFxHost`. A burst already
/// running is replaced, so the cap holds across overlapping events.
pub fn burst(kind: Burst, n: usize) {
    if reduce_motion() {
        return;
    }
    let doc = crate::dom::doc();
    let Some(host) = doc.get_element_by_id("rsFxHost") else { return };
    teardown();
    let n = n.min(MAX_PARTICLES);
    let w = host.client_width().max(1) as u32;
    let h = host.client_height().max(1) as u32;
    let Ok(el) = doc.create_element("canvas") else { return };
    let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() else { return };
    canvas.set_id(CANVAS_ID);
    canvas.set_width(w);
    canvas.set_height(h);
    let _ = canvas.set_attribute("class", "rs-fx-canvas");
    let _ = canvas.set_attribute("aria-hidden", "true");
    if host.append_child(&canvas).is_err() {
        return;
    }
    let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else {
        teardown();
        return;
    };
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    // A cheap deterministic spread: the effect is decoration, not state.
    let mut seed = (js_sys::Date::now() as u64) ^ 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / ((1u64 << 31) as f64)
    };
    let mut ps: Vec<P> = (0..n)
        .map(|_| {
            let a = rnd() * std::f64::consts::TAU;
            let s = 0.5 + rnd() * 2.2;
            match kind {
                // Out of the ring, then drawn up toward the top lantern.
                Burst::Stream => P { x: cx + a.cos() * 50.0, y: cy + a.sin() * 50.0, vx: a.cos() * s * 0.4, vy: -s * 1.2, r: 1.5 + rnd() * 2.0 },
                Burst::Shatter => P { x: cx + a.cos() * 50.0, y: cy + a.sin() * 50.0, vx: a.cos() * s, vy: a.sin() * s * 0.5, r: 1.0 + rnd() * 2.5 },
            }
        })
        .collect();
    LIVE.with(|l| l.set(ps.len()));
    PEAK.with(|p| p.set(p.get().max(ps.len())));
    let token = RUN.with(|r| {
        r.set(r.get().wrapping_add(1));
        r.get()
    });
    let t0 = js_sys::Date::now();
    let slot: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let again = slot.clone();
    let tick = Closure::<dyn FnMut()>::new(move || {
        let stale = RUN.with(Cell::get) != token;
        let age = js_sys::Date::now() - t0;
        if stale || age > BURST_MS {
            if !stale {
                teardown();
            }
            *again.borrow_mut() = None; // drop the closure: the loop ends
            return;
        }
        ctx.clear_rect(0.0, 0.0, w as f64, h as f64);
        let fade = 1.0 - age / BURST_MS;
        for p in ps.iter_mut() {
            p.x += p.vx;
            p.y += p.vy;
            if kind == Burst::Shatter {
                p.vy += 0.12; // dust falls
            }
            let _ = &p.r;
        }
        match kind {
            Burst::Stream => ctx.set_fill_style_str(&format!("rgba(170,205,255,{fade:.2})")), // token-exempt: per-frame alpha, built at runtime (mode-owned art)
            Burst::Shatter => ctx.set_fill_style_str(&format!("rgba(150,140,175,{fade:.2})")), // token-exempt: see above
        }
        for p in &ps {
            ctx.fill_rect(p.x, p.y, p.r * 2.0, p.r * 2.0);
        }
        if let Some(win) = web_sys::window() {
            if let Some(cb) = again.borrow().as_ref() {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }
    });
    if let Some(win) = web_sys::window() {
        let _ = win.request_animation_frame(tick.as_ref().unchecked_ref());
    }
    *slot.borrow_mut() = Some(tick);
}

/// Tear the layer down now (a new game, or the screen closing).
pub fn clear() {
    RUN.with(|r| r.set(r.get().wrapping_add(1)));
    teardown();
}
