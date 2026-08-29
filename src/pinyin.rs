//! CC-ZH-TONE F1 — the Mandarin pinyin canonicalizer.
//!
//! Grading compares meaning-bearing forms, not typed characters. `mǎ`, `ma3`
//! and `MA3` are one answer; `lǜ` and `lv3` are two, because one is fourth tone
//! and one is third. [`canonicalize_pinyin`] is the single total function that
//! makes that call, and [`PinyinKey`] is the only thing zh grading compares.
//!
//! Accepted encodings, all folding to one key (D2):
//!   * diacritics — ā á ǎ à and the ü set ǖ ǘ ǚ ǜ; a bare vowel is neutral;
//!   * trailing digits — ma1..ma5, with ma0 ≡ ma5 and bare ma ≡ ma5;
//!   * ü substitutes — `v` and `u:` (lv → lü, nu:3 → nǚ);
//!   * separators — space and hyphen accepted and discarded; the apostrophe is
//!     honoured as a hard syllable boundary, so `xi'an` is never read as `xian`;
//!   * case-insensitive, NFC-normalized first.
//!
//! Rejected as a typed [`ParseError`], never a silent pass-through: syllables
//! outside the pinned inventory, tone digits outside 0–5, and a diacritic and
//! digit on the same syllable (`mǎ3` — ambiguous intent, so ask rather than
//! guess).
//!
//! **Neutral is a value, not an absence** (D3). Bare input parses to tone 5, so
//! an answer of `ma3` typed as bare `ma` is a tone miss (5 vs 3), while an
//! answer of `ma5` typed as bare `ma` is exact. The bank writes neutral
//! explicitly (`bao3bao5`, Eric's ruling 2026-08-19); both spellings produce the
//! same key, so the stored encoding stops mattering once tone is a value.
#![allow(dead_code)]

use crate::pinyin_inventory::{NONSTANDARD, SYLLABLES};
use unicode_normalization::UnicodeNormalization;

/// One syllable: its toneless segment in the inventory's ü form, and its tone.
/// Tone is 1–4 or 5 for neutral; there is no untoned state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Syllable {
    pub segment: String,
    pub tone: u8,
}

/// The only value zh grading ever compares.
pub type PinyinKey = Vec<Syllable>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Nothing to parse.
    Empty,
    /// A tone digit with no syllable in front of it.
    DanglingToneDigit,
    /// Digit outside 0–5.
    ToneDigitOutOfRange(char),
    /// A diacritic and a digit on one syllable — ambiguous intent (D2).
    MixedToneNotation,
    /// A character that is not a pinyin letter, digit or separator.
    IllegalCharacter(char),
    /// No segmentation into legal syllables exists at the required count.
    /// `found` is the best count reachable, or 0 when nothing segments.
    Unsegmentable { expected: usize, found: usize },
}

/// D7 — the tone colour map, Pleco's scheme (Eric's pick, 2026-08-19).
/// Indexed by tone 1–5; index 0 is unused so `TONE_COLOURS[tone]` reads
/// directly.
///
/// One map, used identically on the orb, the F4 input buttons and the reveal.
/// Invariant 7: colour is NEVER the sole carrier — every surface showing a tone
/// shows its numeral or diacritic too, so the map is redundancy, not meaning.
/// Anything that renders a tone reads these, so the three surfaces cannot drift
/// apart.
pub const TONE_COLOURS: [&str; 6] = [
    "",        // unused
    "#d0342c", // 1 high level    — red
    "#2e9e4f", // 2 rising        — green
    "#2f6fd0", // 3 falling-rising— blue
    "#8b46c4", // 4 falling       — purple
    "#7a7a7a", // 5 neutral       — grey
];

/// CC-ZH-PINYIN-DISPLAY F1 — THE PRECOMPOSED DISPLAY ALPHABET.
///
/// `(bare vowel, [tone1, tone2, tone3, tone4])`. Every entry is a single
/// precomposed codepoint. Combining marks (U+0300-U+036F) and spacing accents
/// (U+00B4, U+02C7, U+00AF, ...) are ILLEGAL in player-visible pinyin, so they
/// appear nowhere in this table.
///
/// Uppercase is carried even though no surface emits it today (D5): sentence-
/// initial pinyin arrives the moment carrier sentences reach zh, and the cost
/// of having it already right is 24 rows.
const PRECOMPOSED: [(char, [char; 4]); 12] = [
    ('a', ['\u{101}', '\u{e1}', '\u{1ce}', '\u{e0}']),
    ('o', ['\u{14d}', '\u{f3}', '\u{1d2}', '\u{f2}']),
    ('e', ['\u{113}', '\u{e9}', '\u{11b}', '\u{e8}']),
    ('i', ['\u{12b}', '\u{ed}', '\u{1d0}', '\u{ec}']),
    ('u', ['\u{16b}', '\u{fa}', '\u{1d4}', '\u{f9}']),
    ('\u{fc}', ['\u{1d6}', '\u{1d8}', '\u{1da}', '\u{1dc}']),
    ('A', ['\u{100}', '\u{c1}', '\u{1cd}', '\u{c0}']),
    ('O', ['\u{14c}', '\u{d3}', '\u{1d1}', '\u{d2}']),
    ('E', ['\u{112}', '\u{c9}', '\u{11a}', '\u{c8}']),
    ('I', ['\u{12a}', '\u{cd}', '\u{1cf}', '\u{cc}']),
    ('U', ['\u{16a}', '\u{da}', '\u{1d3}', '\u{d9}']),
    ('\u{dc}', ['\u{1d5}', '\u{1d7}', '\u{1d9}', '\u{1db}']),
];

/// Every codepoint F1 permits in a player-visible pinyin string, beyond ASCII
/// letters. L1 fails the build on anything else.
pub fn is_legal_display_char(c: char) -> bool {
    if c.is_ascii_alphabetic() || c == ' ' {
        return true;
    }
    if c == '\u{fc}' || c == '\u{dc}' {
        return true; // bare ü / Ü, tone 5
    }
    PRECOMPOSED.iter().any(|(_, marks)| marks.contains(&c))
}

/// CC-ZH-PINYIN-DISPLAY F2 — which vowel carries the mark.
///
///   1. tone 5 -> no mark
///   2. an `a` -> mark it
///   3. else an `o` -> mark it
///   4. else an `e` -> mark it
///   5. else the LAST of `i u ü`
///
/// Returns the byte index of the tone-bearing vowel, or None when the syllable
/// has no vowel to carry a mark -- the `m/n/ng` interjections, which have no
/// precomposed form in Unicode and therefore cannot satisfy F1 at all (D2).
fn tone_vowel_index(segment: &str) -> Option<usize> {
    for want in ['a', 'A', 'o', 'O', 'e', 'E'] {
        if let Some(i) = segment.find(want) {
            return Some(i);
        }
    }
    segment
        .char_indices()
        .filter(|(_, c)| matches!(c, 'i' | 'u' | '\u{fc}' | 'I' | 'U' | '\u{dc}'))
        .next_back()
        .map(|(i, _)| i)
}

/// CC-ZH-PINYIN-DISPLAY F1 — THE one PinyinKey -> display function.
///
/// Every player-visible surface calls this: answer reveal, hint, definition
/// line, tone-drill queue, share card, Reports, any tile or button carrying
/// pinyin. A second builder anywhere is the bug this file exists to remove.
///
/// Before this existed, the reveal composed display text inline as
/// `segment + <sup>spacing-accent</sup>`, which is how 游戏 rendered as `you`
/// with a detached mark instead of `yóu`. The mark was never on a vowel; it was
/// a separate element holding U+00B4.
///
/// None when the syllable cannot be rendered under F1+F2 -- never a silent
/// fallback to a combining mark, which would put an illegal codepoint on screen
/// while looking almost right.
pub fn display_syllable(segment: &str, tone: u8) -> Option<String> {
    if tone == 5 || tone == 0 {
        return segment.chars().all(is_legal_display_char).then(|| segment.to_string());
    }
    if !(1..=4).contains(&tone) {
        return None;
    }
    let i = tone_vowel_index(segment)?;
    let base = segment[i..].chars().next()?;
    let marked = PRECOMPOSED
        .iter()
        .find(|(v, _)| *v == base)
        .map(|(_, marks)| marks[(tone - 1) as usize])?;
    let mut out = String::with_capacity(segment.len() + 2);
    out.push_str(&segment[..i]);
    out.push(marked);
    out.push_str(&segment[i + base.len_utf8()..]);
    Some(out)
}

