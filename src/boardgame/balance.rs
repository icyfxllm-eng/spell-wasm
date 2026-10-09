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

/// The calibrated cells (Full, Jr) keep the 8,000 games and seed they were validated at,
/// so their numbers do not move. New cells run 20,000 (A-P13's count).
const GAMES: u64 = 8_000;
const GAMES_NEW: u64 = 20_000;
const SEED: u64 = 7;

/// Pools at the variant's minimum with exactly `LONG_WORD_FLOOR` long words, as the
/// calibrated cells always ran (the long-word shuffle draws from the seed, so a different
/// pool size would move every later draw and with it the pinned Full numbers).
fn pools_for(v: Variant) -> TierPools {
    pools(v.cfg().min_pool, if v.cfg().has_traps() { LONG_WORD_FLOOR } else { 0 })
}

/// One simulated cell: how often the scripted human wins, and how long games run.
struct Cell {
    win_rate: f64,
    /// The round (1-based) each game ended in; a skipped turn still counts as a turn.
    rounds: Vec<u32>,
}

impl Cell {
    fn median_rounds(&self) -> u32 {
        let mut r = self.rounds.clone();
        r.sort_unstable();
        r[r.len() / 2]
    }
}

fn simulate(variant: Variant, difficulty: Difficulty, npcs: u8, acc_milli: u64, seed0: u64, games: u64) -> Cell {
    let mut wins = 0u64;
    let pl = pools_for(variant);
    let mut rounds = Vec::with_capacity(games as usize);
    let mut pr = Rng::new(seed0);
    for g in 0..games {
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
        // (the prior search sweeps settings that can run games long enough to reuse a word)
        assert!(s.recycled == 0 || super::rules::PRIOR_OVERRIDE.with(|c| c.get()).is_some());
        wins += (s.winner == Some(0)) as u64;
        rounds.push(s.turns / s.players.len() as u32 + 1);
    }
    Cell { win_rate: wins as f64 / games as f64, rounds }
}

fn human_wins(variant: Variant, difficulty: Difficulty, npcs: u8, acc_milli: u64, seed0: u64, games: u64) -> f64 {
    simulate(variant, difficulty, npcs, acc_milli, seed0, games).win_rate
}

const D18: [(&str, Difficulty, f64, f64); 3] = [("easy", Difficulty::Easy, 0.50, 0.60), ("normal", Difficulty::Normal, 0.35, 0.45), ("tough", Difficulty::Tough, 0.22, 0.32)];
const HUMANS: [(&str, u64); 3] = [("100%", 1000), ("70%", 700), ("50%", 500)];

// D27 (v1.2): the formula is (3 + hits) / (2 + attempts), as the sim has it.
#[test]
fn a9_balance_table_is_inside_d18() {
    let mut ok = true;
    let mut out = String::new();
    out.push_str("A9 balance (Rust reducer, 8,000 games per cell, seed 7)\n");
    out.push_str("Standard, 84 tiles, 3 NPCs (D18 range in brackets)\n");
    for (name, d, lo, hi) in D18 {
        for (label, acc) in HUMANS {
            let r = human_wins(Variant::Full, d, 3, acc, SEED, GAMES);
            let bad = r < lo || r > hi;
            ok &= !bad;
            out.push_str(&format!("  {name:6} human {label:>4}: {:5.1}%  [{:.0}%-{:.0}%]{}\n", r * 100.0, lo * 100.0, hi * 100.0, if bad { "  <-- OUT OF RANGE" } else { "" }));
        }
    }
    out.push_str("Spell Jr, 40 tiles, 3 NPCs, delta 0.25\n");
    for (label, acc, lo, hi) in [("60%", 600u64, 0.20, 1.0), ("80%", 800, 0.0, 1.0), ("100%", 1000, 0.0, 0.45)] {
        let r = human_wins(Variant::Jr, Difficulty::Normal, 3, acc, SEED, GAMES);
        let bad = r < lo || r > hi;
        ok &= !bad;
        out.push_str(&format!("  kid {label:>4}: {:5.1}%{}\n", r * 100.0, if bad { "  <-- OUT OF RANGE" } else { "" }));
    }
    let one = human_wins(Variant::Full, Difficulty::Normal, 1, 1000, SEED, GAMES);
    out.push_str(&format!("1 NPC sanity: perfect human vs 1 Normal: {:5.1}%\n", one * 100.0));
    eprintln!("{out}");
    assert!(ok, "A9 FAIL: a cell is outside D18. Do not retune; report the table:\n{out}");
}

