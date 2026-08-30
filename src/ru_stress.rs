//! CC-RUSSIAN-STRESS Phase 1 — the stress data layer.
//!
//! Russian orthography does not write stress, but pronunciation depends on it
//! completely: unstressed `о` reduces to [ɐ], unstressed `е`/`я` to [ɪ]. A
//! player who hears [məlɐˈko] has no acoustic basis for writing молоко over
//! малако. A flat prompt is not a quality problem, it is an unanswerable
//! question.
//!
//! CANONICAL STORAGE IS AN INDEX, NEVER A MARKED STRING (v2 F1). The answer key
//! stays byte-identical to what the player types, so no diacritic can reach
//! grading -- I1 and I2 hold by construction rather than by discipline. Marked
//! strings are derived here and nowhere else.
//!
//! THE INDEX IS A CHARACTER INDEX, NOT A BYTE OFFSET. The spec says "index into
//! the NFC-normalized answer string"; in Python that is a character, in Rust a
//! string index is a byte, and every Cyrillic letter is two bytes. Left
//! ambiguous that is a silent off-by-one across the whole bank, so it is stated
//! here and asserted below.

pub const VOWELS: &str = "аеёиоуыэюя";
const ACUTE: char = '\u{301}';

/// Insert U+0301 after the stressed vowel. `i` is a CHARACTER index.
///
/// None when the index does not point at a vowel. The Phase 0 probe learned
/// this the hard way: its first run indexed `м` and `к` in `замок`, produced
/// `зам́ок` and `замоќ`, measured an 87% difference and reported the voice
/// RESPONSIVE. It was measuring the engine's reaction to two nonsense
/// placements. A mark on a consonant measures nothing and must fail loudly.
pub fn mark_stress(word: &str, i: usize) -> Option<String> {
    let ch = word.chars().nth(i)?;
    if !VOWELS.contains(ch) {
        return None;
    }
    let mut out = String::with_capacity(word.len() + ACUTE.len_utf8());
    for (n, c) in word.chars().enumerate() {
        out.push(c);
        if n == i {
            out.push(ACUTE);
        }
    }
    Some(out)
}

/// Remove every U+0301. The only stripper; per single-source-of-truth doctrine
/// there is no inline `.replace('\u{301}', "")` anywhere else.
pub fn strip_stress(s: &str) -> String {
    s.chars().filter(|c| *c != ACUTE).collect()
}

/// `ё` is inherently stressed, so its index is forced. An entry carrying `ё`
/// with an index elsewhere is a data error, not a variant (v2 rule 3).
pub fn yo_index(word: &str) -> Option<usize> {
    word.chars().position(|c| c == 'ё')
}

/// Monosyllables take null, not a self-referential index (v2 rule 6).
pub fn is_monosyllabic(word: &str) -> bool {
    word.chars().filter(|c| VOWELS.contains(*c)).count() <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// I1 — no U+0301 survives into anything grading compares.
    #[test]
    fn i1_strip_undoes_mark_for_every_vowel_position() {
        for word in ["замок", "молодец", "молоко", "рука", "окно", "начал", "ёж", "своё"] {
            for (i, c) in word.chars().enumerate() {
                if !VOWELS.contains(c) {
                    assert!(mark_stress(word, i).is_none(), "{word}[{i}]={c} is not a vowel");
                    continue;
                }
                let marked = mark_stress(word, i).expect("vowel must mark");
                assert_eq!(strip_stress(&marked), word, "round trip failed for {word}[{i}]");
            }
        }
    }

    /// I4 — exactly one mark in a marked form, zero in a bare one.
    #[test]
    fn i4_exactly_one_mark() {
        let m = mark_stress("замок", 1).unwrap();
        assert_eq!(m.chars().filter(|c| *c == ACUTE).count(), 1);
        assert_eq!(strip_stress(&m).chars().filter(|c| *c == ACUTE).count(), 0);
    }

    /// The minimal pairs Phase 0 measured, spelled out so the indices in the
    /// probe and the indices here can never drift apart.
    #[test]
    fn the_phase_0_pairs_mark_where_the_probe_said() {
        assert_eq!(mark_stress("замок", 1).unwrap(), "за\u{301}мок");
        assert_eq!(mark_stress("замок", 3).unwrap(), "замо\u{301}к");
        assert_eq!(mark_stress("молодец", 1).unwrap(), "мо\u{301}лодец");
        // молодец = м0 о1 л2 о3 д4 е5 ц6 — index 5 is `е`, so the mark lands
        // after it. The first draft of this line expected "моло\u{301}дец",
        // which is the mark after index 3. The code was right and the
        // expectation was wrong, which is the correct way round for a test to
        // fail: an expectation written from memory instead of from the letters.
        assert_eq!(mark_stress("молодец", 5).unwrap(), "молоде\u{301}ц");
    }

    #[test]
    fn yo_is_inherently_stressed_and_monosyllables_take_null() {
        assert_eq!(yo_index("ёж"), Some(0));
        assert_eq!(yo_index("своё"), Some(3));
        assert_eq!(yo_index("замок"), None);
        assert!(is_monosyllabic("ёж"));
        assert!(is_monosyllabic("стол"));
        assert!(!is_monosyllabic("замок"));
        assert!(!is_monosyllabic("молоко"));
    }

    /// A CHARACTER index, not a byte offset. Every Cyrillic letter is two
    /// bytes, so a byte-indexed implementation would mark the wrong vowel or
    /// split a codepoint.
    #[test]
    fn the_index_is_characters_not_bytes() {
        // `замок` is 5 chars and 10 bytes. Char 3 is `о`; byte 3 is mid-`м`.
        assert_eq!("замок".chars().count(), 5);
        assert_eq!("замок".len(), 10);
        assert_eq!(mark_stress("замок", 3).unwrap(), "замо\u{301}к");
        assert!(mark_stress("замок", 7).is_none(), "past the end must be None, not a panic");
    }
}
