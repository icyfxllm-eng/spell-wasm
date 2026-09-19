//! CC-SPELLDOKU v1 — the screen (app only, D9).
//!
//! The rules live in `crate::spelldoku`; this file only draws them and routes
//! taps. It never calls the leaderboard or any score service (D7, I9), never
//! touches shields (D6), and reads the keyboard's letters as data through
//! `keyboard::unit_rows`, the way Word Chains does (D10).

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use crate::spelldoku::canon;
use crate::spelldoku::geo::{size_of, Geo};
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
    let all = [(4, Tier::Easy), (6, Tier::Easy), (6, Tier::Medium), (9, Tier::Easy), (9, Tier::Medium), (9, Tier::Hard), (9, Tier::Expert)];
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

/// Serve a board for `(n, tier)`. A Daily board is the same for everyone and is
/// only recorded in the ledger; any other board skips what this player has seen
/// within the window (I6).
fn serve(app: &App, n: usize, tier: Tier, daily: bool) -> bool {
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
    GAME.with(|g| {
        *g.borrow_mut() = Some(Game {
            unlocks: Unlocks::new(tier),
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

fn render() {
    GAME.with(|cell| {
        let gb = cell.borrow();
        let Some(g) = gb.as_ref() else { return };
        let n = g.board.puzzle.n;
        let size = size_of(n).expect("size");
        let mut html = String::new();
        for i in 0..n * n {
            let (r, c) = (i / n, i % n);
            let mut cls = vec!["sd-cell"];
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

        // The tray: chips earned by spelling (D1), then the letters or digits.
        let mut chips = String::new();
        for v in 1..=n as u8 {
            if g.unlocks.chip(v) {
                chips.push_str(&format!(
                    "<button type=\"button\" class=\"sd-chip\" data-sd-chip=\"{v}\">{}</button>",
                    dom::escape_html(&g.board.label(v))
                ));
            }
        }
        dom::set_html("sdChips", &chips);
        let typed = if g.typed.is_empty() && !g.pencil_mode {
            if g.sel.is_some() { t("sd.spell") } else { t("sd.pick") }
        } else {
            g.typed.clone()
        };
        dom::set_text("sdTyped", &typed);
        let keys = if g.pencil_mode {
            // F1: pencil marks are digits and never open the keyboard.
            (1..=n as u8)
                .map(|v| format!("<button type=\"button\" class=\"kb-key\" data-sd-pen=\"{v}\">{}</button>", dom::escape_html(&g.board.label(v))))
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
            for row in rows.iter().chain(std::iter::once(&extras)).filter(|r| !r.is_empty()) {
                k.push_str("<div class=\"sd-row\">");
                for c in row {
                    k.push_str(&key(c));
                }
                k.push_str("</div>");
            }
            if g.board.lang == "vi" {
                k.push_str("<div class=\"sd-row\">");
                for t in VI_TONE_KEYS {
                    k.push_str(&format!(
                        "<button type=\"button\" class=\"kb-key\" data-sd-key=\"{0}\" aria-label=\"tone mark\">a{0}</button>",
                        t
                    ));
                }
                k.push_str("</div>");
            }
            k.push_str(
                "<div class=\"sd-row\"><button type=\"button\" class=\"kb-key wide\" data-sd-back>\u{232B}</button>\
                 <button type=\"button\" class=\"kb-key wide go\" data-sd-go>\u{2713}</button></div>",
            );
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
        let legend: String = if g.board.is_words() {
            (1..=n as u8)
                .map(|v| {
                    format!(
                        "<button type=\"button\" class=\"sd-say\" data-sd-say=\"{v}\" aria-label=\"{}\"><b>{}</b> \u{25B6}</button>",
                        dom::escape_html(&t("sd.sayAria")),
                        dom::escape_html(&g.board.label(v))
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
                    g.unlocks.record_correct(v);
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
                dom::set_text("sdNote", &t(if g.board.is_words() { "sd.misspelledWord" } else { "sd.misspelled" }));
            }
            Verdict::WrongSystem => {
                record_stat(|s| s.wrong_system += 1);
            }
        }
        g.typed.clear();
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
    }
    render();
}

fn submit_typed() {
    let res = GAME.with(|cell| {
        let gb = cell.borrow();
        let g = gb.as_ref()?;
        let i = g.sel?;
        if g.typed.trim().is_empty() {
            return None;
        }
        let expected = g.board.puzzle.solution[i];
        let values = g.board.values_for(&g.typed);
        let verdict = play::verdict(&values, expected);
        let v = match verdict {
            Verdict::Correct => expected,
            Verdict::WrongValue => values.first().copied().unwrap_or(0),
            _ => 0,
        };
        // D13: a Word Mode misspelling joins the existing missed-words queue,
        // through its existing path, with the band the word came from.
        let missed = match (&g.board.mode, verdict) {
            (Mode::Words(w), Verdict::Misspelled) => w.get(expected as usize - 1).map(|s| (s.spelling.clone(), s.band, g.board.lang.clone())),
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
        let Some((cell_i, tech)) = next_step(&geo, &fixed, &restrict) else { return };
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
            .closest("[data-sd-cell],[data-sd-key],[data-sd-back],[data-sd-go],[data-sd-chip],[data-sd-pen],[data-sd-say]")
            .ok()
            .flatten()
        else {
            return;
        };
        if let Some(v) = el.get_attribute("data-sd-say").and_then(|v| v.parse::<u8>().ok()) {
            GAME.with(|c| {
                if let Some(g) = c.borrow().as_ref() {
                    speak(&g.board, v, || dom::set_text("sdNote", &t("sd.audioOff")));
                }
            });
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
                    if g.sel.is_some() && g.typed.chars().count() < 24 {
                        g.typed = type_key(&g.board.lang, &g.typed, &k);
                    }
                }
            });
            render();
        } else if el.get_attribute("data-sd-back").is_some() {
            GAME.with(|c| {
                if let Some(g) = c.borrow_mut().as_mut() {
                    g.typed = backspace(&g.board.lang, &g.typed);
                }
            });
            render();
        } else if el.get_attribute("data-sd-go").is_some() {
            submit_typed();
        } else if let Some(v) = el.get_attribute("data-sd-chip").and_then(|v| v.parse::<u8>().ok()) {
            let verdict = GAME.with(|c| {
                let gb = c.borrow();
                let g = gb.as_ref()?;
                let i = g.sel?;
                if !g.unlocks.chip(v) {
                    return None;
                }
                Some(if g.board.puzzle.solution[i] == v { Verdict::Correct } else { Verdict::WrongValue })
            });
            if let Some(verdict) = verdict {
                commit_value(v, false, verdict);
            }
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
