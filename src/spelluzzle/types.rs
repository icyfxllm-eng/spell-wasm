//! Tiers, shapes and the board value (F1, F10).

use crate::spelldoku::rng::fnv;

/// F9 G2: no board needs more runes than this.
pub const MAX_RUNES: usize = 16;

/// Bumped whenever a change would alter the board a seed produces (F9
/// determinism). The golden-board test pins it.
pub const GEN_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Jr,
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Tier {
    pub const ALL: [Tier; 5] = [Tier::Jr, Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Jr => "jr",
            Tier::Easy => "easy",
            Tier::Medium => "medium",
            Tier::Hard => "hard",
            Tier::Expert => "expert",
        }
    }

    pub fn from_name(s: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.name() == s)
    }

    /// The bank bands this tier draws from. Spell Jr draws from Easy and Medium (F10).
    pub fn bands(self) -> &'static [&'static str] {
        match self {
            Tier::Jr => &["easy", "medium"],
            Tier::Easy => &["easy"],
            Tier::Medium => &["medium"],
            Tier::Hard => &["hard"],
            Tier::Expert => &["expert"],
        }
    }

    /// F10, with Eric's 2026-10-09 amendment: Expert words are 6 to 10 cells.
    pub fn shape(self) -> Shape {
        match self {
            Tier::Jr => Shape { spoken: 4, silent: 0, lo: 3, hi: 6, silent_share: None },
            Tier::Easy => Shape { spoken: 5, silent: 0, lo: 3, hi: 6, silent_share: None },
            Tier::Medium => Shape { spoken: 5, silent: 1, lo: 4, hi: 8, silent_share: Some((6, 10)) },
            Tier::Hard => Shape { spoken: 4, silent: 2, lo: 5, hi: 9, silent_share: Some((5, 10)) },
            Tier::Expert => Shape { spoken: 3, silent: 3, lo: 6, hi: 10, silent_share: Some((4, 10)) },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub spoken: usize,
    pub silent: usize,
    /// Word length range in cells, inclusive.
    pub lo: usize,
    pub hi: usize,
    /// G7: the share of a silent word's cells the spoken words must decode, as a
    /// (numerator, denominator) so every platform compares exactly.
    pub silent_share: Option<(u32, u32)>,
}

impl Shape {
    /// Every board has exactly one secret slot (F6).
    pub fn slots(&self) -> usize {
        self.spoken + self.silent + 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    Spoken,
    Silent,
    Secret,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub kind: SlotKind,
    /// The answer, one unit per cell.
    pub answer: Vec<char>,
    /// The rune standing at each cell.
    pub runes: Vec<u8>,
}

/// One puzzle. `rune_unit[r]` is the unit rune `r` stands for; runes are numbered
/// by a seeded permutation, never by alphabet order (F1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pub lang: String,
    pub tier: Tier,
    pub seed: u64,
    pub gen_version: u32,
    pub slots: Vec<Slot>,
    pub rune_unit: Vec<char>,
}

impl Board {
    /// Build a board from its words. `order` decides which rune number each
    /// distinct unit gets: `rune_order[i]` is the unit given rune `i`.
    pub fn from_words(
        lang: &str,
        tier: Tier,
        seed: u64,
        spoken: &[Vec<char>],
        silent: &[Vec<char>],
        secret: &[char],
        rune_order: Vec<char>,
    ) -> Board {
        let rune_of = |c: char| rune_order.iter().position(|&u| u == c).expect("unit has a rune") as u8;
        let mk = |kind: SlotKind, w: &[char]| Slot { kind, answer: w.to_vec(), runes: w.iter().map(|&c| rune_of(c)).collect() };
        let mut slots: Vec<Slot> = spoken.iter().map(|w| mk(SlotKind::Spoken, w)).collect();
        slots.extend(silent.iter().map(|w| mk(SlotKind::Silent, w)));
        slots.push(mk(SlotKind::Secret, secret));
        Board { lang: lang.to_string(), tier, seed, gen_version: GEN_VERSION, slots, rune_unit: rune_order }
    }

    /// A board with runes in sorted-unit order. For fixtures and tests only: a
    /// served board always uses the seeded permutation.
    pub fn from_words_sorted(lang: &str, tier: Tier, spoken: &[&str], silent: &[&str], secret: &str) -> Board {
        let v = |s: &str| s.chars().collect::<Vec<char>>();
        let sp: Vec<Vec<char>> = spoken.iter().map(|s| v(s)).collect();
        let si: Vec<Vec<char>> = silent.iter().map(|s| v(s)).collect();
        let sec = v(secret);
        let mut units: Vec<char> = sp.iter().chain(si.iter()).flatten().chain(sec.iter()).copied().collect();
        units.sort_unstable();
        units.dedup();
        Board::from_words(lang, tier, 0, &sp, &si, &sec, units)
    }

    pub fn n_runes(&self) -> usize {
        self.rune_unit.len()
    }

    pub fn cells(&self) -> usize {
        self.slots.iter().map(|s| s.answer.len()).sum()
    }

    pub fn words(&self) -> Vec<String> {
        self.slots.iter().map(|s| s.answer.iter().collect()).collect()
    }

    /// FNV-1a over a canonical form. F11 keeps these to refuse repeats; the form
    /// is independent of rune numbering so a re-permuted board counts as the same.
    pub fn hash(&self) -> u64 {
        let mut ws: Vec<String> = self.slots.iter().map(|s| format!("{:?}:{}", s.kind, s.answer.iter().collect::<String>())).collect();
        ws.sort();
        fnv(format!("{}|{}|{}", self.lang, self.tier.name(), ws.join(",")).as_bytes())
    }
}
