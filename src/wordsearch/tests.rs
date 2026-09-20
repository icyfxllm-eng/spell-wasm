//! CC-WORDGRID §9 acceptance tests for Phase A (Spell Search + F-X1–F-X6).
//!
//! The invariant checker below is written apart from the generator: it reads
//! the finished grid from every cell in all eight directions, so a generator
//! bug cannot hide behind a shared helper.

use std::collections::{HashMap, HashSet};

use super::confusion::{self, decoys, edits, has_decoys};
use super::gen::{hash, judge, path, Hit, Puzzle, Tier, DIRS};
use super::hint::passes;
use super::ledger::{daily_gap, fits, Ledger, WINDOW_DAYS, WINDOW_PUZZLES};
use super::lexicon::{eligible, graphemes, pool, pool_floor, tiers_for, Lexicon, LANGS};
use super::serve::{self, bank, daily, from_list};
use crate::spelldoku::rng::{fnv, Rng};

/// Every configuration a player can be offered, Jr included.
fn configs() -> Vec<(&'static str, Tier)> {
    let mut v = Vec::new();
    for lang in LANGS {
        for t in tiers_for(lang, true).into_iter().chain(tiers_for(lang, false)) {
            v.push((lang, t));
        }
    }
    v
}

/// Every reading of `len` cells from `start` in direction `d`, if it fits.
fn ray(n: usize, start: usize, d: (i32, i32), len: usize) -> Option<Vec<usize>> {
    let (r, c) = ((start / n) as i32, (start % n) as i32);
    let (er, ec) = (r + d.0 * (len as i32 - 1), c + d.1 * (len as i32 - 1));
    if er < 0 || ec < 0 || er >= n as i32 || ec >= n as i32 {
        return None;
    }
    Some((0..len as i32).map(|k| ((r + d.0 * k) * n as i32 + c + d.1 * k) as usize).collect())
}

fn sorted(v: &[usize]) -> Vec<usize> {
    let mut s = v.to_vec();
    s.sort_unstable();
    s
}

/// I1, I2, I3, I9 on a finished puzzle. `Err` names the first failure.
/// `exact`: a bank puzzle must carry its tier's full decoy count; a My Words or
/// Daily puzzle carries as many as its words have between them.
fn check(p: &Puzzle, lang: &str, exact: bool) -> Result<(), String> {
    let n = p.size;
    let lex = Lexicon::get(lang);
    let placed: Vec<(&str, &Vec<usize>)> =
        p.targets.iter().map(|t| (t.word.as_str(), &t.cells)).chain(p.decoys.iter().map(|(d, _)| (d.word.as_str(), &d.cells))).collect();

    // I9: targets and decoys all distinct.
    let words: HashSet<&str> = placed.iter().map(|(w, _)| *w).collect();
    if words.len() != placed.len() {
        return Err("I9: a word appears twice in one puzzle".into());
    }
    // Placements really spell their words, in the tier's directions.
    for (w, cells) in &placed {
        let spelled: String = cells.iter().map(|&i| p.grid[i].as_str()).collect();
        if spelled != *w {
            return Err(format!("placement of {w} spells {spelled}"));
        }
        let step = cells[1] as i64 - cells[0] as i64;
        let ok = p.tier.dirs().iter().any(|&(dr, dc)| step == dr as i64 * n as i64 + dc as i64);
        if !ok {
            return Err(format!("{w} runs in a direction {} does not use", p.tier.id()));
        }
    }
    // I1 / I9: each reads exactly once across all 8 directions.
    let mut found: HashMap<&str, HashSet<Vec<usize>>> = HashMap::new();
    let mut runs: Vec<(String, Vec<usize>)> = Vec::new();
    for start in 0..n * n {
        for d in DIRS {
            for len in 3..=n {
                let Some(cells) = ray(n, start, d, len) else { break };
                let s: String = cells.iter().map(|&i| p.grid[i].as_str()).collect();
                if words.contains(s.as_str()) {
                    found.entry(words.get(s.as_str()).copied().unwrap()).or_default().insert(sorted(&cells));
                }
                runs.push((s, cells));
            }
        }
    }
    for (w, _) in &placed {
        let k = found.get(w).map_or(0, HashSet::len);
        if k != 1 {
            return Err(format!("I1: {w} reads {k} times"));
        }
    }
    // I3: no accidental bank or blocklisted word outside a placed word.
    for (s, cells) in &runs {
        if words.contains(s.as_str()) {
            continue;
        }
        let bad = (graphemes(s).len() >= 4 && lex.words.contains(s)) || crate::profanity::is_blocked(s);
        let inside = placed.iter().any(|(_, pc)| cells.iter().all(|c| pc.contains(c)));
        if bad && !inside {
            return Err(format!("I3: filler spells {s}"));
        }
    }
    // I2: each decoy is one confusion edit from its target, not a word, not a
    // target or its reverse, not blocklisted.
    let targets: HashSet<String> = p.targets.iter().map(|t| t.word.clone()).collect();
    for (d, t) in &p.decoys {
        let target = &p.targets[*t].word;
        let rev: String = graphemes(target).into_iter().rev().collect();
        if !edits(lang, target).contains(&d.word) {
            return Err(format!("I2: {} is not one confusion edit from {target}", d.word));
        }
        if lex.words.contains(&d.word) || targets.contains(&d.word) || d.word == rev || crate::profanity::is_blocked(&d.word) {
            return Err(format!("I2: decoy {} is a word, a target or blocked", d.word));
        }
    }
    // F-S3: the tier's decoy count, where the language has decoys.
    if has_decoys(lang) {
        let cap = p.tier.decoys_per_target();
        let have: usize = p.targets.iter().map(|t| decoys(lang, &t.word).len().min(cap)).sum();
        let tier_wants = p.tier.decoy_count(p.targets.len());
        let want = if exact { tier_wants } else { tier_wants.min(have) };
        if p.decoys.len() != want {
            return Err(format!("F-S3: {} decoys, the tier wants {want}", p.decoys.len()));
        }
    } else if !p.decoys.is_empty() {
        return Err("D11: a language without a dictionary got decoys".into());
    }
    Ok(())
}

