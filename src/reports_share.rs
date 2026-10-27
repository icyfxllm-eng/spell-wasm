//! CC-REPORTS-SHARE v1 — the parent report composer (F1, F2, F4) and its
//! text renderer (F3). Pure: the clock, the time zone, the translator and the
//! date format are all injected, so the same inputs give the same bytes (R-I7)
//! and every rule here is a host test. Read-only over `LearnerQuery` (R-I1);
//! no storage, DOM or network (R-I2). The screen is `reports_share_ui.rs`.

use std::collections::{BTreeMap, BTreeSet};

use crate::learner::{Channel, OutcomeClass};
use crate::learner_query::{LearnerQuery, ProfileId};

const DAY_S: i64 = 86_400;

// ------------------------------------------------------------------ time

/// A time zone, as seconds east of UTC at a given UTC instant. Injected: the
/// live one asks the device (`DeviceZone`), tests name a rule.
pub trait Zone {
    fn offset_s(&self, utc_s: i64) -> i64;
}

/// The device's CURRENT zone, read at compose time (D-R6).
pub struct DeviceZone;

impl Zone for DeviceZone {
    fn offset_s(&self, utc_s: i64) -> i64 {
        let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(utc_s as f64 * 1000.0));
        // getTimezoneOffset is minutes WEST of UTC.
        -(d.get_timezone_offset() as i64) * 60
    }
}

/// (year, month 1..=12, day 1..=31) of a day count since 1970-01-01.
/// Howard Hinnant's civil_from_days, valid far beyond any date this app sees.
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The day count of a civil date (inverse of [`civil`]).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The local Monday (as a day count) of the week holding `utc_s`. Weeks start
/// on Monday in every locale (D-R6); 1970-01-01 was a Thursday.
pub fn week_monday(utc_s: i64, zone: &dyn Zone) -> i64 {
    let local_day = (utc_s + zone.offset_s(utc_s)).div_euclid(DAY_S);
    local_day - (local_day + 3).rem_euclid(7)
}

/// Which week a report covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    /// The Monday–Sunday week that has closed (the default).
    LastWeek,
    /// The week so far, labelled partial.
    ThisWeek,
}

// ---------------------------------------------------------------- report

pub struct ComposeInput<'a> {
    pub q: &'a dyn LearnerQuery,
    pub profile: ProfileId,
    pub langs: &'a [&'a str],
    pub now_s: i64,
    pub zone: &'a dyn Zone,
    pub which: Which,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangSection {
    pub lang: String,
    pub mastered: Vec<String>,
    pub working_on: Vec<String>,
    pub new_words: Vec<String>,
}

