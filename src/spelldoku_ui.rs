//! CC-SPELLDOKU v1 — the screen (app only, D9).
//!
//! The rules live in `crate::spelldoku`; this file only draws them and routes
//! taps. It never calls the leaderboard or any score service (D7, I9), never
//! touches shields (D6), and reads the keyboard's letters as data through
//! `keyboard::unit_rows`, the way Word Chains does (D10).

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use crate::spelldoku::tier::{self as tiermode};
use crate::spelldoku::canon;
use crate::spelldoku::geo::{self, size_of, Geo};
use crate::spelldoku::rules;
use crate::spelldoku::copy::{self, Kind};
use crate::spelldoku::bind::{Board, Mode};
use crate::spelldoku::gen::{Clue, Config, Tier};
use crate::spelldoku::play::{self, Unlocks, Verdict};
use crate::spelldoku::solve::{next_step, Tech};
use crate::spelldoku::table::{self, Build};
use crate::spelldoku::wordmode::{self, Personal};
use crate::{dom, i18n, i18n::t, App};

const SEEN_KEY: &str = "spell_spelldoku_seen_v1";
const STATS_KEY: &str = "spell_spelldoku_stats_v1";
const DAY_MS: f64 = 86_400_000.0;

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
    /// The app, for D13's missed-words path (wired once).
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Test builds only: play the English table as a preview build would.
    #[cfg(feature = "testseam")]
    static PREVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(feature = "testseam")]
pub fn seam_preview(on: bool) {
    PREVIEW.with(|c| c.set(on));
}

/// F9: `off | numbersIndexed | tierSymbols`, a setting inside SpellDoku (D-T1),
/// remembered per device. Default off: SpellDoku is unchanged unless chosen.
const TIER_KEY: &str = "spell_sd_tier";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Reading {
    Off,
    Numbers,
    Symbols,
}

fn reading() -> Reading {
    match crate::storage::get_raw(TIER_KEY).as_deref() {
        Some("numbersIndexed") => Reading::Numbers,
        Some("tierSymbols") => Reading::Symbols,
        _ => Reading::Off,
    }
}

fn build() -> Build {
    #[cfg(feature = "testseam")]
    if PREVIEW.with(|c| c.get()) {
        return Build::Preview;
    }
    table::current_build()
}

struct Game {
    board: Board,
    /// What is in each cell now: givens, and whatever the player committed.
    entries: Vec<u8>,
    pencil: Vec<u16>,
    sel: Option<usize>,
    typed: String,
    unlocks: Unlocks,
    checks_left: u32,
    hint_cell: Option<usize>,
    hint_level: u8,
    pencil_mode: bool,
    wrong: Vec<bool>,
    daily: bool,
    solved: bool,
    audio_ok: bool,
    /// CC-SPELLDOKU v1.3: the tier ladder for THIS board, when Tier Mode is on.
    tier: Option<tiermode::Session>,
    /// The word drawn for the digit the player is committing, if any (F2).
    prompt: Option<tiermode::Draw>,
    prompt_digit: u8,
    /// CC-SPELLDOKU-RULES C2 (Eric, 2026-09-22): the player picks the symbol
    /// FIRST and then spells it, so a rule conflict is refused before anyone is
    /// asked to spell anything (I-R2). This is the symbol being spelled toward.
    picked: Option<u8>,
    /// F1 feedback, cleared on a timer: the cell that refused a placement, and
    /// the already-placed cells that explain why.
    shake: Option<usize>,
    flash: Vec<usize>,
}

/// I4: the three verdicts are counted apart, on this device only.
#[derive(Serialize, Deserialize, Default)]
struct Stats {
    solved: u32,
    correct: u32,
    misspelled: u32,
    wrong_value: u32,
    wrong_system: u32,
}

#[derive(Serialize, Deserialize, Default)]
struct Seen {
    /// (canonical hash, day number) of every board served (I6).
    boards: Vec<(u64, u32)>,
}

/// The player's shared seen-word ledger (CC-WORDGRID D9), as Word Search and
/// Spell Cross store it.
fn window() -> crate::wordsearch::ledger::Ledger {
    crate::storage::get_json(crate::wordsearch_ui::LEDGER_KEY).unwrap_or_default()
}

fn today() -> u32 {
    (js_sys::Date::now() / DAY_MS) as u32
}

fn ymd() -> u32 {
    let d = js_sys::Date::new_0();
    d.get_full_year() * 10_000 + (d.get_month() + 1) * 100 + d.get_date()
}

/// D3/I8: does this language play at all in this build?
pub fn playable(lang: &str) -> bool {
    table::load(lang).is_some_and(|t| t.servable_for(4, build()))
}

fn configs(kid: bool, lang: &str) -> Vec<(usize, Tier)> {
    let all = [
        (4, Tier::Easy),
        (6, Tier::Easy),
        (6, Tier::Medium),
        (9, Tier::Easy),
        (9, Tier::Medium),
        (9, Tier::Hard),
        (9, Tier::Expert),
    ];
    let frag = table::load(lang).is_some_and(|t| crate::spelldoku::bind::number_symbols(&t, 9).0.fragments_possible());
    all.into_iter()
        .filter(|&(n, tier)| play::allowed(kid, n, tier))
        // Hard and Expert need a necessary fragment. Their boards are Word
        // Mode (D12), where bank words make fragments; a language whose number
        // words cannot make one still falls back no further than Medium.
        .filter(|&(n, tier)| frag || matches!(tier, Tier::Easy | Tier::Medium) || wordmode::is_word_mode(kid, n, tier))
        .collect()
}

fn tier_label(tier: Tier) -> String {
    t(&format!("level.{}", tier.id()))
}

fn record_stat(f: impl FnOnce(&mut Stats)) {
    let mut s: Stats = crate::storage::get_json(STATS_KEY).unwrap_or_default();
    f(&mut s);
    crate::storage::set_json(STATS_KEY, &s);
}

/// v1.3 I-T4: fold the OUTGOING board's served words into the player's D9
/// repeat window — the same ledger Word Search and Spell Cross keep, so a word
/// served in any of them stays out of the others for 20 puzzles or 14 days.
/// Called when a board ends: solved, or replaced by the next one.
fn close_tier_session() {
    let led = GAME.with(|c| c.borrow_mut().as_mut().and_then(|g| g.tier.as_mut()?.flush()));
    if let Some(led) = led {
        crate::storage::set_json(crate::wordsearch_ui::LEDGER_KEY, &led);
    }
}

