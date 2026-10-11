//! CC-PICK-QUEUE v1 Phase C -- help choosing: sort, search and "Toughest 5".
//!
//! Pure, DOM-free and host-testable. `pick_ui` renders it for BOTH surfaces
//! (Misses and My Words) through the one shared component (D10); nothing in
//! here asks which language it is in.
//!
//! What this module is allowed to change: the DISPLAY order and visibility of
//! rows on the pick surface, and (Toughest 5) which words get appended to the
//! lineup -- the same append a tap does. It never touches the serve order of an
//! existing lineup, grading, scoring, streaks or when a miss counts as learned
//! (I1, A12). With nothing chosen, [`order`] is the identity, so zero picks and
//! defaults is today's surface exactly (I4).
//!
//! ## Definitions (the contract; tests pin each one)
//!
//! * **As listed** -- the default, the order the surface shows today
//!   (Misses: stored order, newest first-miss first; My Words: list order).
//! * **Missed most** -- descending miss count. Ties: most recently missed
//!   first, then alphabetical by folded spelling, then the default position.
//!   A word with no recorded miss counts 0 and sorts last.
//! * **Due now** -- words whose next spaced check is due (due time <= now)
//!   first, soonest due first; then words with a schedule that is not yet due,
//!   soonest first; then words with no schedule at all (a My Words word that
//!   was never missed). Ties keep the default position.
//! * **Newest** -- most recently added/missed first. Misses: the entry's `ts`
//!   (last miss). My Words: `ListEntry.added_at`. A word with no stamp (a
//!   legacy entry, 0) sorts after every stamped one. Ties keep default order.
//! * **Never practiced** -- Misses only: words with no practice history first,
//!   then the rest, each group in default order. See [`miss_never_practiced`].
//!   My Words does not offer it: its only practice record is `wordstats`
//!   (solo rounds, 500-entry cap), where an evicted word would wrongly read as
//!   never practiced.
//!
//! **Toughest 5** -- always aims for five words in the CURRENT scope (surface
//! + list + language; a lineup never spans two languages, D9). First the
//! missed words by the Missed-most comparator above (most misses, then most
//! recently missed, then alphabetical), at most five; if that is fewer than
//! five, it pads with never-missed words in the surface's default order (As
//! listed), skipping words already picked. If the scope holds fewer words than
//! that, it adds what exists. Already-picked words keep their position; new
//! ones append in that order. Caps are the existing add rules (Standard has
//! none). The button is disabled only when no word at all can be added.
//!
//! **Search** -- case-insensitive and diacritic-insensitive substring match
//! over the word's displayed spelling, through `norm::fold_lenient` (the repo's
//! one fold, which already handles Latin accents, Arabic marks, Cyrillic case
//! and the Russian stress mark). No per-language rule is written here.
//!
//! **State** -- the chosen sort and the search text are per-visit UI state:
//! in memory only, reset every time the surface is opened. No existing store
//! and no new key is written (there is no convention in the repo for
//! persisting list-screen filters; EDITING/RENAMING/OPEN_LIST are the same).

use crate::model::MissEntry;
use crate::pick_queue::Pick;
use crate::word_lists::ListEntry;
use std::cell::RefCell;
use std::collections::HashMap;

/// How many words the quick pick takes.
pub const TOUGHEST_N: usize = 5;

/// Phase C is its own flag and only takes effect while Phase A is also on.
pub fn enabled() -> bool {
    crate::flags::pick_queue_a() && crate::flags::pick_queue_c()
}

/// Whether the sort/search/Toughest-5 controls exist for this player at all:
/// flag on AND a Standard player. Spell Jr: nothing rendered, nothing handled
/// (I7, D4, A10), decided by the same `pick_caps(..).extras` the stars and
/// "Just these" use.
pub fn active(kid: bool) -> bool {
    enabled() && crate::experience::pick_caps(crate::experience::of_kid(kid)).extras
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sort {
    #[default]
    AsListed,
    MissedMost,
    DueNow,
    Newest,
    NeverPracticed,
}

impl Sort {
    pub const ALL: [Sort; 5] = [Sort::AsListed, Sort::MissedMost, Sort::DueNow, Sort::Newest, Sort::NeverPracticed];

    pub fn id(self) -> &'static str {
        match self {
            Sort::AsListed => "listed",
            Sort::MissedMost => "missed",
            Sort::DueNow => "due",
            Sort::Newest => "newest",
            Sort::NeverPracticed => "never",
        }
    }

    pub fn from_id(s: &str) -> Option<Sort> {
        Sort::ALL.into_iter().find(|x| x.id() == s)
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Sort::AsListed => "pick.sort.listed",
            Sort::MissedMost => "pick.sort.missed",
            Sort::DueNow => "pick.sort.due",
            Sort::Newest => "pick.sort.newest",
            Sort::NeverPracticed => "pick.sort.never",
        }
    }
}

