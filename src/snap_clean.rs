//! CC-SNAP-CLEAN v1 — raw OCR lines to clean word candidates.
//!
//! A parent photographs a worksheet. The worksheet carries list numbers,
//! bullets, headers, fill-in blanks and trailing punctuation, and today every
//! one of those characters is theirs to delete on a phone keyboard. The goal
//! is zero edits on a clean list.
//!
//! The rule that shapes every decision below: **when unsure, flag; never
//! delete.** A wrong silent deletion costs more trust than one extra tap, so
//! a line that contains any letter always produces a candidate (I6), and the
//! only things that vanish are parts with no letters at all.
//!
//! # Two things the spec asked for that this does not do
//!
//! * **No per-language runs.** F1 would detect a list scheme separately for
//!   each contiguous run of lines sharing a `lang`. The census found there is
//!   no per-line language to run on: the bridge takes one language for the
//!   whole call (`call.getString("lang")`) and every line on a page inherits
//!   it. Eric dropped the rule on 2026-09-30, and golden row 19 with it.
//!   Scheme detection therefore runs over the whole page at once. `OcrLine`
//!   keeps its `lang` because F5's bank lookup is per language.
//! * **The apostrophe fold target is chosen, not derived.** D1 says fold to
//!   the form each bank stores; the census found that not one entry in any of
//!   the sixteen banks contains an apostrophe of any kind. Eric chose U+0027
//!   on 2026-09-30, on the reasoning that it is what a phone keyboard emits.

use std::sync::OnceLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

// ------------------------------------------------------------------ types

/// One recognized line, as the platform hands it over.
#[derive(Clone, Debug, Default)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
    /// The page's study language. Not per line — see the module note.
    pub lang: String,
    /// CC-SNAP-CLEAN v1.1 U1 — the per-word boxes the OCR engine already
    /// produced for this line, left-to-right. EMPTY is the normal state
    /// today: CC-SNAP-LIST does not yet pass them (v1.1 census C1 HALTs on
    /// exactly that), so every box-gap path below is dormant until it does.
    /// Carrying the field now means the plumbing lands as data, not as a
    /// change to this file.
    pub boxes: Vec<WordBox>,
    /// CC-SNAP-HIGHLIGHT F2 — set when this line's word sits on a
    /// highlighter mark. Dormant for the same reason as `boxes`: it needs
    /// pixels the core is not handed yet.
    pub highlighted: bool,
    /// The dominant hue of the mark, quantized to 12 buckets (HIGHLIGHT F5).
    pub hue_bucket: Option<u8>,
}

/// Why a candidate is or is not pre-checked. Flags are additive and never
/// remove text; they are the "flag, never delete" rule made visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlagSet {
    /// 3+ whitespace tokens and no bank match. Never set for zh/ja (D7).
    pub multi_word: bool,
    /// The line's OCR confidence was below [`LOW_CONFIDENCE`].
    pub low_confidence: bool,
    /// A decimal digit survived into the text.
    pub has_digits: bool,
    /// A fill-in blank was trimmed off the end (`Name: ______`).
    pub blank: bool,
    /// The bank does not have this word. Checked anyway, and highlighted:
    /// custom words are a core My Words use (D3).
    pub not_in_bank: bool,
    /// This candidate came from splitting one line. Informational.
    pub split: bool,
}

impl FlagSet {
    /// The flags that leave a candidate unchecked. `not_in_bank` and `split`
    /// are deliberately absent.
    fn blocks_check(self) -> bool {
        self.multi_word || self.low_confidence || self.has_digits || self.blank
    }
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub text: String,
    /// Always populated (F7). The review screen may show it on edit; this
    /// module only guarantees it is there.
    pub original_line: String,
    pub lang: String,
    pub flags: FlagSet,
    pub checked: bool,
    /// A trailing bracketed group F3 removed, such as the `(n.)` of
    /// `cat (n.)`. Kept so the screen can show what went.
    pub removed_suffix: Option<String>,
    /// CC-SNAP-CLEAN v1.1 U3 — the unbunching offer for this candidate.
    ///
    /// Named `unbunch`, not `split`: `flags.split` already means "this line
    /// was divided on a separator" from v1 F4, and two fields called split
    /// meaning different things is a bug waiting to be written.
    pub unbunch: Option<SplitSuggestion>,
}

