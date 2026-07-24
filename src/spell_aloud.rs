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

use crate::consts::{EN, ES};

/// The push-and-hold MODE surface (CC-SPELL-ALOUD Phase 1), reusing this parser.
pub mod screen;

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
/// PROPOSED confidence threshold — below it, a confusable letter offers a two-choice
/// chip. This is a CALIBRATION value (like the racing pace bands): it ships as a
/// proposal and is tuned against the Phase-5 loopback suite; the final value needs
/// Eric's sign-off before the confusable chips go live.
pub const CONFUSABLE_CONFIDENCE: f32 = 0.55;

fn confusable_group<'a>(lex: &'a Lexicon, letter: &str) -> Option<&'a [String]> {
    lex.confusable.iter().find(|g| g.iter().any(|l| l == letter)).map(Vec::as_slice)
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
// UI wiring (wasm-only; not host-unit-tested — mirrors say_it.rs's DOM layer)
// ===========================================================================

use crate::native_lang;
use crate::App;
use wasm_bindgen_futures::spawn_local;

thread_local! {
    /// True while a letter capture is in flight (mic toggles stop).
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
    /// The answer text present when capture began — voice spelling APPENDS to it,
    /// and any failure state reverts to it (typed input is never lost).
    static BASE: RefCell<String> = const { RefCell::new(String::new()) };
    /// True once the permission explainer has been shown this app-run.
    static EXPLAINED: Cell<bool> = const { Cell::new(false) };
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
    // First condition: the per-language config flag (single source of truth).
    // Also require the native bridge to even be present (off-iOS it never is).
    if !crate::consts::voice_spell(&lang) || !native_lang::available() {
        crate::dom::add_class("voiceSpellMic", "btn-hide");
        return;
    }
    // Second condition: on-device availability for this locale (async). Until it
    // resolves the mic stays hidden — never a broken button.
    crate::dom::add_class("voiceSpellMic", "btn-hide");
    spawn_local(async move {
        let cap = native_lang::speech_capabilities(&lang).await;
        crate::dom::toggle_class("voiceSpellMic", "btn-hide", !cap.available);
    });
}

/// Attach the mic handlers — ONLY when the flag is on. Flag off ⇒ nothing wired.
pub fn wire(app: &App) {
    if !enabled() {
        return;
    }
    let a = app.clone();
    crate::dom::on_click("voiceSpellMic", move || mic_tap(&a));
    crate::dom::on_click("voiceSpellPermClose", || {
        crate::dom::add_class("voiceSpellPerm", "btn-hide");
    });
}

fn set_status(key: &str) {
    let text = if key.is_empty() { String::new() } else { crate::i18n::t(key) };
    crate::dom::set_text("voiceSpellStatus", &text);
}

fn end_capture_ui() {
    CAPTURING.with(|c| c.set(false));
    crate::dom::remove_class("voiceSpellMic", "listening");
}

/// Mic tap: start letter capture, or stop/finalize if already capturing (a second
/// tap or ~2s of silence ends it). NEVER auto-submits; NEVER clears typed text.
pub fn mic_tap(app: &App) {
    if !enabled() {
        return;
    }
    if CAPTURING.with(Cell::get) {
        native_lang::stop_letter_capture();
        return;
    }
    let (lang, base, can) = {
        let s = app.borrow();
        (s.lang.clone(), s.answer.clone(), crate::game::can_type(&s))
    };
    if !can {
        return;
    }
    // The target word is deliberately NOT read here (answer-leak invariant, G-A):
    // the matcher rejects whole words by low letter-yield, never by the answer.
    BASE.with(|b| *b.borrow_mut() = base);
    CAPTURING.with(|c| c.set(true));
    crate::dom::add_class("voiceSpellMic", "listening");
    set_status("voiceSpell.listening");

    let ctx = contextual_strings(&lang);
    let lang_c = lang.clone();
    let a_partial = app.clone();
    let a_final = app.clone();
    let a_error = app.clone();
    let ok = native_lang::start_letter_capture(
        &lang,
        &ctx,
        move |transcript| on_partial(&a_partial, &lang_c, &transcript),
        {
            let lang_f = lang.clone();
            move |transcript| on_final(&a_final, &lang_f, &transcript)
        },
        move |code| on_error(&a_error, &code),
    );
    if !ok {
        // Bridge missing/uncallable — treat as unavailable, revert cleanly.
        end_capture_ui();
        crate::dom::add_class("voiceSpellMic", "btn-hide");
    }
}

/// Live echo: re-parse the growing transcript and preview `base + letters` in the
/// field. Idempotent — re-parsing the full transcript each partial is stable.
fn on_partial(app: &App, lang: &str, transcript: &str) {
    if !CAPTURING.with(Cell::get) {
        return;
    }
    let parsed = parse(lang, transcript);
    let base = BASE.with(|b| b.borrow().clone());
    crate::game::set_answer(app, &format!("{}{}", base, parsed.letters));
}

/// Finalize: commit letters, or revert to the typed base and nudge (whole word) /
/// hint (nothing). Never counts an attempt; never submits.
fn on_final(app: &App, lang: &str, transcript: &str) {
    if !CAPTURING.with(Cell::get) {
        return;
    }
    end_capture_ui();
    let base = BASE.with(|b| b.borrow().clone());
    match interpret(lang, transcript) {
        SpellOutcome::Insert(letters) => {
            crate::game::set_answer(app, &format!("{}{}", base, letters));
            crate::haptics::key_tap();
            set_status("");
        }
        SpellOutcome::WholeWord => {
            crate::game::set_answer(app, &base); // insert nothing; keep typed text
            set_status("voiceSpell.spellItOut");
        }
        SpellOutcome::Nothing => {
            crate::game::set_answer(app, &base);
            set_status("voiceSpell.didntCatch");
        }
    }
}

/// Capture error: never blocks typed input — always revert to the typed base.
fn on_error(app: &App, code: &str) {
    end_capture_ui();
    let base = BASE.with(|b| b.borrow().clone());
    crate::game::set_answer(app, &base);
    match code {
        "PERMISSION_DENIED" => {
            if !EXPLAINED.with(Cell::get) {
                EXPLAINED.with(|c| c.set(true));
                crate::dom::remove_class("voiceSpellPerm", "btn-hide");
            }
        }
        "UNAVAILABLE" => crate::dom::add_class("voiceSpellMic", "btn-hide"),
        _ => set_status("voiceSpell.didntCatch"),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod loopback;
