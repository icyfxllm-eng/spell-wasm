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

#[cfg(test)]
mod tests {
    use super::*;

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