/// The sorts a surface can honestly offer; a surface whose data cannot back a
/// sort drops the option instead of faking it (the spec's rule), in one place.
/// My Words has no reliable practice record (see the module doc), so it does
/// not offer Never practiced.
pub fn sorts_for(surface: &str) -> Vec<Sort> {
    match surface {
        crate::pick_queue::SURFACE_MISSES => Sort::ALL.to_vec(),
        crate::pick_queue::SURFACE_MINE => Sort::ALL.iter().copied().filter(|s| *s != Sort::NeverPracticed).collect(),
        _ => vec![Sort::AsListed],
    }
}

/// One row, in the neutral shape every sort reads. `pos` is the row's place in
/// the order the surface shows today.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: String,
    pub word: String,
    pub lang: String,
    pub pos: usize,
    /// Recorded misses; 0 when the word has no miss entry.
    pub misses: u32,
    /// Last miss time (ms), 0 when none.
    pub last_miss: f64,
    /// Next spaced check (ms); None when the word has no schedule.
    pub due: Option<f64>,
    /// "Newest" stamp (ms), 0 when unknown.
    pub recency: f64,
    /// No practice history for this surface's definition.
    pub never: bool,
}

/// Misses: a miss entry exists because the word was missed, so "never
/// practiced" means it has not been answered correctly since. That is
/// `review.step == 0` (a correct answer advances the step; only a re-miss
/// resets it) together with `misses == 1` (so no re-miss could have reset a
/// step that WAS advanced) and an untouched FSRS card (`reps <= 1`, no lapses).
/// A pre-R2 entry with no `review` is read through the same carry-over the
/// loader applies, never guessed at.
pub fn miss_never_practiced(m: &MissEntry) -> bool {
    let r = m.review.clone().unwrap_or_else(|| crate::review::carry_over(m.box_, m.due));
    m.misses <= 1 && crate::review::untouched(&r)
}

/// Rows for the Misses surface, in the order given (today's order).
pub fn rows_for_misses(misses: &[&MissEntry]) -> Vec<Row> {
    misses
        .iter()
        .enumerate()
        .map(|(pos, m)| Row {
            key: crate::word_id::word_id0(&m.lang, &m.word),
            word: m.word.clone(),
            lang: m.lang.clone(),
            pos,
            misses: m.misses,
            last_miss: m.ts,
            due: Some(m.due),
            recency: m.ts,
            never: miss_never_practiced(m),
        })
        .collect()
}

/// Rows for a My Words list. A My Words entry carries no practice fields of its
/// own, so each comes from the store that really holds it:
///
/// * misses / last miss / due: the miss entry for the word under the My Words
///   study language (`consts::MINE`), where My Words play records its misses;
/// * newest: `ListEntry.added_at`;
/// * never practiced: not offered on this surface, so `never` is always false.
pub fn rows_for_mine(entries: &[ListEntry], misses: &[MissEntry]) -> Vec<Row> {
    let by_key: HashMap<String, &MissEntry> =
        misses.iter().map(|m| (crate::word_id::word_id0(&m.lang, &m.word), m)).collect();
    entries
        .iter()
        .enumerate()
        .map(|(pos, e)| {
            let miss = by_key.get(&crate::word_id::word_id0(crate::consts::MINE, &e.text));
            Row {
                key: crate::word_id::word_id0(&e.lang, &e.text),
                word: e.text.clone(),
                lang: e.lang.clone(),
                pos,
                misses: miss.map(|m| m.misses).unwrap_or(0),
                last_miss: miss.map(|m| m.ts).unwrap_or(0.0),
                due: miss.map(|m| m.due),
                recency: e.added_at,
                never: false,
            }
        })
        .collect()
}

