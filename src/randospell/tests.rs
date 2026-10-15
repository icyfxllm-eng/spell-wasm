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
        s.pending = Some(Roll { face, len: face as usize, word: "qqqqq".into(), tier: 0, ..Roll::default() });
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

fn marathon(seed: u64) -> Session {
    Session::new(seed, Mode::Marathon, 1, full_fake(), BTreeSet::new())
}

/// Put a roll in front of the player without going through the dice.
fn rig(s: &mut Session, len: usize, ok: bool) -> Outcome {
    s.pending = Some(Roll { face: len as u8, len, word: "qqqqq".into(), tier: 0, ..Roll::default() });
    s.answer(ok).unwrap()
}

#[test]
fn a5_marathon_miss_empties_the_pot_but_never_the_score() {
    let mut s = marathon(1);
    s.ward = false; // no Ward in hand
    s.score = 40;
    s.pot = 23;
    s.streak = 2;
    let o = rig(&mut s, 6, false);
    assert_eq!((s.score, s.pot, s.streak), (40, 0, 0));
    assert_eq!((o.lost, o.warded), (23, false));
    assert_eq!(s.largest_lost, 23);
}

#[test]
fn a5_the_ward_absorbs_one_pot_loss_and_the_streak_still_resets() {
    let mut s = marathon(2);
    s.score = 40;
    s.pot = 23;
    s.streak = 2;
    let o = rig(&mut s, 6, false);
    assert_eq!((s.score, s.pot, s.streak), (40, 23, 0));
    assert!(o.warded && !s.ward && s.ward_used);
    assert_eq!(o.lost, 0);
    // The second loss is not covered: one Ward per game.
    let o = rig(&mut s, 6, false);
    assert_eq!((s.pot, o.lost, o.warded), (0, 23, false));
}

#[test]
fn the_ward_is_not_spent_when_nothing_is_at_risk() {
    let mut s = marathon(3);
    let o = rig(&mut s, 6, false);
    assert!(!o.warded && s.ward && !s.ward_used, "an empty pot loses nothing, so the Ward stays");
}

#[test]
fn a6_marathon_ends_only_on_a_bank_that_reaches_the_goal() {
    let mut s = marathon(4);
    s.score = 95;
    s.pot = 3;
    let o = rig(&mut s, 5, true); // pot 8, still waiting on the decision
    assert_eq!((o.pot, o.finished, o.deciding), (8, false, true));
    assert!(s.press().is_none(), "no roll until Bank or Roll again");
    // Roll again: the game continues, the pot stays at risk even above the goal.
    assert!(s.roll_again());
    assert_eq!((s.score, s.pot, s.ended), (95, 8, None));
    let o = rig(&mut s, 12, true); // +24: pot 32, score still 95
    assert_eq!((o.pot, o.finished, s.ended), (32, false, None));
    let b = s.bank().unwrap();
    assert_eq!((b.banked, b.score, b.finished), (32, 127, true));
    assert_eq!(s.ended, Some(End::Goal));
    assert_eq!(s.largest_banked, 32);
}

#[test]
fn a6_bank_103() {
    let mut s = marathon(5);
    s.score = 95;
    s.pot = 3;
    rig(&mut s, 5, true);
    let b = s.bank().unwrap();
    assert_eq!((b.score, b.finished), (103, true));
}

#[test]
fn bank_and_roll_again_need_a_decision_and_sprint_has_neither() {
    let mut m = marathon(6);
    assert!(m.bank().is_none() && !m.roll_again());
    let mut s = session(6, full_fake());
    rig(&mut s, 5, true);
    assert!(s.bank().is_none() && !s.roll_again());
    assert_eq!((s.score, s.pot), (5, 0), "Sprint scores straight to the score");
}

#[test]
fn marathon_correct_answers_add_to_the_pot_with_the_streak_multiplier() {
    let mut s = marathon(7);
    rig(&mut s, 3, true);
    s.roll_again();
    rig(&mut s, 4, true);
    s.roll_again();
    let o = rig(&mut s, 12, true);
    assert_eq!((o.points, s.pot, s.score), (24, 3 + 4 + 24, 0));
    s.roll_again();
    let o = rig(&mut s, 7, true);
    assert_eq!((o.points, o.multiplied), (11, true));
    assert_eq!(s.score, 0, "nothing is banked until the player banks");
}

