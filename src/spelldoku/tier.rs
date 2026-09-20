//! CC-SPELLDOKU v1.3 — Tier Mode: the digit is an index into a difficulty band.
//!
//! One board can serve up to 81 distinct words instead of nine repeated ones,
//! and the player sequences their own difficulty: solve the cells you can spell.
//!
//! THE LADDER IS THIS TABLE AND NOWHERE ELSE (I-T9). No `if board_tier == ...`
//! tier logic may live anywhere but `LADDER`.
//!
//! The tier a digit indexes is a BANK LIST, not a per-row field: no such field
//! exists, and the offline expert-tier calibration scores the same lists
//! (census C1, Eric 2026-09-20). Eligibility inside a list stays Word Mode's —
//! `wordmode::draw_from` — so there is one definition of "servable row".

use super::gen::Tier;
use super::wordmode;
use crate::wordsearch::ledger::Ledger;

/// F5: a (language, board tier) pair is offered only when every tier of its
/// span holds at least this many eligible rows. 27 cells x 1.25.
pub const NEED_A: usize = 34;
/// F6: Reading B fills four symbols, not 27. 4 x 1.25.
pub const NEED_B: usize = 5;

/// F1. Index 0 is digit 1. The top tier of each span sits on digits 4, 8 and 9
/// (D-T2).
///
/// EASY IS PENDING ERIC'S CONFIRMATION: v1.3's F1 table prints
/// `E M E M E E M M M` for Easy (4 Easy digits, 5 Medium) while the same row
/// states "54 E / 27 M" (6 Easy, 3 Medium), which is what D-T2 describes. The
/// totals and D-T2 agree with each other, so the row below follows them.
const LADDER: [(Tier, [&str; 9]); 4] = [
    (Tier::Easy, ["easy", "easy", "easy", "medium", "easy", "easy", "easy", "medium", "medium"]),
    (Tier::Medium, ["easy", "medium", "medium", "hard", "easy", "easy", "medium", "hard", "hard"]),
    (Tier::Hard, ["easy", "medium", "hard", "expert", "easy", "medium", "hard", "expert", "expert"]),
    (Tier::Expert, ["medium", "hard", "hard", "expert", "medium", "medium", "hard", "expert", "expert"]),
];

fn row(board: Tier) -> &'static [&'static str; 9] {
    &LADDER.iter().find(|(t, _)| *t == board).expect("every board tier has a ladder row").1
}

/// The tier digit `d` (1..=9) indexes on a board of this tier.
pub fn band(board: Tier, d: u8) -> Option<&'static str> {
    (1..=9).contains(&d).then(|| row(board)[d as usize - 1])
}

/// F7: Spell Jr plays 4x4 with the Easy row truncated to digits 1..=4, so it
/// can never reach Hard or Expert through any path in this file.
pub fn jr_band(d: u8) -> Option<&'static str> {
    (1..=4).contains(&d).then(|| row(Tier::Easy)[d as usize - 1])
}

/// The distinct tiers a board tier can serve, deepest first, in TIERS order.
pub fn span(board: Tier) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for t in crate::experience::TIERS {
        if row(board).contains(&t) && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// F5: is Reading A offered for this pair? A pair that fails is ABSENT from the
/// picker, never locked.
pub fn available(lang: &str, board: Tier) -> bool {
    span(board).iter().all(|t| wordmode::depth(lang, t) >= NEED_A)
}

/// F6: Reading B needs all four tiers, and only four symbols of each.
pub fn symbols_available(lang: &str) -> bool {
    crate::experience::TIERS.iter().all(|t| wordmode::depth(lang, t) >= NEED_B)
}

/// F6 (Reading B): a 4x4 board whose four symbols ARE the tier badges, by
/// construction the Hard span. Digit d shows and indexes TIERS[d-1].
pub fn symbol_band(d: u8) -> Option<&'static str> {
    (1..=4).contains(&d).then(|| crate::experience::TIERS[d as usize - 1])
}

