//! CC-VISUAL-ECLIPSE F5 -- the game sounds, behind `visual.sound`.
//!
//! Synthesized at runtime from oscillators and envelopes: no audio asset, no
//! licence, no payload. One key (C major pentatonic), every sound 400 ms or
//! less and loudness-matched by its amplitude table. They play through the one
//! shared AudioContext (`audio_boost` owns it) on their own gain node, so the
//! Game sounds level never moves the voice and the voice volume never moves
//! them (AT-16).
//!
//! The voice comes first (F5.3): a sound is never started while the word is
//! being spoken -- it is dropped, not queued -- and a sound already playing
//! when the voice starts fades out within 25 ms. [`Bus`] is that rule, pure, so
//! AT-15 can run 500 scripted events against it.
//!
//! `correct` climbs one pentatonic step per consecutive correct answer, capped
//! at step 8, and a miss resets it.

use std::cell::{Cell, RefCell};

/// The sound set (F5.2). The Typewriter `keystrike` arrives with its theme (F9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    Tap,
    /// The step on the climb, 0..=8.
    Correct(u8),
    Wrong,
    Milestone,
    Unlock,
    /// The Typewriter theme's key sound (F9): a dry click and a soft thunk.
    Keystrike,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Sine,
    Triangle,
}

/// One note of a sound: start offset and length in seconds, peak amplitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub freq: f32,
    pub at: f64,
    pub dur: f64,
    pub amp: f32,
    pub wave: Wave,
}

/// C major pentatonic from C4: C D E G A, five octaves' worth by index.
pub fn penta(i: usize) -> f32 {
    const ROOT_C4: f32 = 261.63;
    const STEPS: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
    let oct = (i / 5) as f32;
    ROOT_C4 * 2f32.powf(oct + STEPS[i % 5] / 12.0)
}

/// The climb's top step.
pub const MAX_STEP: u8 = 8;

/// The notes of a sound. Index 5 is C5.
pub fn notes(s: Sound) -> Vec<Note> {
    use Wave::*;
    let n = |i: usize, at: f64, dur: f64, amp: f32, wave: Wave| Note { freq: penta(i), at, dur, amp, wave };
    match s {
        Sound::Tap => vec![n(9, 0.0, 0.05, 0.22, Sine)],
        Sound::Correct(step) => {
            let k = 5 + step.min(MAX_STEP) as usize;
            vec![n(k, 0.0, 0.12, 0.28, Triangle), n(k + 2, 0.09, 0.16, 0.28, Triangle)]
        }
        // Soft and low, two notes down: E4 then C4.
        Sound::Wrong => vec![n(2, 0.0, 0.14, 0.26, Triangle), n(0, 0.12, 0.18, 0.24, Triangle)],
        Sound::Milestone => vec![n(5, 0.0, 0.4, 0.16, Sine), n(7, 0.0, 0.4, 0.14, Sine), n(8, 0.0, 0.4, 0.14, Sine), n(10, 0.0, 0.4, 0.1, Sine)],
        Sound::Unlock => vec![n(8, 0.0, 0.15, 0.22, Triangle), n(10, 0.08, 0.15, 0.22, Triangle), n(12, 0.16, 0.15, 0.22, Triangle)],
        Sound::Keystrike => vec![n(17, 0.0, 0.025, 0.14, Triangle), n(0, 0.0, 0.04, 0.12, Sine)],
    }
}

/// How long a sound lasts, in seconds.
pub fn length(s: Sound) -> f64 {
    notes(s).iter().map(|n| n.at + n.dur).fold(0.0, f64::max)
}

/// The fade when the voice starts over a sound (F5.3: within 30 ms).
pub const FADE_S: f64 = 0.025;

/// The voice-priority rule, pure. Times are seconds on any one clock.
#[derive(Default, Debug)]
pub struct Bus {
    voice: bool,
    /// When the current sound stops being audible.
    until: f64,
}

impl Bus {
    /// May a sound of `len` seconds start at `now`? Dropped while the voice speaks.
    pub fn request(&mut self, now: f64, len: f64) -> bool {
        if self.voice {
            return false;
        }
        self.until = self.until.max(now + len);
        true
    }

    /// The voice started (`true`) or stopped. Returns when a sound still
    /// playing must be silent by, if one is.
    pub fn voice(&mut self, active: bool, now: f64) -> Option<f64> {
        self.voice = active;
        if active && self.until > now {
            self.until = now + FADE_S;
            return Some(self.until);
        }
        None
    }

