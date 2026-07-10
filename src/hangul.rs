//! Hangul composition automaton. The Korean keyboard emits *compatibility jamo*
//! (ㄱ, ㅏ, …); this module composes a stream of them into precomposed syllable
//! blocks live as the player types — exactly like a real 2-set (Dubeolsik) IME —
//! and decomposes them one jamo at a time on backspace.
//!
//! Public API operates on the whole answer string (like the Vietnamese tone
//! engine): [`feed`] applies one jamo, [`backspace`] removes one jamo. Both are
//! total — any input sequence yields valid NFC Hangul (plus at most a trailing
//! lone jamo for an in-progress syllable); they never panic.
//!
//! Syllable math: `U+AC00 + (initial×21 + medial)×28 + final`, with 19 initials,
//! 21 medials, and 28 finals (index 0 = no final).
//!
//! The composition engine ships ahead of the Korean keyboard/word lists that
//! consume it, so its public surface is allowed to be unused for now.
#![allow(dead_code)]

const SBASE: u32 = 0xAC00;
const SLAST: u32 = 0xD7A3;

/// 19 initial consonants (choseong), keyed by their compatibility jamo.
const INITIALS: [char; 19] = ['ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ'];
/// 21 medial vowels (jungseong).
const MEDIALS: [char; 21] = ['ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ'];
/// 28 finals (jongseong); index 0 is "no final" (sentinel '\0').
const FINALS: [char; 28] = ['\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ'];

/// Compound medials: (first, second) -> combined.
const COMPOUND_MEDIAL: [(char, char, char); 7] = [
    ('ㅗ', 'ㅏ', 'ㅘ'), ('ㅗ', 'ㅐ', 'ㅙ'), ('ㅗ', 'ㅣ', 'ㅚ'),
    ('ㅜ', 'ㅓ', 'ㅝ'), ('ㅜ', 'ㅔ', 'ㅞ'), ('ㅜ', 'ㅣ', 'ㅟ'),
    ('ㅡ', 'ㅣ', 'ㅢ'),
];
/// Compound finals: (first, second) -> combined.
const COMPOUND_FINAL: [(char, char, char); 11] = [
    ('ㄱ', 'ㅅ', 'ㄳ'), ('ㄴ', 'ㅈ', 'ㄵ'), ('ㄴ', 'ㅎ', 'ㄶ'),
    ('ㄹ', 'ㄱ', 'ㄺ'), ('ㄹ', 'ㅁ', 'ㄻ'), ('ㄹ', 'ㅂ', 'ㄼ'),
    ('ㄹ', 'ㅅ', 'ㄽ'), ('ㄹ', 'ㅌ', 'ㄾ'), ('ㄹ', 'ㅍ', 'ㄿ'),
    ('ㄹ', 'ㅎ', 'ㅀ'), ('ㅂ', 'ㅅ', 'ㅄ'),
];

fn idx(table: &[char], c: char) -> Option<usize> {
    table.iter().position(|&x| x == c)
}

pub fn is_vowel(c: char) -> bool {
    MEDIALS.contains(&c)
}

pub fn is_consonant(c: char) -> bool {
    INITIALS.contains(&c) || FINALS[1..].contains(&c)
}

fn compose(i: usize, m: usize, f: usize) -> char {
    char::from_u32(SBASE + ((i * 21 + m) * 28 + f) as u32).unwrap()
}

/// Decompose a precomposed syllable into (initial, medial, final) table indices.
fn decompose(c: char) -> Option<(usize, usize, usize)> {
    let u = c as u32;
    if !(SBASE..=SLAST).contains(&u) {
        return None;
    }
    let s = (u - SBASE) as usize;
    Some((s / (21 * 28), (s / 28) % 21, s % 28))
}

fn compound_medial(a: char, b: char) -> Option<char> {
    COMPOUND_MEDIAL.iter().find(|(x, y, _)| *x == a && *y == b).map(|(_, _, z)| *z)
}
fn split_medial(c: char) -> Option<(char, char)> {
    COMPOUND_MEDIAL.iter().find(|(_, _, z)| *z == c).map(|(x, y, _)| (*x, *y))
}
fn compound_final(a: char, b: char) -> Option<char> {
    COMPOUND_FINAL.iter().find(|(x, y, _)| *x == a && *y == b).map(|(_, _, z)| *z)
}
fn split_final(c: char) -> Option<(char, char)> {
    COMPOUND_FINAL.iter().find(|(_, _, z)| *z == c).map(|(x, y, _)| (*x, *y))
}

