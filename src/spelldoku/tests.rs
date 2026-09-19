//! CC-SPELLDOKU Done criteria, host side.

use super::canon;
use super::geo::{size_of, Geo, S4, S6, S9};
use super::gen::{generate, Clue, Config, Puzzle, Tier};
use super::play::{self, judge, Unlocks, Verdict};
use super::rng::Rng;
use super::solve::{count_solutions, grade, next_step};
use super::table::{self, Build, Table};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Every language SpellDoku serves (Eric, 2026-09-18: all of them, where the mode fits).
const LANGS: &[&str] = &["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ko", "ja", "zh", "fil", "sw", "ar", "hi"];

fn en() -> Table {
    table::load("en").expect("the English table loads")
}

const CONFIGS: &[(usize, Tier)] = &[
    (4, Tier::Easy),
    (6, Tier::Easy),
    (6, Tier::Medium),
    (9, Tier::Easy),
    (9, Tier::Medium),
    (9, Tier::Hard),
    (9, Tier::Expert),
];

/// Done #2.
#[test]
fn a_four_by_four_has_exactly_288_solution_grids() {
    let geo = Geo::new(S4);
    let empty = vec![0u8; 16];
    let open = vec![geo.full(); 16];
    assert_eq!(count_solutions(&geo, &empty, &open, 1000), 288);
}

/// I1, I2, F2 and Done #5 for one board.
fn check_board(p: &Puzzle, t: &Table) {
    let geo = Geo::new(size_of(p.n).unwrap());
    let (fixed, restrict) = p.constraints(t);
    assert_eq!(count_solutions(&geo, &fixed, &restrict, 2), 1, "I1: seed {} is not unique", p.seed);
    assert_eq!(grade(&geo, &fixed, &restrict), Some(p.tier.tech()), "I2: seed {} graded wrong", p.seed);
    // The solution satisfies every clue.
    for (i, c) in p.clues.iter().enumerate() {
        match c {
            Clue::Given(v) | Clue::Word(v) => assert_eq!(*v, p.solution[i]),
            Clue::Fragment(pat) => {
                // F2: the solver's candidate set equals brute force against the table.
                let brute: u16 = (1..=p.n as u32)
                    .filter(|&v| {
                        t.row(v).is_some_and(|r| {
                            r.spellings.iter().any(|s| {
                                let l: Vec<&str> = s.graphemes(true).collect();
                                l.len() == pat.len() && l.iter().zip(pat).all(|(c, q)| q.as_deref().map_or(true, |q| q == *c))
                            })
                        })
                    })
                    .fold(0u16, |m, v| m | (1 << v));
                assert_eq!(restrict[i], brute, "F2: seed {} fragment set differs from brute force", p.seed);
                assert!(restrict[i] & (1 << p.solution[i]) != 0, "the fragment admits the answer");
            }
            Clue::Empty => {}
        }
    }
    // Done #5: Hard and Expert carry a NECESSARY fragment.
    if matches!(p.tier, Tier::Hard | Tier::Expert) {
        let frags: Vec<usize> = (0..p.clues.len()).filter(|&i| matches!(p.clues[i], Clue::Fragment(_))).collect();
        assert!(!frags.is_empty(), "seed {}: Hard/Expert need a fragment", p.seed);
        let necessary = frags.iter().any(|&i| {
            let mut open = restrict.clone();
            open[i] = geo.full();
            count_solutions(&geo, &fixed, &open, 2) > 1
        });
        assert!(necessary, "seed {}: no fragment is necessary", p.seed);
    }
}

fn sweep(per_config: u64) {
    let t = en();
    for &(n, tier) in CONFIGS {
        let cfg = Config { size: size_of(n).unwrap(), tier };
        for seed in 0..per_config {
            let p = generate(seed, &cfg, &t).unwrap_or_else(|| panic!("no board for {n}x{n} {tier:?} seed {seed}"));
            check_board(&p, &t);
        }
    }
}