fn sweep(seeds: u64) {
    let mut unserved: Vec<String> = Vec::new();
    for (lang, tier) in configs() {
        let mut failed_to_serve = 0;
        for s in 0..seeds {
            let ledger = Ledger { counter: s, ..Default::default() };
            let Some(served) = bank(lang, tier, &ledger, 0) else {
                failed_to_serve += 1;
                continue;
            };
            let p = &served.puzzle;
            if let Err(e) = check(p, lang, true) {
                panic!("{lang} {} seed {s} (counter {s}, day 0): {e}", tier.id());
            }
            // I8: the same inputs build the same grid.
            let again = bank(lang, tier, &ledger, 0).expect("served once, serves again");
            assert_eq!(hash(p), hash(&again.puzzle), "{lang} {} seed {s}: not deterministic", tier.id());
        }
        if failed_to_serve > 0 {
            unserved.push(format!("{lang} {}: {failed_to_serve} of {seeds}", tier.id()));
        }
        eprintln!("swept {lang} {}: {seeds} seeds, {failed_to_serve} unserved", tier.id());
    }
    assert!(unserved.is_empty(), "seeds that served nothing: {unserved:?}");
}

/// Test 1, at gate size. The full 10,000-seed sweep is the ignored test below.
#[test]
fn wordgrid_property() {
    sweep(12);
}

/// Test 1 in full: 10,000 seeds × every eligible language × every tier.
/// Run in release: cargo test --release --lib --features audit_preview wordgrid_property_10k -- --ignored
#[test]
#[ignore]
fn wordgrid_property_10k() {
    sweep(10_000);
}

/// Test 2. The host computes this digest; the e2e computes it again in the
/// app's WebAssembly (`__spelltest.spellSearchGolden`) and compares.
pub const GOLDEN: u64 = 0x262e089cc61b94ff;

#[test]
fn wordgrid_determinism() {
    assert_eq!(serve::golden_digest(500), GOLDEN, "the 500-seed grid digest moved");
}