/// State of the last (in-progress) grapheme in the buffer.
enum Last {
    None,
    Init(usize),            // lone initial consonant jamo
    Vowel(char),            // lone vowel jamo
    Im(usize, usize),       // initial + medial (no final)
    Imf(usize, usize, char), // initial + medial + final (final as char)
}

/// Parse the last char of `s`, returning (rest-without-last, state). A trailing
/// non-Hangul char leaves state None (composition starts fresh after it).
fn parse_last(s: &str) -> (&str, Last) {
    let Some(last) = s.chars().last() else {
        return (s, Last::None);
    };
    let rest = &s[..s.len() - last.len_utf8()];
    if let Some((i, m, f)) = decompose(last) {
        return if f == 0 { (rest, Last::Im(i, m)) } else { (rest, Last::Imf(i, m, FINALS[f])) };
    }
    if let Some(i) = idx(&INITIALS, last) {
        return (rest, Last::Init(i));
    }
    if is_vowel(last) {
        return (rest, Last::Vowel(last));
    }
    (s, Last::None) // last char isn't composable Hangul
}

fn syl(i: usize, m: usize, f: char) -> char {
    compose(i, m, if f == '\0' { 0 } else { idx(&FINALS, f).unwrap_or(0) })
}

/// Apply one compatibility jamo to the answer buffer, composing Hangul.
pub fn feed(answer: &str, jamo: char) -> String {
    let (rest, last) = parse_last(answer);
    let mut out = rest.to_string();
    let vowel = is_vowel(jamo);

    match (last, vowel) {
        // ---- vowel input ----
        (Last::Init(i), true) => {
            // lone initial + vowel -> CV syllable
            out.push(syl(i, idx(&MEDIALS, jamo).unwrap(), '\0'));
        }
        (Last::Im(i, m), true) => {
            // try to grow the medial into a compound (ㅗ+ㅏ->ㅘ)
            if let Some(c) = compound_medial(MEDIALS[m], jamo) {
                out.push(syl(i, idx(&MEDIALS, c).unwrap(), '\0'));
            } else {
                out.push(syl(i, m, '\0'));
                out.push(jamo); // starts a new lone vowel
            }
        }
        (Last::Imf(i, m, f), true) => {
            // the final steals: it becomes the initial of a new syllable
            let (keep, steal) = match split_final(f) {
                Some((a, b)) => (a, b), // compound: only the last part moves
                None => ('\0', f),
            };
            out.push(syl(i, m, keep));
            let si = idx(&INITIALS, steal).unwrap_or(idx(&INITIALS, 'ㅇ').unwrap());
            out.push(syl(si, idx(&MEDIALS, jamo).unwrap(), '\0'));
        }
        (Last::Vowel(v), true) => {
            if let Some(c) = compound_medial(v, jamo) {
                out.push(c);
            } else {
                out.push(v);
                out.push(jamo);
            }
        }
        (Last::None, true) => out.push(jamo),

        // ---- consonant input ----
        (Last::Im(i, m), false) => {
            // CV + consonant -> add a final
            out.push(syl(i, m, jamo));
        }
        (Last::Imf(i, m, f), false) => {
            // CVC + consonant -> grow into a compound final, else new syllable
            if let Some(c) = compound_final(f, jamo) {
                out.push(syl(i, m, c));
            } else {
                out.push(syl(i, m, f));
                out.push(jamo);
            }
        }
        (Last::Init(i), false) => {
            out.push(INITIALS[i]);
            out.push(jamo);
        }
        (Last::Vowel(v), false) => {
            out.push(v);
            out.push(jamo);
        }
        (Last::None, false) => out.push(jamo),
    }
    out
}