#[test]
fn marathon_score_never_decreases_under_random_play() {
    for seed in 0..20u64 {
        let mut s = marathon(seed);
        let mut last = 0;
        for i in 0..80u32 {
            if s.press().is_none() {
                break;
            }
            s.answer((i + seed as u32) % 3 != 0);
            if s.deciding {
                if i % 2 == 0 { s.bank(); } else { s.roll_again(); }
            }
            assert!(s.score >= last, "score fell");
            last = s.score;
            if s.ended.is_some() {
                break;
            }
        }
    }
}

// ---- Phase C -------------------------------------------------------------

use super::daily;
use super::store::Store;

fn es_bank() -> Bank {
    Bank::for_lang("es", Standard)
}

#[test]
fn a2_daily_is_identical_for_everyone_whatever_they_answer() {
    // (2026-10-14, es, Hard = tier 2): the dice and the words match on two
    // "devices" even when the players answer completely differently.
    let play = |pattern: u32| {
        let mut s = Session::new_daily("2026-10-14", "es", 2, es_bank()).expect("es is eligible");
        let mut seq = Vec::new();
        for i in 0..40u32 {
            let Some(r) = s.press() else { break };
            seq.push((r.face, r.len, r.word));
            s.answer((i * pattern) % 3 != 0);
            if s.deciding {
                if i % 2 == 0 { s.bank(); } else { s.roll_again(); }
            }
            if s.ended.is_some() {
                break;
            }
        }
        seq
    };
    let a = play(1);
    let b = play(2);
    let c = play(5);
    let n = a.len().min(b.len()).min(c.len());
    assert!(n >= 20, "too short to prove anything: {n}");
    assert_eq!(a[..n], b[..n]);
    assert_eq!(a[..n], c[..n]);
    // A different day, language or tier is a different game.
    let other = |d: &str, l: &str, t: usize| {
        let mut s = Session::new_daily(d, l, t, Bank::for_lang(l, Standard)).unwrap();
        (0..8).filter_map(|_| { let r = s.press(); s.answer(false); r.map(|r| r.word) }).collect::<Vec<_>>()
    };
    let base = other("2026-10-14", "es", 2);
    assert_ne!(base, other("2026-10-15", "es", 2));
    assert_ne!(base, other("2026-10-14", "es", 1));
    assert_ne!(base, other("2026-10-14", "fr", 2));
}

#[test]
fn i6_no_daily_seed_exists_for_an_ineligible_language() {
    for l in INELIGIBLE {
        let bank = Bank::for_lang(l, Standard);
        assert!(daily::daily_seed("2026-10-14", l, 1, &bank).is_none(), "{l}");
        assert!(Session::new_daily("2026-10-14", l, 1, bank).is_none(), "{l}");
    }
    assert!(daily::daily_seed("2026-10-14", "en", 1, &Bank::for_lang("en", Standard)).is_some());
}

#[test]
fn f6_one_attempt_per_day_per_language_per_tier() {
    let mut st = Store::default();
    let k = daily::key("2026-10-14", "es", 2);
    assert!(st.try_start_daily(&k));
    assert!(!st.try_start_daily(&k), "a second attempt on the same key");
    assert!(st.try_start_daily(&daily::key("2026-10-14", "es", 1)), "another tier is another attempt");
    assert!(st.try_start_daily(&daily::key("2026-10-14", "fr", 2)), "another language too");
    // An abandoned attempt is still spent, and a finished one is written once.
    let mut s = Session::new_daily("2026-10-14", "es", 2, es_bank()).unwrap();
    s.press();
    s.answer(true);
    st.finish_daily(&k, &s, true);
    let first = st.daily_entry(&k).cloned().unwrap();
    assert!(first.done && first.score == s.score);
    s.score += 50;
    st.finish_daily(&k, &s, true);
    assert_eq!(st.daily_entry(&k), Some(&first), "a result is written once");
}

#[test]
fn the_store_round_trips_and_keeps_bests() {
    let mut st = Store::default();
    let mut s = Session::new(1, Mode::Sprint, 1, full_fake(), BTreeSet::new());
    s.score = 49;
    s.press();
    s.answer(true);
    assert_eq!(s.ended, Some(End::Goal));
    st.record_game(&s);
    assert_eq!(st.best_sprint_rolls, Some(1));
    assert_eq!(st.histogram.iter().sum::<u32>(), 1);
    let json = serde_json::to_string(&st).unwrap();
    let back: Store = serde_json::from_str(&json).unwrap();
    assert_eq!(back, st);
}

