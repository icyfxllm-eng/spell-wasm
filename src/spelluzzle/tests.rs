//! CC-SPELLUZZLE Phase A acceptance tests A1 to A8 (§7).
//!
//! Counts default small so `cargo test` stays quick. The full run the spec asks
//! for (10,000 seeds per tier) is:
//!   SPZ_N=10000 cargo test --release spelluzzle::tests::a3 -- --nocapture
//! and `SPZ_VALIDITY_FILE=<one word per line>` swaps in a larger validity list.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use proptest::prelude::*;

use super::bank;
use super::gates::{check_board, count_solutions, GateFail};
use super::gen::{generate, generate_after, ATTEMPT_CAP};
use super::lex::{pattern, LexInput, Lexicon};
use super::types::{Board, SlotKind, Tier};
use super::view::{board_view, is_solved, slot_cells, wrong_slots, Entries, RuneState};

fn n_seeds(default: usize) -> usize {
    std::env::var("SPZ_N").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn en() -> &'static Lexicon {
    static L: OnceLock<Lexicon> = OnceLock::new();
    L.get_or_init(|| {
        let extra = std::env::var("SPZ_VALIDITY_FILE")
            .ok()
            .map(|p| std::fs::read_to_string(p).expect("validity file").lines().map(str::to_string).collect())
            .unwrap_or_default();
        bank::load("en", extra).expect("the English bank loads")
    })
}

fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn board_of(lex: &Lexicon, tier: Tier, seed: u64) -> Board {
    generate(seed, tier, lex).unwrap_or_else(|e| panic!("{tier:?} seed {seed}: {e:?}")).board
}

// ---- A1 view_is_pure -------------------------------------------------------

