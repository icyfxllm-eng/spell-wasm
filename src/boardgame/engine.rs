//! F1/F3/F4/F5 -- `new_game` and the reducer.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::board::generate;
use super::rules::{Jr, Ruleset, Standard};
use super::*;
use crate::spelldoku::rng::Rng;

/// D18 / F5. Spell Jr has one fixed delta.
const JR_DELTA_MILLI: i64 = 250;
const NPC_FLOOR_MILLI: i64 = 300;
const NPC_CEIL_MILLI: i64 = 950;

fn delta_milli(variant: Variant, d: Difficulty) -> i64 {
    match (variant, d) {
        (Variant::Jr, _) => JR_DELTA_MILLI,
        (_, Difficulty::Easy) => 300,
        (_, Difficulty::Normal) => 200,
        (_, Difficulty::Tough) => 100,
    }
}

/// A fresh game. All randomness comes from `seed`, in a fixed order: board
/// tiers, traps, first player, then each tier's word order.
pub fn new_game(seed: u64, cfg: GameConfig, pools: TierPools) -> Result<BoardGameState, ConfigError> {
    match cfg.variant {
        Variant::Standard => build::<Standard>(seed, cfg, pools),
        Variant::Jr => build::<Jr>(seed, cfg, pools),
    }
}

fn build<R: Ruleset>(seed: u64, cfg: GameConfig, pools: TierPools) -> Result<BoardGameState, ConfigError> {
    let n = cfg.seats.len();
    if !(2..=MAX_SEATS).contains(&n) {
        return Err(ConfigError::SeatCount);
    }
    let npcs = cfg.seats.iter().filter(|s| s.npc).count();
    if npcs > 0 && n - npcs != 1 || npcs == n {
        return Err(ConfigError::MixedSeats);
    }
    let mut seen = [false; MAX_SEATS];
    for s in &cfg.seats {
        if s.piece as usize >= MAX_SEATS || seen[s.piece as usize] {
            return Err(ConfigError::BadPiece);
        }
        seen[s.piece as usize] = true;
    }
    for &(t, _) in R::WEIGHTS {
        if pools.tiers[t.ix()].len() < R::MIN_POOL {
            return Err(ConfigError::PoolTooSmall(t));
        }
    }
    if R::HAS_TRAPS && pools.long_word.len() < LONG_WORD_FLOOR {
        return Err(ConfigError::LongPoolTooSmall);
    }

    let mut rng = Rng::new(seed);
    let board = generate::<R>(&mut rng);
    let start = (rng.next_u64() % n as u64) as usize;
    let order: Vec<u8> = (0..n).map(|k| ((start + k) % n) as u8).collect();

    let mut queues: [Vec<u32>; 4] = Default::default();
    for &(t, _) in R::WEIGHTS {
        queues[t.ix()] = shuffled(&mut rng, pools.tiers[t.ix()].len());
    }
    let long_queue = if R::HAS_TRAPS { shuffled(&mut rng, pools.long_word.len()) } else { Vec::new() };

    let players = cfg
        .seats
        .iter()
        .map(|&seat| Player { seat, pos: 0, skip: false, rolls: 0, attempts: 0, hits: 0, spelled: Vec::new(), missed: Vec::new() })
        .collect();

    Ok(BoardGameState {
        cfg,
        board,
        players,
        order,
        turn: 0,
        phase: Phase::AwaitRoll,
        pending: None,
        rng,
        pools: Arc::new(pools),
        queues,
        long_queue,
        used: BTreeSet::new(),
        recycled: 0,
        revealed: Vec::new(),
        events: Vec::new(),
        winner: None,
        turns: 0,
    })
}

fn shuffled(rng: &mut Rng, len: usize) -> Vec<u32> {
    let mut v: Vec<u32> = (0..len as u32).collect();
    rng.shuffle(&mut v);
    v
}