/// D8. One platform: the census found `android/` carries no text recognition
/// at all, so there is no second scale to hold a second number for. Vision
/// reports ~1.0 for clean print and commonly 0.3–0.5 for shaky handwriting.
/// 0.4 is the value `photo_import` already shipped provisionally and Eric
/// left standing on 2026-09-30 rather than measuring a distribution.
pub const LOW_CONFIDENCE: f32 = 0.4;

// ------------------------------------------------------- character classes
//
// `regex` is already a dependency (census C6), so the general categories come
// from it rather than from std: `char::is_alphabetic` is Unicode *Alphabetic*,
// which folds in Other_Alphabetic and therefore counts many combining marks as
// letters. I2 and I3 turn on marks and letters being different things.

fn class(pat: &'static str, cell: &'static OnceLock<Regex>) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("static class pattern"))
}

fn matches(c: char, pat: &'static str, cell: &'static OnceLock<Regex>) -> bool {
    let mut buf = [0u8; 4];
    class(pat, cell).is_match(c.encode_utf8(&mut buf))
}

/// Exposed so the invariant tests can ask the same question the pipeline
/// does. I2 and I3 are about these two categories specifically, and a test
/// that used `char::is_alphabetic` instead would be testing a different
/// boundary from the one the code enforces.
pub fn is_letter(c: char) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(c, r"^\p{L}$", &RE)
}

pub fn is_mark(c: char) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(c, r"^\p{M}$", &RE)
}

fn is_punct(c: char) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(c, r"^\p{P}$", &RE)
}

fn is_digit(c: char) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(c, r"^\p{Nd}$", &RE)
}

// ------------------------------------------------------------- F1 schemes

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scheme {
    Digit,
    Letter,
    Roman,
    CjkNumeral,
    Bullet,
}

const BULLETS: [char; 15] = [
    '\u{2022}', '\u{00B7}', '\u{25E6}', '\u{25AA}', '\u{2023}', '\u{2043}', '-', '\u{2013}',
    '\u{2014}', '*', '\u{25CB}', '\u{25CF}', '\u{25A0}', '\u{25A1}', '\u{2713}',
];
const SEPARATORS: [char; 8] = ['.', ')', ']', ':', '\u{FF1A}', '\u{3001}', '\u{FF0E}', '\u{FF09}'];
const CJK_NUMERALS: [char; 10] = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十'];
const ROMAN: [char; 14] = ['i', 'v', 'x', 'l', 'c', 'd', 'm', 'I', 'V', 'X', 'L', 'C', 'D', 'M'];
/// F2's misread alphabet: Vision reads `1.` as `l.` and `0` as `O`.
const DIGITISH: [char; 9] = ['0', 'l', 'I', '|', 'O', 'o', '1', '7', '9'];

/// How many characters a scheme's marker occupies at the head of `line`, or
/// `None` when the line does not open with that scheme.
///
/// Returns a CHARACTER count, not a byte count, because callers slice by
/// chars and a Devanagari or CJK line would otherwise split mid-codepoint.
fn marker_len(line: &str, scheme: Scheme) -> Option<usize> {
    // Leading whitespace and stray punctuation are part of the marker's run.
    //
    // This is what makes the pipeline idempotent (I7), and it took the
    // proptest two rounds to pin down. F3 trims junk off the FRONT of a line,
    // so a marker hidden behind it on the first pass is exposed on the
    // second: ["1.нет", "  1.cat"] detected no scheme (one match, below the
    // two-line floor), stripped nothing, and then stripped everything when
    // re-run on its own output. Then ",1.cat" did the same with a comma.
    // Detection has to see the line the way F3 will leave it.
    //
    // Openers and bullets are exempt because they ARE markers: skipping the
    // "(" of "(3)" or the "•" of a bullet list would hide the thing we are
    // looking for. The skip only takes effect when a marker actually follows
    // -- otherwise marker_len returns None and nothing is sliced -- so a word
    // like 'tis keeps its apostrophe.
    let skippable = |c: char| {
        c.is_whitespace()
            || (is_punct(c) && !matches!(c, '(' | '[' | '\u{FF08}') && !BULLETS.contains(&c))
    };
    let lead: usize = line.chars().take_while(|c| skippable(*c)).count();
    // Brackets and bullets are exempt from the skip above, because either can
    // BEGIN a marker. When one does not, it is junk that F3 trims -- and F3
    // trims a RUN of it, not one character. So detection tries every offset
    // the leading run can expose, nearest first, and takes the first marker
    // it finds.
    //
    // This is the whole of the I7 story, and the proptest told it one
    // character at a time: "  1.cat" (whitespace), ",1.cat" (punctuation),
    // "(n.)нет" (a bracket that never closes), "•1.нет" (a bullet followed by
    // a digit), and finally "•(n.)нет", which needed two characters skipped
    // at once. Stated generally instead of patched case by case: detection
    // must see the line the way F3 will leave it.
    let lead_max: usize =
        line.chars().take_while(|c| c.is_whitespace() || is_punct(*c)).count();
    for offset in lead..=lead_max {
        if let Some(n) = marker_len_at(line, offset, scheme) {
            return Some(n);
        }
    }
    None
}

