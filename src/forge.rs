//! CC-LETTER-FORGE D2 — the puzzle generator.
//!
//! A puzzle is seven typing units, one of them designated the centre. A
//! submission counts when it uses only those seven, includes the centre, and
//! meets the language's minimum length. Acceptance gates per puzzle: at least
//! twenty valid words, and at least one pangram-equivalent using all seven.
//!
//! # Pangram-first, not seven-units-first
//!
//! The obvious generator picks seven units at random and checks the gates.
//! That fails the pangram gate most of the time and burns seeds walking. This
//! picks a PANGRAM first — a word with exactly seven distinct units — and uses
//! its own units as the honeycomb. The pangram gate is then satisfied by
//! construction rather than by luck, and pool size is the only gate that can
//! actually fail. It is also how the format's originals are built.
//!
//! # Units are per-script
//!
//! D1: "whatever the registry already defines for input". Latin, Cyrillic,
//! Arabic and Devanagari are characters. Korean decomposes to JAMO, because a
//! jamo forge assembling syllable blocks is the mode's headline. Chinese
//! splits into tone-numbered pinyin SYLLABLES, since that is what the player
//! composes.
//!
//! Determinism is the whole contract (I1): (lang, seed) reproduces a puzzle
//! byte for byte, so a Daily Forge is the same on every device and a bug
//! report is replayable.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use crate::norm::fold_strict;

/// (folded word, its distinct units) for every word in a language.
/// (folded word, unit COUNT, distinct units). The count is stored because
/// the min-length gate needs it — deriving it in pool() re-decomposed every
/// word on every call and made the memo pointless.
type Decomposed = Rc<Vec<(String, usize, BTreeSet<String>)>>;

thread_local! {
    /// Decomposing the bank is the expensive part — units_of allocates per
    /// character — and both the candidate scan and every pool() call need it.
    /// Doing it per call made a 365-seed report take longer than ten minutes;
    /// memoized, the whole report runs in seconds. Same lazy-derived shape as
    /// word_index, and for the same reason: it cannot drift from the bank
    /// because it has no independent existence.
    static DECOMPOSED: RefCell<HashMap<String, Decomposed>> = RefCell::new(HashMap::new());
}

fn decomposed(lang: &str) -> Decomposed {
    decomposed_in(lang, &crate::experience::TIERS, false)
}

/// The bank restricted to `tiers` — and, for a Spell Jr board, to kid-safe
/// words (D8: Jr Easy/Medium = standard Easy/Medium ∩ kidSafe). Memoized per
/// (lang, tiers, kid_only).
fn decomposed_in(lang: &str, tiers: &[&str], kid_only: bool) -> Decomposed {
    let memo = format!("{lang}|{}|{}", tiers.join(","), if kid_only { "kid" } else { "all" });
    if let Some(hit) = DECOMPOSED.with(|m| m.borrow().get(&memo).cloned()) {
        return hit;
    }
    let mut out: Vec<(String, usize, BTreeSet<String>)> = Vec::new();
    for tier in tiers {
        for w in crate::words::tier_for(lang, tier) {
            if kid_only && !crate::kid_filter::kid_allowed(lang, w) {
                continue;
            }
            let folded = fold_strict(w.split('|').next().unwrap_or(w));
            let seq = units_of(lang, w);
            let n = seq.len();
            out.push((folded, n, seq.into_iter().collect()));
        }
    }
    out.sort();
    out.dedup();
    let rc = Rc::new(out);
    DECOMPOSED.with(|m| m.borrow_mut().insert(memo, rc.clone()));
    rc
}

/// D2's per-language floor, as PROPOSED in the spec pending Eric's review of
/// the table: 4 for Latin-script, 2 blocks for ko, 2 units for ja/zh.
///
/// Measured in UNITS, not characters — 2 for Korean means two syllable
/// blocks, which is four to six jamo.
pub fn forge_min_units(lang: &str) -> usize {
    match lang {
        "ko" | "ja" | "zh" => 2,
        _ => 4,
    }
}

