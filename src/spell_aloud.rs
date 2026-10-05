//! Feature "Spell It Out Loud" — voice spelling INPUT.
//!
//! The player speaks letter *names* ("C… A… T") and this parser produces exactly
//! the string a keyboard user typing those letters would produce. It is an INPUT
//! METHOD (a mic beside the answer field), never a standalone mode, and it shares
//! the ONE on-device microphone with Say-It — two recognition *profiles* over one
//! capture: Say-It reads whole words, this reads letter names.
//!
//! CORE PRINCIPLE: this is CONSTRAINED LETTER-SEQUENCE capture, NOT dictation. A
//! recognizer that hears "cat" and yields "cat" is REJECTED as a whole word (the
//! player is nudged to spell letter by letter) — see [`interpret`]. That rejection
//! is by LETTER YIELD ALONE: the target word is never given to the recognizer or the
//! matcher (answer-leak invariant, gate G-A), so a whole word can never score as a
//! correct spelling and the answer can never leak through the mic path.
//!
//! ALL linguistic knowledge lives in the lexicons (`lexicons/letters/<lang>.json`,
//! the SINGLE SOURCE OF TRUTH). This module only turns a token stream into a
//! letter string; it hardcodes no letter names (Invariant I4).
//!
//! Invariants honored here:
//! * **I1** — output is NFC-normalized, byte-for-byte identical to keyboard input.
//! * **I2 / I5** — no network, no persistence: this module is pure text; the audio
//!   path is on-device only (see the plugin) and nothing outlives the session.
//! * **I3** — the mic renders iff `voiceSpell` (config) AND on-device availability;
//!   [`reflect`] enforces exactly those two conditions (plus the master flag).
//! * **I4** — no letter-name literal outside `lexicons/letters/`.
//! * **I6** — a true no-op when the feature flag is OFF.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

use crate::consts::{AR, DE, EN, ES, FIL, FR, HI, JA, KO, PL, PT, RU, SW, VI, ZH};

// ===========================================================================
// Pure parser (host-unit-tested — no DOM, no wasm, no audio)
// ===========================================================================

/// Raw lexicon shape as authored in `lexicons/letters/<lang>.json`. The four
/// groups are purely organizational (letter names, ASR homophone variants,
/// multigraph phrases, diacritic phrases); they are merged into one lookup table
/// at load. Unknown fields (`lang`, `_doc`) are ignored.
#[derive(serde::Deserialize)]
struct RawLexicon {
    #[serde(default, rename = "letterNames")]
    letter_names: HashMap<String, String>,
    #[serde(default)]
    homophones: HashMap<String, String>,
    #[serde(default)]
    multigraph: HashMap<String, String>,
    #[serde(default)]
    diacritics: HashMap<String, String>,
    /// Spoken editing/turn commands (Phase 2): phrase → command id
    /// (`delete` / `clear` / `done`). Kept in the SAME single-source file (I4).
    #[serde(default)]
    commands: HashMap<String, String>,
    /// Acoustically-confusable letter classes (Phase 3): a low-confidence letter in
    /// a class offers a two-choice chip instead of a guess. Letters, not names.
    #[serde(default)]
    confusable: Vec<Vec<String>>,
    /// Clarifier connectors (Phase 3): after a letter, `<letter> as in <word>`
    /// confirms the letter and the example word is ignored.
    #[serde(default)]
    clarifiers: Vec<String>,
    /// Deterministically-ambiguous letter names (Phase 4): a bare name the recognizer
    /// can't disambiguate (es "be"/"ve" → b/v). In the MODE these ALWAYS chip; the
    /// input-method `parse` keeps its plain mapping. phrase → [choiceA, choiceB].
    #[serde(default)]
    ambiguous: HashMap<String, Vec<String>>,
}

/// A spoken editing / turn command (Phase 2). Distinct from letter input: it acts on
/// the slot buffer instead of adding to it. Never free text — an enumerated id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Remove the last slot (delete / backspace / borrar).
    Delete,
    /// Empty the buffer (clear / start over / borrar todo).
    Clear,
    /// Finish the turn — submit the assembled word (done / listo).
    Done,
}

impl Command {
    fn from_id(s: &str) -> Option<Command> {
        match s {
            "delete" => Some(Command::Delete),
            "clear" => Some(Command::Clear),
            "done" => Some(Command::Done),
            _ => None,
        }
    }
}

/// One parsed unit of a spoken utterance in the mode: a letter (fills a slot), a
/// command (acts on the buffer), or an ambiguous name that must be disambiguated by a
/// two-choice chip (Phase 4). Units can interleave in one breath.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Letter(Slot),
    Command(Command),
    /// A bare b/v-style name the recognizer can't resolve — offer `{a, b}` (never
    /// auto-picked; auto-resolving would leak the answer).
    Chip(String, String),
}

/// A compiled lexicon: the merged phrase→letters table (keys normalized for
/// matching, values NFC), the max phrase length in words, and the original spoken
/// forms to bias the recognizer (`contextualStrings`).
struct Lexicon {
    table: HashMap<String, String>,
    /// Spoken commands (Phase 2): normalized phrase → `Command`.
    commands: HashMap<String, Command>,
    /// Confusable letter classes (Phase 3), letters normalized.
    confusable: Vec<Vec<String>>,
    /// Clarifier connectors (Phase 3) as normalized word sequences, longest first.
    clarifiers: Vec<Vec<String>>,
    /// Ambiguous names (Phase 4): normalized phrase → (choiceA, choiceB).
    ambiguous: HashMap<String, (String, String)>,
    max_words: usize,
    contextual: Vec<String>,
}

/// The raw JSON for a language, `include_str!`'d like the word-list / homophone
/// pattern. One arm per supported language — the ONLY place these files are named.
fn source(lang: &str) -> Option<&'static str> {
    match lang {
        EN => Some(include_str!("../lexicons/letters/en.json")),
        ES => Some(include_str!("../lexicons/letters/es.json")),
        // Mic-everywhere (Eric, 2026-07-27): machine-drafted lexicons pending
        // native review. The capabilities gate still decides where the mic
        // actually appears (on-device speech per language per device).
        FR => Some(include_str!("../lexicons/letters/fr.json")),
        DE => Some(include_str!("../lexicons/letters/de.json")),
        PT => Some(include_str!("../lexicons/letters/pt.json")),
        PL => Some(include_str!("../lexicons/letters/pl.json")),
        VI => Some(include_str!("../lexicons/letters/vi.json")),
        RU => Some(include_str!("../lexicons/letters/ru.json")),
        AR => Some(include_str!("../lexicons/letters/ar.json")),
        HI => Some(include_str!("../lexicons/letters/hi.json")),
        KO => Some(include_str!("../lexicons/letters/ko.json")),
        JA => Some(include_str!("../lexicons/letters/ja.json")),
        ZH => Some(include_str!("../lexicons/letters/zh.json")),
        SW => Some(include_str!("../lexicons/letters/sw.json")),
        FIL => Some(include_str!("../lexicons/letters/fil.json")),
        _ => None,
    }
}

/// Normalize one spoken word for matching: trim edge punctuation the recognizer
/// attaches, then NFC + lowercase. Accents and ñ are PRESERVED (Spanish letter
/// names distinguish them — `eñe` must never collapse to `ene`).
fn norm_word(w: &str) -> String {
    let trimmed = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-');
    trimmed.nfc().collect::<String>().to_lowercase()
}

/// Normalize a (possibly multi-word) lexicon key into the canonical space-joined
/// form the parser looks up.
fn norm_phrase(p: &str) -> String {
    p.split_whitespace().map(norm_word).collect::<Vec<_>>().join(" ")
}