/// The i18n key for a band's player-facing name (D-T8: the difficulty picker's
/// own strings, never a second naming).
pub fn label_key(band: &str) -> &'static str {
    match band {
        "easy" => "level.easy",
        "medium" => "level.medium",
        "hard" => "level.hard",
        _ => "level.expert",
    }
}

/// One word served for one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draw {
    /// What the player types (NFC, lowercase; toned pinyin for zh).
    pub spelling: String,
    /// The bank's own form where it differs (zh Hanzi). Audio needs it.
    pub display: Option<String>,
    /// The band it came from — what the digit's badge promised (I-T3).
    pub band: &'static str,
}

/// The Tier Mode state of ONE board: which words it has served (I-T4) and which
/// (cell, digit) pairs have already been earned (F8).
///
/// It reads the audited bank, and of the player's own data only the D9 repeat
/// window that I-T4 requires — never LearnerQuery, the missed-words queue or My
/// Words (I-T5). The window says which words were served lately, in any mode;
/// it says nothing about how the player did with them, so board content still
/// is not a mirror of their failures.
pub struct Session {
    lang: String,
    board: Tier,
    jr: bool,
    symbols: bool,
    rng: super::rng::Rng,
    used: Vec<String>,
    satisfied: Vec<(usize, u8)>,
    /// I-T4's other half: the player's CC-WORDGRID D9 window, shared with
    /// Spell Cross and Word Search, so a word served there does not come
    /// straight back here. A board is one puzzle, however many bands it spans.
    ledger: Ledger,
    day: u32,
    /// `(lang:band, word)` for every word this board served, and how many of
    /// them had to come out of the window because the band had nothing fresh.
    served: Vec<(String, String)>,
    relaxed: usize,
}

impl Session {
    pub fn new(lang: &str, board: Tier, jr: bool, symbols: bool, seed: u64) -> Self {
        Self::with_ledger(lang, board, jr, symbols, seed, Ledger::default(), 0)
    }

    /// The same, carrying the player's repeat window (I-T4). The caller owns
    /// storage; this module only reads the window and reports what it served.
    pub fn with_ledger(
        lang: &str,
        board: Tier,
        jr: bool,
        symbols: bool,
        seed: u64,
        ledger: Ledger,
        day: u32,
    ) -> Self {
        Session {
            ledger,
            day,
            served: Vec::new(),
            relaxed: 0,
            lang: lang.to_string(),
            board,
            jr,
            // D-T5: Reading B needs four distinct tiers and Jr's span is E-M, so
            // a Jr board is Reading A whatever the setting says. Enforced here,
            // once, rather than at each caller.
            symbols: symbols && !jr,
            rng: super::rng::Rng::new(seed ^ 0x5449_4552), // "TIER"
            used: Vec::new(),
            satisfied: Vec::new(),
        }
    }

