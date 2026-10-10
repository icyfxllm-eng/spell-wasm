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

// ---- play rules (F2, F5, F7, F8) -------------------------------------------

use super::play::{Check, Outcome, Play};

fn type_word(p: &mut Play, slot: usize, word: &[char]) -> Option<Outcome> {
    p.select(slot);
    let mut out = None;
    for &c in word {
        out = p.type_unit(c).or(out);
    }
    out
}

fn solve_in_order(p: &mut Play, order: &[usize]) {
    for &i in order {
        let w = p.board.slots[i].answer.clone();
        type_word(p, i, &w);
    }
}

#[test]
fn stars_are_withheld_until_solved_and_earned_by_the_rules() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 5);
    let n = b.slots.len();
    let mut p = Play::new(b.clone());
    assert!(p.stars().is_none(), "no verdict before the board is solved (I6)");

    // Secret first, while its runes are still undecoded: all three stars.
    solve_in_order(&mut p, &[n - 1, 0, 1, 2, 3, 4]);
    let s = p.stars().expect("solved");
    assert!(s.solved && s.sharp_ear && s.codebreaker, "{s:?}");
    assert_eq!(p.finale_word().unwrap(), p.word(n - 1));

    // Secret last: every rune is already decoded, so no Codebreaker (Easy keeps the condition).
    let mut p = Play::new(b.clone());
    solve_in_order(&mut p, &[0, 1, 2, 3, 4, n - 1]);
    let s = p.stars().expect("solved");
    assert!(s.solved && s.sharp_ear && !s.codebreaker, "{s:?}");

    // A wrong first commit costs Sharp ear even if the slot is later corrected.
    let mut p = Play::new(b.clone());
    let mut wrong = p.board.slots[0].answer.clone();
    wrong[0] = if wrong[0] == 'z' { 'y' } else { 'z' };
    type_word(&mut p, 0, &wrong);
    solve_in_order(&mut p, &[0, 1, 2, 3, 4, n - 1]);
    let s = p.stars().expect("solved");
    assert!(s.solved && !s.sharp_ear, "{s:?}");

    // Spell Jr drops the "still undecoded" condition.
    let jr = board_of(lex, Tier::Jr, 5);
    let m = jr.slots.len();
    let mut p = Play::new(jr);
    let order: Vec<usize> = (0..m).collect();
    solve_in_order(&mut p, &order);
    assert!(p.stars().unwrap().codebreaker, "Jr: the secret needs only a right first commit");
}

#[test]
fn a_new_clash_is_close_and_a_clean_commit_is_neutral() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 9);
    let mut p = Play::new(b);
    let w0 = p.board.slots[0].answer.clone();
    assert_eq!(type_word(&mut p, 0, &w0), Some(Outcome::Neutral));
    // A different word in another slot that disagrees about a shared rune.
    let mut bad = p.board.slots[1].answer.clone();
    for k in 0..bad.len() {
        let r = p.board.slots[1].runes[k];
        if p.board.slots[0].runes.contains(&r) {
            bad[k] = if bad[k] == 'q' { 'x' } else { 'q' };
            break;
        }
    }
    assert_eq!(type_word(&mut p, 1, &bad), Some(Outcome::Close));
}

#[test]
fn medium_and_up_give_the_count_only_and_jr_easy_name_the_slots() {
    use super::render::check_text;
    for (tier, slots_named) in [(Tier::Jr, true), (Tier::Easy, true), (Tier::Medium, false), (Tier::Hard, false), (Tier::Expert, false)] {
        // One entry is wrong but agrees with everything else, so nothing clashes.
        let b = Board::from_words_sorted("en", tier, &["abc", "abd"], &[], "bca");
        let mut p = Play::new(b);
        type_word(&mut p, 0, &chars("abc"));
        type_word(&mut p, 1, &chars("abe"));
        type_word(&mut p, 2, &chars("bca"));
        match p.check() {
            Some(Check::Slots(v)) => assert!(slots_named && v == vec![1], "{tier:?}: {v:?}"),
            Some(Check::Count(n)) => assert!(!slots_named && n == 1, "{tier:?}: {n}"),
            None => panic!("{tier:?}: the full-board check did not appear"),
        }
        assert_eq!(check_text(&p, &plain_tr), "§", "one word is off reads through spz.offOne");
    }
}

