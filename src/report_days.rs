//! CC-GUARDIAN-REPORT v1.1 — Report Days (1–7 day windows). Phase A.
//!
//! This file is the ONLY date math on the report path (invariant 7, A13):
//! the zone and calendar primitives, `window()`, and the F4/F5 rules. The
//! composer, the screen, the history list and the ping scheduler all ask it.
//! Pure: no clock reads, no storage, no DOM. The flag is `guardian.reportDays`;
//! nothing here is reached with the flag off, so v1 output cannot move (A2).

use std::collections::BTreeMap;

use crate::guardian_report::{primary_type, BankView, WordType};
use crate::learner_query::{LearnerQuery, ProfileId};

pub const DAY_S: i64 = 86_400;

// ------------------------------------------------------------------ zone

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

/// The local day count (days since 1970-01-01 in the zone) of a UTC instant.
pub fn local_day(utc_s: i64, zone: &dyn Zone) -> i64 {
    (utc_s + zone.offset_s(utc_s)).div_euclid(DAY_S)
}

/// The local day and the seconds into it.
pub fn local_day_sec(utc_s: i64, zone: &dyn Zone) -> (i64, i64) {
    let l = utc_s + zone.offset_s(utc_s);
    (l.div_euclid(DAY_S), l.rem_euclid(DAY_S))
}

/// 0 = Monday … 6 = Sunday. 1970-01-01 was a Thursday.
pub fn weekday(day: i64) -> u8 {
    (day + 3).rem_euclid(7) as u8
}

/// The local Monday (as a day count) of the week holding `utc_s`. Weeks start
/// on Monday in every locale (D-R6).
pub fn week_monday(utc_s: i64, zone: &dyn Zone) -> i64 {
    let d = local_day(utc_s, zone);
    d - i64::from(weekday(d))
}

/// UTC seconds of local midnight starting `day`. A DST day is 23 or 25 hours
/// and still one day (G-D28). Where midnight does not exist the first instant
/// of the day is used; where it happens twice, the earlier.
pub fn local_midnight_utc(day: i64, zone: &dyn Zone) -> i64 {
    let local = day * DAY_S;
    let mut t = local - zone.offset_s(local);
    for _ in 0..3 {
        let next = local - zone.offset_s(t);
        if next == t {
            break;
        }
        t = next;
    }
    // Step back over a doubled midnight so the earlier instant wins.
    while local_day(t - 1, zone) == day {
        t -= 1;
    }
    // And forward off a missing one.
    while local_day(t, zone) < day {
        t += 1;
    }
    t
}

/// Noon UTC of a day count, in ms: the anchor the UI hands `Intl` so a date
/// prints as that civil date in every zone.
pub fn day_noon_ms(day: i64) -> f64 {
    day as f64 * 86_400_000.0 + 43_200_000.0
}

// -------------------------------------------------------------- schedule

/// A non-empty set of weekdays (bit 0 = Monday … bit 6 = Sunday). G-D23:
/// report DAYS, never every-N-days. The constructor is the A12 model gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekdaySet(u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyWeekdaySet;

impl WeekdaySet {
    pub const WEEKLY: WeekdaySet = WeekdaySet(0b000_0001);
    pub const TWICE_A_WEEK: WeekdaySet = WeekdaySet(0b000_1001);
    pub const DAILY: WeekdaySet = WeekdaySet(0b111_1111);

    /// `Err` for an empty set (or no day in 0..=6): a schedule must name a day.
    pub fn new(days: &[u8]) -> Result<WeekdaySet, EmptyWeekdaySet> {
        let bits = days.iter().filter(|d| **d < 7).fold(0u8, |b, d| b | (1 << d));
        if bits == 0 {
            Err(EmptyWeekdaySet)
        } else {
            Ok(WeekdaySet(bits))
        }
    }

    pub fn contains(self, weekday: u8) -> bool {
        weekday < 7 && self.0 & (1 << weekday) != 0
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }
}

/// One edit of the schedule. `changed_at_s` is the UTC instant (G-D24).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleEntry {
    pub days: WeekdaySet,
    pub changed_at_s: i64,
}

