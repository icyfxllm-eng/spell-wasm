//! Routes word audio through a single shared Web Audio `GainNode` so quiet
//! devices/speakers can be boosted past the browser's normal 100% volume
//! ceiling (native `HtmlAudioElement.volume` caps at 1.0). The `AudioContext`
//! is created lazily and resumed on the first user gesture, since browsers
//! block audio contexts from starting before any interaction.

use std::cell::RefCell;
use web_sys::{AudioContext, GainNode, HtmlAudioElement};

const MIN_GAIN: f32 = 0.5;
const MAX_GAIN: f32 = 2.0;
const DEFAULT_GAIN: f32 = 1.0;

thread_local! {
    static CTX: RefCell<Option<(AudioContext, GainNode)>> = RefCell::new(None);
    static PENDING_GAIN: RefCell<f32> = RefCell::new(DEFAULT_GAIN);
}

pub fn clamp_gain(value: f32) -> f32 {
    value.clamp(MIN_GAIN, MAX_GAIN)
}

pub fn boost_requested() -> bool {
    PENDING_GAIN.with(|g| (*g.borrow() - DEFAULT_GAIN).abs() > 0.01)
}

/// Creates the shared `AudioContext`/`GainNode` on first use. Never called
/// at all unless a boost is actually requested (see `boost_requested`) —
/// several privacy-hardened browsers treat AudioContext creation itself as
/// worth interfering with for anti-fingerprinting purposes, so players who
/// never touch the volume slider should never have one created on their
/// behalf in the first place, not just avoid having their audio routed
/// through it.
fn ensure_ctx() -> Option<(AudioContext, GainNode)> {
    CTX.with(|c| {
        let mut slot = c.borrow_mut();
        if slot.is_none() {
            let ctx = AudioContext::new().ok()?;
            let gain = ctx.create_gain().ok()?;
            gain.gain().set_value(PENDING_GAIN.with(|g| *g.borrow()));
            let _ = gain.connect_with_audio_node(&ctx.destination());
            *slot = Some((ctx, gain));
        }
        slot.clone()
    })
}

/// Call on the first user gesture (e.g. pointerdown on the orb) — resumes
/// the context if the browser started it suspended. No-ops entirely unless
/// a boost is actually requested.
pub fn unlock() {
    if !boost_requested() {
        return;
    }
    if let Some((ctx, _)) = ensure_ctx() {
        if ctx.state() == web_sys::AudioContextState::Suspended {
            let _ = ctx.resume();
        }
    }
}

/// Sets the shared gain (clamped to 0.5-2.0). Safe to call before the
/// context exists — the value is applied once it's created. Setting it
/// back to the default doesn't tear down an already-created context (no
/// need to — `wire()` just stops using it), but also doesn't create one.
pub fn set_gain(value: f32) {
    let value = clamp_gain(value);
    PENDING_GAIN.with(|g| *g.borrow_mut() = value);
    if !boost_requested() {
        return;
    }
    if let Some((_, gain)) = ensure_ctx() {
        gain.gain().set_value(value);
    }
}

/// CC-FINALE D6 (Eric, 2026-07-31): one soft chime, respecting sound
/// settings, no fanfare loop. Two sine partials a fifth-plus-octave apart,
/// a 20ms attack so there's no click, exponential decay, everything silent
/// inside a second. The numbers ARE the sign-off — `chime_tests` pins them
/// to the "soft" bounds so the chime can't quietly grow into a jingle.
const CHIME_PARTIALS: [(f32, f32, f64); 2] = [(880.0, 0.10, 0.9), (1320.0, 0.05, 0.6)];
const CHIME_ATTACK: f64 = 0.02;

/// Plays the completion chime. Fire-and-forget: any failure is silence.
///
/// This is the one caller that creates the shared `AudioContext` without a
/// boost being requested. The anti-fingerprinting caution in `ensure_ctx`
/// is about web browsers — and the only caller of this function is the
/// picture reveal, which is compiled out of the web build entirely, so a
/// browser never reaches here. Routing through the shared gain node is the
/// "respecting sound settings" clause: the volume slider applies to the
/// chime exactly as it does to word audio.
pub fn chime() {
    let Some((ctx, gain)) = ensure_ctx() else { return };
    if ctx.state() == web_sys::AudioContextState::Suspended {
        // The keystroke that completed the picture was a user gesture, so
        // the browser will honor this.
        let _ = ctx.resume();
    }
    let now = ctx.current_time();
    for (freq, amp, dur) in CHIME_PARTIALS {
        let Ok(osc) = ctx.create_oscillator() else { continue };
        let Ok(env) = ctx.create_gain() else { continue };
        osc.frequency().set_value(freq);
        let g = env.gain();
        // Exponential ramps reject zero; start from just-above and land there.
        let _ = g.set_value_at_time(0.0001, now);
        let _ = g.linear_ramp_to_value_at_time(amp, now + CHIME_ATTACK);
        let _ = g.exponential_ramp_to_value_at_time(0.0001, now + dur);
        let _ = osc.connect_with_audio_node(&env);
        let _ = env.connect_with_audio_node(&gain);
        let _ = osc.start_with_when(now);
        let _ = osc.stop_with_when(now + dur + 0.05);
    }
}

/// Routes a freshly-created `<audio>` element through the shared gain node
/// — but only when a boost is actually requested (gain != 100%). Once an
/// element is tapped into a Web Audio graph, its normal direct-to-speakers
/// output is disconnected entirely; audio only reaches the speakers via
/// whatever that graph routes it to. Several privacy-hardened browsers
/// (confirmed: DuckDuckGo's) deliberately neuter or mute AudioContext to
/// block audio-fingerprinting, which produces exactly this: the element
/// "plays" (no error, currentTime advances) but nothing comes out, because
/// the graph it's now solely routed through is a dead end. Skipping this
/// entirely at the default gain means plain <audio> playback — which works
/// everywhere — is never put at risk for the vast majority of players who
/// never touch the volume slider; only those who explicitly opt into a
/// boost take on this compatibility risk.
///
/// Must be called at most once per element (Web Audio forbids tapping the
/// same media element into a graph twice), which holds here since callers
/// only ever invoke this right after constructing a new `HtmlAudioElement`.
pub fn wire(audio: &HtmlAudioElement) {
    if !boost_requested() {
        return;
    }
    let Some((ctx, gain)) = ensure_ctx() else { return };
    if let Ok(source) = ctx.create_media_element_source(audio) {
        let _ = source.connect_with_audio_node(&gain);
    }
}

#[cfg(test)]
mod chime_tests {
    use super::{CHIME_ATTACK, CHIME_PARTIALS};

    /// D6 as bounds. "Soft": no partial above 0.12 against word audio's
    /// 1.0. "No fanfare loop": every partial dead inside one second and
    /// nothing retriggers. The attack floor keeps the strike from being a
    /// click, which is the other way a chime stops being soft.
    #[test]
    fn soft_means_soft_and_short_means_short() {
        assert_eq!(CHIME_PARTIALS.len(), 2, "a chime, not a chord");
        for (freq, amp, dur) in CHIME_PARTIALS {
            assert!((200.0..=4000.0).contains(&freq), "audible, not shrill");
            assert!(amp <= 0.12, "soft means soft: {amp}");
            assert!(dur <= 1.0, "no fanfare: {dur}s");
            assert!(dur > CHIME_ATTACK, "envelope must outlive its attack");
        }
        assert!(CHIME_ATTACK >= 0.01, "no click");
    }
}
