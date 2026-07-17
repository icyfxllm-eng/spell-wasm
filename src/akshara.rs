//! CC-HINDI-PHASE0 F1 — akshara segmentation for Devanagari.
//!
//! # What an akshara is, and why it is the unit
//! Devanagari is an abugida: the thing a reader perceives as "a letter" is an
//! **akshara** — a consonant cluster plus its vowel, which may span several
//! codepoints and render as one inseparable shape. `क` + `्` + `ष` is three
//! codepoints and ONE letter: क्ष. A player who types those three must watch the
//! conjunct form live and be judged on the akshara they produced — never shown a
//! broken half-cluster, never marked wrong on a codepoint they cannot perceive.
//!
//! # D3: one segmentation source of truth
//! The unit is the **extended grapheme cluster** (UAX #29), segmented here in the
//! Rust core and exposed to the frontend. No JS-side segmentation anywhere: two
//! segmenters would eventually disagree, and the one that disagreed would be the
//! one deciding whether a child got a word right. This is the Devanagari instance
//! of the Hangul-jamo decision (`jamo::grade`).
//!
//! # D4: NFC baseline, nuqta decomposed
//! Input is normalized to NFC before segmenting. The precomposed nuqta letters
//! U+0958–U+095F are **composition-excluded**, so NFC does not produce them — it
//! yields base + U+093C. Canonical storage is therefore the decomposed form, and
//! `scripts/devanagari-check.mjs` rejects any word list containing the
//! precomposed range (F3).

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// One akshara: boundaries into the NFC form, plus that cluster's canonical bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    /// Byte offset of the cluster's first codepoint in the NFC-normalized word.
    pub start: usize,
    /// Byte offset one past the cluster's last codepoint.
    pub end: usize,
    /// The cluster's canonical text — what one feedback tile renders.
    pub text: String,
}

impl Cluster {
    /// Codepoints in this cluster. `क्ष` is 1 akshara and 3 chars; the gap between
    /// those numbers is the whole reason this module exists.
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }
}

/// Segment `word` into aksharas (D3).
///
/// Normalizes to NFC first (D4), so boundaries are always relative to the
/// canonical form — a caller that indexes the raw input with these offsets is
/// asking for a mismatch, which is why [`Cluster::text`] carries the bytes rather
/// than expecting the caller to slice.
pub fn segment_aksharas(word: &str) -> Vec<Cluster> {
    let nfc: String = word.nfc().collect();
    nfc.grapheme_indices(true)
        .map(|(start, g)| Cluster { start, end: start + g.len(), text: g.to_string() })
        .collect()
}