fn compile(raw: &RawLexicon) -> Lexicon {
    let mut table: HashMap<String, String> = HashMap::new();
    let mut contextual: Vec<String> = Vec::new();
    let mut max_words = 1;
    for group in [&raw.letter_names, &raw.homophones, &raw.multigraph, &raw.diacritics] {
        for (k, v) in group {
            let key = norm_phrase(k);
            if key.is_empty() {
                continue;
            }
            max_words = max_words.max(key.split(' ').count());
            let value: String = v.nfc().collect();
            table.insert(key, value);
            contextual.push(k.clone());
        }
    }
    // Commands (Phase 2): a parallel phrase→Command table, biased like the letters.
    let mut commands: HashMap<String, Command> = HashMap::new();
    for (k, id) in &raw.commands {
        let key = norm_phrase(k);
        if key.is_empty() {
            continue;
        }
        if let Some(cmd) = Command::from_id(id) {
            max_words = max_words.max(key.split(' ').count());
            commands.insert(key, cmd);
            contextual.push(k.clone());
        }
    }
    // Confusable classes (Phase 3): normalize each letter for matching.
    let confusable: Vec<Vec<String>> =
        raw.confusable.iter().map(|g| g.iter().map(|l| norm_word(l)).collect()).collect();
    // Clarifier connectors (Phase 3): normalized word sequences, longest first, and
    // biased on the recognizer.
    let mut clarifiers: Vec<Vec<String>> = raw
        .clarifiers
        .iter()
        .map(|c| {
            contextual.push(c.clone());
            norm_phrase(c).split(' ').map(str::to_string).collect()
        })
        .filter(|w: &Vec<String>| !w.is_empty())
        .collect();
    clarifiers.sort_by(|a, b| b.len().cmp(&a.len()));

    // Ambiguous names (Phase 4): normalized phrase → (a, b). Only well-formed pairs.
    let mut ambiguous: HashMap<String, (String, String)> = HashMap::new();
    for (k, choices) in &raw.ambiguous {
        let key = norm_phrase(k);
        if key.is_empty() || choices.len() != 2 {
            continue;
        }
        let a: String = choices[0].nfc().collect();
        let b: String = choices[1].nfc().collect();
        max_words = max_words.max(key.split(' ').count());
        ambiguous.insert(key, (a, b));
    }

    contextual.sort();
    contextual.dedup();
    Lexicon { table, commands, confusable, clarifiers, ambiguous, max_words, contextual }
}

/// Per-language compiled lexicon, built once and leaked for a `'static` ref (one
/// small table per language for the process lifetime — same pattern as
/// `homophones.rs`).
fn lexicon(lang: &str) -> Option<&'static Lexicon> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static Lexicon>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(l) = cache.lock().unwrap().get(lang) {
        return Some(*l);
    }
    let raw: RawLexicon = serde_json::from_str(source(lang)?).ok()?;
    let leaked: &'static Lexicon = Box::leak(Box::new(compile(&raw)));
    cache.lock().unwrap().insert(lang.to_string(), leaked);
    Some(leaked)
}

/// One accepted letter-name in a spoken utterance: the letters a single matched
/// token (or multi-word phrase) produced, NFC-normalized. Usually one character
/// ("c"); a multigraph name ("elle"→"ll") or an accented vowel phrase
/// ("a con tilde"→"á") is still ONE slot. The push-and-hold mode (Phase 1) renders
/// one slot per accepted letter; a spoken whole word yields zero slots — the
/// answer-leak-safe unit of "what was accepted", never compared to the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The letters this token produced (NFC).
    pub letters: String,
}

/// The result of parsing a spoken utterance into letters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    /// The letters the player spelled, NFC-normalized (Invariant I1). Exactly what
    /// a keyboard would have produced.
    pub letters: String,
    /// One entry per matched letter-name token, in order (the slot model).
    pub slots: Vec<Slot>,
    /// Word tokens that were consumed by a letter/phrase match.
    pub matched_words: usize,
    /// Total word tokens in the utterance.
    pub total_words: usize,
}

impl Parsed {
    /// Fraction of the utterance that parsed as letter names. High for genuine
    /// spelling ("see ay tee" → 1.0); ~0 for a spoken whole word ("cat").
    pub fn yield_ratio(&self) -> f64 {
        if self.total_words == 0 {
            0.0
        } else {
            self.matched_words as f64 / self.total_words as f64
        }
    }
}

/// Turn a raw recognizer transcript into a letter string for `lang`. Greedy
/// longest-phrase match so multi-word names (multigraph and diacritic phrases)
/// win over their single-word prefixes. All the actual name→letter mappings live
/// in the lexicon (Invariant I4); this function hardcodes none. Languages without
/// a lexicon parse to nothing.
pub fn parse(lang: &str, transcript: &str) -> Parsed {
    let Some(lex) = lexicon(lang) else {
        return Parsed { letters: String::new(), slots: Vec::new(), matched_words: 0, total_words: 0 };
    };
    let words: Vec<String> =
        transcript.split_whitespace().map(norm_word).filter(|w| !w.is_empty()).collect();
    let n = words.len();
    let mut out = String::new();
    let mut slots: Vec<Slot> = Vec::new();
    let mut matched = 0usize;
    let mut i = 0usize;
    while i < n {
        let max_len = lex.max_words.min(n - i);
        let mut hit: Option<(usize, &str)> = None;
        for len in (1..=max_len).rev() {
            let phrase = words[i..i + len].join(" ");
            if let Some(v) = lex.table.get(&phrase) {
                hit = Some((len, v.as_str()));
                break;
            }
        }
        match hit {
            Some((len, v)) => {
                out.push_str(v);
                slots.push(Slot { letters: v.to_string() }); // lexicon values are already NFC
                matched += len;
                i += len;
            }
            None => i += 1,
        }
    }
    let letters: String = out.nfc().collect();
    Parsed { letters, slots, matched_words: matched, total_words: n }
}

/// The spoken forms to hand the recognizer as `contextualStrings` for `lang`
/// (letter names + phrases). Biasing lives in the lexicon, not in Swift.
pub fn contextual_strings(lang: &str) -> Vec<String> {
    lexicon(lang).map(|l| l.contextual.clone()).unwrap_or_default()
}

// ---- whole-word rejection (Feature 7, answer-leak-safe) ----

/// Minimum fraction of an utterance that must parse as letter names for it to count
/// as a spelling. Below this, the player spoke a word (or babble), not a spelling.
const WHOLE_WORD_YIELD: f64 = 0.5;

/// What the input method should do with a finalized utterance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellOutcome {
    /// Append these letters at the cursor (already NFC).
    Insert(String),
    /// The player spoke a whole word / non-letters — nudge "spell it letter by
    /// letter", insert nothing, count no attempt (Feature 7).
    WholeWord,
    /// Nothing was heard at all — a subtle "didn't catch that", insert nothing.
    Nothing,
}

/// Decide what a finalized transcript means in `lang`. **The target word is
/// deliberately NOT a parameter** (answer-leak invariant, gate G-A): a whole word
/// is rejected purely because it yields too few letter names to be a spelling —
/// never by comparing the utterance to the answer. A spoken whole word yields ~0
/// letter names and so structurally cannot be credited; genuine letter-by-letter
/// spelling (high yield) always inserts. Because the target is invisible here,
/// "spoke a word" and "made noise" are the same low-yield signal — both nudge; only
/// a truly empty utterance is `Nothing`.
pub fn interpret(lang: &str, transcript: &str) -> SpellOutcome {
    let parsed = parse(lang, transcript);
    if parsed.total_words == 0 {
        return SpellOutcome::Nothing; // silence / nothing heard
    }
    if parsed.yield_ratio() < WHOLE_WORD_YIELD {
        return SpellOutcome::WholeWord; // a word or babble, not a spelling
    }
    SpellOutcome::Insert(parsed.letters)
}

/// D3 (CC-SPELL-ALOUD-INTEGRATION, Feature 4): does this utterance SAY THE TARGET
/// word — as the whole transcript or as any single whitespace token — NFC +
/// case-insensitive? If so the WHOLE utterance is discarded (zero letters), even with
/// letters embedded alongside ("cat see ay tee"). The target is used ONLY to reject
/// here, NEVER to resolve or disambiguate letters (G-A's anti-leak intent holds for
/// letter mapping — G-INT-2). Matches a single token, not a joined one, so genuine
/// single-letter ASR ("c a t") spelling the word is never mistaken for saying it.
pub fn says_target(transcript: &str, target: &str) -> bool {
    let t = norm_word(target);
    if t.is_empty() {
        return false;
    }
    transcript.split_whitespace().map(norm_word).any(|w| w == t)
}

