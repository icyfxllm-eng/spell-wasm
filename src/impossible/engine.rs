//! CC-IMPOSSIBLE-COMPLETE v1 F5 -- the ONE Impossible rules engine.
//!
//! Every Impossible surface (single-language practice, Mix, and later Daily and the
//! BoardGame trap) takes its rules from here and nowhere else (D-C7, I5, I10):
//! one replay per word, one attempt per word, no hints, strict grading, and the
//! Misses tag. No surface carries its own copy of a limit; `one_engine` in the tests
//! fails if one does.
//!
//! Pure on purpose: game.rs and lib.rs ask these functions and act on the answer.

use unicode_normalization::UnicodeNormalization;

use super::{lang_availability, mix_status, Availability, Inputs, MixStatus, Surface};

/// The Normal-practice level value for single-language Impossible.
pub const LEVEL: &str = "impossible";
/// The Normal-practice level value for Impossible Mix.
pub const MIX_LEVEL: &str = "impossible_mix";
/// The Misses tag (Mix spec M9). A miss recorded under this tier is re-served at
/// Impossible rules in its own language.
pub const MISS_TAG: &str = "impossible";

/// One replay per word, on top of the first play (v1 F4, standing rule 6).
pub const MAX_REPLAYS: u8 = 1;
/// One attempt per word. No retry, no shield, no extra-attempt toggle (I10).
pub const MAX_ATTEMPTS: u8 = 1;

/// Languages whose spelling carries case, so Impossible grades case strictly
/// (D-C4, Eric 2026-10-18: German nouns are capitalised).
const CASE_STRICT: [&str; 1] = ["de"];

/// Does this language need the player to be able to type a capital? True where Impossible grades
/// case strictly. Every on-screen keyboard is lowercase-only, so such a language needs a Shift key
/// (main game) or stays out of a surface that has none (the board-game trap).
pub fn needs_shift(lang: &str) -> bool {
    CASE_STRICT.contains(&lang)
}

pub fn is_level(level: &str) -> bool {
    level == LEVEL || level == MIX_LEVEL
}

pub fn is_mix(level: &str) -> bool {
    level == MIX_LEVEL
}

/// May the player replay now, having already replayed `used` times on this word?
pub fn replay_allowed(used: u8) -> bool {
    used < MAX_REPLAYS
}

/// May the player submit now, having already submitted `used` times on this word?
pub fn attempt_allowed(used: u8) -> bool {
    used < MAX_ATTEMPTS
}

/// Accepted alternates (Eric, 2026-10-19): a word whose sound fits a second real spelling that is not in
/// the bank (comptroller / controller) stays in the pool and counts either spelling as right, instead of
/// leaving the pool. Data lives in `data/impossible/accepted/<lang>.json`; only languages with a file
/// have any. The pool word is the key, so a typed alternate is accepted only for that word.
fn accepted_json(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some(include_str!("../../data/impossible/accepted/en.json")),
        "es" => Some(include_str!("../../data/impossible/accepted/es.json")),
        _ => None,
    }
}

pub fn accepted_alternates(lang: &str, word: &str) -> Vec<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, HashMap<String, Vec<String>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut c = cache.lock().unwrap();
    let map = c.entry(lang.to_string()).or_insert_with(|| {
        accepted_json(lang)
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| serde_json::from_value::<HashMap<String, Vec<String>>>(v.get("accepted")?.clone()).ok())
            .unwrap_or_default()
    });
    map.get(word).cloned().unwrap_or_default()
}

/// Strict grading (F2). NFC, whitespace ignored, accent-strict like Expert, and for
/// the case-bearing languages case-strict too. Everything else about the comparison is
/// today's Expert fold, so a word graded here and at Expert differ only in case for de.
///
/// Sound-alikes never need the accept-any layer: the pool builder removed every word
/// with a valid competing spelling (I1), so exactly one spelling is right.
/// Russian writes ё as е in ordinary print, and the dictionary writes the ё: both are the right spelling (the
/// pool holds "актер", the dictionary has "актёр"). Voice cannot tell them apart, so both are accepted.
fn yo_fold(s: &str) -> String {
    crate::norm::fold_strict(s).replace('ё', "е")
}