#[test]
fn every_configuration_generates_unique_boards_graded_to_their_tier() {
    sweep(3);
}

/// Done #1 in full: 10,000 seeds per configuration. Run with --ignored --release.
#[test]
#[ignore]
fn done_1_ten_thousand_seeds_per_configuration() {
    sweep(10_000);
}

/// Done #3 / I5: same (seed, config), same board.
#[test]
fn generation_is_deterministic() {
    let t = en();
    for &(n, tier) in CONFIGS {
        let cfg = Config { size: size_of(n).unwrap(), tier };
        let a = generate(42, &cfg, &t).unwrap();
        let b = generate(42, &cfg, &t).unwrap();
        assert_eq!(a, b);
    }
}

/// A random symmetry of the board: relabel, rows within bands, bands, columns
/// within stacks, stacks, and transposition where the boxes are square.
fn scramble(p: &Puzzle, seed: u64) -> Puzzle {
    let size = size_of(p.n).unwrap();
    let n = p.n;
    let mut rng = Rng::new(seed);
    let mut relabel: Vec<u8> = (1..=n as u8).collect();
    rng.shuffle(&mut relabel);
    let map = |v: u8| relabel[v as usize - 1];
    let perm = |rng: &mut Rng, group: usize| -> Vec<usize> {
        let mut bands: Vec<usize> = (0..n / group).collect();
        rng.shuffle(&mut bands);
        let mut out = Vec::new();
        for b in bands {
            let mut inner: Vec<usize> = (0..group).collect();
            rng.shuffle(&mut inner);
            out.extend(inner.into_iter().map(|k| b * group + k));
        }
        out
    };
    let rows = perm(&mut rng, size.br);
    let cols = perm(&mut rng, size.bc);
    let transpose = size.br == size.bc && rng.below(2) == 0;
    let mut clues = vec![Clue::Empty; n * n];
    let mut solution = vec![0u8; n * n];
    for r in 0..n {
        for c in 0..n {
            let src = rows[r] * n + cols[c];
            let dst = if transpose { c * n + r } else { r * n + c };
            solution[dst] = map(p.solution[src]);
            clues[dst] = match &p.clues[src] {
                Clue::Given(v) => Clue::Given(map(*v)),
                Clue::Word(v) => Clue::Word(map(*v)),
                other => other.clone(),
            };
        }
    }
    Puzzle { clues, solution, ..p.clone() }
}

/// Done #4 / I6: relabeled and permuted copies hash equal, and a year of
/// served boards holds no repeat.
#[test]
fn isomorphic_copies_share_a_hash_and_a_year_never_repeats() {
    let t = en();
    for &(n, tier) in &[(4usize, Tier::Easy), (6, Tier::Medium), (9, Tier::Medium)] {
        let cfg = Config { size: size_of(n).unwrap(), tier };
        let p = generate(7, &cfg, &t).unwrap();
        for s in 0..20 {
            assert_eq!(canon::hash(&p), canon::hash(&scramble(&p, s)), "{n}x{n}: a scrambled copy must hash equal");
        }
    }
    // A simulated year of Dailies with the served-hash ledger.
    let cfg = Config { size: S6, tier: Tier::Medium };
    let mut seen = std::collections::HashSet::new();
    for day in 0..play::REPEAT_WINDOW_DAYS {
        let mut k = 0u64;
        loop {
            let p = generate(play::daily_seed(20260101 + day, "en").wrapping_add(k), &cfg, &t).unwrap();
            if seen.insert(canon::hash(&p)) {
                break;
            }
            k += 1;
        }
    }
    assert_eq!(seen.len(), play::REPEAT_WINDOW_DAYS as usize);
}