/// Serve a board for `(n, tier)`. A Daily board is the same for everyone and is
/// only recorded in the ledger; any other board skips what this player has seen
/// within the window (I6).
fn serve(app: &App, n: usize, tier: Tier, daily: bool) -> bool {
    close_tier_session(); // the board being replaced enters the window (I-T4)
    let lang = app.borrow().lang.clone();
    let kid = app.borrow().kid;
    if !configs(kid, &lang).contains(&(n, tier)) {
        return false; // F7, and Hard/Expert only where fragments exist
    }
    let Some(tbl) = table::load(&lang).filter(|t| t.servable_for(n, build())) else { return false };
    let Some(size) = size_of(n) else { return false };
    let cfg = Config { size, tier };
    let mut seen: Seen = crate::storage::get_json(SEEN_KEY).unwrap_or_default();
    let now = today();
    seen.boards.retain(|(_, d)| now.saturating_sub(*d) < play::REPEAT_WINDOW_DAYS);
    let base = if daily {
        play::daily_seed(ymd(), &lang)
    } else {
        (js_sys::Date::now() as u64) ^ ((js_sys::Math::random() * 4_294_967_296.0) as u64) << 20
    };
    // D12: 9x9 Medium and up are Word Mode; the Daily draws from the bank
    // alone, so it is the same for everyone (F8). F13: a set that cannot be
    // drawn serves Number Mode instead, never a partial board.
    let personal = if daily { Personal::default() } else { personal(app, &lang) };
    let mut board = None;
    for k in 0..64u64 {
        let seed = base.wrapping_add(k);
        let Some(b) = wordmode::board_or_number(&lang, kid, &cfg, seed, &personal, Some(&tbl)) else { continue };
        let h = canon::hash(&b.puzzle);
        if daily || !seen.boards.iter().any(|(s, _)| *s == h) {
            seen.boards.push((h, now));
            board = Some(b);
            break;
        }
    }
    let Some(board) = board else { return false };
    let puzzle = &board.puzzle;
    crate::storage::set_json(SEEN_KEY, &seen);
    let cells = puzzle.clues.len();
    let entries: Vec<u8> = puzzle
        .clues
        .iter()
        .map(|c| match c {
            Clue::Given(v) | Clue::Spelled(v) => *v,
            _ => 0,
        })
        .collect();
    // v1.3 F9/D-T4: Tier Mode indexes DIGITS, so it rides Number Mode only, and
    // only where F5's depth rule holds. Anything else serves the board as before.
    let want = reading();
    let session = match (want, &board.mode) {
        (Reading::Off, _) | (_, Mode::Words(_)) => None,
        // F6: Reading B IS a 4x4 board of four tier badges. Asked for on any
        // other size, or on a Jr board (D-T5, enforced in Session::new), the
        // setting degrades to Reading A rather than refusing to deal.
        (Reading::Symbols, _) if n == 4 && !kid && tiermode::symbols_available(&lang) => {
            Some(tiermode::Session::with_ledger(&lang, tier, kid, true, base, window(), today()))
        }
        (Reading::Numbers, _) | (Reading::Symbols, _) if tiermode::available(&lang, tier) => {
            Some(tiermode::Session::with_ledger(&lang, tier, kid, false, base, window(), today()))
        }
        _ => None,
    };
    GAME.with(|g| {
        *g.borrow_mut() = Some(Game {
            unlocks: Unlocks::new(tier),
            tier: session,
            prompt: None,
            prompt_digit: 0,
            picked: None,
            shake: None,
            flash: Vec::new(),
            board,
            entries,
            pencil: vec![0; cells],
            sel: None,
            typed: String::new(),
            checks_left: play::CHECK_BOARD_USES,
            hint_cell: None,
            hint_level: 0,
            pencil_mode: false,
            wrong: vec![false; cells],
            daily,
            solved: false,
            audio_ok: true,
        })
    });
    dom::set_text("sdNote", "");
    render();
    true
}

/// F11: the player's own words in this language -- My Words, then the
/// missed-words queue with its band. Read only (the spec's non-goals).
fn personal(app: &App, lang: &str) -> Personal {
    let lists = crate::word_lists::load();
    let mine = crate::word_lists::visible(&lists)
        .iter()
        .flat_map(|l| l.entries.iter())
        .filter(|e| e.lang == lang)
        .map(|e| e.text.clone())
        .collect();
    let missed = app.borrow().misses.iter().filter(|m| m.lang == lang).map(|m| (m.word.clone(), m.tier.clone())).collect();
    Personal { mine, missed }
}

/// D15: the definition card for a symbol, masked by the server and filtered
/// here so it cannot spell the word (I12, the same filter Spell Search uses).
/// A definition that fails is not rewritten: that symbol simply has no card.
fn show_definition(v: u8) {
    let Some((word, lang)) = GAME.with(|c| {
        let gb = c.borrow();
        let g = gb.as_ref()?;
        Some((g.board.spelling(v)?, g.board.lang.clone()))
    }) else {
        return;
    };
    dom::set_text("sdBadge", &t("ws.meaning"));
    wasm_bindgen_futures::spawn_local(async move {
        let text = match crate::api::fetch_meaning(&word, true, &lang).await {
            Ok((_, def, _)) if !def.is_empty() && crate::wordsearch::hint::passes(&word, &def) => def,
            _ => t("ws.noMeaning"),
        };
        dom::set_text("sdBadge", &text);
    });
}

/// What a cell shows for value `v`: the board's own label, or in Reading B the
/// tier badge that IS the symbol (F6).
fn cell_label(g: &Game, v: u8) -> String {
    match g.tier.as_ref().filter(|ts| ts.symbols()).and_then(|ts| ts.band_of(v)) {
        Some(band) => t(tiermode::label_key(band)).chars().next().unwrap_or('?').to_string(),
        None => g.board.label(v),
    }
}

/// F4: what THIS board is, so every string the player reads comes from the one
/// table keyed by it. Nothing else in this file chooses a composer key.
fn board_copy(g: &Game) -> copy::Copy {
    copy::copy(Kind::of(g.board.is_words(), g.tier.is_some()))
}

/// F3 applies on the gentler tiers only (D-R2). Spell Jr plays Easy and Medium
/// spans, so it is covered without naming it (D-R7).
fn dimming(g: &Game) -> bool {
    matches!(g.board.puzzle.tier, Tier::Easy | Tier::Medium)
}