/// The schedule's edit history, oldest first. The first entry anchors the
/// chain of windows; an edit takes effect at the next new report day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleLog(Vec<ScheduleEntry>);

impl ScheduleLog {
    /// The default (G-D1a): Weekly, Mondays, in force since `since_s`.
    pub fn weekly_since(since_s: i64) -> ScheduleLog {
        ScheduleLog(vec![ScheduleEntry { days: WeekdaySet::WEEKLY, changed_at_s: since_s }])
    }

    pub fn from_entries(mut entries: Vec<ScheduleEntry>) -> Option<ScheduleLog> {
        if entries.is_empty() {
            return None;
        }
        entries.sort_by_key(|e| e.changed_at_s);
        Some(ScheduleLog(entries))
    }

    /// Record an edit made at `at_s`.
    pub fn edit(&mut self, days: WeekdaySet, at_s: i64) {
        let at_s = at_s.max(self.0.last().map_or(at_s, |e| e.changed_at_s));
        self.0.push(ScheduleEntry { days, changed_at_s: at_s });
    }

    pub fn entries(&self) -> &[ScheduleEntry] {
        &self.0
    }

    /// The schedule in force at a UTC instant (the first entry before its own
    /// anchor, so the anchor day itself always has a schedule).
    fn in_force(&self, at_s: i64) -> WeekdaySet {
        self.0.iter().rev().find(|e| e.changed_at_s <= at_s).unwrap_or(&self.0[0]).days
    }
}

// ---------------------------------------------------------------- window

/// A report window `[start, end)` at local midnights, 1 to 7 local days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub start_s: i64,
    pub end_s: i64,
    /// The first local day inside it, and the last (inclusive).
    pub first_day: i64,
    pub last_day: i64,
}

impl Window {
    pub fn days(&self) -> i64 {
        self.last_day - self.first_day + 1
    }

    /// The composer's window for this span (F3).
    pub fn which(&self, partial: bool) -> crate::reports_share::Which {
        crate::reports_share::Which::Span {
            start_s: self.start_s,
            end_s: self.end_s,
            first_day: self.first_day,
            last_day: self.last_day,
            partial,
        }
    }
}

/// F2 — the latest closed scheduled window at `now_s`; `None` before the
/// schedule's first report day. Pure.
///
/// `end` is the most recent report-day local midnight at or before `now`.
/// `start` is the report-day midnight before it, each decided by the schedule
/// in force at that midnight, so an edit at `t` only matters from the next
/// midnight after `t` (G-D24): weekly→daily on Wed 14:00 after a Mon report
/// gives Mon 00:00 → Thu 00:00, nothing lost or counted twice.
///
/// G-D25 (7-day cap) is kept by a forced boundary at the 7th day after the
/// previous report day, never by clamping `start`, because a clamp would drop
/// attempts and break the one-window-per-attempt law (invariant 1).
///
/// `pinned_start_s` is the stored `end` of the report delivered just before
/// this window. If the device changed zone since, the recomputed start can sit
/// hours off it; within a day of the computed start the stored instant wins,
/// so no attempt is dropped or counted twice (A6).
pub fn window(now_s: i64, log: &ScheduleLog, zone: &dyn Zone, pinned_start_s: Option<i64>) -> Option<Window> {
    let first = log.0[0];
    let now_day = local_day(now_s, zone);
    // The chain starts on the first entry's report day at or before its edit.
    let mut anchor = local_day(first.changed_at_s, zone);
    for _ in 0..7 {
        if first.days.contains(weekday(anchor)) {
            break;
        }
        anchor -= 1;
    }
    // One entry: boundaries are periodic and independent of history, so only
    // the last fortnight need be walked. Several: walk from the anchor.
    let mut from = anchor;
    if log.0.len() == 1 && now_day - 14 > anchor {
        from = now_day - 14;
        while !first.days.contains(weekday(from)) {
            from -= 1;
        }
    }

    let mut ends: Vec<i64> = Vec::new();
    let mut last_boundary: Option<i64> = None;
    let mut d = from;
    while d <= now_day {
        let mid = local_midnight_utc(d, zone);
        if mid > now_s {
            break;
        }
        let natural = log.in_force(mid).contains(weekday(d));
        let forced = last_boundary.is_some_and(|l| d - l >= 7);
        if last_boundary.is_none() || natural || forced {
            ends.push(d);
            last_boundary = Some(d);
        }
        d += 1;
    }
    let end_day = *ends.last()?;
    let start_day = if ends.len() >= 2 { ends[ends.len() - 2] } else { end_day - 7 };
    let mut start_s = local_midnight_utc(start_day, zone);
    let end_s = local_midnight_utc(end_day, zone);
    if let Some(p) = pinned_start_s {
        if (p - start_s).abs() <= DAY_S + 3600 && p < end_s {
            start_s = p;
        }
    }
    Some(Window { start_s, end_s, first_day: start_day, last_day: end_day - 1 })
}

