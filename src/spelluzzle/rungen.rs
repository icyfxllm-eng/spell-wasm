//! Expedition runs (F19): the generator and the independent X1-X11 checker. Test and build-step
//! only: the device never searches, it builds a run from a shipped record (`run.rs`).
//!
//! Ported from the census C15 trial (reports/spelluzzle_census.md, appendix), with the "guided"
//! weighting always on, because the plain v1 weighting does not finish Expert runs (8,920 of
//! 10,000). The checker shares nothing with the generator but the Lexicon API.

use std::collections::{BTreeMap, HashMap};

use super::gates::count_solutions;
use super::lex::{pattern, Lexicon, Word};
use super::types::{Board, Tier};
use crate::spelldoku::rng::{fnv, mix, Rng};

type U = Vec<char>;

// ---------------------------------------------------------------- checker

pub struct Bits {
    alpha: Vec<char>,
}
impl Bits {
    fn idx(&self, c: char) -> usize {
        self.alpha.binary_search(&c).expect("unit in alphabet")
    }
    fn bit(&self, c: char) -> u64 {
        1u64 << self.idx(c)
    }
    fn mask(&self, w: &[char]) -> u64 {
        w.iter().fold(0, |m, &c| m | self.bit(c))
    }
    /// cells of w whose unit is in `known`
    fn dec(&self, w: &[char], known: u64) -> usize {
        w.iter().filter(|&&c| known & self.bit(c) != 0).count()
    }
}

fn pc(x: u64) -> usize {
    x.count_ones() as usize
}

struct Acc {
    fails: Vec<&'static str>,
    full: bool,
}
impl Acc {
    fn new(full: bool) -> Acc {
        Acc { fails: Vec::new(), full }
    }
    /// record failure; true means stop now
    fn bad(&mut self, l: &'static str) -> bool {
        self.fails.push(l);
        !self.full
    }
}

pub fn tier_cap(t: Tier) -> usize {
    match t {
        Tier::Medium => 10,
        Tier::Hard => 11,
        _ => 12,
    }
}

fn g1(lex: &Lexicon, tier: Tier, w: &[char]) -> bool {
    let s = tier.shape();
    w.iter().all(|c| c.is_alphabetic()) && (s.lo..=s.hi).contains(&w.len()) && lex.in_band(tier, w)
}

/// X2 for `new` words against `earlier` words (and each other).
fn x2(lex: &Lexicon, tier: Tier, earlier: &[U], new: &[U], a: &mut Acc) -> bool {
    for w in new {
        if !g1(lex, tier, w) && a.bad("X2:G1") {
            return true;
        }
    }
    let all: Vec<&U> = earlier.iter().chain(new.iter()).collect();
    let ne = earlier.len();
    for j in ne..all.len() {
        for i in 0..j {
            if all[i] == all[j] && a.bad("X2:distinct") {
                return true;
            }
            if lex.group(all[i]).contains(all[j]) && a.bad("X2:collide") {
                return true;
            }
            if all[i].len() >= 4 && all[j].len() >= 4 && all[i][..4] == all[j][..4] && a.bad("X2:prefix4") {
                return true;
            }
        }
    }
    false
}


