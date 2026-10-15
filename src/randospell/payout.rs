//! CC-RANDOSPELL v1.1 D-R3 -- the per-word payout, as one pure function (I11).
//!
//! The multiplier order is fixed and `ceil` is applied exactly once:
//!
//! 1. the roll value (the length the player SEES)
//! 2. jackpot x2 (a displayed 2 or 12, v1 D7)
//! 3. doubles x2 (the two underlying dice matched, F14)
//! 4. Stretch x1.5 (F13)
//! 5. Ear x1.5 (F17)
//! 6. the heat ladder (F11), or v1's streak x1.5 when heat is off
//! 7. ceil, once
//! 8. + the Call bonus (F12), added after every multiplier
//!
//! The total is capped at 150 per word. Everything is integer arithmetic: each
//! x1.5 is a (3, 2) factor, so nothing rounds before step 7.

pub const WORD_CAP: u32 = 150;
/// D-R3: Ear is not offered when a Stretch word's pre-heat product is above this.
pub const EAR_CEILING: u32 = 60;
pub const MAX_HEAT: u8 = 7;
pub const CALL_BONUS: u32 = 3;
pub const CALL_BONUS_SEVEN: u32 = 6;

/// F12: the three segments of the guess on the roll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    Low,
    Seven,
    High,
}

impl Call {
    /// Does the displayed roll fall in this segment? Low 2-6, Seven, High 8-12.
    pub fn hits(self, roll: usize) -> bool {
        match self {
            Call::Low => (2..=6).contains(&roll),
            Call::Seven => roll == 7,
            Call::High => (8..=12).contains(&roll),
        }
    }

    pub fn bonus(self) -> u32 {
        if self == Call::Seven { CALL_BONUS_SEVEN } else { CALL_BONUS }
    }
}

/// Which v1.1 mechanics are switched on. All false = v1 exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rules {
    pub heat: bool,
    pub call: bool,
    pub stretch: bool,
    pub ear: bool,
    pub doubles: bool,
}

/// F11: the heat ladder as an exact factor (numerator, denominator).
pub fn heat_factor(pips: u8) -> (u32, u32) {
    match pips {
        0..=2 => (1, 1),
        3..=4 => (3, 2),
        5..=6 => (2, 1),
        _ => (3, 1),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PayIn {
    /// The displayed roll (word length after any remap).
    pub roll: usize,
    /// Spell Jr: the die value is `roll - 1`, with no jackpot (F7).
    pub jr: bool,
    pub doubles: bool,
    pub stretch: bool,
    pub ear: bool,
    /// (numerator, denominator): the heat ladder or v1's streak factor.
    pub factor: (u32, u32),
    /// Added after every multiplier; 0 when there was no winning Call.
    pub call_bonus: u32,
}

fn base_value(roll: usize, jr: bool) -> u32 {
    if jr {
        roll as u32 - 1
    } else if roll == 2 || roll == 12 {
        roll as u32 * 2
    } else {
        roll as u32
    }
}

/// Steps 1-5 and 7 without heat: the "pre-heat product", ceiled for display and
/// for the Ear ceiling.
pub fn pre_heat(roll: usize, jr: bool, doubles: bool, stretch: bool, ear: bool) -> u32 {
    let mut num = base_value(roll, jr);
    let mut den = 1;
    if doubles {
        num *= 2;
    }
    if stretch {
        num *= 3;
        den *= 2;
    }
    if ear {
        num *= 3;
        den *= 2;
    }
    num.div_ceil(den)
}

/// D-R3 / I11: the whole payout. Pure: the same inputs always give the same
/// number, at most `WORD_CAP`.
pub fn payout(p: PayIn) -> u32 {
    let mut num = base_value(p.roll, p.jr);
    let mut den = 1;
    if p.doubles {
        num *= 2;
    }
    if p.stretch {
        num *= 3;
        den *= 2;
    }
    if p.ear {
        num *= 3;
        den *= 2;
    }
    num *= p.factor.0;
    den *= p.factor.1;
    (num.div_ceil(den) + p.call_bonus).min(WORD_CAP)
}

/// D-R3: is Ear on the table for this word? Never on a Stretch word whose
/// pre-heat product is already above the ceiling.
pub fn ear_offered(roll: usize, doubles: bool, stretch: bool) -> bool {
    !(stretch && pre_heat(roll, false, doubles, true, false) > EAR_CEILING)
}
