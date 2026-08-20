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

/// The tone's written mark, so a surface can satisfy Invariant 7 without
/// inventing its own label.
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

    #[test]
    fn tones_remain_significant() {
        assert!(!matches("ma1", "ma3"));
        assert!(matches("ma1", "ma1"));
        assert!(matches("ping2 guo3", "ping2guo3"));
        assert!(matches("xie4xie5", "xie4xie5"));
    }
}