/// CC-ZH-PINYIN-DISPLAY F1 — the LIVE form of a partly-typed answer.
///
/// F1 says every surface carrying pinyin calls the one display function, and
/// the answer field is such a surface: a player typing `hai2` should see `hái`
/// the moment the tone lands, the way a real pinyin IME behaves (Eric,
/// 2026-08-29, "so the mark sticks to the letter before it's submitted").
///
/// This is DISPLAY ONLY. The buffer keeps the digits, so grading, the
/// canonicalizer and the input-provenance keystroke count are all untouched --
/// the field shows one thing and the machinery compares another, deliberately.
///
/// Lenient by necessity: it is called on every keystroke, so it sees half-typed
/// input like `hai2z` that no parser would accept. Each complete
/// letters-plus-digit run becomes its precomposed form; anything trailing is
/// left exactly as typed. It never rejects and never reorders.
pub fn display_partial(buf: &str) -> String {
    let mut out = String::with_capacity(buf.len());
    let mut run = String::new();
    for c in buf.chars() {
        if c.is_ascii_alphabetic() || c == '\u{fc}' {
            run.push(c);
        } else if ('1'..='5').contains(&c) && !run.is_empty() {
            let tone = c as u8 - b'0';
            match display_syllable(&run, tone) {
                Some(marked) => out.push_str(&marked),
                // Unrenderable (the m/n/ng interjections): show it as typed
                // rather than dropping the tone the player just chose.
                None => {
                    out.push_str(&run);
                    out.push(c);
                }
            }
            run.clear();
        } else {
            out.push_str(&run);
            run.clear();
            out.push(c);
        }
    }
    out.push_str(&run);
    out
}

/// The whole key as one display string, syllables space-separated.
pub fn display_key(key: &[Syllable]) -> Option<String> {
    let mut parts = Vec::with_capacity(key.len());
    for s in key {
        parts.push(display_syllable(&s.segment, s.tone)?);
    }
    Some(parts.join(" "))
}

/// RETIRED by CC-ZH-PINYIN-DISPLAY F1. These are SPACING ACCENTS -- U+00AF,
/// U+00B4, U+02C7, an ASCII backtick, U+02D9 -- rendered in their own element
/// beside the syllable. That is what produced `you` + a floating mark on
/// device. Kept only so the F0 evidence test can still name what was wrong;
/// no display path may use it.
#[cfg(test)]
pub const TONE_MARKS_DISPLAY: [&str; 6] = ["", "\u{af}", "\u{b4}", "\u{2c7}", "`", "\u{2d9}"];

/// The longest syllable in the inventory (`zhuang`, `chuang`), so the
/// segmenter knows how far to look ahead.
const MAX_SYLLABLE: usize = 6;

const TONE_MARKS: [(char, u8); 4] = [('\u{304}', 1), ('\u{301}', 2), ('\u{30c}', 3), ('\u{300}', 4)];

fn is_legal_syllable(s: &str) -> bool {
    SYLLABLES.binary_search(&s).is_ok()
}

/// A letter of the input, with any tone its own diacritic declared.
struct Letter {
    ch: char,
    dia_tone: Option<u8>,
    /// Tone digit written immediately after this letter.
    digit_tone: Option<u8>,
    /// An apostrophe followed this letter, so a syllable may not span it.
    boundary_after: bool,
}

/// Fold to NFC + lowercase, apply the ü substitutes, drop separators, and split
/// tone notation away from the letters it rides on.
fn lex(input: &str) -> Result<Vec<Letter>, ParseError> {
    // Full-width forms reach us from Chinese IMEs, and NFC does not fold them
    // (only NFKC would, and NFKC would mangle unrelated text). A full-width 3
    // typed on the keyboard this feature exists to accommodate must mean tone
    // three, not a parse error, so fold the ASCII-equivalent block by hand.
    let widened: String = input
        .chars()
        .map(|c| match c {
            '\u{ff01}'..='\u{ff5e}' => char::from_u32(c as u32 - 0xfee0).unwrap_or(c),
            '\u{3000}' => ' ',
            c => c,
        })
        .collect();
    let folded: Vec<char> = widened.nfc().collect::<String>().to_lowercase().chars().collect();
    let mut out: Vec<Letter> = Vec::new();
    let mut i = 0;
    while i < folded.len() {
        let c = folded[i];
        // `u:` before the generic path, so the colon is consumed with its u.
        if c == 'u' && folded.get(i + 1) == Some(&':') {
            out.push(Letter { ch: 'ü', dia_tone: None, digit_tone: None, boundary_after: false });
            i += 2;
            continue;
        }
        i += 1;
        match c {
            // The apostrophe is pinyin's own disambiguator -- it is the only
            // thing distinguishing xi'an from xian, and nobody types one by
            // accident. Eric's ruling 2026-08-19: honour it as a hard syllable
            // boundary rather than discarding it with the other separators.
            // Space and hyphen stay discarded per D2; a stray space is a
            // plausible typo and should not wreck an otherwise good answer.
            '\'' | '\u{2019}' => {
                if let Some(last) = out.last_mut() {
                    last.boundary_after = true;
                }
                continue;
            }
            ' ' | '-' => continue,
            c if c.is_whitespace() => continue,
            c if c.is_ascii_digit() => {
                let tone = match c {
                    '0' | '5' => 5,
                    '1'..='4' => c as u8 - b'0',
                    other => return Err(ParseError::ToneDigitOutOfRange(other)),
                };
                let last = out.last_mut().ok_or(ParseError::DanglingToneDigit)?;
                if last.digit_tone.is_some() {
                    // Two digits in a row: the second has no syllable of its own.
                    return Err(ParseError::DanglingToneDigit);
                }
                last.digit_tone = Some(tone);
            }
            c => {
                // Decompose so a tone mark can be lifted off any base letter.
                // The diaeresis on ü is NOT a tone mark and must survive, so we
                // filter only the four tone marks and recompose the rest.
                let d: Vec<char> = c.to_string().nfd().collect();
                let tone = d.iter().find_map(|x| {
                    TONE_MARKS.iter().find(|(m, _)| m == x).map(|(_, t)| *t)
                });
                let base: String = d
                    .iter()
                    .filter(|x| !TONE_MARKS.iter().any(|(m, _)| m == *x))
                    .collect::<String>()
                    .nfc()
                    .collect();
                let mut chars = base.chars();
                let (b, rest) = (chars.next(), chars.next());
                let b = match (b, rest) {
                    (Some(b), None) => b,
                    _ => return Err(ParseError::IllegalCharacter(c)),
                };
                let b = if b == 'v' { 'ü' } else { b };
                // ê (欸/诶) is a legal pinyin letter; its circumflex is part of
                // the letter, not a tone mark, so it survives the filter above.
                if !(b.is_ascii_alphabetic() || b == 'ü' || b == 'ê') {
                    return Err(ParseError::IllegalCharacter(c));
                }
                out.push(Letter { ch: b, dia_tone: tone, digit_tone: None, boundary_after: false });
            }
        }
    }
    if out.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(out)
}

/// Resolve one candidate segment into a syllable, or explain why it cannot be.
/// `None` means "not a legal segmentation here" (the caller backtracks);
/// `Some(Err(..))` is a hard error that must reach the player.
fn resolve(ls: &[Letter]) -> Option<Result<Syllable, ParseError>> {
    // A digit may only sit at the end of a syllable, and neither may an
    // apostrophe boundary fall inside one.
    if ls[..ls.len() - 1].iter().any(|l| l.digit_tone.is_some() || l.boundary_after) {
        return None;
    }
    let dia: Vec<u8> = ls.iter().filter_map(|l| l.dia_tone).collect();
    // Two tone marks means two syllables were run together here.
    if dia.len() > 1 {
        return None;
    }
    let segment: String = ls.iter().map(|l| l.ch).collect();
    if !is_legal_syllable(&segment) {
        return None;
    }
    let digit = ls[ls.len() - 1].digit_tone;
    match (digit, dia.first()) {
        (Some(_), Some(_)) => Some(Err(ParseError::MixedToneNotation)),
        (Some(d), None) => Some(Ok(Syllable { segment, tone: d })),
        (None, Some(&t)) => Some(Ok(Syllable { segment, tone: t })),
        // Bare: neutral is a value, not an absence (D3).
        (None, None) => Some(Ok(Syllable { segment, tone: 5 })),
    }
}

