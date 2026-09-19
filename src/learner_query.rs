//! CC-LEARNING-ENGINE-L0 R1 — `LearnerQuery`, the frozen read contract.
//!
//! Every consumer of learner data (Reports, Guardian Dash, Stats, Calendar,
//! Yearbook) reads through this trait and nothing else (I6). The boundary
//! scan at the bottom of this file fails the build on any direct access
//! outside the implementation.
//!
//! **Frozen surface (signed 2026-09-19: "4 + 4 read methods").** The four
//! methods the L0 file names, plus the four today's consumers need. After
//! the freeze, ADDING a method is allowed; changing or removing one is a
//! stop-and-ask.
//!
//! **Profile key (C4).** Every method takes a [`ProfileId`]. The device is
//! profile 0 until multi-profile ships, so the key shape (profile, language,
//! entry identity) is fixed now and multi-profile changes the stored data,
//! not this contract.
//!
//! **Read-only.** Nothing here writes. Writes stay on the engine's own
//! doors in `learner.rs` / `misses.rs` (record, placement, reset).
//!
//! **Until R2.** The due queue and the at-risk set read the Leitner misses
//! queue (`misses.rs`), as `reports::rematch_set` did. R2 swaps that for
//! per-word FSRS behind these same methods; consumers don't change.

use std::cell::RefCell;

/// Whose learner data. C4: the device is profile 0 until multi-profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProfileId(pub u32);

pub const DEVICE: ProfileId = ProfileId(0);

/// Why the engine served a word. An enum, never prose (L0 R1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextWordReason {
    /// A missed word came due for review.
    DueReview,
    /// A zh tone-drill word came due (served once real misses are clear).
    ToneDrill,
    /// A placement-probe word.
    Placement,
    /// The player asked for this word (a door from Reports, Calendar, Lists).
    Requested,
    /// A word the player planned for today (CC-CALENDAR).
    Planned,
    /// The learner model promoted it within the band (L1 selection).
    LearnerPick,
    /// The ordinary no-repeat deck.
    Deck,
    /// "In my order" list play.
    InOrder,
    /// The Daily Challenge sequence.
    Daily,
}

/// Totals for the guardian view. Skill detail is [`LearnerQuery::skill_states`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReviewStats {
    /// Attempts in the bounded log (the log is a 512-entry ring).
    pub attempts: u32,
    pub correct: u32,
    /// Skill ids with mastery >= 0.8 after 2+ reviews.
    pub strengths: Vec<String>,
    /// Reviewed skill ids with mastery < 0.45, or more than 7 days overdue.
    pub focus: Vec<String>,
}

/// One tracked skill, as consumers may see it.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillView {
    pub id: String,
    /// BKT mastery, 0..=1.
    pub mastery: f64,
    /// Days past due today (negative = not yet due).
    pub overdue_days: i64,
    /// FSRS reviews so far. 0 = tracked but never reviewed.
    pub reps: u32,
}

/// A logged miss with what the player typed (grapheme-diagnosis material).
#[derive(Clone, Debug, PartialEq)]
pub struct MissView {
    pub word: String,
    pub typed: String,
    pub day: u32,
}

/// One logged attempt, without the typed text.
#[derive(Clone, Debug, PartialEq)]
pub struct AttemptView {
    pub word: String,
    pub day: u32,
    pub correct: bool,
}

#[cfg_attr(not(test), allow(dead_code))] // frozen surface: the R4 inspector and later phases call these
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementStatus {
    NotOffered,
    Skipped,
    Completed,
}

