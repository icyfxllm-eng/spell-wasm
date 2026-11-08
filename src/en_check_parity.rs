//! CC-LAUNCH-EN Phase B item 3: the LOCAL half of the English grading parity
//! check. The App Store build grades English on the device only
//! (`game::en_grade_local`); dev builds still ask the server's /api/check. This
//! test writes what the local grader decides for every (word, typed, kid) case;
//! tools/parity/en_check_parity.py runs the backend's own `check()` on the same
//! cases and requires zero differences.
//!
//! Run through the driver, not on its own:
//!   python3 tools/parity/en_check_parity.py
//!
//! Cases: every word of the English bank, legacy tiers and re-tier tiers, each
//! typed as itself and as a handful of realistic variations (case, a curly
//! apostrophe, a dropped space, accents stripped, a dropped letter, two letters
//! swapped), in normal and Spell Jr grading; plus both directions of every pair
//! in data/en/accepted-variants.tsv (the UK-variants fixture; there is no
//! tests/fixtures/en-uk-variants.json).

use std::collections::BTreeMap;
use std::io::Write;

use unicode_normalization::UnicodeNormalization;

use crate::consts::TIER_ORDER;
use crate::game::{en_grade_local, en_route, EnRoute};

fn typed_forms(w: &str) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = vec![("exact", w.to_string()), ("upper", w.to_uppercase())];
    let mut cs = w.chars();
    if let Some(f) = cs.next() {
        out.push(("capital", f.to_uppercase().chain(cs).collect()));
    }
    if w.contains('\'') {
        out.push(("curly_apostrophe", w.replace('\'', "\u{2019}")));
    }
    if w.contains(' ') {
        out.push(("no_space", w.replace(' ', "")));
    }
    if w.contains('-') {
        out.push(("no_hyphen", w.replace('-', "")));
    }
    let bare: String = w.nfd().filter(|c| !('\u{0300}'..='\u{036f}').contains(c)).collect();
    if bare != w {
        out.push(("accents_stripped", bare));
    }
    let chars: Vec<char> = w.chars().collect();
    if chars.len() > 1 {
        out.push(("drop_last", chars[..chars.len() - 1].iter().collect()));
    }
    if chars.len() > 2 && chars[1] != chars[2] {
        let mut s = chars.clone();
        s.swap(1, 2);
        out.push(("swap_2_3", s.into_iter().collect()));
    }
    out.dedup_by(|a, b| a.1 == b.1);
    out
}

fn route_name(r: EnRoute) -> &'static str {
    match r {
        EnRoute::OtherSense => "other_sense",
        EnRoute::Variant => "variant",
        EnRoute::Grade => "grade",
    }
}

fn tier_banks() -> Vec<(String, &'static [&'static str])> {
    use crate::word_data_en_retier::*;
    let mut v: Vec<(String, &'static [&'static str])> = Vec::new();
    for t in TIER_ORDER {
        v.push((format!("legacy:{t}"), crate::words::tier_for_fixed(crate::consts::EN, t)));
    }
    for (t, bank) in TIER_ORDER.iter().zip([EN_RETIER_EASY, EN_RETIER_MEDIUM, EN_RETIER_HARD, EN_RETIER_EXPERT]) {
        v.push((format!("retier:{t}"), bank));
    }
    v
}

fn uk_pairs() -> Vec<(String, String)> {
    let tsv = include_str!("../data/en/accepted-variants.tsv");
    tsv.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.trim().to_string(), f.next()?.trim().to_string()))
        })
        .collect()
}

#[test]
#[ignore = "run by tools/parity/en_check_parity.py, which sets EN_PARITY_OUT"]
fn en_check_parity_local_half() {
    let out = std::env::var("EN_PARITY_OUT").expect("EN_PARITY_OUT: run tools/parity/en_check_parity.py");
    // word -> the tiers it sits in, across both banks.
    let mut words: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (tier, bank) in tier_banks() {
        for w in bank {
            words.entry(w).or_default().push(tier.clone());
        }
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(&out).expect("create EN_PARITY_OUT"));
    let mut n = 0usize;
    let mut emit = |word: &str, typed: &str, form: &str, source: &str, tiers: &[String], kid: bool| {
        let route = en_route(word, typed);
        let line = serde_json::json!({
            "word": word, "typed": typed, "form": form, "source": source, "tiers": tiers, "kid": kid,
            "route": route_name(route),
            "local": en_grade_local(word, typed, kid),
        });
        writeln!(f, "{line}").unwrap();
        n += 1;
    };
    for (w, tiers) in &words {
        for (form, typed) in typed_forms(w) {
            for kid in [false, true] {
                emit(w, &typed, form, "bank", tiers, kid);
            }
        }
    }
    for (us, uk) in uk_pairs() {
        let tiers = words.get(us.as_str()).cloned().unwrap_or_default();
        for kid in [false, true] {
            emit(&us, &uk, "uk_for_us", "uk_variants", &tiers, kid);
            emit(&uk, &us, "us_for_uk", "uk_variants", &tiers, kid);
        }
    }
    drop(emit);
    f.flush().unwrap();
    eprintln!("en_check_parity: {n} cases from {} words -> {out}", words.len());
}
