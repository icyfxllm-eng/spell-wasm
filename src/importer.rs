use std::collections::HashSet;

use crate::model::{AppState, CustomSet, CUSTOM_KEY};
use crate::profanity;
use crate::storage;
use regex::Regex;

fn word_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\p{L}][\p{L}'\u{2019}\-]*").unwrap())
}

pub fn extract_words(text: &str) -> Vec<String> {
    let re = word_regex();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for m in re.find_iter(text) {
        let trimmed = m.as_str().trim_matches(|c| c == '\'' || c == '\u{2019}' || c == '-');
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
            if out.len() >= 2000 {
                break;
            }
        }
    }
    out
}

pub fn load_custom(state: &mut AppState) {
    if let Some(mut c) = storage::get_json::<CustomSet>(CUSTOM_KEY) {
        // Screen already-stored words too: a list saved before a term entered
        // the blocklist (or carried over from an older build) still gets
        // cleaned on load. Re-persist only if something was actually removed.
        let before = c.words.len();
        let (clean, blocked) = profanity::filter_allowed(c.words);
        c.words = clean;
        state.custom = c;
        if blocked > 0 && before != state.custom.words.len() {
            save_custom(state);
        }
    }
}

fn save_custom(state: &AppState) {
    storage::set_json(CUSTOM_KEY, &state.custom);
}

/// Save a batch of words. **Additive**: the batch is merged into the already-
/// saved set (dedup, existing first) rather than replacing it — the input screen
/// is a clean slate for adding MORE words. Each new word is tagged with this
/// batch's "Speak in" language, so a list built from several batches in different
/// languages speaks each batch in its own voice.
///
/// Returns the BATCH ID this save was recorded under (CC-PHOTO-IMPORT Phase 4):
/// every word the batch introduced NEW is tagged with it, so `undo_batch` can
/// remove exactly that batch — and only words it introduced, never one an
/// earlier save already had. `custom_marks` are the words the review
/// classified out-of-dictionary.
pub fn save_words(
    state: &mut AppState,
    words: Vec<String>,
    speak_lang: String,
    custom_marks: &[String],
) -> u64 {
    let batch = state.custom.next_batch;
    let mut merged = state.custom.words.clone();
    let mut word_lang = std::mem::take(&mut state.custom.word_lang);
    let mut word_batch = std::mem::take(&mut state.custom.word_batch);
    let mut marks = std::mem::take(&mut state.custom.custom_marks);
    for w in words {
        if !merged.iter().any(|e| e == &w) {
            merged.push(w.clone());
            word_batch.insert(w.clone(), batch);
        }
        word_lang.insert(w, speak_lang.clone());
    }
    for m in custom_marks {
        // Marks only apply to words actually in the list (defensive).
        if merged.iter().any(|e| e == m) {
            marks.insert(m.clone());
        }
    }
    state.custom = CustomSet {
        words: merged,
        speak_lang,
        word_lang,
        word_batch,
        next_batch: batch + 1,
        custom_marks: marks,
    };
    save_custom(state);
    batch
}

/// Remove exactly the words `batch_id` INTRODUCED (Phase 4 one-tap undo).
/// Words that already existed before that batch (its save merely re-saved
/// them) carry a different batch id and are untouched. Returns how many
/// words were removed.
pub fn undo_batch(state: &mut AppState, batch_id: u64) -> usize {
    let doomed: Vec<String> = state
        .custom
        .word_batch
        .iter()
        .filter(|(_, b)| **b == batch_id)
        .map(|(w, _)| w.clone())
        .collect();
    if doomed.is_empty() {
        return 0;
    }
    state.custom.words.retain(|w| !doomed.contains(w));
    for w in &doomed {
        state.custom.word_lang.remove(w);
        state.custom.word_batch.remove(w);
        state.custom.custom_marks.remove(w);
    }
    save_custom(state);
    doomed.len()
}

pub fn clear_words(state: &mut AppState) {
    let speak_lang = state.custom.speak_lang.clone();
    state.custom = CustomSet {
        words: Vec::new(),
        speak_lang,
        word_lang: Default::default(),
        word_batch: Default::default(),
        // Batch ids keep counting up across a clear, so an undo handle from
        // before the clear can never collide with a batch saved after it.
        next_batch: state.custom.next_batch,
        custom_marks: Default::default(),
    };
    save_custom(state);
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    use crate::model::AppState;

    fn v(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    /// Undo removes exactly the batch's words — and ONLY words that batch
    /// introduced; a word an earlier save already owned survives a later
    /// batch's undo even though the later batch re-saved it.
    #[test]
    fn undo_removes_exactly_what_the_batch_introduced() {
        let mut s = AppState::default();
        let b1 = save_words(&mut s, v(&["cat", "dog"]), "en-US".into(), &[]);
        let b2 = save_words(&mut s, v(&["dog", "fox", "owl"]), "en-US".into(), &[]);
        assert_ne!(b1, b2);
        assert_eq!(s.custom.words, v(&["cat", "dog", "fox", "owl"]));

        let removed = undo_batch(&mut s, b2);
        assert_eq!(removed, 2, "only fox+owl were INTRODUCED by b2");
        assert_eq!(s.custom.words, v(&["cat", "dog"]), "dog belongs to b1 and survives");
        assert!(s.custom.word_lang.contains_key("dog"));
        assert!(!s.custom.word_batch.contains_key("fox"));
    }

    /// Undoing a stale/unknown batch id is a no-op.
    #[test]
    fn undo_unknown_batch_is_a_noop() {
        let mut s = AppState::default();
        save_words(&mut s, v(&["cat"]), "en-US".into(), &[]);
        assert_eq!(undo_batch(&mut s, 999), 0);
        assert_eq!(s.custom.words, v(&["cat"]));
    }

    /// Custom marks persist for marked words, are dropped with their word on
    /// undo, and never attach to words outside the list.
    #[test]
    fn custom_marks_track_their_words() {
        let mut s = AppState::default();
        let b = save_words(&mut s, v(&["zzblorp", "cat"]), "en-US".into(), &v(&["zzblorp", "ghost"]));
        assert!(s.custom.custom_marks.contains("zzblorp"));
        assert!(!s.custom.custom_marks.contains("ghost"), "marks only apply to saved words");
        undo_batch(&mut s, b);
        assert!(s.custom.custom_marks.is_empty(), "mark leaves with its word");
    }

    /// Batch ids keep counting across a clear — an old undo handle can never
    /// hit a batch saved after the clear.
    #[test]
    fn batch_ids_survive_clear_without_collision() {
        let mut s = AppState::default();
        let b1 = save_words(&mut s, v(&["cat"]), "en-US".into(), &[]);
        clear_words(&mut s);
        let b2 = save_words(&mut s, v(&["dog"]), "en-US".into(), &[]);
        assert!(b2 > b1, "ids are monotonic across clear");
        assert_eq!(undo_batch(&mut s, b1), 0, "stale handle hits nothing");
        assert_eq!(s.custom.words, v(&["dog"]));
    }
}