/// The view written straight from the definition, with no shared code.
fn reference_view(board: &Board, commits: &[(usize, Vec<char>)]) -> Vec<RuneState> {
    let mut pairs: Vec<(u8, char)> = Vec::new();
    for (i, typed) in commits {
        for (&r, &u) in board.slots[*i].runes.iter().zip(typed) {
            pairs.push((r, u));
        }
    }
    (0..board.n_runes() as u8)
        .map(|r| {
            let mut us: Vec<char> = pairs.iter().filter(|p| p.0 == r).map(|p| p.1).collect();
            us.sort_unstable();
            us.dedup();
            match us.len() {
                0 => RuneState::Unknown,
                1 => {
                    let others = pairs.iter().any(|p| p.1 == us[0] && p.0 != r);
                    if others {
                        RuneState::Contested
                    } else {
                        RuneState::Decoded(us[0])
                    }
                }
                _ => RuneState::Contested,
            }
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn a1_view_is_pure(seed in 0u64..400, tier_i in 0usize..5, picks in proptest::collection::vec((any::<bool>(), any::<u8>(), any::<u8>()), 7), order in any::<u64>()) {
        let lex = en();
        let b = board_of(lex, Tier::ALL[tier_i], seed);
        let mut commits: Vec<(usize, Vec<char>)> = Vec::new();
        for (i, (on, cell, unit)) in picks.iter().enumerate().take(b.slots.len()) {
            if !*on { continue }
            let mut typed = b.slots[i].answer.clone();
            if *unit % 3 == 0 {
                let k = *cell as usize % typed.len();
                typed[k] = lex.alphabet[*unit as usize % lex.alphabet.len()];
            }
            commits.push((i, typed));
        }
        let expect = reference_view(&b, &commits);
        let mut shuffled = commits.clone();
        crate::spelldoku::rng::Rng::new(order).shuffle(&mut shuffled);
        let entries: Entries = shuffled.into_iter().collect();
        prop_assert_eq!(board_view(&b, &entries), expect);
    }
}

// ---- A2 clash_rules --------------------------------------------------------

fn fixture_easy_1() -> Board {
    Board::from_words_sorted("en", Tier::Easy, &["global", "worst", "mobile", "agree", "mirror"], &[], "waste")
}

fn rune_at(b: &Board, slot: usize, cell: usize) -> u8 {
    b.slots[slot].runes[cell]
}

#[test]
fn a2_clash_rules() {
    // "definate" in the `definite` slot contests the rune in cells 4 and 6.
    let b = Board::from_words_sorted("en", Tier::Hard, &["definite", "tide", "inside", "bite"], &["identity"], "tied");
    let mut e = Entries::new();
    e.insert(0, chars("definate"));
    let v = board_view(&b, &e);
    let r = rune_at(&b, 0, 3);
    assert_eq!(r, rune_at(&b, 0, 5), "cells 4 and 6 share a rune");
    assert_eq!(v[r as usize], RuneState::Contested);
    assert_eq!(slot_cells(&b, &v, 0)[3].state, RuneState::Contested);
    assert_eq!(slot_cells(&b, &v, 0)[5].state, RuneState::Contested);
    assert_eq!(slot_cells(&b, &v, 2)[0].state, RuneState::Contested, "`inside` starts with the same rune");

    // fixture-easy-1: `worst` committed, then "mirrer" in the `mirror` slot.
    let b = fixture_easy_1();
    let mut e = Entries::new();
    e.insert(1, chars("worst"));
    let v = board_view(&b, &e);
    let o = rune_at(&b, 1, 1);
    assert_eq!(v[o as usize], RuneState::Decoded('o'));
    e.insert(4, chars("mirrer"));
    let v = board_view(&b, &e);
    assert_eq!(v[o as usize], RuneState::Contested);
    let cells: usize = (0..b.slots.len()).map(|i| b.slots[i].runes.iter().filter(|&&r| r == o).count()).sum();
    assert_eq!(cells, 4, "the o-rune has 4 cells on the board");
    for s in [0, 1, 2, 4] {
        for (k, c) in slot_cells(&b, &v, s).iter().enumerate() {
            if b.slots[s].runes[k] == o {
                assert_eq!(c.state, RuneState::Contested, "slot {s} cell {k}");
            }
        }
    }

    // One unit on two runes is contested, in both runes.
    let b = Board::from_words_sorted("en", Tier::Easy, &["cat", "dog"], &[], "act");
    let mut e = Entries::new();
    e.insert(0, chars("cat"));
    e.insert(1, chars("dat"));
    let v = board_view(&b, &e);
    let (rc, rd) = (rune_at(&b, 0, 0), rune_at(&b, 1, 0));
    // c and d both typed as 'c' and 'd'; make them collide on 'a' instead.
    assert_ne!(rc, rd);
    let mut e2 = Entries::new();
    e2.insert(0, chars("cat"));
    e2.insert(1, chars("cog")); // d->c now collides with c->c
    let v2 = board_view(&b, &e2);
    assert_eq!(v2[rc as usize], RuneState::Contested);
    assert_eq!(v2[rd as usize], RuneState::Contested);
    let _ = v;
}

#[test]
fn clash_clears_when_entries_agree_again() {
    let b = fixture_easy_1();
    let mut e = Entries::new();
    e.insert(1, chars("worst"));
    e.insert(4, chars("mirrer"));
    assert!(board_view(&b, &e).contains(&RuneState::Contested));
    e.insert(4, chars("mirror"));
    assert!(!board_view(&b, &e).contains(&RuneState::Contested));
}

#[test]
fn solved_needs_every_slot_right_and_nothing_contested() {
    let b = fixture_easy_1();
    let mut e = Entries::new();
    for (i, s) in b.slots.iter().enumerate() {
        e.insert(i, s.answer.clone());
    }
    assert!(is_solved(&b, &e));
    e.insert(4, chars("mirrer"));
    assert!(!is_solved(&b, &e));
    assert_eq!(wrong_slots(&b, &e), vec![4]);
    e.remove(&0);
    assert!(!is_solved(&b, &e), "a missing slot is unsolved");
}

// ---- A3 gates_hold and A4 one_solution -------------------------------------

fn each_tier_boards(per_tier: usize, mut f: impl FnMut(Tier, u64, &Board)) {
    let lex = en();
    for tier in Tier::ALL {
        for seed in 0..per_tier as u64 {
            f(tier, seed, &board_of(lex, tier, seed));
        }
    }
}

#[test]
fn a3_gates_hold() {
    let n = n_seeds(40);
    let lex = en();
    for tier in Tier::ALL {
        let (mut ok, mut attempts, mut cap) = (0, Vec::new(), 0);
        let (mut earned, mut cascade, mut runes, mut hashes) = (Vec::new(), Vec::new(), 0usize, std::collections::BTreeSet::new());
        for seed in 0..n as u64 {
            match generate(seed, tier, lex) {
                Ok(g) => {
                    // Re-run the checker on the served board: acceptance is the checker's, not the generator's.
                    let m = check_board(&g.board, lex).unwrap_or_else(|e| panic!("{tier:?} seed {seed} served a board the checker rejects: {e:?}"));
                    earned.push(m.earned.as_f64());
                    cascade.push(m.cascade.as_f64());
                    runes += m.runes;
                    hashes.insert(g.board.hash());
                    assert!(g.attempts <= 2 * ATTEMPT_CAP);
                    attempts.push(g.attempts);
                    ok += 1;
                }
                Err(_) => cap += 1,
            }
        }
        attempts.sort_unstable();
        let q = |p: f64| attempts.get(((attempts.len() as f64 * p) as usize).min(attempts.len().saturating_sub(1))).copied().unwrap_or(0);
        println!("A3 {:>6}: {ok}/{n} ok, {cap} hit the cap, attempts p50 {} p95 {} max {}", tier.name(), q(0.5), q(0.95), attempts.last().copied().unwrap_or(0));
        let pct = |v: &mut Vec<f64>, p: f64| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v.get(((v.len() as f64 * p) as usize).min(v.len().saturating_sub(1))).copied().unwrap_or(0.0)
        };
        println!(
            "      runes avg {:.1}; earned p10/p50/p90 {:.3}/{:.3}/{:.3}; cascade {:.3}/{:.3}/{:.3}; {} distinct boards of {ok}",
            runes as f64 / ok.max(1) as f64,
            pct(&mut earned, 0.1), pct(&mut earned, 0.5), pct(&mut earned, 0.9),
            pct(&mut cascade, 0.1), pct(&mut cascade, 0.5), pct(&mut cascade, 0.9),
            hashes.len()
        );
        assert_eq!(cap, 0, "{tier:?}: {cap} of {n} seeds hit the attempt cap");
    }
}

/// An independent solver for A4. It shares nothing with `gates::count_solutions`:
/// it scans the whole validity list rather than a pattern index, and it fills
/// slots in board order.
fn reference_count(board: &Board, lex: &Lexicon, limit: u32) -> u32 {
    fn go(board: &Board, lex: &Lexicon, i: usize, r2u: &mut BTreeMap<u8, char>, found: &mut u32, limit: u32) {
        if *found >= limit {
            return;
        }
        if i == board.slots.len() {
            *found += 1;
            return;
        }
        let s = &board.slots[i];
        let cands: Vec<Vec<char>> = if s.kind == SlotKind::Spoken {
            let g = lex.group(&s.answer);
            if g.is_empty() { vec![s.answer.clone()] } else { g.to_vec() }
        } else {
            lex.all_valid().to_vec()
        };
        for w in cands.iter().filter(|w| w.len() == s.answer.len()) {
            let mut trial = r2u.clone();
            let mut ok = true;
            for (&r, &u) in s.runes.iter().zip(w) {
                match trial.get(&r) {
                    Some(&p) if p != u => { ok = false; break }
                    Some(_) => {}
                    None => {
                        if trial.values().any(|&x| x == u) { ok = false; break }
                        trial.insert(r, u);
                    }
                }
            }
            if ok {
                go(board, lex, i + 1, &mut trial, found, limit);
            }
        }
    }
    let mut found = 0;
    go(board, lex, 0, &mut BTreeMap::new(), &mut found, limit);
    found
}

#[test]
fn a4_one_solution() {
    let n = n_seeds(6);
    let lex = en();
    each_tier_boards(n, |tier, seed, b| {
        assert_eq!(count_solutions(b, lex, 2), 1, "{tier:?} {seed}: the checker's solver");
        assert_eq!(reference_count(b, lex, 2), 1, "{tier:?} {seed}: the independent solver");
    });
}

// ---- A5 single_error_clash -------------------------------------------------

#[test]
fn a5_single_error_clash() {
    let n = n_seeds(12);
    let lex = en();
    let mut tried = 0u64;
    each_tier_boards(n, |tier, seed, b| {
        let correct: Entries = b.slots.iter().enumerate().map(|(i, s)| (i, s.answer.clone())).collect();
        for i in (0..b.slots.len()).filter(|&i| b.slots[i].kind == SlotKind::Spoken) {
            let ans = &b.slots[i].answer;
            let mut wrongs: Vec<Vec<char>> = Vec::new();
            for k in 0..ans.len() {
                for &u in &lex.alphabet {
                    if u != ans[k] {
                        let mut w = ans.clone();
                        w[k] = u;
                        wrongs.push(w);
                    }
                }
            }
            for k in 0..ans.len() - 1 {
                if ans[k] != ans[k + 1] {
                    let mut w = ans.clone();
                    w.swap(k, k + 1);
                    wrongs.push(w);
                }
            }
            for w in wrongs {
                let mut e = correct.clone();
                e.insert(i, w.clone());
                tried += 1;
                assert!(
                    board_view(b, &e).contains(&RuneState::Contested),
                    "{tier:?} seed {seed}: `{}` in the `{}` slot made no clash",
                    w.iter().collect::<String>(),
                    ans.iter().collect::<String>()
                );
            }
        }
    });
    println!("A5: {tried} single errors, every one clashed");
}

// ---- A6 deterministic_boards -----------------------------------------------

const GOLDEN: &str = include_str!("../../tests/fixtures/spelluzzle/golden-en.json");
const GOLDEN_SEEDS: u64 = 100;

fn fingerprint(lex: &Lexicon) -> u64 {
    let mut s = String::new();
    for t in Tier::ALL {
        s.push_str(t.name());
        for w in lex.pool(t) {
            s.push(' ');
            s.extend(w.units.iter());
        }
        s.push('\n');
    }
    s.push_str(&format!("{}", lex.validity_len()));
    crate::spelldoku::rng::fnv(s.as_bytes())
}

fn golden_now(lex: &Lexicon) -> serde_json::Value {
    let mut boards = serde_json::Map::new();
    for tier in Tier::ALL {
        let hs: Vec<String> = (0..GOLDEN_SEEDS).map(|s| format!("{:016x}", board_of(lex, tier, s).hash())).collect();
        boards.insert(tier.name().to_string(), serde_json::json!(hs));
    }
    serde_json::json!({ "fingerprint": format!("{:016x}", fingerprint(lex)), "gen_version": super::types::GEN_VERSION, "boards": boards })
}

#[test]
fn a6_deterministic_boards() {
    let lex = en();
    let now = golden_now(lex);
    // Same inputs, same board bytes, twice over.
    let a = board_of(lex, Tier::Hard, 7);
    assert_eq!(format!("{a:?}"), format!("{:?}", board_of(lex, Tier::Hard, 7)));
    if std::env::var("SPZ_BLESS").is_ok() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/spelluzzle/golden-en.json");
        std::fs::write(path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
        return;
    }
    let want: serde_json::Value = serde_json::from_str(GOLDEN).expect("golden parses");
    assert_eq!(
        want["fingerprint"], now["fingerprint"],
        "the English bank or validity list changed: re-pin with SPZ_BLESS=1 cargo test a6_deterministic_boards, and say why in the commit"
    );
    assert_eq!(want["gen_version"], now["gen_version"], "GEN_VERSION changed: re-pin and explain");
    assert_eq!(want["boards"], now["boards"], "a seed no longer gives the same board: the generator changed without a GEN_VERSION bump");
}

#[test]
fn board_hash_ignores_rune_numbering() {
    let lex = en();
    let a = board_of(lex, Tier::Easy, 3);
    let mut b = a.clone();
    // Renumber the runes: swap runes 0 and 1 everywhere.
    b.rune_unit.swap(0, 1);
    for s in &mut b.slots {
        for r in &mut s.runes {
            *r = match *r { 0 => 1, 1 => 0, x => x };
        }
    }
    assert_eq!(a.hash(), b.hash());
    // Runes are a seeded permutation, not alphabet order.
    let mut sorted_every_time = true;
    for seed in 0..30 {
        let x = board_of(lex, Tier::Easy, seed);
        let mut s = x.rune_unit.clone();
        s.sort_unstable();
        if s != x.rune_unit {
            sorted_every_time = false;
        }
    }
    assert!(!sorted_every_time, "runes must not follow alphabet order (F1)");
}

// ---- A7 no_dead_end --------------------------------------------------------

#[test]
fn a7_no_dead_end() {
    let n = n_seeds(30);
    each_tier_boards(n, |tier, seed, b| {
        let last = b.slots.len() - 1;
        assert_eq!(b.slots[last].kind, SlotKind::Secret);
        let others = b.slots[..last].iter().flat_map(|s| s.runes.iter().copied()).collect::<std::collections::BTreeSet<u8>>();
        for r in &b.slots[last].runes {
            assert!(others.contains(r), "{tier:?} {seed}: secret rune {r} appears in no other word");
        }
        let shape = tier.shape();
        let kinds: Vec<SlotKind> = b.slots.iter().map(|s| s.kind).collect();
        assert_eq!(kinds.iter().filter(|k| **k == SlotKind::Spoken).count(), shape.spoken);
        assert_eq!(kinds.iter().filter(|k| **k == SlotKind::Silent).count(), shape.silent);
        assert!(b.n_runes() <= super::types::MAX_RUNES);
    });
}

// ---- A8 checker_rejects (deliberate failures) ------------------------------

/// A lexicon holding only these words, so the fixture is frozen against any
/// later bank change. `extra` joins the validity list only.
fn toy(tier: Tier, words: &[&str], extra: &[&str], collisions: &[&[&str]]) -> Lexicon {
    Lexicon::new(LexInput {
        lang: "en".into(),
        eligible: vec![(tier, words.iter().map(|s| s.to_string()).collect())],
        validity: extra.iter().map(|s| s.to_string()).collect(),
        collisions: collisions.iter().map(|g| g.iter().map(|s| s.to_string()).collect()).collect(),
    })
    .unwrap()
}

fn rejected(tier: Tier, spoken: &[&str], silent: &[&str], secret: &str, extra: &[&str], collisions: &[&[&str]]) -> GateFail {
    let all: Vec<&str> = spoken.iter().chain(silent.iter()).copied().chain(std::iter::once(secret)).collect();
    let lex = toy(tier, &all, extra, collisions);
    let b = Board::from_words_sorted("en", tier, spoken, silent, secret);
    check_board(&b, &lex).expect_err("the checker must reject this board")
}

#[test]
fn a8_checker_rejects() {
    // Two words from one collision group.
    let e = rejected(Tier::Easy, &["meat", "meet", "team", "mate", "tame"], &[], "tea", &[], &[&["meat", "meet"]]);
    assert_eq!(e.gate, "G3", "{e:?}");

    // A spoken-word rune that appears in one word only.
    let e = rejected(Tier::Easy, &["zip", "pit", "tip", "tap", "pat"], &[], "apt", &[], &[]);
    assert_eq!(e.gate, "G5", "{e:?}");
    assert!(e.detail.starts_with("zip"), "{e:?}");

    // A secret word decoded more than 60% by one word.
    let e = rejected(Tier::Easy, &["post", "spot", "tops", "pots", "stop"], &[], "opts", &[], &[]);
    assert_eq!(e.gate, "G6b", "{e:?}");

    // An unanchored silent word: its last rune is undecoded and several words fit,
    // so the board has two solutions.
    let e = rejected(
        Tier::Medium,
        &["lived", "legal", "animal", "normal", "remove"],
        &["found"],
        "ending",
        &["hound", "wound"],
        &[],
    );
    assert_eq!(e.gate, "G8", "{e:?}");
}

#[test]
fn a8_the_toy_fixtures_are_fair_without_the_defect() {
    // Control: remove the extra validity words and the same Medium board is unique.
    let words = ["lived", "legal", "animal", "normal", "remove", "found", "ending"];
    let lex = toy(Tier::Medium, &words, &[], &[]);
    let b = Board::from_words_sorted("en", Tier::Medium, &words[..5], &["found"], "ending");
    assert_eq!(count_solutions(&b, &lex, 2), 1);
}

// ---- F11 overlap -----------------------------------------------------------

#[test]
fn a9_new_board_shares_at_most_two_words_with_the_previous_one() {
    let lex = en();
    for seed in 0..10u64 {
        let prev = board_of(lex, Tier::Easy, seed).words();
        let g = generate_after(seed + 1000, Tier::Easy, lex, &prev).expect("generates");
        let shared = g.board.words().iter().filter(|w| prev.contains(w)).count();
        assert!(shared <= 2 || g.relaxed_overlap, "seed {seed}: {shared} shared");
    }
}

#[test]
fn only_english_style_units_in_the_alphabet() {
    let lex = en();
    assert!(lex.alphabet.iter().all(|c| c.is_alphabetic()));
    assert!(lex.alphabet.len() <= 64);
    let _ = pattern(&chars("abca"));
}