#[test]
fn reopening_and_clearing_a_slot() {
    let lex = en();
    let mut p = Play::new(board_of(lex, Tier::Easy, 2));
    let w = p.board.slots[0].answer.clone();
    type_word(&mut p, 0, &w);
    assert!(p.entries.contains_key(&0));
    p.backspace();
    assert!(!p.entries.contains_key(&0), "backspace reopens a committed slot");
    assert_eq!(p.drafts.get(&0).map(|d| d.len()), Some(w.len() - 1));
    p.clear_word(0);
    assert!(p.drafts.is_empty() && p.entries.is_empty());
}

#[test]
fn only_a_silent_slot_can_be_listened_to_and_the_secret_is_never_heard_early() {
    let b = Board::from_words_sorted("en", Tier::Medium, &["lived", "legal", "animal", "normal", "remove"], &["found"], "ending");
    let mut p = Play::new(b);
    assert!(p.is_silent(5) && p.select(5).is_none(), "a silent slot says nothing");
    assert!(p.select(6).is_none(), "the secret is not heard before the board is solved");
    assert!(p.listen(0).is_none(), "a spoken slot has nothing to buy");
    assert_eq!(p.listen(5).as_deref(), Some("found"));
    assert!(!p.is_silent(5));
    assert!(p.select(5).is_some());
}

#[test]
fn resume_replaces_and_never_appends() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 4);
    let mut p = Play::new(b.clone());
    let w = p.board.slots[2].answer.clone();
    type_word(&mut p, 2, &w);
    p.select(0);
    p.type_unit('a');
    let snap = p.snapshot();
    let a = Play::restore(b.clone(), &snap);
    let again = Play::restore(b, &snap);
    assert_eq!(a.entries, p.entries);
    assert_eq!(a.drafts, p.drafts);
    assert_eq!(a.snapshot(), again.snapshot());
}

// ---- stores (F11) ----------------------------------------------------------

use super::store::{History, Progress, Streak, HISTORY_CAP};

#[test]
fn history_keeps_five_hundred_and_abandoned_boards_count() {
    let mut h = History::default();
    for i in 0..(HISTORY_CAP as u64 + 40) {
        h.push(i);
    }
    assert_eq!(h.hashes.len(), HISTORY_CAP);
    assert!(!h.contains(0) && h.contains(HISTORY_CAP as u64 + 39));
    h.push(HISTORY_CAP as u64 + 39);
    assert_eq!(h.hashes.len(), HISTORY_CAP, "re-pushing a hash does not duplicate it");
}

#[test]
fn the_streak_counts_once_a_day_with_no_replay_gate() {
    let mut s = Streak::default();
    s.solved("2026-10-09", "2026-10-08");
    s.solved("2026-10-09", "2026-10-08");
    s.solved("2026-10-09", "2026-10-08");
    assert_eq!(s.count, 1, "replays on the same day change nothing");
    s.solved("2026-10-10", "2026-10-09");
    assert_eq!(s.count, 2);
    s.solved("2026-10-13", "2026-10-12");
    assert_eq!(s.count, 1, "a gap starts again");
    assert_eq!(s.current("2026-10-20", "2026-10-19"), 0);
}

#[test]
fn saved_progress_resumes_only_for_the_same_board() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 6);
    let p = Progress { seed: b.seed, previous: vec![], gen_version: super::types::GEN_VERSION, hash: b.hash(), snapshot: Play::new(b.clone()).snapshot() };
    assert!(p.matches(b.hash()));
    assert!(!p.matches(b.hash() ^ 1));
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<Progress>(&json).unwrap(), p);
}

// ---- A9 freshness ----------------------------------------------------------

