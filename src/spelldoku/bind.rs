//! The binding layer (I14): everything between the symbol-agnostic core and
//! the words a player sees and types.
//!
//! It turns a symbol set -- the number-word table (Number Mode) or N bank words
//! (Word Mode, `wordmode`) -- into the core's glyph-id `Symbols`, and turns a
//! board back into text: what a cell, a chip, a pencil mark or a fragment
//! shows. The screen draws only what these functions return, so the I12 check
//! ("no surface spells an unsolved symbol") can be run on them on the host.

use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use super::gen::{generate, Clue, Config, Puzzle, Tier};
use super::symbols::Symbols;
use super::table::{spelling_matches, Table};

/// Glyph ids, in first-seen order.
#[derive(Clone, Debug, Default)]
pub struct Glyphs(Vec<String>);

impl Glyphs {
    pub fn id(&mut self, g: &str) -> u32 {
        match self.0.iter().position(|x| x == g) {
            Some(i) => i as u32,
            None => {
                self.0.push(g.to_string());
                (self.0.len() - 1) as u32
            }
        }
    }

    pub fn text(&self, id: u32) -> &str {
        self.0.get(id as usize).map(String::as_str).unwrap_or("")
    }

    pub fn encode(&mut self, s: &str) -> Vec<u32> {
        s.graphemes(true).map(|g| self.id(g)).collect()
    }
}

/// One symbol of a Word Mode board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSymbol {
    /// What the player types (NFC, lowercase; toned pinyin for zh).
    pub spelling: String,
    /// The one large character a cell shows (F10, I11).
    pub glyph: String,
    /// How the bank writes it, where that differs from the spelling: the
    /// Hanzi of a Chinese row. Audio needs it; the screen never shows it.
    pub display: Option<String>,
    /// The bank band it was drawn from (F12).
    pub band: &'static str,
    /// F11: drawn from the player's own My Words.
    pub mine: bool,
}

#[derive(Clone, Debug)]
pub enum Mode {
    Number(Table),
    Words(Vec<WordSymbol>),
}

/// A served board: the core's puzzle, its symbols, and how to show them.
#[derive(Clone, Debug)]
pub struct Board {
    pub lang: String,
    pub puzzle: Puzzle,
    pub glyphs: Glyphs,
    pub mode: Mode,
}

/// Number Mode symbols: the accepted spellings (D4) of 1..=n, as glyph ids.
/// Interning runs value by value, spelling by spelling, so the ids -- and with
/// them every board -- are the same on every platform (I5).
pub fn number_symbols(table: &Table, n: usize) -> (Symbols, Glyphs) {
    let mut glyphs = Glyphs::default();
    let forms = (1..=n as u32)
        .map(|v| table.row(v).map(|r| r.spellings.iter().map(|s| glyphs.encode(s)).collect()).unwrap_or_default())
        .collect();
    (Symbols::new(forms), glyphs)
}

/// Word Mode symbols: one form per word, its typed spelling.
pub fn word_symbols(words: &[WordSymbol]) -> (Symbols, Glyphs) {
    let mut glyphs = Glyphs::default();
    let forms = words.iter().map(|w| vec![glyphs.encode(&w.spelling)]).collect();
    (Symbols::aimed(forms), glyphs)
}

impl Board {
    pub fn number(table: &Table, seed: u64, cfg: &Config) -> Option<Board> {
        let (symbols, glyphs) = number_symbols(table, cfg.size.n);
        let puzzle = generate(seed, cfg, &symbols)?;
        Some(Board { lang: table.lang.clone(), puzzle, glyphs, mode: Mode::Number(table.clone()) })
    }

    pub fn symbols(&self) -> Symbols {
        match &self.mode {
            Mode::Number(t) => number_symbols(t, self.puzzle.n).0,
            Mode::Words(w) => word_symbols(w).0,
        }
    }

    pub fn is_words(&self) -> bool {
        matches!(self.mode, Mode::Words(_))
    }

    /// The spelling a symbol's audio says and a hint plays (F9 level 3).
    pub fn spelling(&self, v: u8) -> Option<String> {
        match &self.mode {
            Mode::Number(t) => t.word(v).map(str::to_string),
            Mode::Words(w) => w.get((v as usize).checked_sub(1)?).map(|s| s.spelling.clone()),
        }
    }

    /// How a placed or chosen symbol is shown: its numeral, or its glyph.
    pub fn label(&self, v: u8) -> String {
        match &self.mode {
            Mode::Number(_) => v.to_string(),
            Mode::Words(w) => w.get((v as usize).wrapping_sub(1)).map(|s| s.glyph.clone()).unwrap_or_default(),
        }
    }

