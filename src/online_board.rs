//! CC-BOARD-GAME-ONLINE F6/F7 (client): the server's BoardGame, in the shape
//! the solo screen draws.
//!
//! Pure: no DOM, no network. The server owns the game; this file turns what it
//! says into a `BoardGameState` that `boardgame_screen` can render, and into
//! the same `Event`s the solo game produces, so the board, the pieces' walks
//! and the chips are the solo screen's own.
//!
//! THE RULE THIS FILE KEEPS. The state built here is a display mirror. Nothing
//! ever hands it to `boardgame::apply`: it has no words, no word queues and no
//! dice, and every number on it came from the server. Where a field cannot be
//! derived from what arrived (a streak count mid-turn), the next phase's
//! `table` payload sets it right.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::Value;

use crate::boardgame::{
    Board, BoardGameState, Boost, Difficulty, Event, GameConfig, Offer, Phase, Player, Seat, Spell, SpellKind, Tier, TierPools, Trap, Variant,
};

pub const MODE: &str = "boardgame";
pub const MODE_VERSION: u32 = 1;
/// The turn engine's protocol (CC-ONLINE-TURN).
pub const TURN_PROTOCOL: u32 = 2;

pub const AWAITING_SPELL: &str = "awaiting_spell";
pub const AWAITING_CHOICE: &str = "awaiting_choice";

fn variant(size: &str) -> Option<Variant> {
    match size {
        "full" => Some(Variant::Full),
        "sprint" => Some(Variant::Sprint),
        "jr" => Some(Variant::Jr),
        _ => None,
    }
}

pub fn tier(name: &str) -> Option<Tier> {
    Tier::ALL.into_iter().find(|t| t.name() == name)
}

/// The server's trap names (server/spell-server/src/boardgame.rs `trap_name`).
fn trap(name: &str) -> Option<Trap> {
    Some(match name {
        "start" => Trap::BackToStart,
        "long" => Trap::LongWord,
        "lose" => Trap::LoseRoll,
        "switch" => Trap::SwitchTiles,
        "double" => Trap::DoubleExpert,
        _ => return None,
    })
}

fn boost(name: &str) -> Option<Boost> {
    Some(match name {
        "tailwind" => Boost::Tailwind,
        "extra_roll" => Boost::ExtraRoll,
        "ward" => Boost::Ward,
        _ => return None,
    })
}

fn u(v: &Value, k: &str) -> Option<u64> {
    v.get(k).and_then(Value::as_u64)
}

fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// The grader a display state carries. Never called: nothing grades here.
fn no_grade(_: &str, _: bool, _: &str, _: &str, _: Tier) -> bool {
    false
}

fn reveal(st: &mut BoardGameState, tile: u32, trap: Option<Trap>, boost: Option<Boost>) {
    let i = tile as usize;
    if let (Some(t), Some(slot)) = (trap, st.board.traps.get_mut(i)) {
        *slot = Some(t);
    }
    if let (Some(x), Some(slot)) = (boost, st.board.boosts.get_mut(i)) {
        *slot = Some(x);
    }
    if (trap.is_some() || boost.is_some()) && !st.revealed.contains(&tile) {
        st.revealed.push(tile);
    }
}

/// How the server's seats sit at the mirrored table. The state itself is the
/// screen's (`Ui::game`); every method here takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Mirror {
    /// `table[i]` is the wire seat of rules seat `i`.
    pub table: Vec<u8>,
    /// Which seats the server plays (an NPC from the start, or a seat it took over).
    pub npc: Vec<bool>,
}