pub fn grade(lang: &str, typed: &str, word: &str) -> bool {
    if CASE_STRICT.contains(&lang) {
        let strip = |s: &str| s.nfc().filter(|c| !c.is_whitespace()).collect::<String>();
        return strip(typed) == strip(word);
    }
    crate::norm::same_answer(lang, typed, word)
        || (lang == crate::consts::RU && yo_fold(typed) == yo_fold(word))
        || accepted_alternates(lang, word).iter().any(|a| crate::norm::same_answer(lang, typed, a))
        || (lang == crate::consts::EN && crate::variants::accepts(word, typed))
}

/// Mix draw step one (D-C12): a language uniformly from the run's languages. `r` is a
/// uniform sample in [0, 1). Word choice within the language is the existing no-repeat
/// `Deck`, so no word repeats until that language's pool is spent.
pub fn draw_language<'a>(langs: &[&'a str], r: f64) -> Option<&'a str> {
    if langs.is_empty() {
        return None;
    }
    let i = ((r * langs.len() as f64).floor() as usize).min(langs.len() - 1);
    Some(langs[i])
}

/// The languages one serve may draw from, or `None` when the level is not available
/// (a flag turned off mid-session, entitlement lost): the caller falls back to Expert.
/// Single-language Impossible uses the study language; Mix uses every qualifying one.
pub fn serve_langs(level: &str, study_lang: &str, i: &Inputs) -> Option<Vec<&'static str>> {
    if level == MIX_LEVEL {
        return match mix_status(Surface::Normal, i) {
            MixStatus::Available(v) => Some(v),
            _ => None,
        };
    }
    if level == LEVEL {
        return match lang_availability(study_lang, i) {
            Availability::Available(_) => super::registry_langs().find(|l| *l == study_lang).map(|l| vec![l]),
            _ => None,
        };
    }
    None
}

/// One entry of the level picker: the level value, and why it is locked (`None` = playable).
pub type PickerEntry = (&'static str, Option<super::LockReason>);

/// The Impossible entries the Normal-practice level picker shows for this study language (F3,
/// F5, F6). An entry that is merely switched off (a flag, a Jr profile, a closed language) is
/// absent: nothing is offered by default. A real lock shows, disabled, with its reason code's
/// copy. Single-language comes first, then Mix.
pub fn picker_entries(study_lang: &str, i: &Inputs) -> Vec<PickerEntry> {
    use super::LockReason::*;
    let mut out = Vec::new();
    match lang_availability(study_lang, i) {
        Availability::Available(_) => out.push((LEVEL, None)),
        Availability::Locked(r @ (PoolBelowFloor | NoExclusionGate | NoDrawnEntry | NeedsComplete)) => out.push((LEVEL, Some(r))),
        _ => {}
    }
    match mix_status(Surface::Normal, i) {
        MixStatus::Available(_) => out.push((MIX_LEVEL, None)),
        MixStatus::Locked(r @ (NeedsComplete | NeedsTwoLanguages)) => out.push((MIX_LEVEL, Some(r))),
        _ => {}
    }
    out
}

/// The i18n key for a lock's one-line reason (every surface renders this, never its own text).
pub fn lock_copy_key(r: super::LockReason) -> &'static str {
    use super::LockReason::*;
    match r {
        NeedsComplete => "impossible.lock.needs_complete",
        NeedsTwoLanguages => "impossible.lock.needs_two_languages",
        PoolBelowFloor => "impossible.lock.pool_below_floor",
        NoExclusionGate => "impossible.lock.no_exclusion_gate",
        NoDrawnEntry => "impossible.lock.no_drawn_entry",
        ProfileBlocked | FlagOff => "", // never shown: these entries are absent, not locked
    }
}