/// `marker_len`, starting at a known offset. Split out so the caller can try
/// a leading bracket both as part of a marker and as junk in front of one.
fn marker_len_at(line: &str, lead: usize, scheme: Scheme) -> Option<usize> {
    let chars: Vec<char> = line.chars().skip(lead).collect();
    if chars.is_empty() {
        return None;
    }
    let plus = |n: usize| Some(n + lead);
    if scheme == Scheme::Bullet {
        if !BULLETS.contains(&chars[0]) {
            return None;
        }
        // A bullet must be followed by whitespace or a letter, so a hyphenated
        // word at the start of a line ("-ing endings") is not a bullet.
        return match chars.get(1) {
            Some(&c) if c.is_whitespace() || is_letter(c) => plus(1),
            None => plus(1),
            _ => None,
        };
    }

    let mut i = 0;
    let opened = chars[0] == '(' || chars[0] == '[' || chars[0] == '\u{FF08}';
    if opened {
        i = 1;
    }
    let start = i;
    let token_ok = |c: char| match scheme {
        Scheme::Digit => is_digit(c) || DIGITISH.contains(&c),
        Scheme::Letter => c.is_ascii_alphabetic(),
        Scheme::Roman => ROMAN.contains(&c),
        Scheme::CjkNumeral => CJK_NUMERALS.contains(&c),
        Scheme::Bullet => false,
    };
    while i < chars.len() && i - start < 3 && token_ok(chars[i]) {
        i += 1;
    }
    let token: Vec<char> = chars[start..i].to_vec();
    if token.is_empty() {
        return None;
    }
    // A Letter marker is a SINGLE letter: `a.` is a marker, `in.` is a word
    // with a full stop. Roman keeps up to three so `iii.` works.
    if scheme == Scheme::Letter && token.len() != 1 {
        return None;
    }
    // F2 — the misread repair, and the reason Digit is not just \p{Nd}: a
    // list whose first line Vision read as `l.` must still detect as Digit,
    // or golden row 2 has one matching line and never reaches the 2-line
    // minimum. A run counts when it holds a real digit, or when it is a lone
    // `l`, `I` or `|` standing in for a one.
    if scheme == Scheme::Digit {
        let has_real = token.iter().any(|&c| is_digit(c));
        let lone_one = token.len() == 1 && matches!(token[0], 'l' | 'I' | '|');
        if !has_real && !lone_one {
            return None;
        }
    }
    // Every scheme but Bullet needs a separator.
    let sep = *chars.get(i)?;
    if !SEPARATORS.contains(&sep) {
        return None;
    }
    i += 1;
    if opened && chars.get(i - 1) != Some(&')') && chars.get(i - 1) != Some(&']') {
        // `(1)` closes with the separator itself; `(1.` is not a marker.
        if !matches!(sep, ')' | ']' | '\u{FF09}') {
            return None;
        }
    }
    plus(i)
}

