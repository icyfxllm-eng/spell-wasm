//! CC-SPELLDOKU v1.2 Phase A2 — Word Mode Done items 14–18 and 22, plus F10,
//! F11 and the verdicts, host side. The checks below are written apart from
//! `wordmode.rs`: they re-read the bank and the keyboard themselves.

use std::collections::HashSet;

use unicode_segmentation::UnicodeSegmentation;

use super::bind::{Board, Mode, WordSymbol};
use super::gen::{Clue, Config, Tier};
use super::geo::{Geo, S9};
use super::play::{verdict, Verdict};
use super::solve::{count_solutions, grade};
use super::table::{self, norm};
use super::wordmode::{band_mix, board, board_or_number, choose, is_word_mode, typeable, Personal, MAX_GRAPHEMES};

const LANGS: &[&str] = &["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ko", "ja", "zh", "fil", "sw", "ar", "hi"];
const TIERS: [Tier; 3] = [Tier::Medium, Tier::Hard, Tier::Expert];

fn cfg(tier: Tier) -> Config {
    Config { size: S9, tier }
}

/// The bank row a word came from, re-read here: (band, display).
fn bank_row(lang: &str, spelling: &str) -> Vec<(&'static str, Option<&'static str>)> {
    let mut out = Vec::new();
    for band in ["easy", "medium", "hard", "expert"] {
        for e in crate::words::tier_for(lang, band) {
            let mut p = e.split('|');
            if norm(p.next().unwrap_or("")) == spelling {
                out.push((band, p.next()));
            }
        }
    }
    out
}

/// F13's four gates, the band mix (F12) and I11, checked independently.
fn check_set(lang: &str, tier: Tier, words: &[WordSymbol], seed: u64) -> Result<(), String> {
    let mix = band_mix(9, tier).unwrap();
    if words.len() != 9 {
        return Err(format!("{} words", words.len()));
    }
    for &(band, count) in mix {
        let got = words.iter().filter(|w| w.band == band).count();
        if got != count {
            return Err(format!("F12: {got} {band} words, the mix wants {count}"));
        }
    }
    let mut keys = HashSet::new();
    for w in words {
        let rows = bank_row(lang, &w.spelling);
        if !rows.iter().any(|(b, _)| *b == w.band) {
            return Err(format!("gate 2: {} is not a {} bank row", w.spelling, w.band));
        }
        let len = w.spelling.graphemes(true).count();
        if len > MAX_GRAPHEMES || len < 2 {
            return Err(format!("gate 3: {} is {len} graphemes", w.spelling));
        }
        if !typeable(lang, &w.spelling) || crate::profanity::is_blocked(&w.spelling) {
            return Err(format!("{} cannot be typed or is blocked", w.spelling));
        }
        let own = rows.iter().find(|(b, _)| *b == w.band).copied().unwrap_or(rows[0]);
        let first = match lang {
            "zh" => own.1.and_then(|d| d.graphemes(true).next()).unwrap_or("").to_string(),
            _ => w.spelling.graphemes(true).next().unwrap_or("").to_string(),
        };
        if w.glyph.to_lowercase() != first.to_lowercase() {
            return Err(format!("R3: {} shows {} but starts with {first}", w.spelling, w.glyph));
        }
        if (lang == "ru" && ["ы", "ь", "ъ"].contains(&first.as_str())) || (lang == "ja" && "ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮー".contains(&first)) {
            return Err(format!("R3: {} cannot lead a symbol", w.spelling));
        }
        if !keys.insert(first.to_lowercase()) {
            return Err(format!("gate 1 / I11: two words start with {first}"));
        }
    }
    for (i, a) in words.iter().enumerate() {
        for b in &words[i + 1..] {
            if a.spelling == b.spelling {
                return Err(format!("{} twice", a.spelling));
            }
            if crate::homophones::accepts(lang, &a.spelling, &b.spelling) {
                return Err(format!("{} and {} sound alike", a.spelling, b.spelling));
            }
            if lang == "en" {
                let e = crate::wordsearch::confusion::edits("en", &a.spelling);
                let f = crate::wordsearch::confusion::edits("en", &b.spelling);
                if e.contains(&b.spelling) || f.contains(&a.spelling) {
                    return Err(format!("gate 4: {} and {} are one confusion edit apart", a.spelling, b.spelling));
                }
            }
        }
    }
    let _ = seed;
    Ok(())
}