fn h2h() -> Session {
    Session::new(9, Mode::HeadToHead, 1, full_fake(), BTreeSet::new())
}

#[test]
fn a13_a_miss_gives_the_other_player_exactly_one_bonus_roll() {
    let mut s = h2h();
    assert_eq!(s.turn, 0);
    let o = rig(&mut s, 6, false);
    assert!(o.passed, "a miss ends the turn");
    assert_eq!(s.turn, 1);
    assert!(s.bonus_of(1) && s.bonus, "the next player holds one bonus roll");
    // Their first roll is the bonus roll, the second is an ordinary roll.
    let r1 = s.press().expect("roll");
    assert!(s.bonus_roll && !s.bonus, "the bonus is spent on the first press");
    let o1 = s.answer(false).unwrap();
    assert!(o1.bonus_roll && !o1.passed, "a missed bonus roll costs nothing and keeps the turn");
    assert!(!s.bonus_of(0), "a missed bonus roll grants the other player nothing");
    let _ = r1;
    s.press().unwrap();
    assert!(!s.bonus_roll, "the second roll is not a bonus roll");
    let o2 = s.answer(false).unwrap();
    assert!(o2.passed && s.turn == 0 && s.bonus, "an ordinary miss passes the turn and grants a bonus");
}

#[test]
fn h2h_a_bonus_roll_is_banked_at_once_and_a_win_needs_a_bank() {
    let mut s = h2h();
    s.other.bonus = true; // player 1 is not on the move; give player 0 a bonus via a pass
    let _ = rig(&mut s, 5, false); // player 0 misses -> player 1 on the move with a bonus
    assert_eq!(s.turn, 1);
    s.score = 90;
    s.press().unwrap();
    assert!(s.bonus_roll);
    s.pending.as_mut().unwrap().len = 5;
    let o = s.answer(true).unwrap();
    assert_eq!((o.score, o.pot, o.deciding), (95, 0, false), "banked straight away, no pot");
    // An ordinary roll worth 8 sits in the pot, and only the Bank wins (D15).
    let o = rig(&mut s, 8, true);
    assert_eq!((o.pot, o.finished, s.ended), (8, false, None));
    let b = s.bank().unwrap();
    assert!((b.score, b.finished, b.passed) == (103, true, false));
    assert_eq!(s.winner, Some(1));
    assert_eq!(s.score_of(1), 103);
}

#[test]
fn h2h_a_bank_passes_the_turn_and_each_player_keeps_their_own_ward() {
    let mut s = h2h();
    let o = rig(&mut s, 6, true);
    assert!(o.deciding);
    let b = s.bank().unwrap();
    assert!(b.passed && b.score == 6 && s.turn == 1, "{b:?}");
    assert_eq!((s.score, s.score_of(0)), (0, 6));
    // Player 1 spends THEIR Ward; player 0's is untouched.
    s.pot = 10;
    let o = rig(&mut s, 6, false);
    assert!(o.warded);
    assert!(s.ward_of(0), "player 0 still holds a Ward");
    assert!(!s.ward_of(1));
}

#[test]
fn h2h_score_never_falls_for_either_player() {
    for seed in 0..12u64 {
        let mut s = Session::new(seed, Mode::HeadToHead, 1, full_fake(), BTreeSet::new());
        let mut last = [0u32; 2];
        for i in 0..120u32 {
            if s.press().is_none() {
                break;
            }
            s.answer((i + seed as u32) % 4 != 0);
            if s.deciding {
                if i % 3 == 0 { s.bank(); } else { s.roll_again(); }
            }
            for p in 0..2 {
                assert!(s.score_of(p) >= last[p], "player {p} lost banked score");
                last[p] = s.score_of(p);
            }
            if s.ended.is_some() {
                break;
            }
        }
    }
}

// ---- Phase D: the Spell Jr variant ---------------------------------------

use super::bank::{jr_eligible, JR_HI, JR_LO};

fn jr_bank(lang: &str) -> Bank {
    Bank::for_lang(lang, Junior)
}