/// F1 — the list's scheme, or `None` when no scheme covers enough of it.
///
/// D4: at least half the lines and at least two of them. One stray `a.` in a
/// prose page must never trigger letter stripping.
pub fn detect_scheme(lines: &[String]) -> Option<Scheme> {
    let all = [Scheme::Digit, Scheme::Letter, Scheme::Roman, Scheme::CjkNumeral, Scheme::Bullet];
    let mut counts: Vec<(Scheme, usize, usize)> = Vec::new(); // scheme, lines, longest token
    for s in all {
        let mut n = 0;
        let mut longest = 0;
        for l in lines {
            if let Some(len) = marker_len(l, s) {
                n += 1;
                longest = longest.max(len);
            }
        }
        counts.push((s, n, longest));
    }
    let best = counts.iter().map(|c| c.1).max().unwrap_or(0);
    if best < 2 || best * 2 < lines.len() {
        return None;
    }
    let tied: Vec<&(Scheme, usize, usize)> = counts.iter().filter(|c| c.1 == best).collect();
    if tied.len() == 1 {
        return Some(tied[0].0);
    }
    // Letter vs Roman tie: Roman when any matched token ran to two or more
    // characters (`ii`, `iv`), because a single `i.` is equally a letter.
    let letter = tied.iter().find(|c| c.0 == Scheme::Letter);
    let roman = tied.iter().find(|c| c.0 == Scheme::Roman);
    if let (Some(l), Some(r)) = (letter, roman) {
        // `longest` counts the separator too, so 2 chars means a 1-char token.
        return Some(if r.2 > l.2 { Scheme::Roman } else { Scheme::Letter });
    }
    Some(tied[0].0)
}

// ------------------------------------------------------------ F3 trimming

/// Strip one trailing bracketed group: `cat (n.)` -> (`cat`, `(n.)`).
fn strip_trailing_parenthetical(s: &str) -> (String, Option<String>) {
    let t = s.trim_end();
    let (open, close) = match t.chars().last() {
        Some(')') => ('(', ')'),
        Some(']') => ('[', ']'),
        _ => return (s.to_string(), None),
    };
    let chars: Vec<char> = t.chars().collect();
    let mut depth = 0i32;
    for (i, &c) in chars.iter().enumerate().rev() {
        if c == close {
            depth += 1;
        } else if c == open {
            depth -= 1;
            if depth == 0 {
                let head: String = chars[..i].iter().collect();
                // Only when a WORD is left. "(n.)" alone is not a suffix, and
                // neither is the "(n.)" of "•(n.)" -- stripping there would
                // delete the only letters on the line and return nothing,
                // which I6 forbids and which the proptest caught.
                if !head.chars().any(is_letter) {
                    return (s.to_string(), None);
                }
                let suffix: String = chars[i..].iter().collect();
                return (head.trim_end().to_string(), Some(suffix));
            }
        }
    }
    (s.to_string(), None)
}

/// F3 — trim punctuation from the ends, never from inside a word.
///
/// Trimming only from the ends is what protects `don't`, `l'eau`,
/// `well-being` and Catalan `l·l` without a special case for any of them: an
/// interior character has letters on both sides by definition, and this loop
/// never reaches it. Marks and letters are never removed (I2, I3).
fn trim_edges(s: &str) -> (String, bool) {
    let chars: Vec<char> = s.trim().chars().collect();
    let mut start = 0;
    let mut end = chars.len();
    while start < end && (is_punct(chars[start]) || chars[start].is_whitespace()) {
        start += 1;
    }
    while end > start && (is_punct(chars[end - 1]) || chars[end - 1].is_whitespace()) {
        end -= 1;
    }
    let tail: String = chars[end..].iter().collect();
    // A fill-in blank: three or more underscores or dots in a row in what we
    // just removed. `Name: ______` is a form field, not a word list entry.
    let blank = has_run_of_3(&tail, '_') || has_run_of_3(&tail, '.');
    (chars[start..end].iter().collect(), blank)
}

fn has_run_of_3(s: &str, target: char) -> bool {
    let mut run = 0;
    for c in s.chars() {
        run = if c == target { run + 1 } else { 0 };
        if run >= 3 {
            return true;
        }
    }
    false
}

// ------------------------------------------------------------ the pipeline

const SPLITTERS: [char; 7] = [',', ';', '/', '|', '\u{FF0C}', '\u{FF1B}', '\u{3001}'];

/// Remove ONE leading marker, unless doing so would take the last letters
/// with it.
///
/// Exactly one, which is I4 as written. A line carrying two markers still
/// loses both -- "1.1. cat" is the sub-numbering a worksheet really uses --
/// but it loses them across two rounds of the fixed-point loop below, not in
/// one call here. That is the difference between satisfying the invariant and
/// reinterpreting it, and an earlier draft of this function did the latter:
/// it looped internally, which made I7 reachable but left I4 true only in
/// spirit. The loop belongs to the pipeline, not to the strip.
///
/// The letters guard is I6. "a),a)" is nothing but two Letter markers, and
/// stripping them left the line with no candidate at all -- flag, never
/// delete, so a line that is only a marker still shows the parent something.
fn strip_marker(text: &str, scheme: Option<Scheme>) -> String {
    let Some(n) = scheme.and_then(|s| marker_len(text, s)) else {
        return text.to_string();
    };
    let body: String = text.chars().skip(n).collect();
    if text.chars().any(is_letter) && !body.chars().any(is_letter) {
        return text.to_string();
    }
    body
}

