//! A15 -- the cross-target determinism witnesses.
//!
//! Two numbers that must be identical on the test host and on wasm32: a hash of
//! the first 1,000 draws of the stream, and the digest of one complete scripted
//! game. The host test pins them; `tools/boardgame-golden` compiles THIS file
//! (with the rest of the engine) for wasm32 and `scripts/boardgame-wasm-golden.mjs`
//! runs it in node and compares.
//!
//! Self-contained on purpose: an exact-match grader and synthetic pools, so the
//! witness depends on nothing but the engine and `spelldoku::rng`.

use super::*;
use crate::spelldoku::rng::{fnv, Rng};

fn exact(_lang: &str, _kid: bool, typed: &str, word: &str, _t: Tier) -> bool {
    typed == word
}

pub fn rng_1000() -> u64 {
    let mut r = Rng::new(0xB0A4_D6A3);
    let mut bytes = Vec::new();
    for _ in 0..1000 {
        bytes.extend_from_slice(&r.next_u64().to_le_bytes());
    }
    fnv(&bytes)
}

/// The 84-tile Full game: unchanged by Phase B, so its pinned digest is the proof
/// that Full is still bit-for-bit what it was.
pub fn game() -> u64 {
    game_of(Variant::Full, false)
}

/// Phase B's witness: one scripted Sprint game (42 tiles, 3 traps).
pub fn game_sprint() -> u64 {
    game_of(Variant::Sprint, false)
}

/// Phase C's witness: the Sprint game with Stretch on (flag-ON; the two above stay flag-off, which
/// is what keeps A-P9 honest). The script takes Stretch on every other offer.
pub fn game_stretch() -> u64 {
    game_of(Variant::Sprint, true)
}

fn game_of(variant: Variant, stretch: bool) -> u64 {
    let mut pools = TierPools::default();
    for t in Tier::ALL {
        pools.tiers[t.ix()] = (0..variant.cfg().min_pool).map(|i| format!("{}{i:04}", t.name())).collect();
    }
    pools.long_word = (0..30).map(|i| format!("long{i:04}")).collect();
    let cfg = GameConfig::solo(variant, Difficulty::Normal, 3, 1, "en", false, exact).with_stretch(stretch);
    let mut s = new_game(0xB0A4_D6A3, cfg, pools).unwrap();
    // A fixed script with no test-side randomness: every third landing is wrong.
    let mut k = 0u64;
    let mut offers = 0u64;
    while s.phase != Phase::Finished {
        let a = if s.is_npc_turn() {
            Action::AdvanceNpc
        } else {
            match s.phase {
                Phase::AwaitRoll => Action::Roll,
                Phase::AwaitStretch => {
                    offers += 1;
                    Action::ChooseStretch(offers % 2 == 0)
                }
                Phase::AwaitSpelling => {
                    k += 1;
                    let w = s.pending.as_ref().unwrap().word.clone();
                    Action::SubmitSpelling(if k % 3 == 0 { "x".into() } else { w })
                }
                _ => Action::ChooseSwitchTarget(None),
            }
        };
        apply(&mut s, a).unwrap();
    }
    s.digest()
}