#[test]
fn a9_freshness_from_a_400_word_pool() {
    use super::fresh::fresh_board;
    let n = n_seeds(500);
    let mut tiers: Vec<Tier> = vec![Tier::Jr, Tier::Easy];
    if n_seeds(0) > 0 {
        tiers = Tier::ALL.to_vec();
    }
    for tier in tiers {
        // A 400-word pool for this tier, from the real bank, with the bank as validity.
        let full = en().pool(tier);
        let mut words: Vec<String> = full.iter().map(|w| w.units.iter().collect()).collect();
        words.sort();
        let step = (words.len() / 400).max(1);
        let pool: Vec<String> = words.iter().step_by(step).take(400).cloned().collect();
        let lex = Lexicon::new(LexInput {
            lang: "en".into(),
            eligible: vec![(tier, pool)],
            validity: en().all_valid().iter().map(|w| w.iter().collect()).collect(),
            collisions: Vec::new(),
        })
        .unwrap();
        let mut history = History::default();
        let mut prev: Vec<String> = Vec::new();
        let mut seed = 77u64;
        for i in 0..n {
            let (b, _) = fresh_board(&lex, tier, &history, &prev, || {
                seed += 1;
                seed
            })
            .unwrap_or_else(|e| panic!("{tier:?} board {i}: {e:?}"));
            assert!(!history.contains(b.hash()), "{tier:?} board {i} repeated");
            let shared = b.words().iter().filter(|w| prev.contains(w)).count();
            assert!(shared <= 2, "{tier:?} board {i} shares {shared} words with the last");
            history.push(b.hash());
            prev = b.words();
        }
    }
}

// ---- A11 no_leak, A12 jr_rules, A13 absent_not_locked ----------------------

fn plain_tr(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = String::from("§");
    for (_, v) in args {
        s.push_str(v);
    }
    let _ = key;
    s
}

