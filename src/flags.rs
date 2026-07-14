//! Compile-/runtime feature flags. Each flag defaults OFF so an unfinished
//! feature ships dark — no entry point, zero behavioral diff — until it's
//! explicitly turned on.
//!
//! (Sibling branches keep their own `flags.rs`; this file is scoped to the
//! online Spell Off branch and only owns that flag.)

/// Async online "Spell Off" — 1v1 head-to-head across friends via a shared,
/// server-owned seed (see `online_spelloff.rs` + `backend/matches.py`).
///
/// OFF by default: while off there is NO "Challenge a friend" entry point and
/// the whole online flow is unreachable, so the build behaves exactly as it did
/// before this branch. Flip to `true` (and deploy the backend) to enable it.
pub fn online_spelloff() -> bool {
    false
}