fn count_assign(lex: &Lexicon, b: &Bits, ws: &[U], n_spoken: usize, carried: u64, limit: u32) -> u32 {
    struct S<'a> {
        ans: &'a [char],
        cands: Vec<&'a [char]>,
    }
    let mut ss: Vec<S> = Vec::new();
    for (k, w) in ws.iter().enumerate() {
        let pat = pattern(w);
        let base: Vec<&[char]> = if k < n_spoken {
            let g = lex.group(w);
            if g.is_empty() {
                vec![w.as_slice()]
            } else {
                g.iter().filter(|x| x.len() == w.len() && pattern(x) == pat).map(|x| x.as_slice()).collect()
            }
        } else {
            lex.valid_with_pattern(&pat).map(|x| x.as_slice()).collect()
        };
        let cands: Vec<&[char]> = base
            .into_iter()
            .filter(|c| {
                w.iter().zip(c.iter()).all(|(&u, &x)| {
                    let uc = carried & b.bit(u) != 0;
                    let xc = carried & b.bit(x) != 0;
                    if uc {
                        x == u
                    } else {
                        !xc
                    }
                })
            })
            .collect();
        ss.push(S { ans: w, cands });
    }
    ss.sort_by_key(|s| s.cands.len());
    fn go(ss: &[S], k: usize, b: &Bits, carried: u64, r2u: &mut Vec<Option<char>>, used: &mut u64, found: &mut u32, limit: u32) {
        if *found >= limit {
            return;
        }
        if k == ss.len() {
            *found += 1;
            return;
        }
        let s = &ss[k];
        for c in &s.cands {
            let mut bound: Vec<usize> = Vec::new();
            let mut ok = true;
            for (&u, &x) in s.ans.iter().zip(c.iter()) {
                if carried & b.bit(u) != 0 {
                    continue;
                }
                let i = b.idx(u);
                match r2u[i] {
                    Some(p) => {
                        if p != x {
                            ok = false;
                            break;
                        }
                    }
                    None => {
                        if *used & b.bit(x) != 0 {
                            ok = false;
                            break;
                        }
                        r2u[i] = Some(x);
                        *used |= b.bit(x);
                        bound.push(i);
                    }
                }
            }
            if ok {
                go(ss, k + 1, b, carried, r2u, used, found, limit);
            }
            for i in bound {
                let x = r2u[i].take().unwrap();
                *used &= !b.bit(x);
            }
            if *found >= limit {
                return;
            }
        }
    }
    let mut r2u = vec![None; b.alpha.len()];
    let mut used = 0u64;
    let mut found = 0;
    go(&ss, 0, b, carried, &mut r2u, &mut used, &mut found, limit);
    found
}


fn frac_ge(sum: &[(u128, u128)], n: u128, d: u128) -> bool {
    // sum of fractions >= n/d, exact
    let (mut sn, mut sd) = (0u128, 1u128);
    for &(a, c) in sum {
        sn = sn * c + a * sd;
        sd *= c;
        let g = {
            let (mut x, mut y) = (sn, sd);
            while y != 0 {
                let t = x % y;
                x = y;
                y = t;
            }
            x.max(1)
        };
        sn /= g;
        sd /= g;
    }
    sn * d >= n * sd
}


/// Board 1: 4 spoken + secret. Gates X2, X3.
pub fn check_b1(lex: &Lexicon, tier: Tier, b: &Bits, ws: &[U], full: bool) -> Vec<&'static str> {
    let mut a = Acc::new(full);
    if x2(lex, tier, &[], ws, &mut a) {
        return a.fails;
    }
    let m: Vec<u64> = ws.iter().map(|w| b.mask(w)).collect();
    let n = 5;
    let all = m.iter().fold(0, |x, y| x | y);
    if pc(all) > tier_cap(tier) && a.bad("X3:cap") {
        return a.fails;
    }
    // G4
    for i in 0..n {
        let rest = (0..n).filter(|&j| j != i).fold(0u64, |x, j| x | m[j]);
        if pc(m[i] & rest) < 2 && a.bad("X3:G4") {
            return a.fails;
        }
    }
    // G5
    'g5: for i in 0..4 {
        for bit in 0..64 {
            if m[i] >> bit & 1 == 1 && m.iter().filter(|x| **x >> bit & 1 == 1).count() < 2 {
                if a.bad("X3:G5") {
                    return a.fails;
                }
                break 'g5;
            }
        }
    }
    // G6
    let others = m[..4].iter().fold(0, |x, y| x | y);
    if m[4] & !others != 0 && a.bad("X3:G6a") {
        return a.fails;
    }
    for j in 0..4 {
        let d = b.dec(&ws[4], m[j]);
        if d * 10 > 6 * ws[4].len() && a.bad("X3:G6b") {
            return a.fails;
        }
    }
    // G10 earned (denominator 12 covers holders 1..4)
    let spc: usize = ws[..4].iter().map(|w| w.len()).sum();
    let mut e12 = 0usize;
    for bit in 0..64 {
        let holders = (0..4).filter(|&i| m[i] >> bit & 1 == 1).count();
        if holders > 0 {
            let cnt: usize = ws[..4].iter().map(|w| w.iter().filter(|&&c| b.idx(c) == bit).count()).sum();
            e12 += cnt * 12 / holders;
        }
    }
    if e12 * 5 < 2 * 12 * spc && a.bad("X3:G10") {
        return a.fails;
    }
    // G11
    let mut fr = Vec::new();
    for i in 0..4 {
        let (mut dd, mut oc) = (0u128, 0u128);
        for j in 0..n {
            if j != i {
                oc += ws[j].len() as u128;
                dd += b.dec(&ws[j], m[i]) as u128;
            }
        }
        fr.push((dd, oc));
    }
    if !frac_ge(&fr, 4, 4) && a.bad("X3:G11") {
        return a.fails;
    }
    // G8
    let c = count_assign(lex, b, ws, 4, 0, 2);
    if c != 1 && a.bad("X3:G8") {
        return a.fails;
    }
    a.fails
}