/// Done #6 / I4.
#[test]
fn three_distinct_verdicts() {
    let t = en();
    assert_eq!(judge(&t, "seven", 7), Verdict::Correct);
    assert_eq!(judge(&t, "  Seven ", 7), Verdict::Correct, "case and spaces fold (I7)");
    assert_eq!(judge(&t, "sevn", 7), Verdict::Misspelled, "not a number word");
    assert_eq!(judge(&t, "eight", 7), Verdict::WrongValue, "a real number word, wrong cell");
}

/// Done #8 / D1.
#[test]
fn chips_unlock_at_the_signed_thresholds_and_never_on_expert() {
    for (tier, need) in [(Tier::Easy, 1), (Tier::Medium, 2), (Tier::Hard, 3)] {
        let mut u = Unlocks::new(tier);
        for k in 0..need {
            assert!(!u.chip(5), "{tier:?}: no chip after {k}");
            u.record_correct(5);
        }
        assert!(u.chip(5), "{tier:?}: chip after {need}");
    }
    let mut u = Unlocks::new(Tier::Expert);
    for _ in 0..50 {
        u.record_correct(5);
    }
    assert!(!u.chip(5), "Expert never shows a chip");
    // Easy 9x9: one spelling per value, so at most nine full spellings.
    assert_eq!(play::unlock_after(Tier::Easy), Some(1));
}

/// Done #12 / F7.
#[test]
fn spell_jr_only_ever_gets_its_own_column() {
    let mut rng = Rng::new(99);
    let tiers = [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert];
    for _ in 0..10_000 {
        let n = [4usize, 6, 9, 12][rng.below(4)];
        let tier = tiers[rng.below(4)];
        if play::allowed(true, n, tier) {
            assert!(matches!((n, tier), (4, Tier::Easy) | (6, Tier::Medium)), "Jr got {n}x{n} {tier:?}");
        }
    }
}

/// Done #13 / D3 as Eric set it on 2026-09-18: every language plays its sourced
/// 0-12 rows, in every build; 13-45 waits for the audit in production.
#[test]
fn table_gating_follows_the_release_rule() {
    for lang in LANGS {
        let t = table::load(lang).unwrap_or_else(|| panic!("{lang} has a table"));
        assert!(t.rows.iter().all(|r| !r.source.is_empty() && !r.locator.is_empty()), "{lang}: every row cites its source");
        assert!(t.servable_for(9, Build::Production), "{lang}: sourced 1-9 plays in production");
        assert!(t.servable_for(9, Build::Preview));
    }
    let sourced_21 = table::Row { n: 21, spellings: vec!["x".into()], status: table::Status::Sourced, source: "s".into(), locator: "l".into() };
    assert!(!table::row_servable("es", &sourced_21, Build::Production), "13-45 needs the audit in production");
    assert!(table::row_servable("es", &sourced_21, Build::Preview), "and plays in a preview build");
    assert!(table::load("xx").is_none(), "an unknown language has no table");
}

fn keyboard(lang: &str) -> std::collections::HashSet<String> {
    let path = format!("{}/assets/keyboards/{lang}.json", env!("CARGO_MANIFEST_DIR"));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut keys = std::collections::HashSet::new();
    for r in v["rows"].as_array().unwrap() {
        for c in r.as_str().unwrap().chars() {
            keys.insert(c.to_string());
        }
    }
    if let Some(lp) = v["longPress"].as_object() {
        for a in lp.values() {
            for c in a.as_str().unwrap().chars() {
                keys.insert(c.to_string());
            }
        }
    }
    keys
}