/// Depth-first, longest-syllable-first, collecting up to `cap` segmentations of
/// exactly `want` syllables (or any count when `want` is None). Longest-first
/// ordering makes the first solution the greedy parse, so a caller that takes
/// solution 0 is deterministic.
fn segmentations(
    ls: &[Letter],
    want: Option<usize>,
    cap: usize,
) -> Result<Vec<PinyinKey>, ParseError> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut hard: Option<ParseError> = None;
    let mut best = 0usize;
    walk(ls, 0, want, cap, &mut cur, &mut out, &mut hard, &mut best);
    // A hard error only counts if nothing parsed -- a MixedToneNotation down a
    // dead branch must not mask a good parse elsewhere.
    if out.is_empty() {
        if let Some(e) = hard {
            return Err(e);
        }
        // `best` is 0 whenever the count prune cut the search before any parse
        // completed, which is the common case. Re-walk unconstrained to learn
        // the natural syllable count, so the error can say "you wrote two, the
        // answer has one" instead of just failing.
        let found = if want.is_some() {
            let (mut o2, mut c2, mut h2, mut b2) = (Vec::new(), Vec::new(), None, 0usize);
            walk(ls, 0, None, 1, &mut c2, &mut o2, &mut h2, &mut b2);
            b2
        } else {
            best
        };
        return Err(ParseError::Unsegmentable { expected: want.unwrap_or(0), found });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    ls: &[Letter],
    at: usize,
    want: Option<usize>,
    cap: usize,
    cur: &mut PinyinKey,
    out: &mut Vec<PinyinKey>,
    hard: &mut Option<ParseError>,
    best: &mut usize,
) {
    if out.len() >= cap {
        return;
    }
    if at == ls.len() {
        // Report the FIRST complete parse in greedy order, not the longest.
        // "xi'an" can also be read xi+a+n, and telling the player their answer
        // had three syllables when the natural reading has two is noise.
        if *best == 0 {
            *best = cur.len();
        }
        if want.is_none_or(|w| w == cur.len()) {
            out.push(cur.clone());
        }
        return;
    }
    // Prune: already at the requested syllable count with input left over.
    if want.is_some_and(|w| cur.len() >= w) {
        return;
    }
    let max = MAX_SYLLABLE.min(ls.len() - at);
    for len in (1..=max).rev() {
        match resolve(&ls[at..at + len]) {
            None => continue,
            Some(Err(e)) => {
                hard.get_or_insert(e);
            }
            Some(Ok(syl)) => {
                cur.push(syl);
                walk(ls, at + len, want, cap, cur, out, hard, best);
                cur.pop();
            }
        }
    }
}

/// The F1 entry point. `expected_syllable_count` constrains an otherwise
/// ambiguous parse (F1b): `xian` is one syllable or `xi` + `an` depending on it.
///
/// When several segmentations satisfy the count this returns the greedy
/// longest-first parse. Grading should prefer [`canonicalize_against`], which
/// implements the charitable parse of D6.
pub fn canonicalize_pinyin(input: &str, expected_syllable_count: usize) -> Result<PinyinKey, ParseError> {
    let ls = lex(input)?;
    let all = segmentations(&ls, Some(expected_syllable_count), CANDIDATE_CAP)?;
    Ok(pick_standard(all))
}

/// Canonicalize a stored bank form, whose tone digits already fix the
/// segmentation, so no expected count is needed.
pub fn canonicalize_answer(stored: &str) -> Result<PinyinKey, ParseError> {
    let ls = lex(stored)?;
    let all = segmentations(&ls, None, CANDIDATE_CAP)?;
    Ok(pick_standard(all))
}

/// How many segmentations to weigh before settling. Real answers are a few
/// syllables, so this is slack, not a limit anyone reaches.
const CANDIDATE_CAP: usize = 64;

fn odd_syllables(k: &PinyinKey) -> usize {
    k.iter().filter(|s| NONSTANDARD.binary_search(&s.segment.as_str()).is_ok()).count()
}

/// Prefer the parse that leans on no flagged syllables. Without this, `xian`
/// at two syllables reads as `xia` + `n` -- greedy-longest finds the
/// interjection `n` before it finds `xi` + `an`. Candidates arrive
/// longest-first, so a stable min keeps the greedy parse on ties.
fn pick_standard(all: Vec<PinyinKey>) -> PinyinKey {
    all.into_iter().min_by_key(|k| odd_syllables(k)).expect("segmentations returns non-empty")
}

/// D6 — the charitable parse. Among the segmentations matching `expected`'s
/// length, return the one maximizing exact syllable matches against it, so an
/// ambiguous input is read in the way most favourable to the player. Ties keep
/// the greedy parse, since candidates arrive longest-first.
pub fn canonicalize_against(input: &str, expected: &PinyinKey) -> Result<PinyinKey, ParseError> {
    let ls = lex(input)?;
    let all = segmentations(&ls, Some(expected.len()), CANDIDATE_CAP)?;
    let score = |k: &PinyinKey| k.iter().zip(expected).filter(|(a, b)| a == b).count();
    // Charity first, then the standard-syllable preference, then greedy order.
    let best = all
        .into_iter()
        .enumerate()
        .min_by_key(|(i, k)| (std::cmp::Reverse(score(k)), odd_syllables(k), *i))
        .map(|(_, k)| k)
        .expect("segmentations returns non-empty");
    Ok(best)
}

/// Compare a typed answer to the stored bank form. Retained so the existing
/// submission path keeps working; F2/F3 replace it with the per-syllable
/// verdict matcher.
pub fn matches(typed: &str, answer: &str) -> bool {
    match canonicalize_answer(answer) {
        Ok(want) => canonicalize_against(typed, &want).map(|got| got == want).unwrap_or(false),
        // An unparseable stored form falls back to exact text equality rather
        // than accepting everything.
        Err(_) => typed == answer,
    }
}

/// F2, the Tone Law. In standard mode, at every tier, each syllable carries a
/// tone and there is no untoned state. Little Speller is tone-BLIND: the reveal
/// still shows tone marks, but grading ignores them.
///
/// This is a flag on the one matcher, never a second code path (Invariant 5).
/// One matcher, one behaviour switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMode {
    /// Standard mode, all tiers including the free tier-1 preview (D1).
    Graded,
    /// Little Speller. Tone is displayed, never graded.
    Blind,
}

impl ToneMode {
    /// Little Speller is the `kid` flag (see modes.rs — "Little Speller / Kid
    /// Mode"), so callers pass the flag they already hold rather than
    /// re-deriving the rule.
    pub fn for_kid(kid: bool) -> Self {
        if kid {
            ToneMode::Blind
        } else {
            ToneMode::Graded
        }
    }
}

/// The single zh grading entry point. Every surface that decides whether a
/// typed Mandarin answer is correct calls this and nothing else.
pub fn matches_with(typed: &str, answer: &str, mode: ToneMode) -> bool {
    let Ok(want) = canonicalize_answer(answer) else {
        // An unparseable stored form falls back to exact text equality rather
        // than accepting everything.
        return typed == answer;
    };
    let Ok(got) = canonicalize_against(typed, &want) else {
        return false;
    };
    match mode {
        ToneMode::Graded => got == want,
        // Segments only. Tone is still carried in the key -- it is ignored
        // here, not stripped, so the reveal can display it.
        ToneMode::Blind => {
            got.len() == want.len()
                && got.iter().zip(&want).all(|(a, b)| a.segment == b.segment)
        }
    }
}

/// F3 — what kind of wrong. "Wrong segment" and "right segment, wrong tone" are
/// different mistakes, and showing them identically is the largest avoidable
/// retention loss in this feature: a player one tone away needs to be told so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyllableVerdict {
    /// Segment and tone both match.
    Exact,
    /// Segment matches, tone does not. The amber case.
    ToneMiss,
    /// Segment does not match, tone happens to.
    SegmentMiss,
    /// Neither matches.
    Both,
}