/// One pickable symbol: its glyph, how many are still to place (F2), and
/// whether the visible board already rules it out here (F3). Tapping it is now
/// the only way a symbol reaches a cell (C2), so this button carries the state
/// that used to be spread between the legend and the unlock tray.
fn symbol_button(g: &Game, v: u8, base: &str, aria: &str, ruled: u16) -> String {
    let left = rules::remaining(g.board.puzzle.n, &g.entries, v);
    let mut cls = vec![base];
    if left == 0 {
        cls.push("spent");
    }
    if g.picked == Some(v) {
        cls.push("picked");
    }
    if g.unlocks.chip(v) {
        cls.push("free"); // D1: earned -- it goes in without spelling
    }
    // A dimmed chip still responds; the tap runs F1 and is refused, so dimming
    // never stands in for validation.
    if ruled & geo::bit(v) != 0 {
        cls.push("dim");
    }
    format!(
        "<button type=\"button\" class=\"{}\" data-sd-sym=\"{v}\"{}><b>{}</b><i class=\"sd-n\">{left}</i>{}</button>",
        cls.join(" "),
        if aria.is_empty() { String::new() } else { format!(" aria-label=\"{}\"", dom::escape_html(aria)) },
        dom::escape_html(&cell_label(g, v)),
        if g.picked == Some(v) { " \u{25B6}" } else { "" },
    )
}

/// F1's refusal: nothing is written and nothing is consumed (I-R3). The cell
/// shakes, the cells that explain it flash, a warning haptic plays, and the
/// message names the first unit in row -> column -> box order.
fn refuse(cell: Option<usize>, cells: Vec<usize>, msg: String) {
    let kid = APP.with(|a| a.borrow().as_ref().is_some_and(|app| app.borrow().kid));
    GAME.with(|c| {
        if let Some(g) = c.borrow_mut().as_mut() {
            g.shake = cell;
            g.flash = cells;
        }
    });
    dom::set_text("sdNote", &msg);
    crate::haptics::incorrect(kid);
    render();
    // 600 ms per F1, then the board goes quiet again. The message stays a
    // little longer so it can be read.
    dom::after_ms(600, || {
        GAME.with(|c| {
            if let Some(g) = c.borrow_mut().as_mut() {
                g.shake = None;
                g.flash.clear();
            }
        });
        render();
    });
}

/// C2: the player picks the symbol, and THAT is where a rule conflict is
/// caught -- before anyone is asked to spell a word (I-R2). An earned symbol
/// (D1) goes straight in; any other opens the spelling.
fn pick_symbol(v: u8) {
    enum Next {
        Nothing,
        Say,
        Speak,
        Commit(u8, Verdict),
        Refuse(Option<usize>, Vec<usize>, String),
    }
    let next = GAME.with(|cell| {
        let mut gb = cell.borrow_mut();
        let Some(g) = gb.as_mut() else { return Next::Nothing };
        if g.solved || g.pencil_mode {
            return Next::Nothing;
        }
        // No cell chosen yet: the row is still the key to the board, so a tap
        // just says the word.
        let Some(i) = g.sel else { return Next::Say };
        if g.board.puzzle.clues[i] != Clue::Empty && g.entries[i] > 0 {
            return Next::Nothing; // a given: not the player's to fill
        }
        // F2 first: "they are all placed" is a truer thing to say than "it is
        // already in this row", when both are true.
        if rules::remaining(g.board.puzzle.n, &g.entries, v) == 0 {
            let msg = i18n::tp("sd.allPlaced", &[("n", &g.board.puzzle.n.to_string()), ("sym", &cell_label(g, v))]);
            return Next::Refuse(Some(i), Vec::new(), msg);
        }
        let geo = Geo::new(size_of(g.board.puzzle.n).expect("size"));
        if let Err(c) = rules::validate(&geo, &g.entries, i, v) {
            let msg = i18n::tp(c.unit.key(), &[("sym", &cell_label(g, v))]);
            return Next::Refuse(Some(i), c.cells, msg);
        }
        if g.unlocks.chip(v) {
            let expected = g.board.puzzle.solution[i];
            return Next::Commit(v, if v == expected { Verdict::Correct } else { Verdict::WrongValue });
        }
        g.picked = Some(v);
        g.typed.clear();
        Next::Speak
    });
    match next {
        Next::Nothing => {}
        Next::Refuse(cell, cells, msg) => refuse(cell, cells, msg),
        Next::Commit(v, verdict) => commit_value(v, false, verdict),
        Next::Say | Next::Speak => {
            let board = GAME.with(|c| c.borrow().as_ref().map(|g| g.board.clone()));
            if let Some(b) = board {
                speak(&b, v, || {});
            }
            render();
        }
    }
}

/// Play a symbol's word through the one audio resolver. Chinese is spoken from
/// its characters with the reading forced (CC-ZH-TONE F6).
fn speak(board: &Board, v: u8, on_fail: impl FnOnce() + 'static) {
    let Some(spelling) = board.spelling(v) else { return };
    match &board.mode {
        Mode::Words(w) if board.lang == "zh" => {
            let hanzi = w.get(v as usize - 1).and_then(|s| s.display.clone()).unwrap_or_default();
            crate::api::play_word_with(&hanzi, Some(&spelling), "normal", 1.0, &board.lang, on_fail);
        }
        _ => crate::api::play_word(&spelling, "normal", 1.0, &board.lang, on_fail),
    }
}

/// D10: the keys come from the in-app keyboard's own layout file -- the same
/// SSOT the base keyboard is tested against -- so SpellDoku never invents an
/// alphabet. Rows as laid out, then every accented alternate (the base keyboard
/// offers them on long-press, which this tray has no room for).
fn layout(lang: &str) -> (Vec<Vec<String>>, Vec<String>) {
    let raw = match lang {
        "en" => include_str!("../assets/keyboards/en.json"),
        "es" => include_str!("../assets/keyboards/es.json"),
        "fr" => include_str!("../assets/keyboards/fr.json"),
        "de" => include_str!("../assets/keyboards/de.json"),
        "pt" => include_str!("../assets/keyboards/pt.json"),
        "pl" => include_str!("../assets/keyboards/pl.json"),
        "ru" => include_str!("../assets/keyboards/ru.json"),
        "vi" => include_str!("../assets/keyboards/vi.json"),
        "ko" => include_str!("../assets/keyboards/ko.json"),
        "ja" => include_str!("../assets/keyboards/ja.json"),
        "zh" => include_str!("../assets/keyboards/zh.json"),
        "fil" => include_str!("../assets/keyboards/fil.json"),
        "sw" => include_str!("../assets/keyboards/sw.json"),
        "ar" => include_str!("../assets/keyboards/ar.json"),
        "hi" => include_str!("../assets/keyboards/hi.json"),
        _ => include_str!("../assets/keyboards/en.json"),
    };
    let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    let rows: Vec<Vec<String>> = v["rows"]
        .as_array()
        .map(|a| a.iter().filter_map(|r| r.as_str()).map(|r| r.chars().map(|c| c.to_string()).collect()).collect())
        .unwrap_or_default();
    let mut extras: Vec<String> = Vec::new();
    if let Some(lp) = v["longPress"].as_object() {
        for alts in lp.values() {
            for c in alts.as_str().unwrap_or("").chars() {
                let k = c.to_string();
                if !extras.contains(&k) {
                    extras.push(k);
                }
            }
        }
    }
    (rows, extras)
}