/// exactly one validity-list fit for w given `known` units
fn one_fit(lex: &Lexicon, b: &Bits, w: &[char], known: u64) -> bool {
    let pat = pattern(w);
    let mut hits = 0;
    for x in lex.valid_with_pattern(&pat) {
        if w.iter().zip(x.iter()).all(|(&u, &c)| if known & b.bit(u) != 0 { c == u } else { known & b.bit(c) == 0 }) {
            hits += 1;
            if hits > 1 {
                return false;
            }
        }
    }
    hits == 1
}


fn closure(lex: &Lexicon, b: &Bits, words: &[U], mut known: u64, share: (u32, u32)) -> (u64, usize) {
    let mut solved = vec![false; words.len()];
    loop {
        let mut prog = false;
        for i in 0..words.len() {
            if solved[i] {
                continue;
            }
            let d = b.dec(&words[i], known) as u64;
            let len = words[i].len() as u64;
            if d * share.1 as u64 >= share.0 as u64 * len {
                let f = one_fit(lex, b, &words[i], known);
                if !f {
                    continue;
                }
                known |= b.mask(&words[i]);
                solved[i] = true;
                prog = true;
            }
        }
        if !prog {
            break;
        }
    }
    (known, solved.iter().filter(|&&s| s).count())
}


/// Board 2: 3 spoken + 2 silent + secret, given `earlier` (board 1 words) and carried runes.
pub fn check_b2(lex: &Lexicon, tier: Tier, b: &Bits, earlier: &[U], ws: &[U], full: bool) -> Vec<&'static str> {
    let mut a = Acc::new(full);
    let carried = earlier.iter().fold(0u64, |x, w| x | b.mask(w));
    if x2(lex, tier, earlier, ws, &mut a) {
        return a.fails;
    }
    let share = tier.shape().silent_share.unwrap();
    let m: Vec<u64> = ws.iter().map(|w| b.mask(w)).collect();
    let board = m.iter().fold(0, |x, y| x | y);
    if (pc(board) > 16 || pc(board | carried) > 24) && a.bad("X1") {
        return a.fails;
    }
    if pc(board & !carried) < 4 && a.bad("X4") {
        return a.fails;
    }
    let cells: usize = ws.iter().map(|w| w.len()).sum();
    let dec: usize = ws.iter().map(|w| b.dec(w, carried)).sum();
    if dec * 5 > 3 * cells && a.bad("X5:60%") {
        return a.fails;
    }
    if m[5] & !carried == 0 && a.bad("X5:secret") {
        return a.fails;
    }
    // X6
    'x6: for i in 0..3 {
        for bit in 0..64 {
            if m[i] >> bit & 1 == 1 && carried >> bit & 1 == 0 && m.iter().filter(|x| **x >> bit & 1 == 1).count() < 2 {
                if a.bad("X6") {
                    return a.fails;
                }
                break 'x6;
            }
        }
    }
    // X9
    let oth = m[..5].iter().fold(0, |x, y| x | y);
    if m[5] & !carried & !oth != 0 && a.bad("X9") {
        return a.fails;
    }
    // X7
    let known = carried | m[0] | m[1] | m[2];
    for i in 3..5 {
        let d = b.dec(&ws[i], known) as u64;
        let len = ws[i].len() as u64;
        if (d * share.1 as u64) < share.0 as u64 * len || d >= len {
            if a.bad("X7") {
                return a.fails;
            }
            break;
        }
    }
    // X8: earned share counting carried runes as decoded >= 25% of spoken cells
    let spc: usize = ws[..3].iter().map(|w| w.len()).sum();
    let mut e12 = 0usize;
    for bit in 0..64 {
        if carried >> bit & 1 == 1 {
            continue;
        }
        let holders = (0..3).filter(|&i| m[i] >> bit & 1 == 1).count();
        if holders > 0 {
            let cnt: usize = ws[..3].iter().map(|w| w.iter().filter(|&&c| b.idx(c) == bit).count()).sum();
            e12 += cnt * 12 / holders;
        }
    }
    if e12 * 4 < 12 * spc && a.bad("X8") {
        return a.fails;
    }
    // X10
    let c = count_assign(lex, b, ws, 3, carried, 2);
    if c != 1 && a.bad("X10") {
        return a.fails;
    }
    // X11 (board 2 silent words)
    let (_, n) = closure(lex, b, &ws[3..5], known, share);
    if n != 2 && a.bad("X11") {
        return a.fails;
    }
    a.fails
}

