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
//! player is nudged to spell letter by letter) — see [`interpret`].
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
}

/// A compiled lexicon: the merged phrase→letters table (keys normalized for
/// matching, values NFC), the max phrase length in words, and the original spoken
/// forms to bias the recognizer (`contextualStrings`).
struct Lexicon {
    table: HashMap<String, String>,
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
    contextual.sort();
    contextual.dedup();
    Lexicon { table, max_words, contextual }
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

/// The result of parsing a spoken utterance into letters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    /// The letters the player spelled, NFC-normalized (Invariant I1). Exactly what
    /// a keyboard would have produced.
    pub letters: String,
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
        return Parsed { letters: String::new(), matched_words: 0, total_words: 0 };
    };
    let words: Vec<String> =
        transcript.split_whitespace().map(norm_word).filter(|w| !w.is_empty()).collect();
    let n = words.len();
    let mut out = String::new();
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
                matched += len;
                i += len;
            }
            None => i += 1,
        }
    }
    let letters: String = out.nfc().collect();
    Parsed { letters, matched_words: matched, total_words: n }
}

/// The spoken forms to hand the recognizer as `contextualStrings` for `lang`
/// (letter names + phrases). Biasing lives in the lexicon, not in Swift.
pub fn contextual_strings(lang: &str) -> Vec<String> {
    lexicon(lang).map(|l| l.contextual.clone()).unwrap_or_default()
}

// ---- whole-word rejection (Feature 7) ----

/// Below this parsed-letter yield AND at/above [`WHOLE_WORD_SIM`] similarity to the
/// target, an utterance is treated as the whole word spoken aloud, not a spelling.
const WHOLE_WORD_YIELD: f64 = 0.5;
const WHOLE_WORD_SIM: f64 = 0.6;

/// Case/accent-normalized similarity of the utterance to the target word
/// (1.0 = identical). Whitespace-insensitive via `fold_strict`.
fn similarity(transcript: &str, target: &str) -> f64 {
    let a = crate::norm::fold_strict(transcript);
    let b = crate::norm::fold_strict(target);
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let max = a.len().max(b.len());
    if max == 0 {
        return 0.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / max as f64
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// What the input method should do with a finalized utterance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellOutcome {
    /// Append these letters at the cursor (already NFC).
    Insert(String),
    /// The player said the whole word — nudge "spell it letter by letter", insert
    /// nothing, count no attempt (Feature 7).
    WholeWord,
    /// Nothing usable was heard — a subtle "didn't catch that", insert nothing.
    Nothing,
}

/// Decide what a finalized transcript means for `target` in `lang`. Whole-word
/// rejection requires BOTH a low parsed-letter yield AND a fuzzy match to the
/// target, so genuine letter-by-letter spelling (high yield) is never rejected
/// even if it happens to resemble the word.
pub fn interpret(lang: &str, transcript: &str, target: &str) -> SpellOutcome {
    let parsed = parse(lang, transcript);
    let looks_like_word =
        parsed.total_words > 0 && parsed.yield_ratio() < WHOLE_WORD_YIELD && similarity(transcript, target) >= WHOLE_WORD_SIM;
    if looks_like_word {
        return SpellOutcome::WholeWord;
    }
    if parsed.letters.is_empty() {
        return SpellOutcome::Nothing;
    }
    SpellOutcome::Insert(parsed.letters)
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
    /// The target word for the in-flight capture (whole-word rejection).
    static TARGET: RefCell<String> = const { RefCell::new(String::new()) };
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
    let (lang, target, base, can) = {
        let s = app.borrow();
        (s.lang.clone(), s.word.clone(), s.answer.clone(), crate::game::can_type(&s))
    };
    if !can {
        return;
    }
    BASE.with(|b| *b.borrow_mut() = base);
    TARGET.with(|t| *t.borrow_mut() = target);
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
    let target = TARGET.with(|t| t.borrow().clone());
    match interpret(lang, transcript, &target) {
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