/// Korean keystrokes for a syllable string, as the jamo keyboard types them.
fn jamo_keys(word: &str) -> Vec<char> {
    const MED: [(char, char, char); 7] = [('ㅗ', 'ㅏ', 'ㅘ'), ('ㅗ', 'ㅐ', 'ㅙ'), ('ㅗ', 'ㅣ', 'ㅚ'), ('ㅜ', 'ㅓ', 'ㅝ'), ('ㅜ', 'ㅔ', 'ㅞ'), ('ㅜ', 'ㅣ', 'ㅟ'), ('ㅡ', 'ㅣ', 'ㅢ')];
    const FIN: [(char, char, char); 11] = [('ㄱ', 'ㅅ', 'ㄳ'), ('ㄴ', 'ㅈ', 'ㄵ'), ('ㄴ', 'ㅎ', 'ㄶ'), ('ㄹ', 'ㄱ', 'ㄺ'), ('ㄹ', 'ㅁ', 'ㄻ'), ('ㄹ', 'ㅂ', 'ㄼ'), ('ㄹ', 'ㅅ', 'ㄽ'), ('ㄹ', 'ㅌ', 'ㄾ'), ('ㄹ', 'ㅍ', 'ㄿ'), ('ㄹ', 'ㅎ', 'ㅀ'), ('ㅂ', 'ㅅ', 'ㅄ')];
    let split = |c: char, t: &[(char, char, char)]| t.iter().find(|x| x.2 == c).map(|x| vec![x.0, x.1]).unwrap_or(vec![c]);
    let mut out = Vec::new();
    for c in word.chars() {
        let (i, m, f) = crate::hangul::parts(c).expect("a Hangul syllable");
        out.push(i);
        out.extend(split(m, &MED));
        if f != '\0' {
            out.extend(split(f, &FIN));
        }
    }
    out
}

/// "The game mode fits the language": for 1-9, at least one accepted spelling
/// can be typed on that language's own keyboard, through the same input the
/// screen uses -- tone numbers for Mandarin, the tone row for Vietnamese, jamo
/// composition for Korean.
#[test]
fn every_language_can_type_one_to_nine_on_its_own_keyboard() {
    for lang in LANGS {
        let t = table::load(lang).unwrap();
        let keys = keyboard(lang);
        for n in 1..=9u32 {
            let row = t.row(n).unwrap_or_else(|| panic!("{lang}: {n} is sourced"));
            let typeable = row.spellings.iter().any(|s| match *lang {
                "zh" => {
                    let letters: String = s.nfd().filter(|c| !('\u{0300}'..='\u{036F}').contains(c) || *c == '\u{0308}').collect::<String>().nfc().collect();
                    letters.chars().all(|c| keys.contains(&c.to_string()))
                        && (1..=5).any(|d| table::spelling_matches("zh", &format!("{letters}{d}"), s))
                }
                "vi" => {
                    let tones = ['\u{0300}', '\u{0301}', '\u{0309}', '\u{0303}', '\u{0323}'];
                    let base: String = s.nfd().filter(|c| !tones.contains(c)).collect::<String>().nfc().collect();
                    base.chars().all(|c| keys.contains(&c.to_string()))
                }
                "ko" => {
                    let strokes = jamo_keys(s);
                    strokes.iter().all(|k| keys.contains(&k.to_string()))
                        && strokes.iter().fold(String::new(), |acc, &k| crate::hangul::feed(&acc, k)) == *s
                }
                _ => s.chars().all(|c| keys.contains(&c.to_string())),
            });
            assert!(typeable, "{lang}: no spelling of {n} ({:?}) can be typed on its keyboard", row.spellings);
        }
    }
}

/// Every language plays: where its words can make a fragment, a Hard 9x9 is
/// unique, graded Hard, and its fragment -- in that language's own script --
/// matches brute force. Where they cannot (Hindi), Medium is the ceiling.
#[test]
fn every_language_generates_a_graded_board_with_its_own_fragments() {
    for lang in LANGS {
        let t = table::load(lang).unwrap();
        let frag = super::gen::fragments_possible(&t, 9);
        let tier = if frag { Tier::Hard } else { Tier::Medium };
        let p = generate(11, &Config { size: S9, tier }, &t).unwrap_or_else(|| panic!("{lang}: no {tier:?} board"));
        check_board(&p, &t);
        assert_eq!(frag, *lang != "hi", "{lang}: fragment capability changed -- check the tiers it offers");
    }
}