impl Mirror {
    /// From a `room_state.mode_state.view` (or a `game.end` summary).
    pub fn from_view(view: &Value, lang: &str, kid: bool) -> Option<(Mirror, BoardGameState)> {
        let variant = variant(view.get("size")?.as_str()?)?;
        let tiers: Vec<Option<Tier>> = view.get("tiers")?.as_array()?.iter().map(|t| t.as_str().and_then(tier)).collect();
        let n = tiers.len();
        if n < 2 {
            return None;
        }
        let opts = view.get("options").cloned().unwrap_or(Value::Null);
        let (stretch, boosts, streak) = (b(&opts, "stretch"), b(&opts, "boosts"), b(&opts, "streak"));
        let c = variant.cfg();
        let board = Board {
            tiers,
            traps: if c.has_traps() { vec![None; n] } else { Vec::new() },
            boosts: if boosts { vec![None; n] } else { Vec::new() },
            grid: c.grid,
        };
        let seats_json = view.get("seats")?.as_array()?;
        let table: Vec<u8> = seats_json.iter().filter_map(|s| u(s, "seat").map(|x| x as u8)).collect();
        if table.len() != seats_json.len() || !(2..=4).contains(&table.len()) {
            return None;
        }
        let seats: Vec<Seat> = seats_json.iter().map(|s| Seat { piece: u(s, "piece").unwrap_or(0) as u8, npc: b(s, "npc") }).collect();
        let players = seats
            .iter()
            .map(|&seat| Player {
                seat,
                pos: 0,
                skip: false,
                rolls: 0,
                attempts: 0,
                hits: 0,
                spelled: Vec::new(),
                missed: Vec::new(),
                stretch_offered: 0,
                stretch_taken: 0,
                stretch_hits: 0,
                ward: false,
                streak: 0,
                bonus: false,
            })
            .collect();
        let cfg = GameConfig::mixed_table(variant, Difficulty::Normal, seats.clone(), lang, kid, no_grade)
            .with_stretch(stretch)
            .with_boosts(boosts)
            .with_streak(streak);
        let state = BoardGameState {
            cfg,
            board,
            players,
            order: (0..table.len() as u8).collect(),
            turn: 0,
            phase: Phase::AwaitRoll,
            pending: None,
            offer: None,
            effect_used: false,
            rng: crate::spelldoku::rng::Rng::new(0),
            pools: Arc::new(TierPools::default()),
            queues: Default::default(),
            long_queue: Vec::new(),
            imp_queue: Vec::new(),
            used: BTreeSet::new(),
            recycled: 0,
            drawn: Vec::new(),
            revealed: Vec::new(),
            events: Vec::new(),
            winner: None,
            turns: 0,
        };
        let npc = seats.iter().map(|s| s.npc).collect();
        let mut m = Mirror { table, npc };
        let mut state = state;
        m.set_table(&mut state, view);
        if let Some(w) = u(view, "winner").and_then(|w| m.index(w as u8)) {
            state.winner = Some(w);
            state.phase = Phase::Finished;
        }
        Some((m, state))
    }

    pub fn index(&self, wire: u8) -> Option<u8> {
        self.table.iter().position(|s| *s == wire).map(|i| i as u8)
    }

    pub fn wire(&self, index: u8) -> u8 {
        self.table.get(index as usize).copied().unwrap_or(index)
    }

    /// The server's word on every seat and every revealed tile (a `table`
    /// payload, or a view). It wins over anything the effects implied.
    pub fn set_table(&mut self, st: &mut BoardGameState, t: &Value) {
        if let Some(seats) = t.get("seats").and_then(Value::as_array) {
            for s in seats {
                let Some(i) = u(s, "seat").and_then(|w| self.index(w as u8)) else { continue };
                let p = &mut st.players[i as usize];
                p.pos = u(s, "tile").unwrap_or(p.pos as u64) as u32;
                p.skip = b(s, "skip");
                p.ward = b(s, "ward");
                p.streak = u(s, "streak").unwrap_or(0) as u32;
                p.bonus = b(s, "bonus");
                self.npc[i as usize] = b(s, "npc");
            }
        }
        if let Some(rev) = t.get("revealed").and_then(Value::as_array) {
            for r in rev {
                let Some(tile) = u(r, "tile").map(|x| x as u32) else { continue };
                reveal(st, tile, r.get("trap").and_then(Value::as_str).and_then(trap), r.get("boost").and_then(Value::as_str).and_then(boost));
            }
        }
    }

    /// `turn.begin{seat, phase, payload}`: whose turn it is and what it waits for.
    pub fn begin(&mut self, st: &mut BoardGameState, wire: u8, phase: &str, payload: &Value) {
        let Some(i) = self.index(wire) else { return };
        st.turn = i as usize;
        st.pending = None;
        st.offer = None;
        if let Some(t) = payload.get("table") {
            self.set_table(st, t);
        }
        st.phase = match phase {
            AWAITING_SPELL => {
                let tier = payload.get("tier").and_then(Value::as_str).and_then(tier).unwrap_or(Tier::Easy);
                let dest = u(payload, "tile").unwrap_or(0) as u32;
                // The shape of the word only (I2): the word itself stays on the server.
                st.pending = Some(Spell {
                    kind: SpellKind::Landing,
                    tier,
                    word: String::new(),
                    dest,
                    step: u(payload, "step").unwrap_or(0) as u8,
                    stretch: b(payload, "stretch"),
                });
                Phase::AwaitSpelling
            }
            AWAITING_CHOICE => match payload.get("kind").and_then(Value::as_str) {
                Some("stretch") => {
                    let side = |k: &str| {
                        let v = payload.get(k).cloned().unwrap_or(Value::Null);
                        (u(&v, "tile").unwrap_or(0) as u32, v.get("tier").and_then(Value::as_str).and_then(tier).unwrap_or(Tier::Easy))
                    };
                    let ((nd, nt), (sd, stier)) = (side("normal"), side("stretch"));
                    st.offer = Some(Offer { roll: u(payload, "roll").unwrap_or(0) as u8, normal_dest: nd, normal_tier: nt, stretch_dest: sd, stretch_tier: stier });
                    Phase::AwaitStretch
                }
                _ => Phase::AwaitSwitchTarget,
            },
            _ => Phase::AwaitRoll,
        };
    }