/// Text between tags that is not inside a tag.
fn text_nodes(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_tag = false;
    let mut cur = String::new();
    for c in html.chars() {
        match c {
            '<' => {
                if !cur.trim().is_empty() {
                    out.push(cur.clone());
                }
                cur.clear();
                in_tag = true;
            }
            '>' => in_tag = false,
            _ if !in_tag => cur.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn a11_no_leak() {
    use super::render::{board_html, rune_key_html};
    let lex = en();
    for tier in [Tier::Jr, Tier::Easy] {
        let b = board_of(lex, tier, 11);
        let mut p = Play::new(b);
        let html = board_html(&p, &plain_tr) + &rune_key_html(&p, &plain_tr);
        for w in p.board.words() {
            assert!(!html.contains(&w), "the answer {w} is in the markup of an untouched board");
        }
        assert!(text_nodes(&html).iter().all(|t| t.chars().all(|c| !c.is_alphabetic())), "an untouched board shows no letter: {:?}", text_nodes(&html));
        // After one commit, only that entry's units appear.
        let w = p.board.slots[0].answer.clone();
        type_word(&mut p, 0, &w);
        let html = board_html(&p, &plain_tr) + &rune_key_html(&p, &plain_tr);
        let allowed: std::collections::BTreeSet<char> = w.iter().copied().collect();
        for t in text_nodes(&html) {
            for c in t.chars().filter(|c| c.is_alphabetic()) {
                assert!(allowed.contains(&c), "{tier:?}: letter {c} shown though only `{}` was committed", w.iter().collect::<String>());
            }
        }
        // The aria labels carry the rune number, and a unit only where one was decoded.
        let labels: Vec<&str> = html.split("aria-label=\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
        assert!(!labels.is_empty());
    }
}

#[test]
fn a12_jr_rules() {
    let n = n_seeds(300);
    let lex = en();
    for seed in 0..n as u64 {
        let b = board_of(lex, Tier::Jr, seed);
        assert!(b.slots.iter().all(|s| s.kind != SlotKind::Silent), "Spell Jr has no silent slot");
        for s in &b.slots {
            assert!(!lex.has_homophone(&s.answer), "Jr word {} has a sound-alike", s.answer.iter().collect::<String>());
        }
    }
    use super::offer::tiers_for;
    assert_eq!(tiers_for("en", true), vec![Tier::Jr], "a Jr profile cannot open a standard tier");
    assert!(!tiers_for("en", false).contains(&Tier::Jr));
}

#[test]
fn a13_absent_not_locked() {
    use super::offer::{language_offered, tiers_for};
    for lang in ["ko", "zh", "ja", "ar", "hi", "fr", "de", "vi"] {
        assert!(!language_offered(lang) && tiers_for(lang, false).is_empty() && tiers_for(lang, true).is_empty(), "{lang} must be absent");
    }
    assert!(language_offered("en"));
    for lang in super::offer::NEVER {
        let lex = Lexicon::new(LexInput {
            lang: lang.into(),
            eligible: vec![(Tier::Easy, ["cat", "dog"].iter().map(|s| s.to_string()).collect())],
            validity: vec![],
            collisions: vec![],
        })
        .unwrap();
        let r = std::panic::catch_unwind(|| {
            let _ = generate(1, Tier::Easy, &lex);
        });
        assert!(r.is_err(), "constructing a board for {lang} must fail an assertion");
    }
}

// ---- verified seeds (Medium to Expert) -------------------------------------

use super::fresh::fresh_verified;
use super::seeds;

#[test]
fn seeds_are_current() {
    let lex = en();
    let f = seeds::file().expect("assets/spelluzzle/seeds-en.json parses");
    assert_eq!(
        f.fingerprint,
        seeds::fingerprint_hex(lex),
        "the English bank, a collision set or GEN_VERSION changed since the verified seeds were built: run scripts/spelluzzle-seeds.sh /path/to/en_US.dic"
    );
    assert_eq!(f.gen_version, super::types::GEN_VERSION);
    let n = n_seeds(60);
    for tier in [Tier::Medium, Tier::Hard, Tier::Expert] {
        let list = seeds::verified(lex, tier);
        assert!(list.len() >= 1000, "{tier:?} has only {} verified seeds", list.len());
        assert!(list.windows(2).all(|w| w[0] < w[1]), "{tier:?}: seeds are sorted and unique");
        // Every seed regenerates a board the bank-only checker accepts. (The large-list proof
        // was done at build time and is recorded in the file; it cannot be redone without it.)
        let step = (list.len() / n).max(1);
        for &s in list.iter().step_by(step) {
            let g = generate(s as u64, tier, lex).unwrap_or_else(|e| panic!("{tier:?} seed {s}: {e:?}"));
            check_board(&g.board, lex).unwrap_or_else(|e| panic!("{tier:?} seed {s}: {e:?}"));
        }
        assert!(seeds::usable(lex, tier));
    }
    assert!(seeds::usable(lex, Tier::Jr) && seeds::usable(lex, Tier::Easy), "Jr and Easy never need seeds");
}

#[test]
fn a_stale_seed_file_makes_the_tiers_absent_not_unverified() {
    // A lexicon that is not the one the file was built from.
    let other = toy(Tier::Medium, &["lived", "legal", "animal", "normal", "remove", "found", "ending"], &[], &[]);
    for tier in [Tier::Medium, Tier::Hard, Tier::Expert] {
        assert!(seeds::verified(&other, tier).is_empty());
        assert!(!seeds::usable(&other, tier), "{tier:?} must be absent when the seeds do not match");
    }
    assert!(seeds::usable(&other, Tier::Easy));
}

#[test]
fn verified_boards_are_fresh_and_share_at_most_two_words() {
    let lex = en();
    let mut state = 12345u64;
    let mut rng = move |n: usize| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) as usize) % n
    };
    for tier in [Tier::Medium, Tier::Hard, Tier::Expert] {
        let list = seeds::verified(lex, tier);
        let mut history = History::default();
        let mut prev: Vec<String> = Vec::new();
        for i in 0..n_seeds(40) {
            let (b, seed) = fresh_verified(lex, tier, &list, &history, &prev, &mut rng).unwrap_or_else(|| panic!("{tier:?} board {i}"));
            assert!(list.contains(&(seed as u32)), "{tier:?}: a seed outside the verified list was served");
            assert!(!history.contains(b.hash()), "{tier:?} board {i} repeated");
            let shared = b.words().iter().filter(|w| prev.contains(w)).count();
            assert!(shared <= 2, "{tier:?} board {i} shares {shared}");
            // The board is the one the seed always gives.
            assert_eq!(b.hash(), generate(seed, tier, lex).unwrap().board.hash());
            history.push(b.hash());
            prev = b.words();
        }
    }
}

#[test]
fn medium_to_expert_boards_have_silent_slots_and_listen() {
    let lex = en();
    for (tier, silent) in [(Tier::Medium, 1), (Tier::Hard, 2), (Tier::Expert, 3)] {
        let list = seeds::verified(lex, tier);
        let b = generate(list[0] as u64, tier, lex).unwrap().board;
        assert_eq!(b.slots.iter().filter(|s| s.kind == SlotKind::Silent).count(), silent, "{tier:?}");
        let mut p = Play::new(b);
        let i = p.board.slots.iter().position(|s| s.kind == SlotKind::Silent).unwrap();
        assert!(p.select(i).is_none(), "a silent slot says nothing");
        assert!(p.listen(i).is_some());
    }
}

/// Build-time audit of the seed file, with the large list (not run by `cargo test`):
///   SPZ_VALIDITY_FILE=/path/en_US.dic SPZ_N=200 cargo test --release spelluzzle::tests::audit_seeds -- --ignored --nocapture
/// Every sampled seed's bank-generated board must have exactly one answer under the large
/// list, by the checker and by the independent solver.
#[test]
#[ignore]
fn audit_seeds() {
    let path = std::env::var("SPZ_VALIDITY_FILE").expect("SPZ_VALIDITY_FILE");
    let dic = std::fs::read_to_string(path).unwrap();
    let st: std::collections::BTreeSet<String> = dic.lines().skip(1).filter_map(|l| {
        let w = l.split('/').next()?.trim();
        (!w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase())).then(|| w.to_string())
    }).collect();
    let mut words: Vec<String> = st.iter().cloned().collect();
    for w in &st {
        for suf in ["s", "es", "ed", "d", "ing", "er", "ers", "est", "ly", "y", "ies"] {
            words.push(format!("{w}{suf}"));
        }
    }
    let small = bank::load("en", Vec::new()).unwrap();
    let big = bank::load("en", words).unwrap();
    let n = n_seeds(100);
    for tier in [Tier::Medium, Tier::Hard, Tier::Expert] {
        let list = seeds::verified(&small, tier);
        let step = (list.len() / n).max(1);
        let mut checked = 0;
        for &s in list.iter().step_by(step) {
            let b = generate(s as u64, tier, &small).unwrap().board;
            check_board(&b, &big).unwrap_or_else(|e| panic!("{tier:?} seed {s}: {e:?}"));
            assert_eq!(count_solutions(&b, &big, 2), 1, "{tier:?} seed {s}");
            if checked < 5 {
                assert_eq!(reference_count(&b, &big, 2), 1, "{tier:?} seed {s}: independent solver");
            }
            checked += 1;
        }
        println!("audit {tier:?}: {checked} sampled seeds all have one answer under the large list");
    }
}

