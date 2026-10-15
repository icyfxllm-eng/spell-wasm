//! CC-QUICKBEE-DRAW v1 Phase B -- the run: a fixed clock, words drawn cell by
//! cell, one point per right cell, auto-advance with no confirm button.
//!
//! Pure: no DOM, no clock of its own. The screen owns the 60 second timer and the
//! pad; this owns what a verdict means, so every rule is testable.
//!
//! # Rules
//!
//! * A right cell scores 1 and moves to the next cell (F1, D4). Tier changes
//!   difficulty, never the weight of a cell.
//! * A WRONG cell ends that word. The cells already right keep their points and
//!   the next word starts at once (A3).
//! * A cell the recogniser could not read (nothing legible, or a low-confidence
//!   miss) is not the player's fault: it earns one free redraw per word, with no
//!   penalty (Eric, 2026-10-15; a deliberate softening of CC-CJK-INK F3/F4's
//!   "never mark a written character wrong" for a timed loop). A second unread
//!   cell in the same word counts as wrong, so a pad that reads nothing cannot
//!   stall a run forever.
//! * "Right" is the expected character anywhere in the recogniser's top-N for
//!   the tier (Easy 8, Medium 5, Hard and Expert 3), compared on NFC (I7).
//! * Stroke order is not graded (D2) and there are no hints or ghost trace (D3).

use super::{cfg, cells, nfc, top_n};
use unicode_segmentation::UnicodeSegmentation;

/// The fixed clock (Eric, 2026-10-15).
pub const RUN_SECS: u32 = 60;
/// A miss below this confidence is read as "could not read it", not "wrong".
pub const LOW_CONFIDENCE: f64 = 0.30;
/// Free redraws per word.
pub const REDRAWS_PER_WORD: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Right,
    Wrong,
    /// The recogniser gave nothing, or only a weak miss.
    Unsure,
}

/// A recogniser answer judged against the expected character. Candidates are
/// best first, as `drawing::recognize` returns them.
pub fn judge(expected: &str, cands: &[(String, f64)], tier: &str) -> Verdict {
    let want = nfc(expected);
    if want.is_empty() || cands.is_empty() {
        return Verdict::Unsure;
    }
    if cands.iter().take(top_n(tier)).any(|(t, _)| nfc(t).contains(&want)) {
        return Verdict::Right;
    }
    if cands[0].1 < LOW_CONFIDENCE {
        Verdict::Unsure
    } else {
        Verdict::Wrong
    }
}

/// F1: has the player finished a cell, with no confirm button? `strokes` drawn
/// so far, the expected count where a table has one, and the idle time since
/// the last stroke ended.
///
/// With a stroke table the cell is judged once the expected count is reached and
/// the pen has rested for the debounce; if the player drew a different number of
/// strokes it is judged after three debounces so a variant form is not cut off
/// mid-way. Without one (Korean) only the idle debounce can say.
pub fn ready_to_judge(lang: &str, strokes: usize, expected: Option<u8>, idle_ms: u32) -> bool {
    let Some(c) = cfg(lang) else { return false };
    if strokes == 0 {
        return false;
    }
    match (c.has_strokes, expected) {
        (true, Some(n)) => {
            if strokes >= n as usize {
                idle_ms >= c.debounce_ms
            } else {
                idle_ms >= c.debounce_ms * 3
            }
        }
        _ => idle_ms >= c.debounce_ms,
    }
}

/// What a verdict did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// A cell was right and the word goes on at `cell`.
    CellRight { score: u32, cell: usize },
    /// The last cell was right: the word is done and a new one starts.
    WordDone { score: u32 },
    /// A wrong cell ended the word; `kept` cells had already scored.
    WordEnded { score: u32, kept: usize, expected: String },
    /// An unreadable cell: draw it again, free.
    Redraw,
    /// The run has ended (the clock, or the pool ran out).
    Over,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub lang: String,
    pub tier: String,
    words: Vec<String>,
    idx: usize,
    cell: usize,
    redraws_left: u8,
    pub score: u32,
    pub words_done: u32,
    pub words_ended: u32,
    pub redraws_used: u32,
    pub over: bool,
}