#[cfg_attr(not(test), allow(dead_code))] // frozen surface: the R4 inspector and later phases call these
pub trait LearnerQuery {
    /// Missed words due for review now, soonest-due first, at most `limit`.
    fn due_queue(&self, p: ProfileId, lang: &str, limit: usize) -> Vec<String>;
    /// Why `word` was served, if it is the word the engine served last.
    fn next_word_reason(&self, p: ProfileId, word: &str) -> Option<NextWordReason>;
    /// Missed words due within `horizon_days`, soonest first. Kid surfaces
    /// call it "Ready for a rematch", the guardian "At risk this week".
    fn at_risk_set(&self, p: ProfileId, lang: &str, horizon_days: f64) -> Vec<String>;
    fn review_stats(&self, p: ProfileId, lang: &str) -> ReviewStats;
    /// Every tracked skill, weakest first (ties by id).
    fn skill_states(&self, p: ProfileId, lang: &str) -> Vec<SkillView>;
    /// Logged typed misses, oldest first.
    fn miss_log(&self, p: ProfileId, lang: &str) -> Vec<MissView>;
    /// Logged attempts, oldest first.
    fn attempt_log(&self, p: ProfileId, lang: &str) -> Vec<AttemptView>;
    fn placement_status(&self, p: ProfileId, lang: &str) -> PlacementStatus;
}

// ------------------------------------------------------------ serve reasons

thread_local! {
    static LAST_SERVE: RefCell<Option<(String, NextWordReason)>> = const { RefCell::new(None) };
}

/// Engine side: the serve path records why it served `word`. Not part of
/// the read contract; consumers read it back through `next_word_reason`.
pub fn note_serve(word: &str, reason: NextWordReason) {
    LAST_SERVE.with(|c| *c.borrow_mut() = Some((word.to_string(), reason)));
}

#[cfg_attr(not(test), allow(dead_code))] // frozen surface: the R4 inspector and later phases call these
fn last_reason(word: &str) -> Option<NextWordReason> {
    LAST_SERVE.with(|c| c.borrow().as_ref().filter(|(w, _)| w == word).map(|(_, r)| *r))
}

// --------------------------------------------------------- the live version

/// The real implementation, over the misses queue in `AppState` and the
/// stored learner state. Build one per read: `learner_query::live(&state)`.
pub struct Live<'a> {
    misses: &'a [crate::model::MissEntry],
    now_ms: f64,
    day: u32,
}

pub fn live(state: &crate::model::AppState) -> Live<'_> {
    let now_ms = now_ms();
    Live { misses: &state.misses, now_ms, day: (now_ms / 86_400_000.0) as u32 }
}

// Off-wasm (host `cargo test`) there is no JS clock, and `js_sys::Date::now`
// panics. Same doctrine as `storage::storage()`: the host sees epoch 0, so
// pure logic reachable from a consumer can still run in unit tests.
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    0.0
}

/// Test-only constructor with injected time (js_sys panics off-wasm).
#[cfg(test)]
pub fn live_at(misses: &[crate::model::MissEntry], now_ms: f64, day: u32) -> Live<'_> {
    Live { misses, now_ms, day }
}

impl Live<'_> {
    fn due_within(&self, lang: &str, horizon_ms: f64) -> Vec<String> {
        let mut v: Vec<(f64, &str)> = self
            .misses
            .iter()
            .filter(|m| m.lang == lang && m.due <= self.now_ms + horizon_ms)
            .map(|m| (m.due, m.word.as_str()))
            .collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.1.cmp(b.1)));
        v.into_iter().map(|(_, w)| w.to_string()).collect()
    }
}