fn jr(seed: u64, bank: Bank) -> Session {
    Session::new(seed, Mode::Jr, 0, bank, BTreeSet::new())
}

#[test]
fn a12_jr_die_plays_lengths_two_to_seven_only() {
    for lang in ["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ar"] {
        assert!(jr_eligible(lang), "{lang} should be Jr eligible on the Phase D census");
        let mut s = jr(3, jr_bank(lang));
        for _ in 0..120 {
            let Some(r) = s.press() else { break };
            assert!((1..=6).contains(&r.face), "{lang}: die face {}", r.face);
            assert!((JR_LO..=JR_HI).contains(&r.len), "{lang}: length {} outside 2-7", r.len);
            assert_eq!(length_of(&r.word), r.len);
            s.answer(true);
            if s.ended.is_some() {
                break;
            }
        }
    }
}

#[test]
fn jr_sw_and_fil_stay_off_and_the_ineligible_stay_off() {
    for l in ["sw", "fil", "ko", "hi", "ja", "zh"] {
        assert!(!jr_eligible(l), "{l} must be off for Jr");
    }
}

#[test]
fn jr_remaps_are_listed_in_die_faces_and_never_leave_the_die() {
    // en has no length-7 word at the Jr tiers (17), so face 6 plays as face 5.
    let cov = jr_bank("en").cov;
    assert!(!cov.covered(7));
    assert_eq!(cov.jr_remaps(), vec![(6, 5)]);
    // ru covers all six lengths, so nothing is remapped.
    assert!(jr_bank("ru").cov.jr_remaps().is_empty());
    // es has almost no two-letter words: face 1 plays as face 2 (length 3).
    assert_eq!(jr_bank("es").cov.jr_remaps(), vec![(1, 2)]);
    // A closer length outside 2-7 is never chosen (en 8 is covered, 7 is not).
    assert_eq!(cov.length_within(7, JR_LO, JR_HI), Some(6));
}

#[test]
fn jr_scores_the_die_with_no_multiplier_and_no_pot() {
    let mut s = jr(1, full_fake());
    for _ in 0..4 {
        s.pending = Some(Roll { face: 6, len: 7, word: "qqqqqqq".into(), tier: 0, ..Roll::default() });
        let o = s.answer(true).unwrap(); // a die showing 6
        assert_eq!((o.points, o.multiplied, o.pot, o.deciding, o.warded), (6, false, 0, false, false));
    }
    assert_eq!(s.score, 24);
    assert_eq!(s.streak, 4, "the streak is tracked silently");
    assert_eq!(s.best_streak, 4);
    assert!(!s.ward, "Jr has no Ward");
}

#[test]
fn jr_a_miss_replays_the_word_and_allows_one_more_try() {
    let mut s = jr(2, full_fake());
    let r = s.press().unwrap();
    s.streak = 3;
    let first = s.answer(false).unwrap();
    assert!(first.retry && !first.correct, "first miss is a retry");
    assert_eq!((s.score, s.streak), (0, 3), "nothing changes on the first miss");
    assert_eq!(s.pending().map(|p| p.word.clone()), Some(r.word.clone()), "same word, replayed");
    let second = s.answer(false).unwrap();
    assert!(!second.retry && second.points == 0);
    assert_eq!(s.streak, 0, "the second miss moves on and resets the streak");
    assert!(s.pending().is_none());
    // A right answer on the retry scores in full.
    let r = s.press().unwrap();
    s.answer(false);
    let o = s.answer(true).unwrap();
    assert_eq!(o.points as usize, r.len - 1);
}

#[test]
fn the_jr_pool_holds_only_easy_and_medium_words() {
    let b = jr_bank("en");
    for len in 2..=12 {
        assert!(b.of_len(len).iter().all(|e| e.tier <= 1), "length {len}");
    }
}

#[test]
fn only_jr_gets_a_second_try() {
    let mut s = session(4, full_fake());
    s.press().unwrap();
    let o = s.answer(false).unwrap();
    assert!(!o.retry, "Sprint has one attempt per word (D18)");
}

// ---- A1 per language, against the REMAPPED target --------------------------

