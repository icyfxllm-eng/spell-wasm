//! CC-SPELLDOKU v1.3 §0 census — measurement only, no feature code.
//!
//! Run: `SD_TIER_CENSUS_OUT=path cargo test --lib spelldoku_tier_census -- --ignored --nocapture`
//! C1 counts bank rows per (language, tier) through `words::tier_for` — the same
//! lists `tier_for` ships and the offline expert-tier calibration scores — and
//! applies the per-word gates Word Mode already applies at draw time
//! (`wordmode::candidate`): 2..=12 graphemes, keyboard-typeable, not blocked,
//! and a legal initial glyph. C2 evaluates F5's depth rule against C1.

#[test]
#[ignore]
fn spelldoku_tier_census() {
    use unicode_segmentation::UnicodeSegmentation;
    const SPAN: [(&str, &[&str]); 4] = [
        ("easy", &["easy", "medium"]),
        ("medium", &["easy", "medium", "hard"]),
        ("hard", &["easy", "medium", "hard", "expert"]),
        ("expert", &["medium", "hard", "expert"]),
    ];
    let need = (27.0 * 1.25_f64).ceil() as usize; // F5: >= 27 * 1.25
    let need_b = (4.0 * 1.25_f64).ceil() as usize; // F6 (Reading B)
    let mut out = String::new();
    out.push_str("| lang | tier | rows | eligible | too long | untypeable | blocked | no glyph |\n|---|---|---:|---:|---:|---:|---:|---:|\n");
    let mut depth: Vec<(String, String, usize)> = Vec::new();
    for (lang, _n, _s, _d) in crate::consts::BUILTIN_LANGS {
        for tier in crate::experience::TIERS {
            let (mut long, mut untypeable, mut blocked, mut noglyph, mut ok) = (0, 0, 0, 0, 0);
            for row in crate::words::tier_for(lang, tier) {
                let (spelling, display) = match row.split_once('|') {
                    Some((p, d)) => (p, Some(d)),
                    None => (*row, None),
                };
                let len = spelling.graphemes(true).count();
                if !(2..=crate::spelldoku::wordmode::MAX_GRAPHEMES).contains(&len) {
                    long += 1;
                } else if !crate::spelldoku::wordmode::typeable(lang, spelling) {
                    untypeable += 1;
                } else if crate::profanity::is_blocked(spelling) {
                    blocked += 1;
                } else if crate::spelldoku::wordmode::glyph(lang, spelling, display).is_none() {
                    noglyph += 1;
                } else {
                    ok += 1;
                }
            }
            out.push_str(&format!(
                "| {lang} | {tier} | {} | {ok} | {long} | {untypeable} | {blocked} | {noglyph} |\n",
                crate::words::tier_for(lang, tier).len()
            ));
            depth.push((lang.to_string(), tier.to_string(), ok));
        }
    }
    let d = |lang: &str, tier: &str| {
        depth.iter().find(|(l, t, _)| l == lang && t == tier).map(|(_, _, n)| *n).unwrap_or(0)
    };
    out.push_str(&format!("\nC2 — F5 needs >= {need} eligible rows in EVERY tier of the span (Reading B needs >= {need_b}).\n\n"));
    out.push_str("| lang | Easy board (E-M) | Medium (E-H) | Hard (E-X) | Expert (M-X) | Reading B (4 tiers) |\n|---|---|---|---|---|---|\n");
    let (mut pairs_ok, mut pairs) = (0, 0);
    for (lang, _n, _s, _d) in crate::consts::BUILTIN_LANGS {
        let mut cells = Vec::new();
        for (board, span) in SPAN {
            let worst = span.iter().map(|t| d(lang, t)).min().unwrap_or(0);
            let yes = worst >= need;
            pairs += 1;
            pairs_ok += yes as usize;
            cells.push(format!("{} ({worst})", if yes { "yes" } else { "NO" }));
            let _ = board;
        }
        let worst_all = crate::experience::TIERS.iter().map(|t| d(lang, t)).min().unwrap_or(0);
        cells.push(format!("{} ({worst_all})", if worst_all >= need_b { "yes" } else { "NO" }));
        out.push_str(&format!("| {lang} | {} |\n", cells.join(" | ")));
    }
    out.push_str(&format!("\n{pairs_ok} of {pairs} (language, board tier) pairs survive F5.\n"));
    match std::env::var("SD_TIER_CENSUS_OUT") {
        Ok(p) => std::fs::write(p, &out).unwrap(),
        Err(_) => println!("{out}"),
    }
}
