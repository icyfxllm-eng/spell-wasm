//! A6.1 (WASM half): the same fixture, graded by the same function compiled to
//! wasm32, must produce the golden report byte for byte.
//!
//! Run: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!      cargo test -p grader-parity --target wasm32-unknown-unknown
#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn wasm_verdicts_match_the_golden() {
    let report = grader_parity::report();
    assert!(!grader_parity::GOLDEN.trim().is_empty(), "golden is empty");
    assert_eq!(report, grader_parity::GOLDEN, "wasm verdicts differ from the native golden");
}