/// Vault: 4 silent + secret, given all earlier words (boards 1 and 2).
pub fn check_v(lex: &Lexicon, tier: Tier, b: &Bits, earlier: &[U], ws: &[U], full: bool) -> Vec<&'static str> {
    let mut a = Acc::new(full);
    let carried = earlier.iter().fold(0u64, |x, w| x | b.mask(w));
    if x2(lex, tier, earlier, ws, &mut a) {
        return a.fails;
    }
    let share = tier.shape().silent_share.unwrap();
    let m: Vec<u64> = ws.iter().map(|w| b.mask(w)).collect();
    let board = m.iter().fold(0, |x, y| x | y);
    if (pc(board) > 16 || pc(board | carried) > 24) && a.bad("X1") {
        return a.fails;
    }
    if pc(board & !carried) < 3 && a.bad("X4") {
        return a.fails;
    }
    let cells: usize = ws.iter().map(|w| w.len()).sum();
    let dec: usize = ws.iter().map(|w| b.dec(w, carried)).sum();
    if dec * 4 > 3 * cells && a.bad("X5:75%") {
        return a.fails;
    }
    if ws.iter().any(|w| b.dec(w, carried) == w.len()) && a.bad("X5:fullword") {
        return a.fails;
    }
    let oth = m[..4].iter().fold(0, |x, y| x | y);
    if m[4] & !carried & !oth != 0 && a.bad("X9") {
        return a.fails;
    }
    for i in 0..4 {
        let d = b.dec(&ws[i], carried) as u64;
        let len = ws[i].len() as u64;
        if (d * share.1 as u64) < share.0 as u64 * len || d >= len {
            if a.bad("X7") {
                return a.fails;
            }
            break;
        }
    }
    let c = count_assign(lex, b, ws, 0, carried, 2);
    if c != 1 && a.bad("X10") {
        return a.fails;
    }
    let (k, n) = closure(lex, b, &ws[..4], carried, share);
    if (n != 4 || m[4] & !k != 0) && a.bad("X11") {
        return a.fails;
    }
    a.fails
}

/// Whole-run checker, run from scratch on 16 words (never shares state with the generator).
pub fn check_run(lex: &Lexicon, tier: Tier, b: &Bits, w16: &[U]) -> Vec<&'static str> {
    let mut f = Vec::new();
    if w16.len() != 16 {
        return vec!["shape"];
    }
    f.extend(check_b1(lex, tier, b, &w16[..5], true));
    f.extend(check_b2(lex, tier, b, &w16[..5], &w16[5..11], true));
    f.extend(check_v(lex, tier, b, &w16[..11], &w16[11..16], true));
    let all = w16.iter().fold(0u64, |x, w| x | b.mask(w));
    if pc(all) > 24 {
        f.push("X1:run>24");
    }
    f
}


struct PoolX<'a> {
    w: Vec<&'a Word>,
    pre: Vec<u32>,
    mates: Vec<Vec<usize>>,
    bits: Vec<Vec<u8>>,
}

