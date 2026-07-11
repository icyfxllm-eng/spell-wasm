//! Core-side scorer for drawn-character answers (中文/日本語 kanji). Per the
//! drawing-input design decision the boundary is: **webview JS calls the native
//! `DigitalInk` plugin's `recognize`; the Rust core never touches the plugin.**
//! JS hands the ranked candidates here and this returns the verdict.
//!
//! Scoring is expected-answer top-N matching (not the Latin letter-alignment of
//! `draw_judge`): a drawn character is Correct if the expected answer — or an
//! allowed equivalent form — appears in the recognizer's top-N candidates, with
//! N widening for easier tiers so young writers aren't punished for messy but
//! legible characters. Stroke order is never scored.
//!
//! Pure and fully unit-tested. `strokes_to_json` is the one Rust→JS helper (the
//! pad's strokes live in `drawing.rs`, so the core emits them for JS to pass to
//! `recognize`); everything else is JS-independent. Dormant until the native
//! plugin ships and the zh/ja drawing relaunch lands.
#![allow(dead_code)]

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

/// Serialize pad strokes (polylines of (x, y) in CSS pixels) to the JSON the JS
/// adapter forwards to `recognize`: `[[[x,y],...], ...]`. Timestamps are added
/// JS-side (t in ms from session start); DPR is a render concern, not sent.
pub fn strokes_to_json(strokes: &[Vec<(f64, f64)>]) -> String {
    let mut out = String::from("[");
    for (si, stroke) in strokes.iter().enumerate() {
        if si > 0 {
            out.push(',');
        }
        out.push('[');
        for (pi, (x, y)) in stroke.iter().enumerate() {
            if pi > 0 {
                out.push(',');
            }
            out.push_str(&format!("[{:.1},{:.1}]", x, y));
        }
        out.push(']');
    }
    out.push(']');
    out
}

/// How many top candidates count as a match, by tier — easier tiers are more
/// forgiving of messy-but-legible characters (spec §3 scoring).
pub fn top_n_for(tier: &str, kid: bool) -> usize {
    if kid {
        return 8;
    }
    match tier {
        "easy" => 8,
        "medium" => 5,
        _ => 3, // hard / expert
    }
}

#[derive(Deserialize)]
struct RawCand {
    #[serde(default)]
    text: String,
    #[serde(default)]
    score: f32,
}

#[derive(Deserialize)]
struct RawResult {
    #[serde(default)]
    candidates: Vec<RawCand>,
}

/// The recognizer's candidate texts, best-first. Garbage in → empty.
pub fn candidate_texts(json: &str) -> Vec<String> {
    serde_json::from_str::<RawResult>(json)
        .map(|r| r.candidates.into_iter().map(|c| c.text).collect())
        .unwrap_or_default()
}

fn nfc(s: &str) -> String {
    s.trim().nfc().collect()
}

/// Does the drawn character match? `expected` is the canonical answer; `accepted`
/// are extra allowed forms from the word entry (e.g. a Japanese kanji word that
/// also accepts its kana, or a Chinese word that also accepts a traditional
/// variant). Match = any accepted form appears (NFC-normalized) within the
/// tier's top-N candidates. Chinese passes no `accepted` → simplified only.
pub fn char_matches(expected: &str, accepted: &[String], candidates: &[String], tier: &str, kid: bool) -> bool {
    let n = top_n_for(tier, kid);
    let targets: Vec<String> = std::iter::once(nfc(expected)).chain(accepted.iter().map(|a| nfc(a))).collect();
    candidates.iter().take(n).any(|c| {
        let cf = nfc(c);
        targets.iter().any(|t| *t == cf)
    })
}

/// Full path from a recognizer result JSON to a verdict.
pub fn score_drawn(expected: &str, accepted: &[String], result_json: &str, tier: &str, kid: bool) -> bool {
    char_matches(expected, accepted, &candidate_texts(result_json), tier, kid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn strokes_serialize_to_nested_arrays() {
        let strokes = vec![vec![(0.0, 0.0), (10.5, 20.25)], vec![(3.0, 4.0)]];
        assert_eq!(strokes_to_json(&strokes), "[[[0.0,0.0],[10.5,20.2]],[[3.0,4.0]]]");
    }

    #[test]
    fn top_n_widens_for_easier_tiers() {
        assert_eq!(top_n_for("expert", false), 3);
        assert_eq!(top_n_for("hard", false), 3);
        assert_eq!(top_n_for("medium", false), 5);
        assert_eq!(top_n_for("easy", false), 8);
        assert_eq!(top_n_for("hard", true), 8); // kid mode always forgiving
    }

    #[test]
    fn expected_in_top_n_is_a_match() {
        let cands = v(&["猫", "描", "苗"]);
        assert!(char_matches("猫", &[], &cands, "expert", false)); // rank 1, N=3
    }

    #[test]
    fn expected_below_top_n_is_a_miss_at_hard() {
        let cands = v(&["描", "苗", "錨", "猫"]); // 猫 is rank 4
        assert!(!char_matches("猫", &[], &cands, "expert", false)); // N=3 → miss
        assert!(char_matches("猫", &[], &cands, "easy", false)); // N=8 → match
    }

    #[test]
    fn candidate_parsing_from_recognizer_json() {
        let json = r#"{"candidates":[{"text":"薔薇","score":0.0},{"text":"薇"}]}"#;
        assert_eq!(candidate_texts(json), v(&["薔薇", "薇"]));
        assert!(candidate_texts("garbage").is_empty());
    }

    #[test]
    fn accepted_equivalent_forms_match_for_japanese() {
        // A kanji word that also accepts its kana reading (per the word entry).
        let cands = v(&["ねこ"]);
        assert!(!char_matches("猫", &[], &cands, "expert", false)); // kana not accepted by default
        assert!(char_matches("猫", &v(&["ねこ"]), &cands, "expert", false)); // accepted → match
    }

    #[test]
    fn chinese_simplified_only_by_default() {
        // Traditional 貓 is NOT accepted unless the entry lists it.
        assert!(!char_matches("猫", &[], &v(&["貓"]), "hard", false));
    }

    #[test]
    fn end_to_end_result_json_to_verdict() {
        let json = r#"{"candidates":[{"text":"猫"},{"text":"苗"}]}"#;
        assert!(score_drawn("猫", &[], json, "expert", false));
    }
}
