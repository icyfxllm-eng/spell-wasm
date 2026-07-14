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
use std::collections::HashSet;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

/// Per-language profanity lists (LDNOOBW-sourced) for screening user-imported
/// My Words in every language — the English `BANNED_*` sets above only catch
/// Latin/leet English, so without this a non-English (or CJK/Thai) import could
/// smuggle in slurs, which matters most in Kid Mode. Matched as exact whole
/// words (NFC + lowercased) against the union of all languages, so a term that's
/// profane in ANY language is blocked regardless of the import's declared voice.
/// These lists are crowd-sourced and skew toward over-blocking (e.g. an innocent
/// common word occasionally appears) — the safe direction for a kids' app: a
/// rejected import is a minor annoyance, a smuggled slur is not. Native review
/// can prune false positives later without changing this wiring.
static EXTRA_BLOCKLIST: OnceLock<HashSet<String>> = OnceLock::new();

fn extra_blocklist() -> &'static HashSet<String> {
    EXTRA_BLOCKLIST.get_or_init(|| {
        let mut set = HashSet::new();
        for raw in [
            include_str!("../assets/words/profanity/en.txt"),
            include_str!("../assets/words/profanity/es.txt"),
            include_str!("../assets/words/profanity/fr.txt"),
            include_str!("../assets/words/profanity/de.txt"),
            include_str!("../assets/words/profanity/pt.txt"),
            include_str!("../assets/words/profanity/it.txt"),
            include_str!("../assets/words/profanity/nl.txt"),
            include_str!("../assets/words/profanity/pl.txt"),
            include_str!("../assets/words/profanity/sv.txt"),
            include_str!("../assets/words/profanity/nb.txt"),
            include_str!("../assets/words/profanity/tr.txt"),
            include_str!("../assets/words/profanity/vi.txt"),
            include_str!("../assets/words/profanity/ko.txt"),
            include_str!("../assets/words/profanity/ja.txt"),
            include_str!("../assets/words/profanity/th.txt"),
            include_str!("../assets/words/profanity/fil.txt"),
            include_str!("../assets/words/profanity/zh.txt"),
        ] {
            for line in raw.lines() {
                let t = line.trim();
                if t.is_empty() || t.starts_with('#') {
                    continue;
                }
                set.insert(t.nfc().collect::<String>().to_lowercase());
            }
        }
        set
    })
}

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
    // English/Latin + leetspeak layer (accent-strip + homoglyph fold to a-z).
    let n = normalize(word);
    if !n.is_empty() {
        if BANNED_EXACT.contains(&n.as_str()) {
            return true;
        }
        if BANNED_ROOTS.iter().any(|root| n.contains(root)) {
            return true;
        }
    }
    // Per-language layer: exact whole-word match (NFC + lowercased) against the
    // union of every language's list — screens non-English and CJK/Thai imports
    // that the a-z fold above can't see (it strips them to nothing).
    let raw: String = word.trim().nfc().collect::<String>().to_lowercase();
    !raw.is_empty() && extra_blocklist().contains(&raw)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_layer_still_blocks() {
        assert!(is_blocked("fuck"));
        assert!(is_blocked("sh1t")); // leet
        assert!(is_blocked("ASS"));
        assert!(!is_blocked("class")); // "ass" only blocks as a whole word
    }

    #[test]
    fn non_english_layer_blocks_and_allows() {
        // Present in the es/fil runtime lists -> blocked in any My Words import.
        assert!(is_blocked("puta"));
        // Ordinary words are not on any list.
        assert!(!is_blocked("elephant"));
        assert!(!is_blocked("manzana")); // Spanish "apple"
        assert!(!is_blocked("casa")); // Spanish "house"
    }

    #[test]
    fn filipino_seed_blocks_profanity_but_not_clean_words() {
        // Expanded fil seed (audit for Paul): core profanity, insults, sexual
        // terms, and a common numeric-leet form are blocked.
        for w in ["gaga", "putangina", "tarantado", "kupal", "pekpek", "g4go"] {
            assert!(is_blocked(w), "fil profanity '{w}' should be blocked");
        }
        // Legitimate Filipino vocabulary (incl. words we deliberately kept OUT of
        // the seed to avoid false-blocks) must remain addable.
        for w in ["bulaklak", "kaibigan", "hayop", "itlog", "pusa", "bahay"] {
            assert!(!is_blocked(w), "clean fil word '{w}' must not be blocked");
        }
    }
}
