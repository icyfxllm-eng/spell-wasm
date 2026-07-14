//! Runtime feature flags — the one place a shipped-dark feature is switched on.
//!
//! Each flag is a `pub fn -> bool`. A per-flag localStorage key
//! (`spell_flag_<name>`) overrides the coded default, so QA can flip a feature on
//! a device without a rebuild. A flag being OFF must be a true no-op: the feature
//! registers nothing, wires nothing, renders nothing — zero observable diff.
//!
//! This is the UNIFIED Feature Pack 2 module (the sibling branches each authored
//! their own copy; this reconciles them). Defaults below are set for the FP2
//! **integration / TestFlight build**: F1, F2, F6, F7 are ON so testers exercise
//! them; F5 word-stories stays OFF (its CC BY-SA attribution gate is unresolved).
//! For the App Store, these defaults would revert to OFF per each feature's PR.

fn flag(key: &str, default: bool) -> bool {
    match crate::storage::get_raw(key).as_deref() {
        Some("1") | Some("on") | Some("true") | Some("yes") => true,
        Some("0") | Some("off") | Some("false") | Some("no") => false,
        _ => default,
    }
}

/// F6 "Ghost racing in The Climb" — race your best local run. Cross-platform.
pub fn ghost_racing() -> bool {
    flag("spell_flag_ghost_racing", true)
}

/// F7 "Syllable replay on misses" — the "hear it slowly" affordance on the
/// reveal / spaced-repetition surface (es-only). Native syllable-by-syllable
/// AVSpeech with highlight; web falls back to a slow whole-word replay.
pub fn syllable_replay() -> bool {
    flag("spell_flag_syllable_replay", true)
}

/// F2 "Say It" — on-device pronunciation practice (see the word, say it).
/// iOS-only at runtime (needs the on-device SFSpeechRecognizer bridge); the mode
/// is hard-disabled in Kid Mode regardless of this flag (COPPA).
pub fn say_it() -> bool {
    flag("spell_flag_say_it", true)
}

/// F1 "Photo-to-word-list" — VisionKit OCR of a handout into a custom list.
/// iOS-only; hidden in Kid Mode (list management is a parent surface).
pub fn photo_list() -> bool {
    flag("spell_flag_photo_list", true)
}

/// F5 "Word stories" — one-line etymology cards. **Default OFF**: ships dark
/// until the CC BY-SA attribution approach is approved.
pub fn word_stories() -> bool {
    flag("spell_flag_word_stories", false)
}
