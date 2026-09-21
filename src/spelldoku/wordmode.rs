//! CC-SPELLDOKU v1.2 Phase A2 — Word Mode (F10–F13), in the binding layer.
//!
//! A Word Mode board's N symbols are real bank words. This file chooses them
//! and hands the core nothing but glyph ids (`bind::word_symbols`, I14):
//!
//! - F12 band mix: 9×9 Medium 3 medium + 6 easy, Hard 5 hard + 4 medium,
//!   Expert 6 expert + 3 hard; 12×12 Expert 8 expert + 4 hard (Phase A3).
//! - F11 sourcing ladder, per band: the player's My Words, then the
//!   missed-words queue, then the bank. Nothing is generated (I13).
//! - F13 gates, as ruled on 2026-09-19 (docs/CC-SPELLDOKU-v1.2.md):
//!   R1 a bank row of an enabled language is eligible; R3 initial glyphs are
//!   pairwise distinct under each script's rule; R6 at most 12 graphemes;
//!   R4 no two words one confusion edit apart where a confusion list exists
//!   (English). Two more follow from the spec's own rules: the legend is an
//!   audio orb, so two words that sound alike never share a board; and a symbol
//!   is spelled on the in-app keyboard (D10), so every word must be typeable
//!   on it.
//! - R5: a word that fails a gate is redrawn on its own; the set is kept.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use super::bind::{word_symbols, Board, Mode, WordSymbol};
use super::gen::{generate, Config, Tier};
use super::rng::{mix, Rng};
use super::table::norm;

/// R6: the longest word a symbol may be, in graphemes.
pub const MAX_GRAPHEMES: usize = 12;

/// D12 / D16: Word Mode is 9×9 Medium and up; Spell Jr never.
pub fn is_word_mode(kid: bool, n: usize, tier: Tier) -> bool {
    !kid && n == 9 && tier != Tier::Easy
}

/// F12: the band mix of a Word Mode board.
pub fn band_mix(n: usize, tier: Tier) -> Option<&'static [(&'static str, usize)]> {
    match (n, tier) {
        (9, Tier::Medium) => Some(&[("medium", 3), ("easy", 6)]),
        (9, Tier::Hard) => Some(&[("hard", 5), ("medium", 4)]),
        (9, Tier::Expert) => Some(&[("expert", 6), ("hard", 3)]),
        _ => None,
    }
}

/// The player's own words (F11), read by the screen and passed in, so this file
/// stays a pure function of its inputs.
#[derive(Clone, Debug, Default)]
pub struct Personal {
    /// My Words entries in this language, as typed.
    pub mine: Vec<String>,
    /// Missed-words queue entries in this language: (word, band).
    pub missed: Vec<(String, String)>,
}

const SMALL_KANA: &str = "ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶーゝゞヽヾ";

/// R3: the glyph a cell shows, and the key its distinctness is judged on.
/// Chinese shows the first Hanzi of the bank row; every other script the
/// first grapheme of the word. None when the word cannot lead a symbol:
/// Cyrillic ы/ь/ъ and Japanese small kana never begin a word.
pub fn glyph(lang: &str, spelling: &str, display: Option<&str>) -> Option<(String, String)> {
    let first = match (lang, display) {
        ("zh", Some(d)) => d.graphemes(true).next()?.to_string(),
        ("zh", None) => return None,
        _ => spelling.graphemes(true).next()?.to_string(),
    };
    if lang == "ru" && ["ы", "ь", "ъ"].contains(&first.as_str()) {
        return None;
    }
    if lang == "ja" && first.chars().next().is_some_and(|c| SMALL_KANA.contains(c)) {
        return None;
    }
    let key = first.to_lowercase();
    // Shown in capitals where the script has them, unless that changes its
    // length (German ß).
    let up = first.to_uppercase();
    let show = if up.graphemes(true).count() == 1 { up } else { first };
    Some((show, key))
}

