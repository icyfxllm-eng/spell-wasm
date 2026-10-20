//! Vietnamese tone composition for the on-screen keyboard. The player types a
//! base or letter-modified vowel (a / ă / â / e / ê / o / ô / ơ / u / ư / y) and
//! then taps a tone key; [`retone`] applies that tone to the vowel, replacing any
//! existing tone (tapping the same tone again removes it). This mirrors how a
//! real Vietnamese IME works and keeps every one of the ~90 vowel forms reachable
//! in ≤2 taps without a giant keyboard.

use unicode_normalization::UnicodeNormalization;

/// Combining marks that are part of the *letter* (kept when re-toning).
const LETTER_MARKS: [char; 3] = ['\u{302}', '\u{306}', '\u{31b}']; // circumflex, breve, horn
/// The five Vietnamese tone marks (applied/removed by the tone keys).
pub const TONE_MARKS: [char; 5] = ['\u{300}', '\u{301}', '\u{309}', '\u{303}', '\u{323}']; // huyền sắc hỏi ngã nặng

/// Apply `tone` (a combining mark from [`TONE_MARKS`]) to vowel `c`, returning
/// the recomposed grapheme. Returns `None` if `c` isn't a Vietnamese vowel.
/// Re-applying the tone already present removes it (toggle).
pub fn retone(c: char, tone: char) -> Option<String> {
    let decomp: Vec<char> = c.nfd().collect();
    let base = *decomp.first()?;
    if !"aeiouy".contains(base.to_ascii_lowercase()) {
        return None;
    }
    let letter: Vec<char> = decomp.iter().skip(1).filter(|m| LETTER_MARKS.contains(m)).copied().collect();
    let had_tone = decomp.iter().any(|m| *m == tone);
    let mut s: String = std::iter::once(base).chain(letter).collect();
    if !had_tone {
        s.push(tone);
    }
    Some(s.nfc().collect())
}

/// Grading key that makes tone PLACEMENT irrelevant where Vietnamese has two
/// accepted conventions (signing sheet K-Q2, Eric 2026-10-20: accept both).
///
/// Only an OPEN syllable ending in a plain oa / oe / uy carries the choice: hóa or
/// hoá, khỏe or khoẻ, thủy or thuỷ. Closed syllables (hoàng, hoạch) and longer
/// nuclei (ngoài) have one correct position under both conventions, so a tone
/// on the first vowel there stays a misspelling. The key moves a tone from the
/// first vowel of a word-final pair to the second and touches nothing else. It
/// never changes WHICH tone, never adds or drops one, and leaves letter-marked
/// vowels (ô ơ ư ...) alone, because those are spelling.
///
/// Expects ONE whitespace-delimited token, already folded (NFC, lowercase). A
/// token typed without spaces only has its last syllable keyed; the caller keeps
/// the plain comparison too, so nothing that matched before can stop matching.
pub fn placement_key(token: &str) -> String {
    // (base, letter marks, tone) per character, from NFD.
    let mut cs: Vec<(char, Vec<char>, Option<char>)> = token
        .chars()
        .map(|c| {
            let d: Vec<char> = c.nfd().collect();
            let letter = d[1..].iter().filter(|m| !TONE_MARKS.contains(m)).copied().collect();
            let tone = d[1..].iter().find(|m| TONE_MARKS.contains(m)).copied();
            (d[0], letter, tone)
        })
        .collect();
    let n = cs.len();
    if n >= 2 {
        let (i, j) = (n - 2, n - 1);
        let pair = matches!((cs[i].0, cs[j].0), ('o', 'a') | ('o', 'e') | ('u', 'y'));
        let plain = cs[i].1.is_empty() && cs[j].1.is_empty();
        if pair && plain && cs[i].2.is_some() && cs[j].2.is_none() {
            cs[j].2 = cs[i].2.take();
        }
    }
    cs.into_iter()
        .flat_map(|(b, l, t)| std::iter::once(b).chain(l).chain(t))
        .collect::<String>()
        .nfc()
        .collect()
}

/// A whole answer keyed syllable by syllable, then joined without spaces (spaces
/// are not graded, `norm::fold_strict`).
pub fn answer_key(s: &str) -> String {
    s.split_whitespace().map(|t| placement_key(&crate::norm::fold_strict(t))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    const GRAVE: char = '\u{300}';
    const ACUTE: char = '\u{301}';
    const DOT: char = '\u{323}';

    #[test]
    fn tones_on_plain_vowel() {
        assert_eq!(retone('a', GRAVE).as_deref(), Some("à"));
        assert_eq!(retone('a', ACUTE).as_deref(), Some("á"));
    }

    #[test]
    fn tone_on_letter_modified_vowel_keeps_the_letter() {
        // â (a+circumflex) + dot below -> ậ
        assert_eq!(retone('â', DOT).as_deref(), Some("ậ"));
        // ơ (o+horn) + grave -> ờ
        assert_eq!(retone('ơ', GRAVE).as_deref(), Some("ờ"));
    }

    #[test]
    fn re_toning_replaces_not_stacks() {
        // à then acute -> á (not a with two tones)
        let a_grave = retone('a', GRAVE).unwrap();
        assert_eq!(retone(a_grave.chars().next().unwrap(), ACUTE).as_deref(), Some("á"));
    }

    #[test]
    fn same_tone_toggles_off() {
        let a_grave = retone('a', GRAVE).unwrap();
        assert_eq!(retone(a_grave.chars().next().unwrap(), GRAVE).as_deref(), Some("a"));
    }

    fn key(s: &str) -> String {
        answer_key(s)
    }

    #[test]
    fn both_placement_conventions_share_a_key() {
        for (a, b) in [
            ("hóa", "hoá"),
            ("hàng hóa", "hàng hoá"),
            ("văn hóa", "văn hoá"),
            ("sức khỏe", "sức khoẻ"),
            ("thủy", "thuỷ"),
            ("tùy", "tuỳ"),
            ("họa", "hoạ"),
            ("thỏa", "thoả"),
            ("hòa", "hoà"),
        ] {
            assert_eq!(key(a), key(b), "{a} / {b}");
        }
    }

    #[test]
    fn placement_key_never_merges_a_different_tone_or_letter() {
        assert_ne!(key("hóa"), key("hòa"), "different tone");
        assert_ne!(key("hóa"), key("hoa"), "tone dropped");
        assert_ne!(key("hóa"), key("hốa"), "letter mark is spelling");
        assert_ne!(key("thủy"), key("thùy"), "different tone");
        assert_ne!(key("tiếng"), key("tíêng"), "only oa/oe/uy pairs move");
        assert_ne!(key("mùa"), key("muà"), "ua has one placement");
        assert_ne!(key("hai"), key("hái"));
        assert_ne!(key("ngoài"), key("ngòai"), "oai: the tone is always on a");
        assert_ne!(key("hoàng"), key("hòang"), "closed syllable: one placement");
        assert_ne!(key("thú y"), key("thuý"), "a pair split across syllables is not one syllable");
    }

    #[test]
    fn single_placement_syllables_are_unchanged() {
        for w in ["hoàng", "hoạch", "ngoài", "quý", "khuya", "người", "chuyện", "tuyết"] {
            let f = crate::norm::fold_strict(w);
            assert_eq!(answer_key(w), f, "{w}");
        }
    }

    #[test]
    fn non_vowel_is_ignored() {
        assert_eq!(retone('b', GRAVE), None);
        assert_eq!(retone('đ', GRAVE), None);
    }
}