    #[cfg(test)]
    pub fn audible(&self, now: f64) -> bool {
        self.until > now
    }
}

/// The climb: the step for the next correct answer, then advance.
#[derive(Default, Debug)]
pub struct Climb(u8);

impl Climb {
    pub fn correct(&mut self) -> u8 {
        let s = self.0;
        self.0 = (self.0 + 1).min(MAX_STEP);
        s
    }
    pub fn miss(&mut self) {
        self.0 = 0;
    }
}

/// Game sounds are on by default at 40% (V-D16), and off in Education Edition.
pub fn default_on(education: bool) -> bool {
    !education
}
pub const DEFAULT_LEVEL: f32 = 0.4;

// ---------- the web side ----------

thread_local! {
    static ON: Cell<bool> = const { Cell::new(true) };
    static LEVEL: Cell<f32> = const { Cell::new(DEFAULT_LEVEL) };
    static BUS: RefCell<Bus> = RefCell::new(Bus::default());
    static CLIMB: RefCell<Climb> = RefCell::new(Climb::default());
    static OUT: RefCell<Option<web_sys::GainNode>> = const { RefCell::new(None) };
    /// The envelopes of the sound playing now, so the voice can fade them.
    static LIVE: RefCell<Vec<web_sys::GainNode>> = const { RefCell::new(Vec::new()) };
    static PLAYED: Cell<u32> = const { Cell::new(0) };
    static DROPPED: Cell<u32> = const { Cell::new(0) };
    static VOICE_TIMER: Cell<i32> = const { Cell::new(0) };
}

fn flag() -> bool {
    crate::flags::visual_sound()
}

/// The Game sounds toggle and level, from settings.
pub fn configure(on: bool, level: f32) {
    ON.with(|c| c.set(on));
    LEVEL.with(|c| c.set(level.clamp(0.0, 1.0)));
    OUT.with(|o| {
        if let Some(g) = o.borrow().as_ref() {
            g.gain().set_value(level.clamp(0.0, 1.0));
        }
    });
}

/// Sounds played and dropped this session (the seam and AT-15 in the browser).
pub fn counts() -> (u32, u32) {
    (PLAYED.with(Cell::get), DROPPED.with(Cell::get))
}

fn clock() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now() / 1000.0).unwrap_or(0.0)
}

fn out() -> Option<(web_sys::AudioContext, web_sys::GainNode)> {
    let ctx = crate::audio_boost::context()?;
    if ctx.state() == web_sys::AudioContextState::Suspended {
        let _ = ctx.resume();
    }
    let g = OUT.with(|o| o.borrow().clone());
    let g = match g {
        Some(g) => g,
        None => {
            let g = ctx.create_gain().ok()?;
            g.gain().set_value(LEVEL.with(Cell::get));
            g.connect_with_audio_node(&ctx.destination()).ok()?;
            OUT.with(|o| *o.borrow_mut() = Some(g.clone()));
            g
        }
    };
    Some((ctx, g))
}

/// Play `s` if the flag, the toggle and the voice allow it.
pub fn play(s: Sound) {
    if !flag() || !ON.with(Cell::get) || LEVEL.with(Cell::get) <= 0.0 {
        return;
    }
    let len = length(s);
    if !BUS.with(|b| b.borrow_mut().request(clock(), len)) {
        DROPPED.with(|c| c.set(c.get() + 1));
        return;
    }
    let Some((ctx, out)) = out() else { return };
    let t0 = ctx.current_time();
    let mut live = Vec::new();
    for note in notes(s) {
        let (Ok(osc), Ok(env)) = (ctx.create_oscillator(), ctx.create_gain()) else { continue };
        osc.set_type(match note.wave {
            Wave::Sine => web_sys::OscillatorType::Sine,
            Wave::Triangle => web_sys::OscillatorType::Triangle,
        });
        osc.frequency().set_value(note.freq);
        let g = env.gain();
        let start = t0 + note.at;
        let _ = g.set_value_at_time(0.0001, t0);
        let _ = g.set_value_at_time(0.0001, start);
        let _ = g.linear_ramp_to_value_at_time(note.amp, start + 0.008);
        let _ = g.exponential_ramp_to_value_at_time(0.0001, start + note.dur);
        let _ = osc.connect_with_audio_node(&env);
        let _ = env.connect_with_audio_node(&out);
        let _ = osc.start_with_when(start);
        let _ = osc.stop_with_when(start + note.dur + 0.02);
        live.push(env);
    }
    LIVE.with(|l| *l.borrow_mut() = live);
    PLAYED.with(|c| c.set(c.get() + 1));
}