impl Run {
    /// `words` are the drawn forms in the order they will be served.
    pub fn new(lang: &str, tier: &str, words: Vec<String>) -> Run {
        let over = words.is_empty();
        Run {
            lang: lang.to_string(),
            tier: tier.to_string(),
            words,
            idx: 0,
            cell: 0,
            redraws_left: REDRAWS_PER_WORD,
            score: 0,
            words_done: 0,
            words_ended: 0,
            redraws_used: 0,
            over,
        }
    }

    pub fn word(&self) -> Option<&str> {
        if self.over { None } else { self.words.get(self.idx).map(String::as_str) }
    }

    /// Which cell of the current word is being drawn (0-based).
    pub fn cell(&self) -> usize {
        self.cell
    }

    pub fn cell_count(&self) -> usize {
        self.word().map_or(0, cells)
    }

    /// The character to draw now, NFC.
    pub fn expected(&self) -> Option<String> {
        let w = nfc(self.word()?);
        w.graphemes(true).filter(|g| !g.chars().all(char::is_whitespace)).nth(self.cell).map(str::to_string)
    }

    /// Words served so far including the current one: what goes to the history.
    pub fn served(&self) -> &[String] {
        let end = if self.over { self.idx.min(self.words.len()) } else { (self.idx + 1).min(self.words.len()) };
        &self.words[..end]
    }

    fn next_word(&mut self) -> bool {
        self.idx += 1;
        self.cell = 0;
        self.redraws_left = REDRAWS_PER_WORD;
        if self.idx >= self.words.len() {
            self.over = true;
        }
        !self.over
    }

    /// Apply a verdict for the current cell.
    pub fn apply(&mut self, v: Verdict) -> Step {
        if self.over {
            return Step::Over;
        }
        let v = match v {
            Verdict::Unsure if self.redraws_left > 0 => {
                self.redraws_left -= 1;
                self.redraws_used += 1;
                return Step::Redraw;
            }
            // A second unreadable cell in one word is a wrong one.
            Verdict::Unsure => Verdict::Wrong,
            other => other,
        };
        match v {
            Verdict::Right => {
                self.score += 1;
                self.cell += 1;
                // A fresh cell gets its own free redraw only through a new word.
                if self.cell >= self.cell_count() {
                    self.words_done += 1;
                    if self.next_word() {
                        Step::WordDone { score: self.score }
                    } else {
                        Step::Over
                    }
                } else {
                    Step::CellRight { score: self.score, cell: self.cell }
                }
            }
            _ => {
                let expected = self.expected().unwrap_or_default();
                let kept = self.cell;
                self.words_ended += 1;
                if self.next_word() {
                    Step::WordEnded { score: self.score, kept, expected }
                } else {
                    Step::Over
                }
            }
        }
    }