/// Test 3. A player on a real pool never meets a word again inside the window;
/// a shrunk pool relaxes, logs it, and still serves.
#[test]
fn wordgrid_no_repeat() {
    let tiers = Tier::ALL.map(|t| ("en", t)).into_iter().chain([("es", Tier::Easy), ("ru", Tier::Jr)]);
    for (lang, tier) in tiers {
        if pool(lang, tier).len() < pool_floor(tier) {
            continue; // below E4 the window cannot hold; covered by the shrunk case
        }
        let mut ledger = Ledger::default();
        let mut history: Vec<(u64, u32, Vec<String>)> = Vec::new();
        for i in 0..60u32 {
            let day = i / 2; // two puzzles a day
            let s = bank(lang, tier, &ledger, day).unwrap_or_else(|| panic!("{lang} {} puzzle {i}", tier.id()));
            assert_eq!(s.relaxed, 0, "{lang} {}: relaxed at puzzle {i} on a pool above E4", tier.id());
            let words: Vec<String> = s.puzzle.targets.iter().map(|t| t.word.clone()).collect();
            for (n, d, old) in &history {
                let inside = ledger.counter - n < WINDOW_PUZZLES || day - d < WINDOW_DAYS;
                if inside {
                    for w in &words {
                        assert!(!old.contains(w), "{lang} {}: {w} repeated inside the window", tier.id());
                    }
                }
            }
            history.push((ledger.counter, day, words.clone()));
            ledger.record(&s.key, &words, day, s.relaxed);
        }
    }

    let small: Vec<String> = pool("en", Tier::Easy).into_iter().take(30).collect();
    let mut ledger = Ledger::default();
    let mut rng = Rng::new(7);
    let fit = |p: &[String], w: &str| fits("en", 9, p, w);
    for _ in 0..10 {
        let mut picked = Vec::new();
        let relaxed = ledger.select("en:easy", &small, &mut picked, 8, 0, &mut rng, &fit);
        assert_eq!(picked.len(), 8, "a shrunk pool still serves a full puzzle");
        ledger.record("en:easy", &picked, 0, relaxed);
    }
    assert!(ledger.relaxations > 0, "and the relaxation is logged");
}

/// Test 4. The same list, five puzzles in a row, five different layouts.
#[test]
fn wordgrid_mywords_variation() {
    let list: Vec<String> =
        ["planet", "garden", "yellow", "rabbit", "window", "orange", "basket", "candle", "button", "pocket"].map(String::from).to_vec();
    for tier in [Tier::Jr, Tier::Easy, Tier::Hard] {
        let hashes: HashSet<u64> = (0..5).map(|c| hash(&from_list("en", tier, "list-1", &list, c).expect("serves").puzzle)).collect();
        assert_eq!(hashes.len(), 5, "{}: five puzzles from one list must differ", tier.id());
        let p = from_list("en", tier, "list-1", &list, 0).unwrap().puzzle;
        check(&p, "en", false).unwrap();
    }
}

/// Test 5. Definitions that give the word away are all blocked; clean ones are
/// all kept.
#[test]
fn wordgrid_hint_filter() {
    let giveaway = [
        ("en", "receive", "to receive something from someone"),
        ("en", "receive", "the act of RECEIVING a gift"),
        ("en", "running", "moving fast on foot; a runner does this"),
        ("en", "beautiful", "full of beauty"),
        ("en", "cat", "a small domestic cat"),
        ("en", "cat", "Cats purr"),
        ("es", "canción", "una cancion corta"),
        ("es", "escribir", "acción de escribirse una carta"),
        ("es", "niño", "Nino pequeño"),
        ("ru", "молоко", "белое молочное питьё"),
        ("ru", "ёлка", "игрушки для ёлки"),
        ("ru", "ёлка", "елка в лесу"),
    ];
    for (lang, word, def) in giveaway {
        assert!(!passes(word, def), "{lang}: \"{def}\" gives away {word}");
    }
    let clean = [
        ("en", "receive", "to be given something by another person"),
        ("en", "cat", "a small furry pet that purrs"),
        ("en", "beautiful", "very pleasing to look at"),
        ("es", "canción", "música con letra para cantar"),
        ("es", "niño", "persona de poca edad"),
        ("ru", "молоко", "белый напиток от коровы"),
        ("ru", "ёлка", "хвойное дерево"),
    ];
    for (lang, word, def) in clean {
        assert!(passes(word, def), "{lang}: \"{def}\" was blocked for {word} but gives nothing away");
    }
}

/// Test 6. No shipped decoy is a dictionary word, and each is exactly one
/// confusion edit from its target (the Rust rules agree with the build tool).
#[test]
fn wordgrid_decoy_audit() {
    let web2: Option<HashSet<String>> = std::fs::read_to_string("/usr/share/dict/web2")
        .ok()
        .map(|s| s.lines().map(|l| l.trim().to_lowercase()).collect());
    let lex = Lexicon::get("en");
    let table = confusion::all("en");
    assert!(table.len() > 1000, "the English decoy table is present");
    for (target, ds) in &table {
        let e = edits("en", target);
        for d in ds {
            assert!(e.contains(d), "{d} is not one confusion edit from {target}");
            let rd: String = d.chars().rev().collect();
            assert!(!target.contains(d.as_str()) && !d.contains(target.as_str()) && !target.contains(&rd) && !rd.contains(target.as_str()),
                "{d} reads inside {target} or around it");
            assert!(!lex.words.contains(d), "{d} is a bank word");
            if let Some(w) = &web2 {
                assert!(!w.contains(d), "{d} is in web2");
            }
        }
    }
    for lang in LANGS.iter().filter(|l| **l != "en") {
        assert!(!has_decoys(lang) && decoys(lang, "casa").is_empty(), "{lang} has no E6 dictionary, so no decoys (D11)");
    }
}

