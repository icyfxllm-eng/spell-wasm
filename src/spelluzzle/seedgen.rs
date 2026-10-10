//! The build step for verified seeds (see `seeds`). Not part of the app.
//!
//!   scripts/spelluzzle-seeds.sh /path/to/en_US.dic
//!
//! reads a Hunspell dictionary (build input, never committed, never shipped), makes the
//! large validity list, and for Medium, Hard and Expert keeps the first N seeds whose
//! bank-generated board still has exactly one answer under it. It writes
//! `assets/spelluzzle/seeds-en.json`. An `#[ignore]`d test, so `cargo test` never runs it.

use std::collections::BTreeSet;

use super::bank;
use super::gates::check_board;
use super::gen::generate;
use super::seeds;
use super::types::{Tier, GEN_VERSION};

/// Stems from a Hunspell `.dic` (the part before `/`), lowercase letters only.
fn stems(dic: &str) -> BTreeSet<String> {
    dic.lines().skip(1).filter_map(|l| {
        let w = l.split('/').next()?.trim();
        (!w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase())).then(|| w.to_string())
    }).collect()
}

/// Stems plus the common regular endings. Over-including only makes the check stricter
/// (a board is rejected for an alternative that is a word), never looser, so a crude
/// expansion is safe here.
fn expand(stems: &BTreeSet<String>) -> Vec<String> {
    let mut out: BTreeSet<String> = stems.clone();
    for w in stems {
        for suf in ["s", "es", "ed", "d", "ing", "er", "ers", "est", "ly", "y", "ies"] {
            out.insert(format!("{w}{suf}"));
        }
        if let Some(b) = w.strip_suffix('e') {
            out.insert(format!("{b}ing"));
        }
        if let Some(b) = w.strip_suffix('y') {
            out.insert(format!("{b}ies"));
            out.insert(format!("{b}ied"));
        }
    }
    out.into_iter().collect()
}

#[test]
#[ignore]
fn build_seeds() {
    let path = std::env::var("SPZ_VALIDITY_FILE").expect("SPZ_VALIDITY_FILE: path to a Hunspell en_US.dic");
    let sha = std::env::var("SPZ_VALIDITY_SHA256").unwrap_or_default();
    let target: usize = std::env::var("SPZ_TARGET").ok().and_then(|v| v.parse().ok()).unwrap_or(4000);
    let dic = std::fs::read_to_string(&path).expect("read the dictionary");
    let st = stems(&dic);
    let big_words = expand(&st);
    let small = bank::load("en", Vec::new()).expect("bank");
    let big = bank::load("en", big_words).expect("bank with the large list");
    println!("validity: {} stems, {} words with endings (bank-only list has {})", st.len(), big.validity_len(), small.validity_len());

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let mut tiers = serde_json::Map::new();
    let mut report = Vec::new();
    for tier in [Tier::Medium, Tier::Hard, Tier::Expert] {
        let mut kept: Vec<u32> = Vec::new();
        let (mut tried, mut rejected, mut gen_failed) = (0u32, 0u32, 0u32);
        let mut next = 0u32;
        while kept.len() < target && next < 400_000 {
            let batch: Vec<u32> = (next..next + 2000).collect();
            next += 2000;
            let chunk = batch.len().div_ceil(threads);
            let results: Vec<(u32, u8)> = std::thread::scope(|sc| {
                let hs: Vec<_> = batch.chunks(chunk).map(|part| {
                    let (small, big) = (&small, &big);
                    sc.spawn(move || part.iter().map(|&s| {
                        let r = match generate(s as u64, tier, small) {
                            Ok(g) => if check_board(&g.board, big).is_ok() { 1 } else { 2 },
                            Err(_) => 0,
                        };
                        (s, r)
                    }).collect::<Vec<_>>())
                }).collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            for (s, r) in results {
                tried += 1;
                match r {
                    1 if kept.len() < target => kept.push(s),
                    1 => {}
                    2 => rejected += 1,
                    _ => gen_failed += 1,
                }
            }
        }
        report.push(format!("{}: kept {} of {tried} seeds tried ({rejected} rejected by the large list, {gen_failed} could not generate)", tier.name(), kept.len()));
        tiers.insert(tier.name().to_string(), serde_json::json!(kept));
    }
    // F15: Par boards. Generate against the bank, then re-prove each under the large list (par is
    // recomputed there, and may differ), keeping the par that was proved.
    let par_target: usize = std::env::var("SPZ_PAR_TARGET").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    let mut par_out: Vec<(String, Vec<Vec<u32>>)> = Vec::new();
    for tier in [Tier::Hard, Tier::Expert] {
        let mut kept: Vec<Vec<u32>> = Vec::new();
        let (mut tried, mut none, mut rejected) = (0u32, 0u32, 0u32);
        let mut next = 0u32;
        while kept.len() < par_target && next < 400_000 {
            let batch: Vec<u32> = (next..next + 600).collect();
            next += 600;
            let chunk = batch.len().div_ceil(threads);
            let results: Vec<(u32, Option<Vec<u32>>, u8)> = std::thread::scope(|sc| {
                let hs: Vec<_> = batch.chunks(chunk).map(|part| {
                    let (small, big) = (&small, &big);
                    sc.spawn(move || part.iter().map(|&s| match super::par::generate_par(s as u64, tier, small) {
                        None => (s, None, 0u8),
                        Some(g) => match super::par::check_par(&g.board, big, true) {
                            Ok(v) => {
                                let mut row = vec![s, v.par as u32];
                                row.extend(g.record.words.iter().map(|&w| w as u32));
                                (s, Some(row), 1)
                            }
                            Err(_) => (s, None, 2),
                        },
                    }).collect::<Vec<_>>())
                }).collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            for (_, row, r) in results {
                tried += 1;
                match (r, row) {
                    (1, Some(row)) if kept.len() < par_target => kept.push(row),
                    (1, _) => {}
                    (2, _) => rejected += 1,
                    _ => none += 1,
                }
            }
        }
        report.push(format!("par {}: kept {} of {tried} seeds ({rejected} rejected by the large list, {none} could not generate)", tier.name(), kept.len()));
        par_out.push((tier.name().to_string(), kept));
    }
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"gen_version\": {GEN_VERSION},\n  \"lang\": \"en\",\n  \"fingerprint\": \"{}\",\n", seeds::fingerprint_hex(&small)));
    out.push_str(&format!("  \"validity_source\": \"Hunspell en_US (SCOWL) {}; build-time only, never shipped; sha256 {}\",\n  \"validity_words\": {},\n", path.rsplit('/').next().unwrap_or(""), sha, big.validity_len()));
    out.push_str("  \"tiers\": {\n");
    let n = tiers.len();
    for (i, (k, v)) in tiers.iter().enumerate() {
        out.push_str(&format!("    \"{k}\": {}{}\n", v, if i + 1 < n { "," } else { "" }));
    }
    out.push_str("  },\n  \"par\": {\n");
    let m = par_out.len();
    for (i, (k, rows)) in par_out.iter().enumerate() {
        let body = rows.iter().map(|r| format!("{:?}", r)).collect::<Vec<_>>().join(",");
        out.push_str(&format!("    \"{k}\": [{body}]{}\n", if i + 1 < m { "," } else { "" }));
    }
    out.push_str("  }\n}\n");
    let dest = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/spelluzzle/seeds-en.json");
    std::fs::write(dest, out).expect("write the seed file");
    for l in report {
        println!("{l}");
    }
}