// ------------------------------------------------------- F4 / F5 rules

/// G-D26: a word type shows only with at least this many attempts…
pub const GATE_MIN_ATTEMPTS: u32 = 4;
/// …and at least this many misses in the window. Tunable after the census,
/// with Eric's sign-off only.
pub const GATE_MIN_MISSES: u32 = 2;

/// The G-D26 test, for a type or for an overall line.
pub fn passes_gate(attempts: u32, misses: u32) -> bool {
    attempts >= GATE_MIN_ATTEMPTS && misses >= GATE_MIN_MISSES
}

/// What one language did in a window: counts of eligible attempts (Speech,
/// Impossible, and attempts without their CR-1 instant and class never count).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LangStats {
    pub attempts: u32,
    pub misses: u32,
    pub by_type: BTreeMap<WordType, (u32, u32)>,
}

pub fn lang_stats(
    q: &dyn LearnerQuery,
    profile: ProfileId,
    lang: &str,
    bank: &dyn BankView,
    in_window: &dyn Fn(i64) -> bool,
) -> LangStats {
    let mut s = LangStats::default();
    for a in q.attempt_detail_log(profile, lang).iter().filter(|a| crate::reports_share::counts(a)) {
        let (Some(ts), Some(o)) = (a.ts, a.outcome) else { continue };
        if !in_window(ts as i64) {
            continue;
        }
        let miss = crate::reports_share::is_miss(o);
        s.attempts += 1;
        s.misses += u32::from(miss);
        if let Some(t) = primary_type(bank, lang, &a.word) {
            let e = s.by_type.entry(t).or_default();
            e.0 += 1;
            e.1 += u32::from(miss);
        }
    }
    s
}

/// A rate line a delta may be drawn on (F5): rates only, never raw counts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeltaLine {
    /// Share of attempts that were not misses.
    Accuracy,
    /// This type's share of the window's misses.
    TypeShare(WordType),
}

/// "vs last report": percentage points, current minus previous.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delta {
    pub lang: String,
    pub line: DeltaLine,
    pub points: i32,
}

/// Round-half-up integer percent of `n / d`.
fn pct(n: u32, d: u32) -> i32 {
    ((200 * u64::from(n) + u64::from(d)) / (2 * u64::from(d))) as i32
}

/// F5: deltas only where BOTH windows pass G-D26 for that line; otherwise the
/// line is omitted, not shown as zero or a dash.
pub fn deltas(lang: &str, cur: &LangStats, prev: &LangStats) -> Vec<Delta> {
    let mut out = Vec::new();
    if passes_gate(cur.attempts, cur.misses) && passes_gate(prev.attempts, prev.misses) {
        let acc = |s: &LangStats| pct(s.attempts - s.misses, s.attempts);
        out.push(Delta { lang: lang.into(), line: DeltaLine::Accuracy, points: acc(cur) - acc(prev) });
        for (t, &(ca, cm)) in &cur.by_type {
            let Some(&(pa, pm)) = prev.by_type.get(t) else { continue };
            if passes_gate(ca, cm) && passes_gate(pa, pm) {
                let share = |m: u32, s: &LangStats| pct(m, s.misses);
                out.push(Delta { lang: lang.into(), line: DeltaLine::TypeShare(t.clone()), points: share(cm, cur) - share(pm, prev) });
            }
        }
    }
    out
}