/// A6 / D7: if this utterance is a sole editing command — **undo** (delete) or
/// **clear** — return it, for the input method to apply to the answer field. Letters,
/// the target word, and `done` (submission is the on-screen control, not a voice
/// command) all return `None`. Pure + host-tested.
pub fn edit_command(lang: &str, transcript: &str) -> Option<Command> {
    match events(lang, transcript).as_slice() {
        [Event::Command(c @ (Command::Delete | Command::Clear))] => Some(*c),
        _ => None,
    }
}

/// Remove the last grapheme — one "backspace" — so the voice **undo** command behaves
/// exactly like the on-screen backspace / typed delete (D7). Grapheme-aware so an
/// accented letter (á) or ñ is removed as one unit. Pure + host-tested.
pub fn drop_last_grapheme(s: &str) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    let mut g: Vec<&str> = s.graphemes(true).collect();
    g.pop();
    g.concat()
}

/// Fold one finalized utterance into a slot `buffer` — the push-and-hold mode's
/// accumulator (Phase 1). Only a genuine spelling (`Insert`) contributes: its slots
/// are appended and the newly-added letters returned (for spoken echo). A whole word,
/// babble, or silence adds nothing and returns `""` — the caller nudges. The target
/// is never consulted (answer-leak invariant, G-A). Pure and host-unit-tested.
pub fn accept_into(buffer: &mut Vec<Slot>, lang: &str, transcript: &str) -> String {
    if !matches!(interpret(lang, transcript), SpellOutcome::Insert(_)) {
        return String::new();
    }
    let mut added = String::new();
    for slot in parse(lang, transcript).slots {
        added.push_str(&slot.letters);
        buffer.push(slot);
    }
    added
}

/// Parse a spoken utterance into an ordered stream of letter/command events (Phase 2,
/// mode-only). Greedy longest-phrase match over BOTH the letter table and the command
/// table (a command phrase like "borrar todo" wins over "borrar"); unmatched tokens
/// are skipped. Letters and commands may interleave ("c a t done"). The plain
/// `parse`/`interpret` used by the input method stay command-free.
pub fn events(lang: &str, transcript: &str) -> Vec<Event> {
    let Some(lex) = lexicon(lang) else {
        return Vec::new();
    };
    let words: Vec<String> =
        transcript.split_whitespace().map(norm_word).filter(|w| !w.is_empty()).collect();
    let n = words.len();
    let mut out: Vec<Event> = Vec::new();
    let mut i = 0usize;
    while i < n {
        let max_len = lex.max_words.min(n - i);
        // Greedy longest match over commands, ambiguous names, then letters.
        let mut hit: Option<(usize, Match)> = None;
        for len in (1..=max_len).rev() {
            let phrase = words[i..i + len].join(" ");
            if let Some(cmd) = lex.commands.get(&phrase) {
                hit = Some((len, Match::Command(*cmd)));
                break;
            }
            if let Some((a, b)) = lex.ambiguous.get(&phrase) {
                hit = Some((len, Match::Ambiguous(a.clone(), b.clone())));
                break;
            }
            if let Some(v) = lex.table.get(&phrase) {
                hit = Some((len, Match::Letter(v.clone())));
                break;
            }
        }
        match hit {
            Some((len, Match::Command(c))) => {
                out.push(Event::Command(c));
                i += len;
            }
            Some((len, Match::Letter(v))) => {
                out.push(Event::Letter(Slot { letters: v }));
                i += len;
                // Clarifier (Phase 3): after a LETTER, `as in <word>` confirms it and
                // the example word is dropped — so "b as in you" is B, not B then U.
                if let Some((skip, _example)) = clarifier_after(lex, &words[i..]) {
                    i += skip;
                }
            }
            Some((len, Match::Ambiguous(a, b))) => {
                i += len;
                // Phase 4: a bare b/v name ALWAYS chips — UNLESS a clarifier resolves
                // it by the example word's first letter ("be de burro" → B). The
                // example is user speech, not the target, so this never leaks (G-A).
                if let Some((skip, example)) = clarifier_after(lex, &words[i..]) {
                    let first = example.chars().next();
                    let resolved = if first == a.chars().next() {
                        Some(&a)
                    } else if first == b.chars().next() {
                        Some(&b)
                    } else {
                        None
                    };
                    match resolved {
                        Some(letter) => out.push(Event::Letter(Slot { letters: letter.clone() })),
                        None => out.push(Event::Chip(a, b)), // clarifier didn't disambiguate
                    }
                    i += skip;
                } else {
                    out.push(Event::Chip(a, b));
                }
            }
            None => i += 1,
        }
    }
    out
}

/// Internal match kind at one position (kept out of the public `Event`).
enum Match {
    Command(Command),
    Letter(String),
    Ambiguous(String, String),
}

/// If `rest` begins with a clarifier connector followed by at least one example word,
/// return `(tokens_to_skip, example_word)` (skip = connector length + the example).
fn clarifier_after<'a>(lex: &Lexicon, rest: &'a [String]) -> Option<(usize, &'a str)> {
    for conn in &lex.clarifiers {
        let clen = conn.len();
        if rest.len() > clen && rest[..clen] == conn[..] {
            return Some((clen + 1, rest[clen].as_str()));
        }
    }
    None
}

/// English letters that rhyme (the "E-set") and m/n are acoustically confusable; a
/// low-confidence letter in a class is worth confirming rather than guessing (F4).
///
/// Below this, a confusable letter offers a two-choice chip instead of a guess.
///
/// SIGNED OFF by Eric on 2026-10-04 at 0.55, after a device pass where a clearly
/// spoken B came back as P twice running. It remains a CALIBRATION value: too
/// high and the chip interrupts confident spelling, too low and the letters it
/// exists for slip through. Tune it against real device traces, not by taste.
pub const CONFUSABLE_CONFIDENCE: f32 = 0.55;

fn confusable_group<'a>(lex: &'a Lexicon, letter: &str) -> Option<&'a [String]> {
    lex.confusable.iter().find(|g| g.iter().any(|l| l == letter)).map(Vec::as_slice)
}

/// Does this language's lexicon carry clarifier connectors at all? en and es
/// today; the other thirteen have none until the native review supplies them.
pub fn has_clarifiers(lang: &str) -> bool {
    lexicon(lang).map(|lex| !lex.clarifiers.is_empty()).unwrap_or(false)
}

/// Languages where "<letter> as in <word>" is VERIFIED to parse, which is not
/// the same as having clarifier data.
///
/// Spanish has the data and does not work: its connector `de` IS the name of
/// the letter D, so "b de burro" parses as the single letter d and the b is
/// lost, "b como en burro" yields nothing at all, and "be de burro" gives "bd".
/// Teaching that phrasing would be teaching a failure, so the tip stays hidden
/// there until the parser can tell a connector from a letter name — which is
/// CC-SPELL-ALOUD's parser to change, not this file's.
const CLARIFIER_VERIFIED: [&str; 1] = [crate::consts::EN];

/// Whether to offer the "<letter> as in <word>" tip for this language.
pub fn clarifier_tip_ok(lang: &str) -> bool {
    has_clarifiers(lang) && CLARIFIER_VERIFIED.contains(&lang)
}

/// True if `a` and `b` are distinct letters in the same confusable class for `lang`.
pub fn are_confusable(lang: &str, a: &str, b: &str) -> bool {
    if a == b {
        return false;
    }
    lexicon(lang)
        .and_then(|lex| confusable_group(lex, a))
        .map(|g| g.iter().any(|l| l == b))
        .unwrap_or(false)
}