impl LangSection {
    fn is_empty(&self) -> bool {
        self.mastered.is_empty() && self.working_on.is_empty() && self.new_words.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The window's Monday, as a local day count.
    pub monday: i64,
    pub partial: bool,
    /// Languages with something to say, in the order asked for. Empty = an
    /// empty week, which still renders (D-R7).
    pub sections: Vec<LangSection>,
}

/// A miss-class outcome: Working on (F2).
fn is_miss(o: OutcomeClass) -> bool {
    matches!(o, OutcomeClass::Misspelled | OutcomeClass::TimedOut | OutcomeClass::GaveUp | OutcomeClass::Trap)
}

/// F1/F2 — compose one window. Read-only, contract-only (L0 I6).
pub fn compose(input: &ComposeInput) -> Report {
    let this_monday = week_monday(input.now_s, input.zone);
    let (monday, partial) = match input.which {
        Which::LastWeek => (this_monday - 7, false),
        Which::ThisWeek => (this_monday, true),
    };
    let in_window = |utc_s: i64| week_monday(utc_s, input.zone) == monday;
    let mut sections = Vec::new();
    for &lang in input.langs {
        let log = input.q.attempt_detail_log(input.profile, lang);
        // Speech and Impossible attempts are excluded from every bucket; an
        // attempt without its CR-1 instant or class is unknown, never guessed.
        let eligible = || log.iter().filter(|a| a.channel != Channel::Speech && !a.impossible);

        let mastered: BTreeSet<String> = input
            .q
            .redemptions(input.profile, lang)
            .into_iter()
            .filter(|r| in_window((r.mastered_ms / 1000.0).floor() as i64))
            .map(|r| r.word)
            .collect();

        let working: BTreeSet<String> = eligible()
            .filter(|a| matches!((a.ts, a.outcome), (Some(ts), Some(o)) if is_miss(o) && in_window(ts as i64)))
            .map(|a| a.word.clone())
            .filter(|w| !mastered.contains(w))
            .collect();

        // New: every logged attempt of the word is known, and the earliest is
        // in the window. An unknown attempt (pre-CR-1) may be an earlier
        // meeting, so that word is not new — show less.
        let mut first: BTreeMap<&str, Option<i64>> = BTreeMap::new();
        for a in eligible() {
            // Unknown = no instant or no class (CR-1): it proves a meeting at
            // some time we cannot place.
            let known = a.ts.filter(|_| a.outcome.is_some()).map(|t| t as i64);
            let e = first.entry(a.word.as_str()).or_insert(known);
            *e = match (*e, known) {
                (Some(cur), Some(t)) => Some(cur.min(t)),
                _ => None,
            };
        }
        let new_words: BTreeSet<String> = first
            .into_iter()
            .filter(|(_, ts)| ts.is_some_and(in_window))
            .map(|(w, _)| w.to_string())
            .filter(|w| !mastered.contains(w) && !working.contains(w))
            .collect();

        let s = LangSection {
            lang: lang.to_string(),
            mastered: mastered.into_iter().collect(),
            working_on: working.into_iter().collect(),
            new_words: new_words.into_iter().collect(),
        };
        if !s.is_empty() {
            sections.push(s);
        }
    }
    Report { monday, partial, sections }
}

// ---------------------------------------------------------------- render

/// What the renderer may say, and how dates read. Injected so the renderer is
/// pure: the live one is i18n plus Intl in the UI language.
pub struct Voice<'a> {
    /// A `rshare.*` key to its text. The renderer asks for nothing else (R-I4).
    pub t: &'a dyn Fn(&str) -> String,
    /// (year, month, day, with_year) to a date in the UI language's format.
    pub date: &'a dyn Fn(i64, u32, u32, bool) -> String,
}

/// A language's display name: the fixed native-name table, never free text.
pub fn lang_name(lang: &str) -> &str {
    crate::consts::BUILTIN_LANGS.iter().find(|(c, _, _, _)| *c == lang).map(|(_, n, _, _)| *n).unwrap_or(lang)
}

fn fill(s: String, params: &[(&str, &str)]) -> String {
    params.iter().fold(s, |s, (k, v)| s.replace(&format!("{{{k}}}"), v))
}

/// F3 — the report as plain text, for the share sheet. Every word of prose is
/// a `rshare.*` key; the rest is bank words, language names, dates and counts.
pub fn render_text(r: &Report, v: &Voice) -> String {
    let (sy, sm, sd) = civil(r.monday);
    let (ey, em, ed) = civil(r.monday + 6);
    let mut out = vec![
        (v.t)("rshare.title"),
        fill((v.t)("rshare.week"), &[("start", &(v.date)(sy, sm, sd, false)), ("end", &(v.date)(ey, em, ed, true))]),
    ];
    if r.partial {
        out.push((v.t)("rshare.partial"));
    }
    if r.sections.is_empty() {
        out.push(String::new());
        out.push((v.t)("rshare.empty"));
    }
    for s in &r.sections {
        out.push(String::new());
        out.push(lang_name(&s.lang).to_string());
        for (key, words) in [("rshare.mastered", &s.mastered), ("rshare.workingOn", &s.working_on), ("rshare.new", &s.new_words)] {
            if words.is_empty() {
                continue;
            }
            let head = fill((v.t)(key), &[("n", &words.len().to_string())]);
            out.push(format!("{head}: {}", words.join(", ")));
        }
    }
    out.join("\n")
}