#[test]
fn a1_every_eligible_language_matches_its_remapped_triangle() {
    // Spec A1: "faces remapped by D4 are tested against their remapped target".
    // For each language, 10,000 two-dice sums are mapped through the language's
    // own coverage; the observed share of each PLAYED length must sit within
    // 2 pp of the exact share the triangle gives it after remapping.
    let ways = [1u32, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1]; // sums 2..=12 out of 36
    for lang in ["en", "es", "fr", "de", "pt", "pl", "ru", "vi", "ar", "sw", "fil"] {
        let cov = Bank::for_lang(lang, Standard).cov;
        let mut want = [0f64; 11];
        for face in 2..=12u8 {
            let l = cov.length_for(face).expect("a viable language covers something");
            want[l - 2] += ways[face as usize - 2] as f64 / 36.0 * 100.0;
        }
        let mut rng = Rng::new(0xA1A1 ^ lang.len() as u64);
        let mut seen = [0u32; 11];
        for _ in 0..10_000 {
            let face = roll_two_dice(&mut rng);
            seen[cov.length_for(face).unwrap() - 2] += 1;
        }
        for i in 0..11 {
            let got = seen[i] as f64 / 100.0;
            assert!((got - want[i]).abs() <= 2.0, "{lang}: length {} got {got:.1}% want {:.1}%", i + 2, want[i]);
            if want[i] == 0.0 {
                assert_eq!(seen[i], 0, "{lang}: length {} is played but should not be", i + 2);
            }
        }
    }
}

// ---- D10: what the player owns --------------------------------------------

use crate::entitlements::AccessLevel::{Full, None as NoAccess, Preview};

#[test]
fn d10_preview_pools_only_the_first_tier_and_full_pools_all() {
    let full = Bank::for_lang_at("es", Standard, Full);
    let prev = Bank::for_lang_at("es", Standard, Preview);
    for len in 2..=12 {
        assert!(prev.of_len(len).iter().all(|e| e.tier == 0), "length {len}: a Preview bank holds a paid tier");
    }
    assert!(prev.cov.counts.iter().sum::<usize>() < full.cov.counts.iter().sum::<usize>());
    assert!(Bank::for_lang_at("es", Standard, NoAccess).cov.counts.iter().all(|n| *n == 0));
    assert_eq!(Bank::for_lang("es", Standard).cov, full.cov, "for_lang is the Full bank");
}

#[test]
fn d10_a_preview_bank_that_cannot_carry_the_die_is_absent_not_padded() {
    // es Easy alone covers five lengths; the floor is six, so a Preview Spanish
    // player does not get the mode (and nothing is padded to make it appear).
    assert!(!super::bank::eligible_at("es", Standard, Preview));
    assert!(super::bank::eligible_at("es", Standard, Full));
    // English is Full in the free tier, so it is simply free.
    assert!(super::bank::eligible_at("en", Standard, Full));
    // Jr is free (D10): a Jr bank does not depend on the level.
    assert_eq!(Bank::for_lang_at("en", Junior, Preview).cov, Bank::for_lang_at("en", Junior, Full).cov);
}

// ---- v1.1 Phase E: heat, Call, Stretch, Ear, Doubles ----------------------

use super::payout::{self, Call, PayIn, Rules};

fn all_rules() -> Rules {
    Rules { heat: true, call: true, stretch: true, ear: true, doubles: true }
}

/// Four tiers at every length 3..=12, 25 words each, so every harder tier is populated.
fn tiered_bank() -> Bank {
    let mut rows = Vec::new();
    for len in 3..=12usize {
        for t in 0..4usize {
            for i in 0..25usize {
                rows.push((t, format!("{}{}{:02}", "x".repeat(len - 3), ["a", "b", "c", "d"][t], i)));
            }
        }
    }
    Bank::from_rows(rows)
}

fn pay(roll: usize, doubles: bool, stretch: bool, ear: bool, factor: (u32, u32), call_bonus: u32) -> u32 {
    payout::payout(PayIn { roll, jr: false, doubles, stretch, ear, factor, call_bonus })
}

#[test]
fn a17_payout_follows_the_fixed_order_and_ceils_once() {
    // roll 12 -> jackpot 24 -> doubles 48 -> Stretch 72 -> heat 5 (x2) 144 -> +3 Call = 147.
    assert_eq!(pay(12, true, true, false, payout::heat_factor(5), 3), 147);
    // Ear is not offered there: the pre-heat product (72) is above 60.
    assert!(!payout::ear_offered(12, true, true));
    assert_eq!(payout::pre_heat(12, false, true, true, false), 72);
    // Ear is offered on a modest Stretch word.
    assert!(payout::ear_offered(5, false, true));
}