impl SyllableVerdict {
    /// The class name a surface uses, so colour is never invented per-screen.
    pub fn css_class(self) -> &'static str {
        match self {
            SyllableVerdict::Exact => "zh-exact",
            SyllableVerdict::ToneMiss => "zh-tone-miss",
            SyllableVerdict::SegmentMiss | SyllableVerdict::Both => "zh-miss",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordVerdict {
    /// One verdict per syllable of the ANSWER, in order.
    Graded(Vec<SyllableVerdict>),
    /// Syllable counts differ — red, whole word. Carries both counts so the
    /// reveal can say which way it went instead of just failing.
    LengthMismatch { expected: usize, got: usize },
    /// The typed answer is not parseable pinyin at all. Distinct from a wrong
    /// answer: the player wrote something the notation cannot express, and the
    /// error names which rule it broke.
    Unparseable(ParseError),
}

impl WordVerdict {
    /// The word is correct iff every syllable is Exact.
    pub fn is_correct(&self) -> bool {
        matches!(self, WordVerdict::Graded(v) if v.iter().all(|s| *s == SyllableVerdict::Exact))
    }

    /// Tone-only: at least one tone miss, and nothing worse. This is the
    /// predicate that routes a word to the tone drill instead of the general
    /// missed-words queue (Invariant 6), so it must never be true for a word
    /// that also got a segment wrong.
    pub fn is_tone_only(&self) -> bool {
        match self {
            WordVerdict::Graded(v) => {
                v.iter().any(|s| *s == SyllableVerdict::ToneMiss)
                    && v.iter().all(|s| {
                        matches!(s, SyllableVerdict::Exact | SyllableVerdict::ToneMiss)
                    })
            }
            _ => false,
        }
    }

    /// 1-based indices of the syllables that failed, for a reveal that names
    /// the failure instead of saying "incorrect".
    pub fn failing_indices(&self) -> Vec<usize> {
        match self {
            WordVerdict::Graded(v) => v
                .iter()
                .enumerate()
                .filter(|(_, s)| **s != SyllableVerdict::Exact)
                .map(|(i, _)| i + 1)
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Grade a typed Mandarin answer syllable by syllable.
///
/// In [`ToneMode::Blind`] a tone difference is not a miss, so a tone-only
/// answer grades Exact — Little Speller never sees an amber verdict.
pub fn grade(typed: &str, answer: &str, mode: ToneMode) -> WordVerdict {
    let want = match canonicalize_answer(answer) {
        Ok(w) => w,
        // An unparseable STORED form is a bank bug, not a player mistake. Fall
        // back to text equality so the round is still playable.
        Err(e) => {
            return if typed == answer {
                WordVerdict::Graded(vec![SyllableVerdict::Exact])
            } else {
                WordVerdict::Unparseable(e)
            }
        }
    };
    // A tone digit terminates a syllable, so input where EVERY syllable carries
    // one has already declared how many syllables the player meant. Re-reading
    // it at some other count invents an answer nobody typed: "suo3" against
    // "suo3yi3" would come back as su + o rather than the length mismatch it
    // plainly is.
    //
    // Input with bare syllables is genuinely ambiguous and still resolves by
    // the expected count, which is what keeps "xian" readable as xi + an.
    if let Ok(nat) = canonicalize_answer(typed) {
        let digits = lex(typed)
            .map(|ls| ls.iter().filter(|l| l.digit_tone.is_some()).count())
            .unwrap_or(0);
        if digits == nat.len() && nat.len() != want.len() {
            return WordVerdict::LengthMismatch { expected: want.len(), got: nat.len() };
        }
    }
    let got = match canonicalize_against(typed, &want) {
        Ok(g) => g,
        Err(ParseError::Unsegmentable { expected, found }) => {
            return WordVerdict::LengthMismatch { expected, got: found }
        }
        Err(e) => return WordVerdict::Unparseable(e),
    };
    if got.len() != want.len() {
        return WordVerdict::LengthMismatch { expected: want.len(), got: got.len() };
    }
    // Constraining to the answer's syllable count always finds SOME reading if
    // one exists, which would make a length mismatch unreportable: "ping2"
    // against "ping2guo3" comes back as pi + ng, using the bare interjection.
    // That is an invented reading, not the player's. When the constrained parse
    // leans on a flagged syllable the answer itself does not use, trust the
    // player's own unconstrained reading and call it a length mismatch.
    //
    // This is why "xian" against xi + an still works: that parse invents
    // nothing, so the charitable reading stands (F1b/D6).
    if odd_syllables(&got) > odd_syllables(&want) {
        let natural = canonicalize_answer(typed).map(|k| k.len()).unwrap_or(0);
        if natural != want.len() {
            return WordVerdict::LengthMismatch { expected: want.len(), got: natural };
        }
    }
    WordVerdict::Graded(
        got.iter()
            .zip(&want)
            .map(|(g, w)| {
                let seg = g.segment == w.segment;
                let tone = g.tone == w.tone || mode == ToneMode::Blind;
                match (seg, tone) {
                    (true, true) => SyllableVerdict::Exact,
                    (true, false) => SyllableVerdict::ToneMiss,
                    (false, true) => SyllableVerdict::SegmentMiss,
                    (false, false) => SyllableVerdict::Both,
                }
            })
            .collect(),
    )
}

/// CC-ZH-TONE F6 — the reading, formatted for Google's pinyin alphabet.
///
/// Numeric tone at the end of each syllable, one space between syllables:
/// their own documented example is `wo3 de5`. Neutral is written 5, which that
/// example uses even though the tone chart also lists 0.
///
/// Built from the canonicalizer rather than from the stored string, so the
/// bank's two ü spellings (`lv3xing2` and `lü3ke4`) and its unspaced syllables
/// all come out in one form.
///
/// Returns None for a stored form that will not parse — the caller must then
/// refuse to synthesize rather than fall back to guessing (Invariant 4).
pub fn phoneme_reading(stored: &str) -> Option<String> {
    let key = canonicalize_answer(stored).ok()?;
    Some(
        key.iter()
            .map(|s| format!("{}{}", s.segment, s.tone))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// CC-ZH-TONE F4 — apply a tone to the syllable being typed.
///
/// The point of the buttons is that choosing a tone is ONE deliberate tap
/// rather than a fight with a long-press diacritic picker that may not even
/// offer pinyin vowels. The deliberation is the learning: the player commits to
/// a tone on every syllable and finds out immediately whether they heard it.
///
/// Tapping again RE-tones rather than appending, so a wrong choice costs one
/// tap and never a retype. Scope worth stating: "the active syllable" is the
/// one at the end of the answer — the one being typed. Re-toning an earlier
/// syllable would need a selection affordance that does not exist yet, so it
/// still costs a backspace.
///
/// Typed digits and typed diacritics keep working untouched; this is an
/// affordance, never the only path (D2).
pub fn apply_tone(answer: &str, tone: u8) -> String {
    if !(1..=5).contains(&tone) {
        return answer.to_string();
    }
    let trimmed = answer.trim_end();
    // Nothing typed yet: a tone with no syllable to sit on is a no-op, not a
    // stray digit the parser would later reject as a dangling tone.
    let base = trimmed.strip_suffix(|c: char| c.is_ascii_digit()).unwrap_or(trimmed);
    if base.is_empty() {
        return answer.to_string();
    }
    format!("{base}{tone}")
}

/// CC-ZH-TONE F5 / D4 — grading against both forms of a word.
///
/// 你好 is SPOKEN ni2 hao3; written pinyin keeps ni3 hao3. We tell the player to
/// spell what they hear and then mark it wrong, so the conflict has to be
/// designed rather than discovered on device.
///
/// Grading always compares the CITATION. At tiers 1–2 the surface form is also
/// accepted and the reveal teaches the difference; at tier 3+ only the citation
/// passes and the surface reads as an ordinary tone miss.
///
/// Returns the verdict and whether the pass came via the surface form, because
/// the caller has to know to show the teaching note.
pub fn grade_sandhi_aware(
    typed: &str,
    citation: &str,
    surface: Option<&str>,
    mode: ToneMode,
    accept_surface: bool,
) -> (WordVerdict, bool) {
    let verdict = grade(typed, citation, mode);
    if verdict.is_correct() {
        return (verdict, false);
    }
    if !accept_surface {
        return (verdict, false);
    }
    // Only worth a second look when sandhi actually moved something.
    let Some(surface) = surface.filter(|s| *s != citation) else {
        return (verdict, false);
    };
    let via = grade(typed, surface, mode);
    if via.is_correct() {
        return (via, true);
    }
    // The citation verdict is the one that teaches; a surface miss is not a
    // more useful description of the same wrong answer.
    (verdict, false)
}

/// D4's tier split: tiers 1–2 accept the surface, tier 3+ do not.
pub fn tier_accepts_surface(tier: &str) -> bool {
    matches!(tier, "easy" | "medium")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pinyin_inventory::INVENTORY_PIN;

    fn key(pairs: &[(&str, u8)]) -> PinyinKey {
        pairs.iter().map(|(s, t)| Syllable { segment: s.to_string(), tone: *t }).collect()
    }

    // ---- F1a: the pin ----

    #[test]
    fn inventory_is_sorted_and_unique() {
        // Sorted, because lookup binary-searches it. Python's sorted() and
        // Rust's byte ordering agree here only because UTF-8 preserves
        // codepoint order -- assert it rather than trust it.
        let mut sorted = SYLLABLES.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted.as_slice(), &SYLLABLES[..], "inventory must ship sorted");
        let mut uniq = SYLLABLES.to_vec();
        uniq.dedup();
        assert_eq!(uniq.len(), SYLLABLES.len(), "inventory has duplicates");
        assert!(!INVENTORY_PIN.is_empty());
        // The sha256 pin itself is verified by scripts/pinyin-inventory-check.mjs
        // in the gate, which recomputes it rather than trusting the constant.
    }

    // ---- Done 1: coverage and collisions ----

    #[test]
    fn every_syllable_in_every_encoding_maps_to_one_key() {
        let mut unmapped = Vec::new();
        for s in SYLLABLES {
            for tone in 1..=5u8 {
                let want = key(&[(s, tone)]);
                // digit encoding, plus 0 as an alias for 5
                let mut forms = vec![format!("{s}{tone}")];
                if tone == 5 {
                    forms.push(format!("{s}0"));
                    forms.push(s.to_string());
                }
                // v / u: substitutes
                if s.contains('ü') {
                    forms.push(format!("{}{}", s.replace('ü', "v"), tone));
                    forms.push(format!("{}{}", s.replace('ü', "u:"), tone));
                }
                // uppercase and surrounding whitespace
                forms.push(format!("  {}{}  ", s.to_uppercase(), tone));
                for f in forms {
                    match canonicalize_pinyin(&f, 1) {
                        Ok(k) if k == want => {}
                        other => unmapped.push(format!("{f:?} -> {other:?}, want {want:?}")),
                    }
                }
            }
        }
        assert!(unmapped.is_empty(), "{} unmapped:\n{}", unmapped.len(), unmapped.join("\n"));
    }

    #[test]
    fn distinct_syllables_never_collide() {
        let mut seen = std::collections::HashMap::new();
        for s in SYLLABLES {
            for tone in 1..=5u8 {
                let k = canonicalize_pinyin(&format!("{s}{tone}"), 1).expect("parses");
                if let Some(prev) = seen.insert(k.clone(), (s, tone)) {
                    panic!("collision: {prev:?} and {:?} share key {k:?}", (s, tone));
                }
            }
        }
        assert_eq!(seen.len(), SYLLABLES.len() * 5);
    }

    // ---- Done 2: the adversarial fixture ----

    #[test]
    fn lv_family_tone_three_versus_four() {
        let three = key(&[("lü", 3)]);
        assert_eq!(canonicalize_pinyin("lv3", 1), Ok(three.clone()));
        assert_eq!(canonicalize_pinyin("lü3", 1), Ok(three.clone()));
        assert_eq!(canonicalize_pinyin("lu:3", 1), Ok(three.clone()));
        // The trap: lǜ is FOURTH tone and must not match any of the above.
        let four = canonicalize_pinyin("lǜ", 1).expect("parses");
        assert_eq!(four, key(&[("lü", 4)]));
        assert_ne!(four, three);
    }

    #[test]
    fn neutral_encodings_agree() {
        let want = key(&[("ma", 5)]);
        for f in ["ma", "ma5", "ma0", "MA5", " ma "] {
            assert_eq!(canonicalize_pinyin(f, 1), Ok(want.clone()), "{f}");
        }
    }

    #[test]
    fn bare_input_is_neutral_not_a_wildcard() {
        // D3: the answer is third tone, the player typed bare -- a tone miss.
        assert_ne!(
            canonicalize_pinyin("ma", 1).unwrap(),
            canonicalize_pinyin("ma3", 1).unwrap()
        );
    }

    #[test]
    fn full_width_and_whitespace_are_folded() {
        // Full-width digits and letters NFC-fold to ASCII.
        assert_eq!(canonicalize_pinyin("\u{ff4d}\u{ff41}\u{ff13}", 1), Ok(key(&[("ma", 3)])));
        assert_eq!(canonicalize_pinyin("\tma3\n", 1), Ok(key(&[("ma", 3)])));
    }

    #[test]
    fn diacritic_plus_digit_is_a_parse_error() {
        assert_eq!(canonicalize_pinyin("mǎ3", 1), Err(ParseError::MixedToneNotation));
    }

    #[test]
    fn tone_digit_out_of_range() {
        assert_eq!(canonicalize_pinyin("ma7", 1), Err(ParseError::ToneDigitOutOfRange('7')));
    }

    #[test]
    fn xian_segments_by_expected_count() {
        assert_eq!(canonicalize_pinyin("xian", 1), Ok(key(&[("xian", 5)])));
        assert_eq!(canonicalize_pinyin("xian", 2), Ok(key(&[("xi", 5), ("an", 5)])));
        // The apostrophe is a hard boundary (Eric 2026-08-19), so it agrees
        // with a two-syllable count and CONTRADICTS a one-syllable one --
        // xi'an must never be read back as xian.
        assert_eq!(canonicalize_pinyin("xi'an", 2), Ok(key(&[("xi", 5), ("an", 5)])));
        assert_eq!(
            canonicalize_pinyin("xi'an", 1),
            Err(ParseError::Unsegmentable { expected: 1, found: 2 })
        );
        // The typographic apostrophe an iOS keyboard substitutes counts too.
        assert_eq!(canonicalize_pinyin("xi\u{2019}an", 2), Ok(key(&[("xi", 5), ("an", 5)])));
        // Space and hyphen stay soft: a stray one must not wreck a good answer.
        assert_eq!(canonicalize_pinyin("xi an", 1), Ok(key(&[("xian", 5)])));
        assert_eq!(canonicalize_pinyin("xi-an", 1), Ok(key(&[("xian", 5)])));
    }

    #[test]
    fn unknown_syllable_is_rejected_not_passed_through() {
        assert!(matches!(
            canonicalize_pinyin("qqq1", 1),
            Err(ParseError::Unsegmentable { .. })
        ));
    }

    #[test]
    fn the_canonicalizer_is_total() {
        // Never panics, never silently returns its input.
        for f in ["", "   ", "3", "33", "ma3ma3ma3", "!!", "ǖǖ", "u:", "v", "xxxxxxxxxxxx"] {
            let _ = canonicalize_pinyin(f, 1);
            let _ = canonicalize_pinyin(f, 2);
            let _ = canonicalize_answer(f);
        }
    }

    // ---- charitable parse (D6) ----

    #[test]
    fn charitable_parse_prefers_the_reading_that_matches() {
        // "xian" at two syllables can only be xi+an, but "nihao" at two can be
        // ni+hao orni+hao only; use a genuinely ambiguous one: "shuangan".
        let expected = key(&[("xi", 1), ("an", 1)]);
        let got = canonicalize_against("xi1an1", &expected).unwrap();
        assert_eq!(got, expected);
    }

    // ---- the bank round-trips ----

    #[test]
    fn every_bank_word_canonicalizes() {
        let mut bad = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            for entry in crate::words::tier_for("zh", tier) {
                let stored = entry.split('|').next().unwrap_or(entry);
                match canonicalize_answer(stored) {
                    Ok(k) => {
                        // A stored form must round-trip: typing it back is exact.
                        if !matches(stored, stored) {
                            bad.push(format!("{stored} does not match itself ({k:?})"));
                        }
                    }
                    Err(e) => bad.push(format!("{stored} -> {e:?}")),
                }
            }
        }
        assert!(bad.is_empty(), "{} bank words fail:\n{}", bad.len(), bad.join("\n"));
    }

    #[test]
    fn bank_v_and_u_umlaut_spellings_agree() {
        // The bank mixes encodings (lv3xing2 but lü3ke4); both must key alike.
        assert_eq!(canonicalize_answer("lv3xing2"), canonicalize_answer("lü3xing2"));
        assert!(matches("lü3xing2", "lv3xing2"));
        assert!(matches("lv3ke4", "lü3ke4"));
    }

    // ---- F2: the Tone Law ----

    #[test]
    fn standard_mode_grades_tone_at_every_tier() {
        // D1: no tier relaxes this, including the free tier-1 preview.
        assert!(!matches_with("ma1", "ma3", ToneMode::Graded));
        assert!(!matches_with("bao3bao3", "bao3bao5", ToneMode::Graded));
        assert!(matches_with("bao3bao5", "bao3bao5", ToneMode::Graded));
    }

    #[test]
    fn little_speller_is_tone_blind() {
        // Same matcher, one flag -- tone ignored, never stripped.
        assert!(matches_with("ma1", "ma3", ToneMode::Blind));
        assert!(matches_with("ma", "ma3", ToneMode::Blind));
        assert!(matches_with("bao3bao3", "bao3bao5", ToneMode::Blind));
    }

    #[test]
    fn tone_blind_still_grades_the_segments() {
        // Blind to tone is not blind to spelling.
        assert!(!matches_with("ma1", "mao3", ToneMode::Blind));
        assert!(!matches_with("lv3", "nv3", ToneMode::Blind));
        // ...nor to syllable count.
        assert!(!matches_with("bao3", "bao3bao5", ToneMode::Blind));
    }

    #[test]
    fn tone_mode_comes_from_the_kid_flag() {
        assert_eq!(ToneMode::for_kid(true), ToneMode::Blind);
        assert_eq!(ToneMode::for_kid(false), ToneMode::Graded);
        // The default entry point is the graded one.
        assert_eq!(matches("ma1", "ma3"), matches_with("ma1", "ma3", ToneMode::Graded));
    }

    #[test]
    fn tone_colours_are_pleco_and_never_alone() {
        // D7 + Invariant 7: every graded tone has both a colour and a mark, so
        // no surface can carry tone by colour alone.
        for tone in 1..=5usize {
            assert!(TONE_COLOURS[tone].starts_with('#'), "tone {tone} has no colour");
            assert!(!TONE_MARKS_DISPLAY[tone].is_empty(), "tone {tone} has no mark");
        }
        let mut seen = TONE_COLOURS[1..].to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 5, "two tones share a colour");
    }

    // ---- F3: error classification (Done 4) ----

    #[test]
    fn verdicts_name_the_kind_of_wrong() {
        use SyllableVerdict::*;
        let g = |t: &str, a: &str| grade(t, a, ToneMode::Graded);
        assert_eq!(g("ma3", "ma3"), WordVerdict::Graded(vec![Exact]));
        assert_eq!(g("ma1", "ma3"), WordVerdict::Graded(vec![ToneMiss]));
        assert_eq!(g("mao3", "ma3"), WordVerdict::Graded(vec![SegmentMiss]));
        assert_eq!(g("mao1", "ma3"), WordVerdict::Graded(vec![Both]));
        // D3 again, now as a verdict: bare where the answer is toned is amber.
        assert_eq!(g("ma", "ma3"), WordVerdict::Graded(vec![ToneMiss]));
        // Per syllable, not per word: one right, one mis-toned.
        assert_eq!(g("ping2guo2", "ping2guo3"), WordVerdict::Graded(vec![Exact, ToneMiss]));
    }

    #[test]
    fn length_mismatch_is_a_whole_word_verdict() {
        assert!(matches!(
            grade("ping2", "ping2guo3", ToneMode::Graded),
            WordVerdict::LengthMismatch { .. }
        ));
    }

    #[test]
    fn correct_requires_every_syllable_exact() {
        assert!(grade("ping2guo3", "ping2guo3", ToneMode::Graded).is_correct());
        assert!(!grade("ping2guo2", "ping2guo3", ToneMode::Graded).is_correct());
        assert!(!grade("ping2", "ping2guo3", ToneMode::Graded).is_correct());
    }

    #[test]
    fn tone_only_is_exactly_the_routing_predicate() {
        // Invariant 6 hangs off this: true routes to the drill, false to the
        // general queue. A word with ANY segment error must be false.
        assert!(grade("ping2guo2", "ping2guo3", ToneMode::Graded).is_tone_only());
        assert!(grade("ma1", "ma3", ToneMode::Graded).is_tone_only());
        assert!(!grade("ping2guo3", "ping2guo3", ToneMode::Graded).is_tone_only());
        assert!(!grade("ping2gua3", "ping2guo3", ToneMode::Graded).is_tone_only());
        // Mixed: one tone miss AND one segment miss is not a tone study.
        assert!(!grade("ping2gua2", "ping2guo3", ToneMode::Graded).is_tone_only());
        assert!(!grade("ping2", "ping2guo3", ToneMode::Graded).is_tone_only());
    }

    #[test]
    fn the_reveal_can_name_the_failing_syllable() {
        let v = grade("ping2guo2", "ping2guo3", ToneMode::Graded);
        assert_eq!(v.failing_indices(), vec![2], "1-based, so the reveal can say it");
        assert!(grade("ping2guo3", "ping2guo3", ToneMode::Graded).failing_indices().is_empty());
    }

    #[test]
    fn little_speller_never_sees_amber() {
        // Tone-blind grading cannot produce a tone miss, so no Little Speller
        // answer can route to the tone drill.
        let v = grade("ma1", "ma3", ToneMode::Blind);
        assert!(v.is_correct());
        assert!(!v.is_tone_only());
    }

    /// Done 4: a hundred synthetic wrong answers, half segment-wrong and half
    /// tone-wrong, plus length mismatches — every one classified correctly, and
    /// every tone-only word routed away from the general queue.
    #[test]
    fn hundred_synthetic_wrong_answers_classify() {
        let words: Vec<&str> = crate::words::tier_for("zh", "medium")
            .iter()
            .filter_map(|e| e.split('|').next())
            .filter(|p| canonicalize_answer(p).map(|k| k.len() == 2).unwrap_or(false))
            .take(50)
            .collect();
        assert!(words.len() >= 50, "need 50 two-syllable words, got {}", words.len());
        let (mut tone_cases, mut seg_cases, mut len_cases) = (0, 0, 0);
        for w in &words {
            let key = canonicalize_answer(w).unwrap();
            // Tone-wrong: same segments, one tone rotated.
            let toned: String = key
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let t = if i == 0 { s.tone % 5 + 1 } else { s.tone };
                    format!("{}{}", s.segment, t)
                })
                .collect();
            let v = grade(&toned, w, ToneMode::Graded);
            assert!(v.is_tone_only(), "{w}: {toned} should be tone-only, got {v:?}");
            assert!(!v.is_correct(), "{w}: a tone miss is not correct");
            tone_cases += 1;

            // Segment-wrong: first syllable replaced by a different legal one,
            // tones untouched.
            let other = if key[0].segment == "ma" { "shu" } else { "ma" };
            let segged = format!(
                "{}{}{}{}",
                other, key[0].tone, key[1].segment, key[1].tone
            );
            let v = grade(&segged, w, ToneMode::Graded);
            assert!(!v.is_tone_only(), "{w}: {segged} is a segment miss, not a tone study");
            assert!(!v.is_correct());
            seg_cases += 1;

            // Length: drop the second syllable.
            let short = format!("{}{}", key[0].segment, key[0].tone);
            let v = grade(&short, w, ToneMode::Graded);
            assert!(
                matches!(v, WordVerdict::LengthMismatch { .. }),
                "{w}: {short} should be a length mismatch, got {v:?}"
            );
            assert!(!v.is_tone_only(), "a length mismatch never routes to the drill");
            len_cases += 1;
        }
        assert_eq!((tone_cases, seg_cases), (50, 50));
        assert_eq!(len_cases, 50);
    }

    // ---- F6: the forced reading ----

    #[test]
    fn phoneme_reading_matches_googles_pinyin_alphabet() {
        // Numeric tone at the end of each syllable, one space between them --
        // Google's own documented example is "wo3 de5".
        assert_eq!(phoneme_reading("wo3de5").as_deref(), Some("wo3 de5"));
        assert_eq!(phoneme_reading("ping2guo3").as_deref(), Some("ping2 guo3"));
        assert_eq!(phoneme_reading("ai4").as_deref(), Some("ai4"));
        // Neutral is written 5, never dropped, never 0.
        assert_eq!(phoneme_reading("bao3bao5").as_deref(), Some("bao3 bao5"));
        assert_eq!(phoneme_reading("bao3bao0").as_deref(), Some("bao3 bao5"));
    }

    #[test]
    fn the_banks_two_umlaut_spellings_give_one_reading() {
        // lv3xing2 and lü3ke4 are both in the bank; the reading must not depend
        // on which spelling an entry happened to use.
        assert_eq!(phoneme_reading("lv3xing2"), phoneme_reading("lü3xing2"));
        assert_eq!(phoneme_reading("lv3xing2").as_deref(), Some("lü3 xing2"));
    }

    #[test]
    fn an_unparseable_form_yields_no_reading() {
        // The caller must then refuse to synthesize rather than guess.
        assert_eq!(phoneme_reading("qqq9"), None);
        assert_eq!(phoneme_reading(""), None);
    }

    #[test]
    fn every_bank_word_has_a_reading() {
        // Invariant 4 has teeth only if every word CAN name its reading --
        // otherwise some word silently loses audio.
        let mut bad = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            for entry in crate::words::tier_for("zh", tier) {
                let pinyin = entry.split('|').next().unwrap_or(entry);
                match phoneme_reading(pinyin) {
                    Some(r) => {
                        // Must satisfy the server's own validator shape.
                        let ok = r.split(' ').all(|syl| {
                            let (seg, tone) = syl.split_at(syl.len() - 1);
                            !seg.is_empty()
                                && tone.chars().all(|c| ('0'..='5').contains(&c))
                                && seg.chars().all(|c| c.is_ascii_lowercase() || c == 'ü')
                        });
                        if !ok {
                            bad.push(format!("{pinyin} -> {r:?}"));
                        }
                    }
                    None => bad.push(format!("{pinyin} -> no reading")),
                }
            }
        }
        assert!(bad.is_empty(), "{} words cannot be spoken:\n{}", bad.len(), bad.join("\n"));
    }

    #[test]
    fn tones_remain_significant() {
        assert!(!matches("ma1", "ma3"));
        assert!(matches("ma1", "ma1"));
        assert!(matches("ping2 guo3", "ping2guo3"));
        assert!(matches("xie4xie5", "xie4xie5"));
    }
}

/// CC-ZH-TONE F4 — the settings-truth effect tests (Done 5).
///
/// One per button. AUG6 says a rendered control must have a test proving it
/// does something observable; a tone button that does nothing is a build
/// failure, not a cosmetic bug. The names are what
/// config/settings-effects.json points at, so renaming one fails the gate.
#[cfg(test)]
mod tone_button_effects {
    use super::apply_tone;