fn build_pool<'a>(lex: &'a Lexicon, tier: Tier) -> PoolX<'a> {
    let w = lex.pool(tier);
    let idx: HashMap<&[char], usize> = w.iter().enumerate().map(|(i, x)| (x.units.as_slice(), i)).collect();
    let mut pm: HashMap<&[char], u32> = HashMap::new();
    let mut pre = Vec::new();
    for x in &w {
        let n = pm.len() as u32;
        pre.push(*pm.entry(&x.units[..4]).or_insert(n));
    }
    let mates = w
        .iter()
        .map(|x| lex.group(&x.units).iter().filter_map(|g| idx.get(g.as_slice()).copied()).collect())
        .collect();
    let al = &lex.alphabet;
    let bits = w.iter().map(|x| x.units.iter().map(|c| al.binary_search(c).unwrap() as u8).collect()).collect();
    PoolX { w, pre, mates, bits }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    B1,
    B2,
    V,
}

struct RunState {
    ban: Vec<bool>,
    banpre: Vec<bool>,
}

fn conflict(p: &PoolX, i: usize, c: usize) -> bool {
    i == c || p.pre[i] == p.pre[c] || p.mates[c].contains(&i) || p.mates[i].contains(&c)
}

#[allow(clippy::too_many_arguments)]
fn construct(
    kind: Kind,
    tier: Tier,
    p: &PoolX,
    st: &RunState,
    carried: u64,
    elig_all: &[usize],
    elig_sil: &[usize],
    flagged: &[bool],
    shared0: usize,
    max_shared: usize,
    rng: &mut Rng,
) -> Option<(Vec<usize>, usize)> {
    let share = tier.shape().silent_share.unwrap();
    let (n, nsp, boardcap) = match kind {
        Kind::B1 => (5, 4, tier_cap(tier)),
        Kind::B2 => (6, 3, 16),
        Kind::V => (5, 0, 16),
    };
    let mut chosen: Vec<usize> = Vec::new();
    let (mut union, mut spoken_mask) = (0u64, 0u64);
    let mut count = [0u8; 64];
    let mut shared = shared0;
    while chosen.len() < n {
        let pos = chosen.len();
        let secret = pos + 1 == n;
        let is_spoken = pos < nsp;
        let single: u64 = (0..64).filter(|&bt| count[bt] == 1 && spoken_mask >> bt & 1 == 1 && carried >> bt & 1 == 0).fold(0, |m, bt| m | 1u64 << bt);
        let mut cands: Vec<(usize, u64)> = Vec::new();
        let mut total = 0u64;
        let elig = if kind == Kind::V && !secret { elig_sil } else { elig_all };
        for &i in elig {
            let w = p.w[i];
            if flagged[i] && shared >= max_shared {
                continue;
            }
            if chosen.iter().any(|&c| conflict(p, i, c)) {
                continue;
            }
            let u2 = union | w.mask;
            if pc(u2) > boardcap || pc(u2 | carried) > 24 {
                continue;
            }
            if kind != Kind::V && pos > 0 && pc(union & w.mask) < 2 {
                continue;
            }
            let len = w.units.len() as u64;
            if kind != Kind::B1 && !is_spoken && !secret {
                let known = carried | spoken_mask;
                let dcells = cells_i(p, i, known);
                if dcells * (share.1 as u64) < share.0 as u64 * len || dcells >= len {
                    continue;
                }
            }
            if secret {
                if w.mask & !union & !carried != 0 {
                    continue;
                }
                if kind != Kind::V && w.mask & single != single {
                    continue;
                }
            }
            let newr = if kind != Kind::B1 { 1 + pc(w.mask & !carried) as u64 } else { 1 };
            let wt = if kind == Kind::V || secret {
                if secret { 1 } else { newr * newr * newr }
            } else {
                let k = 1 + pc(w.mask & single) as u64;
                k * k * k * newr * newr
            };
            total += wt;
            cands.push((i, wt));
        }
        if cands.is_empty() {
            return None;
        }
        let mut roll = rng.next_u64() % total;
        let mut pick = cands[cands.len() - 1].0;
        for (i, wt) in &cands {
            if roll < *wt {
                pick = *i;
                break;
            }
            roll -= wt;
        }
        if flagged[pick] {
            shared += 1;
        }
        let m = p.w[pick].mask;
        if is_spoken {
            spoken_mask |= m;
        }
        union |= m;
        for bt in 0..64 {
            if m >> bt & 1 == 1 {
                count[bt] += 1;
            }
        }
        chosen.push(pick);
    }
    Some((chosen, shared))
}


