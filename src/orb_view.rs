//! CC-VISUAL-ECLIPSE F3 -- the orb on screen, behind `visual.orb`.
//!
//! Applies `crate::orb` to the base game's `#orbWrap` (Normal, Daily, Climb,
//! Impossible, Jr and Little Speller all play there). Modes with their own orb
//! are untouched until Eric approves each side by side (F3.6).
//!
//! - The state is `#orbWrap[data-orb]`; the stylesheet draws each one, and
//!   under Reduce Motion only their opacity and brightness change.
//! - `fetching` is set logically at once but drawn only after the grace token,
//!   so a cache hit never flickers.
//! - `speaking` drives `--orb-env` (0..1) every frame from a precomputed RMS
//!   envelope read against the playback clock (V-D13). The bytes come from a
//!   SIDE READ (E-Q2 a): on the web a second `fetch` of the same immutable URL
//!   the `<audio>` element plays (the HTTP cache answers it); in the app the
//!   cached file on disk (`SpellAudio.readCached`). Playback itself is not
//!   touched. If the decode fails the orb breathes instead and the failure is
//!   logged.
//! - The idle breath rests when the page is hidden or after 30 s without input
//!   (V-D23): `#orbWrap[data-orb-rest]` pauses it.
//!
//! Off (the default) every entry point returns at once: no attribute, no
//! listener, no fetch, no decode.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::HtmlAudioElement;

use crate::orb::{self, State, Step};

const WRAP: &str = "orbWrap";
/// How many decoded envelopes are kept (replays and the warmed next word).
const ENV_CACHE: usize = 8;
/// The app's native player reports only that play was queued (E-Q2): the
/// envelope clock starts there plus this offset, to be measured on a device
/// for AT-8. Until it is, zero.
const NATIVE_START_OFFSET_MS: f64 = 0.0;
/// The native player sends no end event, so a native clip whose envelope never
/// arrives (a failed read or decode) leaves `speaking` after this long rather
/// than holding it until the answer. Word clips run about a second.
const NATIVE_NO_ENVELOPE_END_S: f64 = 2.5;

/// Where the playback clock comes from while speaking.
#[derive(Clone)]
enum Clock {
    /// An `<audio>` element: its own `currentTime` (the playback clock).
    Element(HtmlAudioElement),
    /// The native player: wall clock from when play resolved.
    Since(f64),
    /// A voice with no bytes to read (the device's own voice): no envelope,
    /// the corona pulses gently instead.
    Untracked(f64),
}

struct Speak {
    key: String,
    clock: Clock,
    rate: f64,
    start_ms: f64,
    logged: bool,
}

thread_local! {
    static STATE: Cell<State> = const { Cell::new(State::Idle) };
    static REFUSED: Cell<u32> = const { Cell::new(0) };
    static GEN: Cell<u32> = const { Cell::new(0) };
    static SPEAK: RefCell<Option<Speak>> = const { RefCell::new(None) };
    static ENVS: RefCell<VecDeque<(String, Rc<Vec<f32>>)>> = const { RefCell::new(VecDeque::new()) };
    static LOADING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Reads to try once more if the one in flight comes back empty.
    static RETRY: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static RAF_ON: Cell<bool> = const { Cell::new(false) };
    static REST_TIMER: Cell<i32> = const { Cell::new(0) };
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
    /// AT-8: per word, ms from audio start to the first orb frame above the
    /// envelope threshold, less where the voice itself crosses it.
    static SYNC: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

fn on() -> bool {
    crate::flags::visual_orb()
}

fn now() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0)
}

fn wrap() -> Option<web_sys::HtmlElement> {
    web_sys::window()?.document()?.get_element_by_id(WRAP)?.dyn_into().ok()
}

fn set_attr(name: &str, v: &str) {
    if let Some(w) = wrap() {
        let _ = w.set_attribute(name, v);
    }
}

/// The state the orb is in, for the seam and the report.
pub fn state() -> State {
    STATE.with(Cell::get)
}

/// How many undefined moves were refused this session (0 in a correct build).
pub fn refused() -> u32 {
    REFUSED.with(Cell::get)
}

/// Install once, at boot, when the flag is on: the starting state and the rest
/// listeners.
pub fn init() {
    if !on() || INSTALLED.with(|c| c.replace(true)) {
        return;
    }
    set_attr("data-orb", State::Idle.name());
    install_rest();
}