/// What to do with one finalized letter from the recognizer, given how sure it was
/// (`confidence`, 0..1) and its best `alt`ernative segment reading. A confident or
/// non-confusable letter is accepted; a low-confidence confusable letter whose
/// alternative is a same-class letter offers a two-choice **chip** instead of a guess
/// (F4). The target is never consulted (G-A) — ambiguity resolves via the chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LetterDecision {
    Accept(String),
    Chip(String, String),
}

pub fn decide_letter(lang: &str, letter: &str, confidence: f32, alt: Option<&str>) -> LetterDecision {
    if confidence >= CONFUSABLE_CONFIDENCE {
        return LetterDecision::Accept(letter.to_string());
    }
    if let Some(alt) = alt {
        if are_confusable(lang, letter, alt) {
            return LetterDecision::Chip(letter.to_string(), alt.to_string());
        }
    }
    // Low confidence but no same-class alternative to offer → take the best guess.
    LetterDecision::Accept(letter.to_string())
}

/// What the mode should do after applying one utterance's events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Letters newly added this utterance (for the spoken echo).
    pub echo: String,
    /// `done` was spoken — the mode submits the assembled word.
    pub done: bool,
    /// A pending two-choice disambiguation (Phase 4): the mode shows a chip and
    /// waits for the player to pick — nothing after it in the utterance is applied.
    pub chip: Option<(String, String)>,
}

/// Apply an event stream to the slot `buffer` (pure + host-tested): letters push,
/// `Delete` pops, `Clear` empties, `Done` flags submit. A `Chip` stops processing and
/// is returned as a pending disambiguation (letters before it are still applied).
/// `done` does NOT clear — the caller reads the assembled buffer to submit.
pub fn apply_events(buffer: &mut Vec<Slot>, evs: &[Event]) -> Applied {
    let mut applied = Applied::default();
    for ev in evs {
        match ev {
            Event::Letter(s) => {
                applied.echo.push_str(&s.letters);
                buffer.push(s.clone());
            }
            Event::Command(Command::Delete) => {
                buffer.pop();
            }
            Event::Command(Command::Clear) => {
                buffer.clear();
            }
            Event::Command(Command::Done) => {
                applied.done = true;
            }
            Event::Chip(a, b) => {
                applied.chip = Some((a.clone(), b.clone()));
                break; // wait for the player's pick before applying anything more
            }
        }
    }
    applied
}

// ===========================================================================
// UI wiring (wasm-only; not host-unit-tested)
// ===========================================================================

use crate::native_lang;
use crate::App;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

thread_local! {
    /// True while ONE VAD segment (one letter) is in flight with the native recognizer.
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
    /// The player edited the box by hand during a session, so voice may no
    /// longer write to it. Stopping the session is NOT enough on its own:
    /// stop_letter_capture is asynchronous and the in-flight final still
    /// arrives, still holding the letters it heard, and still writes them back
    /// over the edit. That is how Eric deleted a stray "t" and watched it
    /// return. Cleared when a new session starts.
    static MANUAL_EDIT: Cell<bool> = const { Cell::new(false) };
    /// Letters from the utterance still in flight, kept apart from the ones
    /// already finished (SESSION_LETTERS). The recognizer emits SEVERAL results
    /// for one spoken letter as it refines -- "M", then "M" again -- and it
    /// emits each letter as its own result rather than a growing transcript.
    /// Without this split a repeat looked like a new letter and Eric's single
    /// "M" landed twice.
    static CURRENT_UTTER: RefCell<String> = const { RefCell::new(String::new()) };
    /// When the in-flight utterance last changed, in ms. A repeat arriving
    /// quickly is the recognizer refining ONE letter; the same letter arriving
    /// after a gap is the player saying it twice, which "book" and "letter"
    /// depend on. Nothing else distinguishes them: the recognizer sends no
    /// boundary and both look like the identical parse arriving again.
    static LAST_PARTIAL_MS: Cell<f64> = const { Cell::new(0.0) };
    /// The letter a confusable chip is offering as its BEST guess, held while
    /// the player decides. It is not lost if they ignore the chip and simply
    /// carry on spelling: the next finalized letter commits it first.
    static PENDING_CHIP: RefCell<String> = const { RefCell::new(String::new()) };
    /// True from the FIRST mic tap until the user taps again to stop (or an error). While
    /// set, each finalized letter auto-restarts capture for the next — one press, spell
    /// the whole word letter by letter with a beat between letters (native VAD segments).
    static LISTENING: Cell<bool> = const { Cell::new(false) };
    /// The answer text present when capture began — voice spelling APPENDS to it,
    /// and any failure state reverts to it (typed input is never lost).
    static BASE: RefCell<String> = const { RefCell::new(String::new()) };
    /// The current target word — read ONLY for D3 whole-word rejection (G-INT-2),
    /// NEVER given to the recognizer or used to resolve letters (G-A).
    static TARGET: RefCell<String> = const { RefCell::new(String::new()) };
    /// Letters accumulated this capture session. MONOTONIC: a partial that retracts
    /// (the recognizer revises to fewer letters) never shrinks it, so a spelled letter
    /// that flickers in the recognizer's partials still sticks on the answer line
    /// instead of appearing then vanishing.
    static SESSION_LETTERS: RefCell<String> = const { RefCell::new(String::new()) };
    /// True once the permission explainer has been shown this app-run.
    static EXPLAINED: Cell<bool> = const { Cell::new(false) };
    /// Mic-everywhere: the capability state reflect() last saw for the current
    /// language — "installed" | "downloadable" | "unavailable". Decides whether
    /// a mic tap starts capture or first fetches the on-device voice pack.
    static CAP_STATE: RefCell<String> = const { RefCell::new(String::new()) };
    /// A voice-pack download is in flight (taps are ignored until it settles).
    static DOWNLOADING: Cell<bool> = const { Cell::new(false) };
}

/// Show or hide the play-screen front door (`spellItBtn`). Offered when the
/// flag is on AND the native bridge is present, so off-iOS it stays hidden.
/// Safe to call on every settings change.
///
/// This lived in the pronunciation mode's module until CC-HUB-DEADROWS removed
/// that mode. The element carried its name, because it owned the button first;
/// CC-HUB-CLEANUP D1 handed the button here and left the old name, which is how
/// a drawer row reading one thing came to launch another. The name is honest now.
pub fn reflect_tile(app: &App) {
    let _ = app;
    let offered = crate::flags::spell_aloud() && native_lang::available();
    crate::dom::toggle_class("spellItBtn", "btn-hide", !offered);
}

/// The master gate: the whole feature is dark unless the flag is on (Invariant I6).
pub fn enabled() -> bool {
    crate::flags::spell_aloud()
}

/// Show or hide the mic beside the answer field. Renders IFF (config `voiceSpell`)
/// AND (on-device recognizer available for the locale) — Invariant I3, no third
/// condition beyond the master flag + the native bridge being present at all.
pub fn reflect(app: &App) {
    if !enabled() {
        crate::dom::add_class("voiceSpellMic", "btn-hide");
        return;
    }
    let lang = app.borrow().lang.clone();
    // The guide's clarifier line follows the lexicon, not the mic: it is advice
    // about phrasing, and it is wrong to show it where it would not parse.
    if crate::dom::exists("spellItGuideClarify") {
        crate::dom::toggle_class("spellItGuideClarify", "btn-hide", !clarifier_tip_ok(&lang));
    }
    // First condition: the per-language config flag (single source of truth).
    // Also require the native bridge to even be present (off-iOS it never is).
    if !crate::consts::voice_spell(&lang) || !native_lang::available() {
        crate::dom::add_class("voiceSpellMic", "btn-hide");
        return;
    }
    // Second condition: on-device availability for this locale (async). Until it
    // resolves the mic stays hidden — never a broken button. Mic-everywhere
    // ladder: "installed" → live mic; "downloadable" → the mic shows with a
    // download badge and the first tap fetches the on-device voice pack;
    // "unavailable" → hidden (no on-device path exists — never a server).
    crate::dom::add_class("voiceSpellMic", "btn-hide");
    let kid = app.borrow().kid;
    spawn_local(async move {
        let cap = native_lang::speech_capabilities(&lang).await;
        // Server rung (mic-everywhere): a language with NO on-device path may
        // still get the mic through the backend — explicit consent card, a 🌐
        // badge, and HARD exclusions: never Kid Mode, never education builds.
        let state = if cap.state == "unavailable"
            && crate::consts::server_stt(&lang)
            && !kid
            && !cfg!(feature = "education")
        {
            "server".to_string()
        } else {
            cap.state
        };
        CAP_STATE.with(|c| *c.borrow_mut() = state.clone());
        crate::dom::toggle_class("voiceSpellMic", "btn-hide", state == "unavailable");
        crate::dom::toggle_class("voiceSpellMic", "dl", state == "downloadable");
        crate::dom::toggle_class("voiceSpellMic", "net", state == "server");
    });
}

