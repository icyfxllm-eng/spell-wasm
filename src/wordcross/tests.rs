//! CC-WORDGRID Phase B — acceptance tests 7, 8 and 9, and the invariants a
//! Spell Cross adds (I4, I5, I7). The checks read the finished grid; they do
//! not reuse the layout's own helpers.

use std::collections::HashSet;

use super::layout::MAX_SIDE;
use super::serve::{bank, from_list, lay, params, usable, Cross, MIN_WORDS};
use super::traps::{distinguishing, trap_positions};
use crate::spelldoku::rng::Rng;
use crate::wordsearch::gen::Tier;
use crate::wordsearch::ledger::Ledger;
use crate::wordsearch::lexicon::{graphemes, pool, tiers_for, LANGS};

/// Every letter of every word sits in its cell, crossings agree (I5), and the
/// grid is one connected component of five or more words (I7).
fn check(c: &Cross) {
    let g = &c.grid;
    assert!(g.words.len() >= MIN_WORDS, "{}: I7 wants five words, got {}", c.lang, g.words.len());
    assert!(g.w <= MAX_SIDE && g.h <= MAX_SIDE, "{}: {}x{} is bigger than a phone shows", c.lang, g.w, g.h);
    for p in &g.words {
        let want = graphemes(&p.word);
        assert_eq!(p.cells.len(), want.len(), "{}: {} has the wrong cell count", c.lang, p.word);
        for (k, cell) in p.cells.iter().enumerate() {
            // I5 / F-C4: the cell holds this word's grapheme -- and since both
            // words read the same cell, an accented letter never crosses its base.
            assert_eq!(g.cells[*cell].as_deref(), Some(want[k].as_str()), "{}: {} at cell {}", c.lang, p.word, cell);
        }
        // Cells run in a straight line, one step apart.
        let step = if p.across { 1 } else { g.w };
        for pair in p.cells.windows(2) {
            assert_eq!(pair[1], pair[0] + step, "{}: {} is not straight", c.lang, p.word);
        }
    }
    // I7: connected through shared cells.
    let mut seen: HashSet<usize> = HashSet::from([0]);
    loop {
        let before = seen.len();
        for (i, a) in g.words.iter().enumerate() {
            if seen.contains(&i) {
                continue;
            }
            if g.words.iter().enumerate().any(|(j, b)| seen.contains(&j) && a.cells.iter().any(|x| b.cells.contains(x))) {
                seen.insert(i);
            }
        }
        if seen.len() == before {
            break;
        }
    }
    assert_eq!(seen.len(), g.words.len(), "{}: I7 wants one connected component", c.lang);
    // Numbers: words that start in the same cell share one.
    for a in &g.words {
        for b in &g.words {
            if a.cells[0] == b.cells[0] {
                assert_eq!(a.number, b.number);
            } else {
                assert_ne!((a.number, a.cells[0]), (b.number, b.cells[0]));
            }
        }
    }
    // F-C6: the keystone is a word of its own, spelled by the shaded cells in
    // reading order, and it is not one of the words in the grid (F-X3).
    if let Some(k) = &g.keystone {
        let want = graphemes(k);
        assert_eq!(g.shaded.len(), want.len(), "{}: keystone cells", c.lang);
        assert!(g.shaded.windows(2).all(|w| w[0] < w[1]), "{}: shaded cells read in order", c.lang);
        for (i, cell) in g.shaded.iter().enumerate() {
            assert_eq!(g.cells[*cell].as_deref(), Some(want[i].as_str()));
        }
        assert!(!g.words.iter().any(|p| p.word == *k), "{}: the keystone is not already in the grid", c.lang);
    }
    // I4 / F-C3: a word that sounds like another has a crossing that tells them
    // apart (no language serves sense cues yet, so that is the only way in).
    let crossings = g.crossings();
    for (i, p) in g.words.iter().enumerate() {
        let Some(pos) = distinguishing(&c.lang, &p.word) else { continue };
        let ok = crossings
            .iter()
            .filter(|(_, a, b)| *a == i || *b == i)
            .any(|&(cell, _, _)| g.index_in(i, cell).is_some_and(|k| pos.contains(&k)));
        assert!(ok, "{}: I4 -- {} sounds like another spelling and no crossing tells them apart", c.lang, p.word);
    }
}

/// Test 7: random eight-word draws from the bank. At least 80% interlock five
/// or more words; the rest return None, so the screen offers Spell Search
/// (D2, F-C7). A disconnected grid is never returned at all (checked above).
#[test]
fn wordcross_yield() {
    for lang in LANGS {
        for tier in tiers_for(lang, false) {
            let words = pool(lang, tier);
            let mut ok = 0;
            let tries = 25;
            for s in 0..tries {
                let mut rng = Rng::new(s * 7919 + 11);
                let mut draw: Vec<String> = words.clone();
                rng.shuffle(&mut draw);
                let spare = draw[8..12].to_vec();
                if let Some(c) = lay(lang, tier, &draw[..8], &spare, &mut rng) {
                    check(&c);
                    ok += 1;
                }
            }
            assert!(ok * 100 >= tries * 80, "{lang} {}: {ok} of {tries} lists made a crossword", tier.id());
        }
    }
}

