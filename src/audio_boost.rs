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
