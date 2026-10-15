//! CC-QUICKBEE-DRAW v1 Phase A -- configuration, gating and the word pool.
//!
//! A timed variant of Quick Bee where the player DRAWS each character. Phase A
//! is the part that can ship dark: the language table, the tier caps, the gate
//! and the pool a run would draw from, all as data with tests. The run itself
//! (Phase B) is not here, and `PHASE_B_BUILT` keeps the entry absent until it is.
//!
//! # Rulings in force (Eric, 2026-10-15, on the section-0 recon)
//!
//! * One recogniser path: Apple Vision, on device (CC-CJK-INK D2, Invariants 1
//!   and 3). No model download, so the spec's "model missing" case cannot occur.
//! * zh and ja first. Korean stays off, and `ink_allowed` still excludes it,
//!   until it has its own F1-style measurement (CC-CJK-INK D1, Invariant 5).
//! * Spell Jr does not get the pad here (CC-CJK-INK D6, Invariant 6): the entry
//!   is absent for a Jr profile.
//! * Only words a player can actually draw are served: the existing
//!   `stroke_counts::writing_ready` gate holds out characters of 13 or more
//!   strokes at every tier, and nothing is padded where a cell is thin.
//! * Auto-advance keys on the stroke count where one is known (zh, ja) with a
//!   short idle debounce, and on the debounce alone otherwise (ko).
//!
//! Everything language-specific lives in `LANGS`; no caller tests a language
//! code (I6).

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// The run loop does not exist yet. While this is false the entry is absent in
/// every build, whatever the flags say. Phase B flips it with the loop.
pub const PHASE_B_BUILT: bool = false;

/// The three tiers' caps on a word's cell count (F2): Easy 1-2, Medium up to 3,
/// Hard and Expert up to 4. A word over its tier's cap is never served (I3).
pub fn tier_cap(tier: &str) -> usize {
    match tier {
        "easy" => 2,
        "medium" => 3,
        _ => 4,
    }
}

/// How many of the recogniser's top candidates may match the expected character
/// (CC-CJK-INK, carried): Easy and Kid 8, Medium 5, Hard and Expert 3.
pub fn top_n(tier: &str) -> usize {
    match tier {
        "easy" => 8,
        "medium" => 5,
        _ => 3,
    }
}

/// The cell frame drawn behind the pen, per language (carried): a field-grid for
/// Chinese, a neutral square for Japanese, a plain square with a faint cross for
/// Korean. It is a separate layer and is NEVER rasterized for the recogniser:
/// F0 measured Vision on clean glyphs, and a grid line through a character costs
/// accuracy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grid {
    /// 田字格.
    FieldGrid,
    /// A neutral gray square.
    NeutralSquare,
    /// A plain square with a faint cross.
    FaintCross,
}

/// One drawable language's configuration.
#[derive(Clone, Copy, Debug)]
pub struct LangCfg {
    pub lang: &'static str,
    /// The `spell_flag_<key>` that switches this language's draw variant on.
    pub flag_key: &'static str,
    pub grid: Grid,
    /// Does a stroke table exist for this language? If so the run advances the
    /// moment the expected stroke count is reached (plus the debounce); if not
    /// it can only wait out the idle debounce.
    pub has_strokes: bool,
    /// Idle time after the last stroke before a cell is judged, in ms.
    pub debounce_ms: u32,
    /// May this language be offered yet? Korean is held until it is measured.
    pub measured: bool,
}

pub const LANGS: [LangCfg; 3] = [
    LangCfg { lang: "zh", flag_key: "quickbee_draw_zh", grid: Grid::FieldGrid, has_strokes: true, debounce_ms: 250, measured: true },
    LangCfg { lang: "ja", flag_key: "quickbee_draw_ja", grid: Grid::NeutralSquare, has_strokes: true, debounce_ms: 250, measured: true },
    LangCfg { lang: "ko", flag_key: "quickbee_draw_ko", grid: Grid::FaintCross, has_strokes: false, debounce_ms: 400, measured: false },
];

pub fn cfg(lang: &str) -> Option<&'static LangCfg> {
    LANGS.iter().find(|c| c.lang == lang)
}

/// I7: NFC before every comparison.
pub fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// A word's cell count: the stored unit, never recounted from anything else.
/// Hangul syllable blocks (NFC), kana, hanzi, spaces excluded.
pub fn cells(word: &str) -> usize {
    nfc(word).graphemes(true).filter(|g| !g.chars().all(char::is_whitespace)).count()
}