pub fn fold(s: &str) -> String {
    crate::norm::fold_lenient(s)
}

/// Whether `word` matches an already-folded query. An empty query matches all.
pub fn matches_folded(word: &str, folded_query: &str) -> bool {
    folded_query.is_empty() || fold(word).contains(folded_query)
}

/// The Missed-most comparator, shared with Toughest 5 so they can never drift.
fn toughest_cmp(a: &Row, b: &Row) -> std::cmp::Ordering {
    b.misses
        .cmp(&a.misses)
        .then(b.last_miss.partial_cmp(&a.last_miss).unwrap_or(std::cmp::Ordering::Equal))
        .then_with(|| fold(&a.word).cmp(&fold(&b.word)))
        .then(a.pos.cmp(&b.pos))
}

fn due_class(r: &Row, now: f64) -> u8 {
    match r.due {
        Some(d) if d <= now => 0,
        Some(_) => 1,
        None => 2,
    }
}

/// Display order of ALL rows (indices into `rows`) for `sort`. Pure; the
/// serve order is not an input and not an output.
pub fn order(rows: &[Row], sort: Sort, now: f64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..rows.len()).collect();
    match sort {
        Sort::AsListed => {}
        Sort::MissedMost => idx.sort_by(|&a, &b| toughest_cmp(&rows[a], &rows[b])),
        Sort::DueNow => idx.sort_by(|&a, &b| {
            let (x, y) = (&rows[a], &rows[b]);
            due_class(x, now)
                .cmp(&due_class(y, now))
                .then(x.due.unwrap_or(0.0).partial_cmp(&y.due.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal))
                .then(x.pos.cmp(&y.pos))
        }),
        Sort::Newest => idx.sort_by(|&a, &b| {
            let (x, y) = (&rows[a], &rows[b]);
            y.recency.partial_cmp(&x.recency).unwrap_or(std::cmp::Ordering::Equal).then(x.pos.cmp(&y.pos))
        }),
        Sort::NeverPracticed => idx.sort_by(|&a, &b| {
            let (x, y) = (&rows[a], &rows[b]);
            y.never.cmp(&x.never).then(x.pos.cmp(&y.pos))
        }),
    }
    idx
}

/// What the surface shows: the visible rows in display order, and how many
/// lined-up words the search is hiding right now.
#[derive(Debug, PartialEq)]
pub struct View {
    pub visible: Vec<usize>,
    pub hidden_picks: usize,
    /// Rows the search removed (for the "no match" note).
    pub filtered_out: usize,
}

pub fn view(rows: &[Row], sort: Sort, query: &str, picks: &[Pick], now: f64) -> View {
    let q = fold(query);
    let ordered = order(rows, sort, now);
    let (visible, hidden): (Vec<usize>, Vec<usize>) = ordered.into_iter().partition(|&i| matches_folded(&rows[i].word, &q));
    let hidden_picks = hidden.iter().filter(|&&i| picks.iter().any(|p| p.key == rows[i].key)).count();
    View { visible, hidden_picks, filtered_out: hidden.len() }
}

/// The words Toughest 5 targets, as indices into `rows` (the caller passes ONLY
/// the current scope's rows): the missed words, most-missed first, at most `n`;
/// then, if short of `n`, never-missed words in default order (`pos`) that are
/// not already in `picked`. Already-picked missed words are included (they
/// count toward the `n` and keep their lineup slot when appended).
pub fn toughest(rows: &[Row], n: usize, picked: &[Pick]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].misses > 0).collect();
    idx.sort_by(|&a, &b| toughest_cmp(&rows[a], &rows[b]));
    idx.truncate(n);
    if idx.len() < n {
        let mut pad: Vec<usize> = (0..rows.len())
            .filter(|&i| rows[i].misses == 0 && !picked.iter().any(|p| p.key == rows[i].key))
            .collect();
        pad.sort_by_key(|&i| rows[i].pos);
        idx.extend(pad.into_iter().take(n - idx.len()));
    }
    idx
}

/// The subset of [`toughest`] that is not already lined up: what a press would
/// actually add. Empty means the button has nothing to do.
pub fn toughest_new(rows: &[Row], n: usize, picked: &[Pick]) -> Vec<usize> {
    toughest(rows, n, picked).into_iter().filter(|&i| !picked.iter().any(|p| p.key == rows[i].key)).collect()
}

