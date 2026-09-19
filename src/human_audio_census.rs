//! CC-HUMAN-AUDIO Phase A — bank dump for the coverage census. Measurement only.
//!
//! Run: `HUMAN_AUDIO_BANK_OUT=path cargo test --lib human_audio_census -- --ignored`
//! Writes one `lang<TAB>tier<TAB>entry` row per bank entry, read through
//! `words::tier_for` (the accessor the game serves from), so the census counts
//! exactly what players are served. The Commons side lives in
//! `tools/human-audio-census/`, because the Commons API is a network call and
//! belongs on Eric's machine, not in CI.

#[test]
#[ignore]
fn human_audio_census_dump() {
    let out = std::env::var("HUMAN_AUDIO_BANK_OUT").expect("set HUMAN_AUDIO_BANK_OUT");
    let mut s = String::new();
    for (code, _name, _status, _dir) in crate::consts::BUILTIN_LANGS {
        for tier in crate::experience::TIERS {
            for w in crate::words::tier_for(code, tier) {
                s.push_str(&format!("{code}\t{tier}\t{w}\n"));
            }
        }
    }
    std::fs::write(out, s).unwrap();
}