impl LearnerQuery for Live<'_> {
    fn due_queue(&self, _p: ProfileId, lang: &str, limit: usize) -> Vec<String> {
        let mut v = self.due_within(lang, 0.0);
        v.truncate(limit);
        v
    }
    fn next_word_reason(&self, _p: ProfileId, word: &str) -> Option<NextWordReason> {
        last_reason(word)
    }
    fn at_risk_set(&self, _p: ProfileId, lang: &str, horizon_days: f64) -> Vec<String> {
        self.due_within(lang, horizon_days * 86_400_000.0)
    }
    fn review_stats(&self, _p: ProfileId, lang: &str) -> ReviewStats {
        stats_of(&crate::learner::load_for(lang), self.day)
    }
    fn skill_states(&self, _p: ProfileId, lang: &str) -> Vec<SkillView> {
        skills_of(&crate::learner::load_for(lang), self.day)
    }
    fn miss_log(&self, _p: ProfileId, lang: &str) -> Vec<MissView> {
        crate::learner::load_for(lang)
            .log
            .iter()
            .filter(|a| !a.correct)
            .filter_map(|a| a.typed.as_ref().map(|t| MissView { word: a.word.clone(), typed: t.clone(), day: a.day }))
            .collect()
    }
    fn attempt_log(&self, _p: ProfileId, lang: &str) -> Vec<AttemptView> {
        crate::learner::load_for(lang)
            .log
            .iter()
            .map(|a| AttemptView { word: a.word.clone(), day: a.day, correct: a.correct })
            .collect()
    }
    fn placement_status(&self, _p: ProfileId, lang: &str) -> PlacementStatus {
        match crate::learner::load_for(lang).placed {
            None => PlacementStatus::NotOffered,
            Some(false) => PlacementStatus::Skipped,
            Some(true) => PlacementStatus::Completed,
        }
    }
}

/// The guardian totals, with the thresholds `learner::guardian_report` used.
fn stats_of(st: &crate::learner::LearnerState, day: u32) -> ReviewStats {
    let r = crate::learner::guardian_report(st, day);
    ReviewStats { attempts: r.attempts, correct: r.correct, strengths: r.strengths, focus: r.focus }
}

fn skills_of(st: &crate::learner::LearnerState, day: u32) -> Vec<SkillView> {
    let mut v: Vec<SkillView> = st
        .skills
        .iter()
        .map(|s| SkillView {
            id: s.id.clone(),
            mastery: s.mastery,
            overdue_days: day as i64 - s.fsrs.due_day as i64,
            reps: s.fsrs.reps,
        })
        .collect();
    v.sort_by(|a, b| a.mastery.partial_cmp(&b.mastery).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.id.cmp(&b.id)));
    v
}

// ------------------------------------------------------ the deterministic fake

