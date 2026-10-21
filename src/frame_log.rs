//! CC-VISUAL-ECLIPSE A5 -- the frame-time logger behind AT-19.
//!
//! AT-19 asks for p95 frame time <= 16.7 ms and no frame over 50 ms on the
//! oldest supported device (the iPhone SE 2nd gen, signed 2026-10-20) during a
//! spoken word, typing, the result, BoardGame hops, a Randospell roll and, once
//! they exist, celebrations. There is no profiler on a TestFlight build, so the
//! app measures itself: while the logger is on, a requestAnimationFrame loop
//! records every frame's duration under whichever moment is happening.
//!
//! Moments are MARKED by the code that starts them (`mark`), never guessed from
//! the DOM: a word starting to play, a keystroke, a checked answer, a hop, a
//! roll. A mark holds for a window; the latest mark wins; frames outside every
//! window count as `idle`. Off is a true no-op: `mark` returns at once and no
//! loop runs.
//!
//! Instrumentation never reaches a player's screen: the switch and the report
//! live only in the dev door (five taps on the logo, `dev.rs`). The switch is
//! persisted (`spell_frame_log`) so a measurement survives a relaunch.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const STORE_KEY: &str = "spell_frame_log";
/// The AT-19 budgets. rAF timestamps land on vsync, so a smooth 60 Hz frame
/// reads 16.6-16.8 ms; the verdict allows 0.5 ms of that jitter on the p95 and
/// says so in the report.
pub const P95_BUDGET_MS: f64 = 16.7;
pub const JITTER_MS: f64 = 0.5;
pub const MAX_FRAME_MS: f64 = 50.0;
/// A gap this long is the app being backgrounded or paused, not a frame.
const PAUSE_MS: f64 = 1000.0;
/// Cap per moment so a long session cannot grow memory without bound.
const CAP_PER_MOMENT: usize = 20_000;

/// The moments AT-19 names, and how long each mark holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Moment {
    Speaking,
    Typing,
    Result,
    BoardgameHop,
    RandospellRoll,
    /// Marked by F7 (Phase D) when celebrations exist; nothing marks it yet.
    #[allow(dead_code)]
    Celebration,
}

impl Moment {
    pub fn label(self) -> &'static str {
        match self {
            Moment::Speaking => "speaking",
            Moment::Typing => "typing",
            Moment::Result => "result",
            Moment::BoardgameHop => "boardgame hop",
            Moment::RandospellRoll => "randospell roll",
            Moment::Celebration => "celebration",
        }
    }
    fn hold_ms(self) -> f64 {
        match self {
            Moment::Speaking => 2000.0,
            Moment::Typing => 500.0,
            Moment::Result => 1500.0,
            Moment::BoardgameHop => 1200.0,
            Moment::RandospellRoll => 2000.0,
            Moment::Celebration => 2500.0,
        }
    }
}

/// Pure frame statistics for one moment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub frames: usize,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
    pub over_max: usize,
}

impl Stats {
    pub fn of(samples: &[f64]) -> Stats {
        if samples.is_empty() {
            return Stats::default();
        }
        let mut v = samples.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let pick = |q: f64| v[(((v.len() - 1) as f64) * q).round() as usize];
        Stats {
            frames: v.len(),
            p50: pick(0.50),
            p95: pick(0.95),
            max: *v.last().unwrap(),
            over_max: v.iter().filter(|&&d| d > MAX_FRAME_MS).count(),
        }
    }
    /// AT-19's verdict for this moment.
    pub fn passes(&self) -> bool {
        self.frames > 0 && self.p95 <= P95_BUDGET_MS + JITTER_MS && self.over_max == 0
    }
}

/// The samples, by moment label ("idle" for unmarked frames).
#[derive(Default)]
pub struct Log {
    samples: BTreeMap<&'static str, Vec<f64>>,
    active: Option<(Moment, f64)>, // (moment, until_ms)
}

