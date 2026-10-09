//! CC-BOARD-GAME v1.1 D20 -- the answer keys, as data.
//!
//! The grid is built on `keyboard::unit_rows(lang)` and `hangul::feed`, Bee
//! style, and the mode never touches the shared game keyboard. One gap in that
//! recipe is closed here, because it makes words unanswerable otherwise: the
//! shared keyboard reaches accents, voiced kana, tense jamo, hamza forms and
//! aspirates by LONG-PRESS, and `unit_rows` carries none of them. On `unit_rows`
//! alone 3 to 25 percent of a bank's words cannot be typed (fr cafe, ja
//! dakuten, ar hamza, hi aspirates). So the keys the bank actually needs and the
//! rows lack are added as extra rows, computed from the bank itself, and the
//! one contract that matters is a test: every bank word in every language can be
//! typed from these keys and passes `boardgame_grade::grade`.
//!
//! Nothing here decides correctness; it only builds the keys and applies a
//! press to the typed string. No DOM, so it is testable on the host.

use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

use crate::consts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// Append this unit (for Korean, feed it to the Hangul automaton).
    Unit(String),
    /// Vietnamese tone mark applied to the last vowel (`viet::retone`).
    Tone(char),
}

impl Key {
    /// What the keycap shows. A tone mark rides on a dotted circle.
    pub fn face(&self) -> String {
        match self {
            Key::Unit(u) if u == " " => "\u{2423}".to_string(),
            Key::Unit(u) => u.clone(),
            Key::Tone(m) => format!("\u{25cc}{m}"),
        }
    }

    /// The `data-k` attribute value.
    pub fn data(&self) -> String {
        match self {
            Key::Unit(u) => format!("u:{u}"),
            Key::Tone(m) => format!("t:{:x}", *m as u32),
        }
    }

    pub fn parse(d: &str) -> Option<Key> {
        if let Some(u) = d.strip_prefix("u:") {
            return Some(Key::Unit(u.to_string()));
        }
        let hex = d.strip_prefix("t:")?;
        char::from_u32(u32::from_str_radix(hex, 16).ok()?).map(Key::Tone)
    }
}

/// The per-language facts of the input path, in one table (single source).
struct Spec {
    /// A tone row, as the Vietnamese keyboard has.
    tone_row: bool,
    /// Korean: syllables are composed from jamo by `hangul::feed`.
    hangul: bool,
    /// A letter the grader accepts typed as another (pinyin: lu-umlaut as v).
    alias: &'static [(char, char)],
}

fn spec(lang: &str) -> Spec {
    match lang {
        l if l == consts::VI => Spec { tone_row: true, hangul: false, alias: &[] },
        l if l == consts::KO => Spec { tone_row: false, hangul: true, alias: &[] },
        l if l == consts::ZH => Spec { tone_row: false, hangul: false, alias: &[('\u{fc}', 'v')] },
        _ => Spec { tone_row: false, hangul: false, alias: &[] },
    }
}

fn row_chars(lang: &str) -> BTreeSet<char> {
    crate::keyboard::unit_rows(lang).iter().flat_map(|r| r.chars()).collect()
}

/// Whether Korean jamo `j` (not on a row) is made by pressing two row keys
/// (a compound vowel, or a compound final after a vowel).
fn composable(j: char, rows: &BTreeSet<char>) -> bool {
    for &a in rows {
        for &b in rows {
            // A compound VOWEL composes anywhere. A doubled consonant does not:
            // after a syllable the first press becomes its final, so tense
            // consonants always need their own key.
            if crate::hangul::is_vowel(j) && crate::hangul::feed(&crate::hangul::feed("", a), b) == j.to_string() {
                return true;
            }
            let s = crate::hangul::feed(&crate::hangul::feed(&crate::hangul::feed("", '\u{3147}'), '\u{314f}'), a);
            let s = crate::hangul::feed(&s, b);
            if s.chars().count() == 1 && s.chars().next().and_then(crate::hangul::parts).map(|p| p.2) == Some(j) {
                return true;
            }
        }
    }
    false
}

/// The units a word needs typed, in the form the grader will compare: Korean
/// as jamo; Spell Jr after its accent-lenient fold (the same fold the grader
/// applies); others NFC-lowercase. Vietnamese tones are stripped (the tone row
/// supplies them).
fn needed(lang: &str, kid: bool, entry: &str) -> Vec<char> {
    let cite = crate::boardgame_grade::citation(entry);
    let sp = spec(lang);
    let mut out: Vec<char> = Vec::new();
    if sp.hangul {
        for c in cite.nfc() {
            match crate::hangul::parts(c) {
                Some((i, m, f)) => {
                    out.push(i);
                    out.push(m);
                    if f != '\0' {
                        out.push(f);
                    }
                }
                None => out.push(c),
            }
        }
        return out;
    }
    let folded: String = if kid && lang != consts::ZH { crate::norm::fold_lenient(cite) } else { cite.nfc().collect::<String>().to_lowercase() };
    for c in folded.chars().filter(|c| !c.is_whitespace()) {
        let c = sp.alias.iter().find(|(from, _)| *from == c).map(|(_, to)| *to).unwrap_or(c);
        if sp.tone_row {
            let bare: String = c.nfd().filter(|m| !crate::viet::TONE_MARKS.contains(m)).collect::<String>().nfc().collect();
            out.extend(bare.chars());
        } else {
            out.push(c);
        }
    }
    out
}

