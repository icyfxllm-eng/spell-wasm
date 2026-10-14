//! CC-RANDOSPELL v1 Phase A harness: A1-A4, A7-A11, A14, plus the forbidden
//! tokens law the other engines carry.

use super::bank::{eligible, length_of, Bank, CELL_FLOOR, INELIGIBLE};
use super::*;
use crate::experience::Experience::{Junior, Standard};
use std::collections::BTreeSet;

fn full_fake() -> Bank {
    let mut rows = Vec::new();
    for len in 2..=12usize {
        // Two letters give 26 x 26 distinct words; longer lengths carry a stem.
        let n = if len == 2 { 40 } else { 60 };
        for i in 0..n {
            let w = if len == 2 {
                format!("{}{}", (b'a' + (i / 26) as u8) as char, (b'a' + (i % 26) as u8) as char)
            } else {
                format!("{}{i:03}", "q".repeat(len - 3))
            };
            rows.push((i % 4, w));
        }
    }
    Bank::from_rows(rows)
}

fn session(seed: u64, bank: Bank) -> Session {
    Session::new(seed, Mode::Sprint, 1, bank, BTreeSet::new())
}

#[test]
fn a1_distribution_matches_the_two_dice_triangle() {
    // 10,000 rolls, English: pooled coverage has no remap, so faces are the target.
    let mut rng = Rng::new(0xA1);
    let mut h = [0u32; 11];
    for _ in 0..10_000 {
        h[roll_two_dice(&mut rng) as usize - 2] += 1;
    }
    let ways = [1u32, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1];
    for i in 0..11 {
        let want = ways[i] as f64 / 36.0 * 100.0;
        let got = h[i] as f64 / 100.0;
        assert!((got - want).abs() <= 2.0, "face {} got {got:.1}% want {want:.1}%", i + 2);
    }
}

#[test]
fn a2_determinism_same_seed_same_dice_and_words() {
    let run = |seed| {
        let mut s = session(seed, full_fake());
        let mut out = Vec::new();
        for i in 0..40 {
            let r = s.press().unwrap();
            out.push(r.clone());
            s.answer(i % 3 != 0);
            if s.ended.is_some() {
                break;
            }
        }
        out
    };
    assert_eq!(run(7), run(7));
    assert_ne!(run(7), run(8));
}

#[test]
fn a3_the_dice_do_not_depend_on_the_bank_or_on_reads() {
    // Same seed, different banks: the faces are identical (I1). Reading
    // state between presses (what skipping or watching the animation does)
    // changes nothing.
    let faces = |bank: Bank, peek: bool| {
        let mut s = session(99, bank);
        let mut f = Vec::new();
        for _ in 0..15 {
            if peek {
                let _ = (s.pending().cloned(), s.served().len(), s.histogram);
            }
            match s.press() {
                Some(r) => f.push(r.face),
                None => break,
            }
            s.answer(false);
        }
        f
    };
    let a = faces(full_fake(), false);
    let b = faces(full_fake(), true);
    let c = faces(Bank::from_rows((3..=12).flat_map(|l| (0..30).map(move |i| (0, format!("{}{i:03}", "z".repeat(l - 3)))))), false);
    assert_eq!(a, b);
    assert_eq!(a[..a.len().min(c.len())], c[..a.len().min(c.len())]);
}

#[test]
fn a4_every_served_word_has_the_displayed_length() {
    for lang in ["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ar", "sw", "fil"] {
        if !eligible(lang, Standard) {
            continue;
        }
        let bank = Bank::for_lang(lang, Standard);
        let cov = bank.cov.clone();
        let mut served = 0;
        for seed in 0..100u64 {
            let mut s = Session::new(seed, Mode::Sprint, (seed % 4) as usize, bank.clone(), BTreeSet::new());
            for _ in 0..10 {
                let Some(r) = s.press() else { break };
                assert_eq!(length_of(&r.word), r.len, "{lang}: {} is not {} long", r.word, r.len);
                assert_eq!(cov.length_for(r.face), Some(r.len), "{lang}: face {}", r.face);
                served += 1;
                s.answer(false);
            }
        }
        assert!(served >= 900, "{lang}: only {served} rolls served");
    }
}

#[test]
fn a7_sprint_ends_when_score_reaches_the_goal_and_records_rolls() {
    let mut s = session(3, full_fake());
    s.score = 47;
    // Find a roll of 5 by pressing with a miss until one lands: score stays 47.
    loop {
        let r = s.press().unwrap();
        if r.face == 5 {
            let o = s.answer(true).unwrap();
            assert_eq!(o.score, 52);
            assert!(o.finished);
            assert_eq!(s.ended, Some(End::Goal));
            assert!(s.rolls >= 1);
            assert!(s.press().is_none(), "no roll after the game ends");
            return;
        }
        s.answer(false);
    }
}