/// Test 8: where a language has a confusion list, Easy puts crossings on trap
/// positions and Hard keeps them off. (A language without one has no trap
/// positions, so the test does not apply to it -- see the census rulings.)
#[test]
fn wordcross_trap_crossings() {
    let share = |tier: Tier| -> f64 {
        let (mut on, mut all) = (0.0, 0.0);
        for s in 0..40u64 {
            // The puzzles the mode serves, which choose their own words (F-X1).
            let ledger = Ledger { counter: s, ..Default::default() };
            let Some(c) = bank("en", tier, &ledger, 0) else { continue };
            for (cell, a, b) in c.grid.crossings() {
                all += 1.0;
                let hit = [a, b].iter().any(|&w| {
                    c.grid.index_in(w, cell).is_some_and(|k| trap_positions("en", &c.grid.words[w].word).contains(&k))
                });
                if hit {
                    on += 1.0;
                }
            }
        }
        assert!(all > 0.0);
        on / all
    };
    let easy = share(Tier::Easy);
    let hard = share(Tier::Hard);
    assert!(easy >= 0.60, "Easy puts {:.0}% of crossings on trap positions, wants 60%", easy * 100.0);
    assert!(hard <= 0.10, "Hard leaves {:.0}% on trap positions, wants 10% or less", hard * 100.0);
}

/// Test 9: a grid holding a known collision pair answers it or leaves it out.
#[test]
fn wordcross_homophones() {
    // English "their / there" and the pairs the table lists: a grid may hold
    // one of a pair only with a crossing that tells them apart.
    let pairs = [("their", "there"), ("pair", "pear"), ("for", "four")];
    for (a, b) in pairs {
        assert!(!crate::homophones::group_members("en", a).is_empty(), "{a} is in a collision group");
    }
    let mut checked = 0;
    for s in 0..60u64 {
        let mut rng = Rng::new(s + 500);
        let list: Vec<String> = ["their", "there", "pair", "pear", "four", "for", "write", "right", "plant", "storm", "cloud", "grain"]
            .iter()
            .map(|w| w.to_string())
            .collect();
        let usable = usable("en", &list);
        let Some(c) = lay("en", Tier::Easy, &usable[..8.min(usable.len())], &[], &mut rng) else { continue };
        check(&c); // the I4 assertion lives there
        // Both spellings may appear only when each has a crossing that tells
        // them apart, which `check` has just asserted (I4). What is never
        // allowed is a cell that would take either spelling, and a cell holds
        // one grapheme by construction.
        checked += 1;
    }
    assert!(checked > 0, "at least one grid held a collision word");
}

/// F-X1 / F-X3: a bank Spell Cross, and the same seed twice.
#[test]
fn bank_and_lists_serve_and_repeat_exactly() {
    for lang in ["en", "es", "ru", "fil"] {
        let ledger = Ledger::default();
        let c = bank(lang, Tier::Easy, &ledger, 0).unwrap_or_else(|| panic!("{lang}: a bank Spell Cross"));
        check(&c);
        let again = bank(lang, Tier::Easy, &ledger, 0).unwrap();
        assert_eq!(c.grid, again.grid, "{lang}: the same counter gives the same grid");
    }
    let list: Vec<String> = ["planet", "garden", "yellow", "rabbit", "window", "orange", "basket", "candle", "button", "pocket"]
        .iter()
        .map(|w| w.to_string())
        .collect();
    let a = from_list("en", Tier::Easy, "l1", &list, 0).expect("a list crossword");
    check(&a);
    let b = from_list("en", Tier::Easy, "l1", &list, 1).expect("a second one");
    assert_ne!(a.grid, b.grid, "the same list twice lays out differently");
    for p in &a.grid.words {
        assert!(list.contains(&p.word), "only the list's own words (D1): {}", p.word);
    }
}

/// D2 / F-C7: three words never make a crossword, and nothing partial is returned.
#[test]
fn too_few_words_falls_back() {
    let short: Vec<String> = ["planet", "garden", "yellow"].iter().map(|w| w.to_string()).collect();
    assert!(from_list("en", Tier::Easy, "l2", &short, 0).is_none());
    assert!(from_list("xx", Tier::Easy, "l3", &short, 0).is_none(), "an unplayable language serves nothing");
}

/// F-C2 / F-C5 by tier.
#[test]
fn tier_table() {
    assert!(params(Tier::Easy).on_traps && params(Tier::Medium).on_traps && params(Tier::Jr).on_traps);
    assert!(!params(Tier::Hard).on_traps && !params(Tier::Expert).on_traps);
    assert!(!params(Tier::Hard).letter_check && !params(Tier::Expert).letter_check, "F-C5: no letter-by-letter at Hard or Expert");
    assert!(params(Tier::Easy).letter_check);
}

/// Phase C: the Daily Spell Cross -- one crossword per language per date, with
/// D-W1 and D-W4: the Daily crossword is a personal draw now -- fresh for each
/// player and each play, its words rotating by the player's ledger -- and every
/// one of them interlocks at least MIN_WORDS words (F5 / I-W5).
#[test]
fn the_daily_crossword_is_a_personal_draw() {
    use crate::wordsearch::ledger::Ledger;
    for lang in ["en", "es", "ru", "de"] {
        let mut led = Ledger::default();
        let mut grids = Vec::new();
        let mut previous: Vec<String> = Vec::new();
        for play in 0..6u32 {
            let c = bank(lang, Tier::Easy, &led, play)
                .unwrap_or_else(|| panic!("{lang}: play {play} has a crossword"));
            check(&c);
            assert!(c.grid.words.len() >= MIN_WORDS, "{lang}: never fewer than {MIN_WORDS} interlocks");
            let words: Vec<String> = c.grid.words.iter().map(|p| p.word.clone()).collect();
            if play > 0 {
                let cap = (words.len() / 4).max(2);
                let shared = words.iter().filter(|w| previous.contains(w)).count();
                assert!(shared <= cap, "{lang}: play {play} shares {shared} words, cap {cap}");
            }
            led.record(&format!("{lang}:{}", Tier::Easy.id()), &words, play, 0);
            grids.push(c.grid.clone());
            previous = words;
        }
        for i in 1..grids.len() {
            assert_ne!(grids[i - 1], grids[i], "{lang}: consecutive plays differ");
        }
    }
}