    /// The clock ran out: whatever cell was in progress does not score.
    pub fn time_up(&mut self) {
        self.over = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(t: &str, conf: f64) -> (String, f64) {
        (t.to_string(), conf)
    }

    #[test]
    fn right_is_the_expected_character_inside_the_tiers_top_n() {
        let cands = [c("木", 0.9), c("本", 0.8), c("末", 0.7), c("未", 0.6), c("休", 0.5)];
        // 休 is the fifth candidate: inside Medium's 5 and Easy's 8, outside Hard's 3.
        assert_eq!(judge("休", &cands, "easy"), Verdict::Right);
        assert_eq!(judge("休", &cands, "medium"), Verdict::Right);
        assert_eq!(judge("休", &cands, "hard"), Verdict::Wrong);
        assert_eq!(judge("木", &cands, "expert"), Verdict::Right);
    }

    #[test]
    fn an_empty_or_weak_read_is_unsure_not_wrong() {
        assert_eq!(judge("木", &[], "easy"), Verdict::Unsure);
        assert_eq!(judge("木", &[c("日", 0.10)], "easy"), Verdict::Unsure, "a weak miss is not a verdict on the player");
        assert_eq!(judge("木", &[c("日", 0.80)], "easy"), Verdict::Wrong, "a confident different character is wrong");
    }

    #[test]
    fn i7_comparison_is_on_nfc() {
        // Decomposed Hangul in the candidate still matches the composed expected.
        assert_eq!(judge("한", &[c("\u{1112}\u{1161}\u{11AB}", 0.9)], "easy"), Verdict::Right);
    }

    #[test]
    fn a3_a_wrong_second_cell_scores_the_first_and_the_next_word_begins() {
        let mut r = Run::new("zh", "medium", vec!["你好".into(), "学校".into()]);
        assert_eq!(r.expected().as_deref(), Some("你"));
        assert_eq!(r.apply(Verdict::Right), Step::CellRight { score: 1, cell: 1 });
        assert_eq!(r.expected().as_deref(), Some("好"));
        let s = r.apply(Verdict::Wrong);
        assert_eq!(s, Step::WordEnded { score: 1, kept: 1, expected: "好".into() });
        assert_eq!(r.word(), Some("学校"));
        assert_eq!(r.cell(), 0);
        assert_eq!(r.score, 1, "exactly the first cell's point");
    }

    #[test]
    fn a_finished_word_starts_the_next_and_each_right_cell_is_one_point() {
        let mut r = Run::new("ja", "easy", vec!["ひま".into(), "ねこ".into()]);
        assert_eq!(r.apply(Verdict::Right), Step::CellRight { score: 1, cell: 1 });
        assert_eq!(r.apply(Verdict::Right), Step::WordDone { score: 2 });
        assert_eq!(r.word(), Some("ねこ"));
        assert_eq!(r.apply(Verdict::Right), Step::CellRight { score: 3, cell: 1 });
        assert_eq!(r.apply(Verdict::Right), Step::Over, "the pool ran out");
        assert_eq!((r.score, r.words_done), (4, 2));
    }

    #[test]
    fn an_unreadable_cell_earns_one_free_redraw_per_word_and_then_counts_as_wrong() {
        let mut r = Run::new("zh", "easy", vec!["你好".into(), "学校".into()]);
        assert_eq!(r.apply(Verdict::Unsure), Step::Redraw);
        assert_eq!(r.score, 0);
        assert_eq!(r.cell(), 0, "still on the same cell");
        // The second unreadable cell of this word ends it.
        assert!(matches!(r.apply(Verdict::Unsure), Step::WordEnded { kept: 0, .. }));
        // A new word brings a new free redraw.
        assert_eq!(r.apply(Verdict::Unsure), Step::Redraw);
        assert_eq!(r.redraws_used, 2);
    }

    #[test]
    fn the_clock_ends_the_run_without_scoring_the_cell_in_progress() {
        let mut r = Run::new("zh", "easy", vec!["你好".into()]);
        r.apply(Verdict::Right);
        r.time_up();
        assert!(r.over && r.word().is_none());
        assert_eq!(r.apply(Verdict::Right), Step::Over);
        assert_eq!(r.score, 1);
    }

    #[test]
    fn served_lists_the_words_a_run_actually_reached() {
        let mut r = Run::new("zh", "easy", vec!["一".into(), "二".into(), "三".into()]);
        assert_eq!(r.served(), ["一"]);
        r.apply(Verdict::Right);
        assert_eq!(r.served(), ["一", "二"]);
        r.apply(Verdict::Wrong);
        assert_eq!(r.served(), ["一", "二", "三"]);
    }

    #[test]
    fn auto_advance_waits_for_the_expected_strokes_then_the_debounce() {
        // zh/ja: a 4-stroke character, 250 ms debounce.
        assert!(!ready_to_judge("zh", 0, Some(4), 5000), "nothing drawn");
        assert!(!ready_to_judge("zh", 2, Some(4), 400), "two of four strokes, a short rest: still drawing");
        assert!(!ready_to_judge("zh", 4, Some(4), 100), "all strokes but the pen has not rested");
        assert!(ready_to_judge("zh", 4, Some(4), 250));
        // A different stroke count is judged after three debounces, not never.
        assert!(!ready_to_judge("zh", 3, Some(4), 700));
        assert!(ready_to_judge("zh", 3, Some(4), 750));
        // More strokes than expected: the expected count is met, so the debounce rules.
        assert!(ready_to_judge("ja", 6, Some(3), 250));
    }

    #[test]
    fn korean_has_no_stroke_table_so_only_the_longer_idle_debounce_decides() {
        assert!(!ready_to_judge("ko", 3, None, 399));
        assert!(ready_to_judge("ko", 3, None, 400));
        // And a language outside the table never judges anything.
        assert!(!ready_to_judge("en", 3, None, 9999));
    }

    #[test]
    fn the_run_is_sixty_seconds_and_one_redraw() {
        assert_eq!(RUN_SECS, 60);
        assert_eq!(REDRAWS_PER_WORD, 1);
    }
}