    /// The tier this digit indexes on this board — what its badge shows.
    pub fn band_of(&self, digit: u8) -> Option<&'static str> {
        if self.symbols {
            symbol_band(digit)
        } else if self.jr {
            jr_band(digit)
        } else {
            band(self.board, digit)
        }
    }

    /// Reading B shows badges in the cells instead of digits.
    pub fn symbols(&self) -> bool {
        self.symbols
    }

    /// F2/F4: a word for this digit, from its band, never one already served on
    /// this board. Every call is a fresh draw, so a failed spelling is followed
    /// by a different word of the same tier.
    pub fn draw(&mut self, digit: u8) -> Option<Draw> {
        let band = self.band_of(digit)?;
        let key = format!("{}:{band}", self.lang);
        // I-T4: prefer a word outside the player's D9 window. When the band has
        // nothing fresh left, take a word from inside it and count the
        // relaxation, exactly as the ledger's own selector does (F-X3: relax,
        // never error).
        let fresh = {
            let (ledger, day, k) = (&self.ledger, self.day, key.as_str());
            wordmode::draw_from_if(&self.lang, band, &mut self.rng, &self.used, &|w| {
                !ledger.holds(k, w, day)
            })
        };
        let (spelling, display) = match fresh.or_else(|| {
            self.relaxed += 1;
            wordmode::draw_from(&self.lang, band, &mut self.rng, &self.used)
        }) {
            Some(w) => w,
            // The band is exhausted -- only reachable after failing hundreds of
            // spellings on one board, since the shallowest band any language
            // offers holds 208 words. I-T2 outranks I-T4 here: repeat a word
            // rather than leave a digit that draws nothing and a cell that can
            // never be filled.
            None => wordmode::draw_from(&self.lang, band, &mut self.rng, &[])?,
        };
        self.used.push(spelling.clone());
        self.served.push((key, spelling.clone()));
        Some(Draw { spelling, display, band })
    }

    /// The board is over (finished, replaced or abandoned): fold what it served
    /// into the window and hand the ledger back to be stored. One puzzle, so
    /// the counter moves once, and a second call with nothing new served writes
    /// nothing — the session stays usable either way, because a solved board is
    /// still on the screen and still has to render its own badges.
    pub fn flush(&mut self) -> Option<Ledger> {
        if self.served.is_empty() {
            return None;
        }
        let served = std::mem::take(&mut self.served);
        let relaxed = std::mem::take(&mut self.relaxed);
        self.ledger.record_many(&served, self.day, relaxed);
        Some(self.ledger.clone())
    }

    /// F8: has this (cell, digit) already been spelled on this board?
    pub fn satisfied(&self, cell: usize, digit: u8) -> bool {
        self.satisfied.contains(&(cell, digit))
    }

    /// F8: record a correct spelling. Re-entering the SAME digit in that cell is
    /// free afterwards; a different digit is charged normally.
    pub fn record(&mut self, cell: usize, digit: u8) {
        if !self.satisfied(cell, digit) {
            self.satisfied.push((cell, digit));
        }
    }

    pub fn served(&self) -> usize {
        self.used.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Done #1 — I-T6: neither the floor nor the ceiling of a span may fall as
    /// the board tier rises, and the top tier owns digits 4, 8, 9 (D-T2).
    #[test]
    fn ladder_table() {
        let order = |t: &str| crate::experience::TIERS.iter().position(|x| *x == t).unwrap();
        let ladder_tiers = [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert];
        let mut last: Option<(usize, usize)> = None;
        for board in ladder_tiers {
            let s = span(board);
            assert!(!s.is_empty(), "{board:?} has a span");
            let floor = order(s.first().unwrap());
            let ceiling = order(s.last().unwrap());
            assert!(floor <= ceiling, "{board:?}: span is ordered");
            if let Some((pf, pc)) = last {
                assert!(floor >= pf, "I-T6: {board:?} floor fell");
                assert!(ceiling >= pc, "I-T6: {board:?} ceiling fell");
            }
            last = Some((floor, ceiling));
            let top = *s.last().unwrap();
            for d in [4, 8, 9] {
                assert_eq!(band(board, d), Some(top), "D-T2: {board:?} digit {d} is the top tier");
            }
        }
        // The exact spans of F1.
        assert_eq!(span(Tier::Easy), vec!["easy", "medium"]);
        assert_eq!(span(Tier::Medium), vec!["easy", "medium", "hard"]);
        assert_eq!(span(Tier::Hard), vec!["easy", "medium", "hard", "expert"]);
        assert_eq!(span(Tier::Expert), vec!["medium", "hard", "expert"]);
        assert_eq!(band(Tier::Expert, 1), Some("medium"), "no Easy word on an Expert board");
        assert_eq!(band(Tier::Easy, 0), None);
        assert_eq!(band(Tier::Easy, 10), None);
    }

    /// Done #9 / F7: no Jr path reaches Hard or Expert. Exhaustive.
    #[test]
    fn jr_span() {
        for d in 0..=255u8 {
            match jr_band(d) {
                Some(b) => {
                    assert!(matches!(b, "easy" | "medium"), "Jr digit {d} served {b}");
                    assert!((1..=4).contains(&d));
                }
                None => assert!(!(1..=4).contains(&d)),
            }
        }
    }

    /// Done #2 / I-T4: no board repeats a word, and no word leaves its band.
    /// A full board is 81 draws, so this is 81 x 200 x 4 = 64,800 English words
    /// served; `every_language_draws_its_whole_board` covers the other 14.
    #[test]
    fn no_repeats() {
        for board in [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
            for seed in 0..200u64 {
                let mut s = Session::new("en", board, false, false, seed);
                let mut seen: Vec<String> = Vec::new();
                for cell in 0..81usize {
                    let digit = (cell % 9 + 1) as u8;
                    let d = s.draw(digit).expect("a board tier that is available always draws");
                    assert_eq!(Some(d.band), band(board, digit), "digit {digit} left its band");
                    assert!(!seen.contains(&d.spelling), "{board:?} repeated {}", d.spelling);
                    seen.push(d.spelling);
                }
                assert_eq!(s.served(), 81);
            }
        }
    }

    /// Done #3 / F4 + I-T2: failing a cell draws a NEW word of the same tier,
    /// and never blocks the digit.
    #[test]
    fn failure_redraw() {
        let mut s = Session::new("en", Tier::Hard, false, false, 7);
        let mut words = Vec::new();
        for _ in 0..5 {
            let d = s.draw(4).expect("digit 4 keeps drawing after a failure");
            assert_eq!(d.band, "expert", "Hard digit 4 is Expert every time");
            assert!(!words.contains(&d.spelling), "a failed attempt must not repeat its word");
            words.push(d.spelling);
        }
        assert_eq!(words.len(), 5);
        assert!(!s.satisfied(0, 4), "five failures leave the cell unearned, never locked");
    }

    /// I-T2: a band that runs dry keeps serving. A board where a digit stops
    /// drawing is a board a player cannot finish, so the no-repeat rule yields.
    #[test]
    fn an_exhausted_band_still_serves() {
        let depth = wordmode::depth("en", "easy");
        assert!(depth > 0);
        let mut s = Session::new("en", Tier::Easy, false, false, 21);
        for k in 0..depth + 5 {
            let d = s.draw(1).unwrap_or_else(|| panic!("digit 1 stopped drawing at attempt {k}"));
            assert_eq!(d.band, "easy");
        }
    }

    /// I-T4's other half: a word served on one board stays out of the next one
    /// while it is inside the CC-WORDGRID D9 window.
    #[test]
    fn d9_window_holds_across_boards() {
        let mut first = Session::with_ledger("en", Tier::Easy, false, false, 31, Ledger::default(), 100);
        let mut served = Vec::new();
        for cell in 0..81usize {
            served.push(first.draw((cell % 9 + 1) as u8).expect("a full board").spelling);
        }
        let led = first.flush().expect("a board that served words records them");
        assert_eq!(led.counter, 1, "one board is one puzzle, not one per band");

        let mut next = Session::with_ledger("en", Tier::Easy, false, false, 77, led.clone(), 100);
        for cell in 0..81usize {
            let d = next.draw((cell % 9 + 1) as u8).expect("a full board");
            assert!(!served.contains(&d.spelling), "{} came straight back", d.spelling);
        }
        // And the window is shared, not SpellDoku's own: Word Search would see
        // the same words held under the same key.
        assert!(led.holds("en:easy", served.iter().find(|_| true).unwrap(), 100));
    }

    /// F-X3: relax, never error. When a band has nothing outside the window
    /// left, it serves a held word rather than stop drawing (I-T2 again).
    #[test]
    fn an_exhausted_window_relaxes() {
        let mut led = Ledger::default();
        let all: Vec<(String, String)> = wordmode::bank_words("en", "easy")
            .into_iter()
            .map(|w| ("en:easy".to_string(), w))
            .collect();
        assert!(!all.is_empty());
        led.record_many(&all, 100, 0);
        let mut s = Session::with_ledger("en", Tier::Easy, false, false, 5, led, 100);
        for k in 0..20 {
            let d = s.draw(1).unwrap_or_else(|| panic!("digit 1 stopped drawing at {k}"));
            assert_eq!(d.band, "easy");
        }
    }

    /// Done #7 / F8: the same digit in the same cell is charged once per board.
    #[test]
    fn recommit() {
        let mut s = Session::new("en", Tier::Medium, false, false, 3);
        assert!(!s.satisfied(12, 5));
        s.record(12, 5);
        assert!(s.satisfied(12, 5), "re-entering the same digit is free");
        assert!(!s.satisfied(12, 6), "a different digit in that cell is charged");
        assert!(!s.satisfied(13, 5), "the same digit in another cell is charged");
        s.record(12, 5);
        s.record(12, 6);
        assert!(s.satisfied(12, 6));
    }

    /// D-T5 + F7: Reading B cannot reach a Jr board, so Jr never sees Hard or
    /// Expert through the tier-symbol setting either.
    #[test]
    fn jr_ignores_reading_b() {
        let mut s = Session::new("en", Tier::Hard, true, true, 4);
        assert!(!s.symbols(), "D-T5: Jr plays Reading A");
        for digit in 1..=9u8 {
            if let Some(d) = s.draw(digit) {
                assert!(matches!(d.band, "easy" | "medium"), "Jr served {}", d.band);
                assert!(digit <= 4, "Jr has no digit {digit}");
            }
        }
    }

    /// F7 again, through the session: a Jr board cannot serve Hard or Expert.
    #[test]
    fn jr_session_never_leaves_easy_medium() {
        let mut s = Session::new("en", Tier::Easy, true, false, 11);
        for digit in 1..=9u8 {
            match s.draw(digit) {
                Some(d) => assert!(matches!(d.band, "easy" | "medium"), "Jr served {}", d.band),
                None => assert!(digit > 4, "Jr has no digit {digit}"),
            }
        }
    }

    /// F6: Reading B spans all four tiers on four symbols.
    #[test]
    fn reading_b_spans_four_tiers() {
        let mut s = Session::new("en", Tier::Hard, false, true, 5);
        let mut bands = Vec::new();
        for d in 1..=4u8 {
            let drawn = s.draw(d).expect("Reading B draws every tier");
            bands.push(drawn.band);
        }
        assert_eq!(bands, crate::experience::TIERS.to_vec(), "one symbol per tier, in order");
        assert!(s.draw(5).is_none(), "Reading B has four symbols");
    }

    /// Done #6 / F5: the availability matrix, checked against the census (C2:
    /// all 60 pairs survive, English included), and checked to be a matrix the
    /// picker can act on -- an absent pair is absent, never locked.
    #[test]
    fn availability() {
        let mut absent = Vec::new();
        for (lang, _, _, _) in crate::consts::BUILTIN_LANGS {
            for board in [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
                if !available(lang, board) {
                    absent.push(format!("{lang} {board:?}"));
                }
            }
            assert!(symbols_available(lang), "C2: {lang} offers Reading B");
        }
        assert!(absent.is_empty(), "C2 says all 60 pairs survive; these did not: {absent:?}");
        for board in [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
            assert!(available("en", board), "English must offer {board:?}");
        }
        // F5: nothing is rendered locked. The picker only ever *omits*, so the
        // tier control must carry no lock affordance.
        let ui = include_str!("../spelldoku_ui.rs");
        let strip = body(ui, "// F3: the ladder is legible", "dom::set_html(\"sdTiers\"").to_lowercase();
        for banned in ["lock", "disabled"] {
            assert!(!strip.contains(banned), "F5: the tier strip renders {banned}");
        }
    }

    /// Done #2, the other 14 languages: every available pair can serve a whole
    /// board without repeating or leaving its band.
    #[test]
    fn every_language_draws_its_whole_board() {
        for (lang, _, _, _) in crate::consts::BUILTIN_LANGS {
            for board in [Tier::Easy, Tier::Medium, Tier::Hard, Tier::Expert] {
                if !available(lang, board) {
                    continue;
                }
                for seed in 0..4u64 {
                    let mut s = Session::new(lang, board, false, false, seed);
                    let mut seen: Vec<String> = Vec::new();
                    for cell in 0..81usize {
                        let digit = (cell % 9 + 1) as u8;
                        let d = s.draw(digit)
                            .unwrap_or_else(|| panic!("{lang} {board:?} ran dry at cell {cell}"));
                        assert_eq!(Some(d.band), band(board, digit), "{lang}: digit {digit} left its band");
                        assert!(!seen.contains(&d.spelling), "{lang} {board:?} repeated {}", d.spelling);
                        seen.push(d.spelling);
                    }
                }
            }
        }
    }

    /// The body of one item in a source file, so a scan can be pointed at the
    /// exact path the invariant talks about instead of a whole module.
    fn body<'a>(src: &'a str, from: &str, to: &str) -> &'a str {
        let start = src.find(from).unwrap_or_else(|| panic!("scan target {from:?} has moved"));
        let rest = &src[start..];
        let end = rest[from.len()..].find(to).map(|k| k + from.len()).unwrap_or(rest.len());
        &rest[..end]
    }

    /// Done #4 / I-T1: the ladder is invisible to the generator, solver and
    /// grader. `Tier` itself is NOT banned here -- v1.0 has always handed the
    /// generator a board tier to dig to, and the non-goals forbid changing it.
    /// What must never reach them is this module: the mapping from a digit to a
    /// band, and the words behind it.
    #[test]
    fn symbol_scan() {
        for (name, src) in [
            ("gen.rs", include_str!("gen.rs")),
            ("solve.rs", include_str!("solve.rs")),
            ("canon.rs", include_str!("canon.rs")),
            ("symbols.rs", include_str!("symbols.rs")),
            ("geo.rs", include_str!("geo.rs")),
        ] {
            let code: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();
            for banned in ["ladder", "tiermode", "jr_band", "symbol_band", "band_of", "draw_from",
                           "wordmode", "spelling", "bank", "super::tier", "spelldoku::tier"] {
                assert!(!code.contains(banned), "I-T1: {name} mentions {banned}");
            }
        }
    }

    /// Done #5 / I-T5: the draw path reads the audited bank and nothing else --
    /// never the learner's history. Writing a miss is untouched (Word Mode's
    /// D13 path does that after a failure); this is about what a board is built
    /// FROM.
    #[test]
    fn no_learner_read() {
        let paths = [
            // Everything above `mod tests` -- the scan must not read its own
            // banned list back out of this file.
            ("tier.rs", body(include_str!("tier.rs"), "//! CC-SPELLDOKU", "\nmod tests").to_string()),
            ("wordmode::draw_from", body(include_str!("wordmode.rs"), "pub fn draw_from", "\nfn ").to_string()),
            ("wordmode::bank", body(include_str!("wordmode.rs"), "fn bank(", "\nfn ").to_string()),
        ];
        for (name, src) in paths {
            let code: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();
            for banned in ["learnerquery", "learner", "misses", "missed", "mywords", "my_words",
                           "personal", "mine", "review", "queue"] {
                assert!(!code.contains(banned), "I-T5: {name} reads {banned}");
            }
        }
    }

    /// Done #8 / I-T7: pencil marks are free. The pencil branch of the click
    /// router toggles a candidate bit and renders; it reaches no gate and no
    /// draw, so no word is ever spent on a mark.
    #[test]
    fn pencil() {
        let ui = include_str!("../spelldoku_ui.rs");
        // The branch ends at its own render(); anything past that belongs to
        // another branch or another function.
        let branch = body(ui, "get_attribute(\"data-sd-pen\")", "render();").to_lowercase();
        assert!(branch.contains("g.pencil[i] ^= 1 << v"), "the pencil branch has moved");
        for banned in ["commit_value", "tier", "draw(", "record(", "unlocks", "speak_prompt"] {
            assert!(!branch.contains(banned), "I-T7: a pencil mark reaches {banned}");
        }
        // And a mark never lands on a committed cell, so it cannot stand in for
        // a commit either.
        assert!(branch.contains("g.entries[i] == 0"));
    }
}