/// Per-script verdicts: D8 tones for Mandarin, tones for Vietnamese, composed
/// jamo for Korean, and the D4 alternates Japanese lists.
#[test]
fn verdicts_hold_in_every_script() {
    let zh = table::load("zh").unwrap();
    assert_eq!(judge(&zh, "qi1", 7), Verdict::Correct, "tone number for qī");
    assert_eq!(judge(&zh, "qi", 7), Verdict::Misspelled, "D8: untoned is a misspelling");
    assert_eq!(judge(&zh, "qi2", 7), Verdict::Misspelled, "D8: wrong tone is a misspelling");
    assert_eq!(judge(&zh, "ba1", 7), Verdict::WrongValue, "bā is eight");
    let vi = table::load("vi").unwrap();
    assert_eq!(judge(&vi, "bay\u{0309}", 7), Verdict::Correct, "tone from the tone row");
    assert_eq!(judge(&vi, "bảy", 7), Verdict::Correct);
    assert_eq!(judge(&vi, "bay", 7), Verdict::Misspelled, "the tone is required");
    let ko = table::load("ko").unwrap();
    let typed = jamo_keys("일곱").iter().fold(String::new(), |a, &k| crate::hangul::feed(&a, k));
    assert_eq!(judge(&ko, &typed, 7), Verdict::Correct, "native 일곱");
    assert_eq!(judge(&ko, "칠", 7), Verdict::Correct, "Sino-Korean 칠 (D4)");
    let ja = table::load("ja").unwrap();
    assert_eq!(judge(&ja, "よん", 4), Verdict::Correct);
    assert_eq!(judge(&ja, "し", 4), Verdict::Correct, "D4: yon and shi both count");
    let ar = table::load("ar").unwrap();
    assert_eq!(judge(&ar, "سبعة", 7), Verdict::Correct);
    let de = table::load("de").unwrap();
    assert_eq!(judge(&de, "Fünf", 5), Verdict::Correct, "case folds, the umlaut does not");
    assert_eq!(judge(&de, "funf", 5), Verdict::Misspelled);
}

/// F9 / Done #9: hints are a cell and a technique -- the types carry no letters.
#[test]
fn hints_point_and_name_but_never_spell() {
    let t = en();
    let p = generate(3, &Config { size: S9, tier: Tier::Medium }, &t).unwrap();
    let geo = Geo::new(S9);
    let (fixed, restrict) = p.constraints(&t);
    let (cell, _tech) = next_step(&geo, &fixed, &restrict).expect("a hint exists");
    assert_eq!(fixed[cell], 0, "the hint points at an empty cell");
    let _ = S6;
}

/// Done #3 / I5: the pinned digest. The browser spec asks the wasm build for
/// the same number; a mismatch on any platform fails.
pub const GOLDEN: u64 = 0x660953a49c47d76c;

#[test]
fn the_golden_digest_is_pinned() {
    let got = super::gen::golden_digest(&en());
    assert_eq!(got, GOLDEN, "golden digest moved: {got:#x}");
}

/// Done #10 / I9, and D6: SpellDoku code never reaches the leaderboard, any
/// score service, or the shields.
#[test]
fn spelldoku_never_touches_the_leaderboard_or_shields() {
    let sources = [
        include_str!("mod.rs"),
        include_str!("canon.rs"),
        include_str!("geo.rs"),
        include_str!("gen.rs"),
        include_str!("play.rs"),
        include_str!("rng.rs"),
        include_str!("solve.rs"),
        include_str!("table.rs"),
        include_str!("../spelldoku_ui.rs"),
    ];
    for needle in ["climb::", "submit_run", "submit-chain", "/api/climb", "/api/match", "online_spelloff", "leaderboard(", "shields", "attempts::"] {
        for src in sources {
            // The rule's own statement in a comment is allowed; a call is not.
            let code: String = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
            assert!(!code.contains(needle), "SpellDoku code must not reference {needle}");
        }
    }
}