// ---------- per-visit UI state ----------

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HelpState {
    pub sort: Sort,
    pub query: String,
}

thread_local! {
    static STATE: RefCell<HashMap<String, HelpState>> = RefCell::new(HashMap::new());
}

fn state_key(surface: &str, list: &str) -> String {
    format!("{surface}|{list}")
}

pub fn state(surface: &str, list: &str) -> HelpState {
    STATE.with(|s| s.borrow().get(&state_key(surface, list)).cloned().unwrap_or_default())
}

pub fn set_sort(surface: &str, list: &str, sort: Sort) {
    STATE.with(|s| s.borrow_mut().entry(state_key(surface, list)).or_default().sort = sort);
}

pub fn set_query(surface: &str, list: &str, query: &str) {
    STATE.with(|s| s.borrow_mut().entry(state_key(surface, list)).or_default().query = query.to_string());
}

/// A new visit to the surface: back to As listed, nothing typed.
pub fn reset(surface: &str, list: &str) {
    STATE.with(|s| {
        s.borrow_mut().remove(&state_key(surface, list));
    });
}

/// The picks Toughest 5 appends for `scope_rows`, as lineup picks.
pub fn toughest_picks(scope_rows: &[Row], n: usize, picked: &[Pick]) -> Vec<Pick> {
    toughest(scope_rows, n, picked)
        .into_iter()
        .map(|i| Pick { key: scope_rows[i].key.clone(), word: scope_rows[i].word.clone(), lang: scope_rows[i].lang.clone() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick_queue::{scope_id, Lineups, Toggled, SURFACE_MINE, SURFACE_MISSES};

    const NOW: f64 = 1_000_000.0;

    fn row(word: &str, pos: usize, misses: u32, last_miss: f64) -> Row {
        Row {
            key: format!("en::{}", word.to_lowercase()),
            word: word.to_string(),
            lang: "en".into(),
            pos,
            misses,
            last_miss,
            due: Some(NOW + 1000.0),
            recency: last_miss,
            never: false,
        }
    }

    fn words(rows: &[Row], idx: &[usize]) -> Vec<String> {
        idx.iter().map(|&i| rows[i].word.clone()).collect()
    }

    fn pk(r: &Row) -> Pick {
        Pick { key: r.key.clone(), word: r.word.clone(), lang: r.lang.clone() }
    }

    #[test]
    fn as_listed_is_the_identity_and_the_default() {
        let rows = vec![row("b", 0, 1, 5.0), row("a", 1, 9, 9.0), row("c", 2, 3, 1.0)];
        assert_eq!(Sort::default(), Sort::AsListed);
        assert_eq!(order(&rows, Sort::AsListed, NOW), vec![0, 1, 2]);
        // Zero picks, no query, default sort: the view IS today's list (I4).
        let v = view(&rows, Sort::default(), "", &[], NOW);
        assert_eq!(v, View { visible: vec![0, 1, 2], hidden_picks: 0, filtered_out: 0 });
        assert_eq!(state("misses", ""), HelpState::default());
    }

    #[test]
    fn missed_most_orders_by_count_with_the_three_way_tie_break() {
        let rows = vec![
            row("delta", 0, 2, 100.0),
            row("alpha", 1, 2, 100.0), // ties delta on count and recency -> alphabetical
            row("echo", 2, 2, 500.0),  // ties on count, more recent -> before both
            row("bravo", 3, 7, 1.0),   // most misses wins whatever the recency
            row("Charlie", 4, 1, 900.0),
        ];
        let o = order(&rows, Sort::MissedMost, NOW);
        assert_eq!(words(&rows, &o), ["bravo", "echo", "alpha", "delta", "Charlie"]);
    }

    #[test]
    fn missed_most_ties_that_are_total_fall_back_to_the_default_position() {
        let rows = vec![row("same", 0, 2, 5.0), row("same", 1, 2, 5.0)];
        assert_eq!(order(&rows, Sort::MissedMost, NOW), vec![0, 1]);
    }

    #[test]
    fn due_now_is_due_first_then_scheduled_then_unscheduled() {
        let due = |w: &str, pos: usize, d: Option<f64>| Row { due: d, ..row(w, pos, 1, 0.0) };
        let rows = vec![
            due("later", 0, Some(NOW + 9000.0)),
            due("overdue", 1, Some(NOW - 5000.0)),
            due("justdue", 2, Some(NOW)), // due means due <= now
            due("soon", 3, Some(NOW + 100.0)),
            due("never-missed", 4, None),
            due("tie-a", 5, Some(NOW - 1.0)),
            due("tie-b", 6, Some(NOW - 1.0)),
        ];
        let o = order(&rows, Sort::DueNow, NOW);
        assert_eq!(words(&rows, &o), ["overdue", "tie-a", "tie-b", "justdue", "soon", "later", "never-missed"]);
    }

    #[test]
    fn newest_is_most_recent_first_and_unstamped_sort_last() {
        let mut rows = vec![row("old", 0, 1, 10.0), row("legacy", 1, 1, 0.0), row("new", 2, 1, 90.0), row("tie", 3, 1, 10.0)];
        let o = order(&rows, Sort::Newest, NOW);
        assert_eq!(words(&rows, &o), ["new", "old", "tie", "legacy"]);
        // My Words reads added_at into the same field.
        rows[1].recency = 50.0;
        assert_eq!(words(&rows, &order(&rows, Sort::Newest, NOW)), ["new", "legacy", "old", "tie"]);
    }

    #[test]
    fn never_practiced_first_then_the_rest_each_in_default_order() {
        let mut rows = vec![row("a", 0, 1, 0.0), row("b", 1, 1, 0.0), row("c", 2, 1, 0.0), row("d", 3, 1, 0.0)];
        rows[1].never = true;
        rows[3].never = true;
        assert_eq!(words(&rows, &order(&rows, Sort::NeverPracticed, NOW)), ["b", "d", "a", "c"]);
    }

    /// A miss entry whose schedule was built through review.rs, never by hand.
    fn miss(word: &str, misses: u32, review: crate::review::ReviewState) -> MissEntry {
        MissEntry {
            word: word.into(),
            lang: "en".into(),
            tier: "easy".into(),
            misses,
            box_: 0,
            due: NOW,
            ts: NOW,
            review: Some(review),
        }
    }

    fn fresh() -> crate::review::ReviewState {
        crate::review::start(NOW).0
    }

    #[test]
    fn a_miss_is_never_practiced_only_until_its_first_correct_answer() {
        use crate::review::{on_correct, on_miss, Grade};
        // Fresh miss: step 0, one FSRS rep from the miss itself.
        assert!(miss_never_practiced(&miss("fresh", 1, fresh())));
        // A correct answer advanced the learning step.
        let mut answered = fresh();
        on_correct(&mut answered, Grade::Good, NOW);
        assert!(!miss_never_practiced(&miss("answered", 1, answered)));
        // Re-missed while still learning: step reset to 0 and FSRS untouched,
        // but the miss count says it has been seen before.
        let mut remissed = fresh();
        on_miss(&mut remissed, NOW);
        assert!(!miss_never_practiced(&miss("remissed", 2, remissed)));
        // Learned, then lapsed on FSRS.
        let mut lapsed = fresh();
        on_correct(&mut lapsed, Grade::Good, NOW);
        on_correct(&mut lapsed, Grade::Good, NOW + 700_000.0);
        on_miss(&mut lapsed, NOW + 2.0 * 86_400_000.0);
        assert!(!miss_never_practiced(&miss("lapsed", 2, lapsed)));
    }

    #[test]
    fn my_words_rows_read_misses_under_the_study_language_and_added_at() {
        let entries = vec![
            ListEntry { text: "Cat".into(), lang: "en".into(), added_at: 10.0, starred: false },
            ListEntry { text: "dog".into(), lang: "en".into(), added_at: 20.0, starred: false },
            ListEntry { text: "eel".into(), lang: "en".into(), added_at: 0.0, starred: false },
        ];
        let mut m = miss("cat", 4, fresh());
        m.lang = crate::consts::MINE.into();
        m.ts = 777.0;
        // Same text under a real language must NOT be read as the My Words miss.
        let mut other = miss("dog", 9, fresh());
        other.lang = "en".into();
        let rows = rows_for_mine(&entries, &[m, other]);
        assert_eq!((rows[0].misses, rows[0].last_miss), (4, 777.0), "case-insensitive identity, MINE only");
        assert_eq!(rows[1].misses, 0);
        assert_eq!(rows[0].recency, 10.0);
        assert!(rows.iter().all(|r| !r.never), "My Words has no never-practiced derivation");
        assert!(rows[2].due.is_none());
    }

    #[test]
    fn search_is_case_diacritic_and_script_insensitive() {
        let q = |s: &str| fold(s);
        assert!(matches_folded("Café", &q("CAFE")));
        assert!(matches_folded("café", &q("é")));
        assert!(matches_folded("Straße", &q("strasse")));
        assert!(matches_folded("Über", &q("uber")));
        assert!(matches_folded("niño", &q("NIN")));
        // Non-Latin: Cyrillic case, with and without the stress mark.
        assert!(matches_folded("Замок", &q("зам")));
        assert!(matches_folded("за\u{301}мок", &q("замок")));
        // Arabic: harakat on the stored word do not block a plain query, and
        // the query may be a middle substring.
        assert!(matches_folded("كَتَبَ", &q("كتب")));
        assert!(matches_folded("مكتبة", &q("كتب")));
        // CJK and Devanagari are plain substrings.
        assert!(matches_folded("苹果汁", &q("果")));
        assert!(matches_folded("किताब", &q("ताब")));
        assert!(!matches_folded("किताब", &q("कलम")));
        // Whitespace in the query does not defeat a multi-word entry.
        assert!(matches_folded("ice cream", &q("ice cream")));
        assert!(matches_folded("anything", ""));
        assert!(!matches_folded("cafe", &q("tea")));
    }

    #[test]
    fn view_filters_and_counts_the_picks_it_hides() {
        let rows = vec![row("apple", 0, 1, 1.0), row("bread", 1, 1, 1.0), row("crane", 2, 1, 1.0), row("apricot", 3, 1, 1.0)];
        let picks = vec![pk(&rows[1]), pk(&rows[0])];
        let v = view(&rows, Sort::AsListed, "ap", &picks, NOW);
        assert_eq!(words(&rows, &v.visible), ["apple", "apricot"]);
        assert_eq!(v.hidden_picks, 1, "bread is lined up but filtered out");
        assert_eq!(v.filtered_out, 2);
        // A sort reorders what is visible and nothing else.
        let v = view(&rows, Sort::MissedMost, "ap", &picks, NOW);
        assert_eq!(words(&rows, &v.visible), ["apple", "apricot"]);
    }

    #[test]
    fn hidden_picks_stay_lined_up_with_their_numbers() {
        // The lineup model is never consulted by the filter: a search cannot
        // change it, which is the property under test.
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MISSES, "", "en");
        let rows = vec![row("apple", 0, 1, 1.0), row("bread", 1, 1, 1.0), row("crane", 2, 1, 1.0)];
        for r in &rows {
            assert!(matches!(l.toggle(&id, SURFACE_MISSES, "", "en", pk(r), None), Toggled::Added(_)));
        }
        let before = l.clone();
        let v = view(&rows, Sort::NeverPracticed, "cr", l.picks(&id), NOW);
        assert_eq!(words(&rows, &v.visible), ["crane"]);
        assert_eq!(v.hidden_picks, 2);
        assert_eq!(l, before, "view() is read-only on the lineup");
        assert_eq!(l.number_of(&id, "en::apple"), Some(1));
        assert_eq!(l.number_of(&id, "en::bread"), Some(2));
        assert_eq!(l.number_of(&id, "en::crane"), Some(3));
        // Clearing the search shows them all again, numbers intact.
        let v = view(&rows, Sort::AsListed, "", l.picks(&id), NOW);
        assert_eq!((v.visible.len(), v.hidden_picks), (3, 0));
    }

    #[test]
    fn toughest_takes_the_five_most_missed_with_the_stated_tie_break() {
        let rows = vec![
            row("w1", 0, 1, 10.0),
            row("w2", 1, 8, 10.0),
            row("w3", 2, 5, 30.0),
            row("w4", 3, 5, 20.0),
            row("beta", 4, 3, 5.0),
            row("alpha", 5, 3, 5.0), // ties beta fully -> alphabetical
            row("w7", 6, 2, 99.0),   // sixth: cut
            row("never", 7, 0, 0.0),
        ];
        let t = toughest(&rows, TOUGHEST_N, &[]);
        assert_eq!(words(&rows, &t), ["w2", "w3", "w4", "alpha", "beta"]);
    }

    #[test]
    fn toughest_with_fewer_than_five_missed_pads_with_never_missed_in_default_order() {
        // Default order is `pos`, not the slice order: z (pos 0) leads the pad.
        let rows = vec![
            row("m", 4, 2, 1.0),
            row("n", 3, 1, 1.0),
            row("z", 0, 0, 0.0),
            row("y", 1, 0, 0.0),
            row("x", 2, 0, 0.0),
            row("w", 5, 0, 0.0),
        ];
        let t = toughest(&rows, TOUGHEST_N, &[]);
        assert_eq!(words(&rows, &t), ["m", "n", "z", "y", "x"], "missed first, then never-missed by position; w is cut");
        // No missed word at all: the first five by position.
        let none: Vec<Row> = (0..7).map(|i| row(&format!("w{i}"), i, 0, 0.0)).collect();
        assert_eq!(words(&none, &toughest(&none, TOUGHEST_N, &[])), ["w0", "w1", "w2", "w3", "w4"]);
    }

    #[test]
    fn toughest_with_a_small_scope_adds_what_exists() {
        let rows = vec![row("a", 0, 0, 0.0), row("b", 1, 2, 1.0), row("c", 2, 0, 0.0)];
        assert_eq!(words(&rows, &toughest(&rows, TOUGHEST_N, &[])), ["b", "a", "c"]);
        assert!(toughest(&[], TOUGHEST_N, &[]).is_empty());
        assert_eq!(toughest(&[row("z", 0, 0, 0.0)], TOUGHEST_N, &[]), vec![0], "a single never-missed word is added");
    }

    #[test]
    fn toughest_padding_skips_words_already_picked() {
        let rows = vec![row("a", 0, 0, 0.0), row("b", 1, 0, 0.0), row("c", 2, 3, 1.0), row("d", 3, 0, 0.0), row("e", 4, 0, 0.0)];
        let picked = vec![pk(&rows[0])];
        let t = toughest(&rows, TOUGHEST_N, &picked);
        assert_eq!(words(&rows, &t), ["c", "b", "d", "e"], "a is picked, so it is not padded in; scope has only four others");
        assert_eq!(words(&rows, &toughest_new(&rows, TOUGHEST_N, &picked)), ["c", "b", "d", "e"]);
        // Everything lined up already: nothing to add (the button is disabled).
        let all: Vec<Pick> = rows.iter().map(pk).collect();
        assert!(toughest_new(&rows, TOUGHEST_N, &all).is_empty());
        assert!(!toughest_new(&rows, TOUGHEST_N, &[]).is_empty());
    }

    #[test]
    fn toughest_pads_through_the_lineup_keeping_picked_slots() {
        let rows = vec![row("a", 0, 0, 0.0), row("b", 1, 4, 1.0), row("c", 2, 0, 0.0), row("d", 3, 0, 0.0), row("e", 4, 0, 0.0), row("f", 5, 0, 0.0)];
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MINE, "l1", "en");
        l.toggle(&id, SURFACE_MINE, "l1", "en", pk(&rows[2]), None); // c, never missed, picked first
        let picks = toughest_picks(&rows, TOUGHEST_N, l.picks(&id));
        let (added, cut) = l.add_many(&id, SURFACE_MINE, "l1", "en", picks, None);
        assert_eq!((added, cut), (5, false));
        let order: Vec<&str> = l.picks(&id).iter().map(|p| p.word.as_str()).collect();
        assert_eq!(order, ["c", "b", "a", "d", "e", "f"], "c keeps slot 1; b missed; a, d, e, f pad in position order (c skipped)");
    }

    #[test]
    fn toughest_keeps_picked_positions_and_appends_new_in_most_missed_order() {
        let rows = vec![row("a", 0, 1, 1.0), row("b", 1, 9, 1.0), row("c", 2, 5, 1.0), row("d", 3, 7, 1.0), row("e", 4, 2, 1.0)];
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MISSES, "", "en");
        // The player had already picked c, then a (a is NOT in the top 5? it is:
        // there are exactly five words, so all are in).
        l.toggle(&id, SURFACE_MISSES, "", "en", pk(&rows[2]), None);
        l.toggle(&id, SURFACE_MISSES, "", "en", pk(&rows[0]), None);
        let (added, cut) = l.add_many(&id, SURFACE_MISSES, "", "en", toughest_picks(&rows, TOUGHEST_N, &[]), None);
        assert_eq!((added, cut), (3, false));
        let order: Vec<&str> = l.picks(&id).iter().map(|p| p.word.as_str()).collect();
        assert_eq!(order, ["c", "a", "b", "d", "e"], "c and a keep slots 1,2; b(9), d(7), e(2) append");
        // Pressing it again changes nothing.
        let (added, _) = l.add_many(&id, SURFACE_MISSES, "", "en", toughest_picks(&rows, TOUGHEST_N, &[]), None);
        assert_eq!(added, 0);
        assert_eq!(l.picks(&id).len(), 5);
    }

    #[test]
    fn toughest_respects_a_cap_when_there_is_one_and_standard_has_none() {
        let rows: Vec<Row> = (0..5).map(|i| row(&format!("w{i}"), i, 10 - i as u32, 1.0)).collect();
        let mut l = Lineups::default();
        let id = scope_id(SURFACE_MINE, "l1", "en");
        let (added, cut) = l.add_many(&id, SURFACE_MINE, "l1", "en", toughest_picks(&rows, 5, &[]), Some(3));
        assert_eq!((added, cut), (3, true));
        assert_eq!(l.picks(&id).len(), 3);
        let mut l2 = Lineups::default();
        let cap = crate::experience::pick_caps(crate::experience::Experience::Standard).max_picks;
        assert_eq!(l2.add_many(&id, SURFACE_MINE, "l1", "en", toughest_picks(&rows, 5, &[]), cap), (5, false));
    }

    #[test]
    fn toughest_is_scoped_to_the_rows_it_is_given_never_mixing_languages() {
        // The caller passes one language's rows (D9); a German row that is
        // missed more is simply not in the slice.
        let mut de = row("hund", 0, 50, 1.0);
        de.lang = "de".into();
        de.key = "de::hund".into();
        let all = vec![row("cat", 0, 2, 1.0), de.clone(), row("dog", 2, 1, 1.0)];
        let en_only: Vec<Row> = all.iter().filter(|r| r.lang == "en").cloned().collect();
        let picks = toughest_picks(&en_only, TOUGHEST_N, &[]);
        assert_eq!(picks.iter().map(|p| p.word.as_str()).collect::<Vec<_>>(), ["cat", "dog"]);
        assert!(picks.iter().all(|p| p.lang == "en"));
    }

    #[test]
    fn state_is_per_surface_and_list_and_resets_on_a_new_visit() {
        reset("mine", "a");
        reset("mine", "b");
        set_sort("mine", "a", Sort::Newest);
        set_query("mine", "a", "ap");
        assert_eq!(state("mine", "a"), HelpState { sort: Sort::Newest, query: "ap".into() });
        assert_eq!(state("mine", "b"), HelpState::default());
        reset("mine", "a");
        assert_eq!(state("mine", "a"), HelpState::default());
        assert_eq!(Sort::from_id("never"), Some(Sort::NeverPracticed));
        assert_eq!(Sort::from_id("nope"), None);
    }

    #[test]
    fn jr_has_no_help_controls_through_the_one_capability_source() {
        // `active` is `enabled && extras`; extras is false for Junior whatever
        // the flags say, so no control is ever rendered or handled.
        assert!(!crate::experience::pick_caps(crate::experience::Experience::Junior).extras);
        assert!(!active(true));
        assert!(crate::experience::pick_caps(crate::experience::Experience::Standard).extras);
    }

    #[test]
    fn my_words_drops_never_practiced_and_unknown_surfaces_only_the_default() {
        assert_eq!(sorts_for(SURFACE_MISSES), Sort::ALL.to_vec());
        assert_eq!(
            sorts_for(SURFACE_MINE),
            vec![Sort::AsListed, Sort::MissedMost, Sort::DueNow, Sort::Newest],
            "wordstats is capped, so My Words cannot say never practiced"
        );
        assert_eq!(sorts_for("elsewhere"), vec![Sort::AsListed]);
    }
}