/// Fixed fixture data for consumer tests (L0 R1: consumers build and test
/// against the fake, never the real implementation). Every field is public
/// so a test states exactly the learner it needs.
#[cfg_attr(not(test), allow(dead_code))] // the fake exists for tests
#[derive(Clone, Debug, Default)]
pub struct Fake {
    pub due: Vec<String>,
    pub at_risk: Vec<String>,
    pub reason: Option<(String, NextWordReason)>,
    pub stats: ReviewStats,
    pub skills: Vec<SkillView>,
    pub misses: Vec<MissView>,
    pub attempts: Vec<AttemptView>,
    pub placement: Option<PlacementStatus>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Fake {
    /// The standard fixture: a mid-progress English learner.
    pub fn mid_progress() -> Self {
        let sv = |id: &str, mastery: f64, overdue_days: i64, reps: u32| SkillView { id: id.into(), mastery, overdue_days, reps };
        Fake {
            due: vec!["receive".into(), "knight".into()],
            at_risk: vec!["receive".into(), "knight".into(), "rhythm".into()],
            reason: Some(("receive".into(), NextWordReason::DueReview)),
            stats: ReviewStats { attempts: 40, correct: 29, strengths: vec!["doubled_consonant".into()], focus: vec!["silent_letters".into()] },
            skills: vec![
                sv("silent_letters", 0.31, 9, 6),
                sv("unstressed_vowel_ambiguity", 0.52, -2, 3),
                sv("doubled_consonant", 0.86, -5, 5),
            ],
            misses: vec![
                MissView { word: "receive".into(), typed: "recieve".into(), day: 20400 },
                MissView { word: "knight".into(), typed: "nite".into(), day: 20401 },
            ],
            attempts: vec![
                AttemptView { word: "receive".into(), day: 20400, correct: false },
                AttemptView { word: "rabbit".into(), day: 20400, correct: true },
                AttemptView { word: "knight".into(), day: 20401, correct: false },
            ],
            placement: Some(PlacementStatus::Completed),
        }
    }
}

impl LearnerQuery for Fake {
    fn due_queue(&self, _p: ProfileId, _lang: &str, limit: usize) -> Vec<String> {
        self.due.iter().take(limit).cloned().collect()
    }
    fn next_word_reason(&self, _p: ProfileId, word: &str) -> Option<NextWordReason> {
        self.reason.as_ref().filter(|(w, _)| w == word).map(|(_, r)| *r)
    }
    fn at_risk_set(&self, _p: ProfileId, _lang: &str, _horizon_days: f64) -> Vec<String> {
        self.at_risk.clone()
    }
    fn review_stats(&self, _p: ProfileId, _lang: &str) -> ReviewStats {
        self.stats.clone()
    }
    fn skill_states(&self, _p: ProfileId, _lang: &str) -> Vec<SkillView> {
        self.skills.clone()
    }
    fn miss_log(&self, _p: ProfileId, _lang: &str) -> Vec<MissView> {
        self.misses.clone()
    }
    fn attempt_log(&self, _p: ProfileId, _lang: &str) -> Vec<AttemptView> {
        self.attempts.clone()
    }
    fn placement_status(&self, _p: ProfileId, _lang: &str) -> PlacementStatus {
        self.placement.unwrap_or(PlacementStatus::NotOffered)
    }
}

// ------------------------------------------------------ I6: the boundary scan

/// What a consumer may not touch: learner storage and its internals. The
/// pre-contract read path (`rematch_set`) is listed so it can't come back.
const FORBIDDEN: &[&str] = &[
    "learner::load_for",
    "LearnerState",
    "guardian_report(",
    ".fsrs.",
    "spell_learner_",
    "rematch_set(",
];

/// Files allowed to read learner storage directly, each with its reason.
const ALLOWED: &[(&str, &str)] = &[
    ("learner.rs", "the implementation"),
    ("learner_query.rs", "the contract and its live implementation"),
    ("game.rs", "the engine: records attempts and selects within the band (writer side, not a consumer)"),
    ("review.rs", "the one review rule (R2): schedules queued words with the crate's FSRS"),
];

/// Violations in one file's source. Comment lines are skipped: naming a
/// symbol in prose is not reading storage.
#[cfg_attr(not(test), allow(dead_code))]
fn boundary_violations(file: &str, src: &str) -> Vec<String> {
    let name = file.rsplit('/').next().unwrap_or(file);
    if ALLOWED.iter().any(|(f, _)| *f == name) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        for tok in FORBIDDEN {
            if code.contains(tok) {
                out.push(format!("{file}:{}: `{tok}` — read learner data through LearnerQuery (L0 I6)", i + 1));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MissEntry;

    fn miss(word: &str, lang: &str, due: f64) -> MissEntry {
        MissEntry { word: word.into(), lang: lang.into(), tier: "easy".into(), misses: 1, box_: 1, due, ts: 0.0, review: None }
    }

    // -- the live implementation ---------------------------------------------

    #[test]
    fn due_queue_and_at_risk_read_the_misses_queue_soonest_first() {
        let day = 86_400_000.0;
        let now = 100.0 * day;
        let m = vec![
            miss("later", "en", now + 2.0 * day), // outside a 1.5-day horizon
            miss("soon", "en", now + 0.5 * day),
            miss("overdue", "en", now - day),
            miss("dueNow", "en", now),
            miss("otherLang", "es", now - day),
        ];
        let q = live_at(&m, now, 100);
        assert_eq!(q.due_queue(DEVICE, "en", 10), vec!["overdue", "dueNow"]);
        assert_eq!(q.due_queue(DEVICE, "en", 1), vec!["overdue"], "limit applies");
        assert_eq!(q.at_risk_set(DEVICE, "en", 1.5), vec!["overdue", "dueNow", "soon"]);
        assert_eq!(q.at_risk_set(DEVICE, "es", 1.5), vec!["otherLang"], "language-scoped");
    }

    #[test]
    fn at_risk_matches_the_rematch_set_it_replaces() {
        // rematch_set's old rule: lang matches and due <= now + horizon_ms,
        // soonest first. The 36-hour window every surface used is 1.5 days.
        let now = 5_000_000_000.0;
        let m = vec![miss("a", "en", now + 36.0 * 3600.0 * 1000.0), miss("b", "en", now + 36.0 * 3600.0 * 1000.0 + 1.0)];
        assert_eq!(live_at(&m, now, 0).at_risk_set(DEVICE, "en", 1.5), vec!["a"], "the boundary is inclusive, as before");
    }

    #[test]
    fn same_state_same_day_same_queue() {
        // I7: equal due times break ties by word, so the order never depends
        // on insertion order.
        let now = 1e9;
        let a = vec![miss("zeta", "en", now - 1.0), miss("alpha", "en", now - 1.0)];
        let b = vec![miss("alpha", "en", now - 1.0), miss("zeta", "en", now - 1.0)];
        assert_eq!(live_at(&a, now, 0).due_queue(DEVICE, "en", 9), live_at(&b, now, 0).due_queue(DEVICE, "en", 9));
    }

    #[test]
    fn next_word_reason_answers_only_for_the_word_served() {
        let q = live_at(&[], 0.0, 0);
        note_serve("knight", NextWordReason::DueReview);
        assert_eq!(q.next_word_reason(DEVICE, "knight"), Some(NextWordReason::DueReview));
        assert_eq!(q.next_word_reason(DEVICE, "rabbit"), None, "not the word just served");
        note_serve("rabbit", NextWordReason::Planned);
        assert_eq!(q.next_word_reason(DEVICE, "rabbit"), Some(NextWordReason::Planned));
        assert_eq!(q.next_word_reason(DEVICE, "knight"), None, "only the latest serve answers");
    }

    #[test]
    fn cold_start_reads_are_empty_and_never_error() {
        // I8: host tests have no storage, which is exactly a new profile.
        let q = live_at(&[], 0.0, 0);
        assert!(q.skill_states(DEVICE, "en").is_empty());
        assert!(q.miss_log(DEVICE, "en").is_empty());
        assert!(q.attempt_log(DEVICE, "en").is_empty());
        assert_eq!(q.review_stats(DEVICE, "en"), ReviewStats::default());
        assert_eq!(q.placement_status(DEVICE, "en"), PlacementStatus::NotOffered);
    }

    #[test]
    fn skill_view_matches_the_stored_state() {
        let mut st = crate::learner::LearnerState::new("en");
        st.record(crate::learner::Attempt {
            day: 10, word: "knight".into(), skills: vec!["silent_letters".into()],
            correct: false, channel: crate::learner::Channel::Typed, typed: Some("nite".into()),
        });
        let s = &st.skills[0];
        let v = skills_of(&st, 30);
        assert_eq!(v, vec![SkillView { id: "silent_letters".into(), mastery: s.mastery, overdue_days: 30 - s.fsrs.due_day as i64, reps: 1 }]);
        let r = stats_of(&st, 30);
        assert_eq!((r.attempts, r.correct), (1, 0));
    }

    // -- fake parity: consumers against the fake only (acceptance 2) ----------

    #[test]
    fn reports_trap_mastery_against_the_fake() {
        let got = crate::reports::trap_mastery(&Fake::mid_progress(), "en");
        let m = |id: &str| got.iter().find(|(t, _)| t == id).map(|(_, p)| *p);
        assert_eq!(m("silent_letters"), Some(31));
        assert_eq!(m("doubled_consonant"), Some(86));
        assert_eq!(m("loanword_spelling"), Some(0), "an untracked taxonomy skill shows 0, not absent");
    }

    #[test]
    fn reports_confusion_pairs_against_the_fake() {
        // recieve/receive is a transposition; nite/knight is not a
        // substitution or transposition, so it contributes nothing.
        let pairs = crate::reports::confusion_pairs(&Fake::mid_progress(), "en");
        assert_eq!(pairs.len(), 1, "{pairs:?}");
        assert_eq!(pairs[0].2, 1);
    }

    #[test]
    fn calendar_overdue_rule_against_the_fake() {
        let skills = Fake::mid_progress().skills;
        // knight exercises silent_letters: reviewed and 9 days overdue.
        assert!(crate::calendar::window_has_overdue(&skills, &["knight".into()], "en"));
        // A word exercising no overdue skill leaves the plan free.
        assert!(!crate::calendar::window_has_overdue(&skills, &["cat".into()], "en"));
        // An overdue skill that was never reviewed doesn't count.
        let unreviewed = vec![SkillView { id: "silent_letters".into(), mastery: 0.25, overdue_days: 40, reps: 0 }];
        assert!(!crate::calendar::window_has_overdue(&unreviewed, &["knight".into()], "en"));
    }

    #[test]
    fn guardian_view_against_the_fake() {
        let stats = Fake::mid_progress().review_stats(DEVICE, "en");
        let html = crate::learner::guardian_report_html(&stats, "en");
        // Skill names render through the audited i18n pool (skill.<id>).
        let name = |id: &str| crate::i18n::t(&format!("skill.{id}"));
        assert!(html.contains(&format!("<ul class=\"g-strong\"><li>{}</li>", name("doubled_consonant"))), "{html}");
        assert!(html.contains(&format!("<ul class=\"g-focus\"><li>{}</li>", name("silent_letters"))), "{html}");
        assert!(html.contains("40") && html.contains("29"), "totals: {html}");
        let empty = crate::learner::guardian_report_html(&ReviewStats::default(), "en");
        assert!(empty.contains("g-empty"), "a fresh learner renders the empty state");
    }

    #[test]
    fn placement_status_against_the_fake() {
        assert_eq!(Fake::mid_progress().placement_status(DEVICE, "en"), PlacementStatus::Completed);
        assert_eq!(Fake::default().placement_status(DEVICE, "en"), PlacementStatus::NotOffered);
    }

    // -- I6: the boundary scan ---------------------------------------------------

    fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    #[test]
    fn i6_no_consumer_reads_learner_storage_directly() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rs_files(&root, &mut files);
        assert!(files.len() > 50, "the scan must actually see the source tree");
        let mut bad = Vec::new();
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_string_lossy().to_string();
            bad.extend(boundary_violations(&rel, &std::fs::read_to_string(&f).unwrap()));
        }
        assert!(bad.is_empty(), "direct learner-storage access outside the contract:\n{}", bad.join("\n"));
    }

    #[test]
    fn i6_scan_catches_a_planted_violation() {
        // A scan that cannot fail reports success forever.
        for planted in [
            "    let st = crate::learner::load_for(&lang);",
            "    let r = crate::learner::guardian_report(&st, day);",
            "fn f(st: &crate::learner::LearnerState) {}",
            "    if s.fsrs.reps > 0 {",
            "    storage::get_raw(\"spell_learner_en\");",
            "    let due = crate::reports::rematch_set(&s, &lang, now, h);",
        ] {
            assert!(!boundary_violations("guardian_dash.rs", planted).is_empty(), "missed: {planted}");
        }
        // ... and stays quiet where it should.
        assert!(boundary_violations("guardian_dash.rs", "    // learner::load_for used to live here").is_empty(), "comments are prose");
        assert!(boundary_violations("learner.rs", "let st = load_for(lang); // learner::load_for").is_empty(), "the implementation is allowed");
        assert!(boundary_violations("stats.rs", "let r = q.review_stats(DEVICE, lang);").is_empty());
    }
}