/// Split a word into the units a player of `lang` composes.
pub fn units_of(lang: &str, word: &str) -> Vec<String> {
    let w = fold_strict(word.split('|').next().unwrap_or(word));
    match lang {
        // Korean: the honeycomb holds jamo and the player assembles blocks,
        // so a word's units are its jamo, not its syllables.
        "ko" => {
            let mut out = Vec::new();
            for c in w.chars() {
                match crate::hangul::parts(c) {
                    Some((i, m, f)) => {
                        out.push(i.to_string());
                        out.push(m.to_string());
                        // a syllable with no final consonant reports a filler
                        if f != '\u{0}' && !f.is_whitespace() {
                            out.push(f.to_string());
                        }
                    }
                    None => out.push(c.to_string()),
                }
            }
            out
        }
        // Mandarin: pinyin LETTERS (and tone digits, which the zh keyboard
        // puts on their own row), not syllables.
        //
        // Syllables were the first reading of D1 and they make the mode
        // impossible: measured, the most distinct syllables in any Chinese
        // word is FOUR — 4,651 of 5,810 words are two syllables — so a
        // seven-syllable pangram does not exist and zh produced zero puzzles
        // from 365 seeds. Letters are also what the honeycomb can actually
        // hold: seven syllable tiles would be a different game. The player
        // still composes pinyin; this only changes what a "unit" is.
        "zh" => w.chars().map(|c| c.to_string()).collect(),
        _ => w.chars().map(|c| c.to_string()).collect(),
    }
}

/// Seven units, one designated centre, and the seed that produced them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    /// Sorted, so a puzzle has ONE representation regardless of which pangram
    /// produced it — two seeds landing on the same honeycomb are the same
    /// puzzle, and a golden test can compare them directly.
    pub units: Vec<String>,
    pub centre: String,
    pub seed: u64,
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Words of `lang` whose distinct-unit count is exactly seven: the pangram
/// candidates a honeycomb can be built from.
fn pangram_candidates(lang: &str, tiers: &[&str], kid_only: bool) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = decomposed_in(lang, tiers, kid_only)
        .iter()
        .filter(|(_, _, u)| u.len() == 7)
        .map(|(_, _, u)| u.iter().cloned().collect())
        .collect();
    out.sort();
    out.dedup();          // two pangrams over the same seven are one puzzle
    out
}