#[test]
fn a18_the_cap_is_150_per_word() {
    // 12 x2 x2 x1.5 x1.5 x3 = 324, force-enabled Ear, capped.
    assert_eq!(pay(12, true, true, true, payout::heat_factor(7), 0), 150);
    // Cap applies to the total, Call bonus included.
    assert_eq!(pay(12, true, true, true, payout::heat_factor(7), 6), 150);
}

#[test]
fn i11_the_payout_is_pure_ceils_once_and_never_exceeds_the_cap() {
    // Reference in floats: ceil(product) + bonus, capped. One ceil, never two.
    for roll in 2..=12usize {
        for &d in &[false, true] {
            for &s in &[false, true] {
                for &e in &[false, true] {
                    for h in 0..=7u8 {
                        for &cb in &[0u32, 3, 6] {
                            let f = payout::heat_factor(h);
                            let got = pay(roll, d, s, e, f, cb);
                            let mut v = roll as f64 * if roll == 2 || roll == 12 { 2.0 } else { 1.0 };
                            if d { v *= 2.0 }
                            if s { v *= 1.5 }
                            if e { v *= 1.5 }
                            v *= f.0 as f64 / f.1 as f64;
                            let want = ((v - 1e-9).ceil() as u32 + cb).min(150);
                            assert_eq!(got, want, "roll {roll} d{d} s{s} e{e} h{h} cb{cb}");
                            assert!(got <= 150);
                        }
                    }
                }
            }
        }
    }
}

fn heat_session() -> Session {
    let mut s = Session::new(1, Mode::Marathon, 1, full_fake(), BTreeSet::new());
    s.rules = Rules { heat: true, ..Rules::default() };
    s
}

fn put(s: &mut Session, len: usize, doubles: bool, call: Option<Call>) {
    s.pending = Some(Roll { face: len as u8, len, word: "qqqqq".into(), tier: 0, doubles, call, ..Roll::default() });
}

#[test]
fn a19_the_heat_ladder_multiplies_by_the_pips_before_the_word() {
    let mut s = heat_session();
    s.pot = 10;
    s.heat = 4;
    put(&mut s, 8, false, None);
    let o = s.answer(true).unwrap();
    assert_eq!((o.points, s.pot, s.heat), (12, 22, 5), "heat 4 is x1.5");
    s.roll_again();
    put(&mut s, 6, false, None);
    let o = s.answer(true).unwrap();
    assert_eq!((o.points, s.pot, s.heat), (12, 34, 6), "heat 5 is x2");
    let b = s.bank().unwrap();
    assert_eq!((b.banked, b.score, s.heat), (34, 34, 0), "a Bank takes the heat to 0");
}

#[test]
fn a20_a_ward_absorbed_miss_still_resets_heat() {
    let mut s = heat_session();
    s.pot = 20;
    s.heat = 3;
    put(&mut s, 6, false, None);
    let o = s.answer(false).unwrap();
    assert!(o.warded);
    assert_eq!((s.pot, s.ward, s.ward_used, s.heat), (20, false, true, 0));
}

#[test]
fn i13_heat_is_the_run_of_unbanked_correct_words_capped_at_seven() {
    let mut s = heat_session();
    for n in 1..=10u8 {
        put(&mut s, 5, false, None);
        s.answer(true).unwrap();
        assert_eq!(s.heat, n.min(7), "after {n} correct words");
        s.roll_again();
    }
    put(&mut s, 5, false, None);
    s.answer(false).unwrap();
    assert_eq!(s.heat, 0, "a miss");
}

#[test]
fn heat_off_restores_the_v1_streak_multiplier_in_marathon() {
    let mut s = Session::new(2, Mode::Marathon, 1, full_fake(), BTreeSet::new()); // rules all off
    for _ in 0..3 {
        put(&mut s, 4, false, None);
        s.answer(true).unwrap();
        s.roll_again();
    }
    put(&mut s, 7, false, None);
    let o = s.answer(true).unwrap();
    assert_eq!((o.points, o.multiplied), (11, true), "v1: ceil(7 x 1.5)");
    assert_eq!(s.heat, 0, "no heat is kept with the rule off");
}

