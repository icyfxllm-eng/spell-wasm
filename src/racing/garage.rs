//! The ghost garage (CC-SPELL-RACING Phase 3, spec F2).
//!
//! Per `(language, tier, circuit)` the garage keeps three slots:
//! - **First Ever** — the first race recorded here, written once and **never**
//!   overwritten, including across app updates (F2 / acceptance #9).
//! - **Personal Best** — fastest completion; ties broken by higher accuracy. A
//!   deterministic rule, no judgment calls.
//! - **Most Recent** — the last race.
//!
//! Local-only, no cloud sync in v1. The pure `Garage` type holds the state and is
//! fully unit-testable; `load`/`save`/`record` are thin `storage` wrappers.
//!
//! **Granularity note (interpretation, flag for Eric):** F2 says "per language ×
//! difficulty tier," but Personal Best is "fastest *clean completion*" — a total
//! time only comparable within a fixed track length. A single PB mixing Sprint (10)
//! and Endurance (40) would be meaningless (Sprint always "wins"). So the key here
//! is `(lang, tier, circuit)`. If strictly per-`(lang,tier)` was intended, this is a
//! one-line key change — surfaced rather than silently chosen.
//!
//! Isolation (D2): no Climb/shield references.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::racing::format::RaceGhost;
use crate::racing::track::Circuit;
use crate::storage;

/// Storage key. Distinct from build-55's `spell_ghost_v1` (the pace-marker best-run),
/// so the new garage never clobbers — and is never clobbered by — that legacy data.
/// Versioned so a future schema change is a deliberate key bump.
const GARAGE_KEY: &str = "spell_racing_garage_v1";

/// Which named slot a stored ghost occupies (for the opponent picker, Phase 5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlotKind {
    FirstEver,
    PersonalBest,
    MostRecent,
}

/// The three slots for one `(language, tier, circuit)`.
#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub struct Slots {
    #[serde(default)]
    pub first_ever: Option<RaceGhost>,
    #[serde(default)]
    pub personal_best: Option<RaceGhost>,
    #[serde(default)]
    pub most_recent: Option<RaceGhost>,
}

/// The whole garage: `"lang|tier|circuit" -> Slots`. `#[serde(default)]` + `Option`
/// fields mean a newer app version adding fields cannot wipe existing slots — a First
/// Ever recorded today survives later updates (F2 / #9). serde ignores unknown fields
/// by default, so an *older* app also tolerates a newer file.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Garage {
    #[serde(default)]
    entries: HashMap<String, Slots>,
}

/// Total time of a completed race: elapsed ms at the final finish.
pub fn total_time_ms(g: &RaceGhost) -> u32 {
    g.events.last().map(|e| e.finish_ms).unwrap_or(0)
}

/// Correct-answer count — the accuracy numerator (denominator is the fixed circuit
/// length, so comparing counts within a circuit compares accuracy).
fn correct_count(g: &RaceGhost) -> usize {
    g.events.iter().filter(|e| e.correct).count()
}

/// F2 Personal-Best rule: lower total time wins; on a tie, higher accuracy wins.
/// Deterministic and total — never a judgment call.
fn beats_pb(candidate: &RaceGhost, current: &RaceGhost) -> bool {
    let (ct, tt) = (total_time_ms(candidate), total_time_ms(current));
    if ct != tt {
        return ct < tt;
    }
    correct_count(candidate) > correct_count(current)
}

impl Garage {
    fn key(lang: &str, tier: &str, circuit: Circuit) -> String {
        format!("{lang}|{tier}|{}", circuit.id())
    }

    /// The slots for one `(lang, tier, circuit)` (empty if none recorded yet).
    pub fn slots(&self, lang: &str, tier: &str, circuit: Circuit) -> Slots {
        self.entries.get(&Self::key(lang, tier, circuit)).cloned().unwrap_or_default()
    }

    /// The opponents available to race here, in pick order. Skips unset slots and
    /// de-dups (e.g. First Ever == Most Recent after a single race shows one).
    pub fn opponents(&self, lang: &str, tier: &str, circuit: Circuit) -> Vec<(SlotKind, RaceGhost)> {
        let s = self.slots(lang, tier, circuit);
        let mut out: Vec<(SlotKind, RaceGhost)> = Vec::new();
        for (kind, g) in [
            (SlotKind::PersonalBest, s.personal_best),
            (SlotKind::FirstEver, s.first_ever),
            (SlotKind::MostRecent, s.most_recent),
        ] {
            if let Some(g) = g {
                if !out.iter().any(|(_, existing)| *existing == g) {
                    out.push((kind, g));
                }
            }
        }
        out
    }

    /// Record a completed race into the three slots (pure; no storage).
    /// - Most Recent: always replaced.
    /// - First Ever: set only if unset — **immutable** thereafter.
    /// - Personal Best: replaced only if the new run strictly beats it.
    pub fn record(&mut self, lang: &str, tier: &str, circuit: Circuit, ghost: RaceGhost) {
        let slot = self.entries.entry(Self::key(lang, tier, circuit)).or_default();
        if slot.first_ever.is_none() {
            slot.first_ever = Some(ghost.clone());
        }
        let replace_pb = match &slot.personal_best {
            None => true,
            Some(pb) => beats_pb(&ghost, pb),
        };
        if replace_pb {
            slot.personal_best = Some(ghost.clone());
        }
        slot.most_recent = Some(ghost);
    }
}

