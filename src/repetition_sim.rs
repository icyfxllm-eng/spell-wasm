//! CC-LAUNCH-EN F7 / item 5: the repetition simulation, run with
//! `cargo test --lib repetition_sim`.
//!
//! A7.2: 200 Normal draws per English tier repeat nothing at Easy, Medium and
//! Hard, for both the legacy and the re-tiered bank. A7.1: 30 consecutive
//! Daily dates never serve the same word twice. The July bug was a Daily that
//! repeated across days (fixed eb966f38); this pins it for the release bank.

use crate::deck::Deck;

fn xorshift(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

fn draws(pool: &[&str], n: usize, seed: u64) -> Vec<String> {
    let pool: Vec<String> = pool.iter().map(|s| s.to_string()).collect();
    let mut deck = Deck::default();
    let mut s = seed;
    (0..n).map(|_| deck.next_with(&pool, &mut |k| (xorshift(&mut s) * k as f64) as usize % k)).collect()
}

fn assert_no_repeat(label: &str, words: &[String]) {
    let mut seen = std::collections::HashSet::new();
    for (i, w) in words.iter().enumerate() {
        assert!(seen.insert(w), "{label}: '{w}' repeated at draw {i}");
    }
}

/// A7.2 for one bank. 200 draws must fit inside one pass of the deck, so the
/// bank has to hold at least 200 words, and the deck must not repeat inside it.
fn check_bank(bank: &str, tier_of: fn(&str) -> &'static [&'static str]) {
    for tier in ["easy", "medium", "hard"] {
        let pool = tier_of(tier);
        assert!(pool.len() >= 200, "{bank}/{tier}: only {} words, 200 draws must repeat", pool.len());
        for seed in [1u64, 0x9e37_79b9_7f4a_7c15, 42, 7_777_777] {
            assert_no_repeat(&format!("{bank}/{tier} seed {seed}"), &draws(pool, 200, seed));
        }
    }
}

#[test]
fn repetition_sim_normal_200_draws_legacy_bank() {
    check_bank("legacy", |t| crate::words::tier_for_fixed(crate::consts::EN, t));
}

#[test]
fn repetition_sim_normal_200_draws_retier_bank() {
    // The re-tiered bank is off for the release (Eric 2026-11-07) but must stay
    // sound for when EN-EXPERT-GROWTH turns it back on.
    crate::flags::set_test_override(Some("on"));
    assert!(crate::flags::en_retier());
    check_bank("retier", crate::words::en_tier);
    crate::flags::set_test_override(None);
}

/// A7.1: 30 consecutive dates, standard and Little Speller: no word twice.
///
/// It failed on main until CC-LAUNCH-EN D23 widened the Daily's cycle-boundary
/// guard (daily::cycle_perm). Little Speller had been dealt "china" on
/// 2026-11-08 and again on 2026-11-14.
#[test]
fn repetition_sim_daily_30_dates() {
    for kid in [false, true] {
        // A Little Speller profile reads the legacy English bank (words::en_tier).
        crate::words::set_junior_for_test(kid);
        let mut all = Vec::new();
        for day in 1..=30 {
            let date = format!("2026-11-{day:02}");
            let (_, words) = crate::daily::build_words(crate::consts::EN, &date, kid);
            assert!(!words.is_empty(), "{date}: empty Daily");
            all.extend(words);
        }
        crate::words::set_junior_for_test(false);
        assert_no_repeat(&format!("daily kid={kid}"), &all);
    }
}


/// A7.2 for Spell Jr: free play draws from the legacy bank through the kid
/// filter (game.rs active_word_list), which CC-EN-VARIANTS made stricter. Each
/// tier a child can reach must still hold 200 words, and 200 draws must not
/// repeat.
#[test]
fn repetition_sim_normal_200_draws_spell_jr_pool() {
    for tier in ["easy", "medium", "hard"] {
        let base: Vec<String> = crate::words::tier_for_fixed(crate::consts::EN, tier).iter().map(|s| s.to_string()).collect();
        let pool = crate::kid_filter::filter_kid(crate::consts::EN, base);
        assert!(pool.len() >= 200, "spell jr/{tier}: only {} words", pool.len());
        let refs: Vec<&str> = pool.iter().map(String::as_str).collect();
        for seed in [1u64, 42, 7_777_777] {
            assert_no_repeat(&format!("spell jr/{tier} seed {seed}"), &draws(&refs, 200, seed));
        }
    }
}