fn sets(per: u64) {
    for lang in LANGS {
        for tier in TIERS {
            for seed in 0..per {
                let Some(words) = choose(lang, 9, tier, seed, &Personal::default()) else {
                    panic!("{lang} {tier:?} seed {seed}: no word set (R5 should always find one)");
                };
                if let Err(e) = check_set(lang, tier, &words, seed) {
                    panic!("{lang} {tier:?} seed {seed}: {e}");
                }
            }
        }
    }
}

/// Done #15 and #22 at gate size; the 10,000-set sweep is ignored below.
#[test]
fn word_sets_pass_every_gate_and_match_the_mix() {
    sets(40);
}

/// Done #15 / #22 in full: 10,000 sets per (language, 9×9, tier).
/// cargo test --release --lib --features audit_preview word_sets_ten_thousand -- --ignored
#[test]
#[ignore]
fn word_sets_ten_thousand() {
    sets(10_000);
}

/// I1, I2, F2 (against this board's own words) and Done #5 for a Word Mode board.
fn check_board(b: &Board) {
    let p = &b.puzzle;
    let geo = Geo::new(S9);
    let symbols = b.symbols();
    let (fixed, restrict) = p.constraints(&symbols);
    assert_eq!(count_solutions(&geo, &fixed, &restrict, 2), 1, "{}: I1 not unique", b.lang);
    assert_eq!(grade(&geo, &fixed, &restrict), Some(p.tier.tech()), "{}: I2 graded wrong", b.lang);
    let Mode::Words(words) = &b.mode else { panic!("not Word Mode") };
    for (i, c) in p.clues.iter().enumerate() {
        if let Clue::Fragment(ids) = c {
            let pat = b.fragment_text(ids);
            let brute: u16 = words
                .iter()
                .enumerate()
                .filter(|(_, w)| {
                    let l: Vec<&str> = w.spelling.graphemes(true).collect();
                    l.len() == pat.len() && l.iter().zip(&pat).all(|(c, q)| q.as_deref().map_or(true, |q| q == *c))
                })
                .fold(0u16, |m, (v, _)| m | (1 << (v + 1)));
            assert_eq!(restrict[i], brute, "{}: F10 fragment set differs from brute force over the board's words", b.lang);
        }
    }
    if matches!(p.tier, Tier::Hard | Tier::Expert) {
        let necessary = (0..p.clues.len()).filter(|&i| matches!(p.clues[i], Clue::Fragment(_))).any(|i| {
            let mut open = restrict.clone();
            open[i] = geo.full();
            count_solutions(&geo, &fixed, &open, 2) > 1
        });
        assert!(necessary, "{}: Hard/Expert need a necessary fragment", b.lang);
    }
}

/// Done #16 / I12: no surface spells a symbol's word.
fn check_no_free_spelling(b: &Board) {
    let Mode::Words(words) = &b.mode else { return };
    let shown: Vec<String> = b.surfaces().iter().map(|s| norm(s)).collect();
    for w in words {
        for s in &shown {
            assert!(!s.contains(&norm(&w.spelling)), "{}: \"{s}\" spells {}", b.lang, w.spelling);
        }
    }
    for (i, c) in b.puzzle.clues.iter().enumerate() {
        if matches!(c, Clue::Spelled(_)) {
            assert_eq!(b.clue_text(i).graphemes(true).count(), 1, "a Word Mode given shows only its glyph");
        }
    }
}

/// Done #17 (cold start: no saved words), F10, I12, for every language and tier.
#[test]
fn every_language_plays_word_mode_from_a_cold_start() {
    for lang in LANGS {
        for tier in TIERS {
            for seed in 0..2u64 {
                let b = board(lang, &cfg(tier), 1000 + seed, &Personal::default())
                    .unwrap_or_else(|| panic!("{lang} {tier:?}: no Word Mode board"));
                check_board(&b);
                check_no_free_spelling(&b);
                let again = board(lang, &cfg(tier), 1000 + seed, &Personal::default()).unwrap();
                assert_eq!(b.puzzle, again.puzzle, "{lang}: I5 deterministic");
                assert_eq!(b.surfaces(), again.surfaces());
            }
        }
    }
}