// ----------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learner_query::{AttemptDetail, Fake, RedemptionView, DEVICE};

    /// America/Indiana/Indianapolis: US Eastern rules since 2006. EDT from the
    /// second Sunday of March 02:00 EST (07:00 UTC) to the first Sunday of
    /// November 02:00 EDT (06:00 UTC).
    struct Indianapolis;

    struct Utc;

    impl Zone for Utc {
        fn offset_s(&self, _: i64) -> i64 {
            0
        }
    }

    fn nth_sunday(y: i64, m: u32, n: i64) -> i64 {
        let first = days_from_civil(y, m, 1);
        let to_sun = (6 - (first + 3).rem_euclid(7)).rem_euclid(7);
        first + to_sun + 7 * (n - 1)
    }

    impl Zone for Indianapolis {
        fn offset_s(&self, utc_s: i64) -> i64 {
            let (y, _, _) = civil(utc_s.div_euclid(DAY_S));
            let start = nth_sunday(y, 3, 2) * DAY_S + 7 * 3600;
            let end = nth_sunday(y, 11, 1) * DAY_S + 6 * 3600;
            if (start..end).contains(&utc_s) { -4 * 3600 } else { -5 * 3600 }
        }
    }

    /// UTC seconds of a local wall time in Indianapolis, for a given offset.
    fn at(y: i64, m: u32, d: u32, h: i64, mi: i64, s: i64, offset_h: i64) -> i64 {
        days_from_civil(y, m, d) * DAY_S + h * 3600 + mi * 60 + s - offset_h * 3600
    }

    fn det(word: &str, ts: Option<i64>, outcome: Option<OutcomeClass>) -> AttemptDetail {
        AttemptDetail {
            word: word.into(),
            word_id: format!("en::{word}"),
            day: ts.map(|t| (t / DAY_S) as u32).unwrap_or(0),
            ts: ts.map(|t| t as u64),
            outcome,
            channel: Channel::Typed,
            mode: Some("standard".into()),
            impossible: false,
        }
    }

    const OCT19: (i64, u32, u32) = (2026, 10, 19);
    const OCT26: (i64, u32, u32) = (2026, 10, 26);

    fn monday_of(d: (i64, u32, u32)) -> i64 {
        days_from_civil(d.0, d.1, d.2)
    }

    #[test]
    fn the_calendar_helpers_agree() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(days_from_civil(2026, 10, 25)), (2026, 10, 25));
        assert_eq!((days_from_civil(2026, 10, 19) + 3).rem_euclid(7), 0, "Oct 19 2026 is a Monday");
        assert_eq!(nth_sunday(2026, 11, 1), days_from_civil(2026, 11, 1));
        assert_eq!(nth_sunday(2026, 3, 2), days_from_civil(2026, 3, 8));
    }

    /// A2 (window): Sunday 23:59:59 local closes the week of Oct 19; Monday
    /// 00:00:00 opens the week of Oct 26.
    #[test]
    fn the_week_turns_at_local_monday_midnight() {
        let z = Indianapolis;
        assert_eq!(week_monday(at(2026, 10, 25, 23, 59, 59, -4), &z), monday_of(OCT19));
        assert_eq!(week_monday(at(2026, 10, 26, 0, 0, 0, -4), &z), monday_of(OCT26));
        // The same instants in UTC sit in a different place: the zone matters.
        assert_eq!(week_monday(at(2026, 10, 25, 23, 59, 59, -4), &Utc), monday_of(OCT26));
    }

    /// A2 (DST): 01:30 happens twice on Sun 2026-11-01. Both instants are in
    /// the week of Oct 26, and a word answered at each counts once.
    #[test]
    fn the_fall_back_hour_is_two_instants_in_one_week() {
        let z = Indianapolis;
        let edt = at(2026, 11, 1, 1, 30, 0, -4);
        let est = at(2026, 11, 1, 1, 30, 0, -5);
        assert_eq!(est - edt, 3600);
        assert_eq!((z.offset_s(edt), z.offset_s(est)), (-4 * 3600, -5 * 3600));
        assert_eq!(week_monday(edt, &z), monday_of(OCT26));
        assert_eq!(week_monday(est, &z), monday_of(OCT26));
        let fake = Fake {
            details: vec![det("rhythm", Some(edt), Some(OutcomeClass::Misspelled)), det("rhythm", Some(est), Some(OutcomeClass::Misspelled))],
            ..Fake::default()
        };
        let r = compose(&ComposeInput { q: &fake, profile: DEVICE, langs: &["en"], now_s: at(2026, 11, 2, 9, 0, 0, -5), zone: &z, which: Which::LastWeek });
        assert_eq!(r.monday, monday_of(OCT26));
        assert_eq!(r.sections[0].working_on, ["rhythm"]);
    }

    fn week_fixture() -> Fake {
        let mon = at(2026, 10, 19, 9, 0, 0, -4);
        let before = at(2026, 10, 12, 9, 0, 0, -4);
        let mut speech = det("spoken", Some(mon), Some(OutcomeClass::Misspelled));
        speech.channel = Channel::Speech;
        let mut hard = det("impossibly", Some(mon), Some(OutcomeClass::Misspelled));
        hard.impossible = true;
        Fake {
            details: vec![
                det("because", Some(mon), Some(OutcomeClass::Misspelled)), // mastered AND missed
                det("receive", Some(mon), Some(OutcomeClass::Misspelled)), // working on
                det("receive", Some(mon + 60), Some(OutcomeClass::Correct)),
                det("knight", Some(before), Some(OutcomeClass::Correct)), // met last week
                det("knight", Some(mon), Some(OutcomeClass::Correct)),
                det("rabbit", Some(mon), Some(OutcomeClass::Correct)), // new
                det("pair", Some(mon), Some(OutcomeClass::OtherSense)), // new, other sense is correct
                det("legacy", None, None), // pre-CR-1 meeting...
                det("legacy", Some(mon), Some(OutcomeClass::Correct)), // ...so not new
                det("unknown", Some(mon), None), // no class: unknown, so in no bucket
                speech,
                hard,
            ],
            redeemed: vec![
                RedemptionView { word: "because".into(), mastered_ms: (mon + 3600) as f64 * 1000.0 },
                RedemptionView { word: "old".into(), mastered_ms: before as f64 * 1000.0 },
            ],
            ..Fake::default()
        }
    }

    fn compose_week(fake: &Fake) -> Report {
        compose(&ComposeInput {
            q: fake,
            profile: DEVICE,
            langs: &["en"],
            now_s: at(2026, 10, 27, 8, 0, 0, -4),
            zone: &Indianapolis,
            which: Which::LastWeek,
        })
    }

    /// A3: one bucket per word, Mastered beats Working on beats New; Speech,
    /// Impossible and unknown attempts appear nowhere.
    #[test]
    fn each_word_lands_in_at_most_one_bucket_and_the_excluded_nowhere() {
        let r = compose_week(&week_fixture());
        assert_eq!((r.monday, r.partial), (monday_of(OCT19), false));
        assert_eq!(r.sections.len(), 1);
        // A language with nothing in the window gets no section at all.
        let quiet = Fake::default();
        let none = compose(&ComposeInput { q: &quiet, profile: DEVICE, langs: &["en", "es"], now_s: at(2026, 10, 27, 8, 0, 0, -4), zone: &Indianapolis, which: Which::LastWeek });
        assert!(none.sections.is_empty());
        let s = &r.sections[0];
        assert_eq!(s.mastered, ["because"]);
        assert_eq!(s.working_on, ["receive"]);
        assert_eq!(s.new_words, ["pair", "rabbit"]);
        let all: Vec<&String> = s.mastered.iter().chain(&s.working_on).chain(&s.new_words).collect();
        for gone in ["spoken", "impossibly", "legacy", "knight", "old", "unknown"] {
            assert!(!all.iter().any(|w| *w == gone), "{gone} must not appear");
        }
    }

    #[test]
    fn this_week_so_far_is_the_partial_window() {
        let fake = week_fixture();
        let r = compose(&ComposeInput { q: &fake, profile: DEVICE, langs: &["en"], now_s: at(2026, 10, 21, 8, 0, 0, -4), zone: &Indianapolis, which: Which::ThisWeek });
        assert_eq!((r.monday, r.partial), (monday_of(OCT19), true));
        assert!(!r.sections.is_empty());
    }

    // -- rendering ----------------------------------------------------------

    fn locale(lang: &str) -> serde_json::Map<String, serde_json::Value> {
        let raw = match lang {
            "en" => include_str!("i18n/locales/en.json"),
            "ar" => include_str!("i18n/locales/ar.json"),
            "de" => include_str!("i18n/locales/de.json"),
            "es" => include_str!("i18n/locales/es.json"),
            "fil" => include_str!("i18n/locales/fil.json"),
            "fr" => include_str!("i18n/locales/fr.json"),
            "hi" => include_str!("i18n/locales/hi.json"),
            "ja" => include_str!("i18n/locales/ja.json"),
            "ko" => include_str!("i18n/locales/ko.json"),
            "pl" => include_str!("i18n/locales/pl.json"),
            "pt" => include_str!("i18n/locales/pt.json"),
            "ru" => include_str!("i18n/locales/ru.json"),
            "sw" => include_str!("i18n/locales/sw.json"),
            "vi" => include_str!("i18n/locales/vi.json"),
            "zh" => include_str!("i18n/locales/zh.json"),
            other => panic!("no locale fixture for {other}"),
        };
        serde_json::from_str::<serde_json::Value>(raw).unwrap().as_object().unwrap().clone()
    }

    pub(super) const LOCALES: [&str; 15] = ["en", "ar", "de", "es", "fil", "fr", "hi", "ja", "ko", "pl", "pt", "ru", "sw", "vi", "zh"];

    fn iso(y: i64, m: u32, d: u32, with_year: bool) -> String {
        if with_year { format!("{y:04}-{m:02}-{d:02}") } else { format!("{m:02}-{d:02}") }
    }

    /// Render in `lang`, failing on any key outside `rshare.*` (R-I4).
    fn render_in(r: &Report, lang: &str) -> String {
        let loc = locale(lang);
        let t = |k: &str| -> String {
            assert!(k.starts_with("rshare."), "the renderer asked for a key outside rshare.*: {k}");
            loc.get(k).and_then(|v| v.as_str()).unwrap_or_else(|| panic!("{lang} lacks {k}")).to_string()
        };
        render_text(r, &Voice { t: &t, date: &iso })
    }

    /// A1: the same fixture renders byte-identically, run after run.
    #[test]
    fn the_report_is_byte_identical_across_runs() {
        let fake = week_fixture();
        let first = render_in(&compose_week(&fake), "en");
        for _ in 0..3 {
            assert_eq!(render_in(&compose_week(&fake), "en"), first);
        }
        assert_eq!(
            first,
            "Spelling progress\n10-19 – 2026-10-25\n\nEnglish\nMastered (1): because\nWorking on (1): receive\nNew this week (2): pair, rabbit"
        );
    }

    /// A8 / D-R7: an empty week is a short report, not nothing.
    #[test]
    fn an_empty_week_is_a_short_report() {
        let r = compose_week(&Fake::default());
        assert!(r.sections.is_empty());
        assert_eq!(render_in(&r, "en"), "Spelling progress\n10-19 – 2026-10-25\n\nNo practice this week yet.");
    }

    /// What is left of a render once every rshare.* string, bank word,
    /// language name, date and count is taken out. Must be only separators.
    pub(super) fn free_text_residue(rendered: &str, r: &Report, lang: &str) -> String {
        let loc = locale(lang);
        let mut rest = rendered.to_string();
        let mut pieces: Vec<String> = Vec::new();
        for (k, v) in &loc {
            if k.starts_with("rshare.") {
                for part in v.as_str().unwrap().split(&['{', '}'][..]) {
                    pieces.push(part.to_string());
                }
            }
        }
        for s in &r.sections {
            pieces.push(lang_name(&s.lang).to_string());
            pieces.extend(s.mastered.iter().chain(&s.working_on).chain(&s.new_words).cloned());
        }
        pieces.retain(|p| p.trim().len() > 1);
        pieces.sort_by_key(|p| std::cmp::Reverse(p.len()));
        for p in pieces {
            rest = rest.replace(p.trim(), "");
        }
        rest.chars().filter(|c| !c.is_ascii_digit() && !" \n:,-–()".contains(*c)).collect()
    }

    /// A4: nothing reaches the text that is not a key, a word, a name, a date
    /// or a count — in every language — and a planted string is caught.
    #[test]
    fn the_renderer_speaks_only_the_closed_lexicon() {
        let r = compose_week(&week_fixture());
        for lang in LOCALES {
            let text = render_in(&r, lang);
            assert_eq!(free_text_residue(&text, &r, lang), "", "{lang} rendered free text:\n{text}");
        }
        let planted = format!("{}\nGreat job!", render_in(&r, "en"));
        assert_ne!(free_text_residue(&planted, &r, "en"), "", "the residue check must catch a planted string");
    }

    /// A file's code, without its test module (which names what it forbids).
    fn code_of(src: &str) -> &str {
        src.find("#[cfg(test)]\nmod ").map(|i| &src[..i]).unwrap_or(src)
    }

    // -- F3 prohibited lists (D-R8) -----------------------------------------

    const PROHIBITED: &str = include_str!("../config/report-prohibited.json");

    /// Every problem the prohibited lint finds in `text` for `lang`.
    pub(super) fn prohibited_hits(lists: &serde_json::Value, lang: &str, text: &str) -> Result<Vec<String>, String> {
        let entry = lists["langs"].get(lang).ok_or_else(|| format!("{lang} has no prohibited list"))?;
        let low = text.to_lowercase();
        Ok(entry["tokens"].as_array().unwrap().iter().filter_map(|t| t.as_str()).filter(|t| low.contains(&t.to_lowercase())).map(String::from).collect())
    }

    /// A5: no rshare.* string, and no rendered report, says a prohibited word
    /// in its language; every language has a list; and the lint bites.
    #[test]
    fn no_report_text_says_a_prohibited_word() {
        let lists: serde_json::Value = serde_json::from_str(PROHIBITED).unwrap();
        let r = compose_week(&week_fixture());
        for lang in LOCALES {
            for (k, v) in locale(lang) {
                if k.starts_with("rshare.") {
                    let hits = prohibited_hits(&lists, lang, v.as_str().unwrap()).unwrap();
                    assert!(hits.is_empty(), "{lang}/{k} says {hits:?}");
                }
            }
            let hits = prohibited_hits(&lists, lang, &render_in(&r, lang)).unwrap();
            assert!(hits.is_empty(), "the {lang} report says {hits:?}");
        }
        // Planted: G-D7's word is caught, and a language with no list fails.
        assert_eq!(prohibited_hits(&lists, "en", "Words she Struggled with").unwrap(), ["struggled", "struggle"]);
        assert!(prohibited_hits(&lists, "xx", "anything").is_err());
    }

    /// The lists say honestly what they are: en is native, every other
    /// language is a flagged English-seed draft until its native pass.
    #[test]
    fn the_prohibited_lists_flag_their_drafts() {
        let lists: serde_json::Value = serde_json::from_str(PROHIBITED).unwrap();
        for lang in LOCALES {
            let st = lists["langs"][lang]["status"].as_str().unwrap_or_else(|| panic!("{lang} has no list status"));
            assert!(st == "native" || st == "draft-en-seed", "{lang}: unknown status {st}");
            if st == "draft-en-seed" {
                assert_eq!(lists["langs"][lang]["tokens"], lists["langs"]["en"]["tokens"], "{lang} says draft-en-seed but is not the seed");
            }
        }
        assert_eq!(lists["langs"]["en"]["status"], "native");
    }

    /// R-I2: the composer and its screen reach no network.
    #[test]
    fn the_module_has_no_egress() {
        for (f, src) in [("reports_share.rs", include_str!("reports_share.rs")), ("reports_share_ui.rs", include_str!("reports_share_ui.rs"))] {
            let code = code_of(src);
            for bad in ["fetch(", "XmlHttpRequest", "crate::api", "online_net", "web_sys::Request", "WebSocket", "spawn_local", "sendBeacon"] {
                let hit = code.lines().filter(|l| !l.trim_start().starts_with("//")).any(|l| l.contains(bad));
                assert!(!hit, "{f} names {bad}");
            }
        }
    }

    /// R-I1: read-only. No write door is named by the composer or its screen.
    #[test]
    fn the_module_writes_no_learner_state() {
        for (f, src) in [("reports_share.rs", include_str!("reports_share.rs")), ("reports_share_ui.rs", include_str!("reports_share_ui.rs"))] {
            let code = code_of(src);
            for door in ["note_attempt", "note_built", "note_placement", "storage::set", "learner::reset", "add_miss", "set_json", "set_raw"] {
                let hit = code.lines().filter(|l| !l.trim_start().starts_with("//")).any(|l| l.contains(door));
                assert!(!hit, "{f} names the write door {door}");
            }
        }
    }
}
