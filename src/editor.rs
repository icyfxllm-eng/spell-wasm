//! Grapheme-cluster answer buffer with an in-word cursor (Feature 1).
//!
//! All string editing lives here in the Rust core — the JS layer only reports
//! tap positions and key events and never slices the word buffer. Editing
//! operates on **extended grapheme clusters** (UAX #29), never bytes / UTF-16 /
//! code points, so Vietnamese `ế`, Korean syllable blocks, and ZWJ emoji are
//! each one indivisible cell.
//!
//! Normalization policy (INV-8): every inserted string is normalized to **NFC**
//! at the boundary and then segmented into grapheme clusters. So decomposed
//! input (`e` + ◌̂ + ◌́) and its precomposed form (`ế`) behave identically — one
//! cell, deleted as a unit. Recorded in CLAUDE.md.
//!
//! Hangul backspace: one full grapheme cluster (a whole syllable block `학` →
//! gone). Live typing composes jamo incrementally via `hangul.rs` *before* it
//! reaches this buffer, so by the time a syllable is a cell here it deletes
//! whole — consistent with that composition granularity.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// A word being typed: a list of grapheme clusters plus a cursor in
/// `0..=grapheme_count`. Default cursor is end-of-buffer, so never tapping
/// behaves exactly like today's append/backspace-at-end.
#[derive(Clone, Debug, Default)]
pub struct EditorBuffer {
    graphemes: Vec<String>,
    cursor: usize,
    cap: Option<usize>,
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// A buffer with a max grapheme count (word-length cap). Insert at the cap is
    /// a no-op (INV-7), matching today's end-of-word behavior.
    pub fn with_cap(cap: usize) -> Self {
        Self { graphemes: Vec::new(), cursor: 0, cap: Some(cap) }
    }

    /// Build from existing text (cursor at end). Normalizes to NFC and segments.
    pub fn from_text(s: &str) -> Self {
        let graphemes = Self::split(s);
        let cursor = graphemes.len();
        Self { graphemes, cursor, cap: None }
    }

    fn split(s: &str) -> Vec<String> {
        let nfc: String = s.nfc().collect();
        nfc.graphemes(true).map(String::from).collect()
    }

    /// Insert `g` (one or more graphemes) at the cursor; the cursor advances past
    /// it. `g` is NFC-normalized then segmented, so each cluster is one cell.
    pub fn insert(&mut self, g: &str) {
        for cluster in Self::split(g) {
            if self.cap.is_some_and(|c| self.graphemes.len() >= c) {
                return; // INV-7: no-op at the cap
            }
            self.graphemes.insert(self.cursor, cluster);
            self.cursor += 1;
        }
    }

    /// Delete the single grapheme immediately before the cursor; the cursor moves
    /// back one. At position 0 it does nothing (INV-4).
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.graphemes.remove(self.cursor);
    }

    /// Set the cursor, clamped to `0..=grapheme_count` (INV-2).
    pub fn set_cursor(&mut self, idx: usize) {
        self.cursor = idx.min(self.graphemes.len());
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn grapheme_count(&self) -> usize {
        self.graphemes.len()
    }

    /// The grapheme cluster at `idx`, if any (for rendering tappable cells).
    pub fn grapheme_at(&self, idx: usize) -> Option<&str> {
        self.graphemes.get(idx).map(String::as_str)
    }

    /// Canonical buffer contents — what answer-checking reads (unchanged input).
    pub fn text(&self) -> String {
        self.graphemes.concat()
    }

    pub fn is_empty(&self) -> bool {
        self.graphemes.is_empty()
    }

    /// Replace the whole buffer (e.g. clear, or load a value), cursor to end.
    pub fn set_text(&mut self, s: &str) {
        self.graphemes = Self::split(s);
        self.cursor = self.graphemes.len();
    }

    pub fn clear(&mut self) {
        self.graphemes.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_only_matches_string_push() {
        // Never tapping (cursor stays at end) == today's push/pop.
        let mut b = EditorBuffer::new();
        for g in ["s", "t", "r", "a", "i", "t"] {
            b.insert(g);
        }
        assert_eq!(b.text(), "strait");
        assert_eq!(b.cursor(), 6);
    }

    #[test]
    fn cursor_edit_replaces_middle_grapheme() {
        let mut b = EditorBuffer::from_text("cot"); // want "cat"
        b.set_cursor(2); // after 'o' (cells: c o t)
        b.backspace(); // remove 'o' -> "ct", cursor 1
        b.insert("a"); // -> "cat", cursor 2
        assert_eq!(b.text(), "cat");
        assert_eq!(b.cursor(), 2);
    }

    #[test]
    fn backspace_at_zero_is_noop() {
        let mut b = EditorBuffer::from_text("cat");
        b.set_cursor(0);
        b.backspace();
        assert_eq!(b.text(), "cat");
        assert_eq!(b.cursor(), 0);
    }

    #[test]
    fn vietnamese_grapheme_is_one_cell() {
        // ế composed vs decomposed → one cell either way (NFC boundary policy).
        let mut b = EditorBuffer::new();
        b.insert("tr");
        b.insert("\u{1ebf}"); // ế precomposed
        b.insert("ng");
        let mut b2 = EditorBuffer::new();
        b2.insert("tr");
        b2.insert("e\u{302}\u{301}"); // ế decomposed
        b2.insert("ng");
        assert_eq!(b.text(), b2.text());
        assert_eq!(b.grapheme_count(), b2.grapheme_count());
        // backspace removes the whole ế, not just a mark
        b.set_cursor(3);
        b.backspace();
        assert_eq!(b.text(), "trng");
    }

    #[test]
    fn korean_syllable_deletes_whole() {
        let mut b = EditorBuffer::from_text("학교"); // 2 syllable cells
        assert_eq!(b.grapheme_count(), 2);
        b.backspace();
        assert_eq!(b.text(), "학");
    }

    #[test]
    fn cap_makes_insert_noop_at_limit() {
        let mut b = EditorBuffer::with_cap(3);
        for g in ["a", "b", "c", "d"] {
            b.insert(g);
        }
        assert_eq!(b.text(), "abc");
        assert_eq!(b.grapheme_count(), 3);
    }
}