/// The Vietnamese tone row: the five marks, entered after the syllable.
const VI_TONE_KEYS: [char; 5] = ['\u{0300}', '\u{0301}', '\u{0309}', '\u{0303}', '\u{0323}'];

/// Add one key to what has been typed. Korean composes jamo into syllables the
/// way the base keyboard does; every other script simply appends.
fn type_key(lang: &str, typed: &str, key: &str) -> String {
    if lang == "ko" {
        if let Some(j) = key.chars().next() {
            return crate::hangul::feed(typed, j);
        }
    }
    let mut out = typed.to_string();
    out.push_str(key);
    out
}

fn backspace(lang: &str, typed: &str) -> String {
    if lang == "ko" {
        return crate::hangul::backspace(typed);
    }
    let mut out = typed.to_string();
    out.pop();
    out
}

fn is_clue(g: &Game, i: usize) -> bool {
    matches!(g.board.puzzle.clues[i], Clue::Given(_) | Clue::Spelled(_))
}

/// C5, closed by Eric on 2026-09-21 from a screenshot of a letters board whose
/// chip read "Numbers": the chip names the SYMBOLS the player is looking at, so
/// cycling boards says which kind each one is. It is the Tier Mode picker's
/// "off" entry, which is why it was hardcoded to the one reading -- and why,
/// with Tier Mode ON, it names the ordinary board for this size and tier
/// instead of the ladder board on screen, since that is what switching off
/// would serve.
fn mode_chip(g: &Game) {
    let letters = match &g.tier {
        None => matches!(g.board.mode, Mode::Words(_)),
        Some(_) => {
            let kid = APP.with(|a| a.borrow().as_ref().is_some_and(|app| app.borrow().kid));
            wordmode::is_word_mode(kid, g.board.puzzle.n, g.board.puzzle.tier)
        }
    };
    dom::set_text("sdTierOff", &t(if letters { "sd.tier.letters" } else { "sd.tier.off" }));
}