    /// One server effect as the solo game's event, in rules seats.
    pub fn event(&self, e: &Value) -> Option<Event> {
        let seat = || u(e, "seat").and_then(|w| self.index(w as u8));
        let tile = || u(e, "tile").map(|x| x as u32);
        let n = |k: &str| u(e, k).map(|x| x as u32);
        Some(match e.get("e")?.as_str()? {
            "rolled" => Event::Rolled { seat: seat()?, roll: u(e, "roll")? as u8, dest: n("dest")? },
            "streak_earned" => Event::StreakEarned { seat: seat()? },
            "streak_bonus_used" => Event::StreakBonusUsed { seat: seat()? },
            "boost" => Event::BoostHit { seat: seat()?, tile: tile()?, boost: boost(e.get("boost")?.as_str()?)? },
            "ward_gained" => Event::WardGained { seat: seat()? },
            "trap_blocked" => Event::TrapBlocked { seat: seat()?, tile: tile()?, trap: trap(e.get("trap")?.as_str()?)? },
            "stretched" => Event::Stretched { seat: seat()? },
            "moved" => Event::Moved { seat: seat()?, from: n("from")?, to: n("to")? },
            "missed" => Event::Missed { seat: seat()?, at: n("at")?, word: None },
            "trap" => Event::TrapHit { seat: seat()?, tile: tile()?, trap: trap(e.get("trap")?.as_str()?)? },
            "trap_ok" => Event::TrapSpellOk { seat: seat()?, trap: trap(e.get("trap")?.as_str()?)? },
            "teleported" => Event::Teleported { seat: seat()?, from: n("from")?, to: n("to")? },
            "skip_set" => Event::SkipSet { seat: seat()? },
            "skipped" => Event::Skipped { seat: seat()? },
            "swapped" => {
                let a = u(e, "a").and_then(|w| self.index(w as u8))?;
                let b = u(e, "b").and_then(|w| self.index(w as u8))?;
                Event::Swapped { a, b }
            }
            "switch_declined" => Event::SwitchDeclined { seat: seat()? },
            "finished" => Event::Finished { seat: seat()? },
            _ => return None,
        })
    }

    /// Applies effects as far as they say (I4: positions always), and appends
    /// them to the event log the screen reads.
    pub fn effects(&self, s: &mut BoardGameState, effects: &[Value]) {
        for v in effects {
            let Some(ev) = self.event(v) else { continue };
            match ev {
                Event::Moved { seat, to, .. } | Event::Teleported { seat, to, .. } => s.players[seat as usize].pos = to,
                Event::Swapped { a, b } => {
                    let (pa, pb) = (s.players[a as usize].pos, s.players[b as usize].pos);
                    s.players[a as usize].pos = pb;
                    s.players[b as usize].pos = pa;
                }
                Event::WardGained { seat } => s.players[seat as usize].ward = true,
                Event::StreakEarned { seat } => {
                    s.players[seat as usize].streak = 0;
                    s.players[seat as usize].bonus = true;
                }
                Event::StreakBonusUsed { seat } => s.players[seat as usize].bonus = false,
                Event::SkipSet { seat } => s.players[seat as usize].skip = true,
                Event::Skipped { seat } => s.players[seat as usize].skip = false,
                Event::Rolled { seat, .. } => s.players[seat as usize].rolls += 1,
                Event::Finished { seat } => {
                    s.winner = Some(seat);
                    s.phase = Phase::Finished;
                }
                _ => {}
            }
            match ev {
                Event::TrapHit { tile, trap, .. } => reveal(s, tile, Some(trap), None),
                Event::TrapBlocked { seat, tile, trap } => {
                    s.players[seat as usize].ward = false;
                    reveal(s, tile, Some(trap), None);
                }
                Event::BoostHit { tile, boost, .. } => reveal(s, tile, None, Some(boost)),
                _ => {}
            }
            s.events.push(ev);
        }
    }