/// Every word playable on this puzzle: built only from its units, containing
/// the centre, and at least `forge_min_units` long.
/// Drawn only from `tiers` (CC-ONBOARD-JR: a Spell Jr pool holds Easy + Medium).
fn pool_in(lang: &str, p: &Puzzle, tiers: &[&str], kid_only: bool) -> Vec<String> {
    let allowed: BTreeSet<&String> = p.units.iter().collect();
    let min = forge_min_units(lang);
    let mut out: Vec<String> = decomposed_in(lang, tiers, kid_only)
        .iter()
        .filter(|(_, n, u)| {
            *n >= min && u.contains(&p.centre) && u.iter().all(|x| allowed.contains(x))
        })
        .map(|(w, _, _)| w.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Does `word` use all seven units?
pub fn is_pangram(lang: &str, p: &Puzzle, word: &str) -> bool {
    let u: BTreeSet<String> = units_of(lang, word).into_iter().collect();
    p.units.iter().all(|x| u.contains(x))
}

/// D2's pool gate, per language.
///
/// D2 states a flat 20. That number is calibrated for the format's originals,
/// which draw on a ~30k-word answer list; our largest bank is 6.8k and English
/// is 3.2k. At a flat 20, six languages could not generate a puzzle at all and
/// none reached D2's 80% first-try bar. These values are the highest threshold
/// that still yields ZERO ungenerable seeds across a 365-seed year, measured
/// by `calibrate_min_pool` below — not guesses. Raise them as banks grow.
pub fn min_pool(lang: &str) -> usize {
    match lang {
        // These clear D2's flat 20 with room to spare (measured ceilings
        // 25-30), so they keep the number the spec asked for.
        "en" | "es" | "fr" | "de" | "pt" | "fil" | "sw" | "ar" | "ko" => 20,
        // Reduced to what the bank can actually sustain, each a few below its
        // measured ceiling so a bank edit does not make the mode ungenerable
        // overnight. Ceilings were vi 17, zh 15, ja 15, ru 12, pl 10.
        "vi" => 15,
        "ja" | "zh" => 12,
        "ru" => 10,
        "pl" => 8,
        // hi's ceiling is FIVE. A five-word puzzle is not a puzzle; hi is
        // flagged not-ready rather than shipped degraded — see forge_ready.
        "hi" => 4,
        _ => 20,
    }
}

/// Can this language sustain a puzzle worth playing?
///
/// Generability is not the same question as playability. Hindi CAN produce a
/// puzzle if the gate drops to five words, and a five-word honeycomb is a
/// worse experience than no honeycomb — the rank ladder has nothing to climb
/// and "Reveal remaining" is the whole game. Its bank is 2,674 words against
/// English's 3,206 spread over a much larger character set, so few words fit
/// inside any seven characters.
///
/// The mode should offer the languages that work and stay silent about the
/// rest, rather than shipping everyone a thin board. Revisit as banks grow —
/// this is a data limit, not a language limit.
pub fn forge_ready(lang: &str) -> bool {
    min_pool(lang) >= 8
}

/// CC-ONBOARD-JR — the pool gate for a Spell Jr board, whose letters and pool
/// come from Easy + Medium only. Measured by `calibrate_junior_min_pool` at the
/// screen's walk (512): the highest gate that yields a board on every day of a
/// year, kept a step or two below that ceiling as `min_pool` is, and never
/// above the standard gate. Ceilings measured 2026-09-11: en 29, es 25, fr 18,
/// de 35, pt 25, fil 33, sw 28, ar 11, ko 10, zh 11, ru 11, ja 9, pl 7; vi has
/// no Easy + Medium pangram at all.
pub fn junior_min_pool(lang: &str) -> usize {
    match lang {
        "fr" => 16,
        "ar" | "zh" => 9,
        // ja keeps a margin of one: its ceiling is 9 and the floor is 8.
        "ko" | "ja" => 8,
        // Below forge_ready's floor of eight: Spell Jr is told the board is
        // unavailable rather than dealt a thin one, the same call hi gets.
        "pl" => 7,
        "vi" => 0,
        other => min_pool(other),
    }
}

/// Can this language sustain a board for `exp`? Standard is exactly
/// `forge_ready`. A Spell Jr board must clear the same floor of eight words
/// from Easy + Medium alone.
pub fn forge_ready_for(exp: crate::experience::Experience, lang: &str) -> bool {
    forge_ready(lang) && (exp == crate::experience::Experience::Standard || junior_min_pool(lang) >= 8)
}

/// Kept for callers that want D2's literal number.
pub const MIN_POOL: usize = 20;

/// Generate the puzzle for `(lang, seed)`, walking seed+1 until the gates
/// pass. Returns the puzzle and its pool.
///
/// `walk_limit` bounds the search so a language that can never satisfy the
/// gates reports that loudly instead of hanging — an empty result is a
/// generability failure worth surfacing, not a puzzle worth shipping.
pub fn generate(lang: &str, seed: u64, walk_limit: u32) -> Option<(Puzzle, Vec<String>, u32)> {
    generate_with(lang, seed, walk_limit, min_pool(lang))
}

/// `generate` with an explicit pool gate, so the calibration diagnostic can
/// sweep thresholds without editing the table it is calibrating.
pub fn generate_with(
    lang: &str,
    seed: u64,
    walk_limit: u32,
    gate: usize,
) -> Option<(Puzzle, Vec<String>, u32)> {
    generate_in(lang, seed, walk_limit, gate, &crate::experience::TIERS, false)
}

/// CC-ONBOARD-JR I5 — the day's board for `exp`. Standard is exactly
/// `generate`. A Spell Jr board builds BOTH its seven letters and its answer
/// pool from the tiers the resolver allows, kid-safe words only (D8), so
/// neither the honeycomb nor "Reveal remaining" can hand a junior player a
/// Hard word or an unfriendly one.
pub fn generate_for(
    exp: crate::experience::Experience,
    lang: &str,
    seed: u64,
    walk_limit: u32,
) -> Option<(Puzzle, Vec<String>, u32)> {
    let tiers = crate::experience::allowed_tiers(exp, "letter_forge");
    let (gate, kid_only) = match exp {
        crate::experience::Experience::Standard => (min_pool(lang), false),
        crate::experience::Experience::Junior => (junior_min_pool(lang), true),
    };
    generate_in(lang, seed, walk_limit, gate, &tiers, kid_only)
}

fn generate_in(
    lang: &str,
    seed: u64,
    walk_limit: u32,
    gate: usize,
    tiers: &[&str],
    kid_only: bool,
) -> Option<(Puzzle, Vec<String>, u32)> {
    let cands = pangram_candidates(lang, tiers, kid_only);
    if cands.is_empty() {
        return None;
    }
    for step in 0..walk_limit {
        let s = seed.wrapping_add(step as u64);
        let mut st = s;
        let units = &cands[(next_u64(&mut st) % cands.len() as u64) as usize];
        let centre = units[(next_u64(&mut st) % units.len() as u64) as usize].clone();
        let p = Puzzle { units: units.clone(), centre, seed: s };
        let words = pool_in(lang, &p, tiers, kid_only);
        if words.len() >= gate {
            return Some((p, words, step));
        }
    }
    None
}

// ---- D6 scoring and D4 ranks (pure, so the screen holds no rules) ----

/// D6: a word scores its unit count; a pangram takes a flat bonus on top.
pub const PANGRAM_BONUS: u32 = 7;

/// What one submission is worth. Korean scores JAMO, not blocks — the unit is
/// the thing the honeycomb holds, so a two-block word built from five jamo
/// scores five. Anything else would make the tiles and the score disagree.
pub fn score_word(lang: &str, p: &Puzzle, word: &str) -> u32 {
    let n = units_of(lang, word).len() as u32;
    if is_pangram(lang, p, word) {
        n + PANGRAM_BONUS
    } else {
        n
    }
}

/// Everything the pool is worth — the denominator the rank ladder divides by,
/// and the number D4 forbids showing before "Reveal remaining".
pub fn max_score(lang: &str, p: &Puzzle, pool: &[String]) -> u32 {
    pool.iter().map(|w| score_word(lang, p, w)).sum()
}

/// D4's ladder. Deliberately generous at the bottom: the intent is that a
/// beginner can reach the first rung and feel finished, so "Good" sits at a
/// twentieth of the board rather than a quarter. Only the TOP rung requires
/// the whole pool, and reaching it is not expected — it is the completionist's
/// horizon, not a bar to clear.
pub const RANK_THRESHOLDS: [u32; 4] = [5, 25, 55, 100];

/// i18n keys, in ladder order. The screen never spells a rank name itself.
pub const RANK_KEYS: [&str; 4] =
    ["forge.rank.good", "forge.rank.great", "forge.rank.amazing", "forge.rank.master"];

/// Index into [`RANK_KEYS`], or `None` below the first rung.
///
/// Integer maths on purpose: `score * 100 / max` cannot drift the way a float
/// comparison against 5.0 can, and a rank that flickers at the boundary would
/// be felt immediately in a mode built on hundreds of submissions.
pub fn rank_index(score: u32, max: u32) -> Option<usize> {
    if max == 0 {
        return None;
    }
    let pct = (score as u64 * 100 / max as u64) as u32;
    RANK_THRESHOLDS.iter().rposition(|t| pct >= *t)
}

/// Progress toward the NEXT rung, 0..=100 — what the bar fills to. At the top
/// rung it reads full. This is a ratio, never a count: showing "12 of 214" is
/// exactly what D4 forbids.
pub fn rank_progress(score: u32, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let pct = (score as u64 * 100 / max as u64) as u32;
    match rank_index(score, max) {
        Some(i) if i + 1 >= RANK_THRESHOLDS.len() => 100,
        Some(i) => {
            let (lo, hi) = (RANK_THRESHOLDS[i], RANK_THRESHOLDS[i + 1]);
            ((pct - lo) * 100 / (hi - lo)).min(100)
        }
        None => (pct * 100 / RANK_THRESHOLDS[0]).min(100),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// I1 — the whole contract. Same (lang, seed) reproduces the puzzle.
    #[test]
    fn generation_is_deterministic() {
        for lang in ["en", "de", "es"] {
            let a = generate(lang, 42, 64);
            let b = generate(lang, 42, 64);
            assert_eq!(a, b, "{lang}: same seed must reproduce the puzzle and pool");
            assert!(a.is_some(), "{lang}: seed 42 produced nothing");
        }
    }

    /// I2 — every shipped puzzle satisfies both gates. The pangram gate holds
    /// by construction; this proves the construction actually holds.
    #[test]
    fn every_puzzle_meets_both_gates() {
        for lang in ["en", "de", "es"] {
            for seed in [1u64, 7, 99, 1234, 65_535] {
                let Some((p, words, _)) = generate(lang, seed, 64) else {
                    panic!("{lang}/{seed}: no puzzle within the walk limit");
                };
                assert_eq!(p.units.len(), 7, "{lang}/{seed}: seven units");
                assert!(p.units.contains(&p.centre), "{lang}/{seed}: centre is one of the seven");
                assert!(words.len() >= MIN_POOL, "{lang}/{seed}: pool {} < {MIN_POOL}", words.len());
                assert!(
                    words.iter().any(|w| is_pangram(lang, &p, w)),
                    "{lang}/{seed}: no pangram — the pangram-first construction failed"
                );
            }
        }
    }

    /// Every pooled word is actually playable: only puzzle units, contains
    /// the centre, long enough. This is the half of I3 the generator owns.
    #[test]
    fn the_pool_contains_only_playable_words() {
        let (p, words, _) = generate("en", 7, 64).expect("a puzzle");
        let allowed: BTreeSet<&String> = p.units.iter().collect();
        for w in &words {
            let u = units_of("en", w);
            assert!(u.len() >= forge_min_units("en"), "{w}: shorter than the floor");
            assert!(u.iter().any(|x| *x == p.centre), "{w}: missing the centre unit");
            assert!(u.iter().all(|x| allowed.contains(x)), "{w}: uses a unit not on the board");
        }
    }

    /// Korean decomposes to jamo, not syllables — the mode's headline.
    #[test]
    fn korean_units_are_jamo() {
        // 강 = ㄱ + ㅏ + ㅇ
        let u = units_of("ko", "\u{AC15}");
        assert_eq!(u.len(), 3, "a closed syllable is three jamo, saw {u:?}");
        // 가 = ㄱ + ㅏ, no final
        let open = units_of("ko", "\u{AC00}");
        assert_eq!(open.len(), 2, "an open syllable is two jamo, saw {open:?}");
    }

    /// Chinese units are pinyin LETTERS, not syllables.
    ///
    /// Syllables were the first reading and they made the mode impossible:
    /// the most distinct syllables in any Chinese word is four, so a
    /// seven-syllable pangram does not exist and zh generated nothing from
    /// 365 seeds. This pins the corrected reading — and that the hanzi half
    /// never leaks into the honeycomb, since the player composes pinyin.
    #[test]
    fn chinese_units_are_pinyin_letters() {
        assert_eq!(
            units_of("zh", "dong4wu4|\u{52A8}\u{7269}"),
            vec!["d", "o", "n", "g", "4", "w", "u", "4"]
        );
        assert_eq!(units_of("zh", "shu1"), vec!["s", "h", "u", "1"]);
        assert!(
            !units_of("zh", "shu1|\u{4E66}").iter().any(|u| u == "\u{4E66}"),
            "the hanzi half must not reach the honeycomb"
        );
    }

    /// Every language the mode offers can actually produce a puzzle at its
    /// own gate, across a full year of seeds. This is the promise forge_ready
    /// makes; hi is excluded because it cannot keep it.
    #[test]
    fn every_ready_language_generates_a_full_year() {
        for lang in ["en","es","fr","de","pt","pl","vi","ko","ja","fil","ru","sw","ar","zh"] {
            assert!(forge_ready(lang), "{lang} should be offered");
            for day in [0u64, 90, 180, 364] {
                assert!(
                    generate(lang, day.wrapping_mul(0x9E3779B9), 64).is_some(),
                    "{lang}: day {day} produced no puzzle at gate {}", min_pool(lang)
                );
            }
        }
        assert!(!forge_ready("hi"), "hi cannot sustain a playable pool and must not be offered");
    }

    /// F3's generability report: of 365 seeds (a year of Daily Forges), how
    /// many produce a puzzle on the FIRST try rather than walking? D2 flags
    /// any language under 80% for review with the failing constraint.
    ///
    /// Ignored by default — it is a report, not an assertion, and it walks
    /// every bank. Run with: cargo test --lib generability -- --ignored --nocapture
    #[test]
    #[ignore]
    fn generability_report() {
        println!("{:5} {:>9} {:>9} {:>9} {:>8}  {}", "lang", "first-try", "walked", "failed", "med pool", "verdict");
        for lang in ["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "ru", "sw", "ar", "hi", "zh"] {
            let (mut first, mut walked, mut failed) = (0, 0, 0);
            let mut pools: Vec<usize> = Vec::new();
            for day in 0..365u64 {
                match generate(lang, day.wrapping_mul(0x9E3779B9), 64) {
                    Some((_, words, 0)) => { first += 1; pools.push(words.len()); }
                    Some((_, words, _)) => { walked += 1; pools.push(words.len()); }
                    None => failed += 1,
                }
            }
            pools.sort_unstable();
            let med = pools.get(pools.len() / 2).copied().unwrap_or(0);
            let pct = first as f64 / 365.0 * 100.0;
            let verdict = if failed > 0 { "UNGENERABLE" }
                          else if pct < 80.0 { "under 80% — review" }
                          else { "ok" };
            println!("{lang:5} {first:9} {walked:9} {failed:9} {med:8}  {verdict}");
        }
    }

    /// A puzzle is its honeycomb, not the pangram that produced it: units are
    /// sorted so two seeds landing on the same seven compare equal.
    #[test]
    fn units_are_canonically_ordered() {
        let (p, _, _) = generate("en", 3, 64).expect("a puzzle");
        let mut sorted = p.units.clone();
        sorted.sort();
        assert_eq!(p.units, sorted, "units must be canonically ordered");
    }
}

#[cfg(test)]
mod diag {
    use super::*;
    use std::collections::BTreeMap;
    const JR_LANGS: [&str; 15] = ["en", "es", "fr", "de", "pt", "fil", "sw", "ar", "ko", "vi", "ja", "zh", "ru", "pl", "hi"];

    /// (Easy + Medium pangram candidates, the smallest best pool any day of a
    /// year reaches within `walk` steps). A junior board generates on EVERY day
    /// at gate g exactly when g <= that floor. Pool size depends only on
    /// (letters, centre), so each distinct board is sized once and the walk is
    /// a lookup — a year of boards in milliseconds instead of minutes.
    fn junior_floor(lang: &str, walk: u32) -> (usize, usize) {
        let tiers = &crate::experience::TIERS[..2];
        let cands = pangram_candidates(lang, tiers, true);
        if cands.is_empty() {
            return (0, 0);
        }
        let mut sized: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::new();
        let floor = (0..365u64)
            .map(|d| {
                let seed = d.wrapping_mul(0x9E3779B9);
                (0..walk)
                    .map(|step| {
                        let mut st = seed.wrapping_add(step as u64);
                        let ci = (next_u64(&mut st) % cands.len() as u64) as usize;
                        let units = &cands[ci];
                        let ui = (next_u64(&mut st) % units.len() as u64) as usize;
                        *sized.entry((ci, ui)).or_insert_with(|| {
                            let p = Puzzle { units: units.clone(), centre: units[ui].clone(), seed: 0 };
                            pool_in(lang, &p, tiers, true).len()
                        })
                    })
                    .max()
                    .unwrap_or(0)
            })
            .min()
            .unwrap_or(0);
        (cands.len(), floor)
    }

    /// CC-ONBOARD-JR I5 — a Spell Jr board draws its letters AND its answer
    /// pool from Easy + Medium only, through the real screen path (walk 512).
    /// Standard boards are unchanged by the refactor.
    #[test]
    fn junior_boards_hold_only_easy_and_medium_words() {
        use crate::experience::{Experience, TIERS};
        for lang in JR_LANGS {
            let seeds = (0..30u64).map(|d| d.wrapping_mul(0x9E3779B9));
            if !forge_ready_for(Experience::Junior, lang) {
                continue;
            }
            let jr: BTreeSet<String> = decomposed_in(lang, &TIERS[..2], true).iter().map(|(w, _, _)| w.clone()).collect();
            for seed in seeds {
                let (_, words, _) = generate_for(Experience::Junior, lang, seed, 512)
                    .unwrap_or_else(|| panic!("{lang}: no junior board for seed {seed}"));
                for w in &words {
                    assert!(jr.contains(w), "{lang}: junior pool holds {w:?}, which is not a kid-safe Easy or Medium word");
                }
                assert_eq!(generate_for(Experience::Standard, lang, seed, 64), generate(lang, seed, 64));
            }
        }
    }

    /// CC-ONBOARD-JR — every language offered a junior board gets one on every
    /// day of a year at its junior gate, and no language that COULD sustain
    /// one is silently refused it.
    #[test]
    fn every_junior_ready_language_gets_a_board_every_day() {
        use crate::experience::Experience;
        for lang in JR_LANGS {
            let (_, floor) = junior_floor(lang, 512);
            if forge_ready_for(Experience::Junior, lang) {
                assert!(floor >= junior_min_pool(lang),
                        "{lang}: junior gate {} but some day's best board holds only {floor}", junior_min_pool(lang));
            } else {
                assert!(floor < 8 || !forge_ready(lang),
                        "{lang}: Easy + Medium sustains a board of {floor} every day, yet Spell Jr is refused one");
            }
        }
    }

    /// The measurement behind junior_min_pool().
    #[test]
    #[ignore]
    fn calibrate_junior_min_pool() {
        for lang in JR_LANGS {
            let (cands, w512) = junior_floor(lang, 512);
            let (_, w64) = junior_floor(lang, 64);
            println!("JUNIOR_CEILING {lang:4} cands={cands:5} walk512={w512:3} walk64={w64:3} junior_gate={} standard_gate={}",
                     junior_min_pool(lang), min_pool(lang));
        }
    }

    /// The measurement behind min_pool(): for each language, the highest
    /// gate that still leaves ZERO ungenerable seeds in a 365-seed year.
    /// Re-run when banks grow — the table should rise with them.
    /// cargo test --lib calibrate_min_pool -- --ignored --nocapture
    #[test]
    #[ignore]
    fn calibrate_min_pool() {
        println!("{:5} {:>9} {:>10} {:>9}", "lang", "max gate", "first-try", "at shipped");
        for lang in ["en","es","fr","de","pt","pl","vi","ko","ja","fil","ru","sw","ar","hi","zh"] {
            let mut best = 0usize;
            for gate in (4..=30).rev() {
                let ok = (0..365u64).all(|d| {
                    generate_with(lang, d.wrapping_mul(0x9E3779B9), 64, gate).is_some()
                });
                if ok { best = gate; break; }
            }
            let first = (0..365u64)
                .filter(|d| matches!(
                    generate_with(lang, d.wrapping_mul(0x9E3779B9), 64, super::min_pool(lang)),
                    Some((_, _, 0))))
                .count();
            println!("{lang:5} {best:9} {:>10} {:9}", format!("{:.0}%", first as f64/3.65), super::min_pool(lang));
        }
    }

    #[test]
    #[ignore]
    fn why_ungenerable() {
        for lang in ["en", "hi", "zh", "pl", "ru", "ja"] {
            let d = decomposed(lang);
            let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
            for (_, _, u) in d.iter() { *hist.entry(u.len()).or_default() += 1; }
            let sevens = hist.get(&7).copied().unwrap_or(0);
            let max = hist.keys().max().copied().unwrap_or(0);
            println!("{lang:4} words={:6}  distinct-unit counts: max={max:3}  ==7: {sevens:5}  \
                      (1u:{:5} 2u:{:5} 3u:{:5})",
                d.len(), hist.get(&1).copied().unwrap_or(0),
                hist.get(&2).copied().unwrap_or(0), hist.get(&3).copied().unwrap_or(0));
        }
    }
}