/// F11: My Words first, then missed words, then the bank. A player with enough
/// saved words in the right bands gets at least ⌈2N/3⌉ of them.
#[test]
fn the_ladder_prefers_the_players_own_words() {
    // Spread through the bank, so the words start with different letters.
    let bank = |band: &str, k: usize| -> Vec<String> {
        crate::words::tier_for("en", band).iter().step_by(31).take(k).map(|w| w.to_string()).collect()
    };
    let mine: Vec<String> = bank("medium", 12).into_iter().chain(bank("easy", 20)).collect();
    let p = Personal { mine: mine.clone(), missed: Vec::new() };
    for seed in 0..30 {
        let w = choose("en", 9, Tier::Medium, seed, &p).unwrap();
        let from_mine = w.iter().filter(|s| s.mine).count();
        assert!(from_mine >= 6, "seed {seed}: {from_mine} of 9 from My Words, want at least 6");
        assert!(w.iter().filter(|s| s.mine).all(|s| mine.iter().any(|m| norm(m) == s.spelling)));
        check_set("en", Tier::Medium, &w, seed).unwrap();
    }
    // Missed words come before the bank.
    let missed: Vec<(String, String)> = bank("hard", 10).into_iter().map(|w| (w, "hard".to_string())).collect();
    let p = Personal { mine: Vec::new(), missed: missed.clone() };
    let w = choose("en", 9, Tier::Hard, 5, &p).unwrap();
    let from_missed = w.iter().filter(|s| s.band == "hard" && missed.iter().any(|(m, _)| norm(m) == s.spelling)).count();
    assert!(from_missed >= 4, "missed words lead the hard band ({from_missed} of 5)");
    // D14: the badge counts the player's words.
    let b = board("en", &cfg(Tier::Medium), 3, &Personal { mine, missed: Vec::new() }).unwrap();
    assert!(b.mine() >= 6);
}

/// Done #18 / F13 fallback: a set that cannot be drawn serves Number Mode,
/// never a partial board; Spell Jr and Easy never see Word Mode (D12, D16).
#[test]
fn a_set_that_cannot_be_drawn_falls_back_to_number_mode() {
    let en = table::load("en").unwrap();
    // An empty bank cannot draw a set.
    super::wordmode::EMPTY_BANK.with(|e| e.set(true));
    let none = choose("en", 9, Tier::Medium, 1, &Personal::default()).is_none();
    let fallback = board_or_number("en", false, &cfg(Tier::Medium), 1, &Personal::default(), Some(&en));
    let nothing = board_or_number("en", false, &cfg(Tier::Medium), 1, &Personal::default(), None);
    super::wordmode::EMPTY_BANK.with(|e| e.set(false));
    assert!(none, "no set from an empty bank");
    assert!(!fallback.expect("Number Mode").is_words(), "the fallback is a whole Number Mode board");
    // No number table either: nothing is served, rather than something partial.
    assert!(nothing.is_none());
    // D12 / D16.
    assert!(!is_word_mode(false, 9, Tier::Easy) && !is_word_mode(true, 9, Tier::Medium) && !is_word_mode(false, 6, Tier::Medium));
    assert!(board_or_number("en", false, &cfg(Tier::Medium), 1, &Personal::default(), Some(&en)).unwrap().is_words());
    assert!(!board_or_number("en", true, &cfg(Tier::Medium), 1, &Personal::default(), Some(&en)).unwrap().is_words());
}

/// I4 in Word Mode: the board's own words are the vocabulary.
#[test]
fn word_mode_verdicts() {
    for lang in ["en", "zh", "ko", "vi", "ar"] {
        let b = board(lang, &cfg(Tier::Medium), 77, &Personal::default()).unwrap();
        let Mode::Words(w) = &b.mode else { panic!() };
        let (one, two) = (&w[0].spelling, &w[1].spelling);
        assert_eq!(verdict(&b.values_for(one), 1), Verdict::Correct, "{lang}: {one}");
        assert_eq!(verdict(&b.values_for(two), 1), Verdict::WrongValue, "{lang}: another board word");
        assert_eq!(verdict(&b.values_for("qqqqzz"), 1), Verdict::Misspelled);
    }
}

/// Done #14 / I14: the generator, solver, grader and canonical hasher never
/// name a word, a number word, a table or a language.
#[test]
fn the_core_is_symbol_agnostic() {
    for (name, src) in [
        ("gen.rs", include_str!("gen.rs")),
        ("solve.rs", include_str!("solve.rs")),
        ("canon.rs", include_str!("canon.rs")),
        ("symbols.rs", include_str!("symbols.rs")),
        ("geo.rs", include_str!("geo.rs")),
    ] {
        let code: String = src
            .lines()
            .map(|l| l.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // Identifier parts, split on anything that is not a letter or digit.
        let parts: HashSet<&str> = code.split(|c: char| !c.is_ascii_alphanumeric()).filter(|p| !p.is_empty()).collect();
        for banned in ["lang", "language", "word", "words", "table", "grapheme", "graphemes", "spelling", "spellings", "glyphs", "bind", "wordmode", "wordsearch", "unicode"] {
            assert!(!parts.contains(banned), "I14: {name} mentions {banned}");
        }
    }
}
