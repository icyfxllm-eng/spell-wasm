//! The board as markup, a pure function of the play state (F3, F4, F7, F14).
//!
//! Nothing in the output depends on what an answer is until the player has put
//! it there (I3): an untouched cell is a rune glyph with a numbered label, a
//! decoded cell shows the unit a committed entry gave it, and no attribute
//! carries an answer. The caller supplies the translator so the host tests can
//! run with no locale table.

use crate::spelldoku::rng::{mix, Rng};

use std::collections::BTreeMap;

use super::pencil;
use super::play::{Check, Play, Stars};
use super::types::{Board, SlotKind};
use super::view::{board_view, RuneState};

/// How many glyphs the bundled sheet holds (`assets/spelluzzle/runes.svg`).
pub const GLYPHS: usize = 28;

/// Translate a key, filling `{name}` placeholders.
pub type Tr<'a> = &'a dyn Fn(&str, &[(&str, &str)]) -> String;

/// Which glyph stands for each rune on this board: a seeded permutation, so a
/// glyph never tells you anything about the letter (F1, F14).
pub fn glyph_map(board: &Board) -> Vec<usize> {
    let mut v: Vec<usize> = (0..GLYPHS).collect();
    Rng::new(mix(board.seed, 0x474c_5950)).shuffle(&mut v);
    v.truncate(board.n_runes());
    v
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn glyph_svg(g: usize) -> String {
    format!("<svg class=\"sz-rune\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><use href=\"#rune-{g}\"/></svg>")
}

fn cell(class: &str, state: &str, rune: u8, aria: &str, inner: &str) -> String {
    format!("<span class=\"sz-cell {class}\" data-state=\"{state}\" data-rune=\"{rune}\" aria-label=\"{}\">{inner}</span>", esc(aria))
}

/// Phase E layers drawn over the v1 board. `Default` is v1, byte for byte (I13).
#[derive(Default, Clone)]
pub struct Opts {
    /// F16: marks to draw (the caller passes only the visible ones).
    pub marks: BTreeMap<u8, char>,
    /// F17: the rune whose cells are lit.
    pub highlight: Option<u8>,
    /// F16: the rune a long-press is waiting to mark.
    pub target: Option<u8>,
}

impl Opts {
    fn is_default(&self) -> bool {
        self.marks.is_empty() && self.highlight.is_none() && self.target.is_none()
    }
}

fn rune_label(tr: Tr, rune: u8) -> String {
    tr("spz.runeLabel", &[("k", &(rune as u32 + 1).to_string())])
}

/// One slot's cells.
fn slot_cells_html(p: &Play, view: &[RuneState], glyphs: &[usize], slot: usize, tr: Tr, opts: &Opts) -> String {
    let s = &p.board.slots[slot];
    let typed: Option<&Vec<char>> = p.entries.get(&slot).or_else(|| p.drafts.get(&slot));
    let mut h = String::new();
    for (k, &r) in s.runes.iter().enumerate() {
        let st = view[r as usize];
        let amber = st == RuneState::Contested;
        let mut c = if let Some(u) = typed.and_then(|t| t.get(k)) {
            let class = if amber { "typed amber" } else { "typed" };
            cell(class, if amber { "contested" } else { "typed" }, r, &format!("{} {}", rune_label(tr, r), u), &esc(&u.to_string()))
        } else {
            match st {
                RuneState::Unknown => match opts.marks.get(&r) {
                    // F16: a pencilled cell keeps its glyph, smaller, with the mark in the corner.
                    Some(m) => cell(
                        "pencilled",
                        "unknown",
                        r,
                        &format!("{} {}", rune_label(tr, r), tr("spz.pencil.label", &[("unit", &m.to_string())])),
                        &format!("{}<small class=\"sz-pencil\">{}</small>", glyph_svg(glyphs[r as usize]), esc(&m.to_string())),
                    ),
                    None => cell("", "unknown", r, &rune_label(tr, r), &glyph_svg(glyphs[r as usize])),
                },
                RuneState::Contested => cell("amber", "contested", r, &rune_label(tr, r), &glyph_svg(glyphs[r as usize])),
                RuneState::Decoded(u) => cell("decoded", "decoded", r, &format!("{} {}", rune_label(tr, r), u), &esc(&u.to_string())),
            }
        };
        if opts.highlight == Some(r) {
            c = c.replacen("class=\"sz-cell ", "class=\"sz-cell hl ", 1);
        }
        if opts.target == Some(r) {
            c = c.replacen("class=\"sz-cell ", "class=\"sz-cell target ", 1);
        }
        h.push_str(&c);
    }
    h
}

/// The whole board: one row per slot.
pub fn board_html(p: &Play, tr: Tr) -> String {
    let view = board_view(&p.board, &p.entries);
    board_html_with(p, &view, tr, &Opts::default())
}

/// The board under an explicit view (the ripple holds some runes back) and Phase E options.
pub fn board_html_with(p: &Play, view: &[RuneState], tr: Tr, opts: &Opts) -> String {
    let glyphs = glyph_map(&p.board);
    let check = p.check();
    let off: Vec<usize> = match &check {
        Some(Check::Slots(v)) => v.clone(),
        _ => Vec::new(),
    };
    let mut h = String::new();
    for (i, s) in p.board.slots.iter().enumerate() {
        let mut class = String::from("sz-row");
        let kind = match s.kind {
            SlotKind::Spoken => "spoken",
            SlotKind::Silent => "silent",
            SlotKind::Secret => "secret",
        };
        if p.selected == Some(i) {
            class.push_str(" sel");
        }
        if p.is_silent(i) {
            class.push_str(" quiet");
        }
        if off.contains(&i) {
            class.push_str(" off");
        }
        h.push_str(&format!("<div class=\"{class}\" id=\"szRow{i}\" data-slot=\"{i}\" data-kind=\"{kind}\">"));
        let mut head = String::new();
        if s.kind == SlotKind::Secret {
            head.push_str(&format!("<span class=\"sz-label\">{}</span>", esc(&tr("spz.secret", &[]))));
        }
        // F15 / census C21: on a Par board every word is silent, so Listen is one shared bar above the
        // keyboard (`par_bar_html`), not a button on every row.
        if p.is_silent(i) && p.board.par.is_none() {
            head.push_str(&format!(
                "<button type=\"button\" class=\"sz-listen\" data-listen=\"{i}\">{}</button><span class=\"sz-cost\">{}</span>",
                esc(&tr("spz.listen", &[])),
                esc(&tr("spz.listenCost", &[]))
            ));
        }
        if p.entries.contains_key(&i) && !p.solved() {
            head.push_str(&format!("<button type=\"button\" class=\"sz-clear\" data-clear=\"{i}\">{}</button>", esc(&tr("spz.clear", &[]))));
        }
        if !head.is_empty() {
            h.push_str(&format!("<div class=\"sz-head\">{head}</div>"));
        }
        h.push_str(&format!("<div class=\"sz-cells\" dir=\"ltr\">{}</div></div>", slot_cells_html(p, view, &glyphs, i, tr, opts)));
    }
    h
}

/// The rune key under the board: every rune with what is known about it.
pub fn rune_key_html(p: &Play, tr: Tr) -> String {
    let view = board_view(&p.board, &p.entries);
    let glyphs = glyph_map(&p.board);
    let mut h = String::new();
    for r in 0..p.board.n_runes() as u8 {
        let (class, state, inner) = match view[r as usize] {
            RuneState::Unknown => ("", "unknown", String::new()),
            RuneState::Decoded(u) => ("decoded", "decoded", esc(&u.to_string())),
            RuneState::Contested => ("amber", "contested", String::new()),
        };
        let aria = match view[r as usize] {
            RuneState::Decoded(u) => format!("{} {}", rune_label(tr, r), u),
            _ => rune_label(tr, r),
        };
        h.push_str(&format!(
            "<span class=\"sz-k {class}\" data-state=\"{state}\" aria-label=\"{}\">{}<b>{inner}</b></span>",
            esc(&aria),
            glyph_svg(glyphs[r as usize])
        ));
    }
    h
}

/// F7: the full-board message. Medium and above get the count only.
pub fn check_text(p: &Play, tr: Tr) -> String {
    match p.check() {
        Some(Check::Count(n)) => off_text(n, tr),
        Some(Check::Slots(v)) => off_text(v.len(), tr),
        None => String::new(),
    }
}

fn off_text(n: usize, tr: Tr) -> String {
    if n == 1 {
        tr("spz.offOne", &[])
    } else {
        tr("spz.off", &[("n", &n.to_string())])
    }
}

/// F8: shown only when the board is solved.
pub fn stars_html(p: &Play, tr: Tr) -> String {
    let Some(Stars { solved, sharp_ear, codebreaker }) = p.stars() else { return String::new() };
    let one = |on: bool, key: &str| format!("<li class=\"sz-star{}\"><span aria-hidden=\"true\">{}</span> {}</li>", if on { " on" } else { "" }, if on { "\u{2605}" } else { "\u{2606}" }, esc(&tr(key, &[])));
    format!("<ul class=\"sz-stars\">{}{}{}</ul>", one(solved, "spz.star.solved"), one(sharp_ear, "spz.star.ear"), one(codebreaker, "spz.star.code"))
}

/// F17: the rune strip. One entry per rune: its glyph, its cell count, and its unit once
/// decoded (or the pencil mark). Built from rune counts and the view only (I15).
pub fn strip_html(p: &Play, view: &[RuneState], tr: Tr, opts: &Opts) -> String {
    let glyphs = glyph_map(&p.board);
    let slots: Vec<&[u8]> = p.board.slots.iter().map(|s| s.runes.as_slice()).collect();
    let mut h = String::new();
    for e in pencil::strip(&slots, view, &opts.marks) {
        let (class, state, letter) = match e.state {
            RuneState::Unknown => ("", "unknown", e.mark.map(|m| m.to_string()).unwrap_or_default()),
            RuneState::Decoded(u) => ("decoded", "decoded", u.to_string()),
            RuneState::Contested => ("amber", "contested", String::new()),
        };
        let mut aria = tr("spz.strip.label", &[("k", &(e.rune as u32 + 1).to_string()), ("n", &e.cells.to_string())]);
        if let RuneState::Decoded(u) = e.state {
            aria.push_str(&format!(" {u}"));
        }
        let mut cls = format!("sz-k sz-s {class}");
        if opts.highlight == Some(e.rune) {
            cls.push_str(" hl");
        }
        if opts.target == Some(e.rune) {
            cls.push_str(" target");
        }
        h.push_str(&format!(
            "<button type=\"button\" class=\"{cls}\" data-rune=\"{}\" data-state=\"{state}\" aria-label=\"{}\">{}<small>{}<b>{}</b></small></button>",
            e.rune,
            esc(&aria),
            glyph_svg(glyphs[e.rune as usize]),
            e.cells,
            esc(&letter)
        ));
    }
    h
}

/// F15: "Listens n · Par p".
pub fn par_header(p: &Play, tr: Tr) -> String {
    match p.par() {
        Some(par) => tr("spz.par.header", &[("n", &p.listens().to_string()), ("p", &par.to_string())]),
        None => String::new(),
    }
}

/// F15: the one Listen button, for the selected word. Enabled only on a silent word that has not
/// been bought; the secret word never has one (v1 F6).
pub fn par_bar_html(p: &Play, tr: Tr) -> String {
    let Some(i) = p.selected else { return String::new() };
    if p.par().is_none() || !p.is_silent(i) || p.board.slots[i].kind == SlotKind::Secret {
        return String::new();
    }
    format!("<button type=\"button\" class=\"sz-listen sz-bar-listen\" data-listen=\"{i}\">{}</button>", esc(&tr("spz.listen", &[])))
}

/// F15 / D34: the result line, shown when the board is solved.
pub fn par_result(p: &Play, tr: Tr) -> String {
    let (Some(par), true) = (p.par(), p.solved()) else { return String::new() };
    let l = p.listens() as i64;
    let par = par as i64;
    match l.cmp(&par) {
        std::cmp::Ordering::Equal => tr("spz.type.par", &[]),
        std::cmp::Ordering::Less => tr("spz.par.under", &[("n", &(par - l).to_string())]),
        std::cmp::Ordering::Greater => tr("spz.par.over", &[("n", &(l - par).to_string())]),
    }
}