/// Move the orb. Anything not in `orb::TABLE` panics in debug and is refused,
/// counted and logged in release.
fn go(to: State) {
    if !on() {
        return;
    }
    let from = state();
    let step = if cfg!(debug_assertions) { orb::step(from, to) } else { orb::check(from, to) };
    match step {
        Step::Stayed => {}
        Step::Refused => {
            REFUSED.with(|c| c.set(c.get() + 1));
            web_sys::console::error_1(&format!("orb: undefined transition {} -> {}", from.name(), to.name()).into());
        }
        Step::Moved => {
            STATE.with(|c| c.set(to));
            let g = GEN.with(|c| {
                c.set(c.get().wrapping_add(1));
                c.get()
            });
            if from == State::Speaking {
                SPEAK.with(|s| s.borrow_mut().take());
                if let Some(w) = wrap() {
                    let _ = w.style().remove_property("--orb-env");
                }
            }
            if to == State::Fetching {
                // Drawn only if the clip is still on its way after the grace.
                show_later(g, orb::fetch_grace_ms());
            } else {
                set_attr("data-orb", to.name());
            }
            wake();
        }
    }
}

fn show_later(gen: u32, ms: f64) {
    let Some(win) = web_sys::window() else { return };
    let cb = Closure::once_into_js(move || {
        if GEN.with(Cell::get) == gen && state() == State::Fetching {
            set_attr("data-orb", State::Fetching.name());
        }
    });
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms as i32);
}

// ---------- the audio router's observations (api.rs) ----------

/// A word was asked for.
pub fn requested() {
    go(State::Fetching);
}

/// Every source failed: nothing will be heard, so the orb waits for the answer.
pub fn unheard() {
    if state() == State::Fetching {
        go(State::Awaiting);
    }
}

/// A new `<audio>` element for `src` (server clip, human clip, pack clip):
/// start the side read now, so the envelope is usually ready before the first
/// sound, and follow the element's own start and end.
pub fn watch_element(audio: &HtmlAudioElement, src: &str) {
    if !on() {
        return;
    }
    prepare(src.to_string(), Source::Url(src.to_string()));
    let key = src.to_string();
    let a = audio.clone();
    let playing = Closure::<dyn FnMut()>::new(move || {
        start_speaking(key.clone(), Clock::Element(a.clone()), a.playback_rate());
    });
    let _ = audio.add_event_listener_with_callback("playing", playing.as_ref().unchecked_ref());
    playing.forget();
    let ended = Closure::<dyn FnMut()>::new(|| {
        if state() == State::Speaking {
            go(State::Awaiting);
        }
    });
    let _ = audio.add_event_listener_with_callback("ended", ended.as_ref().unchecked_ref());
    ended.forget();
}

/// The app's native player is about to play `asset_id`: read the cached file.
pub fn prepare_native(asset_id: &str) {
    if on() {
        prepare(asset_id.to_string(), Source::Cached(asset_id.to_string()));
    }
}

/// The native player's play() resolved.
pub fn native_started(asset_id: &str, rate: f64) {
    if on() {
        // On a first play the clip was not on disk when `prepare_native` ran
        // (the player downloads it first), so that read found nothing. It is on
        // disk now; read again. A no-op when the envelope is already in hand.
        let key = asset_id.to_string();
        if LOADING.with(|l| l.borrow().contains(&key)) {
            RETRY.with(|r| r.borrow_mut().push(key.clone()));
        } else {
            prepare(key.clone(), Source::Cached(key.clone()));
        }
        start_speaking(asset_id.to_string(), Clock::Since(now() + NATIVE_START_OFFSET_MS), rate);
    }
}

/// The device's own voice (AVSpeech, Web Speech) started; there are no bytes.
pub fn untracked_started() {
    if on() {
        start_speaking(String::new(), Clock::Untracked(now()), 1.0);
    }
}

/// The device's own voice finished.
pub fn untracked_ended() {
    let untracked = SPEAK.with(|s| matches!(s.borrow().as_ref().map(|s| &s.clock), Some(Clock::Untracked(_))));
    if on() && untracked && state() == State::Speaking {
        go(State::Awaiting);
    }
}