/// Phase B baseline: Sprint, 42 tiles, 3 traps, Standard tiers and ladder, every rule
/// flag off (none exist yet), 3 NPCs, 20,000 games per cell. D18's ranges are the pass
/// criteria; if Sprint lands outside them for the existing deltas the deltas are NOT
/// retuned (D18, D-P18): the table is the report.
#[test]
fn a9_sprint_baseline_is_inside_d18() {
    let mut ok = true;
    let mut out = format!("A9 Sprint baseline (Rust reducer, 42 tiles, 3 traps, 3 NPCs, {GAMES_NEW} games per cell, seed {SEED})\n");
    for (name, d, lo, hi) in D18 {
        for (label, acc) in HUMANS {
            let c = simulate(Variant::Sprint, d, 3, acc, SEED, GAMES_NEW);
            let r = c.win_rate;
            let bad = r < lo || r > hi;
            ok &= !bad;
            out.push_str(&format!(
                "  {name:6} human {label:>4}: {:5.1}%  [{:.0}%-{:.0}%]  median {} rounds{}\n",
                r * 100.0,
                lo * 100.0,
                hi * 100.0,
                c.median_rounds(),
                if bad { "  <-- OUT OF RANGE" } else { "" }
            ));
        }
    }
    eprintln!("{out}");
    assert!(ok, "A9 Sprint FAIL: a cell is outside D18. Do not retune; report the table:\n{out}");
}

/// A-P15: the simulated median game length (the round the game ends in; a human of 80%
/// against 3 Normal NPCs, the model's reference setup) is at most 16 rounds for Sprint
/// and 34 for Full. Other accuracies are printed, not gated.
#[test]
fn a_p15_median_game_length() {
    let mut out = String::from("A-P15 median rounds (3 NPCs Normal, 6,000 games per row, seed 7)\n");
    let mut ok = true;
    for (v, cap) in [(Variant::Sprint, 16u32), (Variant::Full, 34)] {
        for (label, acc) in [("100%", 1000u64), ("80%", 800), ("50%", 500)] {
            let m = simulate(v, Difficulty::Normal, 3, acc, SEED, 6_000).median_rounds();
            let gated = acc == 800;
            let bad = gated && m > cap;
            ok &= !bad;
            out.push_str(&format!("  {v:?} human {label:>4}: median {m} rounds{}{}\n", if gated { format!("  (cap {cap})") } else { String::new() }, if bad { "  <-- OVER" } else { "" }));
        }
    }
    eprintln!("{out}");
    assert!(ok, "A-P15 FAIL:\n{out}");
}

/// D-P20 search (run by hand: `cargo test --lib sprint_prior_search -- --ignored --nocapture`).
/// Sweeps Sprint's starting counts over the nine D18 cells at 20,000 games each and prints
/// every combination with its smallest distance to a range edge (negative = outside).
#[test]
#[ignore]
fn sprint_prior_search() {
    use super::rules::PRIOR_OVERRIDE;
    // Fourth pass, in thousandths: around the one pass-zone found in the third.
    let mut combos = Vec::new();
    for ph in (1225..=1400i64).step_by(25) {
        combos.push((ph, 1000));
    }
    for ph in (550..=800i64).step_by(25) {
        combos.push((ph, 500));
    }
    for ph in (850..=1100i64).step_by(25) {
        combos.push((ph, 750));
    }
    let chunks: Vec<Vec<(i64, i64)>> = (0..8).map(|k| combos.iter().copied().skip(k).step_by(8).collect()).collect();
    let handles: Vec<_> = chunks
        .into_iter()
        .map(|chunk| {
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for (ph, pa) in chunk {
                    PRIOR_OVERRIDE.with(|c| c.set(Some((ph, pa))));
                    let mut worst = f64::MAX;
                    let mut cells = Vec::new();
                    for (_, d, lo, hi) in D18 {
                        for (_, acc) in HUMANS {
                            let r = simulate(Variant::Sprint, d, 3, acc, SEED, GAMES_NEW).win_rate;
                            worst = worst.min((r - lo).min(hi - r));
                            cells.push(r);
                        }
                    }
                    out.push((ph, pa, worst, cells));
                }
                out
            })
        })
        .collect();
    let mut all: Vec<_> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    all.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    for (ph, pa, worst, c) in &all {
        let cs: Vec<String> = c.iter().map(|r| format!("{:.1}", r * 100.0)).collect();
        eprintln!("PRIOR {ph}/{pa} (milli): margin {:+.1} pts {} cells E100,E70,E50,N100,N70,N50,T100,T70,T50 = {}", worst * 100.0, if *worst >= 0.0 { "PASS" } else { "fail" }, cs.join(" "));
    }
}