/// The base game's verdict: the climb on a correct answer, a reset on a miss.
pub fn verdict(correct: bool) {
    if correct {
        let step = CLIMB.with(|c| c.borrow_mut().correct());
        play(Sound::Correct(step));
    } else {
        CLIMB.with(|c| c.borrow_mut().miss());
        play(Sound::Wrong);
    }
}

pub fn tap() {
    play(Sound::Tap);
}

/// A keystroke, heard only in the Typewriter theme (F9).
pub fn keystrike() {
    if crate::flags::visual_themes() && crate::themes::current() == crate::themes::Theme::Typewriter {
        play(Sound::Keystrike);
    }
}

/// The tap sound on the chrome a player presses to go somewhere: the menu
/// button, the setup bar, and every primary button except Check spelling
/// (whose press is answered by the verdict sound instead). One delegated
/// listener; it does nothing while the flag is off.
pub fn install_taps() {
    use wasm_bindgen::{closure::Closure, JsCast};
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if !flag() {
            return;
        }
        let hit = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".burger, .setup-chip, .btn-check:not(#checkBtn)").ok().flatten());
        if hit.is_some() {
            tap();
        }
    });
    let _ = doc.add_event_listener_with_callback_and_bool("click", cb.as_ref().unchecked_ref(), true);
    cb.forget();
}

/// An unlock usually lands on the answer that earned it, so it waits for the
/// correct sound to finish (that one is at most 250 ms long).
pub fn unlock() {
    use wasm_bindgen::{closure::Closure, JsCast};
    if !flag() {
        return;
    }
    let cb = Closure::once_into_js(|| play(Sound::Unlock));
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), UNLOCK_AFTER_MS);
    }
}

const UNLOCK_AFTER_MS: i32 = 380;

#[allow(dead_code)]
pub fn milestone() {
    play(Sound::Milestone);
}

// ---------- the voice (the audio router's observations, api.rs) ----------

/// The longest a voice may hold the game sounds off without saying it ended.
/// A voice that fails silently (no end, no error) must not mute them for the
/// rest of the session; a spoken word is a second or two.
const VOICE_WATCHDOG_MS: f64 = 4000.0;

/// The word started sounding: fade anything playing within 25 ms.
pub fn voice_started() {
    voice_started_for(VOICE_WATCHDOG_MS);
}

/// As `voice_started`, ending on its own after `ms` unless told sooner.
fn voice_started_for(ms: f64) {
    use wasm_bindgen::{closure::Closure, JsCast};
    if !flag() {
        return;
    }
    if let Some(win) = web_sys::window() {
        let old = VOICE_TIMER.with(Cell::get);
        if old != 0 {
            win.clear_timeout_with_handle(old);
        }
        let cb = Closure::once_into_js(voice_ended);
        let id = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms as i32).unwrap_or(0);
        VOICE_TIMER.with(|c| c.set(id));
    }
    let now = clock();
    if BUS.with(|b| b.borrow_mut().voice(true, now)).is_some() {
        if let Some(ctx) = crate::audio_boost::context() {
            let t = ctx.current_time();
            LIVE.with(|l| {
                for env in l.borrow().iter() {
                    let g = env.gain();
                    let _ = g.cancel_scheduled_values(t);
                    let _ = g.set_value_at_time(g.value().max(0.0001), t);
                    let _ = g.exponential_ramp_to_value_at_time(0.0001, t + FADE_S);
                }
            });
        }
    }
}

pub fn voice_ended() {
    if flag() {
        BUS.with(|b| b.borrow_mut().voice(false, clock()));
    }
}

/// A word clip on an `<audio>` element: follow its own start and end.
pub fn watch_voice(audio: &web_sys::HtmlAudioElement) {
    use wasm_bindgen::{closure::Closure, JsCast};
    if !flag() {
        return;
    }
    let start = Closure::<dyn FnMut()>::new(voice_started);
    let _ = audio.add_event_listener_with_callback("playing", start.as_ref().unchecked_ref());
    start.forget();
    for ev in ["ended", "pause", "error"] {
        let end = Closure::<dyn FnMut()>::new(voice_ended);
        let _ = audio.add_event_listener_with_callback(ev, end.as_ref().unchecked_ref());
        end.forget();
    }
}