fn start_speaking(key: String, clock: Clock, rate: f64) {
    if state() == State::Speaking {
        // A replay while still speaking: restart the clock, stay in state.
        SPEAK.with(|s| s.borrow_mut().take());
    }
    go(State::Speaking);
    if state() != State::Speaking {
        return;
    }
    let rate = if rate > 0.0 { rate } else { 1.0 };
    SPEAK.with(|s| *s.borrow_mut() = Some(Speak { key, clock, rate, start_ms: now(), logged: false }));
    run_frames();
}

// ---------- the game's verdicts (game.rs) ----------

pub fn verdict(correct: bool) {
    go(if correct { State::Correct } else { State::Wrong });
}

/// A new round, or leaving the play screen.
pub fn idle() {
    go(State::Idle);
}

/// Totality (F7 drives this; nothing in Phase B does).
#[allow(dead_code)]
pub fn milestone() {
    go(State::Milestone);
}

// ---------- the envelope ----------

enum Source {
    Url(String),
    Cached(String),
}

fn cached_env(key: &str) -> Option<Rc<Vec<f32>>> {
    ENVS.with(|e| e.borrow().iter().find(|(k, _)| k == key).map(|(_, v)| Rc::clone(v)))
}

fn prepare(key: String, src: Source) {
    if cached_env(&key).is_some() || LOADING.with(|l| l.borrow().contains(&key)) {
        return;
    }
    LOADING.with(|l| l.borrow_mut().push(key.clone()));
    spawn_local(async move {
        let env = match bytes(&src).await {
            Some(buf) => decode(buf).await,
            None => None,
        };
        LOADING.with(|l| l.borrow_mut().retain(|k| k != &key));
        match env {
            Some(env) => ENVS.with(|e| {
                let mut e = e.borrow_mut();
                e.retain(|(k, _)| k != &key);
                e.push_back((key, Rc::new(env)));
                while e.len() > ENV_CACHE {
                    e.pop_front();
                }
            }),
            None => {
                let again = RETRY.with(|r| {
                    let mut r = r.borrow_mut();
                    let had = r.contains(&key);
                    r.retain(|k| k != &key);
                    had
                });
                if again {
                    prepare(key, src);
                } else {
                    // E-Q2 (a): a failed decode means the orb breathes; say so once.
                    web_sys::console::warn_1(&"orb: no envelope for this clip; the corona breathes instead".into());
                }
            }
        }
    });
}

async fn bytes(src: &Source) -> Option<js_sys::ArrayBuffer> {
    match src {
        Source::Url(url) => {
            let win = web_sys::window()?;
            let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(url)).await.ok()?.dyn_into().ok()?;
            if !resp.ok() {
                return None;
            }
            JsFuture::from(resp.array_buffer().ok()?).await.ok()?.dyn_into().ok()
        }
        Source::Cached(asset_id) => {
            let p = crate::native_audio::read_cached(asset_id)?;
            let v = JsFuture::from(p).await.ok()?;
            v.dyn_into().ok()
        }
    }
}

/// Decode in an OfflineAudioContext: it renders nothing and never touches the
/// audio session, so playback cannot hear it.
async fn decode(buf: js_sys::ArrayBuffer) -> Option<Vec<f32>> {
    if buf.byte_length() == 0 {
        return None;
    }
    let ctx = web_sys::OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 22_050.0).ok()?;
    let ab: web_sys::AudioBuffer = JsFuture::from(ctx.decode_audio_data(&buf).ok()?).await.ok()?.dyn_into().ok()?;
    let samples = ab.get_channel_data(0).ok()?;
    Some(orb::envelope(&samples, ab.sample_rate()))
}

// ---------- the speaking frames ----------