/// D10: the keys of the language's in-app keyboard, long-press alternates
/// included.
fn keyboard(lang: &str) -> &'static HashSet<char> {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static HashSet<char>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(k) = cache.lock().unwrap().get(lang) {
        return k;
    }
    let raw = match lang {
        "es" => include_str!("../../assets/keyboards/es.json"),
        "fr" => include_str!("../../assets/keyboards/fr.json"),
        "de" => include_str!("../../assets/keyboards/de.json"),
        "pt" => include_str!("../../assets/keyboards/pt.json"),
        "pl" => include_str!("../../assets/keyboards/pl.json"),
        "ru" => include_str!("../../assets/keyboards/ru.json"),
        "vi" => include_str!("../../assets/keyboards/vi.json"),
        "ko" => include_str!("../../assets/keyboards/ko.json"),
        "ja" => include_str!("../../assets/keyboards/ja.json"),
        "zh" => include_str!("../../assets/keyboards/zh.json"),
        "fil" => include_str!("../../assets/keyboards/fil.json"),
        "sw" => include_str!("../../assets/keyboards/sw.json"),
        "ar" => include_str!("../../assets/keyboards/ar.json"),
        "hi" => include_str!("../../assets/keyboards/hi.json"),
        _ => include_str!("../../assets/keyboards/en.json"),
    };
    let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    let mut keys = HashSet::new();
    for r in v["rows"].as_array().into_iter().flatten() {
        keys.extend(r.as_str().unwrap_or("").chars());
    }
    if let Some(lp) = v["longPress"].as_object() {
        for a in lp.values() {
            keys.extend(a.as_str().unwrap_or("").chars());
        }
    }
    let leaked: &'static HashSet<char> = Box::leak(Box::new(keys));
    cache.lock().unwrap().insert(lang.to_string(), leaked);
    leaked
}

/// Can this spelling be typed on the language's own keyboard, through the input
/// the screen uses? Mandarin types letters and tone numbers; Vietnamese adds
/// its tone row; Korean composes syllables from jamo keys.
pub fn typeable(lang: &str, spelling: &str) -> bool {
    let keys = keyboard(lang);
    match lang {
        "zh" => spelling.chars().all(|c| keys.contains(&c) || ('1'..='5').contains(&c)),
        "vi" => {
            const TONES: [char; 5] = ['\u{0300}', '\u{0301}', '\u{0309}', '\u{0303}', '\u{0323}'];
            let base: String = spelling.nfd().filter(|c| !TONES.contains(c)).collect::<String>().nfc().collect();
            base.chars().all(|c| keys.contains(&c))
        }
        "ko" => spelling.chars().all(|c| crate::hangul::parts(c).is_some_and(|(i, m, f)| {
            keys.contains(&i) && (keys.contains(&m) || "ㅘㅙㅚㅝㅞㅟㅢ".contains(m)) && (f == '\0' || keys.contains(&f) || "ㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ".contains(f))
        })),
        _ => spelling.chars().all(|c| keys.contains(&c)),
    }
}