/// Attach the mic handlers — ONLY when the flag is on. Flag off ⇒ nothing wired.
/// F13 — set once the guide has been read, so it shows exactly one time.
const GUIDE_SEEN: &str = "spell_spellit_guide_seen";

thread_local! {
    /// True while the guide is open as HELP (from Help & Settings) rather than
    /// as the door into play. "Got it" then just closes it: a player who opened
    /// a help page did not ask to be put into a round. It also leaves
    /// GUIDE_SEEN alone, so reading help early does not rob a first-time player
    /// of the guide on their actual first entry.
    static GUIDE_AS_HELP: Cell<bool> = const { Cell::new(false) };
}

/// Route into play through the SAME hidden entry every other launcher
/// uses, so the guide adds a step in front of the flow rather than a
/// second way in.
fn enter_play() {
    if let Ok(el) = crate::dom::el("spellAloudEnter").dyn_into::<web_sys::HtmlElement>() {
        el.click();
    }
}

/// Split one segment's letters into what commits now and what (if anything) is
/// held back for a chip.
///
/// Pure on purpose. The surrounding commit path is DOM-bound and no host test
/// can reach it — which is exactly how `surface_set` came to call itself for ten
/// builds without a single test noticing.
pub fn split_for_chip(
    lang: &str,
    committed: &str,
    confidence: f32,
    alt: &str,
) -> (String, Option<(String, String)>) {
    let Some(last) = committed.chars().last() else {
        return (String::new(), None);
    };
    let last = last.to_string();
    let head: String = {
        let mut it = committed.chars();
        it.next_back();
        it.collect()
    };
    let alt_opt = if alt.is_empty() { None } else { Some(alt) };
    match decide_letter(lang, &last, confidence, alt_opt) {
        LetterDecision::Accept(_) => (committed.to_string(), None),
        LetterDecision::Chip(best, other) => (head, Some((best, other))),
    }
}

/// Offer the two-choice confusable chip. `best` is the recognizer's own first
/// choice and is what gets committed if the player ignores the chip and keeps
/// spelling — never nothing, so a chip can only ever IMPROVE on the guess.
fn show_chip(best: &str, alt: &str) {
    PENDING_CHIP.with(|c| *c.borrow_mut() = best.to_string());
    crate::dom::set_text("voiceSpellChipA", &best.to_uppercase());
    crate::dom::set_text("voiceSpellChipB", &alt.to_uppercase());
    crate::dom::set_hidden("voiceSpellChips", false);
}

fn hide_chip() {
    PENDING_CHIP.with(|c| c.borrow_mut().clear());
    crate::dom::set_hidden("voiceSpellChips", true);
}

/// Append one letter to the answer exactly as a finalized letter does, and keep
/// BASE in step so the next letter lands after it.
///
/// Deliberately NOT routed through game::type_char: that marks a MANUAL EDIT,
/// which tears the capture session down on the grounds that the player's hands
/// won. A chip tap is part of the voice flow, not a hand overriding it.
fn append_letter(app: &App, letter: &str) {
    let base = BASE.with(|b| b.borrow().clone());
    let updated = format!("{base}{letter}");
    BASE.with(|b| *b.borrow_mut() = updated.clone());
    surface_set(app, &updated);
    crate::haptics::key_tap();
}

/// The player picked one of the two offered letters.
fn pick_chip(app: &App, letter: &str) {
    if PENDING_CHIP.with(|c| c.borrow().is_empty()) {
        return; // no chip in flight; a stray tap commits nothing
    }
    hide_chip();
    append_letter(app, letter);
    set_status(if LISTENING.with(Cell::get) { "voiceSpell.listening" } else { "" });
}

/// A chip was showing and something else finalized. The player did not choose,
/// so the recognizer's own best guess stands — the same letter they would have
/// got with no chip at all.
fn settle_pending_chip(app: &App) {
    let pending = PENDING_CHIP.with(|c| c.borrow().clone());
    if pending.is_empty() {
        return;
    }
    hide_chip();
    append_letter(app, &pending);
}

pub fn wire(app: &App) {
    if !enabled() {
        return;
    }
    // ONE-PRESS continuous: tap the mic once, then spell the word letter by letter with
    // a short beat between letters. Native Voice Activity Detection ends each letter on
    // the pause after it (finalizing that letter), and capture auto-restarts for the
    // next — so the whole word is spelled from a single tap. Tap again to stop.
    let a_tap = app.clone();
    crate::dom::on_click("voiceSpellMic", move || mic_tap(&a_tap));
    // The two confusable chips. Their LABELS change per offer; which letter each
    // one carries is read from the DOM at tap time rather than captured here.
    let a_chip_a = app.clone();
    crate::dom::on_click("voiceSpellChipA", move || {
        let l = crate::dom::text("voiceSpellChipA").to_lowercase();
        pick_chip(&a_chip_a, &l);
    });
    let a_chip_b = app.clone();
    crate::dom::on_click("voiceSpellChipB", move || {
        let l = crate::dom::text("voiceSpellChipB").to_lowercase();
        pick_chip(&a_chip_b, &l);
    });
    // A9 / Feature 8: "Type instead" — dismiss the permission-denied fallback. Typing
    // is captured by the window keydown, so dismissing lands the player in typed
    // standard mode HOLDING THE SAME WORD (the mode is standard-mode-with-voice; the
    // word is untouched). Not a dead sentence.
    crate::dom::on_click("voiceSpellPermClose", || {
        crate::dom::add_class("voiceSpellPerm", "btn-hide");
    });
    // Server-rung consent card (mic-everywhere): OK persists and starts the
    // session; "No thanks" closes and nothing changes.
    let a_net = app.clone();
    crate::dom::on_click("voiceSpellNetOk", move || {
        crate::storage::set_raw("spell_stt_ok", "1");
        crate::dom::add_class("voiceSpellNet", "btn-hide");
        mic_tap(&a_net);
    });
    crate::dom::on_click("voiceSpellNetNo", || {
        crate::dom::add_class("voiceSpellNet", "btn-hide");
    });

    // Play-hub entry (CC-SPELL-ALOUD-INTEGRATION Feature 1): the hub routes here. Enter
    // the mode by ensuring a round is active — serve a word if there's none/answered,
    // else replay it — so the player hears what to spell. The voice mic is already
    // shown for voice-spell languages. Mirrors the orb; scoring stays the typed path.
    // CC-HUB-CLEANUP D1: the home quick tile (spellItBtn) is the mode's single
    // front door — same behavior as the hub entry.
    //
    // AUDITPASS F13: it now routes through the guide screen the note here
    // has been waiting for. ONCE (Eric, 2026-08-08) — a returning child
    // should not re-read four steps to spell a word — with "How this
    // works" on the mic surface to bring it back, so showing it once does
    // not make it unfindable, which was the point of the annotation.
    let a_tile = app.clone();
    crate::dom::on_click("spellItBtn", move || {
        GUIDE_AS_HELP.with(|h| h.set(false));
        if crate::storage::get_raw(GUIDE_SEEN).is_none() {
            crate::dom::add_class("spellItGuide", "show");
        } else {
            enter_play();
        }
        let _ = &a_tile;
    });
    // GUARD, not optional. `dom::on_click` -> `el()` PANICS on a missing
    // element, and a panic at wire time kills boot outright. The guide
    // markup was briefly inside a Spell-Picture-stripped region, so the
    // SITE build had no #spellItGuideGo and every site e2e test timed out
    // waiting for wasm that never started. The markup is placed correctly
    // now; this guard means a future strip-region edit cannot take the
    // whole site down again. Same idiom as wire_placement's dom::exists.
    if crate::dom::exists("spellItGuideGo") {
        crate::dom::on_click("spellItGuideGo", || {
            crate::dom::remove_class("spellItGuide", "show");
            if GUIDE_AS_HELP.with(|h| h.replace(false)) {
                return;
            }
            crate::storage::set_raw(GUIDE_SEEN, "1");
            enter_play();
        });
    }
    // The way back: replay the guide from the mic surface. Does NOT clear
    // the seen flag — asking for help is not the same as never having read
    // it, and clearing would re-show it unbidden on the next entry.
    if crate::dom::exists("spellItGuideAgain") {
        crate::dom::on_click("spellItGuideAgain", || {
            GUIDE_AS_HELP.with(|h| h.set(true));
            crate::dom::add_class("spellItGuide", "show");
        });
    }
    let a_enter = app.clone();
    crate::dom::on_click("spellAloudEnter", move || {
        let (answered, active) = {
            let s = a_enter.borrow();
            (s.answered, crate::game::has_active_word(&s))
        };
        if !active || answered {
            crate::game::next_word(&a_enter);
        } else {
            crate::game::speak_current(&a_enter);
        }
    });
}