fn render() {
    GAME.with(|cell| {
        let gb = cell.borrow();
        let Some(g) = gb.as_ref() else { return };
        let n = g.board.puzzle.n;
        let size = size_of(n).expect("size");
        mode_chip(g);
        let mut html = String::new();
        for i in 0..n * n {
            let (r, c) = (i / n, i % n);
            let mut cls = vec!["sd-cell"];
            // F1: the refused cell shakes, the cells that explain the refusal flash.
            if g.shake == Some(i) {
                cls.push("shake");
            }
            if g.flash.contains(&i) {
                cls.push("flash");
            }
            if (c + 1) % size.bc == 0 && c + 1 < n {
                cls.push("bx-r");
            }
            if (r + 1) % size.br == 0 && r + 1 < n {
                cls.push("bx-b");
            }
            if g.sel == Some(i) {
                cls.push("sel");
            }
            if g.hint_cell == Some(i) {
                cls.push("hint");
            }
            if g.wrong[i] {
                cls.push("wrong");
            }
            let b = &g.board;
            let body = match &b.puzzle.clues[i] {
                Clue::Given(_) => {
                    cls.push("given");
                    dom::escape_html(&b.clue_text(i))
                }
                Clue::Spelled(_) if !b.is_words() => {
                    cls.push("given word");
                    format!("<span dir=\"auto\">{}</span>", dom::escape_html(&b.clue_text(i)))
                }
                Clue::Spelled(_) => {
                    cls.push("given");
                    dom::escape_html(&b.clue_text(i))
                }
                Clue::Fragment(p) => {
                    cls.push("frag");
                    let pat: String =
                        b.fragment_text(p).into_iter().map(|c| c.unwrap_or_else(|| "_".into())).collect::<Vec<_>>().join(" ");
                    if g.entries[i] > 0 {
                        format!("<b>{}</b><small dir=\"auto\">{}</small>", dom::escape_html(&b.label(g.entries[i])), dom::escape_html(&pat))
                    } else {
                        format!("<small dir=\"auto\">{}</small>", dom::escape_html(&pat))
                    }
                }
                Clue::Empty => {
                    if g.entries[i] > 0 {
                        cls.push("mine");
                        dom::escape_html(&b.label(g.entries[i]))
                    } else if g.pencil[i] != 0 {
                        let marks: String = (1..=n as u8)
                            .filter(|v| g.pencil[i] & (1 << v) != 0)
                            .map(|v| dom::escape_html(&b.label(v)))
                            .collect::<Vec<_>>()
                            .join(" ");
                        format!("<small class=\"sd-pen\">{marks}</small>")
                    } else {
                        String::new()
                    }
                }
            };
            html.push_str(&format!(
                "<button type=\"button\" class=\"{}\" data-sd-cell=\"{i}\">{body}</button>",
                cls.join(" ")
            ));
        }
        dom::set_html("sdGrid", &html);
        let _ = dom::el("sdGrid").set_attribute("style", &format!("--sd-n:{n}"));

        // F3: the ladder is legible from board load — every digit carries its
        // tier, before any cell is attempted (I-T3). Earned digits say so.
        let mut strip = String::new();
        if let Some(ts) = g.tier.as_ref() {
            for v in 1..=n as u8 {
                let Some(band) = ts.band_of(v) else { continue };
                let earned = g.sel.is_some_and(|i| ts.satisfied(i, v));
                strip.push_str(&format!(
                    "<button type=\"button\" class=\"sd-tierkey band-{band}{}\" data-sd-tier=\"{v}\"><b>{}</b><small>{}</small></button>{}",
                    if earned { " earned" } else { "" },
                    dom::escape_html(&cell_label(g, v)),
                    dom::escape_html(&t(tiermode::label_key(band))),
                    // C7 (Eric, 2026-09-22): the digit being spelled carries a
                    // replay. Tapping the digit again would DRAW ANOTHER WORD,
                    // so without this a player who missed the audio could never
                    // hear that word again.
                    if g.prompt.is_some() && g.prompt_digit == v {
                        format!(
                            "<button type=\"button\" class=\"sd-replay\" data-sd-replay aria-label=\"{}\">\u{25B6}</button>",
                            dom::escape_html(&t("sd.sayAria"))
                        )
                    } else {
                        String::new()
                    }
                ));
            }
        }
        dom::set_html("sdTiers", &strip);

        // CC-SPELLDOKU-RULES: ONE row of symbols the player picks from, in every
        // mode. It carries F2's count, F3's dimming and D1's unlock state, so a
        // symbol that cannot legally go anywhere reads as unavailable BEFORE the
        // player spells anything. Word Mode draws its own richer row into the
        // legend below (it owns the audio and the definition card), so the two
        // never both render.
        let ruled = match (dimming(g), g.sel) {
            (true, Some(i)) => rules::ruled_out(&Geo::new(size), &g.entries, i),
            _ => 0,
        };
        let chips: String = if g.board.is_words() || g.tier.is_some() {
            // Word Mode picks from the legend; Tier Mode picks from the ladder
            // strip. Neither wants a second row of the same symbols.
            String::new()
        } else {
            (1..=n as u8).map(|v| symbol_button(g, v, "sd-chip", "", ruled)).collect()
        };
        dom::set_html("sdChips", &chips);
        let typed = if g.typed.is_empty() && !g.pencil_mode {
            let c = board_copy(g);
            let spelling = g.picked.is_some() || g.prompt.is_some();
            if g.sel.is_none() {
                t(c.pick_cell)
            } else if spelling {
                t(c.spell)
            } else {
                t(c.pick_symbol)
            }
        } else {
            g.typed.clone()
        };
        dom::set_text("sdTyped", &typed);
        let keys = if g.pencil_mode {
            // F1: pencil marks are digits and never open the keyboard.
            (1..=n as u8)
                .map(|v| format!("<button type=\"button\" class=\"kb-key\" data-sd-pen=\"{v}\">{}</button>", dom::escape_html(&cell_label(g, v))))
                .collect::<String>()
        } else {
            let mut k = String::new();
            let (rows, extras) = layout(&g.board.lang);
            let key = |c: &str| {
                format!(
                    "<button type=\"button\" class=\"kb-key\" data-sd-key=\"{0}\">{0}</button>",
                    dom::escape_html(c)
                )
            };
            // Every row this language types: its letters, then its extras, then
            // Vietnamese tone marks. Backspace and Lock It In ride along on the
            // LAST of them rather than claiming a row of their own -- a row is
            // ~52 px, which a 9x9 board on a small phone does not have to spare
            // (the geometry census measured a 263 px overrun). This is also how
            // a phone keyboard normally places its backspace.
            let mut lines: Vec<String> = rows
                .iter()
                .chain(std::iter::once(&extras))
                .filter(|r| !r.is_empty())
                .map(|row| row.iter().map(|c| key(c)).collect::<String>())
                .collect();
            if g.board.lang == "vi" {
                lines.push(
                    VI_TONE_KEYS
                        .iter()
                        .map(|t| {
                            format!(
                                "<button type=\"button\" class=\"kb-key\" data-sd-key=\"{0}\" aria-label=\"tone mark\">a{0}</button>",
                                t
                            )
                        })
                        .collect::<String>(),
                );
            }
            const COMMIT: &str = "<button type=\"button\" class=\"kb-key wide\" data-sd-back>\u{232B}</button>\
                                  <button type=\"button\" class=\"kb-key wide go\" data-sd-go>\u{2713}</button>";
            match lines.last_mut() {
                Some(last) => last.push_str(COMMIT),
                None => lines.push(COMMIT.to_string()),
            }
            for line in lines {
                k.push_str("<div class=\"sd-row\">");
                k.push_str(&line);
                k.push_str("</div>");
            }
            k
        };
        dom::set_html("sdKeys", &keys);
        let _ = dom::el("sdPencil").set_attribute("aria-pressed", if g.pencil_mode { "true" } else { "false" });
        // D2: Check Board exists on Hard and Expert, three times.
        let check = !play::logic_errors_shown_at_once(g.board.puzzle.tier);
        dom::toggle_class("sdCheck", "btn-hide", !check);
        dom::set_text("sdCheck", &i18n::tp("sd.check", &[("n", &g.checks_left.to_string())]));
        dom::set_disabled("sdCheck", g.checks_left == 0 || g.solved);
        dom::toggle_class("sdDaily", "on", g.daily);

        // F10: in Word Mode the legend names each symbol by its glyph and an
        // audio orb -- never its spelling (I12). D14: a quiet source badge.
        // D15: the audio orb everywhere, and a definition card where the
        // language's definition pool is live (consts::def_match) and the
        // server can serve a masked one. The card is filtered before it is
        // shown, so it never spells the symbol (I12).
        let cards = crate::consts::def_match(&g.board.lang) && crate::api::meaning_supported(&g.board.lang);
        let legend: String = if g.board.is_words() {
            (1..=n as u8)
                .map(|v| {
                    format!(
                        "<span class=\"sd-sym\">{}{}</span>",
                        symbol_button(g, v, "sd-say", &t("sd.sayAria"), ruled),
                        if cards {
                            format!(
                                "<button type=\"button\" class=\"sd-def\" data-sd-def=\"{v}\" aria-label=\"{}\">?</button>",
                                dom::escape_html(&t("ws.meaning"))
                            )
                        } else {
                            String::new()
                        }
                    )
                })
                .collect()
        } else {
            String::new()
        };
        dom::set_html("sdLegend", &legend);
        let mine = g.board.mine();
        dom::set_text("sdBadge", &if mine > 0 { i18n::tp("sd.badge", &[("n", &mine.to_string())]) } else { String::new() });
    });
}

fn note(key: &str) {
    dom::set_text("sdNote", &t(key));
}