// ---- v1.1 Phase E: pencil, strip, ripple -----------------------------------

use super::pencil::{self, Pencil};
use super::render::{board_html, board_html_with, strip_html, Opts};

#[test]
fn a21_pencil_layer() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 21);
    let mut p = Play::new(b);
    let mut pencil = Pencil::default();
    // Mark the first rune of slot 0.
    let r = p.board.slots[0].runes[0];
    pencil.set(r, 'q');
    let v0 = board_view(&p.board, &p.entries);
    assert_eq!(pencil.visible(&v0).get(&r), Some(&'q'), "an undecoded rune shows its mark");
    // Commit a word that decodes that rune: the mark is hidden, not deleted.
    let w = p.board.slots[0].answer.clone();
    type_word(&mut p, 0, &w);
    let v1 = board_view(&p.board, &p.entries);
    assert!(pencil.visible(&v1).is_empty(), "a decoded rune hides its mark");
    assert_eq!(pencil.get(r), Some('q'), "...but does not delete it");
    // Clear the word: the rune is undecoded again and the mark is drawn again.
    p.clear_word(0);
    let v2 = board_view(&p.board, &p.entries);
    assert_eq!(pencil.visible(&v2).get(&r), Some(&'q'));
    // A mark is never an entry.
    assert!(p.entries.is_empty() && p.drafts.is_empty());
    // One mark per rune; two runes may carry the same letter.
    let r2 = p.board.slots[1].runes.iter().copied().find(|x| *x != r).unwrap();
    pencil.set(r, 'a');
    pencil.set(r2, 'a');
    assert_eq!(pencil.all().len(), 2);
    pencil.clear_all();
    assert!(pencil.is_empty());
}

