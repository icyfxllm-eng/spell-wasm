//! A6.3 — informational, not gating: how often the shared online grader and
//! the solo `submit_guess` ladder disagree on the A6.1 fixture. Prints a
//! table for the phase report; asserts nothing about the counts.

#[test]
fn a6_3_report_where_shared_and_solo_grading_disagree() {
    println!("lang cases shared_only solo_only");
    for (lang, n, shared_only, solo_only, ex) in grader_parity::disagreements(3) {
        println!("{lang} {n} {shared_only} {solo_only}");
        for e in ex {
            println!("    {e}");
        }
    }
}

/// The comparator is not blind: a known difference must show up. Solo English
/// exact matches go to Flask (`lower().strip()`), which keeps an inner space;
/// the shared grader's fold drops it.
#[test]
fn a6_3_the_comparator_sees_a_known_difference() {
    use spell_wasm::native_api::Tier;
    let c = grader_parity::Case {
        lang: "en",
        tier: Tier::Easy,
        entry: "cat".into(),
        typed: "c at".into(),
        kid: false,
        how: "probe",
    };
    let shared = spell_wasm::native_api::grade(c.lang, c.kid, &c.typed, &c.entry, c.tier);
    let solo = grader_parity::solo_verdict(&c);
    println!("probe 'c at' for 'cat': shared={shared} solo={solo}");
    assert!(shared && !solo, "expected the inner-space difference to be visible");
}