fn run_frames() {
    if RAF_ON.with(|c| c.replace(true)) {
        return;
    }
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);
    *g.borrow_mut() = Some(Closure::new(move |_ts: f64| {
        if state() != State::Speaking || !frame() {
            RAF_ON.with(|c| c.set(false));
            f.borrow_mut().take();
            return;
        }
        if let (Some(win), Some(cb)) = (web_sys::window(), f.borrow().as_ref()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }));
    let first = g.borrow();
    if let (Some(win), Some(cb)) = (web_sys::window(), first.as_ref()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

/// One frame. False stops the loop.
fn frame() -> bool {
    let t_now = now();
    let mut done = false;
    let level = SPEAK.with(|s| {
        let mut s = s.borrow_mut();
        let Some(sp) = s.as_mut() else { return None };
        let env = cached_env(&sp.key);
        let (t, untracked) = match &sp.clock {
            Clock::Element(a) => (a.current_time(), false),
            Clock::Since(t0) => ((t_now - t0) / 1000.0 * sp.rate, false),
            Clock::Untracked(t0) => ((t_now - t0) / 1000.0, true),
        };
        let level = match (&env, untracked) {
            (Some(env), false) => {
                if matches!(sp.clock, Clock::Since(_)) && t > orb::duration(env) + 0.1 {
                    done = true; // the native player sends no end event
                }
                if !sp.logged && orb::level_at(env, t) >= orb::ENVELOPE_THRESHOLD {
                    if let Some(onset) = orb::first_above(env) {
                        sp.logged = true;
                        let started = match &sp.clock {
                            Clock::Since(t0) => *t0,
                            _ => sp.start_ms,
                        };
                        let delay = (t_now - started) - onset * 1000.0 / sp.rate;
                        SYNC.with(|v| v.borrow_mut().push(delay.max(0.0)));
                    }
                }
                orb::level_at(env, t)
            }
            // No envelope (yet, or ever): a gentle pulse, never frozen. A
            // native clip has no end event, so without an envelope it ends on
            // a fixed limit instead of holding `speaking` until the answer.
            _ => {
                if matches!(sp.clock, Clock::Since(_)) && t > NATIVE_NO_ENVELOPE_END_S * sp.rate {
                    done = true;
                }
                0.35 + 0.25 * ((t * std::f64::consts::TAU * 1.6).sin() as f32)
            }
        };
        Some(level)
    });
    let Some(level) = level else { return false };
    if let Some(w) = wrap() {
        let _ = w.style().set_property("--orb-env", &format!("{level:.3}"));
    }
    if done {
        go(State::Awaiting);
        return false;
    }
    true
}

/// AT-8, for the frame report: words measured and the p95 delay.
pub fn sync_report() -> String {
    if !on() {
        return String::new();
    }
    SYNC.with(|v| {
        let mut v = v.borrow().clone();
        if v.is_empty() {
            return format!("orb sync (AT-8): no words measured yet | state {} | refused moves {}\n", state().name(), refused());
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95 = v[((v.len() as f64 * 0.95).ceil() as usize).saturating_sub(1).min(v.len() - 1)];
        format!(
            "orb sync (AT-8): {} words | p95 {:.1} ms | {} (needs 100 words, <= 50 ms) | refused moves {}\n",
            v.len(),
            p95,
            if v.len() >= 100 && p95 <= 50.0 { "PASS" } else if p95 > 50.0 { "FAIL" } else { "so far" },
            refused()
        )
    })
}

// ---------- rest (V-D23) ----------

fn install_rest() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let vis = Closure::<dyn FnMut()>::new(|| apply_rest());
    let _ = doc.add_event_listener_with_callback("visibilitychange", vis.as_ref().unchecked_ref());
    vis.forget();
    let input = Closure::<dyn FnMut()>::new(|| touched());
    for ev in ["pointerdown", "keydown"] {
        let _ = doc.add_event_listener_with_callback_and_bool(ev, input.as_ref().unchecked_ref(), true);
    }
    input.forget();
    touched();
}

fn hidden() -> bool {
    web_sys::window().and_then(|w| w.document()).map(|d| d.hidden()).unwrap_or(false)
}

fn apply_rest() {
    if hidden() {
        set_attr("data-orb-rest", "hidden");
    } else if wrap().and_then(|w| w.get_attribute("data-orb-rest")).as_deref() == Some("hidden") {
        touched();
    }
}

/// Any input wakes the orb and restarts the 30 s count.
fn touched() {
    let Some(win) = web_sys::window() else { return };
    wake();
    let old = REST_TIMER.with(Cell::get);
    if old != 0 {
        win.clear_timeout_with_handle(old);
    }
    let cb = Closure::once_into_js(|| {
        REST_TIMER.with(|c| c.set(0));
        set_attr("data-orb-rest", "idle");
    });
    let id = win
        .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), orb::rest_after_ms() as i32)
        .unwrap_or(0);
    REST_TIMER.with(|c| c.set(id));
}

fn wake() {
    if let Some(w) = wrap() {
        let _ = w.remove_attribute("data-orb-rest");
    }
    if hidden() {
        set_attr("data-orb-rest", "hidden");
    }
}
