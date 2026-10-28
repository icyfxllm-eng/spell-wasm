//! CC-VISUAL-ECLIPSE F7 -- celebration moments, behind `visual.celebrate`.
//!
//! If every correct answer got confetti, confetti would mean nothing. So:
//! - **Big** (V-D18): a day-streak tier (7/30/100/365), a Climb rank-up, a
//!   perfect run (10 in a row, no miss, in Normal, Daily or Climb), a new
//!   personal best, a theme unlock (Phase E). The orb goes to totality, the
//!   corona blooms and at most 200 particles fly; 2.5 s at most; a tap skips.
//! - **Small**: everything else (an achievement). An orb flare plus a toast.
//! - **Never during an active word**: a celebration asked for mid-word waits
//!   for the gap between words or the result screen.
//! - **At most one big per 60 s**: another big inside the minute plays small.
//! - **Reduce Motion**: the big one is a cross-fade card, no particles, no scale.
//!
//! [`Planner`] is the placement rule, pure, so AT-18 can script it. The records
//! E-Q9 signed (perfect runs, the best Climb rank per difficulty) are local and
//! display-only; they touch no grading.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Big,
    Small,
}

/// What earned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    StreakTier(u32),
    RankUp(u32),
    PerfectRun,
    PersonalBest(u32),
    #[allow(dead_code)]
    ThemeUnlock,
    Achievement,
}

pub fn size_of(t: Trigger) -> Size {
    match t {
        Trigger::Achievement => Size::Small,
        _ => Size::Big,
    }
}

/// The day-streak tiers that are big (V-D18).
pub const STREAK_TIERS: [u32; 4] = [7, 30, 100, 365];
/// A perfect run: this many in a row, no miss.
pub const PERFECT_RUN: u32 = 10;
/// A personal best is only worth a big one once the old best meant something.
pub const BEST_FLOOR: u32 = 3;

pub const BIG_MS: f64 = 2500.0;
pub const SMALL_MS: f64 = 900.0;
pub const BIG_EVERY_S: f64 = 60.0;
pub const MAX_PARTICLES: usize = 200;

/// The placement rule.
#[derive(Default, Debug)]
pub struct Planner {
    queue: VecDeque<(Trigger, String)>,
    last_big: Option<f64>,
}

impl Planner {
    pub fn ask(&mut self, t: Trigger, message: String) {
        self.queue.push_back((t, message));
    }

    /// The next celebration to play now, if one may: never during a word, and a
    /// big one within a minute of the last big plays small.
    pub fn next(&mut self, word_active: bool, now_s: f64) -> Option<(Size, Trigger, String)> {
        if word_active {
            return None;
        }
        let (t, msg) = self.queue.pop_front()?;
        let mut size = size_of(t);
        if size == Size::Big {
            if self.last_big.is_some_and(|at| now_s - at < BIG_EVERY_S) {
                size = Size::Small;
            } else {
                self.last_big = Some(now_s);
            }
        }
        Some((size, t, msg))
    }

    #[cfg(test)]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

/// The run-level rules for the base game's streak: what a new streak value
/// earns. `run_best` is the best as it stood when this run STARTED: while a run
/// is itself the record, the live best climbs with the streak, so comparing
/// against it would call every answer a new best.
pub fn streak_triggers(streak: u32, run_best: u32, eligible: bool) -> Vec<Trigger> {
    let mut v = Vec::new();
    if !eligible {
        return v;
    }
    if streak == PERFECT_RUN {
        v.push(Trigger::PerfectRun);
    }
    // Once per run: the answer that first passes the best the run started from.
    if run_best >= BEST_FLOOR && streak == run_best + 1 {
        v.push(Trigger::PersonalBest(streak));
    }
    v
}

pub fn streak_tier(streak: u32) -> Option<Trigger> {
    STREAK_TIERS.contains(&streak).then_some(Trigger::StreakTier(streak))
}

/// A Climb rank, against the best one stored for that difficulty: a rank-up
/// only when there was a stored rank and this one is better (smaller).
pub fn rank_up(stored: Option<u32>, rank: u32) -> Option<Trigger> {
    match stored {
        Some(prev) if rank > 0 && rank < prev => Some(Trigger::RankUp(rank)),
        _ => None,
    }
}

// ---------- the web side ----------

const RECORDS_KEY: &str = "spell_celebrate_v1";

/// E-Q9's local records: display-only.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Records {
    #[serde(default)]
    pub perfect_runs: u32,
    #[serde(default)]
    pub best_rank: std::collections::BTreeMap<String, u32>,
}