/// The key rows for a game: the shared layout's own rows, then a row (or rows)
/// of the units this bank needs that those rows lack, then the tone row.
pub fn layout(lang: &str, kid: bool, entries: &[String]) -> Vec<Vec<Key>> {
    let rows = row_chars(lang);
    let sp = spec(lang);
    let mut extra: BTreeSet<char> = BTreeSet::new();
    for e in entries {
        for c in needed(lang, kid, e) {
            if !rows.contains(&c) {
                extra.insert(c);
            }
        }
    }
    if sp.hangul {
        extra.retain(|&j| !composable(j, &rows));
    }
    let mut out: Vec<Vec<Key>> = crate::keyboard::unit_rows(lang).iter().map(|r| r.chars().map(|c| Key::Unit(c.to_string())).collect()).collect();
    let width = crate::keyboard::unit_rows(lang).iter().map(|r| r.chars().count()).max().unwrap_or(10).max(8);
    let extra: Vec<char> = extra.into_iter().collect();
    for chunk in extra.chunks(width) {
        out.push(chunk.iter().map(|c| Key::Unit(c.to_string())).collect());
    }
    if sp.tone_row {
        out.push(crate::viet::TONE_MARKS.iter().map(|&m| Key::Tone(m)).collect());
    }
    out
}

fn find(rows: &[Vec<Key>], k: &Key) -> bool {
    rows.iter().any(|r| r.contains(k))
}

/// Type `entry` the way a player would, using only `rows`; None when a
/// needed key is missing.
pub fn type_word(lang: &str, kid: bool, rows: &[Vec<Key>], entry: &str) -> Option<String> {
    let sp = spec(lang);
    let cite = crate::boardgame_grade::citation(entry);
    let mut typed = String::new();
    let unit = |typed: &mut String, c: char| -> bool {
        let k = Key::Unit(c.to_string());
        if find(rows, &k) {
            *typed = press(lang, typed, &k);
            true
        } else {
            false
        }
    };
    if sp.hangul {
        for c in cite.nfc() {
            match crate::hangul::parts(c) {
                Some((i, m, f)) => {
                    for j in [i, m, f].into_iter().filter(|j| *j != '\0') {
                        if !unit(&mut typed, j) {
                            // A compound: two presses.
                            let rows_c = row_chars(lang);
                            let pair = rows_c.iter().flat_map(|&a| rows_c.iter().map(move |&b| (a, b))).find(|&(a, b)| {
                                let v = crate::hangul::feed(&crate::hangul::feed("", a), b);
                                let s = crate::hangul::feed(&crate::hangul::feed(&crate::hangul::feed("", '\u{3147}'), '\u{314f}'), a);
                                let s = crate::hangul::feed(&s, b);
                                (crate::hangul::is_vowel(j) && v == j.to_string()) || s.chars().next().and_then(crate::hangul::parts).map(|p| p.2) == Some(j)
                            })?;
                            if !unit(&mut typed, pair.0) || !unit(&mut typed, pair.1) {
                                return None;
                            }
                        }
                    }
                }
                None => {
                    if !unit(&mut typed, c) {
                        return None;
                    }
                }
            }
        }
        return Some(typed);
    }
    let folded: String = if kid && lang != consts::ZH { crate::norm::fold_lenient(cite) } else { cite.nfc().collect::<String>().to_lowercase() };
    for c in folded.chars().filter(|c| !c.is_whitespace()) {
        let c = sp.alias.iter().find(|(from, _)| *from == c).map(|(_, to)| *to).unwrap_or(c);
        if sp.tone_row {
            let nfd: Vec<char> = c.nfd().collect();
            let tone = nfd.iter().copied().find(|m| crate::viet::TONE_MARKS.contains(m));
            let bare: String = nfd.iter().filter(|m| !crate::viet::TONE_MARKS.contains(m)).collect::<String>().nfc().collect();
            for b in bare.chars() {
                if !unit(&mut typed, b) {
                    return None;
                }
            }
            if let Some(t) = tone {
                let k = Key::Tone(t);
                if !find(rows, &k) {
                    return None;
                }
                typed = press(lang, &typed, &k);
            }
        } else if !unit(&mut typed, c) {
            return None;
        }
    }
    Some(typed)
}