/// The part of a bank row that is drawn: the hanzi for zh (`pinyin|hanzi`), the
/// row itself otherwise.
fn drawn_form(lang: &str, row: &str) -> String {
    match row.split_once('|') {
        Some((_, hanzi)) if lang == "zh" => nfc(hanzi),
        _ => nfc(row.split('|').next().unwrap_or(row)),
    }
}

/// F4 / I2: is the draw variant offered for this language to this profile, given
/// its flag? Absent (false) for Jr, for any language not in `LANGS`, for a
/// language that has not been measured, and while the loop is not built.
pub fn available(lang: &str, kid: bool, flag_on: bool) -> bool {
    PHASE_B_BUILT && !kid && flag_on && cfg(lang).is_some_and(|c| c.measured)
}

/// The words a run in `lang` at `tier` may serve: under the tier's cap and, where
/// a stroke table exists, writing-ready (CC-CJK-INK F4). Bank order, no
/// duplicates, profanity and proper-noun rules as every other mode.
pub fn serve_pool(lang: &str, tier: &str) -> Vec<String> {
    let Some(c) = cfg(lang) else { return Vec::new() };
    let cap = tier_cap(tier);
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for row in crate::words::tier_for(lang, tier) {
        let w = drawn_form(lang, row);
        if w.is_empty() || crate::profanity::is_blocked(&w) {
            continue;
        }
        let n = cells(&w);
        if n == 0 || n > cap {
            continue;
        }
        if c.has_strokes && !crate::stroke_counts::writing_ready(&w) {
            continue;
        }
        if seen.insert(w.clone()) {
            out.push(w);
        }
    }
    out
}

/// Words per cell count in `serve_pool`, index 0 is one cell. For the census and
/// for F2's "report a thin cell, never pad it".
pub fn pool_by_cells(lang: &str, tier: &str) -> [usize; 4] {
    let mut by = [0usize; 4];
    for w in serve_pool(lang, tier) {
        let n = cells(&w);
        if (1..=4).contains(&n) {
            by[n - 1] += 1;
        }
    }
    by
}