pub fn records() -> Records {
    crate::storage::get_json(RECORDS_KEY).unwrap_or_default()
}

fn save_records(r: &Records) {
    crate::storage::set_json(RECORDS_KEY, r);
}

thread_local! {
    /// The best as it stood when the current run started.
    static RUN_BEST: Cell<u32> = const { Cell::new(0) };
    static PLAN: RefCell<Planner> = RefCell::new(Planner::default());
    static WORD_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static BUSY_UNTIL: Cell<f64> = const { Cell::new(0.0) };
    static PLAYED: RefCell<Vec<(Size, f64)>> = const { RefCell::new(Vec::new()) };
}

fn on() -> bool {
    crate::flags::visual_celebrate()
}

fn now_s() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now() / 1000.0).unwrap_or(0.0)
}

/// Celebrations played this session, (size, at s) -- the seam and AT-18.
pub fn played() -> Vec<(Size, f64)> {
    PLAYED.with(|p| p.borrow().clone())
}

/// The base game's streak moved to `streak`; `best_before` is the best before
/// this answer. Returns what it earns, remembering where a new run started.
pub fn on_streak(streak: u32, best_before: u32, eligible: bool) -> Vec<Trigger> {
    if streak == 1 {
        RUN_BEST.with(|c| c.set(best_before));
    }
    streak_triggers(streak, RUN_BEST.with(Cell::get), eligible)
}

/// How long the current big celebration still holds the screen, in ms.
pub fn remaining_ms() -> f64 {
    ((BUSY_UNTIL.with(Cell::get) - now_s()) * 1000.0).max(0.0)
}

/// The base game says whether a word is being asked right now.
pub fn word_active(active: bool) {
    WORD_ACTIVE.with(|c| c.set(active));
    if !active {
        pump();
        crate::themes::at_word_boundary(); // CC-THEMES-COSMOS I-T5: backgrounds change only here
    }
}

/// Ask for a celebration; it plays now if no word is active, else waits.
pub fn ask(t: Trigger, message: String) {
    if !on() {
        return;
    }
    match t {
        Trigger::PerfectRun => {
            let mut r = records();
            r.perfect_runs += 1;
            save_records(&r);
        }
        _ => {}
    }
    PLAN.with(|p| p.borrow_mut().ask(t, message));
    pump();
}

/// A Climb record came back with `rank` on `difficulty`.
pub fn climb_rank(difficulty: &str, rank: u32) {
    if !on() || rank == 0 {
        return;
    }
    let mut r = records();
    let stored = r.best_rank.get(difficulty).copied();
    if stored.is_none_or(|s| rank < s) {
        r.best_rank.insert(difficulty.to_string(), rank);
        save_records(&r);
    }
    if let Some(t) = rank_up(stored, rank) {
        ask(t, crate::i18n::tp("celebrate.rankUp", &[("rank", &rank.to_string())]));
    }
}

/// The Theme Lab's "Play unlock moment" (F1.5): the big celebration with
/// `msg`, now, writing nothing (no record, no queue).
#[cfg(feature = "theme_lab")]
pub fn preview_big(msg: &str) {
    big(msg);
}

fn pump() {
    if remaining_ms() > 0.0 {
        return;
    }
    let next = PLAN.with(|p| p.borrow_mut().next(WORD_ACTIVE.with(Cell::get), now_s()));
    if let Some((size, _t, msg)) = next {
        PLAYED.with(|p| p.borrow_mut().push((size, now_s())));
        match size {
            Size::Big => big(&msg),
            Size::Small => small(&msg),
        }
    }
}

fn reduce_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn small(msg: &str) {
    crate::dom::add_class("orbWrap", "cel-flare");
    later(SMALL_MS, || crate::dom::remove_class("orbWrap", "cel-flare"));
    crate::dom::set_html("toast", &crate::icons::label("celebrate", msg));
    crate::dom::add_class("toast", "show");
    later(2400.0, || crate::dom::remove_class("toast", "show"));
}

