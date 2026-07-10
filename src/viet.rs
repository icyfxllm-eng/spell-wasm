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

    #[test]
    fn non_vowel_is_ignored() {
        assert_eq!(retone('b', GRAVE), None);
        assert_eq!(retone('đ', GRAVE), None);
    }
}