#[test]
fn a8_jackpot_streak_and_multiplier() {
    // The scoring law, face by face (F3, D7).
    assert_eq!(base_points(2), 4);
    assert_eq!(base_points(12), 24);
    assert_eq!(multiplied(base_points(7)), 11);
    assert_eq!(multiplied(base_points(12)), 36);
    // Through a session: two prior correct, a 12 pays 24 (multiplier not active),
    // then 7 pays 11, then 12 pays 36, then a miss resets.
    let mut s = session(1, full_fake());
    let rig = |s: &mut Session, face: u8, ok: bool| {
        s.pending = Some(Roll { face, len: 5, word: "qqqqq".into() });
        s.answer(ok).unwrap()
    };
    rig(&mut s, 3, true);
    rig(&mut s, 4, true);
    let o = rig(&mut s, 12, true);
    assert_eq!((o.points, o.multiplied, o.streak), (24, false, 3));
    let o = rig(&mut s, 7, true);
    assert_eq!((o.points, o.multiplied), (11, true));
    let o = rig(&mut s, 12, true);
    assert_eq!(o.points, 36);
    let before = s.score;
    let o = rig(&mut s, 6, false);
    assert_eq!((o.points, o.streak, s.score), (0, 0, before));
    // I7: exactly 1 after a miss.
    let o = rig(&mut s, 6, true);
    assert_eq!((o.points, o.multiplied), (6, false));
}

#[test]
fn a9_coverage_remap_is_explicit_and_nearest_longer_on_a_tie() {
    let bank = Bank::from_rows((3..=11).flat_map(|l| (0..CELL_FLOOR).map(move |i| (0, format!("{}{i:03}", "k".repeat(l - 3))))).chain(
        // length 12 holds one word fewer than the floor.
        (0..CELL_FLOOR - 1).map(|i| (0, format!("{}{i:03}", "k".repeat(9)))),
    ));
    let cov = &bank.cov;
    assert!(!cov.covered(12));
    assert_eq!(cov.length_for(12), Some(11));
    assert!(cov.remaps().contains(&(12, 11)));
    assert_eq!(cov.length_for(2), Some(3));
    // Equidistant: face 5 with 4 and 6 covered, 5 not, plays the longer.
    let b2 = Bank::from_rows([4usize, 6].iter().flat_map(|l| (0..CELL_FLOOR).map(move |i| (0, format!("{}{i:03}", "m".repeat(l - 3))))));
    assert_eq!(b2.cov.length_for(5), Some(6));
}

#[test]
fn a10_ineligible_languages_are_off_and_the_rest_cover_enough() {
    for l in INELIGIBLE {
        assert!(!eligible(l, Standard), "{l} must be off");
    }
    for l in ["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ar", "sw", "fil"] {
        assert!(eligible(l, Standard), "{l} should be eligible on the census numbers");
    }
}

#[test]
fn a11_no_word_repeats_in_a_200_roll_session() {
    for lang in ["en", "es", "ru", "vi"] {
        let bank = Bank::for_lang(lang, Standard);
        for seed in 0..5u64 {
            let mut s = Session::new(seed, Mode::Sprint, 1, bank.clone(), BTreeSet::new());
            for _ in 0..200 {
                if s.press().is_none() {
                    break;
                }
                s.answer(false); // misses keep the game going
            }
            let mut seen = BTreeSet::new();
            for w in s.served() {
                assert!(seen.insert(w.clone()), "{lang}: {w} repeated");
            }
            assert_eq!(s.served().len(), 200, "{lang}: session ended early");
        }
    }
}

#[test]
fn history_words_are_used_only_after_fresh_ones() {
    let bank = full_fake();
    let avoid: BTreeSet<String> = bank.of_len(5).iter().take(59).map(|e| e.word.clone()).collect();
    let keep = bank.of_len(5).iter().map(|e| e.word.clone()).find(|w| !avoid.contains(w)).unwrap();
    let mut s = Session::new(11, Mode::Sprint, 0, bank, avoid);
    loop {
        let r = s.press().unwrap();
        if r.len == 5 {
            assert_eq!(r.word, keep, "the only fresh length-5 word goes first");
            return;
        }
        s.answer(false);
    }
}

#[test]
fn a14_timeout_is_a_miss() {
    let mut s = session(5, full_fake());
    s.press().unwrap();
    s.streak = 4;
    let o = s.timeout().unwrap();
    assert_eq!((o.correct, o.points, o.streak), (false, 0, 0));
    assert_eq!(TIMER_SECS, 30);
}

#[test]
fn score_never_decreases_and_longest_word_is_tracked() {
    let mut s = session(21, full_fake());
    let mut last = 0;
    for i in 0..60 {
        if s.press().is_none() {
            break;
        }
        s.answer(i % 2 == 0);
        assert!(s.score >= last);
        last = s.score;
        if s.ended.is_some() {
            break;
        }
    }
    let longest = s.longest.clone().unwrap();
    let served_max = s.served().iter().map(|w| length_of(w)).max().unwrap();
    assert!(length_of(&longest) <= served_max);
    assert!(s.correct > 0);
}

#[test]
fn the_engine_source_has_no_forbidden_tokens() {
    for f in ["mod.rs", "bank.rs"] {
        let src = std::fs::read_to_string(format!("{}/src/randospell/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        for t in ["js_sys", "web_sys", "Math.random", "SystemTime", "Instant::now", "f64", "f32"] {
            assert!(!src.contains(t), "{f} must not contain {t}");
        }
    }
}

#[test]
fn a_junior_profile_never_pools_hard_or_expert_words() {
    // CC-ONBOARD-JR I5: the pool is built from allowed_tiers, so no Jr roll
    // can land on a Hard or Expert word whatever the face.
    let jr = Bank::for_lang("en", Junior);
    let std = Bank::for_lang("en", Standard);
    for len in 2..=12 {
        assert!(jr.of_len(len).iter().all(|e| e.tier <= 1), "length {len}");
    }
    assert!(jr.cov.counts.iter().sum::<usize>() < std.cov.counts.iter().sum::<usize>());
}