/// A commit of value `v` into the selected cell, however it was entered.
fn commit_value(v: u8, spelled: bool, verdict: Verdict) {
    // F1 belt and braces: every path that reaches a cell passes here, so the
    // validator sits here too. pick_symbol already refused a conflict before
    // any spelling (I-R2); this makes I-R1 true of the write itself, whatever
    // new path a future change adds.
    let blocked = GAME.with(|cell| {
        let gb = cell.borrow();
        let g = gb.as_ref()?;
        let i = g.sel?;
        if !matches!(verdict, Verdict::Correct | Verdict::WrongValue) {
            return None;
        }
        let geo = Geo::new(size_of(g.board.puzzle.n).expect("size"));
        rules::validate(&geo, &g.entries, i, v)
            .err()
            .map(|c| (i, c.cells, i18n::tp(c.unit.key(), &[("sym", &cell_label(g, v))])))
    });
    if let Some((i, cells, msg)) = blocked {
        refuse(Some(i), cells, msg);
        return;
    }
    let mut solved_now = false;
    GAME.with(|cell| {
        let mut gb = cell.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        let Some(i) = g.sel else { return };
        match verdict {
            Verdict::Correct => {
                g.entries[i] = v;
                g.wrong[i] = false;
                g.pencil[i] = 0;
                if spelled {
                    g.unlocks.record_correct(i, v);
                }
                record_stat(|s| s.correct += 1);
                dom::set_text("sdNote", "");
            }
            Verdict::WrongValue => {
                record_stat(|s| s.wrong_value += 1);
                if play::logic_errors_shown_at_once(g.board.puzzle.tier) {
                    dom::set_text("sdNote", &t("sd.wrongValue"));
                } else {
                    // D2: on Hard and Expert a logic error waits for Check Board.
                    g.entries[i] = v;
                    g.pencil[i] = 0;
                    dom::set_text("sdNote", "");
                }
            }
            Verdict::Misspelled => {
                record_stat(|s| s.misspelled += 1);
                dom::set_text("sdNote", &t(board_copy(g).misspelled));
            }
            Verdict::WrongSystem => {
                record_stat(|s| s.wrong_system += 1);
            }
        }
        g.typed.clear();
        if matches!(verdict, Verdict::Correct | Verdict::WrongValue) {
            g.picked = None; // placed: the next cell starts from its own pick
        }
        g.hint_cell = None;
        g.hint_level = 0;
        if !g.solved && g.entries == g.board.puzzle.solution {
            g.solved = true;
            solved_now = true;
        }
    });
    if solved_now {
        record_stat(|s| s.solved += 1);
        note("sd.solved");
        close_tier_session(); // a finished board's words enter the window (I-T4)
    }
    render();
}

/// F2: the player picked a digit. If this cell already earned that digit (F8),
/// it goes straight in. Otherwise a word is drawn from the digit's tier and
/// spoken, and the player spells it.
fn tier_digit(v: u8) {
    enum Next {
        Nothing,
        Commit(u8, Verdict),
        Speak,
        Refuse(usize, Vec<usize>, String),
    }
    let next = GAME.with(|cell| {
        let mut gb = cell.borrow_mut();
        let Some(g) = gb.as_mut() else { return Next::Nothing };
        let Some(i) = g.sel else {
            return Next::Nothing; // the prompt line already says "pick a cell"
        };
        if g.pencil_mode || g.entries[i] != 0 && g.board.puzzle.clues[i] != Clue::Empty {
            return Next::Nothing;
        }
        // F1 reaches Tier Mode too: the ladder picks the digit first, so the
        // conflict is caught here, before a word is drawn and spoken (I-R2).
        if rules::remaining(g.board.puzzle.n, &g.entries, v) == 0 {
            let msg = i18n::tp("sd.allPlaced", &[("n", &g.board.puzzle.n.to_string()), ("sym", &cell_label(g, v))]);
            return Next::Refuse(i, Vec::new(), msg);
        }
        let geo = Geo::new(size_of(g.board.puzzle.n).expect("size"));
        if let Err(c) = rules::validate(&geo, &g.entries, i, v) {
            let msg = i18n::tp(c.unit.key(), &[("sym", &cell_label(g, v))]);
            return Next::Refuse(i, c.cells, msg);
        }
        let Some(ts) = g.tier.as_mut() else { return Next::Nothing };
        if ts.satisfied(i, v) {
            let expected = g.board.puzzle.solution[i];
            let verdict = if v == expected { Verdict::Correct } else { Verdict::WrongValue };
            return Next::Commit(v, verdict);
        }
        match ts.draw(v) {
            Some(d) => {
                g.prompt = Some(d);
                g.prompt_digit = v;
                g.typed.clear();
                Next::Speak
            }
            None => Next::Nothing, // the band ran dry: the digit stays available
        }
    });
    match next {
        Next::Commit(v, verdict) => commit_value(v, false, verdict),
        Next::Refuse(cell, cells, msg) => refuse(Some(cell), cells, msg),
        Next::Speak => {
            speak_prompt();
            render();
        }
        Next::Nothing => render(),
    }
}

/// The drawn word, through the one audio resolver (I1). zh speaks its characters
/// with the reading forced, as everywhere else.
fn speak_prompt() {
    let said = GAME.with(|cell| {
        let gb = cell.borrow();
        let g = gb.as_ref()?;
        let d = g.prompt.as_ref()?;
        Some((d.spelling.clone(), d.display.clone(), g.board.lang.clone()))
    });
    if let Some((spelling, display, lang)) = said {
        match display {
            Some(hanzi) if lang == "zh" => {
                crate::api::play_word_with(&hanzi, Some(&spelling), "normal", 1.0, &lang, || {})
            }
            _ => crate::api::play_word(&spelling, "normal", 1.0, &lang, || {}),
        }
    }
}

