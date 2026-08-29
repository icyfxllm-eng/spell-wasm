//! CC-WORDPICTURE v7 F7 / D8 — spoken whole words are never a spelling
//! answer, in ANY mode.
//!
//! Two layers, because one is not enough on iOS:
//!  1. Suppression — every spelling surface sets `inputmode="none"`, so
//!     the system keyboard (and therefore its dictation mic) never opens.
//!     The base game already had this by construction: it renders its own
//!     keyboard and has no DOM input at all.
//!  2. Provenance — where a platform inserts dictated text anyway, the
//!     SUBMIT PATH rejects it. Typing arrives as single-character
//!     `insertText` events; dictation arrives as a multi-character chunk
//!     (`insertReplacementText`, `insertFromDictation`, or a long
//!     `insertText`). We count keystrokes per field and compare against
//!     the value length: a value longer than the keystrokes that built it
//!     did not come from a keyboard.
//!
//! Enforced once, at the single submit path — not per mode (D8).

use std::cell::RefCell;

thread_local! {
    /// field id -> (keystroke count, longest single insertion seen)
    static PROV: RefCell<Vec<(String, u32, u32)>> = const { RefCell::new(Vec::new()) };
}

/// Insert types that are dictation or paste, never a keystroke.
pub fn is_speech_input_type(t: &str) -> bool {
    matches!(
        t,
        "insertFromDictation" | "insertReplacementText" | "insertFromPaste" | "insertTranscription"
    )
}

/// Record one insertion on a field. `data_len` is the inserted length in
/// characters; `input_type` is the InputEvent inputType.
pub fn note_insert(field: &str, input_type: &str, data_len: u32) {
    let speech = is_speech_input_type(input_type);
    PROV.with(|p| {
        let mut v = p.borrow_mut();
        match v.iter_mut().find(|(f, _, _)| f == field) {
            Some(e) => {
                if speech || data_len > 1 {
                    e.2 = e.2.max(data_len.max(2));
                } else {
                    e.1 += 1;
                }
            }
            None => {
                let (k, chunk) = if speech || data_len > 1 { (0, data_len.max(2)) } else { (1, 0) };
                v.push((field.to_string(), k, chunk));
            }
        }
    });
}

pub fn reset(field: &str) {
    PROV.with(|p| p.borrow_mut().retain(|(f, _, _)| f != field));
}

/// True when the field's current value could not have been typed: a
/// chunk insertion was seen, or the value outran its keystrokes.
pub fn is_dictated(field: &str, value_len: u32) -> bool {
    PROV.with(|p| {
        match p.borrow().iter().find(|(f, _, _)| f == field) {
            Some((_, keys, chunk)) => *chunk >= 2 || value_len > *keys + 1,
            // no record at all for a non-empty value: it appeared without
            // a single keystroke — that is not typing either.
            None => value_len > 0,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh(f: &str) {
        reset(f);
    }

    #[test]
    fn typed_letters_are_accepted() {
        fresh("t1");
        for _ in 0..5 {
            note_insert("t1", "insertText", 1);
        }
        assert!(!is_dictated("t1", 5));
    }

    #[test]
    fn dictated_chunk_is_rejected() {
        fresh("t2");
        note_insert("t2", "insertReplacementText", 6);
        assert!(is_dictated("t2", 6), "a dictated chunk must never pass");
    }

    #[test]
    fn multichar_insert_is_rejected_even_as_inserttext() {
        // iOS delivers dictation as a long insertText on some versions.
        fresh("t3");
        note_insert("t3", "insertText", 7);
        assert!(is_dictated("t3", 7));
    }

    #[test]
    fn value_outrunning_keystrokes_is_rejected() {
        fresh("t4");
        note_insert("t4", "insertText", 1);
        assert!(is_dictated("t4", 9), "value longer than its keystrokes is not typing");
    }

    #[test]
    fn a_value_with_no_events_at_all_is_rejected() {
        fresh("t5");
        assert!(is_dictated("t5", 4));
        assert!(!is_dictated("t5", 0), "an empty field is not a violation");
    }

    /// Regression: the base game has no DOM input — it builds its answer
    /// from its own keyboard through game::type_char, which records here.
    /// If that recording is ever lost, EVERY base-game submission would be
    /// refused as dictation. This test is the tripwire.
    #[test]
    fn keyboard_built_answers_are_never_refused() {
        fresh("answerField");
        for _ in 0.."elephant".chars().count() {
            note_insert("answerField", "insertText", 1);
        }
        assert!(!is_dictated("answerField", "elephant".chars().count() as u32));
    }

    #[test]
    fn speech_input_types_are_named_exhaustively() {
        for t in ["insertFromDictation", "insertReplacementText", "insertFromPaste", "insertTranscription"] {
            assert!(is_speech_input_type(t));
        }
        assert!(!is_speech_input_type("insertText"));
        assert!(!is_speech_input_type("deleteContentBackward"));
    }
}

#[cfg(test)]
mod tone_taps_are_keystrokes {
    //! Eric on device: typed the EXACT bank answer `hai2zi5` for 孩子 and Check
    //! silently refused it. Nothing graded, no feedback -- submit_guess returns
    //! early when this module says the value was dictated.
    //!
    //! A tone-button tap IS a keystroke, but `tap_tone` changed the answer
    //! without recording one. So the field held 7 characters while this module
    //! had counted 5, and `value_len > keys + 1` declared it speech.
    use super::*;

    #[test]
    fn two_tone_taps_must_not_read_as_dictation() {
        // 5 letters typed, then two tone taps: h a i [2] z i [5] -> "hai2zi5"
        reset("answerField");
        for _ in 0..5 {
            note_insert("answerField", "insertText", 1);
        }
        // THE BUG: with the two tone taps unrecorded, a 7-char field looks
        // dictated. The +1 tolerance hides it for ONE tone and not for two,
        // which is exactly the reported symptom.
        assert!(
            is_dictated("answerField", 7),
            "precondition: unrecorded tone taps are what made this look dictated"
        );
        assert!(
            !is_dictated("answerField", 6),
            "one tone tap slips under the +1 tolerance -- why one tone 'worked'"
        );

        // THE FIX: a tone tap notes a keystroke like any other key.
        reset("answerField");
        for _ in 0..7 {
            note_insert("answerField", "insertText", 1);
        }
        assert!(!is_dictated("answerField", 7), "hai2zi5 must be accepted");
    }

    /// Korean had the same hole and worse. Hangul composition SHRINKS the
    /// buffer -- 6 jamo become 2 blocks -- so with `type_jamo` recording no
    /// keystroke at all, `value_len > keys + 1` fired on any word of two
    /// blocks or more. Every multi-block Korean answer was silently discarded.
    #[test]
    fn korean_multi_block_words_must_not_read_as_dictation() {
        // BEFORE: no keystroke recorded for jamo taps.
        reset("answerField");
        assert!(is_dictated("answerField", 2), "precondition: 2 blocks, 0 keys read as dictated");

        // AFTER: one keystroke per jamo. 한글 is six jamo composing to two blocks.
        reset("answerField");
        for _ in 0..6 {
            note_insert("answerField", "insertText", 1);
        }
        assert!(!is_dictated("answerField", 2), "한글 must be accepted");

        // A long word stays safe: composition can only shrink, never grow.
        reset("answerField");
        for _ in 0..12 {
            note_insert("answerField", "insertText", 1);
        }
        assert!(!is_dictated("answerField", 4), "a 4-block word must be accepted");
    }
}
