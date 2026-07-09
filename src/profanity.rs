//! Client-side profanity screen for user-imported "My Words". Kept as its own
//! module so the wordlist is trivial to extend — add terms to `BANNED_EXACT`
//! (whole-word match, for words that also appear inside innocent ones) or
//! `BANNED_ROOTS` (substring match, for roots that never occur inside normal
//! spelling words). It runs on every word before it's saved (lib.rs
//! `saveWords`) and again over any already-stored list on load
//! (`importer::load_custom`), so a list saved before a term was added — or one
//! carried over from an older build — still gets cleaned. This matters most
//! with Kid Mode, where a custom list shouldn't be able to smuggle in slurs.
//!
//! By the time a word reaches here it has already been through
//! `importer::extract_words`, whose regex splits on anything that isn't a
//! letter/apostrophe/hyphen — so space- and digit-based leetspeak ("f u c k",
//! "sh1t") is already broken into harmless fragments. `normalize` closes the
//! remaining gaps: it folds the symbol/digit leet homoglyphs (in case this is
//! ever run on raw text), strips accents, drops apostrophes/hyphens
//! ("f-u-c-k"), and keeps only a–z before matching.

use std::cell::Cell;

use unicode_normalization::UnicodeNormalization;

/// Roots that never legitimately occur inside a normal English spelling word,
/// so they're safe to match as a substring (catches "motherfucker",
/// "bullshit", "shithead", "niggard"-style evasions, etc.).
const BANNED_ROOTS: &[&str] = &["fuck", "shit", "cunt", "nigg", "faggot"];

/// Whole-word matches — terms that can be legitimate substrings of innocent
/// words ("ass" in "grass", "cock" in "peacock"), so they only block when the
/// entire custom word normalizes to them (or a listed variant).
const BANNED_EXACT: &[&str] = &[
    "ass", "asshole", "arse", "arsehole", "bastard", "bitch", "biatch", "bollocks", "bugger",
    "cock", "cocks", "crap", "damn", "dick", "dickhead", "dildo", "douche", "douchebag", "dumbass",
    "jackass", "piss", "pissed", "prick", "pussy", "pussies", "slut", "slag", "skank", "twat",
    "wank", "wanker", "whore", "boner", "horny", "porn", "porno", "penis", "vagina", "boob",
    "boobs", "tit", "tits", "titty", "titties", "anus", "turd", "jizz", "cum", "cumming",
    "retard", "retarded", "spastic", "minge", "knob",
    // common letter-only leet / misspell variants the extractor leaves intact
    "fuk", "fuc", "fux", "fuq", "fck", "fcuk", "phuck", "azz", "azzhole", "biotch", "shite",
];

/// Lowercase, fold leet homoglyphs, strip accents/marks, and keep only a–z.
fn normalize(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for ch in word.nfd().filter(|c| !('\u{0300}'..='\u{036f}').contains(c)) {
        let c = ch.to_ascii_lowercase();
        let folded = match c {
            '@' | '4' => 'a',
            '3' => 'e',
            '1' | '!' | '|' => 'i',
            '0' => 'o',
            '5' | '$' => 's',
            '7' | '+' => 't',
            '8' => 'b',
            '9' => 'g',
            other => other,
        };
        if folded.is_ascii_alphabetic() {
            out.push(folded);
        }
    }
    out
}

/// True if this custom word should be blocked from "My Words".
pub fn is_blocked(word: &str) -> bool {
    let n = normalize(word);
    if n.is_empty() {
        return false;
    }
    if BANNED_EXACT.contains(&n.as_str()) {
        return true;
    }
    BANNED_ROOTS.iter().any(|root| n.contains(root))
}

/// Splits a word list into the allowed words (order preserved) and a count of
/// how many were rejected, so callers can save the clean set and tell the user.
pub fn filter_allowed(words: Vec<String>) -> (Vec<String>, usize) {
    let mut blocked = 0usize;
    let allowed: Vec<String> = words
        .into_iter()
        .filter(|w| {
            let ok = !is_blocked(w);
            if !ok {
                blocked += 1;
            }
            ok
        })
        .collect();
    (allowed, blocked)
}

/// A friendly, gently-varied rejection line. "Non-repeating" — successive
/// calls never return the same phrasing twice in a row, so a player pasting a
/// list with several bad words isn't shown the exact same sentence each time.
pub fn rejection_message() -> &'static str {
    const MESSAGES: [&str; 5] = [
        "That word can't be added.",
        "Let's keep it friendly \u{2014} that one's not allowed.",
        "Skipped a word that isn't allowed here.",
        "That one's off-limits \u{2014} the rest are fine.",
        "Some words can't be added.",
    ];
    thread_local! {
        static LAST: Cell<usize> = const { Cell::new(usize::MAX) };
    }
    LAST.with(|last| {
        let prev = last.get();
        // Advance to a different index than last time (deterministic, so no
        // RNG dependency): first call picks 0, then steps forward, skipping
        // the previous pick.
        let mut idx = if prev == usize::MAX { 0 } else { (prev + 1) % MESSAGES.len() };
        if idx == prev {
            idx = (idx + 1) % MESSAGES.len();
        }
        last.set(idx);
        MESSAGES[idx]
    })
}