#[derive(Default, Clone)]
pub struct RunOut {
    pub ok: bool,
    pub first_pass: bool,
    pub att: [u32; 3],
    pub att_last: [u32; 3],
    pub shared_prev: usize,
    pub redraws: u32,
    pub runes: usize,
    pub b2_dec: f64,
    pub v_dec: f64,
    pub x11_nolisten: bool,
    pub words: Vec<U>,
    pub idx: Vec<usize>,
    pub caps: u32,
    pub verified: bool,
}

type Rej = [BTreeMap<&'static str, u64>; 3];

fn gen_run(seed: u64, tier: Tier, lex: &Lexicon, p: &PoolX, b: &Bits, prev: &[usize], rej: &mut Rej) -> RunOut {
    let mut rng = Rng::new(mix(seed ^ fnv(format!("en:{}:expedition", tier.name()).as_bytes()), 1));
    let mut flagged = vec![false; p.w.len()];
    for &i in prev {
        flagged[i] = true;
    }
    let mut out = RunOut::default();
    let npre = *p.pre.iter().max().unwrap() as usize + 1;
    'run: for redraw in 0..40u32 {
        let max_shared = if redraw < 3 { 4 } else { usize::MAX };
        let att0 = out.att;
        let mut st = RunState { ban: vec![false; p.w.len()], banpre: vec![false; npre] };
        let mut shared = 0usize;
        let mut all_idx: Vec<usize> = Vec::new();
        let mut all_words: Vec<U> = Vec::new();
        for (bi, kind) in [Kind::B1, Kind::B2, Kind::V].into_iter().enumerate() {
            let carried = all_words.iter().fold(0u64, |x, w| x | b.mask(w));
            let share = tier.shape().silent_share.unwrap();
            let bcap = if kind == Kind::B1 { tier_cap(tier) } else { 16 };
            let elig_all: Vec<usize> = (0..p.w.len())
                .filter(|&i| !(st.ban[i] || st.banpre[p.pre[i] as usize] || pc(p.w[i].mask) > bcap || pc(p.w[i].mask | carried) > 24))
                .collect();
            let elig: Vec<usize> = (0..p.w.len())
                .filter(|&i| {
                    if st.ban[i] || st.banpre[p.pre[i] as usize] || pc(p.w[i].mask) > bcap || pc(p.w[i].mask | carried) > 24 {
                        return false;
                    }
                    if kind == Kind::V {
                        let d = cells_i(p, i, carried);
                        let len = p.w[i].units.len() as u64;
                        return d * share.1 as u64 >= share.0 as u64 * len && d < len;
                    }
                    true
                })
                .collect();
            let mut got = None;
            for _ in 0..2000 {
                out.att[bi] += 1;
                let cres = construct(kind, tier, p, &st, carried, &elig_all, &elig, &flagged, shared, max_shared, &mut rng);
                let Some((ch, sh)) = cres else {
                    *rej[bi].entry("construct").or_default() += 1;
                    continue;
                };
                if kind != Kind::B1 {
                    // cheap mask-level screen (same rules as X4/X5; the checker stays authoritative)
                    let un = ch.iter().fold(0u64, |x, &i| x | p.w[i].mask);
                    let cells: u64 = ch.iter().map(|&i| p.w[i].units.len() as u64).sum();
                    let dec: u64 = ch.iter().map(|&i| cells_i(p, i, carried)).sum();
                    let (minnew, num, den, lab4, lab5) = if kind == Kind::B2 { (4, 3, 5, "X4", "X5:60%") } else { (3, 3, 4, "X4", "X5:75%") };
                    if pc(un & !carried) < minnew {
                        *rej[bi].entry(lab4).or_default() += 1;
                        continue;
                    }
                    if dec * den > num * cells {
                        *rej[bi].entry(lab5).or_default() += 1;
                        continue;
                    }
                }
                let ws: Vec<U> = ch.iter().map(|&i| p.w[i].units.clone()).collect();
                let f = match kind {
                    Kind::B1 => check_b1(lex, tier, b, &ws, false),
                    Kind::B2 => check_b2(lex, tier, b, &all_words, &ws, false),
                    Kind::V => check_v(lex, tier, b, &all_words, &ws, false),
                };
                if f.is_empty() {
                    got = Some((ch, sh, ws));
                    break;
                }
                *rej[bi].entry(f[0]).or_default() += 1;
            }
            let Some((ch, sh, ws)) = got else {
                out.caps += 1;
                out.redraws = redraw + 1;
                continue 'run;
            };
            shared = sh;
            for &i in &ch {
                st.ban[i] = true;
                st.banpre[p.pre[i] as usize] = true;
                for &g in &p.mates[i] {
                    st.ban[g] = true;
                }
            }
            all_idx.extend(ch);
            all_words.extend(ws);
        }
        out.ok = true;
        out.att_last = [out.att[0] - att0[0], out.att[1] - att0[1], out.att[2] - att0[2]];
        out.shared_prev = prev.iter().filter(|i| all_idx.contains(i)).count();
        out.first_pass = redraw == 0;
        out.redraws = redraw;
        out.idx = all_idx;
        out.words = all_words;
        break;
    }
    if out.ok {
        let w = &out.words;
        let c1 = w[..5].iter().fold(0u64, |x, y| x | b.mask(y));
        let c2 = w[..11].iter().fold(0u64, |x, y| x | b.mask(y));
        let all = w.iter().fold(0u64, |x, y| x | b.mask(y));
        out.runes = pc(all);
        let cells = |ws: &[U]| ws.iter().map(|x| x.len()).sum::<usize>() as f64;
        let dec = |ws: &[U], k: u64| ws.iter().map(|x| b.dec(x, k)).sum::<usize>() as f64;
        out.b2_dec = dec(&w[5..11], c1) / cells(&w[5..11]);
        out.v_dec = dec(&w[11..16], c2) / cells(&w[11..16]);
        let sh = tier.shape().silent_share.unwrap();
        let (k, n) = closure(lex, b, &w[11..15], c2, sh);
        out.x11_nolisten = n == 4 && w[15].iter().all(|&c| k & b.bit(c) != 0);
        out.verified = check_run(lex, tier, b, w).is_empty();
    }
    out
}