/// Whether a word can be answered from `rows` and the grader accepts the
/// result. For every language but Korean this is true by construction of
/// `layout` (and a test proves it); Korean is the one place it can fail, so the
/// pool builder asks (see `needs_check`).
pub fn answerable(lang: &str, kid: bool, rows: &[Vec<Key>], entry: &str, tier: crate::boardgame::Tier) -> bool {
    type_word(lang, kid, rows, entry).is_some_and(|typed| crate::boardgame_grade::grade(lang, kid, &typed, entry, tier))
}

/// True for the languages whose composition automaton can lose a keystroke.
/// `hangul::feed` drops a tense initial (ㄸ ㅃ ㅉ) typed straight after an open
/// syllable (it tries to make it a final, finds none, and writes nothing), so
/// about 10 Korean words cannot be typed in ANY mode that uses it. Not fixed
/// here (the shared automaton is out of scope); those words are left out of this
/// mode's pools instead of being served unanswerable.
pub fn needs_check(lang: &str) -> bool {
    spec(lang).hangul
}

/// Apply one press to the typed string.
pub fn press(lang: &str, typed: &str, key: &Key) -> String {
    match key {
        Key::Unit(u) => {
            if spec(lang).hangul {
                match u.chars().next() {
                    Some(j) => crate::hangul::feed(typed, j),
                    None => typed.to_string(),
                }
            } else {
                format!("{typed}{u}")
            }
        }
        Key::Tone(m) => {
            let mut s = typed.to_string();
            if let Some(last) = s.chars().last() {
                if let Some(r) = crate::viet::retone(last, *m) {
                    s.pop();
                    s.push_str(&r);
                }
            }
            s
        }
    }
}

pub fn backspace(lang: &str, typed: &str) -> String {
    if spec(lang).hangul {
        crate::hangul::backspace(typed)
    } else {
        let mut s = typed.to_string();
        s.pop();
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boardgame::Tier;
    use crate::boardgame_pools::{bank, tiers_of, variant_for};

    fn find(rows: &[Vec<Key>], k: &Key) -> bool {
        rows.iter().any(|r| r.contains(k))
    }

    /// The contract: with the keys this layout offers, every word of every
    /// tier in play, in every language, for both audiences, can be typed and
    /// the grader accepts it.
    #[test]
    fn every_bank_word_can_be_answered_from_the_keys() {
        for lang in crate::consts::BUILTIN_LANGS.iter().map(|l| l.0) {
            for kid in [false, true] {
                let v = variant_for(kid);
                let mut all: Vec<(Tier, String)> = Vec::new();
                for &t in tiers_of(v) {
                    for w in bank(lang, t, kid) {
                        all.push((t, w));
                    }
                }
                let entries: Vec<String> = all.iter().map(|x| x.1.clone()).collect();
                let rows = layout(lang, kid, &entries);
                let mut bad: Vec<String> = Vec::new();
                for (t, w) in &all {
                    match type_word(lang, kid, &rows, w) {
                        None => bad.push(format!("{w}: a key is missing")),
                        Some(typed) => {
                            if !crate::boardgame_grade::grade(lang, kid, &typed, w, *t) {
                                bad.push(format!("{w}: typed {typed:?} is not accepted"));
                            }
                        }
                    }
                }
                eprintln!("KEYS {lang} kid={kid}: {} rows, {:?}, {} words, {} bad", rows.len(), rows.iter().map(Vec::len).collect::<Vec<_>>(), all.len(), bad.len());
                assert!(bad.is_empty(), "{lang} kid={kid}: {} words cannot be answered, e.g. {:?}", bad.len(), &bad[..bad.len().min(5)]);
            }
        }
    }

    #[test]
    fn presses_compose_and_erase() {
        assert_eq!(press("en", "ca", &Key::Unit("t".into())), "cat");
        assert_eq!(backspace("en", "cat"), "ca");
        let mut s = String::new();
        for j in ['ㅎ', 'ㅏ', 'ㄴ'] {
            s = press("ko", &s, &Key::Unit(j.to_string()));
        }
        assert_eq!(s, "한");
        assert_eq!(backspace("ko", &s), "하");
        let s = press("vi", "ma", &Key::Tone('\u{301}'));
        assert_eq!(s, "má");
    }

    #[test]
    fn key_data_round_trips() {
        for k in [Key::Unit("a".into()), Key::Unit("ㅎ".into()), Key::Unit(" ".into()), Key::Tone('\u{323}')] {
            assert_eq!(Key::parse(&k.data()), Some(k.clone()));
        }
        assert_eq!(Key::parse("x"), None);
    }

    #[test]
    fn english_needs_no_extra_row() {
        let rows = layout("en", false, &bank("en", Tier::Hard, false));
        assert_eq!(rows.len(), crate::keyboard::unit_rows("en").len());
    }
}