/// F5's bank lookup, behind the same wall as the index it uses.
///
/// `word_index` is `#[cfg(not(feature = "web"))]` — Letter Forge and Word
/// Chains are app-only, and the index exists for them. C4 says not to build
/// a second index, so this does not: on web there is simply no bank opinion,
/// every candidate is `NotInBank`, and no casing is applied. That is inert
/// rather than wrong, because the photo path is iOS-only (`photo_list` is
/// `platforms: ["ios"]` in the registry) — the site never reaches this code
/// with a photograph. The alternative was duplicating `in_banks`' linear
/// scan to return a form instead of a bool, which is the second lookup C4
/// warned about.
#[cfg(not(feature = "web"))]
fn bank_lookup(lang: &str, word: &str) -> Option<String> {
    crate::word_index::canonical(lang, word)
}

#[cfg(feature = "web")]
fn bank_lookup(_lang: &str, _word: &str) -> Option<String> {
    None
}

/// F5 — one apostrophe form. Eric chose U+0027 (2026-09-30) because no bank
/// holds one to copy and a phone keyboard emits the straight quote.
fn fold_apostrophes(s: &str) -> String {
    s.replace('\u{2019}', "'").replace('\u{02BC}', "'")
}


/// CC-SNAP-CLEAN v1.1 U1 — one word box, as the OCR engine reported it.
///
/// `x0`/`x1` are the horizontal edges in the cropped image's own pixel space.
/// Only the horizontal axis is carried: unbunching asks whether two boxes on
/// one line are separated, and nothing here needs their height.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WordBox {
    pub text: String,
    pub x0: f32,
    pub x1: f32,
    pub confidence: f32,
}

/// CC-SNAP-CLEAN v1.1 U3 — the pieces a merged token would become.
///
/// `applied` distinguishes the two surfaces the review screen owes the
/// reader: `true` means the split already happened and the first piece needs
/// a Rejoin control; `false` means the merged token is still intact and wants
/// a "Split into a / b / c?" chip. This file only guarantees the field.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitSuggestion {
    pub pieces: Vec<String>,
    pub applied: bool,
}

/// What a language permits the dictionary splitter to do (v1.1 D-U1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitPolicy {
    /// Split without asking, and offer Rejoin.
    Auto,
    /// Never split on dictionary evidence alone; offer a chip.
    Suggest,
    /// No dictionary splitting at all. Box gaps are still honoured.
    Never,
}

/// D-U1's table, written out rather than defaulted.
///
/// An unknown language returns `Never`, not `Auto`. C5 says a language
/// missing from this table HALTs; in code the equivalent is to do the
/// least possible, because a new language silently inheriting auto-split
/// is exactly the failure C5 is guarding against.
pub fn split_policy(lang: &str) -> SplitPolicy {
    match lang {
        "en" | "es" | "fr" | "pt" | "pl" | "ru" | "fil" | "sw" | "hi" | "ar" => SplitPolicy::Auto,
        // Compounds and spacing conventions here are real words even when the
        // bank has never heard of them.
        "de" | "ko" | "vi" => SplitPolicy::Suggest,
        // No inter-word space to recover; a box gap is still a gap.
        "zh" | "ja" => SplitPolicy::Never,
        _ => SplitPolicy::Never,
    }
}