fn big(msg: &str) {
    use wasm_bindgen::JsCast;
    BUSY_UNTIL.with(|c| c.set(now_s() + BIG_MS / 1000.0));
    crate::orb_view::milestone();
    crate::sfx::milestone();
    crate::haptics::milestone();
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Some(el) = doc.get_element_by_id("celebrate") else { return };
    crate::dom::set_html("celebrateMsg", &crate::icons::label("star", msg));
    let _ = el.set_attribute("aria-label", msg);
    let calm = reduce_motion();
    let _ = el.class_list().toggle_with_force("calm", calm);
    let _ = el.class_list().add_1("show");
    // A tap anywhere on the overlay ends it.
    let cb = wasm_bindgen::closure::Closure::once_into_js(end_big);
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_once(true);
    let _ = el.add_event_listener_with_callback_and_add_event_listener_options("pointerdown", cb.unchecked_ref(), &opts);
    if !calm {
        particles();
    }
    later(BIG_MS, end_big);
}

fn end_big() {
    if BUSY_UNTIL.with(Cell::get) == 0.0 {
        return;
    }
    BUSY_UNTIL.with(|c| c.set(0.0));
    crate::dom::remove_class("celebrate", "show");
    if crate::orb_view::state() == crate::orb::State::Milestone {
        crate::orb_view::idle();
    }
    pump();
}

fn later(ms: f64, f: impl FnOnce() + 'static) {
    use wasm_bindgen::JsCast;
    let cb = wasm_bindgen::closure::Closure::once_into_js(f);
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms as i32);
    }
}