/// Test 10. A misspelling reaches the base game's learner log the same way from
/// Lock It In as from the base game: one recording function, the typed channel.
#[test]
fn wordgrid_stats_parity() {
    use crate::learner::{attempt, Channel};
    let base = attempt("en", "receive", false, Channel::Typed, Some("recieve"), 100);
    let lock_in = super::lock_in_attempt("en", "receive", "recieve", false, 100);
    assert_eq!(serde_json::to_string(&base).unwrap(), serde_json::to_string(&lock_in).unwrap());
    // TRAP_MISS lands in the same log, as a miss on the target with the decoy typed.
    let trap = super::trap_attempt("en", "receive", "recieve", 100);
    assert_eq!(trap.channel, Channel::Trap);
    assert!(!trap.correct && trap.typed.as_deref() == Some("recieve") && trap.word == "receive");
    assert_eq!(trap.skills, base.skills, "and exercises the same skills");
}

/// Test 12. An under-13 player reaches Jr only: → ↓, no decoys.
#[test]
fn wordgrid_jr() {
    for lang in LANGS {
        assert_eq!(tiers_for(lang, true), vec![Tier::Jr], "{lang}");
        for s in 0..20 {
            let p = bank(lang, Tier::Jr, &Ledger { counter: s, ..Default::default() }, 0).unwrap().puzzle;
            assert!(p.decoys.is_empty());
            for t in &p.targets {
                let step = t.cells[1] - t.cells[0];
                assert!(step == 1 || step == p.size, "{lang} Jr: {} runs off → ↓", t.word);
            }
        }
    }
    assert!(!Tier::Jr.dirs().contains(&(1, 1)) && Tier::Jr.decoy_count(8) == 0);
}

/// Test 13 / I11. Nothing in Spell Search reads or writes the shield count.
#[test]
fn wordgrid_shields() {
    for (name, src) in [
        ("gen", include_str!("gen.rs")),
        ("ledger", include_str!("ledger.rs")),
        ("serve", include_str!("serve.rs")),
        ("mod", include_str!("mod.rs")),
        ("confusion", include_str!("confusion.rs")),
        ("lexicon", include_str!("lexicon.rs")),
        ("hint", include_str!("hint.rs")),
        ("ui", include_str!("../wordsearch_ui.rs")),
    ] {
        assert!(!src.to_lowercase().contains(&["shi", "eld"].concat()), "wordsearch/{name} mentions shields");
    }
}

/// Test 14 and D4. Korean and Chinese never appear, whatever a gate says.
#[test]
fn wordgrid_langs() {
    for lang in ["ko", "zh", "ja", "hi", "vi", "ar", "sw"] {
        assert!(!eligible(lang) && tiers_for(lang, false).is_empty() && tiers_for(lang, true).is_empty(), "{lang}");
    }
    for lang in LANGS {
        assert!(eligible(lang));
    }
}

/// E3 / I6: every launch language has a server voice in the live backend's table.
#[test]
fn every_language_has_a_server_voice() {
    let app = include_str!("../../backend/app.py");
    let voices = app.split("LANG_VOICES = {").nth(1).and_then(|s| s.split('}').next()).expect("LANG_VOICES");
    for lang in LANGS {
        assert!(voices.contains(&format!("\"{lang}\":")), "{lang} has no server voice");
    }
}

/// E4: every offered tier's pool meets targets × 20 × 1.5, or the no-repeat
/// test above could not mean anything for it.
#[test]
fn pools_meet_e4() {
    for (lang, tier) in configs() {
        let n = pool(lang, tier).len();
        assert!(n >= pool_floor(tier), "{lang} {}: {n} words, E4 wants {}", tier.id(), pool_floor(tier));
    }
}

