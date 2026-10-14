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
        s.pending = Some(Roll { face, len: face as usize, word: "qqqqq".into(), tier: 0 });
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
    s.pending = Some(Roll { face: len as u8, len, word: "qqqqq".into(), tier: 0 });
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
        s.pending = Some(Roll { face: 6, len: 7, word: "qqqqqqq".into(), tier: 0 });
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
