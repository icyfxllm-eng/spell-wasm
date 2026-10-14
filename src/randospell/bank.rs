//! The length-indexed word supply and its coverage (D3, D4, D12).

use crate::wordsearch::lexicon::graphemes;

/// D4: a length with fewer words than this is not a die face.
pub const CELL_FLOOR: usize = 20;
/// Census item 3: fewer covered lengths than this and Randospell is not viable.
pub const MIN_COVERED: usize = 6;
/// D1 and the 2026-10-14 census ruling: off until a bank grows. Hard list, so
/// a remote flag can never turn these on by accident.
pub const INELIGIBLE: [&str; 4] = ["ko", "hi", "ja", "zh"];
/// The mode id the experience resolver knows this mode by (I5 of CC-ONBOARD-JR).
pub const MODE_ID: &str = "randospell";
/// Faces 2..=12.
pub const FACES: std::ops::RangeInclusive<u8> = 2..=12;

/// D12: grapheme clusters, spaces excluded (the space stays part of the
/// answer). Derived, since the bank stores no length.
pub fn length_of(word: &str) -> usize {
    graphemes(word).iter().filter(|g| !g.chars().all(char::is_whitespace)).count()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub word: String,
    /// 0..=3, easy..expert.
    pub tier: usize,
}

/// Which lengths are real faces, and where the others go (D4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    /// Words per length 2..=12 (index 0 is length 2).
    pub counts: [usize; 11],
}

impl Coverage {
    pub fn covered(&self, len: usize) -> bool {
        (2..=12).contains(&len) && self.counts[len - 2] >= CELL_FLOOR
    }

    pub fn covered_lengths(&self) -> Vec<usize> {
        (2..=12).filter(|l| self.covered(*l)).collect()
    }

    /// D4: the length a face plays as: itself when covered, else the nearest
    /// covered length, a tie going to the longer. None when nothing is covered.
    pub fn length_for(&self, face: u8) -> Option<usize> {
        let f = face as usize;
        if self.covered(f) {
            return Some(f);
        }
        self.covered_lengths().into_iter().min_by_key(|l| (l.abs_diff(f), std::cmp::Reverse(*l)))
    }

    /// The info-sheet lines (D4): every face that plays as another length.
    pub fn remaps(&self) -> Vec<(u8, usize)> {
        FACES.filter_map(|f| self.length_for(f).filter(|l| *l != f as usize).map(|l| (f, l))).collect()
    }

    /// Census item 3 plus the language hard-off list.
    pub fn viable(&self) -> bool {
        self.covered_lengths().len() >= MIN_COVERED
    }
}

#[derive(Clone, Debug)]
pub struct Bank {
    by_len: Vec<Vec<Entry>>, // index 0 is length 2
    pub cov: Coverage,
}

impl Bank {
    /// Build from (tier, answer) rows. Duplicates collapse to the lowest tier.
    pub fn from_rows<I: IntoIterator<Item = (usize, String)>>(rows: I) -> Bank {
        let mut by_len: Vec<Vec<Entry>> = vec![Vec::new(); 11];
        let mut seen = std::collections::BTreeSet::new();
        let mut rows: Vec<(usize, String)> = rows.into_iter().collect();
        rows.sort();
        for (tier, word) in rows {
            let n = length_of(&word);
            if (2..=12).contains(&n) && seen.insert(word.clone()) {
                by_len[n - 2].push(Entry { word, tier });
            }
        }
        let mut counts = [0usize; 11];
        for (i, v) in by_len.iter().enumerate() {
            counts[i] = v.len();
        }
        Bank { by_len, cov: Coverage { counts } }
    }

    /// Every answer in the bank (the keyboard is built from these).
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.by_len.iter().flatten().map(|e| e.word.as_str())
    }

    pub fn of_len(&self, len: usize) -> &[Entry] {
        if (2..=12).contains(&len) {
            &self.by_len[len - 2]
        } else {
            &[]
        }
    }

    /// The device bank for a language: the tier files `exp` may be served
    /// (experience::allowed_tiers, so a Jr profile never pools Hard or Expert),
    /// with the profanity, audio and proper-noun gates the other modes apply
    /// (D11). Tier indexes are positions on the full ladder.
    pub fn for_lang(lang: &str, exp: crate::experience::Experience) -> Bank {
        let mut rows = Vec::new();
        for name in crate::experience::allowed_tiers(exp, MODE_ID) {
            let t = crate::consts::TIER_ORDER.iter().position(|x| *x == name).unwrap_or(0);
            for row in crate::words::tier_for(lang, name) {
                let w = row.split('|').next().unwrap_or(row);
                if crate::profanity::is_blocked(w) || !crate::audio_verdict::servable(lang, w, "normal") {
                    continue;
                }
                // German capitalises every noun, so case says nothing there.
                if lang != "de" && w.chars().next().map_or(false, char::is_uppercase) {
                    continue;
                }
                rows.push((t, w.to_string()));
            }
        }
        Bank::from_rows(rows)
    }
}

/// F9/D1/I6: is Randospell offered for this language at all? Pure of any
/// remote flag: the hub additionally needs `flags::randospell()`.
pub fn eligible(lang: &str, exp: crate::experience::Experience) -> bool {
    !INELIGIBLE.contains(&lang) && Bank::for_lang(lang, exp).cov.viable()
}