    /// The symbols whose spelling matches what was typed.
    pub fn values_for(&self, typed: &str) -> Vec<u8> {
        match &self.mode {
            Mode::Number(t) => t.values_for(typed),
            Mode::Words(w) => w
                .iter()
                .enumerate()
                .filter(|(_, s)| spelling_matches(&self.lang, typed, &s.spelling))
                .map(|(i, _)| i as u8 + 1)
                .collect(),
        }
    }

    pub fn fragment_text(&self, p: &[Option<u32>]) -> Vec<Option<String>> {
        p.iter().map(|g| g.map(|id| self.glyphs.text(id).to_string())).collect()
    }

    /// The text a clue cell shows. A spelled given is spelled out only in
    /// Number Mode; in Word Mode every given shows its glyph (I12).
    pub fn clue_text(&self, i: usize) -> String {
        match &self.puzzle.clues[i] {
            Clue::Given(v) => self.label(*v),
            Clue::Spelled(v) => match &self.mode {
                Mode::Number(t) => t.word(*v).unwrap_or("").to_string(),
                Mode::Words(_) => self.label(*v),
            },
            Clue::Fragment(p) => self.fragment_text(p).into_iter().map(|g| g.unwrap_or_else(|| "_".into())).collect::<Vec<_>>().join(""),
            Clue::Empty => String::new(),
        }
    }

    /// Every piece of text the screen can show for this board: each clue
    /// cell, each symbol label (cells, chips, pencil marks, legend). I12 is
    /// checked against exactly this.
    pub fn surfaces(&self) -> Vec<String> {
        let mut out: Vec<String> = (0..self.puzzle.clues.len()).map(|i| self.clue_text(i)).collect();
        out.extend((1..=self.puzzle.n as u8).map(|v| self.label(v)));
        out
    }

    /// I12: does any surface spell out a symbol's whole word? (A solved cell
    /// shows only the glyph, so this holds for solved symbols too.)
    pub fn spells_a_symbol(&self) -> bool {
        let Mode::Words(w) = &self.mode else { return false };
        let shown: Vec<String> = self.surfaces().iter().map(|s| super::table::norm(s)).collect();
        w.iter().any(|s| {
            let word = super::table::norm(&s.spelling);
            shown.iter().any(|t| t.contains(&word))
        })
    }

    /// F11 / D14: how many symbols came from the player's own words.
    pub fn mine(&self) -> usize {
        match &self.mode {
            Mode::Words(w) => w.iter().filter(|s| s.mine).count(),
            Mode::Number(_) => 0,
        }
    }
}

/// The puzzle as Phase A serialized it (number words and the language in it).
/// The golden digest is computed over this shape, so the refactor to glyph ids
/// is proven not to have moved a single board: the pinned value is unchanged.
#[derive(Serialize)]
struct LegacyPuzzle {
    n: usize,
    tier: Tier,
    lang: String,
    clues: Vec<LegacyClue>,
    solution: Vec<u8>,
    seed: u64,
}

#[derive(Serialize)]
enum LegacyClue {
    Empty,
    Given(u8),
    Word(u8),
    Fragment(Vec<Option<String>>),
}

/// I5 / Done #3: a digest of fixed seeds across every configuration. The host
/// test pins it and the browser test asks the wasm build for the same value.
pub fn golden_digest(table: &Table) -> u64 {
    let mut bytes = Vec::new();
    for &(n, tier) in &[(4usize, Tier::Easy), (6, Tier::Easy), (6, Tier::Medium), (9, Tier::Easy), (9, Tier::Medium), (9, Tier::Hard), (9, Tier::Expert)] {
        let cfg = Config { size: super::geo::size_of(n).expect("size"), tier };
        for seed in 0..3u64 {
            if let Some(b) = Board::number(table, seed, &cfg) {
                let p = &b.puzzle;
                let legacy = LegacyPuzzle {
                    n: p.n,
                    tier: p.tier,
                    lang: b.lang.clone(),
                    clues: p
                        .clues
                        .iter()
                        .map(|c| match c {
                            Clue::Empty => LegacyClue::Empty,
                            Clue::Given(v) => LegacyClue::Given(*v),
                            Clue::Spelled(v) => LegacyClue::Word(*v),
                            Clue::Fragment(f) => LegacyClue::Fragment(b.fragment_text(f)),
                        })
                        .collect(),
                    solution: p.solution.clone(),
                    seed: p.seed,
                };
                bytes.extend(serde_json::to_vec(&legacy).unwrap_or_default());
            }
        }
    }
    super::rng::fnv(&bytes)
}