fn submit_typed() {
    // v1.3 F2: in Tier Mode the typed word is judged against the word THIS cell
    // was served, and the digit is the one whose badge the player tapped. A
    // misspelling draws a new word of the same tier and never fills the cell (F4).
    let tier_turn = GAME.with(|cell| {
        let gb = cell.borrow();
        let g = gb.as_ref()?;
        let d = g.prompt.as_ref()?;
        let i = g.sel?;
        if g.typed.trim().is_empty() {
            return None;
        }
        let spelled_right = crate::spelldoku::table::norm(&g.typed) == d.spelling;
        Some((i, g.prompt_digit, spelled_right, d.spelling.clone(), d.band, g.board.lang.clone(), g.board.puzzle.solution[i]))
    });
    if let Some((i, v, spelled_right, word, band, lang, expected)) = tier_turn {
        if spelled_right {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    if let Some(ts) = g.tier.as_mut() {
                        ts.record(i, v);
                    }
                    g.prompt = None;
                }
            });
            let verdict = if v == expected { Verdict::Correct } else { Verdict::WrongValue };
            commit_value(v, true, verdict);
        } else {
            // D13: the miss joins the existing missed-words queue, as in Word Mode.
            APP.with(|a| {
                if let Some(app) = a.borrow().as_ref() {
                    crate::misses::add_miss(&mut app.borrow_mut(), &word, &lang, band);
                }
            });
            record_stat(|s| s.misspelled += 1);
            note(copy::copy(Kind::Tier).misspelled);
            let redrawn = GAME.with(|c| {
                let mut gb = c.borrow_mut();
                let Some(g) = gb.as_mut() else { return false };
                g.typed.clear();
                let v = g.prompt_digit;
                match g.tier.as_mut().and_then(|ts| ts.draw(v)) {
                    Some(d) => {
                        g.prompt = Some(d);
                        true
                    }
                    None => {
                        g.prompt = None;
                        false
                    }
                }
            });
            if redrawn {
                speak_prompt();
            }
            render();
        }
        return;
    }
    // F2: with a ladder on the board, the number word for a digit is no longer
    // the price of that digit. Typing one commits nothing -- the player taps the
    // digit's badge, hears its word, and spells that.
    if GAME.with(|c| c.borrow().as_ref().is_some_and(|g| g.tier.is_some())) {
        GAME.with(|c| {
            if let Some(g) = c.borrow_mut().as_mut() {
                g.typed.clear();
            }
        });
        note(copy::copy(Kind::Tier).pick_symbol);
        render();
        return;
    }
    // C2: the symbol was chosen before the spelling began, so the typed word is
    // judged against THAT symbol's word. A misspelling never places anything
    // and never changes which symbol is on the hook.
    let res = GAME.with(|cell| {
        let gb = cell.borrow();
        let g = gb.as_ref()?;
        let i = g.sel?;
        let v = g.picked?;
        if g.typed.trim().is_empty() {
            return None;
        }
        // The one matcher, so Mandarin's tone rule and Vietnamese's tone row
        // keep working: does what was typed name the symbol that was picked?
        let right = g.board.values_for(&g.typed).contains(&v);
        let expected = g.board.puzzle.solution[i];
        let verdict = if !right {
            Verdict::Misspelled
        } else if v == expected {
            Verdict::Correct
        } else {
            Verdict::WrongValue
        };
        // D13: a Word Mode misspelling joins the existing missed-words queue,
        // through its existing path, with the band the word came from.
        let missed = match (&g.board.mode, right) {
            (Mode::Words(w), false) => w.get((v as usize).checked_sub(1)?).map(|s| (s.spelling.clone(), s.band, g.board.lang.clone())),
            _ => None,
        };
        Some((v, verdict, missed))
    });
    if let Some((v, verdict, missed)) = res {
        if let Some((word, band, lang)) = missed {
            APP.with(|a| {
                if let Some(app) = a.borrow().as_ref() {
                    crate::misses::add_miss(&mut app.borrow_mut(), &word, &lang, band);
                }
            });
        }
        commit_value(v, true, verdict);
    }
}

fn hint() {
    GAME.with(|cell| {
        let mut gb = cell.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        if g.solved {
            return;
        }
        let geo = Geo::new(size_of(g.board.puzzle.n).expect("size"));
        let (mut fixed, restrict) = g.board.puzzle.constraints(&g.board.symbols());
        // Only the player's RIGHT entries count as known: a wrong one would
        // mislead the ladder, and using them reveals nothing on screen.
        for i in 0..fixed.len() {
            if fixed[i] == 0 && g.entries[i] == g.board.puzzle.solution[i] {
                fixed[i] = g.entries[i];
            }
        }
        let Some((cell_i, tech)) = next_step(&geo, &fixed, &restrict) else {
            // F1: the ladder cannot move because something already placed is
            // wrong. Say which kind of thing, and point at the one tool that
            // can find it.
            dom::set_text("sdNote", &t(board_copy(g).hint_blocked));
            return;
        };
        g.hint_level = if g.hint_cell == Some(cell_i) { g.hint_level + 1 } else { 1 };
        g.hint_cell = Some(cell_i);
        g.sel = Some(cell_i);
        match g.hint_level {
            1 => dom::set_text("sdNote", &t("sd.hintCell")),
            2 => {
                let name = match tech {
                    Tech::Single => t("sd.tech.single"),
                    Tech::Subset => t("sd.tech.subset"),
                    Tech::Intersection => t("sd.tech.intersection"),
                    Tech::Fish => t("sd.tech.fish"),
                };
                let line = i18n::tp("sd.hintTech", &[("tech", &name)]);
                // F9 / I12: a technique name that happens to spell one of this
                // board's words would give it away; point at the cell instead.
                let spells = (1..=g.board.puzzle.n as u8)
                    .filter_map(|v| g.board.spelling(v))
                    .any(|w| table::norm(&line).contains(&table::norm(&w)));
                dom::set_text("sdNote", &if spells { t("sd.hintCell") } else { line });
            }
            _ if g.audio_ok => {
                // F9 level 3: hear it through the one resolver; still spell it.
                dom::set_text("sdNote", &t("sd.hintAudio"));
                speak(&g.board, g.board.puzzle.solution[cell_i], || {
                    GAME.with(|c| {
                        if let Some(g) = c.borrow_mut().as_mut() {
                            g.audio_ok = false; // level 3 is not offered again
                        }
                    });
                    dom::set_text("sdNote", &t("sd.audioOff"));
                });
            }
            _ => {}
        }
    });
    render();
}

fn check_board() {
    GAME.with(|cell| {
        let mut gb = cell.borrow_mut();
        let Some(g) = gb.as_mut() else { return };
        if g.checks_left == 0 {
            return;
        }
        g.checks_left -= 1;
        let mut wrong = 0;
        for i in 0..g.entries.len() {
            let bad = !is_clue(g, i) && g.entries[i] > 0 && g.entries[i] != g.board.puzzle.solution[i];
            g.wrong[i] = bad;
            if bad {
                wrong += 1;
            }
        }
        if wrong == 0 {
            dom::set_text("sdNote", &t("sd.checkRight"));
        } else {
            dom::set_text("sdNote", &i18n::tp("sd.checkWrong", &[("n", &wrong.to_string())]));
        }
    });
    render();
}

fn fill_picker(kid: bool, lang: &str) {
    let opts: String = configs(kid, lang)
        .into_iter()
        .map(|(n, tier)| {
            format!(
                "<option value=\"{n}-{}\">{n}\u{00D7}{n} \u{00B7} {}</option>",
                tier.id(),
                dom::escape_html(&tier_label(tier))
            )
        })
        .collect();
    dom::set_html("sdPick", &opts);
    dom::select("sdTier").set_value(match reading() {
        Reading::Numbers => "numbersIndexed",
        Reading::Symbols => "tierSymbols",
        Reading::Off => "off",
    });
}