impl Log {
    pub fn mark(&mut self, m: Moment, now_ms: f64) {
        self.active = Some((m, now_ms + m.hold_ms()));
    }
    /// Record one frame of `dt` ms that ended at `now_ms`.
    pub fn frame(&mut self, dt: f64, now_ms: f64) {
        if !(dt > 0.0) || dt >= PAUSE_MS {
            return;
        }
        let label = match self.active {
            Some((m, until)) if now_ms <= until => m.label(),
            _ => "idle",
        };
        let v = self.samples.entry(label).or_default();
        if v.len() < CAP_PER_MOMENT {
            v.push(dt);
        }
    }
    pub fn clear(&mut self) {
        self.samples.clear();
        self.active = None;
    }
    pub fn stats(&self) -> Vec<(&'static str, Stats)> {
        self.samples.iter().map(|(k, v)| (*k, Stats::of(v))).collect()
    }
    /// Plain-text report: what Eric pastes back from the device.
    pub fn report(&self, device: &str) -> String {
        let mut out = format!(
            "SpellGame frame log (CC-VISUAL-ECLIPSE AT-19)\n{device}\n\
             budget: p95 <= {P95_BUDGET_MS} ms (+{JITTER_MS} ms vsync jitter), no frame > {MAX_FRAME_MS} ms\n\n\
             moment            frames    p50     p95     max   >50ms  AT-19\n"
        );
        for (label, s) in self.stats() {
            let verdict = if label == "idle" { "-" } else if s.passes() { "PASS" } else { "FAIL" };
            out.push_str(&format!(
                "{label:<16} {:>7} {:>6.1} {:>7.1} {:>7.1} {:>6}  {verdict}\n",
                s.frames, s.p50, s.p95, s.max, s.over_max
            ));
        }
        out
    }
}

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static LOG: RefCell<Log> = RefCell::new(Log::default());
    static RUN: Cell<u32> = const { Cell::new(0) };
    /// Loop health, printed in the report so an empty report says WHY:
    /// callbacks seen, frames skipped as hidden, deltas rejected as pauses.
    static TICKS: Cell<u64> = const { Cell::new(0) };
    static HIDDEN: Cell<u64> = const { Cell::new(0) };
}

fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0)
}

/// Mark that a measured moment just started. A no-op unless the logger is on.
pub fn mark(m: Moment) {
    if ON.with(Cell::get) {
        let t = now();
        LOG.with(|l| l.borrow_mut().mark(m, t));
    }
}

pub fn is_on() -> bool {
    ON.with(Cell::get)
}

pub fn report() -> String {
    let device = web_sys::window()
        .map(|w| {
            let ua = w.navigator().user_agent().unwrap_or_default();
            let dpr = w.device_pixel_ratio();
            let px = |v: Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>| v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let (sw, sh) = (px(w.inner_width()), px(w.inner_height()));
            let build = js_sys::Reflect::get(&w, &"SPELL_BUILD".into()).ok().and_then(|v| v.as_string()).unwrap_or_default();
            format!("build {build} | viewport {sw}x{sh} @{dpr}x | {ua}")
        })
        .unwrap_or_default();
    let health = format!(
        "logger {} | loop callbacks {} | hidden-skipped {}\n",
        if is_on() { "on" } else { "off" },
        TICKS.with(Cell::get),
        HIDDEN.with(Cell::get)
    );
    let mut r = LOG.with(|l| l.borrow().report(&device));
    if LOG.with(|l| l.borrow().stats().is_empty()) {
        r.push_str(if is_on() {
            "\nNo frames yet. Close this menu and play; frames are counted while the app is on screen.\n"
        } else {
            "\nNo frames recorded: the logger is OFF. Tick Frame logger (AT-19) above, close this menu, play, then come back here.\n"
        });
    }
    // The health line goes right after the device line.
    match r.find("budget:") {
        Some(i) => format!("{}{}{}", &r[..i], health, &r[i..]),
        None => r + &health,
    }
}

pub fn clear() {
    LOG.with(|l| l.borrow_mut().clear());
}