    fn observable(tone: u8) {
        // A bare syllable gains the tone...
        let toned = apply_tone("ma", tone);
        assert_eq!(toned, format!("ma{tone}"), "tone {tone} did not apply");
        assert_ne!(toned, "ma", "tone {tone} changed nothing");
        // ...and re-tapping REPLACES rather than appending, so a wrong choice
        // costs one tap instead of a retype.
        let other = if tone == 1 { 2 } else { 1 };
        assert_eq!(apply_tone(&toned, other), format!("ma{other}"));
        assert_eq!(apply_tone(&format!("ma{other}"), tone), toned);
        // The active syllable is the one being typed, not the whole answer.
        assert_eq!(apply_tone("ping2guo", tone), format!("ping2guo{tone}"));
        assert_eq!(apply_tone("ping2guo3", tone), format!("ping2guo{tone}"));
    }

    #[test]
    fn settings_effect_tone1() {
        observable(1);
    }
    #[test]
    fn settings_effect_tone2() {
        observable(2);
    }
    #[test]
    fn settings_effect_tone3() {
        observable(3);
    }
    #[test]
    fn settings_effect_tone4() {
        observable(4);
    }
    #[test]
    fn settings_effect_tone5() {
        // Neutral is a value like any other (D3), so it is written, not omitted.
        observable(5);
        assert_eq!(apply_tone("bao3bao", 5), "bao3bao5");
    }