/// Remove one jamo from the end (stepwise decomposition):
/// 값 -> 갑 -> 가 -> ㄱ -> "".
pub fn backspace(answer: &str) -> String {
    let (rest, last) = parse_last(answer);
    let mut out = rest.to_string();
    match last {
        Last::Imf(i, m, f) => {
            // drop the final (compound -> reduce to first component)
            match split_final(f) {
                Some((a, _)) => out.push(syl(i, m, a)),
                None => out.push(syl(i, m, '\0')),
            }
        }
        Last::Im(i, m) => {
            // drop the medial (compound -> reduce to first), leaving the initial
            match split_medial(MEDIALS[m]) {
                Some((a, _)) => out.push(syl(i, idx(&MEDIALS, a).unwrap(), '\0')),
                None => out.push(INITIALS[i]),
            }
        }
        Last::Init(_) | Last::Vowel(_) => { /* drop the lone jamo entirely */ }
        Last::None => {
            // non-Hangul trailing char: normal char delete
            let mut chars: Vec<char> = answer.chars().collect();
            chars.pop();
            return chars.into_iter().collect();
        }
    }
    out
}

/// True if the string ends in an incomplete syllable (a lone trailing jamo),
/// which submit should reject with "incomplete", not a wrong-answer penalty.
pub fn ends_incomplete(s: &str) -> bool {
    matches!(s.chars().last(), Some(c) if is_consonant(c) || is_vowel(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_seq(seq: &str) -> String {
        seq.chars().fold(String::new(), |acc, c| feed(&acc, c))
    }

    #[test]
    fn composes_hanguk() {
        // ㅎㅏㄴㄱㅜㄱ -> 한국
        assert_eq!(type_seq("ㅎㅏㄴㄱㅜㄱ"), "한국");
    }

    #[test]
    fn final_steals_when_vowel_follows() {
        // 가 + ㄱ + ㅏ -> 가가 (not 각 + ㅏ)
        assert_eq!(type_seq("ㄱㅏㄱㅏ"), "가가");
    }

    #[test]
    fn compound_vowel() {
        // ㄱㅗㅏ -> 과
        assert_eq!(type_seq("ㄱㅗㅏ"), "과");
    }

    #[test]
    fn compound_final_then_steal() {
        // ㄱㅏㅂㅅ -> 값 ; then ㅣ -> 갑 + 시
        assert_eq!(type_seq("ㄱㅏㅂㅅ"), "값");
        assert_eq!(type_seq("ㄱㅏㅂㅅㅣ"), "갑시");
    }

    #[test]
    fn backspace_decomposes_stepwise() {
        let s = type_seq("ㄱㅏㅂㅅ"); // 값
        assert_eq!(s, "값");
        let s = backspace(&s);
        assert_eq!(s, "갑");
        let s = backspace(&s);
        assert_eq!(s, "가");
        let s = backspace(&s);
        assert_eq!(s, "ㄱ");
        let s = backspace(&s);
        assert_eq!(s, "");
    }

    #[test]
    fn compound_vowel_backspace_reduces() {
        let s = type_seq("ㄱㅗㅏ"); // 과
        assert_eq!(backspace(&s), "고");
    }

    #[test]
    fn incomplete_detection() {
        assert!(ends_incomplete(&type_seq("ㅎ"))); // lone consonant
        assert!(ends_incomplete(&type_seq("ㅏ"))); // lone vowel
        assert!(!ends_incomplete(&type_seq("ㄱㅏ"))); // 가 is complete
    }

    #[test]
    fn every_precomposed_syllable_round_trips() {
        // decompose(compose(i,m,f)) is the identity across the whole block.
        for i in 0..19 {
            for m in 0..21 {
                for f in 0..28 {
                    let (di, dm, df) = decompose(compose(i, m, f)).unwrap();
                    assert_eq!((di, dm, df), (i, m, f));
                }
            }
        }
    }

    #[test]
    fn fuzz_never_panics_and_stays_valid() {
        // Deterministic PRNG over the jamo the keyboard can emit.
        let jamos: Vec<char> = INITIALS.iter().chain(MEDIALS.iter()).copied().collect();
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..20_000 {
            let mut s = String::new();
            for _ in 0..(next() % 12) {
                if next() % 5 == 0 && !s.is_empty() {
                    s = backspace(&s);
                } else {
                    let j = jamos[(next() as usize) % jamos.len()];
                    s = feed(&s, j);
                }
            }
            // Every char must be a Hangul syllable or a compatibility jamo.
            for c in s.chars() {
                let u = c as u32;
                let ok = (SBASE..=SLAST).contains(&u) || is_consonant(c) || is_vowel(c);
                assert!(ok, "invalid char {c:?} produced by sequence -> {s:?}");
            }
        }
    }
}
