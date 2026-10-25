//! A6.1 (native half): the shared grader's verdicts on the fixture match the
//! golden file. `tests/wasm.rs` in grader-parity checks the same golden from
//! WASM, so the two targets agree exactly when both pass.
//!
//! Regenerate the golden with `UPDATE_GOLDEN=1` (then the WASM half must pass
//! against it unchanged, or the targets disagree).

#[test]
fn native_verdicts_match_the_golden() {
    let report = grader_parity::report();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../grader-parity/golden.txt");
        std::fs::write(path, &report).expect("write golden");
        return;
    }
    assert!(!grader_parity::GOLDEN.trim().is_empty(), "golden is empty: run with UPDATE_GOLDEN=1");
    assert_eq!(report, grader_parity::GOLDEN, "native verdicts drifted from the golden");
}

#[test]
fn the_fixture_covers_every_active_language_at_full_size() {
    let langs = grader_parity::languages();
    assert!(langs.len() >= 14, "expected the app's active languages, got {langs:?}");
    let report = grader_parity::report();
    for lang in &langs {
        let line = report.lines().find(|l| l.split(' ').next() == Some(lang)).expect("language line");
        let n: usize = line.split(' ').nth(1).unwrap().parse().unwrap();
        assert_eq!(n, grader_parity::CASES_PER_LANG, "{lang} has {n} cases");
    }
}