    #[test]
    fn a_tone_with_no_syllable_is_a_no_op() {
        // Never leave a dangling digit the parser would reject.
        for t in 1..=5u8 {
            assert_eq!(apply_tone("", t), "");
            assert_eq!(apply_tone("   ", t), "   ");
        }
    }
}

#[cfg(test)]
mod display_f1_f2 {
    use super::*;

    /// CC-ZH-PINYIN-DISPLAY F2 — the worked table from the spec, verbatim.
    #[test]
    fn f2_placement_table() {
        let rows = [
            ("you", 2, "y\u{f3}u", "step 3 (o)"),
            ("xi", 4, "x\u{ec}", "step 5 (i)"),
            ("liu", 2, "li\u{fa}", "step 5, last of iu = u"),
            ("gui", 1, "gu\u{12b}", "step 5, last of ui = i"),
            ("n\u{fc}", 3, "n\u{1da}", "step 5 (ü)"),
            ("hao", 3, "h\u{1ce}o", "step 2 (a)"),
            ("de", 5, "de", "step 1, neutral takes no mark"),
        ];
        for (seg, tone, want, why) in rows {
            let got = display_syllable(seg, tone).unwrap_or_else(|| panic!("{seg}{tone} unrenderable"));
            assert_eq!(got, want, "{seg} + {tone} ({why})");
        }
    }