fn picked() -> Option<(usize, Tier)> {
    let v = dom::select("sdPick").value();
    let (n, tier) = v.split_once('-')?;
    Some((n.parse().ok()?, Tier::parse(tier)?))
}

pub fn open(app: &App) {
    let kid = app.borrow().kid;
    let lang = app.borrow().lang.clone();
    fill_picker(kid, &lang);
    if let Some((n, tier)) = configs(kid, &lang).first().copied() {
        dom::select("sdPick").set_value(&format!("{n}-{}", tier.id()));
        if !serve(app, n, tier, false) {
            return;
        }
    }
    dom::add_class("sdScreen", "show");
}

fn close() {
    dom::remove_class("sdScreen", "show");
}

pub fn wire(app: &App) {
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));
    {
        let a = app.clone();
        dom::on_click("sdOpenBtn", move || open(&a));
    }
    dom::on_click("sdExit", close);
    {
        let a = app.clone();
        dom::on::<web_sys::Event, _>("sdPick", "change", move |_| {
            if let Some((n, tier)) = picked() {
                serve(&a, n, tier, false);
            }
        });
    }
    {
        // F9: switching reading starts a fresh board — the symbols change.
        let a = app.clone();
        dom::on::<web_sys::Event, _>("sdTier", "change", move |_| {
            crate::storage::set_raw(TIER_KEY, &dom::select("sdTier").value());
            if let Some((n, tier)) = picked() {
                serve(&a, n, tier, false);
            }
        });
    }
    {
        let a = app.clone();
        dom::on_click("sdNew", move || {
            if let Some((n, tier)) = picked() {
                serve(&a, n, tier, false);
            }
        });
    }
    {
        let a = app.clone();
        dom::on_click("sdDaily", move || {
            let kid = a.borrow().kid;
            // The Daily: 9x9 Medium, or Spell Jr's 6x6 Medium. Unranked (D7).
            let (n, tier) = if kid { (6, Tier::Medium) } else { (9, Tier::Medium) };
            dom::select("sdPick").set_value(&format!("{n}-{}", tier.id()));
            serve(&a, n, tier, true);
        });
    }
    dom::on_click("sdPencil", || {
        GAME.with(|c| {
            if let Some(g) = c.borrow_mut().as_mut() {
                g.pencil_mode = !g.pencil_mode;
                g.typed.clear();
            }
        });
        render();
    });
    dom::on_click("sdHint", hint);
    dom::on_click("sdCheck", check_board);
    dom::on::<web_sys::Event, _>("sdScreen", "click", move |ev| {
        use wasm_bindgen::JsCast;
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(el) = target
            .closest("[data-sd-cell],[data-sd-key],[data-sd-back],[data-sd-go],[data-sd-sym],[data-sd-pen],[data-sd-def],[data-sd-tier],[data-sd-replay]")
            .ok()
            .flatten()
        else {
            return;
        };
        if let Some(v) = el.get_attribute("data-sd-def").and_then(|v| v.parse::<u8>().ok()) {
            show_definition(v);
        } else if let Some(i) = el.get_attribute("data-sd-cell").and_then(|v| v.parse::<usize>().ok()) {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    if !is_clue(g, i) && !g.solved {
                        g.sel = Some(i);
                        g.typed.clear();
                    }
                }
            });
            render();
        } else if let Some(k) = el.get_attribute("data-sd-key") {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    // Nothing to spell until a symbol is picked (C2). Tier
                    // Mode's own draw counts as the pick.
                    let armed = g.picked.is_some() || g.prompt.is_some();
                    if g.sel.is_some() && armed && g.typed.chars().count() < 24 {
                        g.typed = type_key(&g.board.lang, &g.typed, &k);
                    }
                }
            });
            render();
        } else if let Some(v) = el.get_attribute("data-sd-tier").and_then(|v| v.parse::<u8>().ok()) {
            tier_digit(v);
        } else if el.get_attribute("data-sd-back").is_some() {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    g.typed = backspace(&g.board.lang, &g.typed);
                }
            });
            render();
        } else if el.get_attribute("data-sd-go").is_some() {
            submit_typed();
        } else if let Some(v) = el.get_attribute("data-sd-sym").and_then(|v| v.parse::<u8>().ok()) {
            pick_symbol(v);
        } else if el.get_attribute("data-sd-replay").is_some() {
            speak_prompt(); // C7: hear the pending word again, without redrawing
        } else if let Some(v) = el.get_attribute("data-sd-pen").and_then(|v| v.parse::<u8>().ok()) {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    if let Some(i) = g.sel {
                        if g.entries[i] == 0 {
                            g.pencil[i] ^= 1 << v;
                        }
                    }
                }
            });
            render();
        }
    });
}

/// Test builds only: the golden digest as this build computes it (Done #3).
#[cfg(feature = "testseam")]
pub fn seam_golden() -> String {
    table::load("en").map(|t| format!("{:#x}", crate::spelldoku::bind::golden_digest(&t))).unwrap_or_default()
}

/// Test builds only: the served board, so a browser test knows the answers.
#[cfg(feature = "testseam")]
pub fn seam_board() -> String {
    GAME.with(|c| {
        c.borrow()
            .as_ref()
            .map(|g| {
                let mut v = serde_json::to_value(&g.board.puzzle).unwrap_or_default();
                // Word Mode: the words, so a browser test can spell them.
                if let Mode::Words(w) = &g.board.mode {
                    v["words"] = w.iter().map(|s| s.spelling.clone()).collect::<Vec<_>>().into();
                    v["glyphs"] = w.iter().map(|s| s.glyph.clone()).collect::<Vec<_>>().into();
                    v["mine"] = g.board.mine().into();
                }
                v["lang"] = g.board.lang.clone().into();
                v.to_string()
            })
            .unwrap_or_default()
    })
}

/// Test builds only: the word Tier Mode is asking for right now, and the band
/// its badge promised, so a browser test can spell it (v1.3 F2/F4).
#[cfg(feature = "testseam")]
pub fn seam_prompt() -> String {
    GAME.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|g| {
                let d = g.prompt.as_ref()?;
                Some(serde_json::json!({
                    "spelling": d.spelling, "band": d.band, "digit": g.prompt_digit,
                }).to_string())
            })
            .unwrap_or_default()
    })
}