/// Up to MAX_PARTICLES sparks from the orb, drawn on #celebrateFx until the big
/// celebration ends or is tapped away.
fn particles() {
    use std::rc::Rc;
    use wasm_bindgen::{closure::Closure, JsCast};
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let Some(canvas) = doc.get_element_by_id("celebrateFx").and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
    let dpr = win.device_pixel_ratio();
    let (w, h) = (win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(375.0), win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(667.0));
    canvas.set_width((w * dpr) as u32);
    canvas.set_height((h * dpr) as u32);
    let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return };
    let _ = ctx.scale(dpr, dpr);
    let (ox, oy) = doc
        .get_element_by_id("orbWrap")
        .map(|o| {
            let r = o.get_bounding_client_rect();
            (r.left() + r.width() / 2.0, r.top() + r.height() / 2.0)
        })
        .unwrap_or((w / 2.0, h / 3.0));
    let style = win.get_computed_style(&doc.document_element().unwrap()).ok().flatten();
    let var = |n: &str, d: &str| style.as_ref().and_then(|s| s.get_property_value(n).ok()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).unwrap_or(d.to_string());
    use crate::tokens::{COLOR_ACCENT_SOFT, COLOR_GLOW, COLOR_TEAL, COLOR_TEXT};
    let colours = [
        var(COLOR_GLOW.css, COLOR_GLOW.value),
        var(COLOR_TEAL.css, COLOR_TEAL.value),
        var(COLOR_ACCENT_SOFT.css, COLOR_ACCENT_SOFT.value),
        var(COLOR_TEXT.css, COLOR_TEXT.value),
    ];
    let mut seed: u32 = (js_sys::Date::now() as u32) | 1;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64) / (u32::MAX as f64)
    };
    let mut parts: Vec<[f64; 6]> = (0..MAX_PARTICLES)
        .map(|i| {
            let a = rnd() * std::f64::consts::TAU;
            let v = 2.0 + rnd() * 6.0;
            [ox, oy, a.cos() * v, a.sin() * v - 3.0, 2.0 + rnd() * 3.0, (i % 4) as f64]
        })
        .collect();
    let t0 = now_s();
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);
    *g.borrow_mut() = Some(Closure::new(move || {
        let age = (now_s() - t0) * 1000.0;
        ctx.clear_rect(0.0, 0.0, w, h);
        if age > BIG_MS || BUSY_UNTIL.with(Cell::get) == 0.0 {
            f.borrow_mut().take();
            return;
        }
        let fade = (1.0 - age / BIG_MS).max(0.0);
        ctx.set_global_alpha(fade);
        for p in parts.iter_mut() {
            p[0] += p[2];
            p[1] += p[3];
            p[3] += 0.18;
            p[2] *= 0.99;
            ctx.set_fill_style_str(&colours[p[5] as usize]);
            ctx.fill_rect(p[0], p[1], p[4], p[4]);
        }
        if let (Some(win), Some(cb)) = (web_sys::window(), f.borrow().as_ref()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }));
    let first = g.borrow();
    if let Some(cb) = first.as_ref() {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg() -> String {
        String::from("m")
    }

    #[test]
    fn sizes_follow_v_d18() {
        for t in [Trigger::StreakTier(7), Trigger::RankUp(3), Trigger::PerfectRun, Trigger::PersonalBest(9), Trigger::ThemeUnlock] {
            assert_eq!(size_of(t), Size::Big, "{t:?}");
        }
        assert_eq!(size_of(Trigger::Achievement), Size::Small);
    }

    // AT-18: no celebration starts while a word is active.
    #[test]
    fn nothing_plays_during_a_word_and_it_waits_for_the_gap() {
        let mut p = Planner::default();
        p.ask(Trigger::PerfectRun, msg());
        assert_eq!(p.next(true, 0.0), None);
        assert_eq!(p.pending(), 1);
        assert_eq!(p.next(false, 1.0).map(|x| x.0), Some(Size::Big));
    }

    // AT-18: at most one big per 60 s; extra bigs are downgraded to small.
    #[test]
    fn a_second_big_inside_a_minute_plays_small() {
        let mut p = Planner::default();
        p.ask(Trigger::PerfectRun, msg());
        p.ask(Trigger::PersonalBest(12), msg());
        assert_eq!(p.next(false, 0.0).unwrap().0, Size::Big);
        assert_eq!(p.next(false, 30.0).unwrap().0, Size::Small);
        p.ask(Trigger::StreakTier(7), msg());
        assert_eq!(p.next(false, 61.0).unwrap().0, Size::Big, "a minute later a big is big again");
    }

    #[test]
    fn a_scripted_session_never_plays_two_bigs_in_a_minute() {
        let mut p = Planner::default();
        let mut bigs: Vec<f64> = Vec::new();
        let mut t = 0.0;
        for i in 0u32..200 {
            t += 7.0 + (i % 5) as f64;
            let trig = [Trigger::PerfectRun, Trigger::PersonalBest(i), Trigger::Achievement, Trigger::RankUp(2)][i as usize % 4];
            p.ask(trig, msg());
            let active = i % 3 == 0;
            while let Some((size, _, _)) = p.next(active, t) {
                if size == Size::Big {
                    bigs.push(t);
                }
            }
        }
        for w in bigs.windows(2) {
            assert!(w[1] - w[0] >= BIG_EVERY_S, "bigs at {} and {}", w[0], w[1]);
        }
        assert!(bigs.len() > 5);
    }

    #[test]
    fn durations_are_within_the_spec() {
        assert!(BIG_MS <= 2500.0 && SMALL_MS <= 900.0 && MAX_PARTICLES <= 200);
    }

    #[test]
    fn a_perfect_run_is_ten_in_a_row_and_fires_once() {
        assert_eq!(streak_triggers(10, 50, true), vec![Trigger::PerfectRun]);
        assert!(streak_triggers(11, 50, true).is_empty());
        assert!(streak_triggers(10, 50, false).is_empty(), "not in Spell Off or Misses");
    }

    #[test]
    fn a_personal_best_fires_once_when_the_old_best_is_passed() {
        assert_eq!(streak_triggers(8, 7, true), vec![Trigger::PersonalBest(8)]);
        assert!(streak_triggers(9, 7, true).is_empty(), "once per run, not every word after");
        assert!(streak_triggers(3, 2, true).is_empty(), "an old best under the floor is not worth a big one");
        assert_eq!(streak_triggers(10, 9, true), vec![Trigger::PerfectRun, Trigger::PersonalBest(10)]);
    }

    #[test]
    fn a_record_run_celebrates_its_best_once_not_every_word() {
        // The live best climbs with a record run; the run's starting best does not.
        let run_best = 5;
        let fired: Vec<u32> = (1..=12).filter(|s| streak_triggers(*s, run_best, true).contains(&Trigger::PersonalBest(*s))).collect();
        assert_eq!(fired, vec![6]);
    }

    #[test]
    fn day_streak_tiers() {
        assert_eq!(streak_tier(7), Some(Trigger::StreakTier(7)));
        assert_eq!(streak_tier(365), Some(Trigger::StreakTier(365)));
        assert_eq!(streak_tier(8), None);
    }

    #[test]
    fn a_rank_up_needs_a_stored_rank_and_a_better_one() {
        assert_eq!(rank_up(None, 4), None, "the first record is not a rank-up");
        assert_eq!(rank_up(Some(9), 4), Some(Trigger::RankUp(4)));
        assert_eq!(rank_up(Some(4), 4), None);
        assert_eq!(rank_up(Some(4), 9), None);
    }
}
