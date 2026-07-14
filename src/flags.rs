//! Central feature-flag registry (Decision D1 — one flags module).
//!
//! Each in-flight feature exposes a single `pub fn -> bool` here. A flag that
//! returns `false` guarantees ZERO behavioral difference from the shipped
//! build: gated code paths must be no-ops when their flag is off. Sibling
//! feature branches (FP2…) register their own flags in this same module.
//!
//! Flags default OFF and are flipped to on (returning `true`) only once the
//! feature's review gate has cleared — for Word Stories that gate is Eric's
//! sign-off on the CC BY-SA attribution approach (Decision D3), so it stays
//! `false` in every build until then.

/// F5 "Word stories" (etymology cards). OFF until the Wiktionary CC BY-SA
/// attribution approach is approved (Decision D3). While off, no etymology
/// card renders and the etymology insight keeps its existing meaning-card slot,
/// so the build behaves exactly as it did before the feature landed.
pub fn word_stories() -> bool {
    false
}