/// The reducer, in place. A rejected action returns `Err` WITHOUT touching the
/// state: every check below runs before the first write.
pub fn apply(s: &mut BoardGameState, action: Action) -> Result<(), Rejected> {
    if matches!(action, Action::SkipAnimation) {
        return Ok(()); // no state effect, in any phase
    }
    if s.phase == Phase::Finished {
        return Err(Rejected::GameOver);
    }
    let seat = s.current_seat();
    let npc = s.players[seat as usize].seat.npc;
    match action {
        Action::SkipAnimation => Ok(()),
        Action::AdvanceNpc => {
            if s.phase != Phase::AwaitRoll {
                return Err(Rejected::WrongPhase);
            }
            if !npc {
                return Err(Rejected::NotNpcTurn);
            }
            s.npc_turn(seat);
            Ok(())
        }
        Action::Roll => {
            if npc {
                return Err(Rejected::NotHumanTurn);
            }
            if s.phase != Phase::AwaitRoll {
                return Err(Rejected::WrongPhase);
            }
            s.human_roll(seat);
            Ok(())
        }
        Action::SubmitSpelling(typed) => {
            if npc {
                return Err(Rejected::NotHumanTurn);
            }
            if s.phase != Phase::AwaitSpelling || s.pending.is_none() {
                return Err(Rejected::WrongPhase);
            }
            s.human_spell(seat, &typed);
            Ok(())
        }
        Action::ChooseSwitchTarget(target) => {
            if npc {
                return Err(Rejected::NotHumanTurn);
            }
            if s.phase != Phase::AwaitSwitchTarget {
                return Err(Rejected::WrongPhase);
            }
            if let Some(t) = target {
                if t == seat || t as usize >= s.players.len() {
                    return Err(Rejected::BadTarget);
                }
            }
            s.human_switch(seat, target);
            Ok(())
        }
    }
}

/// The same, as a pure function of the old state.
pub fn applied(s: &BoardGameState, action: Action) -> Result<BoardGameState, Rejected> {
    let mut next = s.clone();
    apply(&mut next, action)?;
    Ok(next)
}

enum Src {
    Tier(Tier),
    Long,
}

impl BoardGameState {
    fn roll_d6(&mut self) -> u8 {
        1 + (self.rng.next_u64() % 6) as u8
    }

    /// Per-mille chance in 0..1000.
    fn chance(&mut self, milli: i64) -> bool {
        ((self.rng.next_u64() % 1000) as i64) < milli
    }

    /// I5: the next word of a source that this game has not used. A queue that
    /// runs dry is refilled and the draw may repeat a word; that is counted in
    /// `recycled`, which the bank gate keeps at zero.
    fn draw(&mut self, src: Src) -> String {
        let pools = Arc::clone(&self.pools);
        let (list, qi) = match src {
            Src::Tier(t) => (&pools.tiers[t.ix()], Some(t.ix())),
            Src::Long => (&pools.long_word, None),
        };
        loop {
            let q = match qi {
                Some(i) => &mut self.queues[i],
                None => &mut self.long_queue,
            };
            while let Some(ix) = q.pop() {
                let w = &list[ix as usize];
                if self.used.insert(w.clone()) {
                    return w.clone();
                }
            }
            // Dry. Refill; the next pop is allowed to repeat.
            self.recycled += 1;
            let fresh = shuffled(&mut self.rng, list.len());
            let q = match qi {
                Some(i) => &mut self.queues[i],
                None => &mut self.long_queue,
            };
            *q = fresh;
            if let Some(ix) = q.pop() {
                let w = list[ix as usize].clone();
                self.used.insert(w.clone());
                return w;
            }
        }
    }

    fn push(&mut self, e: Event) {
        self.events.push(e);
    }

    fn finish(&mut self, seat: u8) {
        self.winner = Some(seat);
        self.phase = Phase::Finished;
        self.pending = None;
        self.push(Event::Finished { seat });
    }

    /// Move to the next seat in order, burning the turn of anyone who owes a
    /// skip (the calibration sim counts a skipped turn as a turn).
    fn end_turn(&mut self) {
        self.pending = None;
        self.phase = Phase::AwaitRoll;
        let n = self.order.len();
        for _ in 0..=n {
            self.turn = (self.turn + 1) % n;
            self.turns += 1;
            let seat = self.order[self.turn];
            if self.players[seat as usize].skip {
                self.players[seat as usize].skip = false;
                self.push(Event::Skipped { seat });
                continue;
            }
            return;
        }
    }

