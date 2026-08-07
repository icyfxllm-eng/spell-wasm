//! CC-PHOTO-IMPORT Phase 1 — candidate classification, ALL in the core (Rust).
//!
//! Raw OCR lines are shaped by `native_lang::parse_candidates` (tokenize, NFC,
//! dedupe — the existing seed); this module adds the spec-F3 classification:
//! every candidate is exactly one of {in-dictionary, custom, filtered}, decided
//! deterministically and offline:
//!
//! * `Filtered`      — fails the shared save gate (`native_lang::gate_reason`:
//!                     profanity screen or charset). Filtered candidates are
//!                     SHOWN flagged in the adult review, never silently dropped.
//! * `InDictionary`  — present in the study language's own word banks
//!                     (`words::tier_for`, all four tiers), compared NFC-folded
//!                     (`norm::fold_strict`). Per gate G-C the banks are the
//!                     AUTHORITY; the async `UITextChecker` bridge may later
//!                     promote Custom→InDictionary but never demote.
//! * `Custom`        — clean, but not in the banks: importable as the player's
//!                     own word.
//!
//! Low-confidence tokens are carried (`confidence_low`), NEVER dropped — the
//! review sheet renders them dimmed and editable (Phase 3). The threshold that
//! decides "low" is the caller's (calibrated in Phase 7 against the handwriting
//! fixtures); this module just transports the bit.
//!
//! Classification uses the CURRENT STUDY LANGUAGE passed in by the caller —
//! never an auto-detected one (single-source-of-truth doctrine).

use crate::norm;

/// The spec-F3 class of one OCR candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordClass {
    /// In the study language's word banks — known-good.
    InDictionary,
    /// Clean but unknown to the banks — imports as the player's own word.
    Custom,
    /// Fails the shared save gate (profanity/charset) — flagged, adult-editable.
    Filtered,
}

/// One classified candidate, ready for the review sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The candidate word, NFC-normalized by the parser upstream.
    pub word: String,
    pub class: WordClass,
    /// Vision reported low confidence for the line this token came from. The
    /// review sheet pre-shows these dimmed + editable; they are never dropped.
    pub confidence_low: bool,
}

/// PROVISIONAL low-confidence threshold (Vision line confidence 0..1) — chips
/// at or below render dimmed-but-editable, never dropped. To be CALIBRATED
/// against the Phase 7 handwriting fixtures and signed off by Eric (plan: the
/// number is brought to him, not buried); until then it errs low so few chips
/// dim. Vision's .accurate path reports ~1.0 for clean print and commonly
/// 0.3–0.5 for shaky handwriting.
pub const LOW_CONFIDENCE: f32 = 0.4;

/// Phase 2 entry: recognized `(line, confidence)` pairs → parsed, deduped,
/// classified candidates. Each token inherits its LINE's Vision confidence
/// (Vision reports per line, not per word); dedupe is case-insensitive across
/// the whole page set, first occurrence wins (keeping its confidence bit).
pub fn extract_classified(lang: &str, lines: &[(String, f32)]) -> Vec<Candidate> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut tokens: Vec<(String, bool)> = Vec::new();
    for (line, confidence) in lines {
        let low = *confidence <= LOW_CONFIDENCE;
        for word in crate::native_lang::parse_candidates(&[line.clone()]) {
            if seen.insert(word.to_lowercase()) {
                tokens.push((word, low));
            }
        }
    }
    classify(lang, &tokens)
}

/// Classify parsed tokens for the study language `lang`. `tokens` pairs each
/// word with its low-confidence bit (false when the bridge has no confidence
/// data — today's payload — so behavior is unchanged until Phase 2 plumbs it).
pub fn classify(lang: &str, tokens: &[(String, bool)]) -> Vec<Candidate> {
    tokens
        .iter()
        .map(|(word, low)| Candidate {
            word: word.clone(),
            class: classify_one(lang, word),
            confidence_low: *low,
        })
        .collect()
}

/// Classify a single word — the review sheet's LIVE re-classify on edit
/// (Phase 3): fixing a misread updates the chip's class before save.
pub fn classify_word(lang: &str, word: &str) -> WordClass {
    classify_one(lang, word)
}

fn classify_one(lang: &str, word: &str) -> WordClass {
    if crate::native_lang::gate_reason(word).is_some() {
        return WordClass::Filtered;
    }
    if in_banks(lang, word) {
        WordClass::InDictionary
    } else {
        WordClass::Custom
    }
}