/// The native player gives no end event: hold the voice for a word's length.
pub fn voice_started_native(rate: f64) {
    voice_started_for(1500.0 / rate.max(0.3));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_400_ms_or_less() {
        for s in [Sound::Tap, Sound::Correct(0), Sound::Correct(8), Sound::Wrong, Sound::Milestone, Sound::Unlock, Sound::Keystrike] {
            assert!(length(s) <= 0.4 + 1e-9, "{s:?} lasts {}", length(s));
        }
    }

    #[test]
    fn every_note_is_in_c_major_pentatonic() {
        let pcs = [0, 2, 4, 7, 9];
        for s in [Sound::Tap, Sound::Correct(3), Sound::Correct(8), Sound::Wrong, Sound::Milestone, Sound::Unlock, Sound::Keystrike] {
            for n in notes(s) {
                let semis = (12.0 * (n.freq / 261.63).log2()).round() as i32;
                assert!(pcs.contains(&semis.rem_euclid(12)), "{s:?} has {} Hz", n.freq);
            }
        }
    }

    #[test]
    fn loudness_is_matched_within_a_band() {
        // Peak summed amplitude, a cheap stand-in for loudness: no sound may be
        // twice another.
        let peak = |s: Sound| notes(s).iter().filter(|n| n.at == 0.0).map(|n| n.amp).sum::<f32>().max(notes(s)[0].amp);
        let all: Vec<f32> = [Sound::Tap, Sound::Correct(0), Sound::Wrong, Sound::Milestone, Sound::Unlock].iter().map(|s| peak(*s)).collect();
        let (lo, hi) = (all.iter().cloned().fold(9.0, f32::min), all.iter().cloned().fold(0.0, f32::max));
        assert!(hi / lo <= 2.6, "{all:?}");
    }

    #[test]
    fn wrong_is_low_and_goes_down() {
        let n = notes(Sound::Wrong);
        assert!(n[0].freq > n[1].freq && n[0].freq < 400.0);
    }

    #[test]
    fn correct_climbs_one_step_per_answer_capped_at_eight_and_resets_on_a_miss() {
        let mut c = Climb::default();
        let steps: Vec<u8> = (0..12).map(|_| c.correct()).collect();
        assert_eq!(steps, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8]);
        c.miss();
        assert_eq!(c.correct(), 0);
        let f = |s: u8| notes(Sound::Correct(s))[0].freq;
        assert!(f(1) > f(0) && f(8) > f(7));
        assert_eq!(f(8), f(12), "capped");
    }

    #[test]
    fn game_sounds_default_on_at_40_percent_and_off_in_education() {
        assert!(default_on(false));
        assert!(!default_on(true));
        assert_eq!(DEFAULT_LEVEL, 0.4);
    }

    #[test]
    fn a_sound_is_dropped_while_the_voice_speaks() {
        let mut b = Bus::default();
        b.voice(true, 0.0);
        assert!(!b.request(0.1, 0.2));
        b.voice(false, 1.0);
        assert!(b.request(1.1, 0.2));
    }

    #[test]
    fn the_voice_fades_a_playing_sound_within_30_ms() {
        let mut b = Bus::default();
        assert!(b.request(0.0, 0.4));
        let silent_by = b.voice(true, 0.1).expect("a sound was playing");
        assert!(silent_by - 0.1 <= 0.030);
        assert!(!b.audible(0.1 + 0.031));
    }

    // AT-15: over 500 scripted events, zero overlaps between a UI sound and TTS.
    #[test]
    fn five_hundred_scripted_events_never_overlap_the_voice() {
        let mut b = Bus::default();
        let mut seed: u64 = 0x5eed_1234;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let sounds = [Sound::Tap, Sound::Correct(2), Sound::Wrong, Sound::Milestone, Sound::Unlock];
        let (mut now, mut voice, mut started, mut dropped) = (0.0f64, false, 0, 0);
        let mut voice_from = 0.0f64;
        for _ in 0..500 {
            now += (rnd() % 400) as f64 / 1000.0;
            match rnd() % 3 {
                0 => {
                    voice = !voice;
                    if voice {
                        voice_from = now;
                    }
                    b.voice(voice, now);
                }
                _ => {
                    let s = sounds[(rnd() % 5) as usize];
                    if b.request(now, length(s)) {
                        started += 1;
                        assert!(!voice, "a sound started over the voice at {now}");
                    } else {
                        dropped += 1;
                        assert!(voice, "a sound was dropped with no voice at {now}");
                    }
                }
            }
            // Whatever is audible while the voice speaks is only the fade tail.
            if voice && b.audible(now) {
                assert!(now - voice_from <= FADE_S + 1e-9, "a sound outlived its fade at {now}");
            }
        }
        assert!(started > 50 && dropped > 20, "the script exercised too little: {started} started, {dropped} dropped");
    }
}