fn cells_i(p: &PoolX, i: usize, known: u64) -> u64 {
    p.bits[i].iter().filter(|&&b| known >> b & 1 == 1).count() as u64
}

/// Run generator and checker bound to one lexicon and tier.
pub struct RunGen<'a> {
    lex: &'a Lexicon,
    tier: Tier,
    pool: PoolX<'a>,
    bits: Bits,
}

impl<'a> RunGen<'a> {
    pub fn new(lex: &'a Lexicon, tier: Tier) -> RunGen<'a> {
        RunGen { lex, tier, pool: build_pool(lex, tier), bits: Bits { alpha: lex.alphabet.clone() } }
    }

    /// Pool indexes of a run for `seed`, or None if it never finished within the redraw bound.
    /// `prev` are the pool indexes of the previous run (at most four are shared where possible).
    pub fn generate(&self, seed: u64, prev: &[usize]) -> Option<Vec<usize>> {
        let mut rej: Rej = Default::default();
        let out = gen_run(seed, self.tier, self.lex, &self.pool, &self.bits, prev, &mut rej);
        out.ok.then_some(out.idx)
    }

    /// The independent whole-run checker on 16 words. Empty means the run is fair.
    pub fn check(&self, words: &[Vec<char>]) -> Vec<&'static str> {
        check_run(self.lex, self.tier, &self.bits, words)
    }

    pub fn words(&self, idx: &[usize]) -> Vec<Vec<char>> {
        idx.iter().map(|&i| self.pool.w[i].units.clone()).collect()
    }
}

#[allow(dead_code)]
fn _unused(_: &Board) {}
