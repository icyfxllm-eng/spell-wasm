//! F-X4 — a hint never gives the answer away (I10).
//!
//! A hint may help the player hear or understand the word. It may never show
//! the word itself in any case or accent, nor any run of its letters long
//! enough to spell it for them: 4 graphemes, or 3 for a target under 5. Any
//! inflected form of the word shares such a run with it, so this blocks those
//! too. A definition that fails is not rewritten: that word simply offers no
//! definition hint.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Lowercase with every accent and diacritic removed, so "Café" and "cafe"
/// compare equal.
fn bare(s: &str) -> String {
    s.to_lowercase().nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

/// Does `text` pass the filter for `target`?
pub fn passes(target: &str, text: &str) -> bool {
    let t: Vec<String> = bare(target).graphemes(true).map(str::to_string).collect();
    if t.is_empty() {
        return true;
    }
    let h = bare(text);
    let k = if t.len() < 5 { 3 } else { 4 };
    if t.len() <= k {
        return !h.contains(&t.concat());
    }
    t.windows(k).all(|w| !h.contains(&w.concat()))
}
