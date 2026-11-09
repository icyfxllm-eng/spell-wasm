//! CC-LAUNCH-EN Phase C: English quality acceptance tests, run with
//! `cargo test --lib en_quality`.
//!
//! F2 (British spelling): A2.1, A2.2, A2.3. Normal, Daily, Climb and Spell Jr
//! all grade through the one submission path, `game::submit_guess`, and for
//! English outside Impossible that path is exactly: `en_route` (other sense or
//! national variant, resolved locally) else `en_grade_local` (the App Store
//! build's only grader). Tier and mode never enter it; `kid` (Spell Jr) does.
//! So these tests drive those two real functions for both kid values instead
//! of a copy of them.
//!
//! I2.3 (Climb score and combo identical for v and w): scoring reads only the
//! boolean verdict; the other-sense marker feeds the learner tag, never the
//! score. Asserting the verdict is therefore asserting the score.

use crate::consts::EN;
use crate::game::{en_grade_local, en_route, EnRoute};

/// The verdict the release build reaches for English: `submit_guess` with
/// `impossible == false`.
fn en_correct(word: &str, typed: &str, kid: bool) -> bool {
    match en_route(word, typed) {
        EnRoute::OtherSense | EnRoute::Variant => true,
        EnRoute::Grade => en_grade_local(word, typed, kid),
    }
}

fn fixture() -> Vec<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/en-uk-variants.json")).expect("fixture parses");
    v["pairs"]
        .as_array()
        .expect("pairs")
        .iter()
        .map(|p| (p["us"].as_str().unwrap().to_string(), p["uk"].as_str().unwrap().to_string()))
        .collect()
}

/// A2.1: every frozen pair grades correct both ways (English is graded both
/// ways: Eric 2026-10-19), for a standard player and for Spell Jr.
#[test]
fn en_quality_a2_1_uk_spellings_grade_correct() {
    let pairs = fixture();
    assert!(pairs.len() >= 300, "A2.1 wants 300 pairs, fixture has {}", pairs.len());
    for name in ["colour", "organise", "centre", "grey", "programme", "tyre", "aluminium", "jewellery", "mould", "plough", "defence", "licence", "practise", "catalogue", "cheque"] {
        assert!(pairs.iter().any(|(_, uk)| uk == name), "spec example {name} missing from the fixture");
    }
    let mut bad = Vec::new();
    for (us, uk) in &pairs {
        for kid in [false, true] {
            if !en_correct(us, uk, kid) {
                bad.push(format!("{uk} for {us} (kid {kid})"));
            }
            if !en_correct(uk, us, kid) {
                bad.push(format!("{us} for {uk} (kid {kid})"));
            }
        }
    }
    assert!(bad.is_empty(), "{} of {} pairs graded wrong: {:?}", bad.len(), pairs.len(), &bad[..bad.len().min(20)]);
}

/// I2.4: a variant is accepted per row. A UK form never grades correct
/// against a different row that merely shares a pair member elsewhere.
#[test]
fn en_quality_i2_4_variant_acceptance_is_per_row() {
    let pairs = fixture();
    let mut bad = Vec::new();
    for (i, (_, uk)) in pairs.iter().enumerate() {
        // Compare against a fixed, unrelated row: the next pair whose US form
        // differs and is not itself paired with this UK form.
        let (other, _) = &pairs[(i + 37) % pairs.len()];
        let related = pairs.iter().any(|(u, k)| u == other && k == uk);
        if !related && other != uk && crate::variants::accepts(other, uk) {
            bad.push(format!("{uk} accepted for {other}"));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

fn bank() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for tier in ["easy", "medium", "hard", "expert"] {
        all.extend_from_slice(crate::words::tier_for_fixed(EN, tier));
    }
    all
}

/// One deterministic one-letter corruption: the middle letter moved one step
/// along the alphabet (z wraps to a).
fn corrupt(w: &str) -> String {
    let mut c: Vec<char> = w.chars().collect();
    let m = c.len() / 2;
    c[m] = match c[m] {
        'z' => 'a',
        ch if ch.is_ascii_lowercase() => ((ch as u8) + 1) as char,
        _ => 'e',
    };
    c.into_iter().collect()
}

/// A2.2: 100 US-only bank words with a one-letter corruption -> 0 accepted.
/// A corruption that happens to be another real bank word or a listed
/// homophone is skipped: accepting that would be correct, not false.
#[test]
fn en_quality_a2_2_corrupted_us_words_are_never_accepted() {
    let bank = bank();
    let known: std::collections::HashSet<&str> = bank.iter().copied().collect();
    let mut candidates: Vec<&str> = bank
        .iter()
        .copied()
        .filter(|w| w.len() >= 4 && w.chars().all(|c| c.is_ascii_lowercase()))
        .filter(|w| crate::accepted::alternates(EN, w).is_empty() && crate::variants::display(w, true) == *w)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    let step = (candidates.len() / 120).max(1);
    let mut tested = 0;
    let mut bad = Vec::new();
    for w in candidates.iter().step_by(step) {
        let c = corrupt(w);
        if known.contains(c.as_str()) || crate::homophones::accepts(EN, w, &c) {
            continue;
        }
        for kid in [false, true] {
            if en_correct(w, &c, kid) {
                bad.push(format!("{c} for {w} (kid {kid})"));
            }
        }
        tested += 1;
        if tested == 100 {
            break;
        }
    }
    assert_eq!(tested, 100, "control set came up short");
    assert!(bad.is_empty(), "false accepts: {bad:?}");
}

/// A2.3: the result card names the word through `variants::display` with the
/// player's preference; the default (British switch off) shows the stored US
/// form for every pair, including when the player typed the UK form, and the
/// rows the bank stores in British form (towards, afterwards) show US too.
/// Accent (`diacritic`) and `compound` rows are accepted answers only, never a
/// display pair (`accepted::pairs`), and the bank stores their plain form, so
/// only the US direction applies to them.
#[test]
fn en_quality_a2_3_card_shows_us_form_by_default() {
    let v: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/en-uk-variants.json")).unwrap();
    for p in v["pairs"].as_array().unwrap() {
        let (us, uk, kind) = (p["us"].as_str().unwrap(), p["uk"].as_str().unwrap(), p["kind"].as_str().unwrap());
        assert_eq!(crate::variants::display(us, false), us, "card for {us}");
        if !matches!(kind, "diacritic" | "compound") {
            assert_eq!(crate::variants::display(uk, false), us, "card for a {uk} row");
        }
    }
}

/// F8 / A8.1: 50 true homophones grade through the other-sense path, in both
/// directions and for Spell Jr too, never a plain wrong. With English cueMode
/// off (config/sense-cue.json) this is the accept-all fallback (SENSE-CUE F2a):
/// `en_route` returns `OtherSense`, `submit_guess` marks VALID_OTHER_SENSE and
/// grades it correct locally.
#[test]
fn en_quality_a8_1_true_homophones_take_the_other_sense_path() {
    let v: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/en-homophones.json")).unwrap();
    let pairs = v["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 50);
    let mut bad = Vec::new();
    for p in pairs {
        let (a, b) = (p["a"].as_str().unwrap(), p["b"].as_str().unwrap());
        for (word, typed) in [(a, b), (b, a)] {
            if en_route(word, typed) != EnRoute::OtherSense {
                bad.push(format!("{typed} for {word}: not other-sense"));
            }
            for kid in [false, true] {
                if !en_correct(word, typed, kid) {
                    bad.push(format!("{typed} for {word} (kid {kid}): wrong"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}