    fn reveal(&mut self, tile: u32) {
        if !self.revealed.contains(&tile) {
            self.revealed.push(tile);
        }
    }

    // ---------------------------------------------------------------- human

    fn human_roll(&mut self, seat: u8) {
        let roll = self.roll_d6();
        let from = self.players[seat as usize].pos;
        let dest = (from + roll as u32).min(self.board.last());
        self.players[seat as usize].rolls += 1;
        self.push(Event::Rolled { seat, roll, dest });
        if dest == self.board.last() {
            // D2: overshoot finishes, and no word is owed.
            self.players[seat as usize].pos = dest;
            self.push(Event::Moved { seat, from, to: dest });
            self.finish(seat);
            return;
        }
        let tier = self.board.tiers[dest as usize].expect("a non-end tile has a tier");
        let word = self.draw(Src::Tier(tier));
        self.pending = Some(Spell { kind: SpellKind::Landing, tier, word, dest, step: 0 });
        self.phase = Phase::AwaitSpelling;
    }

    fn grade(&self, typed: &str, sp: &Spell) -> bool {
        (self.cfg.grader)(&self.cfg.lang, self.cfg.kid, typed, &sp.word, sp.tier)
    }

    fn human_spell(&mut self, seat: u8, typed: &str) {
        let sp = self.pending.take().expect("checked by apply");
        let ok = self.grade(typed, &sp);
        match sp.kind {
            SpellKind::Landing => {
                let from = self.players[seat as usize].pos;
                let p = &mut self.players[seat as usize];
                p.attempts += 1;
                if ok {
                    p.hits += 1;
                    p.spelled.push(sp.word.clone());
                    self.land(seat, from, sp.dest);
                } else {
                    // I4: a wrong spelling changes no position.
                    p.missed.push(sp.word.clone());
                    self.push(Event::Missed { seat, at: from, word: Some(sp.word) });
                    self.end_turn();
                }
            }
            SpellKind::LongWord => {
                if ok {
                    self.players[seat as usize].spelled.push(sp.word.clone());
                    self.push(Event::TrapSpellOk { seat, trap: Trap::LongWord });
                    self.end_turn();
                } else {
                    self.players[seat as usize].missed.push(sp.word.clone());
                    let from = self.players[seat as usize].pos;
                    let to = from.saturating_sub(6);
                    self.players[seat as usize].pos = to;
                    self.push(Event::Missed { seat, at: from, word: Some(sp.word) });
                    self.push(Event::Teleported { seat, from, to });
                    self.end_turn();
                }
            }
            SpellKind::DoubleExpert => {
                if !ok {
                    self.players[seat as usize].missed.push(sp.word.clone());
                    let at = self.players[seat as usize].pos;
                    self.players[seat as usize].skip = true;
                    self.push(Event::Missed { seat, at, word: Some(sp.word) });
                    self.push(Event::SkipSet { seat });
                    self.end_turn();
                } else if sp.step == 0 {
                    self.players[seat as usize].spelled.push(sp.word.clone());
                    let word = self.draw(Src::Tier(Tier::Expert));
                    self.pending = Some(Spell { kind: SpellKind::DoubleExpert, tier: Tier::Expert, word, dest: sp.dest, step: 1 });
                    self.phase = Phase::AwaitSpelling;
                } else {
                    self.players[seat as usize].spelled.push(sp.word.clone());
                    self.push(Event::TrapSpellOk { seat, trap: Trap::DoubleExpert });
                    self.end_turn();
                }
            }
        }
    }

    /// A correct landing spell: move, then resolve the tile (human).
    fn land(&mut self, seat: u8, from: u32, dest: u32) {
        self.players[seat as usize].pos = dest;
        self.push(Event::Moved { seat, from, to: dest });
        let Some(trap) = self.board.trap_at(dest) else {
            self.end_turn();
            return;
        };
        self.reveal(dest);
        self.push(Event::TrapHit { seat, tile: dest, trap });
        match trap {
            Trap::BackToStart => {
                self.players[seat as usize].pos = 0;
                self.push(Event::Teleported { seat, from: dest, to: 0 });
                self.end_turn();
            }
            Trap::LoseRoll => {
                self.players[seat as usize].skip = true;
                self.push(Event::SkipSet { seat });
                self.end_turn();
            }
            Trap::LongWord => {
                let word = self.draw(Src::Long);
                self.pending = Some(Spell { kind: SpellKind::LongWord, tier: Tier::Expert, word, dest, step: 0 });
                self.phase = Phase::AwaitSpelling;
            }
            Trap::DoubleExpert => {
                let word = self.draw(Src::Tier(Tier::Expert));
                self.pending = Some(Spell { kind: SpellKind::DoubleExpert, tier: Tier::Expert, word, dest, step: 0 });
                self.phase = Phase::AwaitSpelling;
            }
            Trap::SwitchTiles => {
                self.phase = Phase::AwaitSwitchTarget;
            }
        }
    }

