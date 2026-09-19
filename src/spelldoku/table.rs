//! Number-word tables (D3). Loaded read-only; never composed, generated or
//! derived. A row without a citation is invalid and never loads.

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Sourced,
    Audited,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Row {
    pub n: u32,
    pub spellings: Vec<String>,
    pub status: Status,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub locator: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Table {
    pub lang: String,
    pub rows: Vec<Row>,
}

/// Which release rule applies (D3). The ship lane builds without
/// `audit_preview`, so a TestFlight binary is a production binary; only dev
/// builds are previews.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Build {
    Preview,
    Production,
}

pub fn current_build() -> Build {
    if cfg!(feature = "audit_preview") {
        Build::Preview
    } else {
        Build::Production
    }
}

/// I7: NFC, and case-folded where the script has case.
pub fn norm(s: &str) -> String {
    s.trim().nfc().collect::<String>().to_lowercase()
}

fn parse(raw: &str) -> Option<Table> {
    let mut t: Table = serde_json::from_str(raw).ok()?;
    // D3: an uncited row is invalid in every build.
    t.rows.retain(|r| !r.source.trim().is_empty() && !r.locator.trim().is_empty() && !r.spellings.is_empty());
    for r in &mut t.rows {
        r.spellings = r.spellings.iter().map(|s| norm(s)).collect();
    }
    Some(t)
}

/// The table for a language, if one exists. English cites Merriam-Webster; the
/// others were sourced from Wiktionary by tools/numbers/source_numbers.py.
pub fn load(lang: &str) -> Option<Table> {
    let raw = match lang {
        "en" => include_str!("../../config/numbers/en.json"),
        "es" => include_str!("../../config/numbers/es.json"),
        "fr" => include_str!("../../config/numbers/fr.json"),
        "de" => include_str!("../../config/numbers/de.json"),
        "pt" => include_str!("../../config/numbers/pt.json"),
        "pl" => include_str!("../../config/numbers/pl.json"),
        "ru" => include_str!("../../config/numbers/ru.json"),
        "vi" => include_str!("../../config/numbers/vi.json"),
        "ko" => include_str!("../../config/numbers/ko.json"),
        "ja" => include_str!("../../config/numbers/ja.json"),
        "zh" => include_str!("../../config/numbers/zh.json"),
        "fil" => include_str!("../../config/numbers/fil.json"),
        "sw" => include_str!("../../config/numbers/sw.json"),
        "ar" => include_str!("../../config/numbers/ar.json"),
        "hi" => include_str!("../../config/numbers/hi.json"),
        _ => return None,
    };
    parse(raw)
}

/// D3's release rule for one row, as Eric set it on 2026-09-18: every language
/// is audited by being played, so every language plays its 0-12 rows once they
/// are sourced -- English included. 13-45 (Phase B) waits for the audit in a
/// production build. A TestFlight binary is a production binary here.
pub fn row_servable(_lang: &str, row: &Row, build: Build) -> bool {
    match build {
        Build::Preview => true,
        Build::Production => row.n <= 12 || row.status == Status::Audited,
    }
}

/// The Vietnamese tone marks, entered from their own key row. Placement on the
/// syllable is the keyboard's job, not the speller's: a word matches when its
/// letters match and it carries the same tones (D8's spirit -- the tone is
/// required, not where the finger put it).
const VI_TONES: [char; 5] = ['\u{0300}', '\u{0301}', '\u{0309}', '\u{0303}', '\u{0323}'];

fn vi_split(s: &str) -> (String, Vec<char>) {
    let mut base = String::new();
    let mut tones = Vec::new();
    for c in s.nfd() {
        if VI_TONES.contains(&c) {
            tones.push(c);
        } else {
            base.push(c);
        }
    }
    tones.sort_unstable();
    (base.nfc().collect(), tones)
}

/// Does what was typed spell this accepted spelling, in this language?
pub fn spelling_matches(lang: &str, typed: &str, spelling: &str) -> bool {
    match lang {
        // D8: toned pinyin; the keyboard types tone numbers, and the game's own
        // pinyin matcher is the one judge of that. Untoned never matches.
        "zh" => crate::pinyin::matches(&norm(typed), spelling),
        "vi" => vi_split(&norm(typed)) == vi_split(spelling),
        _ => norm(typed) == spelling,
    }
}

impl Table {
    pub fn row(&self, n: u32) -> Option<&Row> {
        self.rows.iter().find(|r| r.n == n)
    }

    /// Every value a board of side `size_n` needs is servable in this build.
    pub fn servable_for(&self, size_n: usize, build: Build) -> bool {
        (1..=size_n as u32).all(|n| self.row(n).is_some_and(|r| row_servable(&self.lang, r, build)))
    }

    /// The values whose accepted spellings (D4) match what was typed.
    pub fn values_for(&self, typed: &str) -> Vec<u8> {
        self.rows
            .iter()
            .filter(|r| r.n > 0 && r.n < 16 && r.spellings.iter().any(|s| spelling_matches(&self.lang, typed, s)))
            .map(|r| r.n as u8)
            .collect()
    }

    /// The display spelling of a value: its first accepted spelling.
    pub fn word(&self, v: u8) -> Option<&str> {
        self.row(v as u32).and_then(|r| r.spellings.first()).map(|s| s.as_str())
    }

    /// F2: the values in `1..=max` with an accepted spelling matching the
    /// pattern grapheme for grapheme (None = hidden). The mask is the
    /// constraint. Graphemes, not code points, so a Devanagari vowel sign or a
    /// Vietnamese tone never becomes a blank of its own.
    pub fn fragment_set(&self, pattern: &[Option<String>], max: usize) -> u16 {
        let mut m = 0u16;
        for v in 1..=max as u32 {
            let Some(row) = self.row(v) else { continue };
            let fits = row.spellings.iter().any(|s| {
                let letters: Vec<&str> = s.graphemes(true).collect();
                letters.len() == pattern.len()
                    && letters.iter().zip(pattern).all(|(c, p)| p.as_deref().map_or(true, |p| p == *c))
            });
            if fits {
                m |= 1u16 << v;
            }
        }
        m
    }
}