/// Is auto-split allowed to act, or only to offer?
///
/// FALSE, and deliberately, pending Eric's decision. Every D-U decision in
/// v1.1 is "Signed: none yet", so shipping auto-split would ship an unsigned
/// behaviour -- and measurement says that behaviour is not safe at the bank's
/// current size.
///
/// Measured 2026-10-01 against the 36,000-word frequency list in
/// tools/wordpipe/sources/freq_en.txt: of the 33,068 real English words the
/// 3,165-word bank does NOT contain, **2,218 (6.71%) have a unique
/// segmentation into bank words** and would therefore be split without being
/// asked. Not edge cases:
///
/// ```text
/// maybe    -> may + be        tomorrow -> tom + or + row
/// listen   -> list + en       asking   -> as + king
/// yourself -> your + self     ladies   -> la + dies
/// everyone -> every + one     nobody   -> no + body
/// ```
///
/// D-U5 does not help: it suspends the split on a TIE, and every one of these
/// is unambiguous. The cause is not the ranking, it is that a 3,165-word bank
/// is 2% of English and makes a poor dictionary.
///
/// With this false the whole stage still runs and still attaches its
/// `SplitSuggestion` -- the reader is offered the split and can take it in one
/// tap, which is what the file's own Intent asks for ("a one-tap suggestion
/// over a silent change"). Flipping this to true activates D-U1 exactly as
/// written, and is a one-line change once Eric has ruled.
const AUTO_SPLIT_ENABLED: bool = false;

/// D-U4. A gap at least this fraction of the median glyph width is a space
/// the OCR engine dropped. Starting value; the census may move it.
const GAP_RATIO: f32 = 0.35;

/// D-U3. Pieces shorter than this are not considered, except for a language's
/// own one-letter bank entries.
const MIN_PIECE_LEN: usize = 2;

/// Longest token the dictionary splitter will consider.
///
/// The search is O(n^2) bank probes and each probe is a binary search, so the
/// real cost is small -- but a 45-character token (MAX_WORD_LENGTH) is not a
/// merged phrase, it is OCR noise, and running a segmentation search over it
/// buys nothing.
const UNBUNCH_MAX_LEN: usize = 24;

/// U1 — split one line's boxes wherever the page shows a gap.
///
/// This is reading the page, not guessing at it, which is why it runs for
/// every language including zh/ja (D-U1 governs only the dictionary path).
/// The median glyph width is estimated per line from the boxes themselves:
/// box width over character count, which is crude but scale-free, and the
/// comparison is a ratio so it survives any resolution.
///
/// Returns one string per gap-separated run, or a single element when the
/// line has no usable boxes -- so a caller can always use the result.
pub fn split_on_box_gaps(line: &OcrLine) -> Vec<String> {
    let boxes: Vec<&WordBox> = line.boxes.iter().filter(|b| !b.text.trim().is_empty()).collect();
    if boxes.len() < 2 {
        return vec![line.text.clone()];
    }
    let mut widths: Vec<f32> = boxes
        .iter()
        .filter(|b| b.text.chars().count() > 0 && b.x1 > b.x0)
        .map(|b| (b.x1 - b.x0) / b.text.chars().count() as f32)
        .collect();
    if widths.is_empty() {
        return vec![line.text.clone()];
    }
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let glyph = widths[widths.len() / 2];
    let threshold = GAP_RATIO * glyph;

    let mut runs: Vec<String> = vec![boxes[0].text.clone()];
    for pair in boxes.windows(2) {
        let (prev, next) = (pair[0], pair[1]);
        if next.x0 - prev.x1 >= threshold {
            runs.push(next.text.clone());
        } else {
            // No gap on the page: the engine split a word that is printed
            // solid. Rejoining here is what stops U2 being asked to re-merge.
            runs.last_mut().expect("seeded above").push_str(&next.text);
        }
    }
    runs
}

/// Is `w` usable as a piece of a split?
fn piece_ok(lang: &str, w: &str) -> bool {
    let n = w.chars().count();
    if n == 0 {
        return false;
    }
    // A one-letter piece is allowed only where the bank itself holds that
    // word. The whitelist is the bank, never a hand-typed list -- and note
    // that as of 2026-10-01 no bank holds a one-letter entry in any of the
    // fifteen, so in practice this clause admits nothing yet.
    if n < MIN_PIECE_LEN {
        return bank_lookup(lang, w).is_some();
    }
    bank_lookup(lang, w).is_some()
}