#[test]
fn a21_call_pays_only_with_a_right_guess_and_a_right_word() {
    let mut s = heat_session();
    s.rules.call = true;
    put(&mut s, 9, false, Some(Call::High));
    assert_eq!(s.answer(true).unwrap().points, 9 + 3);
    s.roll_again();
    put(&mut s, 9, false, Some(Call::Low));
    assert_eq!(s.answer(true).unwrap().points, 9, "wrong guess costs nothing and pays nothing");
    s.roll_again();
    put(&mut s, 7, false, Some(Call::Seven));
    // heat is 2 here, still x1
    assert_eq!(s.answer(true).unwrap().points, 7 + 6);
    // A right guess with a wrong spelling: nothing extra, and the pot goes as v1.
    s.roll_again();
    put(&mut s, 9, false, Some(Call::High));
    let o = s.answer(false).unwrap();
    assert_eq!(o.call_bonus, 0);
}

#[test]
fn the_call_is_locked_on_the_press_and_cleared_by_a_second_tap() {
    let mut s = heat_session();
    s.rules.call = true;
    assert!(s.set_call(Some(Call::High)));
    assert_eq!(s.call(), Some(Call::High));
    assert!(s.set_call(None));
    assert_eq!(s.call(), None);
    assert!(s.set_call(Some(Call::Low)));
    let r = s.press_roll().expect("rolls");
    assert_eq!(r.call, Some(Call::Low));
    assert!(!s.set_call(Some(Call::High)), "no change after the press (D-R15)");
    assert_eq!(s.call(), None, "the guess was consumed by the roll");
}

#[test]
fn a22_stretch_is_logged_before_the_word_is_drawn_and_draws_a_tier_up() {
    let mut s = Session::new(5, Mode::Marathon, 1, tiered_bank(), BTreeSet::new());
    s.rules = all_rules();
    let r = s.press_roll().expect("rolls");
    assert!(r.word.is_empty(), "no word exists before the choice (D-R9)");
    assert!(s.stretch_offered());
    let w = s.reveal(true, false).expect("draws");
    assert!(w.stretch && w.tier == 2, "tier {}", w.tier);
    assert_eq!(s.events, vec![Ev::StretchChosen(true), Ev::WordDrawn { tier: 2 }]);
}

#[test]
fn stretch_is_absent_where_no_harder_word_exists_and_at_expert_and_in_jr() {
    // full_fake has 15 words per tier per length: under the floor, so no Stretch.
    let mut s = Session::new(6, Mode::Marathon, 1, full_fake(), BTreeSet::new());
    s.rules = all_rules();
    s.press_roll().unwrap();
    assert!(!s.stretch_offered());
    // Expert has no tier above it.
    let mut e = Session::new(7, Mode::Marathon, 3, tiered_bank(), BTreeSet::new());
    e.rules = all_rules();
    e.press_roll().unwrap();
    assert!(!e.stretch_offered());
    // A Stretch request that is not on offer is ignored, not honoured.
    let w = e.reveal(true, false).unwrap();
    assert!(!w.stretch);
    // Jr never offers it.
    let mut j = Session::new(8, Mode::Jr, 0, tiered_bank(), BTreeSet::new());
    j.rules = all_rules();
    j.press_roll().unwrap();
    assert!(!j.stretch_offered() && !j.ear_offered(false));
}

#[test]
fn a23_ear_allows_exactly_one_playback() {
    let mut s = Session::new(9, Mode::Marathon, 1, tiered_bank(), BTreeSet::new());
    s.rules = all_rules();
    s.press_roll().unwrap();
    let w = s.reveal(false, true).unwrap();
    assert!(w.ear);
    assert!(s.request_audio(), "the first play is always allowed");
    assert!(!s.request_audio(), "no replay for an Ear word (I15)");
    // Without Ear the replay stays available.
    let mut n = Session::new(10, Mode::Marathon, 1, tiered_bank(), BTreeSet::new());
    n.rules = all_rules();
    n.press_roll().unwrap();
    n.reveal(false, false).unwrap();
    assert!(n.request_audio() && n.request_audio());
}