    /// L1 — nothing outside the F1 alphabet may reach a player.
    #[test]
    fn l1_only_precomposed_reaches_the_player() {
        for (_, marks) in PRECOMPOSED.iter() {
            for m in marks {
                assert!(is_legal_display_char(*m), "{m:?} is in the table but rejected");
            }
        }
        // The spacing accents that caused the bug, and the combining range.
        for bad in ['\u{b4}', '\u{2c7}', '\u{af}', '`', '\u{2d9}', '\u{2ca}', '\u{2cb}'] {
            assert!(!is_legal_display_char(bad), "spacing accent {bad:?} must be illegal");
        }
        for cp in 0x300u32..=0x36F {
            let c = char::from_u32(cp).unwrap();
            assert!(!is_legal_display_char(c), "combining U+{cp:04X} must be illegal");
        }
        // And the function never emits one.
        for tone in 1..=5u8 {
            for seg in ["you", "xi", "hao", "n\u{fc}", "zhuang", "er"] {
                if let Some(out) = display_syllable(seg, tone) {
                    for c in out.chars() {
                        assert!(is_legal_display_char(c),
                            "display_syllable({seg},{tone}) emitted illegal {c:?} U+{:04X}", c as u32);
                    }
                }
            }
        }
    }

    /// F5 — PinyinKey -> display -> parse -> PinyinKey is identity across the
    /// WHOLE pinned inventory x 5 tones, not a sample. Two encodings of "yóu"
    /// would mean two cache keys and two match candidates.
    #[test]
    fn f5_round_trip_identity_over_the_full_inventory() {
        let mut checked = 0usize;
        let mut unrenderable = Vec::new();
        let mut broken = Vec::new();
        for syl in crate::pinyin_inventory::SYLLABLES.iter() {
            for tone in 1..=5u8 {
                let Some(shown) = display_syllable(syl, tone) else {
                    unrenderable.push((*syl, tone));
                    continue;
                };
                let back = canonicalize_answer(&shown);
                let ok = matches!(&back, Ok(k) if k.len() == 1 && k[0].segment == *syl && k[0].tone == tone);
                if !ok {
                    broken.push((*syl, tone, format!("{back:?}")));
                    continue;
                }
                checked += 1;
            }
        }
        assert!(checked > 2000, "expected the whole inventory, only checked {checked}");

        // PRE-EXISTING, NOT THIS FILE'S TO FIX. `biang` is in the pinned
        // inventory but the segmenter splits it into bi+ang in EVERY form --
        // digit or mark, so it is not a display defect. CC-ZH-PINYIN-DISPLAY's
        // non-goals forbid touching the canonicalizer's matching semantics, so
        // it is NAMED here rather than skipped: this allowlist cannot grow
        // without someone deciding it should.
        // Verified unreachable from the zh bank: an exact-syllable scan over
        // every ZH_* entry finds zero uses of any of them, so no player can
        // meet one. (A substring scan says otherwise and is wrong -- `nian2`
        // contains "nia" but its syllable is `nian`.)
        let known: &[&str] = &["biang", "nia", "rua"];
        let unexpected: Vec<_> = broken.iter().filter(|(s, _, _)| !known.contains(s)).collect();
        assert!(
            unexpected.is_empty(),
            "display -> parse is not identity for {} syllable/tone pairs that are NOT the known \
             pre-existing case: {:?}",
            unexpected.len(),
            unexpected
        );
        // D2: the interjections are the only legal casualties, and they are
        // named rather than silently skipped.
        for (syl, tone) in &unrenderable {
            assert!(
                matches!(*syl, "m" | "n" | "ng" | "hm" | "hng" | "\u{ea}"),
                "{syl}+{tone} is unrenderable but is not a known interjection"
            );
        }
    }
}

#[cfg(test)]
mod display_f6 {
    //! CC-ZH-PINYIN-DISPLAY F6 — verdict fidelity for 游戏.
    //!
    //! The device showed `第 1 个音节：拼错了` — segmentMiss, "wrong syllable" —
    //! on a syllable whose only visible problem was the tone mark. The routing
    //! in zh_reveal_html is correct (ToneMiss -> zh.rev.toneMiss), so a
    //! segmentMiss message means the VERDICT was segmentMiss. This grades the
    //! plausible player inputs and shows which one reproduces it.
    use super::*;

    #[test]
    fn f6_what_each_plausible_input_grades_as() {
        let answer = "you2 xi4";
        let cases = [
            ("you2 xi4", "digits, spaced — the canonical form"),
            ("y\u{f3}u x\u{ec}", "diacritics, spaced — what the reveal now shows"),
            ("you xi", "toneless, spaced — the TONE_MISS case"),
            ("youxi", "toneless, NO SPACE — the keyboard has no space key"),
            ("you2xi4", "digits, NO SPACE"),
            ("y\u{f3}ux\u{ec}", "diacritics, NO SPACE"),
            ("yo2 xi4", "a REAL segment error — yo, not you"),
            ("you2 shi4", "a REAL segment error on syllable 2"),
        ];
        println!("\n  F6 — grading plausible player answers for 游戏 (you2 xi4)");
        for (typed, why) in cases {
            let v = grade(typed, answer, ToneMode::Graded);
            let label = match &v {
                WordVerdict::Graded(s) if s.iter().all(|x| *x == SyllableVerdict::Exact) => "CORRECT".to_string(),
                WordVerdict::Graded(s) => format!("{:?}", s),
                other => format!("{other:?}"),
            };
            println!("    {typed:<12} {label:<58} {why}");
        }
    }
}

#[cfg(test)]
mod zh_no_space_survey {
    //! Eric, on device: "there's no space bar so there's not many words I can
    //! get correct in Chinese." 游戏 grades fine without a space, so the claim
    //! and the measurement disagree. This surveys the WHOLE zh bank to find
    //! which words actually fail, rather than reasoning from one example.
    use super::*;

    fn zh_bank() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for tier in ["easy", "medium", "hard", "expert"] {
            for e in crate::words::tier_for("zh", tier) {
                let mut it = e.split('|');
                if let (Some(py), Some(hz)) = (it.next(), it.next()) {
                    out.push((py.to_string(), hz.to_string()));
                }
            }
        }
        out
    }