/// Mic-everywhere: first tap on a downloadable-state mic fetches the ON-DEVICE
/// voice pack (iOS 26 Speech assets — recognition still never leaves the
/// phone), streaming progress into the status line, then flips the mic live.
/// Failure resets to the downloadable state so the tap can retry.
fn download_pack_then_reflect(app: &App) {
    DOWNLOADING.with(|d| d.set(true));
    crate::dom::add_class("voiceSpellMic", "listening"); // pulse = something's happening
    set_status("voiceSpell.dlPrep");
    let lang = app.borrow().lang.clone();
    let a = app.clone();
    spawn_local(async move {
        let ok = native_lang::download_speech_assets(&lang, move |fraction| {
            let pct = (fraction * 100.0).round() as u32;
            let text = format!("{} {pct}%", crate::i18n::t("voiceSpell.dlPrep"));
            crate::dom::set_text("voiceSpellStatus", &text);
        })
        .await;
        DOWNLOADING.with(|d| d.set(false));
        crate::dom::remove_class("voiceSpellMic", "listening");
        if ok {
            CAP_STATE.with(|c| *c.borrow_mut() = "installed".into());
            crate::dom::remove_class("voiceSpellMic", "dl");
            set_status("voiceSpell.dlReady");
        } else {
            set_status("voiceSpell.dlFail");
        }
        reflect(&a);
    });
}

fn set_status(key: &str) {
    let text = if key.is_empty() { String::new() } else { crate::i18n::t(key) };
    crate::dom::set_text("voiceSpellStatus", &text);
}

fn end_capture_ui() {
    CAPTURING.with(|c| c.set(false));
    LISTENING.with(|l| l.set(false));
    crate::dom::remove_class("voiceSpellMic", "listening");
    // A chip belongs to a live capture. Left showing after the session ends it
    // would hang over a finished round offering a letter nothing is waiting for.
    hide_chip();
}

/// Mic TAP: toggle one-press continuous capture. First tap starts listening (and the
/// first letter segment); a second tap while listening stops. NEVER auto-submits;
/// NEVER clears typed text.
/// v7 F7 / D9 — which surface the capture component is serving. The
/// component is NOT reimplemented per mode: it reads the target and the
/// append-base from the active surface and writes letters back to it.
/// D3-strict whole-word rejection is untouched and applies to both.
#[derive(Clone, Copy, PartialEq)]
pub enum Surface {
    Game,
    SpellPicture,
}

thread_local! {
    static SURFACE: Cell<u8> = const { Cell::new(0) };
}

pub fn set_surface(s: Surface) {
    SURFACE.with(|c| c.set(match s {
        Surface::Game => 0,
        Surface::SpellPicture => 1,
    }));
}

pub fn surface() -> Surface {
    if SURFACE.with(Cell::get) == 1 { Surface::SpellPicture } else { Surface::Game }
}

/// Target word, current buffer, and whether input is live — from
/// whichever surface is active.
fn surface_state(app: &App) -> (String, String, bool) {
    match surface() {
        Surface::Game => {
            let s = app.borrow();
            (s.word.clone(), s.answer.clone(), crate::game::can_type(&s))
        }
        Surface::SpellPicture => crate::surface_hooks::get()
            .voice_state
            .map(|f| f())
            .unwrap_or_default(),
    }
}

/// Write the assembled letters back to the active surface.
fn surface_set(app: &App, text: &str) {
    match surface() {
        // CC-SPELLIT-MIC-FIX: this called ITSELF. On the play surface every
        // write recursed until the wasm stack was exhausted and the instance
        // trapped, so no letter ever reached the box — while the on-screen diag
        // kept updating, because that is written by JavaScript and never enters
        // Rust at all. The app looked alive and was not.
        //
        // game::set_answer is the Game surface's setter, and its own doc says
        // it exists "by Spell It Out Loud to append parsed letters".
        Surface::Game => crate::game::set_answer(app, text),
        Surface::SpellPicture => {
            if let Some(f) = crate::surface_hooks::get().voice_set {
                f(text);
            }
        }
    }
}

/// What a tap on the mic should do. Pure, so the ORDER of these branches is
/// testable on the host — `mic_tap` itself is DOM-bound and no host test can
/// reach it, which is how the ordering below went unnoticed.
#[derive(Debug, PartialEq, Eq)]
pub enum Tap {
    /// Stop a live session. CC-SPELLIT-MIC-FIX I-M1: always reachable.
    Stop,
    /// Nothing: the feature is off, or a voice pack is mid-download.
    Ignore,
    /// Fetch the on-device voice pack first.
    Download,
    /// Ask for consent before the server rung's first use.
    Consent,
    /// Begin listening.
    Start,
}

/// CC-SPELLIT-MIC-FIX F1.2 — "a tap in starting or listening stops the mic.
/// ALWAYS". `listening` is therefore tested FIRST, ahead of every capability
/// branch.
///
/// It used to be tested fifth. On the `downloadable` and `server` rungs the tap
/// took an earlier branch and returned, so the stop was unreachable and the
/// player was left with a live microphone and no way to switch it off by
/// tapping — exactly what I-M1 forbids. English with the pack installed sits on
/// the `installed` rung, which is why it survived testing.
pub fn tap_action(listening: bool, enabled: bool, downloading: bool, cap: &str, consented: bool) -> Tap {
    if listening {
        return Tap::Stop;
    }
    if !enabled || downloading {
        return Tap::Ignore;
    }
    match cap {
        "downloadable" => Tap::Download,
        "server" if !consented => Tap::Consent,
        _ => Tap::Start,
    }
}

/// The player touched the answer box with their hands — a key, a jamo, or
/// backspace — while a voice session was live.
///
/// Their edit wins and the session ends. Before this, `on_partial` rewrote the
/// box from BASE + SESSION_LETTERS on every partial, so a deleted letter came
/// straight back on the next one and the box fought anyone trying to correct
/// it: Eric spelled "eyes", deleted a letter, watched it reappear, and then
/// could not get a correct answer to stay long enough to check it.
///
/// Stopping is the honest resolution rather than merging. The recognizer's
/// transcript for the session still contains the deleted letter, so the next
/// partial would re-add it however the two were reconciled; only ending voice
/// input makes the box the player's again. Tapping the mic starts a fresh
/// session that appends to whatever they left, which is F1.7 exactly.
pub fn note_manual_edit() {
    if LISTENING.with(Cell::get) || CAPTURING.with(Cell::get) {
        // Mark BEFORE stopping: the final can land as soon as capture ends.
        MANUAL_EDIT.with(|m| m.set(true));
        SESSION_LETTERS.with(|s| s.borrow_mut().clear());
        CURRENT_UTTER.with(|u| u.borrow_mut().clear());
        // The hands won, so a letter the chip was holding is void with the rest.
        hide_chip();
        stop_session();
    }
}

