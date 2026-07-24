//! CC-SPELL-RACING — the Spell Racing mode (REVIEW-GATED, built in phases).
//!
//! Phase 0 (`crate::wordid`) gave us stable word IDs. Phase 1 (`format`) is the
//! versioned, free-text-free ghost data model + its load/validate/resolve logic.
//! Later phases add tracks, garage, pace ghosts, the screen, sharing, and Daily.
//!
//! Hard wall from The Climb (D2): nothing under `racing::` may reference Climb or
//! shield modules. Nothing offline-breaking: no network here.
#![allow(dead_code)] // foundation API; consumers arrive in later CC-SPELL-RACING phases

pub mod engine;
pub mod format;
pub mod garage;
pub mod pace;
pub mod screen;
pub mod session;
pub mod track;