/// U2 — the dictionary split.
///
/// Returns the best segmentation and whether it is unambiguous. "Best" is
/// fewest pieces; "unambiguous" means exactly one segmentation achieves that
/// minimum. Both come out of one dynamic program over suffix positions, which
/// matters: the spec asks to "compute every segmentation", and enumerating
/// them is exponential in the worst case, while the only facts the ranking
/// needs are the minimum piece count and whether more than one way reaches it.
///
/// `None` means no segmentation into bank words exists, which is the common
/// answer and the safe one.
fn dictionary_split(token: &str, lang: &str) -> Option<(Vec<String>, bool)> {
    let chars: Vec<char> = token.chars().collect();
    let n = chars.len();
    if n < MIN_PIECE_LEN * 2 || n > UNBUNCH_MAX_LEN {
        return None;
    }
    // best[i] = (min pieces for chars[i..], ways achieving it, first cut)
    // ways saturates at 2: "one way" and "more than one" is the whole question.
    let mut best: Vec<Option<(usize, u8, usize)>> = vec![None; n + 1];
    best[n] = Some((0, 1, n));
    for i in (0..n).rev() {
        let mut found: Option<(usize, u8, usize)> = None;
        for j in (i + 1)..=n {
            let piece: String = chars[i..j].iter().collect();
            if !piece_ok(lang, &piece) {
                continue;
            }
            let Some((rest_count, rest_ways, _)) = best[j] else { continue };
            let count = rest_count + 1;
            found = Some(match found {
                None => (count, rest_ways, j),
                Some((bc, _, _)) if count < bc => (count, rest_ways, j),
                Some((bc, bw, bj)) if count == bc => (bc, bw.saturating_add(rest_ways).min(2), bj),
                Some(prev) => prev,
            });
        }
        best[i] = found;
    }
    let (count, ways, _) = best[0]?;
    if count < 2 {
        return None; // the whole token is one bank word; I-U2 handles that
    }
    let mut pieces = Vec::new();
    let mut i = 0usize;
    while i < n {
        let (_, _, cut) = best[i].expect("reachable by construction");
        pieces.push(chars[i..cut].iter().collect::<String>());
        i = cut;
    }
    Some((pieces, ways == 1))
}

