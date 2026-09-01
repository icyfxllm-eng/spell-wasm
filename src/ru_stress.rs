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

/// The audited index for a word, or None if it has no coverage.
///
/// Returns None for EVERYTHING while `ru_stress_data::AUDITED` is false. The
/// table is present and diffable but inert: Wiktionary pre-fill is a good
/// source, not an audit, and a wrong mark teaches a wrong word.
///
/// None is a legitimate answer, not a gap to paper over: I5 excludes an
/// uncovered entry from selection rather than rendering it bare, so a partial
/// audit ships correctly instead of half-working. Monosyllables are None by
/// rule; so is anything the auditor left blank, including the 112 spellings
/// Wiktionary gives more than one stress for.
pub fn stress_index(word: &str) -> Option<u8> {
    if !crate::ru_stress_data::AUDITED {
        return None;
    }
    crate::ru_stress_data::RU_STRESS
        .binary_search_by(|(k, _)| (*k).cmp(word))
        .ok()
        .map(|i| crate::ru_stress_data::RU_STRESS[i].1)
}

/// The display form for a covered word: `молоко` -> `молоко́`.
/// None where there is no coverage, which is the same thing as "do not mark".
pub fn marked(word: &str) -> Option<String> {
    mark_stress(word, stress_index(word)? as usize)
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
    /// The audited table must satisfy I1 and I4 for EVERY row, not just the
    /// hand-picked examples below: strip the mark and the answer key comes
    /// back byte-identical. The ingest asserts this on the sheet; this asserts
    /// it on what actually shipped.
    #[test]
    fn the_shipped_table_round_trips_and_marks_only_vowels() {
        let n = crate::ru_stress_data::RU_STRESS.len();
        assert!(n > 0, "no stress data shipped");
        let mut prev = "";
        for (word, i) in crate::ru_stress_data::RU_STRESS.iter() {
            assert!(*word > prev, "table must be sorted for binary search: {prev} then {word}");
            prev = word;
            let ch = word.chars().nth(*i as usize)
                .unwrap_or_else(|| panic!("{word}: index {i} is past the end"));
            assert!(VOWELS.contains(ch), "{word}: index {i} is {ch:?}, not a vowel");
            let m = mark_stress(word, *i as usize).expect("covered word must mark");
            assert_eq!(strip_stress(&m), *word, "{word}: round trip changed the answer key");
            assert_eq!(m.chars().filter(|c| *c == ACUTE).count(), 1, "{word}: not exactly one mark");
            if crate::ru_stress_data::AUDITED {
                assert_eq!(stress_index(word), Some(*i), "{word}: lookup disagrees with the table");
            }
        }
        assert!(stress_index("нетакогослова").is_none(), "an unknown word must be None");
    }

    /// The dark rule, asserted rather than trusted. This is the test that will
    /// FAIL the day someone flips the claim, which is the point: flipping it
    /// must be a deliberate act that shows up in a diff, not a default.
    #[test]
    fn the_table_is_dark_until_a_human_signs_off() {
        let covered = crate::ru_stress_data::RU_STRESS.first().map(|(w, _)| *w);
        if crate::ru_stress_data::AUDITED {
            assert_eq!(stress_index(covered.unwrap()), Some(crate::ru_stress_data::RU_STRESS[0].1),
                       "audited: the lookup must serve the table");
        } else {
            assert!(stress_index(covered.unwrap()).is_none(),
                    "unaudited: every lookup must be None, even for a word in the table");
            assert!(marked(covered.unwrap()).is_none(), "unaudited: nothing may be marked");
        }
    }

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
