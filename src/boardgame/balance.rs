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
    /// Games in which a word queue ran dry and a word was reused (Stretch draws harder tiers faster).
    recycled_games: u64,
}

impl Cell {
    fn median_rounds(&self) -> u32 {
        let mut r = self.rounds.clone();
        r.sort_unstable();
        r[r.len() / 2]
    }
}

/// The two reference humans of A-P13. `None` in `simulate_with` is the flag off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Policy {
    /// Takes the normal move every time.
    Never,
    /// Takes Stretch whenever it raises expected distance: the Stretch word is spelled `STRETCH_DROP`
    /// less accurately, so it pays when `(acc - drop) * (roll + 2) > acc * roll`.
    /// The payload is the accuracy drop on a Stretch word, in thousandths.
    Smart(u64),
}

/// The reference human is 0.20 less accurate one tier up (the spec's section 8 model); a second
/// sensitivity row uses 0.35.
const STRETCH_DROP: u64 = 200;
const STRETCH_DROP_HARD: u64 = 350;

fn simulate(variant: Variant, difficulty: Difficulty, npcs: u8, acc_milli: u64, seed0: u64, games: u64) -> Cell {
    simulate_with(variant, difficulty, npcs, acc_milli, seed0, games, None)
}

fn simulate_with(variant: Variant, difficulty: Difficulty, npcs: u8, acc_milli: u64, seed0: u64, games: u64, stretch: Option<Policy>) -> Cell {
    let mut wins = 0u64;
    let pl = pools_for(variant);
    let mut rounds = Vec::with_capacity(games as usize);
    let mut recycled = 0u64;
    let mut pr = Rng::new(seed0);
    for g in 0..games {
        let cfg = GameConfig::solo(variant, difficulty, npcs, 0, "en", false, exact).with_stretch(stretch.is_some());
        let mut s = new_game(seed0.wrapping_mul(0x9E37_79B9).wrapping_add(g), cfg, pl.clone()).unwrap();
        while s.phase != Phase::Finished {
            let a = if s.is_npc_turn() {
                Action::AdvanceNpc
            } else {
                match s.phase {
                    Phase::AwaitRoll => Action::Roll,
                    Phase::AwaitStretch => {
                        let roll = s.offer.unwrap().roll as u64;
                        Action::ChooseStretch(matches!(stretch, Some(Policy::Smart(d)) if acc_milli.saturating_sub(d) * (roll + 2) > acc_milli * roll))
                    }
                    Phase::AwaitSpelling => {
                        let w = s.pending.as_ref().unwrap().word.clone();
                        let acc_now = if s.pending.as_ref().unwrap().stretch { acc_milli.saturating_sub(match stretch { Some(Policy::Smart(d)) => d, _ => STRETCH_DROP }) } else { acc_milli };
                        if pr.next_u64() % 1000 < acc_now {
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
        assert!(s.recycled == 0 || super::rules::PRIOR_OVERRIDE.with(|c| c.get()).is_some() || stretch.is_some());
        recycled += (s.recycled > 0) as u64;
        wins += (s.winner == Some(0)) as u64;
        rounds.push(s.turns / s.players.len() as u32 + 1);
    }
    Cell { win_rate: wins as f64 / games as f64, rounds, recycled_games: recycled }
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

/// A-P13 / D-P17: Sprint and Full, each NPC difficulty, two reference humans, 20,000 games a cell,
/// every NPC rival playing Stretch at the configured rate and penalty. The never-stretch human
/// must stay within 0.05 BELOW the D18 range (>= lo - 0.05); the stretching human may sit up to
/// 0.15 ABOVE it (<= hi + 0.15). Also prints the stretch-vs-never gap and the share of games that
/// had to reuse a word, so a thin pool shows up. A-P14: if this fails, tune only the NPC rate and
/// penalty (see `stretch_tuning_search`); never D18 or the prior.
#[test]
fn a_p13_stretch_balance_both_reference_humans() {
    let mut ok = true;
    let mut out = format!("A-P13 (Stretch on for everyone, {GAMES_NEW} games per cell, seed {SEED}, 3 NPCs; D18 range, never >= lo-5, smart <= hi+15)\n");
    for v in [Variant::Sprint, Variant::Full] {
        out.push_str(&format!("{v:?}\n"));
        for (name, d, lo, hi) in D18 {
            for (label, acc) in HUMANS {
                let never = simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Never));
                let smart = simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Smart(STRETCH_DROP)));
                let bad_n = never.win_rate < lo - 0.05;
                let bad_s = smart.win_rate > hi + 0.15;
                ok &= !(bad_n || bad_s);
                out.push_str(&format!(
                    "  {name:6} human {label:>4}: never {:5.1}% [>= {:.0}%]{}  smart {:5.1}% [<= {:.0}%]{}  gap {:+.1}  reuse {}/{}\n",
                    never.win_rate * 100.0,
                    (lo - 0.05) * 100.0,
                    if bad_n { " <-- LOW" } else { "" },
                    smart.win_rate * 100.0,
                    (hi + 0.15) * 100.0,
                    if bad_s { " <-- HIGH" } else { "" },
                    (smart.win_rate - never.win_rate) * 100.0,
                    never.recycled_games,
                    smart.recycled_games
                ));
            }
        }
    }
    eprintln!("{out}");
    assert!(ok, "A-P13 FAIL (tune only the NPC Stretch rate and penalty, A-P14):\n{out}");
}

/// A-P14 / D-P21 (run by hand: `cargo test --lib stretch_tuning_search -- --ignored --nocapture`):
/// sweep ONLY the NPC Stretch rate base, rate slope and penalty, 20,000 games per cell, all 18 A-P13
/// cells for both reference humans, and print every setting with the smallest margin to an A-P13
/// limit (negative = a cell is outside). D18 deltas, ranges, the Sprint prior and the bonus are fixed.
#[test]
#[ignore]
fn stretch_tuning_search() {
    use super::rules::STRETCH_OVERRIDE;
    let mut combos = Vec::new();
    for base in [0i64, 25, 50, 75, 100] {
        for slope in [800i64, 900, 1000, 1200, 1500] {
            for pen in [175i64, 200, 225, 250] {
                combos.push((base, slope, pen));
            }
        }
    }
    let handles: Vec<_> = (0..8)
        .map(|k| {
            let chunk: Vec<(i64, i64, i64)> = combos.iter().copied().skip(k).step_by(8).collect();
            std::thread::spawn(move || {
                let mut out = Vec::new();
                for (base, slope, pen) in chunk {
                    STRETCH_OVERRIDE.with(|c| c.set(Some((base, slope, pen))));
                    let mut worst = f64::MAX;
                    let mut worst_at = String::new();
                    for v in [Variant::Sprint, Variant::Full] {
                        for (name, d, lo, hi) in D18 {
                            for (label, acc) in HUMANS {
                                let n = simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Never)).win_rate - (lo - 0.05);
                                let s = (hi + 0.15) - simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Smart(STRETCH_DROP))).win_rate;
                                for (m, kind) in [(n, "never"), (s, "smart")] {
                                    if m < worst {
                                        worst = m;
                                        worst_at = format!("{v:?} {name} {label} {kind}");
                                    }
                                }
                            }
                        }
                    }
                    out.push((base, slope, pen, worst, worst_at));
                }
                out
            })
        })
        .collect();
    let mut all: Vec<_> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    all.sort_by_key(|a| (a.0, a.1, a.2));
    for (b, sl, pen, worst, at) in &all {
        eprintln!("STRETCH base {b} slope {sl} penalty {pen}: margin {:+.1} pts {}  worst cell: {at}", worst * 100.0, if *worst >= 0.0 { "PASS" } else { "fail" });
    }
}

/// Sensitivity (reported, not gated): the smart human at a 0.35 Stretch-accuracy drop.
#[test]
fn a_p13_sensitivity_smart_human_at_a_035_drop() {
    let mut out = format!("A-P13 sensitivity: smart human, Stretch accuracy drop 0.35 ({GAMES_NEW} games per cell)\n");
    for v in [Variant::Sprint, Variant::Full] {
        out.push_str(&format!("{v:?}\n"));
        for (name, d, lo, hi) in D18 {
            for (label, acc) in HUMANS {
                let never = simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Never)).win_rate;
                let smart = simulate_with(v, d, 3, acc, SEED, GAMES_NEW, Some(Policy::Smart(STRETCH_DROP_HARD))).win_rate;
                out.push_str(&format!(
                    "  {name:6} human {label:>4}: never {:5.1}%  smart(0.35) {:5.1}% [<= {:.0}%]{}  gap {:+.1}\n",
                    never * 100.0,
                    smart * 100.0,
                    (hi + 0.15) * 100.0,
                    if smart > hi + 0.15 { " <-- HIGH" } else { "" },
                    (smart - never) * 100.0
                ));
                let _ = lo;
            }
        }
    }
    eprintln!("{out}");
}