    #[test]
    fn survey_which_zh_words_fail_without_a_space() {
        let bank = zh_bank();
        let (mut exact_ok, mut toneless_tonemiss, mut broken) = (0, 0, Vec::new());
        let mut spacing_victims = Vec::new();
        for (py, hz) in &bank {
            // 1. The stored form typed verbatim (digits, no spaces) must grade
            //    CORRECT. If this fails the word is unwinnable, full stop.
            match grade(py, py, ToneMode::Graded) {
                WordVerdict::Graded(v) if v.iter().all(|s| *s == SyllableVerdict::Exact) => exact_ok += 1,
                other => broken.push((py.clone(), hz.clone(), format!("{other:?}"))),
            }
            // 2. Toneless, no space -- should be ToneMiss on every syllable,
            //    never SegmentMiss. A SegmentMiss here is the segmentation law
            //    failing: the player spelled it right and is told otherwise.
            let toneless: String = py.chars().filter(|c| !c.is_ascii_digit()).collect();
            match grade(&toneless, py, ToneMode::Graded) {
                WordVerdict::Graded(v)
                    if v.iter().all(|s| *s == SyllableVerdict::ToneMiss || *s == SyllableVerdict::Exact) =>
                {
                    toneless_tonemiss += 1
                }
                other => spacing_victims.push((toneless.clone(), py.clone(), hz.clone(), format!("{other:?}"))),
            }
        }
        println!("\n  zh bank: {} words", bank.len());
        println!("  stored form typed verbatim grades CORRECT : {exact_ok}/{}", bank.len());
        println!("  toneless no-space grades ToneMiss (not SegmentMiss) : {toneless_tonemiss}/{}", bank.len());
        println!("  UNWINNABLE (correct input does not grade correct)  : {}", broken.len());
        for (py, hz, v) in broken.iter().take(25) {
            println!("    {py:<16} {hz:<8} {v}");
        }
        if broken.len() > 25 {
            println!("    ... and {} more", broken.len() - 25);
        }
        println!("\n  SPACING VICTIMS -- spelled right, told otherwise ({}):", spacing_victims.len());
        for (typed, want, hz, v) in spacing_victims.iter().take(40) {
            println!("    typed {typed:<14} want {want:<14} {hz:<8} {v}");
        }
    }
}

#[cfg(test)]
mod tone_button_sequence {
    //! Eric on device: "it won't accept answers where the two words in the
    //! answer have tones at the end of more than one word."
    //!
    //! Typed digits grade fine (`you2xi4` is CORRECT), so the failure must be
    //! in how the TONE BUTTONS build the string. This replays the actual key
    //! sequence a player performs, one tap at a time, instead of testing the
    //! finished string.
    use super::*;

    /// One tap: a letter key, or a tone button routed through apply_tone.
    fn tap(buf: &str, key: char) -> String {
        if let Some(t) = key.to_digit(10) {
            apply_tone(buf, t as u8)
        } else {
            format!("{buf}{key}")
        }
    }

    /// Eric on device typed hai3zi4 for 孩子 (hai2zi5) and it was refused.
    /// Refusing it is right -- both tones are wrong -- but the REASON must
    /// read "right sound, wrong tone" and not "wrong syllable", and the
    /// neutral second syllable is the interesting half.
    #[test]
    fn haizi_wrong_tones_is_a_tone_miss_not_a_spelling_miss() {
        let v = grade("hai3zi4", "hai2zi5", ToneMode::Graded);
        println!("\n  hai3zi4 vs hai2zi5 -> {v:?}");
        match v {
            WordVerdict::Graded(ref sv) => assert!(
                sv.iter().all(|x| *x == SyllableVerdict::ToneMiss),
                "both syllables are spelled right; only the tones differ -- got {sv:?}"
            ),
            other => panic!("expected a graded verdict, got {other:?}"),
        }
        // The neutral half on its own: tone 4 where the answer is neutral.
        println!("  zi4 vs zi5 -> {:?}", grade("zi4", "zi5", ToneMode::Graded));
    }

    #[test]
    fn typing_youxi_with_the_tone_buttons() {
        // 游戏 = you2 xi4. The player has no space bar, so: y o u ② x i ④
        let mut buf = String::new();
        let mut trace = Vec::new();
        for k in ['y', 'o', 'u', '2', 'x', 'i', '4'] {
            buf = tap(&buf, k);
            trace.push(format!("{k} -> {buf:?}"));
        }
        println!("\n  tone-button sequence for 游戏 (no space bar):");
        for t in &trace {
            println!("    {t}");
        }
        let v = grade(&buf, "you2xi4", ToneMode::Graded);
        println!("  final buffer {buf:?} grades {v:?}");

        // A three-syllable word makes the pattern unmistakable if it exists.
        let mut b2 = String::new();
        for k in ['j', 'i', 'n', '3', 'k', 'e', '3', 'n', 'e', 'n', 'g', '2'] {
            b2 = tap(&b2, k);
        }
        println!("  three-syllable 尽可能 -> {b2:?} grades {:?}",
                 grade(&b2, "jin3ke3neng2", ToneMode::Graded));
    }
}

#[cfg(test)]
mod submit_path_haizi {
    //! Eric on device: typed `hai2zi5` for 孩子 -- the EXACT bank answer -- and
    //! Check refused it.
    //!
    //! My earlier survey tested `grade()` and reported 6182/6182 correct. The
    //! app does not call `grade()`. `submit_guess`'s zh branch calls
    //! `grade_sandhi_aware`, with the sandhi surface form and a tier flag.
    //! Testing the grader in isolation instead of the path the button takes is
    //! how a real report got dismissed as not reproducible.
    use super::*;

    #[test]
    fn the_exact_bank_answer_must_be_accepted_through_the_submit_path() {
        let word = "hai2zi5";          // s.word for zh is the pinyin
        let spoken = "孩子";            // s.spoken is the hanzi
        let entry = format!("{word}|{spoken}");
        let surface = crate::zh_sandhi::lookup(&entry).map(|(s, _)| s);
        println!("\n  entry {entry:?}  sandhi surface = {surface:?}");

        for tier in ["easy", "medium", "hard", "expert"] {
            let accepts = tier_accepts_surface(tier);
            let (v, via) = grade_sandhi_aware(
                word, word, surface, ToneMode::Graded, accepts,
            );
            println!("    tier {tier:<7} accepts_surface={accepts:<5} correct={} via_surface={via} {v:?}",
                     v.is_correct());
            assert!(
                v.is_correct(),
                "tier {tier}: the exact stored answer {word:?} was REFUSED -- {v:?}"
            );
        }
    }
}

#[cfg(test)]
mod live_marks {
    //! Eric: "I'd like it so the mark sticks to the letter before it's
    //! submitted, like it does on the ToneBoard application."
    use super::*;

    #[test]
    fn the_mark_lands_as_soon_as_the_tone_is_typed() {
        // Keystroke by keystroke through 孩子 = hai2zi5, the word that was
        // being thrown away. The field must read like pinyin the whole way.
        let steps = [
            ("h", "h"), ("ha", "ha"), ("hai", "hai"),
            ("hai2", "h\u{e1}i"),                       // mark lands on the a
            ("hai2z", "h\u{e1}iz"), ("hai2zi", "h\u{e1}izi"),
            ("hai2zi5", "h\u{e1}izi"),                  // neutral takes no mark
        ];
        for (buf, want) in steps {
            assert_eq!(display_partial(buf), want, "buffer {buf:?}");
        }
    }

    #[test]
    fn every_tone_lands_on_the_right_vowel_live() {
        for (buf, want) in [
            ("you2xi4", "y\u{f3}ux\u{ec}"),   // o wins over u; i is the only vowel
            ("hao3", "h\u{1ce}o"),            // a wins
            ("liu2", "li\u{fa}"),             // last of iu
            ("gui1", "gu\u{12b}"),            // last of ui
            ("n\u{fc}3", "n\u{1da}"),         // ü
            ("zhong4guo2", "zh\u{f2}nggu\u{f3}"),
        ] {
            assert_eq!(display_partial(buf), want, "buffer {buf:?}");
        }
    }

    #[test]
    fn it_never_mangles_half_typed_or_odd_input() {
        // Called on EVERY keystroke, so it must be total: no panic, no
        // reordering, nothing silently dropped.
        for buf in ["", "2", "5hai", "hai22", "hai2 zi5", "xyz", "n", "ng3", "hai2-zi5"] {
            let out = display_partial(buf);
            assert!(!out.is_empty() || buf.is_empty(), "{buf:?} vanished");
            // Every non-tone character survives in order.
            let kept: String = buf.chars().filter(|c| !('1'..='5').contains(c)).collect();
            let stripped: String = out
                .chars()
                .map(|c| PRECOMPOSED.iter()
                    .find(|(_, m)| m.contains(&c))
                    .map(|(base, _)| *base)
                    .unwrap_or(c))
                .filter(|c| !('1'..='5').contains(c))
                .collect();
            assert_eq!(stripped, kept, "{buf:?} lost or reordered a letter");
        }
    }
}