/// The Daily is the same for everyone on a date, differs day to day, and its
/// words stay away for as long as the pool allows.
#[test]
fn daily_is_shared_and_spaced() {
    for (lang, tier) in [("en", Tier::Easy), ("es", Tier::Easy), ("en", Tier::Jr)] {
        let a = daily(lang, tier, 20260918, 20_714).unwrap();
        let b = daily(lang, tier, 20260918, 20_714).unwrap();
        assert_eq!(hash(&a.puzzle), hash(&b.puzzle));
        let c = daily(lang, tier, 20260919, 20_715).unwrap();
        assert_ne!(hash(&a.puzzle), hash(&c.puzzle));
        check(&a.puzzle, lang, false).unwrap();
        let gap = daily_gap(pool(lang, tier).len(), tier.targets());
        let words = |d: u32| daily(lang, tier, d, d).unwrap().puzzle.targets.into_iter().map(|t| t.word).collect::<HashSet<_>>();
        let first = words(0);
        for d in 1..gap.min(40) as u32 {
            assert!(first.is_disjoint(&words(d)), "{lang} {}: day {d} repeats day 0 inside the {gap}-day gap", tier.id());
        }
    }
}

/// Drag geometry: straight lines only, either way.
#[test]
fn drags_and_hits() {
    assert_eq!(path(9, 0, 8), Some((0..9).collect()));
    assert_eq!(path(9, 0, 80), Some((0..9).map(|k| k * 10).collect()));
    assert_eq!(path(9, 0, 11), None);
    let p = bank("en", Tier::Hard, &Ledger::default(), 0).unwrap().puzzle;
    let t = &p.targets[0].cells;
    assert_eq!(judge(&p, t), Hit::Target(0));
    let back: Vec<usize> = t.iter().rev().copied().collect();
    assert_eq!(judge(&p, &back), Hit::Target(0));
    assert_eq!(judge(&p, &p.decoys[0].0.cells), Hit::Decoy(0));
    assert_eq!(judge(&p, &t[..2]), Hit::Miss);
}

/// F-X2: the seed is a hash of the source, the counter and the date only.
#[test]
fn seeds_follow_their_inputs() {
    use super::ledger::seed;
    assert_eq!(seed("bank:en:easy", 3, 0), fnv(b"bank:en:easy|3|0"));
    assert_ne!(seed("bank:en:easy", 3, 0), seed("bank:en:easy", 4, 0));
}

/// Phase C / D9: the Daily keeps a word away for 90 days, in every language
/// and both modes, and is the same puzzle for everyone on a date. The window
/// is a property of the pool: one tier's slice could only manage 32 days in
/// Spanish, so the Daily draws from the whole bank.
#[test]
fn the_daily_holds_a_ninety_day_window() {
    use super::lexicon::daily_pool;
    const WINDOW: u32 = super::ledger::DAILY_WINDOW_DAYS;
    for lang in LANGS {
        for tier in [Tier::Jr, Tier::Easy] {
            let pool_len = daily_pool(lang, tier).len();
            let gap = daily_gap(pool_len, tier.targets());
            assert!(gap as u32 >= WINDOW, "{lang} {}: a word returns after {gap} days", tier.id());

            // Walk a year of Spell Search Dailies and hold every word to it.
            let mut seen: Vec<(String, u32)> = Vec::new();
            for day in 0..365u32 {
                let s = daily(lang, tier, 20260101 + day, day).unwrap_or_else(|| panic!("{lang} {} day {day}", tier.id()));
                for t in &s.puzzle.targets {
                    if let Some((_, was)) = seen.iter().find(|(w, _)| *w == t.word) {
                        assert!(day - was >= WINDOW, "{lang} {}: {} came back after {} days", tier.id(), t.word, day - was);
                    }
                    seen.retain(|(w, _)| *w != t.word);
                    seen.push((t.word.clone(), day));
                }
            }
        }
    }
}

/// The same date gives every player the same Daily, in both modes.
#[test]
fn the_daily_is_one_puzzle_for_everyone() {
    for lang in ["en", "es", "ru"] {
        let a = daily(lang, Tier::Easy, 20260405, 20_910).unwrap();
        let b = daily(lang, Tier::Easy, 20260405, 20_910).unwrap();
        assert_eq!(hash(&a.puzzle), hash(&b.puzzle), "{lang}: two players, one Spell Search Daily");
        let c = daily(lang, Tier::Easy, 20260406, 20_911).unwrap();
        assert_ne!(hash(&a.puzzle), hash(&c.puzzle), "{lang}: a new day, a new puzzle");
    }
}