/// F4/F5 facts for a window, carried on the report. The words for them
/// ("Not enough practice yet to spot patterns.", "No practice this period.")
/// are Phase B/D copy; the numbers and flags are decided here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DaysInfo {
    /// Eligible attempts and the correct ones, over every language.
    pub attempts: u32,
    pub correct: u32,
    /// Zero attempts: F6, no ping, reads "No practice this period."
    pub empty: bool,
    /// Every language's word types were hidden by G-D26 (or none existed):
    /// the report falls back to count and accuracy, never a hinted type.
    pub not_enough: bool,
    pub deltas: Vec<Delta>,
}

// ----------------------------------------------------------------- tests

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A US-rule zone (second Sunday of March 02:00 to first Sunday of
    /// November 02:00 local): `-5` is Indianapolis and New York, `-8` Los Angeles.
    pub(crate) struct UsDst(pub i64);
    pub(crate) struct Fixed(pub i64);

    fn nth_sunday(y: i64, m: u32, n: i64) -> i64 {
        let first = days_from_civil(y, m, 1);
        let to_sun = (6 - weekday(first) as i64).rem_euclid(7);
        first + to_sun + 7 * (n - 1)
    }

    impl Zone for UsDst {
        fn offset_s(&self, utc_s: i64) -> i64 {
            let (y, _, _) = civil(utc_s.div_euclid(DAY_S));
            let start = nth_sunday(y, 3, 2) * DAY_S + 2 * 3600 - self.0 * 3600;
            let end = nth_sunday(y, 11, 1) * DAY_S + 2 * 3600 - (self.0 + 1) * 3600;
            if (start..end).contains(&utc_s) { (self.0 + 1) * 3600 } else { self.0 * 3600 }
        }
    }

    impl Zone for Fixed {
        fn offset_s(&self, _: i64) -> i64 {
            self.0 * 3600
        }
    }

    /// UTC seconds of a local wall time `h:mi` on a civil date, in a fixed offset.
    fn at(y: i64, m: u32, d: u32, h: i64, mi: i64, off_h: i64) -> i64 {
        days_from_civil(y, m, d) * DAY_S + h * 3600 + mi * 60 - off_h * 3600
    }

    fn ymd(day: i64) -> (i64, u32, u32) {
        civil(day)
    }

    /// A tiny deterministic generator (xorshift): the property test is repeatable.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn weekly_since(y: i64, m: u32, d: u32) -> ScheduleLog {
        ScheduleLog::weekly_since(days_from_civil(y, m, d) * DAY_S)
    }

    /// A12 (model): an empty weekday set cannot be built; the UI half is Phase B.
    #[test]
    fn an_empty_weekday_set_is_rejected() {
        assert_eq!(WeekdaySet::new(&[]), Err(EmptyWeekdaySet));
        assert_eq!(WeekdaySet::new(&[9]), Err(EmptyWeekdaySet), "no real day named");
        assert_eq!(WeekdaySet::new(&[0, 3]), Ok(WeekdaySet::TWICE_A_WEEK));
        assert_eq!(WeekdaySet::DAILY.len(), 7);
        assert!(ScheduleLog::from_entries(vec![]).is_none());
    }

    /// The default schedule gives v1's Monday–Sunday week (A2, window half).
    #[test]
    fn the_default_schedule_is_the_closed_monday_to_sunday_week() {
        let z = UsDst(-5);
        let log = weekly_since(2026, 9, 1);
        // Tue Oct 27 2026, 09:00 EDT: the week of Oct 19 has just closed.
        let w = window(at(2026, 10, 27, 9, 0, -4), &log, &z, None).unwrap();
        assert_eq!((ymd(w.first_day), ymd(w.last_day)), ((2026, 10, 19), (2026, 10, 25)));
        assert_eq!(w.days(), 7);
        // Monday 00:00:00 itself already closes it; one second before does not.
        let w = window(at(2026, 10, 26, 0, 0, -4), &log, &z, None).unwrap();
        assert_eq!(ymd(w.first_day), (2026, 10, 19));
        let w = window(at(2026, 10, 25, 23, 59, -4), &log, &z, None).unwrap();
        assert_eq!(ymd(w.first_day), (2026, 10, 12));
    }

    /// A3: {Mon, Thu}.
    #[test]
    fn twice_a_week_alternates_three_and_four_days() {
        let z = UsDst(-5);
        let log = ScheduleLog::from_entries(vec![ScheduleEntry { days: WeekdaySet::TWICE_A_WEEK, changed_at_s: at(2026, 9, 1, 0, 0, -4) }]).unwrap();
        let thu = window(at(2026, 10, 22, 9, 0, -4), &log, &z, None).unwrap();
        assert_eq!((ymd(thu.first_day), ymd(thu.last_day + 1)), ((2026, 10, 19), (2026, 10, 22)), "Mon 00:00 to Thu 00:00");
        assert_eq!(thu.days(), 3);
        let mon = window(at(2026, 10, 26, 9, 0, -4), &log, &z, None).unwrap();
        assert_eq!((ymd(mon.first_day), ymd(mon.last_day + 1)), ((2026, 10, 22), (2026, 10, 26)), "Thu 00:00 to Mon 00:00");
        assert_eq!(mon.days(), 4);
    }

    /// A4: weekly → daily on Wed 14:00, after a Monday report.
    #[test]
    fn a_schedule_edit_takes_effect_at_the_next_new_report_day() {
        let z = UsDst(-5);
        let mut log = weekly_since(2026, 9, 1);
        log.edit(WeekdaySet::DAILY, at(2026, 10, 21, 14, 0, -4)); // Wed Oct 21
        let w = window(at(2026, 10, 22, 0, 30, -4), &log, &z, None).unwrap();
        assert_eq!((ymd(w.first_day), ymd(w.last_day + 1)), ((2026, 10, 19), (2026, 10, 22)), "Mon 00:00 to Thu 00:00");
        // From then on, one day each.
        let w = window(at(2026, 10, 23, 0, 30, -4), &log, &z, None).unwrap();
        assert_eq!((ymd(w.first_day), w.days()), ((2026, 10, 22), 1));
    }

    /// An edit that would leave a gap longer than 7 days gets a forced report
    /// day at day 7, never a clamped start that drops attempts (G-D25, inv. 1).
    #[test]
    fn the_seven_day_cap_forces_a_boundary_instead_of_dropping_attempts() {
        let z = Fixed(0);
        let mut log = weekly_since(2026, 9, 1); // Mondays
        log.edit(ScheduleEntry_tue(), at(2026, 10, 20, 0, 1, 0)); // Tue Oct 20, just after Tuesday's midnight
        let mut ends = vec![];
        for day in 0..16 {
            if let Some(w) = window(at(2026, 10, 19, 0, 0, 0) + day * DAY_S + 5, &log, &z, None) {
                if ends.last() != Some(&w.end_s) {
                    ends.push(w.end_s);
                    assert!((1..=7).contains(&w.days()), "{w:?}");
                }
            }
        }
        assert!(ends.len() >= 2, "{ends:?}");
    }

    #[allow(non_snake_case)]
    fn ScheduleEntry_tue() -> WeekdaySet {
        WeekdaySet::new(&[1]).unwrap()
    }

    /// A5: DST. The window across a clock change still starts and ends at local
    /// midnight, and the 23- or 25-hour day is one day.
    #[test]
    fn a_dst_day_is_one_day() {
        for (z, name) in [(UsDst(-5), "Indianapolis/New_York"), (UsDst(-8), "Los_Angeles")] {
            let log = weekly_since(2026, 1, 5);
            // Spring forward Sun Mar 8 2026: week Mon Mar 2 → Mon Mar 9.
            let w = window(at(2026, 3, 10, 9, 0, -4), &log, &z, None).unwrap();
            assert_eq!((ymd(w.first_day), w.days()), ((2026, 3, 2), 7), "{name}");
            assert_eq!(w.end_s - w.start_s, 7 * DAY_S - 3600, "{name}: 167 hours");
            assert_eq!(local_day_sec(w.end_s, &z).1, 0, "{name}: ends at local midnight");
            assert_eq!(local_day_sec(w.start_s, &z).1, 0);
            // Fall back Sun Nov 1 2026: week Mon Oct 26 → Mon Nov 2.
            let w = window(at(2026, 11, 3, 9, 0, -5), &log, &z, None).unwrap();
            assert_eq!((ymd(w.first_day), w.days()), ((2026, 10, 26), 7), "{name}");
            assert_eq!(w.end_s - w.start_s, 7 * DAY_S + 3600, "{name}: 169 hours");
            assert_eq!(local_day_sec(w.end_s, &z).1, 0);
        }
    }

    /// A6: the device moves from UTC−5 to UTC+9 mid-window. The stored end of
    /// the delivered report pins the next start, so nothing is dropped or doubled.
    #[test]
    fn a_zone_change_drops_and_doubles_nothing() {
        let log = weekly_since(2026, 9, 1);
        let west = Fixed(-5);
        let east = Fixed(9);
        let first = window(at(2026, 10, 27, 9, 0, -5), &log, &west, None).unwrap(); // week of Oct 19, in UTC−5
        // Play across the move, then the next report is asked for in UTC+9.
        let attempts: Vec<i64> = (0..60).map(|i| first.end_s - 3 * DAY_S + i * 3 * 3600 + 17).collect();
        let second = window(at(2026, 11, 3, 9, 0, 9), &log, &east, Some(first.end_s)).unwrap();
        assert_eq!(second.start_s, first.end_s, "the stored end pins the start");
        assert!(second.end_s > second.start_s);
        for &t in attempts.iter().filter(|t| **t >= first.start_s && **t < second.end_s) {
            let n = [&first, &second].iter().filter(|w| (w.start_s..w.end_s).contains(&t)).count();
            assert_eq!(n, 1, "attempt at {t}");
        }
        // Without the pin the 14-hour shift would double-count or drop a stretch.
        let unpinned = window(at(2026, 11, 3, 9, 0, 9), &log, &east, None).unwrap();
        assert_ne!(unpinned.start_s, first.end_s);
    }

    /// A1: 10,000 random schedules with random mid-stream edits, over 90 days,
    /// across 5 zones: the windows chain with no gap or overlap, every one is 1
    /// to 7 local days, each ends at a local midnight, and each attempt falls
    /// in exactly one.
    #[test]
    fn every_attempt_is_in_exactly_one_window() {
        let zones: [&dyn Zone; 5] = [&Fixed(0), &Fixed(9), &Fixed(-5), &UsDst(-5), &UsDst(-8)];
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let day0 = days_from_civil(2026, 8, 1);
        for case in 0..10_000u64 {
            let z = zones[(case % 5) as usize];
            let mut sets = vec![WeekdaySet::new(&[rng.below(7) as u8]).unwrap()];
            let start = day0 * DAY_S + rng.below(DAY_S as u64) as i64;
            let mut entries = vec![ScheduleEntry { days: sets[0], changed_at_s: start }];
            for _ in 0..rng.below(4) {
                let bits: Vec<u8> = (0..7).filter(|_| rng.below(2) == 0).collect();
                let Ok(days) = WeekdaySet::new(&if bits.is_empty() { vec![rng.below(7) as u8] } else { bits }) else { continue };
                sets.push(days);
                entries.push(ScheduleEntry { days, changed_at_s: start + (1 + rng.below(80 * 24)) as i64 * 3600 });
            }
            let log = ScheduleLog::from_entries(entries).unwrap();
            let mut seen: Vec<Window> = Vec::new();
            for h in 0..(90 * 24 / 6) {
                let now = start + h * 6 * 3600 + 1;
                if let Some(w) = window(now, &log, z, None) {
                    if seen.last().map(|l| l.end_s) != Some(w.end_s) {
                        seen.push(w);
                    }
                }
            }
            // The first window may reach before the schedule began: skip it.
            let chain = &seen[1.min(seen.len())..];
            for pair in chain.windows(2) {
                assert_eq!(pair[1].start_s, pair[0].end_s, "case {case}: gap or overlap");
            }
            for w in chain {
                assert!((1..=7).contains(&w.days()), "case {case}: {w:?}");
                assert_eq!(local_day_sec(w.end_s, z).1, 0, "case {case}: not at midnight");
                assert_eq!(local_day_sec(w.start_s, z).1, 0, "case {case}");
            }
            if let (Some(a), Some(b)) = (chain.first(), chain.last()) {
                for _ in 0..20 {
                    let t = a.start_s + rng.below((b.end_s - a.start_s) as u64) as i64;
                    let n = chain.iter().filter(|w| (w.start_s..w.end_s).contains(&t)).count();
                    assert_eq!(n, 1, "case {case}: attempt at {t} is in {n} windows");
                }
            }
        }
    }

    fn stats(attempts: u32, misses: u32, ty: Option<(WordType, u32, u32)>) -> LangStats {
        LangStats { attempts, misses, by_type: ty.map(|(t, a, m)| (t, (a, m))).into_iter().collect() }
    }

    /// G-D26's line: at least 4 attempts and at least 2 misses, both.
    #[test]
    fn the_gate_needs_four_attempts_and_two_misses() {
        assert!(passes_gate(4, 2));
        assert!(!passes_gate(3, 3), "three attempts is too few");
        assert!(!passes_gate(9, 1), "one miss is too few");
    }

    /// A8: the previous window is below the gate, the current above: no delta,
    /// not a zero and not a dash. And rates, not counts, are compared.
    #[test]
    fn a_delta_needs_both_windows_to_pass_the_gate() {
        let t = WordType::Tier(1);
        let cur = stats(10, 4, Some((t.clone(), 6, 3)));
        let thin = stats(3, 2, Some((t.clone(), 3, 2)));
        assert!(deltas("en", &cur, &thin).is_empty(), "previous below threshold");
        assert!(deltas("en", &thin, &cur).is_empty(), "current below threshold");
        // Both pass: accuracy 60% vs 80%, and the type's share of misses.
        let prev = stats(5, 1 + 1, Some((t.clone(), 4, 2)));
        let d = deltas("en", &cur, &prev);
        let acc = d.iter().find(|d| d.line == DeltaLine::Accuracy).unwrap();
        assert_eq!(acc.points, 60 - 60);
        // 10 attempts, 4 misses → 60%; 5 attempts, 2 misses → 60%. Same rate: 0 points, shown.
        let better = stats(5, 2, Some((t.clone(), 4, 2)));
        let slip = stats(10, 5, Some((t.clone(), 8, 4)));
        let d = deltas("en", &slip, &better);
        assert_eq!(d.iter().find(|d| d.line == DeltaLine::Accuracy).unwrap().points, 50 - 60);
        let share = d.iter().find(|d| matches!(d.line, DeltaLine::TypeShare(_))).unwrap();
        assert_eq!(share.points, 80 - 100, "4 of 5 misses vs 2 of 2");
        // Only a type that passes in BOTH windows gets a share delta.
        let other = stats(10, 5, Some((WordType::Long, 8, 4)));
        assert!(deltas("en", &slip, &other).iter().all(|d| d.line == DeltaLine::Accuracy));
    }

    /// A13: no calendar or clock arithmetic on the report path outside this
    /// file. Tests are cut off; the DASH files are excluded by name (signed
    /// amendment 2026-11-07). `Date::now` is a clock READ and `Date::new` only formats; both stay in the UI.
    #[test]
    fn a13_no_date_math_outside_window() {
        const FILES: [(&str, &str); 3] = [
            ("reports_share.rs", include_str!("reports_share.rs")),
            ("guardian_report.rs", include_str!("guardian_report.rs")),
            ("reports_share_ui.rs", include_str!("reports_share_ui.rs")),
        ];
        const BANNED: [&str; 7] = ["86_400", "DAY_S", "div_euclid", "rem_euclid", ".offset_s(", "get_timezone_offset", "Calendar"];
        for (name, src) in FILES {
            let live = ["#[cfg(test)]\nmod ", "#[cfg(test)]\npub(crate) mod "].iter().filter_map(|m| src.find(m)).min().map(|i| &src[..i]).unwrap_or(src);
            for line in live.lines().filter(|l| !l.trim_start().starts_with("//")) {
                for b in BANNED {
                    // The one re-export of the constant is not arithmetic.
                    if line.contains(b) && !line.starts_with("pub use crate::report_days::") {
                        panic!("{name} does the date math {b:?} itself: {line}");
                    }
                }
            }
        }
    }
}