/// Bank membership for the study language, NFC-folded on both sides so a
/// decomposed OCR form ("nin" + combining tilde + "o") matches the bank's
/// composed "niño" — and the IMPORTED text stays byte-identical to the
/// dictionary form because the parser already NFC-normalized it (acceptance #3).
fn in_banks(lang: &str, word: &str) -> bool {
    let folded = norm::fold_strict(word);
    if folded.is_empty() {
        return false;
    }
    for tier in ["easy", "medium", "hard", "expert"] {
        for entry in crate::words::tier_for(lang, tier) {
            // Mandarin entries are "pinyin|hanzi" — a photographed page may
            // carry either form, so both halves count as membership.
            match entry.split_once('|') {
                Some((typed, spoken)) => {
                    if norm::fold_strict(typed) == folded || norm::fold_strict(spoken) == folded {
                        return true;
                    }
                }
                None => {
                    if norm::fold_strict(entry) == folded {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ── F14: smashed-word segmentation ──────────────────────────────────
//
// Eric's audit: photographing a page can yield "Thisisanexample" as ONE
// candidate where a child sees four words. OCR loses the spaces; the
// import had no way to get them back.
//
// D7 (recommendation, and the reason this only ever PROPOSES): a
// prototype over the real EN bank split 1 in 10 plausible custom words
// wrongly — "Sundeep" becomes "sun" + "deep". Silently auto-splitting
// would rename somebody's child. So this returns a suggestion and the
// review sheet asks; nothing splits without a tap.
//
// Its ceiling is the bank. "thecatsatonthemat" does not segment because
// `sat` and `mat` are missing from the EN bank (see BD-G3, the
// core-vocabulary gap). That is the right failure: no proposal at all
// beats a wrong one.

/// No 1-letter pieces — "a"/"i" turn every long token into confetti.
const MIN_PIECE: usize = 2;
/// Beyond this the DP is not worth running on a review sheet.
const MAX_WORD: usize = 24;

/// Folded bank forms for one language, built once. `in_banks` walks all
/// ~6,800 entries per call; the segmenter needs hundreds of lookups per
/// word, so it needs a set rather than a scan.
fn bank_set(lang: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for tier in ["easy", "medium", "hard", "expert"] {
        for entry in crate::words::tier_for(lang, tier) {
            match entry.split_once('|') {
                Some((typed, spoken)) => {
                    set.insert(norm::fold_strict(typed));
                    set.insert(norm::fold_strict(spoken));
                }
                None => {
                    set.insert(norm::fold_strict(entry));
                }
            }
        }
    }
    set
}

/// Propose a split of a smashed token into two or more bank words, or
/// `None` when the word is already known, cannot be fully segmented, or
/// is too long to bother with. Prefers the FEWEST pieces, so
/// "onetwothree" is three words rather than any longer shredding.
pub fn propose_split(lang: &str, word: &str) -> Option<Vec<String>> {
    let folded = norm::fold_strict(word);
    let chars: Vec<char> = folded.chars().collect();
    let n = chars.len();
    if n < MIN_PIECE * 2 || n > MAX_WORD {
        return None;
    }
    let bank = bank_set(lang);
    if bank.contains(&folded) {
        return None; // a real word is not a smash-up
    }
    // best[i] = fewest-piece segmentation of the first i chars
    let mut best: Vec<Option<Vec<String>>> = vec![None; n + 1];
    best[0] = Some(Vec::new());
    for i in MIN_PIECE..=n {
        for j in 0..=i.saturating_sub(MIN_PIECE) {
            let Some(prefix) = best[j].clone() else { continue };
            let piece: String = chars[j..i].iter().collect();
            if !bank.contains(&piece) {
                continue;
            }
            let mut cand = prefix;
            cand.push(piece);
            let better = match &best[i] {
                Some(cur) => cand.len() < cur.len(),
                None => true,
            };
            if better {
                best[i] = Some(cand);
            }
        }
    }
    match best.pop().flatten() {
        Some(pieces) if pieces.len() >= 2 => Some(pieces),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F14 — the smash-up Eric photographed actually comes apart.
    #[test]
    fn propose_split_recovers_lost_spaces() {
        assert_eq!(
            propose_split("en", "Thisisanexample"),
            Some(vec!["this".into(), "is".into(), "an".into(), "example".into()])
        );
        assert_eq!(
            propose_split("en", "goodmorning"),
            Some(vec!["good".into(), "morning".into()])
        );
        // fewest pieces wins — never shredded into more than needed
        assert_eq!(
            propose_split("en", "onetwothree"),
            Some(vec!["one".into(), "two".into(), "three".into()])
        );
    }

    /// A real word is not a smash-up, however splittable it looks.
    /// "notebook" IS note+book, and both are bank words — the guard that
    /// stops it is the whole-word bank check, not the segmenter.
    #[test]
    fn propose_split_leaves_real_words_alone() {
        for w in ["example", "notebook", "together", "understand", "morning"] {
            assert_eq!(propose_split("en", w), None, "{w} is already a word");
        }
    }

    /// F14/D7 — WHY this only proposes. A prototype over the real bank
    /// mis-split 1 in 10 plausible custom words, and this is the one:
    /// a person's name becomes two nouns. The behaviour is CORRECT (the
    /// pieces really are bank words); the safeguard is that a human
    /// confirms. If this ever starts returning None, the segmenter got
    /// more conservative and D7's confirm step could be revisited.
    #[test]
    fn a_name_can_still_be_mis_split_which_is_why_a_human_confirms() {
        assert_eq!(
            propose_split("en", "Sundeep"),
            Some(vec!["sun".into(), "deep".into()]),
            "documented false positive — never auto-applied"
        );
    }

    /// The bank is the ceiling. `sat` and `mat` are missing from the EN
    /// bank (BD-G3), so this sentence cannot be recovered — and NO
    /// proposal is the right answer, not a partial or wrong one.
    #[test]
    fn an_unsegmentable_smash_up_proposes_nothing() {
        assert_eq!(propose_split("en", "thecatsatonthemat"), None);
        assert_eq!(propose_split("en", "zzzqqqxxx"), None);
        assert_eq!(propose_split("en", "ab"), None, "too short to be two pieces");
        let long = "a".repeat(40);
        assert_eq!(propose_split("en", &long), None, "past the length cap");
    }

    fn words(tokens: &[&str]) -> Vec<(String, bool)> {
        tokens.iter().map(|t| (t.to_string(), false)).collect()
    }

    /// Acceptance #1 (shape): a clean printed English list classifies as
    /// in-dictionary chips with zero junk surviving unfiltered.
    #[test]
    fn a1_printed_english_words_classify_in_dictionary() {
        let out = classify("en", &words(&["horse", "mouse", "tree"]));
        assert!(out.iter().all(|c| c.class == WordClass::InDictionary), "{out:?}");
    }

    /// Out-of-bank but clean → Custom (the player's own word), never dropped.
    #[test]
    fn unknown_clean_word_is_custom() {
        let out = classify("en", &words(&["zzblorp"]));
        assert_eq!(out[0].class, WordClass::Custom);
    }

    /// Acceptance #4: a profanity token classifies Filtered (flagged in review,
    /// blocked from save by the shared gate) — not silently absent.
    #[test]
    fn a4_profanity_classifies_filtered() {
        let out = classify("en", &words(&["shit"]));
        assert_eq!(out.len(), 1, "filtered candidates are carried, not dropped");
        assert_eq!(out[0].class, WordClass::Filtered);
    }

    /// Acceptance #3: Spanish diacritics — a DECOMPOSED (NFD) OCR form matches
    /// the bank and the candidate text stays the composed NFC form byte-for-byte.
    #[test]
    fn a3_spanish_diacritics_roundtrip_nfc_byte_identical() {
        // "niño" typed as n-i-n-combining tilde-o (what OCR can emit).
        let nfd = "nin\u{0303}o";
        // The parser NFC-normalizes upstream; mirror that here.
        let parsed = crate::native_lang::parse_candidates(&[nfd.to_string()]);
        assert_eq!(parsed, vec!["niño".to_string()], "parser composes to NFC");
        let out = classify("es", &words(&[&parsed[0]]));
        assert_eq!(out[0].class, WordClass::InDictionary, "NFD form found in the es bank");
        assert_eq!(out[0].word.as_bytes(), "niño".as_bytes(), "byte-identical NFC round-trip");
    }

    /// The classification is study-language-scoped: a Spanish bank word is
    /// Custom under English (and vice versa) — no cross-language bleed.
    #[test]
    fn classification_is_study_language_scoped() {
        assert_eq!(classify("en", &words(&["araña"]))[0].class, WordClass::Custom);
        assert_eq!(classify("es", &words(&["araña"]))[0].class, WordClass::InDictionary);
    }

    /// The low-confidence bit is transported untouched — carried, never a drop
    /// signal (spec: nothing is dropped for low confidence).
    #[test]
    fn low_confidence_is_carried_not_dropped() {
        let out = classify("en", &[("horse".to_string(), true)]);
        assert_eq!(out.len(), 1);
        assert!(out[0].confidence_low);
        assert_eq!(out[0].class, WordClass::InDictionary);
    }

    /// Phase 2: line confidence maps onto every token from that line; dedupe
    /// spans lines (first occurrence wins) and junk lines contribute nothing.
    #[test]
    fn extract_classified_maps_line_confidence_and_dedupes() {
        let lines = vec![
            ("1. horse  mouse".to_string(), 0.95_f32),
            ("tree".to_string(), 0.2),
            ("HORSE".to_string(), 0.2),  // dup of line-1 horse — dropped
            ("###".to_string(), 0.9),    // junk line — no tokens
        ];
        let out = extract_classified("en", &lines);
        let words: Vec<&str> = out.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(words, vec!["horse", "mouse", "tree"]);
        assert!(!out[0].confidence_low && !out[1].confidence_low, "clean line is not low");
        assert!(out[2].confidence_low, "0.2 line is below LOW_CONFIDENCE");
        assert!(out.iter().all(|c| c.class == WordClass::InDictionary));
    }

    /// Mandarin "pinyin|hanzi" entries match from either side of the bar.
    #[test]
    fn mandarin_matches_pinyin_or_hanzi() {
        let bank = crate::words::tier_for("zh", "easy");
        let Some(entry) = bank.iter().find(|e| e.contains('|')) else {
            panic!("zh easy bank has no pinyin|hanzi entry");
        };
        let (pinyin, hanzi) = entry.split_once('|').unwrap();
        assert_eq!(classify("zh", &words(&[pinyin]))[0].class, WordClass::InDictionary);
        assert_eq!(classify("zh", &words(&[hanzi]))[0].class, WordClass::InDictionary);
    }
}