pub fn mic_tap(app: &App) {
    let action = tap_action(
        LISTENING.with(Cell::get),
        enabled(),
        DOWNLOADING.with(Cell::get),
        &CAP_STATE.with(|c| c.borrow().clone()),
        crate::storage::get_raw("spell_stt_ok").as_deref() == Some("1"),
    );
    match action {
        Tap::Stop => {
            stop_session();
            return;
        }
        Tap::Ignore => return,
        Tap::Download => {
            download_pack_then_reflect(app);
            return;
        }
        Tap::Consent => {
            // First use of the internet rung: the consent card, never a silent
            // fallback. OK persists the choice; dismiss just closes.
            crate::dom::remove_class("voiceSpellNet", "btn-hide");
            return;
        }
        Tap::Start => {}
    }
    let (target, base, can) = surface_state(app);
    if !can {
        return;
    }
    // The target is read ONLY for D3 whole-word rejection (G-INT-2). It is NOT given
    // to the recognizer or the letter matcher, and never resolves letters (G-A).
    BASE.with(|b| *b.borrow_mut() = base);
    TARGET.with(|t| *t.borrow_mut() = target);
    LISTENING.with(|l| l.set(true));
    crate::dom::add_class("voiceSpellMic", "listening");
    set_status("voiceSpell.listening");
    begin_session(app);
}

/// A new word was served: the live capture session, its append-base, and its target
/// all belong to the OLD word. Stop any capture and reset, so the new word starts
/// with a clean answer line — a trailing final from the old session is ignored
/// (CAPTURING is already false) instead of resurrecting the last word's letters.
pub fn on_new_word() {
    if CAPTURING.with(Cell::get) || LISTENING.with(Cell::get) {
        native_lang::stop_letter_capture();
    }
    end_capture_ui();
    BASE.with(|b| b.borrow_mut().clear());
    TARGET.with(|t| t.borrow_mut().clear());
    SESSION_LETTERS.with(|s| s.borrow_mut().clear());
    set_status("");
}

/// User stop: end listening and finalize any in-flight segment. The final segment's
/// `on_final` sees LISTENING == false and does NOT restart.
fn stop_session() {
    LISTENING.with(|l| l.set(false));
    if CAPTURING.with(Cell::get) {
        native_lang::stop_letter_capture();
    } else {
        end_capture_ui();
        set_status("");
    }
}

/// Start the ONE native capture session for the whole word. Native VAD segments it
/// per letter, delivering each via `on_final(is_end == false)` while it keeps
/// listening; `is_end == true` arrives when the user stops. BASE/TARGET are set by
/// `mic_tap` and persist across segments; SESSION_LETTERS resets per segment.
fn begin_session(app: &App) {
    MANUAL_EDIT.with(|m| m.set(false));
    SESSION_LETTERS.with(|s| s.borrow_mut().clear());
    CURRENT_UTTER.with(|u| u.borrow_mut().clear());
    CAPTURING.with(|c| c.set(true));
    let lang = app.borrow().lang.clone();
    let ctx = contextual_strings(&lang);
    let lang_c = lang.clone();
    let a_partial = app.clone();
    let a_final = app.clone();
    let a_error = app.clone();
    let server_url = if CAP_STATE.with(|c| c.borrow().clone()) == "server" {
        Some(format!("{}/api/stt", crate::api::api_base()))
    } else {
        None
    };
    let ok = native_lang::start_letter_capture(
        &lang,
        &ctx,
        server_url.as_deref(),
        move |transcript| on_partial(&a_partial, &lang_c, &transcript),
        {
            // F4: confidence and the recognizer's own second choice decide whether
            // the last letter is committed or offered as a chip.
            let lang_f = lang.clone();
            move |transcript, confidence, alt, is_end| {
                on_final(
                    &a_final,
                    &lang_f,
                    &transcript,
                    confidence as f32,
                    alt.as_deref().unwrap_or(""),
                    is_end,
                )
            }
        },
        move |code| on_error(&a_error, &code),
        on_boundary,
    );
    if !ok {
        // Bridge missing/uncallable — treat as unavailable, revert cleanly.
        end_capture_ui();
        crate::dom::add_class("voiceSpellMic", "btn-hide");
    }
}

/// Live echo: preview `base + accumulated letters`. MONOTONIC — the shown letters only
/// grow; a partial that the recognizer later revises to FEWER letters does not shrink
/// them, so a spelled letter never appears then vanishes as the hypothesis jitters.
/// The VAD heard the pause after a letter: bank the utterance in flight so the
/// next parse starts a new one. This is the EXACT signal the clock below only
/// estimates — with it, a letter said twice quickly still counts twice.
fn on_boundary() {
    CURRENT_UTTER.with(|u| {
        let mut cur = u.borrow_mut();
        if cur.is_empty() {
            return;
        }
        SESSION_LETTERS.with(|s| s.borrow_mut().push_str(&cur));
        cur.clear();
    });
    // The clock rule is the fallback for engines that send no boundary (the
    // iOS 26 analyzer, the server rung). Reset it so it cannot double-bank
    // what this just banked.
    LAST_PARTIAL_MS.with(|t| t.set(0.0));
}

/// A repeat arriving sooner than this is the recognizer refining one letter;
/// later, it is the player saying the letter again. Generous, because the gap
/// between two deliberately spoken letters is most of a second and a refinement
/// lands in a few hundred milliseconds.
pub const REPEAT_IS_NEW_UTTERANCE_MS: f64 = 900.0;

/// One step of the accumulator, pure so the rule can be tested against what a
/// real device sends. `on_partial` is DOM-bound and no host test can reach it —
/// the same blind spot that hid surface_set calling itself.
pub fn accumulate_step(done: &mut String, cur: &mut String, parsed: String, gap_ms: f64) {
    if parsed.is_empty() {
        return;
    }
    // CUMULATIVE TRANSCRIPT. Since F6.3 stopped closing the request at each letter
    // boundary, the legacy recognizer runs ONE request for the whole session and
    // every partial carries the whole transcript so far: "b", "bo", "boo",
    // "book". Banking the in-flight text at a boundary then re-banks everything
    // already said -- thirteen boundaries turned "book" into "bbooooo" on Eric's
    // phone (req=1, bnd=4/9 in that trace).
    //
    // So: when the new parse contains everything accumulated so far as a prefix,
    // it SUPERSEDES it rather than extending it. This is self-correcting -- a
    // spurious boundary is undone by the very next partial -- and it leaves the
    // per-letter-request engines untouched, because there a fresh transcript
    // never starts with what came before.
    let whole = format!("{done}{cur}");
    if !whole.is_empty() && parsed.starts_with(whole.as_str()) {
        done.clear();
        *cur = parsed;
        return;
    }
    if parsed == *cur {
        // The identical parse again. Fast = the recognizer settling on one
        // letter. Slow = the player really said it twice, and "book" needs it.
        if gap_ms >= REPEAT_IS_NEW_UTTERANCE_MS {
            done.push_str(cur);
        }
        return;
    }
    if parsed.starts_with(cur.as_str()) {
        *cur = parsed;
    } else {
        done.push_str(cur);
        *cur = parsed;
    }
}

/// `"|"` is a VAD boundary — the pause after a letter — rather than a parse.
#[cfg(test)]
pub fn accumulate_partial_for_test(parses: &[(&str, f64)]) -> String {
    let (mut done, mut cur) = (String::new(), String::new());
    for (p, gap) in parses {
        if *p == "|" {
            done.push_str(&cur);
            cur.clear();
            continue;
        }
        accumulate_step(&mut done, &mut cur, parse("en", p).letters, *gap);
    }
    format!("{done}{cur}")
}