#[test]
fn ear_on_a_big_stretch_word_is_not_offered() {
    let mut s = Session::new(11, Mode::Marathon, 1, tiered_bank(), BTreeSet::new());
    s.rules = all_rules();
    s.rolled = Some(Roll { face: 12, len: 12, doubles: true, ..Roll::default() });
    assert!(!s.ear_offered(true), "12 doubles Stretch: pre-heat 72 > 60");
    assert!(s.ear_offered(false));
}

#[test]
fn i12_doubles_come_from_the_same_draw_and_all_2s_and_12s_are_doubles() {
    let mut rng = Rng::new(0xD0B1E5);
    let (mut dbl, n) = (0u32, 10_000u32);
    for _ in 0..n {
        let (a, b) = roll_two_dice_pair(&mut rng);
        if a == b {
            dbl += 1;
        }
        if a + b == 2 || a + b == 12 {
            assert_eq!(a, b, "a 2 or a 12 is always a double");
        }
    }
    let p = dbl as f64 / n as f64 * 100.0;
    assert!((p - 16.67).abs() <= 2.0, "doubles {p:.1}%");
    // The pair and the sum are one draw: roll_two_dice sums the same pair.
    let (mut r1, mut r2) = (Rng::new(5), Rng::new(5));
    for _ in 0..50 {
        let (a, b) = roll_two_dice_pair(&mut r1);
        assert_eq!(roll_two_dice(&mut r2), a + b);
    }
}

#[test]
fn a24_doubles_pay_double_in_sprint_and_a_jr_six_does() {
    let mut s = Session::new(12, Mode::Sprint, 1, full_fake(), BTreeSet::new());
    s.rules = Rules { doubles: true, ..Rules::default() };
    put(&mut s, 5, true, None);
    assert_eq!(s.answer(true).unwrap().points, 10, "a doubles 5 scores 10 (D-R20)");
    // Rule off: the glow flag is carried but pays nothing.
    let mut off = Session::new(13, Mode::Sprint, 1, full_fake(), BTreeSet::new());
    put(&mut off, 5, true, None);
    assert_eq!(off.answer(true).unwrap().points, 5);
    // Jr: face 6 is the lucky face; it pays the die (6) doubled.
    let mut j = Session::new(14, Mode::Jr, 0, full_fake(), BTreeSet::new());
    j.rules = Rules { doubles: true, ..Rules::default() };
    j.pending = Some(Roll { face: 6, len: 7, word: "qqqqqqq".into(), doubles: true, ..Roll::default() });
    assert_eq!(j.answer(true).unwrap().points, 12);
}

#[test]
fn a_jr_press_marks_a_six_as_doubles() {
    let mut seen_six = false;
    for seed in 0..40u64 {
        let mut j = Session::new(seed, Mode::Jr, 0, jr_bank("en"), BTreeSet::new());
        let r = j.press_roll().unwrap();
        assert_eq!(r.doubles, r.face == 6);
        seen_six |= r.face == 6;
    }
    assert!(seen_six, "forty seeds produced no six");
}

#[test]
fn stretching_does_not_change_the_dice_or_the_ordinary_words() {
    // Daily fairness (A29): two players on one seed, one Stretching every word,
    // one never. The dice and the normal word stream are identical; only the
    // served word differs, and it is a tier up.
    let run = |stretch: bool| {
        let mut s = Session::new(77, Mode::Marathon, 1, tiered_bank(), BTreeSet::new());
        s.rules = all_rules();
        let mut faces = Vec::new();
        for _ in 0..25 {
            let Some(r) = s.press_roll() else { break };
            faces.push(r.face);
            let w = s.reveal(stretch, false).expect("draws");
            if stretch {
                assert_eq!(w.tier, 2, "a Stretched word comes from the tier above");
            }
            s.answer(false);
            s.pending = None;
        }
        (faces, s.drawn.clone(), s.used.clone())
    };
    let (f1, d1, u1) = run(false);
    let (f2, d2, u2) = run(true);
    assert_eq!(f1, f2, "the dice");
    assert_eq!(d1, d2, "the ordinary word stream");
    assert_ne!(u1, u2, "what was served differs");
    let mut seen = std::collections::BTreeSet::new();
    assert!(u2.iter().all(|w| seen.insert(w.clone())), "no repeats among Stretched words");
}
