//! Head-to-head: two players on one device compete for the longest chain of
//! correctly-spelled words. Turn-based — a player keeps spelling until they
//! miss, which ends their turn; each gets a fixed number of turns and the
//! longest single chain wins.
//!
//! This module is deliberately pure state + transitions with NO DOM / web-sys
//! calls, so the exact same logic can back a future online version (turns
//! driven by the word server instead of a local tap) without change. All
//! rendering and word flow lives in `game.rs` / `lib.rs`.

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    P1,
    P2,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::P1 => Side::P2,
            Side::P2 => Side::P1,
        }
    }
}

#[derive(Clone, Default)]
pub struct Player {
    pub name: String,
    /// Chain being built during the current turn.
    pub current: u32,
    /// Longest chain this player has reached across all their turns.
    pub best: u32,
    pub turns_left: u32,
}

#[derive(Clone, Default)]
pub struct Versus {
    pub enabled: bool,
    pub p1: Player,
    pub p2: Player,
    pub active: Side,
    pub over: bool,
}

impl Versus {
    pub fn start(name1: String, name2: String, turns_each: u32) -> Versus {
        Versus {
            enabled: true,
            p1: Player { name: name1, current: 0, best: 0, turns_left: turns_each },
            p2: Player { name: name2, current: 0, best: 0, turns_left: turns_each },
            active: Side::P1,
            over: false,
        }
    }

    pub fn player(&self, side: Side) -> &Player {
        match side {
            Side::P1 => &self.p1,
            Side::P2 => &self.p2,
        }
    }

    fn player_mut(&mut self, side: Side) -> &mut Player {
        match side {
            Side::P1 => &mut self.p1,
            Side::P2 => &mut self.p2,
        }
    }

    pub fn active_player(&self) -> &Player {
        self.player(self.active)
    }

    /// A correct spelling extends the active player's current chain (and their
    /// best, if this is the longest they've reached).
    pub fn record_correct(&mut self) {
        let side = self.active;
        let p = self.player_mut(side);
        p.current += 1;
        if p.current > p.best {
            p.best = p.current;
        }
    }

    /// A miss ends the active player's turn: reset their running chain, spend a
    /// turn, and hand off to the other player if they still have turns left.
    /// Sets `over` once neither player has a turn remaining.
    pub fn end_turn(&mut self) {
        let side = self.active;
        {
            let p = self.player_mut(side);
            p.current = 0;
            p.turns_left = p.turns_left.saturating_sub(1);
        }
        let other = side.opposite();
        if self.player(other).turns_left > 0 {
            // Strict alternation while both have turns.
            self.active = other;
        } else if self.active_player().turns_left == 0 {
            // Neither side has a turn left — the match is done.
            self.over = true;
        }
        // else: the other side is out but the active side still has turns —
        // keep the active side to play its remaining turns out.
    }

    /// The winning side once the match is over, or `None` for a tie (also
    /// `None` while the match is still in progress).
    pub fn winner(&self) -> Option<Side> {
        if !self.over {
            return None;
        }
        if self.p1.best > self.p2.best {
            Some(Side::P1)
        } else if self.p2.best > self.p1.best {
            Some(Side::P2)
        } else {
            None
        }
    }
}