// ---- storage wrappers (thin) ------------------------------------------------

pub fn load() -> Garage {
    storage::get_json(GARAGE_KEY).unwrap_or_default()
}

pub fn save(g: &Garage) {
    storage::set_json(GARAGE_KEY, g);
}

/// Record into the persisted garage (the call the race-finish flow makes).
pub fn record(lang: &str, tier: &str, circuit: Circuit, ghost: RaceGhost) {
    let mut g = load();
    g.record(lang, tier, circuit, ghost);
    save(&g);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::racing::format::{Identity, RaceGhost, WordEvent, SCHEMA_VERSION};

    // A minimal completed run: `n` laps, total finish at `finish`, `correct` right.
    fn ghost(finish: u32, correct: usize, n: usize) -> RaceGhost {
        let events = (0..n)
            .map(|i| WordEvent {
                word_id: i as u64 + 1,
                start_ms: i as u32 * 100,
                finish_ms: if i + 1 == n { finish } else { i as u32 * 100 + 50 },
                keystrokes_ms: vec![10, 20],
                correct: i < correct,
            })
            .collect();
        RaceGhost {
            schema_version: SCHEMA_VERSION,
            language: "en".into(),
            tier: "easy".into(),
            word_list_hash: 0,
            identity: Identity { avatar_id: 0, color_id: 0 },
            events,
        }
    }

    #[test]
    fn first_ever_is_immutable() {
        let mut g = Garage::default();
        let first = ghost(5000, 10, 10);
        g.record("en", "easy", Circuit::Sprint, first.clone());
        // record several more, including a better run
        g.record("en", "easy", Circuit::Sprint, ghost(3000, 10, 10));
        g.record("en", "easy", Circuit::Sprint, ghost(9000, 4, 10));
        let s = g.slots("en", "easy", Circuit::Sprint);
        assert_eq!(s.first_ever, Some(first), "First Ever never changes");
    }

    #[test]
    fn personal_best_is_lowest_time_then_highest_accuracy() {
        let mut g = Garage::default();
        g.record("en", "easy", Circuit::Sprint, ghost(5000, 10, 10));
        g.record("en", "easy", Circuit::Sprint, ghost(3000, 8, 10)); // faster -> new PB
        g.record("en", "easy", Circuit::Sprint, ghost(7000, 10, 10)); // slower -> ignored
        assert_eq!(total_time_ms(g.slots("en", "easy", Circuit::Sprint).personal_best.as_ref().unwrap()), 3000);

        // Tie on time -> higher accuracy wins.
        g.record("en", "easy", Circuit::Sprint, ghost(3000, 10, 10)); // same time, more correct
        let pb = g.slots("en", "easy", Circuit::Sprint).personal_best.unwrap();
        assert_eq!(total_time_ms(&pb), 3000);
        assert_eq!(correct_count(&pb), 10, "tie broken by accuracy");
    }

    #[test]
    fn most_recent_always_updates_and_slots_are_per_circuit() {
        let mut g = Garage::default();
        g.record("en", "easy", Circuit::Sprint, ghost(5000, 10, 10));
        g.record("en", "easy", Circuit::Sprint, ghost(9000, 3, 10));
        assert_eq!(total_time_ms(g.slots("en", "easy", Circuit::Sprint).most_recent.as_ref().unwrap()), 9000);
        // A different circuit is a separate garage entry, untouched.
        assert_eq!(g.slots("en", "easy", Circuit::GrandPrix), Slots::default());
        // ...and a different tier too.
        assert_eq!(g.slots("en", "medium", Circuit::Sprint), Slots::default());
    }

    /// Acceptance #9: a First Ever recorded now survives a later app update. Simulate
    /// an update by serializing, injecting an unknown field a newer version might add,
    /// and deserializing back — First Ever (and all slots) must be intact.
    #[test]
    fn first_ever_survives_a_schema_forward_change() {
        let mut g = Garage::default();
        let first = ghost(4200, 9, 10);
        g.record("en", "easy", Circuit::Sprint, first.clone());
        let json = serde_json::to_string(&g).unwrap();

        // A newer app adds a field the current struct doesn't know about.
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        v.as_object_mut().unwrap().insert("future_field".into(), serde_json::json!({"x": 1}));
        let restored: Garage = serde_json::from_value(v).unwrap();

        assert_eq!(restored.slots("en", "easy", Circuit::Sprint).first_ever, Some(first));
    }

    #[test]
    fn opponents_lists_distinct_slots() {
        let mut g = Garage::default();
        // one race: first_ever == pb == most_recent -> one distinct opponent
        g.record("en", "easy", Circuit::Sprint, ghost(5000, 10, 10));
        assert_eq!(g.opponents("en", "easy", Circuit::Sprint).len(), 1);
        // a faster, less accurate later race -> pb(first run) + most_recent differ,
        // first_ever == pb -> two distinct opponents
        g.record("en", "easy", Circuit::Sprint, ghost(4000, 6, 10));
        let ops = g.opponents("en", "easy", Circuit::Sprint);
        assert_eq!(ops.len(), 2);
    }
}