    fn human_switch(&mut self, seat: u8, target: Option<u8>) {
        match target {
            Some(t) => self.swap(seat, t),
            None => self.push(Event::SwitchDeclined { seat }),
        }
        self.end_turn();
    }

    fn swap(&mut self, a: u8, b: u8) {
        let (pa, pb) = (self.players[a as usize].pos, self.players[b as usize].pos);
        self.players[a as usize].pos = pb;
        self.players[b as usize].pos = pa;
        self.push(Event::Swapped { a, b });
    }

    // ------------------------------------------------------------------ npc

    /// D13: `clamp(human_acc - delta, 0.30, 0.95)`, in thousandths.
    pub fn npc_acc_milli(&self) -> i64 {
        (self.human_acc_milli() - delta_milli(self.cfg.variant, self.cfg.difficulty)).clamp(NPC_FLOOR_MILLI, NPC_CEIL_MILLI)
    }

    /// One whole NPC turn. NPCs draw no words (I11): their outcomes come from
    /// the seeded stream, the adaptive formula and the state (I9), and they are
    /// subject to every trap (I10).
    fn npc_turn(&mut self, seat: u8) {
        let p = self.npc_acc_milli();
        let roll = self.roll_d6();
        let from = self.players[seat as usize].pos;
        let dest = (from + roll as u32).min(self.board.last());
        self.players[seat as usize].rolls += 1;
        self.push(Event::Rolled { seat, roll, dest });
        if dest == self.board.last() {
            self.players[seat as usize].pos = dest;
            self.push(Event::Moved { seat, from, to: dest });
            self.finish(seat);
            return;
        }
        if !self.chance(p) {
            self.push(Event::Missed { seat, at: from, word: None });
            self.end_turn();
            return;
        }
        self.players[seat as usize].pos = dest;
        self.push(Event::Moved { seat, from, to: dest });
        let Some(trap) = self.board.trap_at(dest) else {
            self.end_turn();
            return;
        };
        self.reveal(dest);
        self.push(Event::TrapHit { seat, tile: dest, trap });
        match trap {
            Trap::BackToStart => {
                self.players[seat as usize].pos = 0;
                self.push(Event::Teleported { seat, from: dest, to: 0 });
            }
            Trap::LongWord => {
                if !self.chance(p) {
                    let to = dest.saturating_sub(6);
                    self.players[seat as usize].pos = to;
                    self.push(Event::Teleported { seat, from: dest, to });
                } else {
                    self.push(Event::TrapSpellOk { seat, trap });
                }
            }
            Trap::LoseRoll => {
                self.players[seat as usize].skip = true;
                self.push(Event::SkipSet { seat });
            }
            Trap::SwitchTiles => {
                // F5: swap with the leader if it is ahead, otherwise nobody.
                let mut lead = 0usize;
                for (i, pl) in self.players.iter().enumerate() {
                    if pl.pos > self.players[lead].pos {
                        lead = i;
                    }
                }
                if self.players[lead].pos > self.players[seat as usize].pos {
                    self.swap(seat, lead as u8);
                } else {
                    self.push(Event::SwitchDeclined { seat });
                }
            }
            Trap::DoubleExpert => {
                // Both required; the second draw is not made once the first fails.
                if !self.chance(p) || !self.chance(p) {
                    self.players[seat as usize].skip = true;
                    self.push(Event::SkipSet { seat });
                } else {
                    self.push(Event::TrapSpellOk { seat, trap });
                }
            }
        }
        self.end_turn();
    }
}