/// The NFC form `segment_aksharas` works over — the canonical storage form (D4).
pub fn canonical(word: &str) -> String {
    word.nfc().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render a segmentation as `क्ष|मा` so a failure shows WHERE the boundary went.
    fn seg(word: &str) -> String {
        segment_aksharas(word).iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("|")
    }
    fn count(word: &str) -> usize {
        segment_aksharas(word).len()
    }

    // ---- the F1 fixture set ----

    #[test]
    fn plain_consonants_are_one_akshara_each() {
        assert_eq!(seg("कमल"), "क|म|ल", "three bare consonants, three aksharas");
        assert_eq!(count("कमल"), 3);
    }

    #[test]
    fn independent_vowels_are_one_akshara_each() {
        assert_eq!(seg("आम"), "आ|म");
        assert_eq!(seg("इधर"), "इ|ध|र");
    }

    #[test]
    fn consonant_plus_matra_is_ONE_akshara() {
        // का = क + ा (U+093E). The matra is a SpacingMark; UAX #29 GB9a keeps it
        // attached. Two codepoints, one letter.
        assert_eq!(seg("का"), "का");
        assert_eq!(count("का"), 1);
        assert_eq!(segment_aksharas("का")[0].char_count(), 2, "one akshara, two codepoints");
    }

    #[test]
    fn reordering_i_matra_stays_with_its_consonant() {
        // कि = क + ि (U+093F). STORED after the consonant, RENDERED before it —
        // the shaper does the visual swap; we segment LOGICAL order. If this ever
        // split, a tile would show a stranded ि with nothing to attach to.
        assert_eq!(seg("कि"), "कि");
        assert_eq!(count("कि"), 1);
        assert_eq!(segment_aksharas("कि")[0].char_count(), 2, "one akshara, two codepoints");
        // In a word: कि|ता|ब — the ि rides with its क, and ा with its त.
        assert_eq!(seg("किताब"), "कि|ता|ब");
        assert_eq!(count("किताब"), 3);
    }

    #[test]
    fn nuqta_is_part_of_its_akshara() {
        // ज़ = ज + ़ (U+093C, Extend). One letter.
        assert_eq!(seg("ज़"), "ज़");
        assert_eq!(count("ज़"), 1);
        assert_eq!(segment_aksharas("ज़")[0].char_count(), 2);
    }

    #[test]
    fn anusvara_and_candrabindu_attach() {
        assert_eq!(count("अं"), 1, "anusvara ं attaches");
        assert_eq!(count("अँ"), 1, "candrabindu ँ attaches");
    }

    #[test]
    fn visarga_attaches() {
        assert_eq!(count("अः"), 1, "visarga ः attaches");
    }

    /// THE question F1 exists to answer. If UAX #29 does not group Indic conjuncts,
    /// D3's premise fails and Devanagari needs custom segmentation.
    #[test]
    fn two_consonant_conjunct_is_ONE_akshara() {
        // क्ष = क + ् (virama) + ष. Renders as a single conjunct glyph.
        assert_eq!(seg("क्ष"), "क्ष", "conjunct must not be split at the virama");
        assert_eq!(count("क्ष"), 1);
        assert_eq!(segment_aksharas("क्ष")[0].char_count(), 3, "one akshara, three codepoints");
    }

    #[test]
    fn three_consonant_conjunct_is_ONE_akshara() {
        // स्त्री = स + ् + त + ् + र + ी
        assert_eq!(count("स्त्री"), 1, "a 3-consonant conjunct + matra is one akshara");
    }

    // ---- the F2 word set, segmented ----

    #[test]
    fn f2_word_set_segments_without_bisecting_a_cluster() {
        // Recorded as the source of truth for the F2 render prototype: one tile
        // per akshara here means one tile per akshara there.
        for (word, expect) in [
            ("क्षमा", "क्ष|मा"),
            ("प्रश्न", "प्र|श्न"),
            ("स्त्री", "स्त्री"),
            ("हिन्दी", "हि|न्दी"),
            ("ज़रूरी", "ज़|रू|री"),
            ("किताब", "कि|ता|ब"),
            ("अँधेरा", "अँ|धे|रा"),
        ] {
            assert_eq!(seg(word), expect, "{word} segmented wrong");
        }
    }

    // ---- D4 ----

    #[test]
    fn nfc_decomposes_the_precomposed_nuqta_letters() {
        // U+095B (ज़) is composition-excluded: NFC yields ज + ़ (U+093C), NOT the
        // precomposed letter. This is why canonical storage is decomposed and why
        // the F3 CI gate can reject U+0958–U+095F outright.
        let pre = "\u{095B}";
        let canon = canonical(pre);
        assert_eq!(canon.chars().count(), 2, "NFC must decompose U+095B");
        assert_eq!(canon, "\u{091C}\u{093C}", "ज + nuqta");
        // ...and it still segments as ONE akshara.
        assert_eq!(count(pre), 1);
    }

    #[test]
    fn segmentation_is_normalization_stable() {
        // The same word, precomposed vs decomposed, must segment identically —
        // otherwise two keyboards could produce different tile counts for the same
        // word (which is exactly what F4 checks at the device level).
        assert_eq!(seg("\u{095B}\u{0930}"), seg("\u{091C}\u{093C}\u{0930}"));
    }

    #[test]
    fn boundaries_index_the_canonical_form_and_reassemble_it() {
        for word in ["क्षमा", "हिन्दी", "ज़रूरी", "किताब"] {
            let canon = canonical(word);
            let cs = segment_aksharas(word);
            // Boundaries must tile the canonical string exactly: no gaps, no overlap.
            let mut at = 0;
            for c in &cs {
                assert_eq!(c.start, at, "{word}: gap or overlap at {at}");
                assert_eq!(&canon[c.start..c.end], c.text, "{word}: text disagrees with its own boundaries");
                at = c.end;
            }
            assert_eq!(at, canon.len(), "{word}: boundaries must cover the whole word");
            assert_eq!(cs.iter().map(|c| c.text.as_str()).collect::<String>(), canon);
        }
    }

    #[test]
    fn empty_and_ascii_are_sane() {
        assert!(segment_aksharas("").is_empty());
        assert_eq!(seg("cat"), "c|a|t", "the API is not Devanagari-only; it is UAX #29");
    }
}
