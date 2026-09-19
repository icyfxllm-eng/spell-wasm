//! CC-WORDGRID §3 census — measurement only, no feature code.
//!
//! Run: `cargo test --lib wordgrid_census -- --ignored --nocapture`
//! It reads the REAL banks through the same accessors the game uses and the
//! keyboard SSOT files, segments with the same grapheme library, and prints one
//! row per bank language. Gates it cannot measure from this repo (E3 audio,
//! E5 language confusion matrix, E6 external dictionary) are reported as such
//! rather than guessed.

use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

fn keyboard(code: &str) -> Option<(HashSet<String>, HashSet<String>)> {
    let path = format!("{}/assets/keyboards/{code}.json", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let mut keys = HashSet::new();
    for row in v["rows"].as_array()? {
        for g in row.as_str()?.graphemes(true) {
            keys.insert(g.nfc().collect::<String>().to_lowercase());
        }
    }
    let mut long = HashSet::new();
    if let Some(lp) = v["longPress"].as_object() {
        for alts in lp.values() {
            for g in alts.as_str().unwrap_or("").graphemes(true) {
                long.insert(g.nfc().collect::<String>().to_lowercase());
            }
        }
    }
    Some((keys, long))
}

#[test]
#[ignore]
fn wordgrid_census() {
    println!("| lang | dir | E2 words fully typeable | E2 fail % | clusters needing >1 press | E4 audio-agnostic pool per tier (easy/medium/hard/expert) | E7 blocklist |");
    println!("|---|---|---|---|---|---|---|");
    for (code, _name, _status, dir) in crate::consts::BUILTIN_LANGS {
        let (keys, long) = keyboard(code).unwrap_or_default();
        let mut total = 0usize;
        let mut ok = 0usize;
        let mut multi: std::collections::BTreeSet<String> = Default::default();
        let mut per_tier = Vec::new();
        for tier in crate::experience::TIERS {
            let words = crate::words::tier_for(code, tier);
            let mut n_ok = 0usize;
            for w in words {
                // zh stores "pinyin|hanzi"; the typed form is the first part.
                let typed = w.split('|').next().unwrap_or(w);
                let nfc: String = typed.nfc().collect::<String>().to_lowercase();
                total += 1;
                let mut all = true;
                for g in nfc.graphemes(true) {
                    if keys.contains(g) || long.contains(g) {
                        continue;
                    }
                    // A cluster the keyboard can only build from several keys.
                    if g.chars().count() > 1
                        && g.chars().all(|c| {
                            let s = c.to_string();
                            keys.contains(&s) || long.contains(&s)
                        })
                    {
                        if multi.len() < 12 {
                            multi.insert(g.to_string());
                        }
                        continue;
                    }
                    all = false;
                }
                if all {
                    ok += 1;
                    n_ok += 1;
                }
            }
            per_tier.push(n_ok);
        }
        let fail_pct = if total == 0 { 0.0 } else { 100.0 * (total - ok) as f64 / total as f64 };
        let block = std::path::Path::new(&format!("{}/assets/words/profanity/{code}.txt", env!("CARGO_MANIFEST_DIR"))).exists();
        println!(
            "| {code} | {:?} | {ok}/{total} | {fail_pct:.1}% | {} | {} | {} |",
            dir,
            if multi.is_empty() { "none".to_string() } else { multi.into_iter().collect::<Vec<_>>().join(" ") },
            per_tier.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("/"),
            if block { "yes" } else { "**none**" },
        );
    }
}