/// Turn the logger on or off, persisting the choice.
pub fn set_on(on: bool) {
    crate::storage::set_raw(STORE_KEY, if on { "on" } else { "off" });
    apply(on);
}

/// At startup: resume a measurement that was on before a relaunch.
pub fn restore() {
    apply(crate::storage::get_raw(STORE_KEY).as_deref() == Some("on"));
}

fn apply(on: bool) {
    let was = ON.with(|c| c.replace(on));
    if on && !was {
        start_loop();
    }
}

/// The sampler: one rAF callback per frame while on. A new run id retires any
/// older loop, so toggling quickly never leaves two loops counting.
fn start_loop() {
    let run = RUN.with(|r| {
        r.set(r.get().wrapping_add(1));
        r.get()
    });
    let Some(win) = web_sys::window() else { return };
    let last = Rc::new(Cell::new(-1.0f64));
    let tick: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let tick2 = tick.clone();
    *tick.borrow_mut() = Some(Closure::new(move |t: f64| {
        if !ON.with(Cell::get) || RUN.with(Cell::get) != run {
            tick2.borrow_mut().take(); // drop the closure: the loop ends here
            return;
        }
        TICKS.with(|c| c.set(c.get() + 1));
        let prev = last.replace(t);
        if prev >= 0.0 {
            let hidden = web_sys::window().and_then(|w| w.document()).map(|d| d.hidden()).unwrap_or(false);
            if hidden {
                HIDDEN.with(|c| c.set(c.get() + 1));
            } else {
                LOG.with(|l| l.borrow_mut().frame(t - prev, t));
            }
        }
        if let (Some(w), Some(cb)) = (web_sys::window(), tick2.borrow().as_ref()) {
            let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }));
    let first = tick.borrow();
    if let Some(cb) = first.as_ref() {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
    drop(first);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_percentiles_and_budget() {
        let mut v = vec![16.7; 95];
        v.extend([33.4; 5]);
        let s = Stats::of(&v);
        assert_eq!(s.frames, 100);
        assert!((s.p50 - 16.7).abs() < 1e-9);
        assert!((s.p95 - 16.7).abs() < 1e-9, "the 95th of 100 sits on the last smooth frame");
        assert_eq!(s.over_max, 0);
        assert!(s.passes());
        let mut bad = vec![16.7; 90];
        bad.extend([33.4; 10]);
        assert!(!Stats::of(&bad).passes(), "10% dropped frames fails the p95");
        let mut spike = vec![16.7; 200];
        spike.push(60.0);
        assert!(!Stats::of(&spike).passes(), "one frame over 50 ms fails");
        assert!(!Stats::of(&[]).passes(), "no frames is not a pass");
    }

    #[test]
    fn frames_land_under_the_latest_mark_until_it_expires() {
        let mut l = Log::default();
        l.frame(16.7, 10.0);
        l.mark(Moment::Typing, 100.0); // holds until 600
        l.frame(16.7, 200.0);
        l.mark(Moment::Result, 300.0); // latest wins, holds until 1800
        l.frame(16.7, 400.0);
        l.frame(16.7, 2000.0);
        let got: BTreeMap<_, _> = l.stats().into_iter().map(|(k, s)| (k, s.frames)).collect();
        assert_eq!(got.get("idle"), Some(&2));
        assert_eq!(got.get("typing"), Some(&1));
        assert_eq!(got.get("result"), Some(&1));
    }

    #[test]
    fn pauses_are_not_frames() {
        let mut l = Log::default();
        l.frame(5000.0, 5000.0);
        l.frame(0.0, 5001.0);
        assert!(l.stats().is_empty(), "a backgrounded gap or a zero delta is not a frame");
    }

    #[test]
    fn report_names_every_moment_and_the_verdict() {
        let mut l = Log::default();
        l.mark(Moment::Speaking, 0.0);
        for i in 0..50 {
            l.frame(16.7, i as f64 * 16.7);
        }
        let r = l.report("test device");
        assert!(r.contains("test device"));
        assert!(r.contains("speaking") && r.contains("PASS"));
    }
}
