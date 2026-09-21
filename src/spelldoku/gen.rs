//! The generator (F8, F2, F7): a pure function of `(seed, config)`.

use serde::{Deserialize, Serialize};

use super::geo::{bit, Geo, Size};
use super::rng::{mix, Rng};
use super::solve::{count_solutions, grade, Tech};
use super::symbols::Symbols;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Tier {
    /// The rung of the ladder a board of this tier must need, exactly (I2).
    pub fn tech(self) -> Tech {
        match self {
            Tier::Easy => Tech::Single,
            Tier::Medium => Tech::Subset,
            Tier::Hard => Tech::Intersection,
            Tier::Expert => Tech::Fish,
        }
    }

    pub fn parse(s: &str) -> Option<Tier> {
        match s {
            "easy" => Some(Tier::Easy),
            "medium" => Some(Tier::Medium),
            "hard" => Some(Tier::Hard),
            "expert" => Some(Tier::Expert),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Tier::Easy => "easy",
            Tier::Medium => "medium",
            Tier::Hard => "hard",
            Tier::Expert => "expert",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clue {
    Empty,
    /// A given shown as its symbol.
    Given(u8),
    /// A given the binding layer may show spelled out (Number Mode); Word Mode
    /// shows it as its glyph, since a spelled symbol would be a free spelling (I12).
    Spelled(u8),
    /// F2: a symbol's form, glyph id by glyph id, with some hidden. The value
    /// is NOT stored here.
    Fragment(Vec<Option<u32>>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    pub n: usize,
    pub tier: Tier,
    pub clues: Vec<Clue>,
    pub solution: Vec<u8>,
    pub seed: u64,
}

impl Puzzle {
    /// Placed values and per-cell candidate restrictions, as the solvers take them.
    pub fn constraints(&self, symbols: &Symbols) -> (Vec<u8>, Vec<u16>) {
        let geo = Geo::new(super::geo::size_of(self.n).expect("size"));
        let full = geo.full();
        let mut fixed = vec![0u8; self.clues.len()];
        let mut restrict = vec![full; self.clues.len()];
        for (i, c) in self.clues.iter().enumerate() {
            match c {
                Clue::Given(v) | Clue::Spelled(v) => fixed[i] = *v,
                Clue::Fragment(p) => restrict[i] = symbols.fragment_set(p) & full,
                Clue::Empty => {}
            }
        }
        (fixed, restrict)
    }
}

/// F7: which (size, tier) pairs exist for standard players.
pub fn standard_allowed(n: usize, tier: Tier) -> bool {
    matches!(
        (n, tier),
        (4, Tier::Easy) | (6, Tier::Easy) | (6, Tier::Medium) | (9, _)
    )
}

/// F7: Spell Jr sees only its own column.
pub fn jr_allowed(n: usize, tier: Tier) -> bool {
    matches!((n, tier), (4, Tier::Easy) | (6, Tier::Medium))
}

pub struct Config {
    pub size: Size,
    pub tier: Tier,
}

fn fill(geo: &Geo, rng: &mut Rng) -> Vec<u8> {
    let mut vals = vec![0u8; geo.cells()];
    fn rec(geo: &Geo, vals: &mut Vec<u8>, rng: &mut Rng) -> bool {
        let Some(i) = (0..vals.len()).find(|&i| vals[i] == 0) else { return true };
        let mut order: Vec<u8> = (1..=geo.size.n as u8).collect();
        rng.shuffle(&mut order);
        for v in order {
            if geo.peers[i].iter().all(|&p| vals[p] != v) {
                vals[i] = v;
                if rec(geo, vals, rng) {
                    return true;
                }
                vals[i] = 0;
            }
        }
        false
    }
    rec(geo, &mut vals, rng);
    vals
}

/// Up to `tries` reveal patterns for a symbol's form: at least one glyph shown
/// and at least one hidden, deterministic in the stream. An all-hidden pattern
/// would say only "a three-glyph symbol", which is length, not spelling.
fn patterns(form: &[u32], rng: &mut Rng, tries: usize) -> Vec<Vec<Option<u32>>> {
    let mut out = Vec::new();
    if form.len() < 2 {
        return out;
    }
    for _ in 0..tries {
        let mut p: Vec<Option<u32>> = form.iter().map(|c| Some(*c)).collect();
        let hide = 1 + rng.below(form.len() - 1);
        let mut idx: Vec<usize> = (0..form.len()).collect();
        rng.shuffle(&mut idx);
        for &k in idx.iter().take(hide) {
            p[k] = None;
        }
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// Aimed reveal patterns (`Symbols::aimed`): for each other symbol that agrees
/// with `v` somewhere, reveal exactly the agreeing glyphs, then a random
/// non-empty part of them. Each pattern fits at least two symbols.
fn aimed_patterns(symbols: &Symbols, v: u8, rng: &mut Rng) -> Vec<Vec<Option<u32>>> {
    let Some(form) = symbols.first(v) else { return Vec::new() };
    let mut out = Vec::new();
    let mut groups = symbols.agreements(v);
    rng.shuffle(&mut groups);
    for a in groups {
        let mut subsets = vec![a.clone()];
        if a.len() > 1 {
            let mut part = a.clone();
            rng.shuffle(&mut part);
            part.truncate(1 + rng.below(a.len() - 1));
            subsets.push(part);
        }
        for keep in subsets {
            let p: Vec<Option<u32>> = form.iter().enumerate().map(|(i, c)| keep.contains(&i).then_some(*c)).collect();
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

const ATTEMPTS: u64 = 4000;

/// Build one board. None only if every attempt failed (the sweep tests pin
/// that this does not happen for any allowed configuration).
pub fn generate(seed: u64, cfg: &Config, symbols: &Symbols) -> Option<Puzzle> {
    let geo = Geo::new(cfg.size);
    (0..ATTEMPTS).find_map(|attempt| attempt_with(&geo, seed, attempt, cfg, symbols))
}

/// One attempt of `generate`, on its own. A seed pack (D18) names the attempt
/// that grades, so a device rebuilds that board with a single dig.
pub fn generate_at(seed: u64, attempt: u64, cfg: &Config, symbols: &Symbols) -> Option<Puzzle> {
    attempt_with(&Geo::new(cfg.size), seed, attempt, cfg, symbols)
}

/// The grid of one attempt, before any fragment: filled, dug, and graded
/// exactly to the tier -- or None. Symbols play no part in it, which is why a
/// seed pack can be found ahead of time for every language at once.
pub fn grid_at(seed: u64, attempt: u64, cfg: &Config) -> Option<Vec<u8>> {
    let geo = Geo::new(cfg.size);
    let (_, fixed, _) = dig(&geo, seed, attempt, cfg)?;
    Some(fixed)
}

fn dig(geo: &Geo, seed: u64, attempt: u64, cfg: &Config) -> Option<(Vec<u8>, Vec<u8>, Rng)> {
    let target = cfg.tier.tech();
    let cells = geo.cells();
    let unrestricted = vec![geo.full(); cells];
    let mut rng = Rng::new(mix(seed, attempt));
    let solution = fill(geo, &mut rng);
    let mut fixed = solution.clone();
    let mut order: Vec<usize> = (0..cells).collect();
    rng.shuffle(&mut order);
    for &i in &order {
        let v = fixed[i];
        fixed[i] = 0;
        // A board the technique ladder finishes has exactly one solution
        // (every step is a sound deduction), so a finished grade proves
        // I1 as well as I2. The exhaustive count, which cost 99% of the
        // time on a 12x12, adds nothing here; the pinned golden digest
        // shows the boards are unchanged.
        let ok = grade(geo, &fixed, &unrestricted).is_some_and(|t| t <= target);
        if !ok {
            fixed[i] = v;
        }
    }
    if grade(geo, &fixed, &unrestricted) != Some(target) {
        return None;
    }
    Some((solution, fixed, rng))
}

fn attempt_with(geo: &Geo, seed: u64, attempt: u64, cfg: &Config, symbols: &Symbols) -> Option<Puzzle> {
    let (solution, mut fixed, mut rng) = dig(geo, seed, attempt, cfg)?;
    let full = geo.full();
    let target = cfg.tier.tech();
    let cells = geo.cells();
    let unrestricted = vec![full; cells];
    let mut restrict = unrestricted.clone();
    let mut fragment_at: Option<(usize, Vec<Option<u32>>)> = None;
    let want_fragment = match cfg.tier {
        Tier::Easy => false,
        Tier::Medium => rng.below(2) == 0,
        Tier::Hard | Tier::Expert => true,
    };
    if want_fragment {
        let mut givens: Vec<usize> = (0..cells).filter(|&i| fixed[i] > 0).collect();
        rng.shuffle(&mut givens);
        'found: for &g in &givens {
            let v = fixed[g];
            let Some(form) = symbols.first(v) else { continue };
            let cuts = if symbols.is_aimed() { aimed_patterns(symbols, v, &mut rng) } else { patterns(form, &mut rng, 10) };
            for p in cuts {
                let set = symbols.fragment_set(&p) & full;
                if set.count_ones() < 2 || set & bit(v) == 0 {
                    continue;
                }
                let mut f = fixed.clone();
                f[g] = 0;
                let mut r = restrict.clone();
                r[g] = set;
                if grade(geo, &f, &r) != Some(target) {
                    continue;
                }
                // F2: the fragment must be NECESSARY -- without it, not unique.
                let mut open = r.clone();
                open[g] = full;
                if count_solutions(geo, &f, &open, 2) < 2 {
                    continue;
                }
                fixed = f;
                restrict = r;
                fragment_at = Some((g, p));
                break 'found;
            }
        }
        if fragment_at.is_none() && matches!(cfg.tier, Tier::Hard | Tier::Expert) {
            return None;
        }
    }
    let _ = &restrict;
    // Above Easy, about a third of givens are marked spelled.
    let clues: Vec<Clue> = (0..cells)
        .map(|i| {
            if let Some((g, p)) = &fragment_at {
                if *g == i {
                    return Clue::Fragment(p.clone());
                }
            }
            if fixed[i] == 0 {
                Clue::Empty
            } else if cfg.tier != Tier::Easy && rng.below(3) == 0 {
                Clue::Spelled(fixed[i])
            } else {
                Clue::Given(fixed[i])
            }
        })
        .collect();
    return Some(Puzzle {
        n: cfg.size.n,
        tier: cfg.tier,
        clues,
        solution,
        seed,
    });
}