#[cfg(test)]
thread_local! {
    /// Test only: a bank with nothing in it, to exercise the F13 fallback.
    pub static EMPTY_BANK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One bank row, ready to be drawn.
#[derive(Clone, Debug)]
struct Cand {
    spelling: String,
    display: Option<String>,
    show: String,
    key: String,
    band: &'static str,
}

/// Every eligible bank row of a band (R1, R3, R6, typeable, not blocklisted),
/// in bank order. Built once per (language, band).
fn bank(lang: &str, band: &'static str) -> &'static [Cand] {
    #[cfg(test)]
    if EMPTY_BANK.with(|e| e.get()) {
        return &[];
    }
    static CACHE: OnceLock<Mutex<HashMap<(String, &'static str), &'static [Cand]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(c) = cache.lock().unwrap().get(&(lang.to_string(), band)) {
        return c;
    }
    let mut seen = HashSet::new();
    let rows: Vec<Cand> = crate::words::tier_for(lang, band)
        .iter()
        .filter_map(|entry| {
            let mut parts = entry.split('|');
            let spelling = norm(parts.next()?);
            let display = parts.next();
            candidate(lang, &spelling, display, band)
        })
        .filter(|c| seen.insert(c.spelling.clone()))
        .collect();
    let leaked: &'static [Cand] = Box::leak(rows.into_boxed_slice());
    cache.lock().unwrap().insert((lang.to_string(), band), leaked);
    leaked
}

/// How many eligible rows a band holds for this language. Tier Mode's F5 gate
/// reads this so there is one definition of "servable row" (I-T8).
pub fn depth(lang: &str, band: &'static str) -> usize {
    bank(lang, band).len()
}

/// Draw one eligible row of `band`, skipping anything already drawn on this
/// board (I-T4). Returns (what the player types, the bank's own form for audio).
pub fn draw_from(
    lang: &str,
    band: &'static str,
    rng: &mut Rng,
    used: &[String],
) -> Option<(String, Option<String>)> {
    draw_from_if(lang, band, rng, used, &|_| true)
}

/// Every spelling a band can serve, for tests and tools that need the pool
/// itself rather than a draw from it.
#[cfg(test)]
pub fn bank_words(lang: &str, band: &'static str) -> Vec<String> {
    bank(lang, band).iter().map(|c| c.spelling.clone()).collect()
}

/// The same draw, with one more thing the word has to be. v1.3 uses it to skip
/// words still inside their CC-WORDGRID D9 repeat window without copying the
/// window into a list first.
pub fn draw_from_if(
    lang: &str,
    band: &'static str,
    rng: &mut Rng,
    used: &[String],
    keep: &dyn Fn(&str) -> bool,
) -> Option<(String, Option<String>)> {
    let pool = bank(lang, band);
    if pool.is_empty() {
        return None;
    }
    let start = rng.below(pool.len());
    (0..pool.len())
        .map(|k| &pool[(start + k) % pool.len()])
        .find(|c| !used.contains(&c.spelling) && keep(&c.spelling))
        .map(|c| (c.spelling.clone(), c.display.clone()))
}

fn candidate(lang: &str, spelling: &str, display: Option<&str>, band: &'static str) -> Option<Cand> {
    let len = spelling.graphemes(true).count();
    if !(2..=MAX_GRAPHEMES).contains(&len) || !typeable(lang, spelling) || crate::profanity::is_blocked(spelling) {
        return None;
    }
    let (show, key) = glyph(lang, spelling, display)?;
    Some(Cand { spelling: spelling.to_string(), display: display.map(str::to_string), show, key, band })
}

/// Which bank band an entry the player saved belongs to, if any: its row in
/// the bank, matched on the typed form (or, for Chinese, the characters).
fn band_of(lang: &str, text: &str) -> Option<(&'static str, Option<&'static str>)> {
    let t = norm(text);
    for band in ["easy", "medium", "hard", "expert"] {
        for entry in crate::words::tier_for(lang, band) {
            let mut parts = entry.split('|');
            let typed = parts.next().unwrap_or("");
            let display = parts.next();
            if norm(typed) == t || display.is_some_and(|d| d == text.trim()) {
                return Some((band, display));
            }
        }
    }
    None
}

/// F13 gates 1 and 4, plus the sound-alike rule: may `c` join `picked`?
pub fn compatible(lang: &str, picked: &[(String, String)], spelling: &str, key: &str) -> bool {
    picked.iter().all(|(s, k)| {
        k != key
            && s != spelling
            && !crate::homophones::accepts(lang, s, spelling)
            && !(crate::wordsearch::confusion::has_decoys(lang) && confusable(lang, s, spelling))
    })
}

/// R4: one confusion edit apart, either way (English list).
pub fn confusable(lang: &str, a: &str, b: &str) -> bool {
    crate::wordsearch::confusion::edits(lang, a).contains(b) || crate::wordsearch::confusion::edits(lang, b).contains(a)
}

/// F11–F13: choose the board's words, or None when this (language, size,
/// tier) cannot fill its mix -- the caller then serves Number Mode (F13
/// fallback). The order is the symbol order, shuffled.
pub fn choose(lang: &str, n: usize, tier: Tier, seed: u64, personal: &Personal) -> Option<Vec<WordSymbol>> {
    let mix_of = band_mix(n, tier)?;
    let mut rng = Rng::new(mix(seed, 0x5752_444D)); // "WRDM"
    let mut picked: Vec<(String, String)> = Vec::new();
    let mut out: Vec<WordSymbol> = Vec::new();
    for &(band, count) in mix_of {
        let mut mine: Vec<Cand> = personal
            .mine
            .iter()
            .filter_map(|w| band_of(lang, w).filter(|(b, _)| *b == band).and_then(|(_, d)| candidate(lang, &norm(w), d, band)))
            .collect();
        rng.shuffle(&mut mine);
        let mut missed: Vec<Cand> = personal
            .missed
            .iter()
            .filter(|(_, b)| b == band)
            .filter_map(|(w, _)| candidate(lang, &norm(w), band_of(lang, w).and_then(|(_, d)| d), band))
            .collect();
        rng.shuffle(&mut missed);
        let mut from_bank: Vec<&Cand> = bank(lang, band).iter().collect();
        rng.shuffle(&mut from_bank);
        let ladder = mine.iter().map(|c| (c, true)).chain(missed.iter().map(|c| (c, false))).chain(from_bank.into_iter().map(|c| (c, false)));
        let mut got = 0;
        for (c, is_mine) in ladder {
            if got == count {
                break;
            }
            if compatible(lang, &picked, &c.spelling, &c.key) {
                picked.push((c.spelling.clone(), c.key.clone()));
                out.push(WordSymbol {
                    spelling: c.spelling.clone(),
                    display: c.display.clone(),
                    glyph: c.show.clone(),
                    band: c.band,
                    mine: is_mine,
                });
                got += 1;
            }
        }
        if got < count {
            return None;
        }
    }
    rng.shuffle(&mut out);
    Some(out)
}

/// A Word Mode board, or None (serve Number Mode instead). Hard and Expert need
/// a necessary fragment, so a word set that cannot make one is redrawn.
pub fn board(lang: &str, cfg: &Config, seed: u64, personal: &Personal) -> Option<Board> {
    for k in 0..24u64 {
        let words = choose(lang, cfg.size.n, cfg.tier, mix(seed, k), personal)?;
        let (symbols, glyphs) = word_symbols(&words);
        if matches!(cfg.tier, Tier::Hard | Tier::Expert) && !symbols.fragments_possible() {
            continue;
        }
        let Some(puzzle) = generate(seed, cfg, &symbols) else { continue };
        let b = Board { lang: lang.to_string(), puzzle, glyphs, mode: Mode::Words(words) };
        // I12: a fragment that happens to spell another symbol's whole word
        // (a "_tone" beside "tone") would be a free spelling: draw again.
        if b.spells_a_symbol() {
            continue;
        }
        return Some(b);
    }
    None
}

/// What the screen serves for a configuration: Word Mode where D12 says so and
/// a set can be drawn, otherwise Number Mode -- never a partial board (F13).
pub fn board_or_number(
    lang: &str,
    kid: bool,
    cfg: &Config,
    seed: u64,
    personal: &Personal,
    numbers: Option<&super::table::Table>,
) -> Option<Board> {
    let words = if is_word_mode(kid, cfg.size.n, cfg.tier) { board(lang, cfg, seed, personal) } else { None };
    words.or_else(|| numbers.and_then(|t| Board::number(t, seed, cfg)))
}
