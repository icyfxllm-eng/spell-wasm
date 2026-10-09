//! F1/F3/F4/F5 -- `new_game` and the reducer.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::board::generate;
use super::*;
use crate::spelldoku::rng::Rng;

const NPC_FLOOR_MILLI: i64 = 300;
const NPC_CEIL_MILLI: i64 = 950;

/// A fresh game. All randomness comes from `seed`, in a fixed order: board
/// tiers, traps, first player, then each tier's word order.
pub fn new_game(seed: u64, cfg: GameConfig, pools: TierPools) -> Result<BoardGameState, ConfigError> {
    let c = cfg.variant.cfg();
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
    for t in c.tiers() {
        if pools.tiers[t.ix()].len() < c.min_pool {
            return Err(ConfigError::PoolTooSmall(t));
        }
    }
    if c.has_traps() && pools.long_word.len() < LONG_WORD_FLOOR {
        return Err(ConfigError::LongPoolTooSmall);
    }

    let mut rng = Rng::new(seed);
    let board = generate(&mut rng, cfg.variant, cfg.boosts);
    let start = (rng.next_u64() % n as u64) as usize;
    let order: Vec<u8> = (0..n).map(|k| ((start + k) % n) as u8).collect();

    let mut queues: [Vec<u32>; 4] = Default::default();
    for t in c.tiers() {
        queues[t.ix()] = shuffled(&mut rng, pools.tiers[t.ix()].len());
    }
    let long_queue = if c.has_traps() { shuffled(&mut rng, pools.long_word.len()) } else { Vec::new() };

    let players = cfg
        .seats
        .iter()
        .map(|&seat| Player { seat, pos: 0, skip: false, rolls: 0, attempts: 0, hits: 0, spelled: Vec::new(), missed: Vec::new(), stretch_offered: 0, stretch_taken: 0, stretch_hits: 0, ward: false, streak: 0, bonus: false })
        .collect();

    Ok(BoardGameState {
        cfg,
        board,
        players,
        order,
        turn: 0,
        phase: Phase::AwaitRoll,
        pending: None,
        offer: None,
        effect_used: false,
        rng,
        pools: Arc::new(pools),
        queues,
        long_queue,
        used: BTreeSet::new(),
        recycled: 0,
        drawn: Vec::new(),
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
        Action::ChooseStretch(take) => {
            if npc {
                return Err(Rejected::NotHumanTurn);
            }
            if s.phase != Phase::AwaitStretch || s.offer.is_none() {
                return Err(Rejected::WrongPhase);
            }
            s.human_stretch(seat, take);
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

/// What the tile a piece arrived on does.
enum TileFx {
    Nothing,
    Trap(Trap),
    Boost(Boost),
}

/// What happens to the turn after a boost.
enum Flow {
    End,
    /// Extra Roll: the same seat rolls again.
    Again,
    /// The boost carried the piece to the finish.
    Done,
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
        let tier = match src {
            Src::Tier(t) => t,
            Src::Long => Tier::Expert,
        };
        let w = self.draw_raw(src);
        self.drawn.push((tier, w.clone()));
        w
    }

    fn draw_raw(&mut self, src: Src) -> String {
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
        self.effect_used = false;
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

    /// Feature 3: the pending streak bonus goes onto the roll being taken now (and is spent
    /// whether or not that roll's spelling turns out right). A turn that is skipped takes no roll,
    /// so it leaves the bonus waiting.
    fn take_bonus(&mut self, seat: u8) -> u32 {
        if !(self.cfg.streak && self.players[seat as usize].bonus) {
            return 0;
        }
        self.players[seat as usize].bonus = false;
        self.push(Event::StreakBonusUsed { seat });
        self.cfg.variant.cfg().streak_bonus
    }

    /// Feature 3 (D-P3): every graded spelling counts, Stretch and trap words included. A right
    /// one adds 1; at `streak_length` the counter resets and the bonus is pending; a wrong one resets.
    fn graded(&mut self, seat: u8, ok: bool) {
        if !self.cfg.streak {
            return;
        }
        let len = self.cfg.variant.cfg().streak_length;
        let p = &mut self.players[seat as usize];
        if !ok {
            p.streak = 0;
            return;
        }
        p.streak += 1;
        if p.streak >= len {
            p.streak = 0;
            p.bonus = true;
            self.push(Event::StreakEarned { seat });
        }
    }

    /// I-P4: the tile effect for a piece that has just arrived by its own spelling. At most one
    /// trap or boost resolves per turn, the first one reached; once `effect_used` is set (a Tailwind
    /// landing on another special, an Extra Roll's second roll) nothing further triggers and the
    /// tile stays hidden. A held Ward turns the trap into a revealed no-op and is spent.
    fn tile_effect(&mut self, seat: u8, tile: u32) -> TileFx {
        if self.effect_used {
            return TileFx::Nothing;
        }
        if let Some(trap) = self.board.trap_at(tile) {
            self.reveal(tile);
            self.effect_used = true;
            if self.players[seat as usize].ward {
                self.players[seat as usize].ward = false;
                self.push(Event::TrapBlocked { seat, tile, trap });
                return TileFx::Nothing;
            }
            self.push(Event::TrapHit { seat, tile, trap });
            return TileFx::Trap(trap);
        }
        if let Some(boost) = self.board.boost_at(tile) {
            self.reveal(tile);
            self.effect_used = true;
            self.push(Event::BoostHit { seat, tile, boost });
            return TileFx::Boost(boost);
        }
        TileFx::Nothing
    }

    /// Resolve a triggered boost. Tailwind moves on (clamped at the finish, which wins the game),
    /// Extra Roll keeps the turn, Ward is collected (one charge at most).
    fn apply_boost(&mut self, seat: u8, tile: u32, boost: Boost) -> Flow {
        match boost {
            Boost::Tailwind => {
                let to = (tile + self.cfg.variant.cfg().tailwind_tiles).min(self.board.last());
                self.players[seat as usize].pos = to;
                self.push(Event::Moved { seat, from: tile, to });
                if to == self.board.last() {
                    self.finish(seat);
                    Flow::Done
                } else {
                    Flow::End
                }
            }
            Boost::ExtraRoll => Flow::Again,
            Boost::Ward => {
                self.players[seat as usize].ward = true;
                self.push(Event::WardGained { seat });
                Flow::End
            }
        }
    }

    // ---------------------------------------------------------------- human

    fn human_roll(&mut self, seat: u8) {
        let roll = self.roll_d6();
        let from = self.players[seat as usize].pos;
        let bonus = self.take_bonus(seat);
        let dest = (from + roll as u32 + bonus).min(self.board.last());
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
        if let Some(harder) = self.stretch_tier(tier) {
            // Feature 1 (I-P6): stop here. Neither word is drawn until the choice is committed.
            let bonus = self.cfg.variant.cfg().stretch_bonus;
            let stretch_dest = (dest + bonus).min(self.board.last());
            self.players[seat as usize].stretch_offered += 1;
            self.offer = Some(Offer { roll, normal_dest: dest, normal_tier: tier, stretch_dest, stretch_tier: harder });
            self.phase = Phase::AwaitStretch;
            return;
        }
        let word = self.draw(Src::Tier(tier));
        self.pending = Some(Spell { kind: SpellKind::Landing, tier, word, dest, step: 0, stretch: false });
        self.phase = Phase::AwaitSpelling;
    }

    /// Feature 1: the tier a Stretch from a tile of `tier` would use, if Stretch is on in this
    /// game and a harder tier exists in this variant's pool (never from Expert, never in Jr).
    fn stretch_tier(&self, tier: Tier) -> Option<Tier> {
        let c = self.cfg.variant.cfg();
        if !(self.cfg.stretch && c.stretch) {
            return None;
        }
        c.harder(tier)
    }

    fn human_stretch(&mut self, seat: u8, take: bool) {
        let o = self.offer.take().expect("checked by apply");
        let (tier, dest) = if take { (o.stretch_tier, o.stretch_dest) } else { (o.normal_tier, o.normal_dest) };
        if take {
            self.players[seat as usize].stretch_taken += 1;
        }
        let word = self.draw(Src::Tier(tier));
        self.pending = Some(Spell { kind: SpellKind::Landing, tier, word, dest, step: 0, stretch: take });
        self.phase = Phase::AwaitSpelling;
    }

    fn grade(&self, typed: &str, sp: &Spell) -> bool {
        (self.cfg.grader)(&self.cfg.lang, self.cfg.kid, typed, &sp.word, sp.tier)
    }

    fn human_spell(&mut self, seat: u8, typed: &str) {
        let sp = self.pending.take().expect("checked by apply");
        let ok = self.grade(typed, &sp);
        self.graded(seat, ok);
        match sp.kind {
            SpellKind::Landing => {
                let from = self.players[seat as usize].pos;
                let p = &mut self.players[seat as usize];
                // D-P11: a Stretch spelling is not an attempt at the tile's own tier, so it stays
                // out of the running accuracy the NPCs track.
                if sp.stretch {
                    p.stretch_hits += ok as u32;
                } else {
                    p.attempts += 1;
                    p.hits += ok as u32;
                }
                if sp.stretch {
                    self.push(Event::Stretched { seat });
                }
                let p = &mut self.players[seat as usize];
                if ok {
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
                    self.pending = Some(Spell { kind: SpellKind::DoubleExpert, tier: Tier::Expert, word, dest: sp.dest, step: 1, stretch: false });
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
        if dest == self.board.last() {
            // Feature 1: a Stretch bonus can carry a piece onto the finish.
            self.finish(seat);
            return;
        }
        let trap = match self.tile_effect(seat, dest) {
            TileFx::Nothing => {
                self.end_turn();
                return;
            }
            TileFx::Boost(b) => {
                match self.apply_boost(seat, dest, b) {
                    Flow::End => self.end_turn(),
                    // The same seat rolls again; the turn (and its one tile effect) carries on.
                    Flow::Again => {
                        self.pending = None;
                        self.phase = Phase::AwaitRoll;
                    }
                    Flow::Done => {}
                }
                return;
            }
            TileFx::Trap(t) => t,
        };
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
                self.pending = Some(Spell { kind: SpellKind::LongWord, tier: Tier::Expert, word, dest, step: 0, stretch: false });
                self.phase = Phase::AwaitSpelling;
            }
            Trap::DoubleExpert => {
                let word = self.draw(Src::Tier(Tier::Expert));
                self.pending = Some(Spell { kind: SpellKind::DoubleExpert, tier: Tier::Expert, word, dest, step: 0, stretch: false });
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
        (self.human_acc_milli() - self.cfg.variant.cfg().delta(self.cfg.difficulty)).clamp(NPC_FLOOR_MILLI, NPC_CEIL_MILLI)
    }

    /// One whole NPC turn. NPCs draw no words (I11): their outcomes come from
    /// the seeded stream, the adaptive formula and the state (I9), and they are
    /// subject to every trap (I10). An Extra Roll keeps the turn: the NPC rolls again inside
    /// the same `AdvanceNpc` (at most twice, since only one tile effect resolves per turn).
    fn npc_turn(&mut self, seat: u8) {
        loop {
            match self.npc_roll(seat) {
                Flow::Again => continue,
                Flow::End => self.end_turn(),
                Flow::Done => {}
            }
            return;
        }
    }

    fn npc_roll(&mut self, seat: u8) -> Flow {
        let p = self.npc_acc_milli();
        let roll = self.roll_d6();
        let from = self.players[seat as usize].pos;
        let bonus = self.take_bonus(seat);
        let dest = (from + roll as u32 + bonus).min(self.board.last());
        self.players[seat as usize].rolls += 1;
        self.push(Event::Rolled { seat, roll, dest });
        if dest == self.board.last() {
            self.players[seat as usize].pos = dest;
            self.push(Event::Moved { seat, from, to: dest });
            self.finish(seat);
            return Flow::Done;
        }
        // Feature 1 (D-P10, D-P21). Stretch is considered only when it is on and offered AND the NPC
        // can pay the penalty without hitting the accuracy floor (`p - penalty >= floor`): an NPC
        // that cannot lose accuracy by stretching never stretches, and makes no draw for it. Eligible
        // NPCs take it with chance `base + slope * (p - 500) / 1000` (clamped 0..=1000): one draw,
        // then one spelling draw at `p - penalty`. Off or ineligible, the turn draws exactly as before.
        let mut dest = dest;
        let (base, slope, penalty) = self.cfg.variant.cfg().npc_stretch();
        let eligible = self.stretch_tier(self.board.tiers[dest as usize].expect("a non-end tile has a tier")).is_some() && p - penalty >= NPC_FLOOR_MILLI;
        let mut stretched = false;
        if eligible {
            self.players[seat as usize].stretch_offered += 1;
            let rate = (base + slope * (p - 500) / 1000).clamp(0, 1000);
            if self.chance(rate) {
                stretched = true;
                self.players[seat as usize].stretch_taken += 1;
                self.push(Event::Stretched { seat });
                dest = (dest + self.cfg.variant.cfg().stretch_bonus).min(self.board.last());
            }
        }
        let p_spell = if stretched { p - penalty } else { p };
        let ok = self.chance(p_spell);
        self.graded(seat, ok);
        if !ok {
            self.push(Event::Missed { seat, at: from, word: None });
            return Flow::End;
        }
        if stretched {
            self.players[seat as usize].stretch_hits += 1;
        }
        self.players[seat as usize].pos = dest;
        self.push(Event::Moved { seat, from, to: dest });
        if dest == self.board.last() {
            self.finish(seat);
            return Flow::Done;
        }
        let trap = match self.tile_effect(seat, dest) {
            TileFx::Nothing => return Flow::End,
            TileFx::Boost(b) => return self.apply_boost(seat, dest, b),
            TileFx::Trap(t) => t,
        };
        match trap {
            Trap::BackToStart => {
                self.players[seat as usize].pos = 0;
                self.push(Event::Teleported { seat, from: dest, to: 0 });
            }
            Trap::LongWord => {
                let ok = self.chance(p);
                self.graded(seat, ok);
                if !ok {
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
                let first = self.chance(p);
                self.graded(seat, first);
                let both = first && {
                    let second = self.chance(p);
                    self.graded(seat, second);
                    second
                };
                if !both {
                    self.players[seat as usize].skip = true;
                    self.push(Event::SkipSet { seat });
                } else {
                    self.push(Event::TrapSpellOk { seat, trap });
                }
            }
        }
        Flow::End
    }
}