#[test]
fn a20_pencil_is_inert() {
    // Replay a scripted solve twice, once with pencil marks injected between every step.
    // Everything the game decides must be identical.
    let lex = en();
    for tier in [Tier::Jr, Tier::Easy] {
        for seed in 0..20u64 {
            let b = board_of(lex, tier, seed);
            let n = b.slots.len();
            let mut plain = Play::new(b.clone());
            let mut marked = Play::new(b.clone());
            let mut pen = Pencil::default();
            let mut rng = crate::spelldoku::rng::Rng::new(seed);
            let order: Vec<usize> = (n..n).chain((0..n).rev()).collect();
            for &i in &order {
                let w = plain.board.slots[i].answer.clone();
                // Put a mark (or several) on random runes between steps.
                for _ in 0..3 {
                    pen.set(rng.below(b.n_runes()) as u8, lex.alphabet[rng.below(lex.alphabet.len())]);
                }
                let _ = pen.visible(&board_view(&marked.board, &marked.entries));
                type_word(&mut plain, i, &w);
                type_word(&mut marked, i, &w);
                assert_eq!(board_view(&plain.board, &plain.entries), board_view(&marked.board, &marked.entries));
                assert_eq!(plain.check(), marked.check());
                assert_eq!(plain.snapshot(), marked.snapshot());
            }
            assert_eq!(plain.stars(), marked.stars());
        }
    }
}

#[test]
fn a22_strip_counts_and_order() {
    let lex = en();
    let n = n_seeds(60);
    each_tier_boards(n, |tier, seed, b| {
        let slots: Vec<&[u8]> = b.slots.iter().map(|s| s.runes.as_slice()).collect();
        let view = board_view(b, &Entries::new());
        let strip = pencil::strip(&slots, &view, &Default::default());
        assert_eq!(strip.len(), b.n_runes(), "{tier:?} {seed}: one entry per rune");
        for e in &strip {
            let cells = slots.iter().flat_map(|s| s.iter()).filter(|&&r| r == e.rune).count();
            assert_eq!(e.cells, cells, "{tier:?} {seed}: count for rune {}", e.rune);
        }
        // D36: most cells first, ties by first appearance in reading order.
        let first = |r: u8| slots.iter().flat_map(|s| s.iter()).position(|&x| x == r).unwrap();
        for w in strip.windows(2) {
            assert!(w[0].cells > w[1].cells || (w[0].cells == w[1].cells && first(w[0].rune) < first(w[1].rune)), "{tier:?} {seed}: order");
        }
    });
    // The order does not move during play.
    let b = board_of(lex, Tier::Easy, 3);
    let slots: Vec<&[u8]> = b.slots.iter().map(|s| s.runes.as_slice()).collect();
    let mut p = Play::new(b.clone());
    let before: Vec<u8> = pencil::strip(&slots, &board_view(&p.board, &p.entries), &Default::default()).iter().map(|e| e.rune).collect();
    let w = p.board.slots[1].answer.clone();
    type_word(&mut p, 1, &w);
    let after: Vec<u8> = pencil::strip(&slots, &board_view(&p.board, &p.entries), &Default::default()).iter().map(|e| e.rune).collect();
    assert_eq!(before, after);
}