/// The one entry point. Pure: no I/O, no clock, no randomness (I5).
pub fn clean_ocr_lines(lines: &[OcrLine]) -> Vec<Candidate> {
    // One unit of work: a piece of text on its way to becoming a candidate,
    // plus the line it came from and what has happened to it.
    #[derive(Clone)]
    struct Unit {
        text: String,
        line_ix: usize,
        split: bool,
        blank: bool,
        suffix: Option<String>,
    }

    // U1 runs HERE, before the fixed point, rather than at the position v1.1
    // F4 names (after the marker strip). The effect is the same and the code
    // is honest: a marker attaches to the first box, so splitting first leaves
    // it on the first unit, where the loop strips it exactly as before. Doing
    // it inside the loop would mean re-splitting text that no longer has
    // boxes attached to it.
    //
    // With no boxes -- which is every caller today -- this is the identity.
    let mut units: Vec<Unit> = lines
        .iter()
        .enumerate()
        .flat_map(|(line_ix, l)| {
            split_on_box_gaps(l).into_iter().map(move |t| Unit {
                text: t.nfc().collect(),
                line_ix,
                split: false,
                blank: false,
                suffix: None,
            })
        })
        .collect();

    // F1–F4, run to a FIXED POINT rather than once.
    //
    // I7 asks that cleaning the cleaner's own output change nothing, and a
    // single pass cannot promise that: every stage alters what the next
    // election sees. Stripping a Letter marker can expose a Digit one;
    // trimming junk off the front can expose a marker that was hidden behind
    // it; splitting a line can produce a part carrying its own marker. The
    // proptest found five of these in a row -- "  1.cat", ",1.cat",
    // "(n.)нет", "•1.нет", "•(n.)нет" -- and each individual patch simply
    // moved the failure somewhere else. The last one was not a stripping bug
    // at all but an election one: Letter won on the raw page and Digit won on
    // the cleaned page, so the two passes disagreed about what the markers
    // even were.
    //
    // Iterating to a fixed point makes the invariant structural instead of a
    // list of cases, and it is also what lets `strip_marker` take exactly one
    // marker and stay literally within I4: a doubled "1.1." loses both halves
    // across two ROUNDS rather than two strips. The cap is a guard against a
    // rule that never settles, not an expected path; four rounds is far past
    // anything a worksheet has produced.
    for _ in 0..4 {
        let texts: Vec<String> = units.iter().map(|u| u.text.clone()).collect();
        let scheme = detect_scheme(&texts);
        let mut next: Vec<Unit> = Vec::new();
        for u in &units {
            let body = strip_marker(&u.text, scheme);
            let parts: Vec<&str> = body.split(|c| SPLITTERS.contains(&c)).collect();
            let split = u.split || parts.iter().filter(|p| !p.trim().is_empty()).count() >= 2;
            for part in parts {
                if part.trim().is_empty() {
                    continue;
                }
                let (part, suffix) = strip_trailing_parenthetical(part);
                let (text, blank) = trim_edges(&part);
                // The only thing that disappears: a piece with no letters at
                // all -- page numbers, stray bullets, empty lines.
                if !text.chars().any(is_letter) {
                    continue;
                }
                next.push(Unit {
                    text,
                    line_ix: u.line_ix,
                    split,
                    blank: u.blank || blank,
                    suffix: u.suffix.clone().or(suffix),
                });
            }
        }
        // Dedupe INSIDE the loop, not after it. D4's threshold is a fraction
        // of the line count, so removing a duplicate can change which scheme
        // wins: a page of five units where two match is below half, and the
        // same page deduped to four is exactly half. The proptest found that
        // on "нет,一、一、(n.)нет" after six thousand cases. The loop has to
        // converge on precisely what it will emit, or "what it emits" and
        // "what it converged on" are different pages.
        let mut deduped: Vec<Unit> = Vec::new();
        let mut keys: Vec<(String, String)> = Vec::new();
        for u in next {
            let key = (
                lines[u.line_ix].lang.clone(),
                crate::norm::fold_lenient(&fold_apostrophes(&u.text)),
            );
            if keys.contains(&key) {
                continue;
            }
            keys.push(key);
            deduped.push(u);
        }
        let settled = deduped.len() == units.len()
            && deduped.iter().zip(units.iter()).all(|(a, b)| a.text == b.text);
        units = deduped;
        if settled {
            break;
        }
    }

    // F5 and F6, once, on the settled text.
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: Vec<(String, String)> = Vec::new(); // (lang, folded text)
    for u in units {
        let line = &lines[u.line_ix];
        let text = fold_apostrophes(&u.text).nfc().collect::<String>();
        let bank = bank_lookup(&line.lang, &text);
        let text = bank.clone().unwrap_or(text);

        let key = (line.lang.clone(), crate::norm::fold_lenient(&text));
        if seen.contains(&key) {
            continue; // D6: duplicates go silently, first wins
        }
        seen.push(key);

        // U2 runs here: after the fixed point has settled the text, before
        // the bank canonicalization below, so each piece is canonicalized on
        // its own. I-U2 is structural -- a token the bank already knows took
        // the `bank.is_some()` branch and is never offered a split.
        let policy = split_policy(&line.lang);
        let unbunch = if bank.is_some() || policy == SplitPolicy::Never {
            None
        } else {
            dictionary_split(&text, &line.lang).map(|(pieces, unambiguous)| SplitSuggestion {
                applied: AUTO_SPLIT_ENABLED && policy == SplitPolicy::Auto && unambiguous,
                pieces,
            })
        };

        let cjk = matches!(line.lang.as_str(), "zh" | "ja");
        let flags = FlagSet {
            multi_word: !cjk && bank.is_none() && text.split_whitespace().count() >= 3,
            low_confidence: line.confidence < LOW_CONFIDENCE,
            has_digits: text.chars().any(is_digit),
            blank: u.blank,
            not_in_bank: bank.is_none(),
            split: u.split,
        };
        // An applied split emits its pieces as separate candidates; the
        // merged token does not survive, which is what makes Rejoin a real
        // control rather than a second row to delete.
        if let Some(sug) = unbunch.as_ref().filter(|s| s.applied) {
            for (n, piece) in sug.pieces.iter().enumerate() {
                let canon = bank_lookup(&line.lang, piece).unwrap_or_else(|| piece.clone());
                out.push(Candidate {
                    text: canon,
                    original_line: line.text.clone(),
                    lang: line.lang.clone(),
                    checked: !flags.blocks_check(),
                    flags: FlagSet { not_in_bank: false, ..flags },
                    removed_suffix: u.suffix.clone(),
                    // Only the first piece carries the offer, so the review
                    // screen has one Rejoin per split and not one per piece.
                    unbunch: (n == 0).then(|| sug.clone()),
                });
            }
            continue;
        }
        out.push(Candidate {
            text,
            original_line: line.text.clone(),
            lang: line.lang.clone(),
            checked: !flags.blocks_check(),
            flags,
            removed_suffix: u.suffix,
            unbunch,
        });
    }
    out
}