    /// `turn.verdict` for my own seat: the word goes to my no-repeat history
    /// (F8, TURN D-O20) as the solo game records it.
    pub fn my_verdict(st: &mut BoardGameState, me: u8, word: &str, correct: bool) {
        let Some(tier) = st.pending.as_ref().map(|p| p.tier) else { return };
        if word.is_empty() {
            return;
        }
        let p = &mut st.players[me as usize];
        p.attempts += 1;
        if correct {
            p.hits += 1;
            p.spelled.push(word.to_string());
        } else {
            p.missed.push(word.to_string());
        }
        st.drawn.push((tier, word.to_string()));
    }

    /// `game.end`: the final board, and the order the server ranked.
    pub fn end(&mut self, st: &mut BoardGameState, ranking: &[u8], summary: &Value) -> Vec<u8> {
        self.set_table(st, summary);
        if let Some(w) = u(summary, "winner").and_then(|w| self.index(w as u8)) {
            st.winner = Some(w);
        }
        st.phase = Phase::Finished;
        st.pending = None;
        st.offer = None;
        ranking.iter().filter_map(|w| self.index(*w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn view() -> Value {
        json!({
            "size": "sprint",
            "tiers": [null, "easy", "medium", "hard", "expert", "easy", null],
            "revealed": [{ "tile": 3, "trap": "start", "boost": null }],
            "seats": [
                { "seat": 0, "piece": 2, "npc": false, "tile": 4, "skip": false, "ward": true, "streak": 1, "bonus": false },
                { "seat": 2, "piece": 0, "npc": true, "tile": 1, "skip": true, "ward": false, "streak": 0, "bonus": false },
                { "seat": 3, "piece": 1, "npc": false, "tile": 0, "skip": false, "ward": false, "streak": 0, "bonus": true }
            ],
            "winner": null,
            "options": { "stretch": true, "boosts": true, "streak": true, "npc": "normal" }
        })
    }

    fn mirror() -> (Mirror, BoardGameState) {
        Mirror::from_view(&view(), "en", false).expect("a view builds a mirror")
    }

    #[test]
    fn f7_a_view_rebuilds_the_board_the_seats_and_the_revealed_traps() {
        let (m, s) = mirror();
        assert_eq!(m.table, vec![0, 2, 3]);
        assert_eq!(m.npc, vec![false, true, false]);
        assert_eq!(s.board.len(), 7);
        assert_eq!(s.board.tiers[2], Some(Tier::Medium));
        assert_eq!(s.board.trap_at(3), Some(Trap::BackToStart));
        assert_eq!(s.board.trap_count(), 1, "unrevealed traps stay unknown");
        assert_eq!(s.players.iter().map(|p| (p.seat.piece, p.pos)).collect::<Vec<_>>(), vec![(2, 4), (0, 1), (1, 0)]);
        assert!(s.players[0].ward && s.players[1].skip && s.players[2].bonus);
        assert!(s.cfg.stretch && s.cfg.boosts && s.cfg.streak);
        assert_eq!(s.phase, Phase::AwaitRoll);
        assert!(Mirror::from_view(&json!({ "size": "huge" }), "en", false).is_none());
    }

    #[test]
    fn turn_begin_sets_the_seat_and_the_word_shape_never_a_word() {
        let (mut m, mut s) = mirror();
        m.begin(&mut s, 3, AWAITING_SPELL, &json!({ "tier": "hard", "length": 6, "tile": 3, "of": 6, "stretch": false, "step": 0 }));
        assert_eq!(s.current_seat(), 2, "wire seat 3 is rules seat 2");
        assert_eq!(s.phase, Phase::AwaitSpelling);
        let p = s.pending.as_ref().unwrap();
        assert_eq!((p.tier, p.dest, p.word.as_str()), (Tier::Hard, 3, ""));
        m.begin(&mut s, 0, AWAITING_CHOICE, &json!({ "kind": "stretch", "roll": 2, "normal": { "tile": 2, "tier": "medium" }, "stretch": { "tile": 4, "tier": "expert" } }));
        assert_eq!(s.phase, Phase::AwaitStretch);
        assert_eq!(s.offer, Some(Offer { roll: 2, normal_dest: 2, normal_tier: Tier::Medium, stretch_dest: 4, stretch_tier: Tier::Expert }));
        assert_eq!(s.pending, None);
        m.begin(&mut s, 2, AWAITING_CHOICE, &json!({ "kind": "switch" }));
        assert_eq!((s.phase, s.offer), (Phase::AwaitSwitchTarget, None));
        m.begin(&mut s, 0, "awaiting_roll", &json!({ "table": { "seats": [{ "seat": 2, "tile": 5, "npc": false }] } }));
        assert_eq!(s.players[1].pos, 5, "the table payload is the truth");
        assert!(!m.npc[1], "a seat back from the server's hands");
    }

    #[test]
    fn i4_every_effect_becomes_the_solo_event_and_moves_pieces() {
        let (m, mut s) = mirror();
        let fx = vec![
            json!({ "e": "rolled", "seat": 3, "roll": 4, "dest": 4 }),
            json!({ "e": "moved", "seat": 3, "from": 0, "to": 4 }),
            json!({ "e": "trap", "seat": 3, "tile": 4, "trap": "switch" }),
            json!({ "e": "swapped", "a": 3, "b": 0 }),
            json!({ "e": "boost", "seat": 2, "tile": 5, "boost": "ward" }),
            json!({ "e": "ward_gained", "seat": 2 }),
            json!({ "e": "trap_blocked", "seat": 0, "tile": 2, "trap": "lose" }),
            json!({ "e": "teleported", "seat": 2, "from": 1, "to": 0 }),
            json!({ "e": "missed", "seat": 0, "at": 4 }),
            json!({ "e": "streak_earned", "seat": 0 }),
            json!({ "e": "nonsense" }),
            json!({ "e": "finished", "seat": 0 }),
        ];
        m.effects(&mut s, &fx);
        assert_eq!(s.events.len(), 11, "the unknown effect is dropped, the rest kept in order");
        assert_eq!(s.events[0], Event::Rolled { seat: 2, roll: 4, dest: 4 });
        assert_eq!(s.events[3], Event::Swapped { a: 2, b: 0 });
        assert_eq!(s.events[8], Event::Missed { seat: 0, at: 4, word: None }, "no word rides on an effect");
        assert_eq!(s.players.iter().map(|p| p.pos).collect::<Vec<_>>(), vec![4, 0, 4]);
        assert_eq!(s.board.trap_at(4), Some(Trap::SwitchTiles));
        assert_eq!(s.board.trap_at(2), Some(Trap::LoseRoll));
        assert_eq!(s.board.boost_at(5), Some(Boost::Ward));
        assert!(s.players[1].ward && !s.players[0].ward, "ward gained, and the blocker spent its own");
        assert!(s.players[0].bonus);
        assert_eq!((s.winner, s.phase), (Some(0), Phase::Finished));
    }

    #[test]
    fn f8_only_my_verdicts_reach_the_history_and_game_end_ranks_in_rules_seats() {
        let (mut m, mut s) = mirror();
        Mirror::my_verdict(&mut s, 0, "cat", true);
        assert!(s.drawn.is_empty(), "no word owed, nothing recorded");
        m.begin(&mut s, 0, AWAITING_SPELL, &json!({ "tier": "medium", "tile": 2 }));
        Mirror::my_verdict(&mut s, 0, "cat", true);
        Mirror::my_verdict(&mut s, 0, "", false);
        Mirror::my_verdict(&mut s, 0, "dog", false);
        assert_eq!(s.drawn, vec![(Tier::Medium, "cat".to_string()), (Tier::Medium, "dog".to_string())]);
        assert_eq!((s.players[0].hits, s.players[0].attempts), (1, 2));
        let order = m.end(&mut s, &[3, 0, 2], &json!({ "seats": [{ "seat": 3, "tile": 6 }], "winner": 3 }));
        assert_eq!(order, vec![2, 0, 1]);
        assert_eq!((s.winner, s.phase, s.players[2].pos), (Some(2), Phase::Finished, 6));
    }

    #[test]
    fn the_servers_names_match_the_rules_names() {
        // server/spell-server/src/boardgame.rs: trap_name / boost_name / Tier::name.
        let server = include_str!("../server/spell-server/src/boardgame.rs");
        for (name, t) in [("start", Trap::BackToStart), ("long", Trap::LongWord), ("lose", Trap::LoseRoll), ("switch", Trap::SwitchTiles), ("double", Trap::DoubleExpert)] {
            assert_eq!(trap(name), Some(t));
            assert!(server.contains(&format!("{t:?} => \"{name}\"")), "{name}");
        }
        for (name, x) in [("tailwind", Boost::Tailwind), ("extra_roll", Boost::ExtraRoll), ("ward", Boost::Ward)] {
            assert_eq!(boost(name), Some(x));
            assert!(server.contains(&format!("{x:?} => \"{name}\"")), "{name}");
        }
        for t in Tier::ALL {
            assert_eq!(tier(t.name()), Some(t));
        }
    }
}