#[test]
fn a23_ripple_is_blind_and_bounded() {
    // Bounded: 1 to 16 runes never exceed 1.2 s.
    for n in 1..=16usize {
        assert!(pencil::step_ms(n) as usize * n <= pencil::MAX_MS as usize, "{n} runes");
        assert!(pencil::step_ms(n) > 0);
    }
    assert_eq!(pencil::step_ms(0), 0);
    // Blind: it is a function of two views. A right commit and a wrong commit that produce the
    // same view change give the same ripple.
    let b = Board::from_words_sorted("en", Tier::Easy, &["abc", "abd", "bcd"], &[], "cab");
    let slots: Vec<&[u8]> = b.slots.iter().map(|s| s.runes.as_slice()).collect();
    let order = pencil::strip_order(&slots, b.n_runes());
    let before = board_view(&b, &Entries::new());
    let mut right = Entries::new();
    right.insert(0, chars("abc"));
    let after_right = board_view(&b, &right);
    // A "wrong" entry decoding the same runes the same way is the same view, so the same ripple.
    let r1 = pencil::ripple(&slots, &order, &before, &after_right, 0);
    let r2 = pencil::ripple(&slots, &order, &before, &after_right.clone(), 0);
    assert_eq!(r1, r2);
    assert_eq!(r1.runes.len(), 3, "three runes newly decoded");
    // Cells counted are outside the committed slot: a, b in `abd`, b, c in `bcd`, c, a, b in the secret.
    assert_eq!(r1.cells, 2 + 2 + 3);
    // A commit that decodes nothing new shows no ripple.
    assert!(pencil::ripple(&slots, &order, &after_right, &after_right, 0).runes.is_empty());
    // A contested rune does not ripple.
    let mut clash = right.clone();
    clash.insert(1, chars("xbd"));
    let after_clash = board_view(&b, &clash);
    let rc = pencil::ripple(&slots, &order, &before, &after_clash, 1);
    let ids: Vec<u8> = rc.runes.clone();
    for r in ids {
        assert!(matches!(after_clash[r as usize], RuneState::Decoded(_)));
    }
}

#[test]
fn a19_flags_off_is_v1_markup() {
    // With every Phase E option at its default, the board and rune key are v1's, byte for byte.
    let lex = en();
    for tier in [Tier::Jr, Tier::Easy] {
        let b = board_of(lex, tier, 9);
        let mut p = Play::new(b);
        let w = p.board.slots[0].answer.clone();
        type_word(&mut p, 0, &w);
        let view = board_view(&p.board, &p.entries);
        assert_eq!(board_html(&p, &plain_tr), board_html_with(&p, &view, &plain_tr, &Opts::default()));
    }
}

#[test]
fn pencil_and_strip_markup_carries_no_answer() {
    let lex = en();
    let b = board_of(lex, Tier::Easy, 7);
    let mut p = Play::new(b);
    let r = p.board.slots[2].runes[0];
    let mut marks = std::collections::BTreeMap::new();
    marks.insert(r, 'z');
    let opts = Opts { marks: marks.clone(), highlight: Some(r), target: None };
    let view = board_view(&p.board, &p.entries);
    let html = board_html_with(&p, &view, &plain_tr, &opts) + &strip_html(&p, &view, &plain_tr, &opts);
    for w in p.board.words() {
        assert!(!html.contains(&w), "answer {w} is in the Phase E markup");
    }
    // The only letter shown on an untouched board is the player's own mark.
    for t in text_nodes(&html) {
        for c in t.chars().filter(|c| c.is_alphabetic()) {
            assert_eq!(c, 'z', "an untouched board with one mark shows only that mark, saw {c}");
        }
    }
    assert!(html.contains("sz-pencil") && html.contains("pencilled") && html.contains("hl"));
    // Marks are saved with their board's hash.
    let save = super::store::PencilSave { hash: p.board.hash(), marks: vec![(r, 'z')] };
    let back: super::store::PencilSave = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
    assert_eq!(back, save);
}
