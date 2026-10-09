//! A9 -- the calibration, ported from the reference sim (`boardgame_balance.py`,
//! seed 7, 8,000 games per cell) and run against the REAL reducer.
//!
//! The Python sim and this test differ in one respect on purpose: the sim
//! rolled the human's spelling as a coin; here the human is a scripted player
//! who submits the right word or a wrong one, so every rule being measured
//! (traps, switch, skips, the adaptive NPC) is the shipped code. The Rust
//! stream is not Python's, so the numbers move; D18's RANGES are the criteria.
//!
//! If a cell falls outside its range the test fails and prints the table. It
//! does not retune deltas (D18).

use super::tests::{exact, pools};
use super::*;
use crate::spelldoku::rng::Rng;

const GAMES: u64 = 8_000;
const SEED: u64 = 7;

fn human_wins(variant: Variant, difficulty: Difficulty, npcs: u8, acc_milli: u64, seed0: u64) -> f64 {
    let mut wins = 0u64;
    let pl = match variant {
        Variant::Standard => pools(MIN_POOL_STANDARD, LONG_WORD_FLOOR),
        Variant::Jr => pools(MIN_POOL_JR, 0),
    };
    let mut pr = Rng::new(seed0);
    for g in 0..GAMES {
        let cfg = GameConfig::solo(variant, difficulty, npcs, 0, "en", false, exact);
        let mut s = new_game(seed0.wrapping_mul(0x9E37_79B9).wrapping_add(g), cfg, pl.clone()).unwrap();
        while s.phase != Phase::Finished {
            let a = if s.is_npc_turn() {
                Action::AdvanceNpc
            } else {
                match s.phase {
                    Phase::AwaitRoll => Action::Roll,
                    Phase::AwaitSpelling => {
                        let w = s.pending.as_ref().unwrap().word.clone();
                        if pr.next_u64() % 1000 < acc_milli {
                            Action::SubmitSpelling(w)
                        } else {
                            Action::SubmitSpelling(format!("{w}!"))
                        }
                    }
                    _ => {
                        // The sim's rule for everyone: swap with the leader if it is ahead.
                        let me = s.current_seat();
                        let mut lead = 0u8;
                        for i in 0..s.players.len() as u8 {
                            if s.players[i as usize].pos > s.players[lead as usize].pos {
                                lead = i;
                            }
                        }
                        let ahead = s.players[lead as usize].pos > s.players[me as usize].pos;
                        Action::ChooseSwitchTarget(ahead.then_some(lead))
                    }
                }
            };
            apply(&mut s, a).unwrap();
        }
        assert_eq!(s.recycled, 0);
        wins += (s.winner == Some(0)) as u64;
    }
    wins as f64 / GAMES as f64
}

// STOPPED, not tuned (D18). With the spec's formula (3 + hits) / (5 + attempts)
// every Standard cell and Jr 100% land OUTSIDE D18 (human wins 41-73%). The
// reference sim reaches D18 because its prior is `hits, attempts = (3, 2)`,
// i.e. (3 + hits) / (2 + attempts), which starts at 1.5 and is clamped to 0.95.
// The same Rust engine with that denominator passes every cell (Tough/100% is
// 31.5% against the 32% ceiling). Which formula is intended is Eric's call;
// remove the ignore once it is settled and the table is inside D18.
#[test]
#[ignore = "A9 outside D18 with the spec formula (3+h)/(5+a); see comment, awaiting a ruling"]
fn a9_balance_table_is_inside_d18() {
    let mut ok = true;
    let mut out = String::new();
    out.push_str("A9 balance (Rust reducer, 8,000 games per cell, seed 7)\n");
    out.push_str("Standard, 84 tiles, 3 NPCs (D18 range in brackets)\n");
    for (name, d, lo, hi) in [("easy", Difficulty::Easy, 0.50, 0.60), ("normal", Difficulty::Normal, 0.35, 0.45), ("tough", Difficulty::Tough, 0.22, 0.32)] {
        for (label, acc) in [("100%", 1000u64), ("70%", 700), ("50%", 500)] {
            let r = human_wins(Variant::Standard, d, 3, acc, SEED);
            let bad = r < lo || r > hi;
            ok &= !bad;
            out.push_str(&format!("  {name:6} human {label:>4}: {:5.1}%  [{:.0}%-{:.0}%]{}\n", r * 100.0, lo * 100.0, hi * 100.0, if bad { "  <-- OUT OF RANGE" } else { "" }));
        }
    }
    out.push_str("Spell Jr, 40 tiles, 3 NPCs, delta 0.25\n");
    for (label, acc, lo, hi) in [("60%", 600u64, 0.20, 1.0), ("80%", 800, 0.0, 1.0), ("100%", 1000, 0.0, 0.45)] {
        let r = human_wins(Variant::Jr, Difficulty::Normal, 3, acc, SEED);
        let bad = r < lo || r > hi;
        ok &= !bad;
        out.push_str(&format!("  kid {label:>4}: {:5.1}%{}\n", r * 100.0, if bad { "  <-- OUT OF RANGE" } else { "" }));
    }
    let one = human_wins(Variant::Standard, Difficulty::Normal, 1, 1000, SEED);
    out.push_str(&format!("1 NPC sanity: perfect human vs 1 Normal: {:5.1}%\n", one * 100.0));
    eprintln!("{out}");
    assert!(ok, "A9 FAIL: a cell is outside D18. Do not retune; report the table:\n{out}");
}