fn on_partial(app: &App, lang: &str, transcript: &str) {
    if !CAPTURING.with(Cell::get) || MANUAL_EDIT.with(Cell::get) {
        return;
    }
    let parsed = parse(lang, transcript).letters;
    let base = BASE.with(|b| b.borrow().clone());
    let shown = SESSION_LETTERS.with(|s| CURRENT_UTTER.with(|u| {
        let mut done = s.borrow_mut();
        let mut cur = u.borrow_mut();
        // Where one letter ends and the next begins. Two facts, both learned
        // from Eric's device:
        //
        //   * each spoken letter arrives as its OWN result, not as a growing
        //     transcript — "See", then "A", then "T";
        //   * the SAME letter arrives several times while the recognizer
        //     refines it — "M", then "M" again.
        //
        // So: a parse that EXTENDS the utterance in flight replaces it (the
        // refinement case, and the growing-transcript case if a device ever
        // does that). A parse that does NOT extend it means the previous
        // utterance is over — bank it and start a new one.
        //
        // "grow only" lost every letter after the first; appending every
        // non-extending parse doubled repeats. This does neither.
        //
        // That leaves one case shape alone cannot call: the SAME letter twice,
        // which is either a refinement or a real repeat. `on_boundary` settles
        // it when the engine reports the pause it measured (the iOS legacy
        // recognizer does); `gap_ms` against REPEAT_IS_NEW_UTTERANCE_MS is the
        // fallback for engines that report none.
        let now = js_sys::Date::now();
        let gap = LAST_PARTIAL_MS.with(|t| {
            let prev = t.get();
            t.set(now);
            if prev == 0.0 { 0.0 } else { now - prev }
        });
        accumulate_step(&mut done, &mut cur, parsed, gap);
        format!("{}{}", done, cur)
    }));
    surface_set(app, &format!("{}{}", base, shown));
}

/// One VAD segment (one letter) finalized: commit it. The NATIVE session keeps
/// listening between segments (`is_end == false`) — nothing to restart here; the
/// session lands when `is_end` arrives (user stop, or an old-payload single-shot).
/// Never counts an attempt; never submits.
fn on_final(app: &App, lang: &str, transcript: &str, confidence: f32, alt: &str, is_end: bool) {
    if !CAPTURING.with(Cell::get) {
        return;
    }
    // The player's hands won while this was in flight. Tear the session down
    // and write NOTHING: the box is theirs.
    if MANUAL_EDIT.with(Cell::get) {
        SESSION_LETTERS.with(|s| s.borrow_mut().clear());
        CURRENT_UTTER.with(|u| u.borrow_mut().clear());
        end_capture_ui();
        set_status("");
        return;
    }
    let still = !is_end && LISTENING.with(Cell::get);
    // A chip was offered and the player kept spelling instead of choosing. Their
    // silence is not a rejection of the letter, only of the question: commit the
    // recognizer's best guess, which is what they would have had with no chip.
    settle_pending_chip(app);
    let base = BASE.with(|b| b.borrow().clone());
    let target = TARGET.with(|t| t.borrow().clone());
    // Letters accumulated (monotonically) during this segment — the ones already shown.
    // Committing THIS (not a re-parse of the final transcript, which the recognizer may
    // have shrunk) is what makes spelled letters stick.
    let accumulated = SESSION_LETTERS.with(|s| s.borrow().clone())
        + &CURRENT_UTTER.with(|u| u.borrow().clone());
    SESSION_LETTERS.with(|s| s.borrow_mut().clear());
    CURRENT_UTTER.with(|u| u.borrow_mut().clear());
    // D3 (Feature 4): the utterance SAYS THE TARGET WORD (whole or embedded) → discard
    // it, nudge to spell it out. Zero letters, never a miss. BASE is unchanged. (D2)
    if says_target(transcript, &target) {
        surface_set(app, &base);
        set_status("voiceSpell.spellItOut");
    } else if let Some(cmd) = edit_command(lang, transcript) {
        // A6 / D7: a voice edit command acts on the answer field like backspace / start
        // over (never `done` — submission is the on-screen control). Never a miss.
        let edited = match cmd {
            Command::Delete => drop_last_grapheme(&base),
            Command::Clear => String::new(),
            Command::Done => base.clone(), // ignored: submit is the on-screen control
        };
        BASE.with(|b| *b.borrow_mut() = edited.clone());
        surface_set(app, &edited);
        crate::haptics::key_tap();
        set_status(if still { "voiceSpell.listening" } else { "" });
    } else if accumulated.is_empty()
        && matches!(interpret(lang, transcript), SpellOutcome::WholeWord)
    {
        // AUDITPASS F13 — a full-word utterance is not a spelling, so it
        // produces nothing and nudges instead. `interpret` has computed
        // this since the module was written and NOTHING outside it ever
        // called it, so the rule existed on paper only: say any word that
        // is not the target and the parse scraped whatever letters it
        // could out of the noise.
        //
        // TWO GUARDS, both load-bearing:
        //   * it sits AFTER `edit_command`, because "undo" and "clear"
        //     are single words — ahead of that check, wiring this would
        //     have swallowed voice editing whole.
        //   * it fires only when the segment accumulated NOTHING. Letters
        //     spelled during a segment are already on screen; a child who
        //     spells c-a-t and then trails off into a word must keep them.
        //     `says_target` may discard visible letters (D3: saying the
        //     answer voids the attempt) but a stray word must not.
        surface_set(app, &base);
        set_status("voiceSpell.spellItOut");
    } else {
        // Commit the accumulated letters (or the final parse if it's somehow longer).
        let final_letters = parse(lang, transcript).letters;
        let committed = if final_letters.chars().count() > accumulated.chars().count() {
            final_letters
        } else {
            accumulated
        };
        if committed.is_empty() {
            // An empty segment — a false VAD boundary (a pause with no clear letter) is
            // routine in one-press mode. Keep the field and keep listening silently.
            surface_set(app, &base);
            set_status(if still {
                "voiceSpell.listening"
            } else {
                "voiceSpell.didntCatch"
            });
        } else {
            // F4 — the confusable chip. The LAST letter of the segment is the one
            // in question; anything before it was settled by earlier utterances.
            // A confident letter, or one whose alternative is not a same-class
            // letter, commits exactly as before: the chip never interrupts
            // spelling the recognizer was sure about.
            let (to_commit, chip) = split_for_chip(lang, &committed, confidence, alt);
            let updated = format!("{}{}", base, to_commit);
            // Persist across segments so the next letter appends after this one.
            BASE.with(|b| *b.borrow_mut() = updated.clone());
            surface_set(app, &updated);
            crate::haptics::key_tap();
            match chip {
                Some((best, other)) => show_chip(&best, &other),
                None => hide_chip(),
            }
            set_status(if still { "voiceSpell.listening" } else { "" });
        }
    }
    // Mid-stream (`still`): the native session is already listening for the next
    // letter — nothing to do. Session over: land the UI.
    if !still {
        end_capture_ui();
    }
}

/// Capture error: never blocks typed input — always revert to the typed base.
fn on_error(app: &App, code: &str) {
    end_capture_ui();
    let base = BASE.with(|b| b.borrow().clone());
    surface_set(app, &base);
    match code {
        "PERMISSION_DENIED" => {
            if !EXPLAINED.with(Cell::get) {
                EXPLAINED.with(|c| c.set(true));
                crate::dom::remove_class("voiceSpellPerm", "btn-hide");
            }
        }
        "UNAVAILABLE" => crate::dom::add_class("voiceSpellMic", "btn-hide"),
        // Server rung only: the backend was unreachable — keep the mic, say so.
        "NETWORK" => set_status("voiceSpell.netErr"),
        // I-M3: the samples arrived carrying no signal. Saying "didn't catch
        // that" here blames the player for a fault in the capture path, which
        // is what hid this bug for four builds.
        "MIC_SILENT" => set_status("voiceSpell.micSilent"),
        _ => set_status("voiceSpell.didntCatch"),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod loopback;