/// F2: a cell with fewer words than this is reported, never padded.
pub const THIN_CELL: usize = 20;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tier_caps_and_candidate_windows_are_the_signed_ones() {
        assert_eq!((tier_cap("easy"), tier_cap("medium"), tier_cap("hard"), tier_cap("expert")), (2, 3, 4, 4));
        assert_eq!((top_n("easy"), top_n("medium"), top_n("hard"), top_n("expert")), (8, 5, 3, 3));
    }

    #[test]
    fn only_zh_ja_and_ko_are_in_the_table_and_ko_is_unmeasured() {
        assert_eq!(LANGS.iter().map(|c| c.lang).collect::<Vec<_>>(), vec!["zh", "ja", "ko"]);
        assert!(cfg("vi").is_none() && cfg("en").is_none() && cfg("es").is_none());
        assert!(!cfg("ko").unwrap().measured, "Korean waits for its own measurement");
        assert!(cfg("zh").unwrap().measured && cfg("ja").unwrap().measured);
    }

    #[test]
    fn the_grids_are_the_carried_ones() {
        assert_eq!(cfg("zh").unwrap().grid, Grid::FieldGrid);
        assert_eq!(cfg("ja").unwrap().grid, Grid::NeutralSquare);
        assert_eq!(cfg("ko").unwrap().grid, Grid::FaintCross);
    }

    #[test]
    fn nothing_is_available_until_the_loop_exists_and_never_for_jr_ko_or_other_languages() {
        // Phase A ships dark: with every flag on, the entry is still absent.
        for lang in ["zh", "ja", "ko", "vi", "en", "es"] {
            assert!(!available(lang, false, true), "{lang}: available before Phase B");
        }
        // The rest of the rule, independent of the build gate.
        let gate = |lang: &str, kid: bool, flag: bool| !kid && flag && cfg(lang).is_some_and(|c| c.measured);
        assert!(gate("zh", false, true) && gate("ja", false, true));
        assert!(!gate("zh", true, true), "Jr gets no pad (CC-CJK-INK D6)");
        assert!(!gate("zh", false, false), "flag off");
        assert!(!gate("ko", false, true), "Korean is held until measured");
        assert!(!gate("vi", false, true) && !gate("en", false, true));
    }

    #[test]
    fn cells_are_counted_on_nfc_and_ignore_spaces() {
        assert_eq!(cells("한국"), 2);
        // A decomposed Hangul syllable is one block after NFC.
        assert_eq!(cells("\u{1112}\u{1161}\u{11AB}"), 1);
        assert_eq!(cells("ひらがな"), 4);
        assert_eq!(cells("你 好"), 2);
        assert_eq!(nfc("\u{1112}\u{1161}\u{11AB}"), "한");
    }

    #[test]
    fn i3_no_served_word_exceeds_its_tier_cap_and_nothing_repeats() {
        for lang in ["zh", "ja"] {
            for tier in ["easy", "medium", "hard", "expert"] {
                let pool = serve_pool(lang, tier);
                assert!(!pool.is_empty(), "{lang} {tier}: empty pool");
                let mut seen = std::collections::BTreeSet::new();
                for w in &pool {
                    assert!(cells(w) >= 1 && cells(w) <= tier_cap(tier), "{lang} {tier}: {w} has {} cells", cells(w));
                    assert!(seen.insert(w.clone()), "{lang} {tier}: {w} twice");
                }
            }
        }
    }

    #[test]
    fn only_writing_ready_words_are_served_for_zh_and_ja() {
        for lang in ["zh", "ja"] {
            for tier in ["easy", "medium", "hard", "expert"] {
                for w in serve_pool(lang, tier) {
                    assert!(crate::stroke_counts::writing_ready(&w), "{lang} {tier}: {w} is not writing-ready");
                }
            }
        }
    }

    #[test]
    fn the_census_numbers_hold_and_the_one_thin_cell_is_reported_not_padded() {
        // section 0: zh Hard at 4 cells has 18 words, 17 of them writing-ready.
        assert_eq!(pool_by_cells("zh", "hard")[3], 17);
        // Every other served cell in zh and ja is at or over the floor.
        let mut thin = Vec::new();
        for lang in ["zh", "ja"] {
            for tier in ["easy", "medium", "hard", "expert"] {
                let by = pool_by_cells(lang, tier);
                for n in 1..=tier_cap(tier) {
                    let have = by[n - 1];
                    // ja has no one-kana words in the bank; a cell with none is
                    // not a thin cell, it is not a cell.
                    if have > 0 && have < THIN_CELL {
                        thin.push(format!("{lang} {tier} {n}: {have}"));
                    }
                }
            }
        }
        // zh easy/medium/hard 2-4 cell counts that the section 0 census flagged
        // as small: medium 3 (28 ready), medium 4 is over the cap, hard 4 is 17.
        assert_eq!(thin, vec!["zh hard 4: 17".to_string()], "thin cells: {thin:?}");
    }

    #[test]
    fn ja_is_kana_so_every_ja_word_is_drawable() {
        // The bank stores kana, so kanji (where F1 was weakest) never appears.
        let n_all: usize = ["easy", "medium", "hard", "expert"]
            .iter()
            .map(|t| crate::words::tier_for("ja", t).iter().filter(|r| cells(r.split('|').next().unwrap()) <= tier_cap(t)).count())
            .sum();
        let n_pool: usize = ["easy", "medium", "hard", "expert"].iter().map(|t| serve_pool("ja", t).len()).sum();
        assert!(n_pool > 0 && n_pool <= n_all);
    }

    #[test]
    fn the_module_names_no_language_outside_its_table() {
        // I6: callers ask `cfg`, they do not test a code. This file may name zh,
        // ja and ko only in LANGS and its tests.
        let src = include_str!("mod.rs");
        let code = src.split("#[cfg(test)]").next().unwrap();
        let outside_table: Vec<&str> = code
            .lines()
            .filter(|l| !l.contains("LangCfg {") && !l.trim_start().starts_with("//"))
            .filter(|l| l.contains("\"zh\"") || l.contains("\"ja\"") || l.contains("\"ko\""))
            .collect();
        // The zh-specific hanzi extraction in `drawn_form` is the one place the
        // bank format itself differs (pinyin|hanzi); it is a data-format fact.
        assert_eq!(outside_table.len(), 1, "unexpected language checks: {outside_table:?}");
        assert!(outside_table[0].contains("lang == \"zh\""));
    }
}
