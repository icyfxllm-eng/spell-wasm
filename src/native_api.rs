//! CC-GAME-SERVER F6: the surface the native room server calls.
//!
//! Re-exports ONLY. The spec allows the core crate to change by visibility,
//! cfg and feature lines and nothing else, so this file may hold `pub use`
//! lines and comments and must never grow a function body (A6.2).

pub use crate::boardgame::Tier;
pub use crate::boardgame_grade::{citation, grade};
pub use crate::consts::{is_active_lang, BUILTIN_LANGS, TIER_ORDER};
pub use crate::words::tier_for;
pub use crate::profanity::is_blocked;

// A6.3 (informational): the solo path's primitives, so a test can count where
// the shared grader and `game::submit_guess` disagree. Not used by the server.
pub use crate::homophones::accepts as homophone_accepts;
pub use crate::jamo::grade as jamo_grade;
pub use crate::norm::{answer_matches_in, fold_strict};
pub use crate::pinyin::{grade_sandhi_aware, tier_accepts_surface, ToneMode};
pub use crate::variants::accepts as variant_accepts;
pub use crate::zh_sandhi::lookup as sandhi_lookup;
