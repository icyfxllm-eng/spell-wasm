//! CC-THEMES-COSMOS F4: the Moon and the sky, with no network and no location.
//!
//! Pure functions of a calendar date or a clock time. The caller passes the
//! date (the Lab clock override, I-T11, lives with the caller: there is no
//! clock here). Nothing in this module reads the network, location, storage
//! or telemetry, and scripts/astro-check.mjs (AT-T14) keeps it that way.
//!
//! Moon phases come from Jean Meeus, Astronomical Algorithms (2nd ed.), ch. 49:
//! the mean phase plus the periodic and planetary corrections, good to a
//! minute or so. The full-moon, April and eclipse windows read the generated
//! table in `astro_events.rs` (2026-2040, cross-checked against NASA GSFC).
//!
//! Time zones: there is no location, so there is no time zone. Phase
//! instants are treated as UTC and compared with the local date by calendar
//! day; the +-1 day principal-phase window ("the local date is within +-1 day
//! of that phase's instant") absorbs any real offset (at most 14 h). The
//! local date is evaluated at noon.

// The per-phase lists and APRIL_FULL_MOONS are read by the tests (AT-T7/AT-T8);
// the app path computes phases with Meeus directly and agrees with them to the minute.
#[allow(dead_code)]
#[path = "astro_events.rs"]
mod events;

pub use events::{EVENTS_FIRST_YEAR, EVENTS_LAST_YEAR};

/// The four principal phases of the Moon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Principal {
    New,
    FirstQuarter,
    Full,
    LastQuarter,
}

impl Principal {
    fn index(self) -> u8 {
        match self {
            Principal::New => 0,
            Principal::FirstQuarter => 2,
            Principal::Full => 4,
            Principal::LastQuarter => 6,
        }
    }
    fn quarter(self) -> f64 {
        match self {
            Principal::New => 0.0,
            Principal::FirstQuarter => 0.25,
            Principal::Full => 0.5,
            Principal::LastQuarter => 0.75,
        }
    }
    const ALL: [Principal; 4] = [Principal::New, Principal::FirstQuarter, Principal::Full, Principal::LastQuarter];
}

/// The Moon on a local calendar date (evaluated at local noon).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    /// 0 new, 1 waxing crescent, 2 first quarter, 3 waxing gibbous, 4 full,
    /// 5 waning gibbous, 6 last quarter, 7 waning crescent. An even index
    /// appears exactly on a principal-phase day (`principal` is Some); every
    /// other day carries the odd index of the stretch it falls in.
    pub index: u8,
    /// Illuminated fraction of the disc, 0..=1.
    pub illumination: f64,
    /// Days since the preceding new moon.
    pub age_days: f64,
    /// Some(p) when the date is within +-1 calendar day of p's instant (UTC).
    pub principal: Option<Principal>,
}

/// The four bands of the sky through a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Sunrise,
    Day,
    Sunset,
    Night,
}

/// Sunrise 05:00-08:00, Day 08:00-17:00, Sunset 17:00-20:00, Night 20:00-05:00
/// (start inclusive, end exclusive). Hours past 23 wrap.
pub fn sky_band(hour: u32, minute: u32) -> Band {
    let m = ((hour % 24) * 60 + minute.min(59)) as u32;
    match m {
        300..=479 => Band::Sunrise,
        480..=1019 => Band::Day,
        1020..=1199 => Band::Sunset,
        _ => Band::Night,
    }
}

const MIN_PER_DAY: i64 = 1440;

/// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y as i64 - 1 } else { y as i64 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y as i32, m, d)
}

fn sind(x: f64) -> f64 {
    x.to_radians().sin()
}
fn cosd(x: f64) -> f64 {
    x.to_radians().cos()
}

/// Meeus ch. 49: the instant (minutes since 1970-01-01 UTC) of the phase with
/// lunation number `k` (integer) and `q` in {0, .25, .5, .75}.
fn meeus_minutes(k: i64, q: f64) -> i64 {
    let k = k as f64 + q;
    let t = k / 1236.85;
    let (t2, t3, t4) = (t * t, t * t * t, t * t * t * t);
    let jde0 = 2451550.09766 + 29.530588861 * k + 0.00015437 * t2 - 0.000000150 * t3 + 0.00000000073 * t4;
    let e = 1.0 - 0.002516 * t - 0.0000074 * t2;
    let m = 2.5534 + 29.10535670 * k - 0.0000014 * t2 - 0.00000011 * t3;
    let mp = 201.5643 + 385.81693528 * k + 0.0107582 * t2 + 0.00001238 * t3 - 0.000000058 * t4;
    let f = 160.7108 + 390.67050284 * k - 0.0016118 * t2 - 0.00000227 * t3 + 0.000000011 * t4;
    let om = 124.7746 - 1.56375588 * k + 0.0020672 * t2 + 0.00000215 * t3;
    let mut c = if q == 0.0 || q == 0.5 {
        let a: [f64; 7] = if q == 0.0 {
            [-0.40720, 0.17241, 0.01608, 0.01039, 0.00739, -0.00514, 0.00208]
        } else {
            [-0.40614, 0.17302, 0.01614, 0.01043, 0.00734, -0.00515, 0.00209]
        };
        a[0] * sind(mp) + a[1] * e * sind(m) + a[2] * sind(2.0 * mp) + a[3] * sind(2.0 * f)
            + a[4] * e * sind(mp - m) + a[5] * e * sind(mp + m) + a[6] * e * e * sind(2.0 * m)
            - 0.00111 * sind(mp - 2.0 * f) - 0.00057 * sind(mp + 2.0 * f) + 0.00056 * e * sind(2.0 * mp + m)
            - 0.00042 * sind(3.0 * mp) + 0.00042 * e * sind(m + 2.0 * f) + 0.00038 * e * sind(m - 2.0 * f)
            - 0.00024 * e * sind(2.0 * mp - m) - 0.00017 * sind(om) - 0.00007 * sind(mp + 2.0 * m)
            + 0.00004 * sind(2.0 * mp - 2.0 * f) + 0.00004 * sind(3.0 * m) + 0.00003 * sind(mp + m - 2.0 * f)
            + 0.00003 * sind(2.0 * mp + 2.0 * f) - 0.00003 * sind(mp + m + 2.0 * f)
            + 0.00003 * sind(mp - m + 2.0 * f) - 0.00002 * sind(mp - m - 2.0 * f)
            - 0.00002 * sind(3.0 * mp + m) + 0.00002 * sind(4.0 * mp)
    } else {
        let w = 0.00306 - 0.00038 * e * cosd(m) + 0.00026 * cosd(mp) - 0.00002 * cosd(mp - m)
            + 0.00002 * cosd(mp + m) + 0.00002 * cosd(2.0 * f);
        let c = -0.62801 * sind(mp) + 0.17172 * e * sind(m) - 0.01183 * e * sind(mp + m) + 0.00862 * sind(2.0 * mp)
            + 0.00804 * sind(2.0 * f) + 0.00454 * e * sind(mp - m) + 0.00204 * e * e * sind(2.0 * m)
            - 0.00180 * sind(mp - 2.0 * f) - 0.00070 * sind(mp + 2.0 * f) - 0.00040 * sind(3.0 * mp)
            - 0.00034 * e * sind(2.0 * mp - m) + 0.00032 * e * sind(m + 2.0 * f) + 0.00032 * e * sind(m - 2.0 * f)
            - 0.00028 * e * e * sind(mp + 2.0 * m) + 0.00027 * e * sind(2.0 * mp + m) - 0.00017 * sind(om)
            - 0.00005 * sind(mp - m - 2.0 * f) + 0.00004 * sind(2.0 * mp + 2.0 * f)
            - 0.00004 * sind(mp + m + 2.0 * f) + 0.00004 * sind(mp - 2.0 * m) + 0.00003 * sind(mp + m - 2.0 * f)
            + 0.00003 * sind(3.0 * m) + 0.00002 * sind(2.0 * mp - 2.0 * f) + 0.00002 * sind(mp - m + 2.0 * f)
            - 0.00002 * sind(3.0 * mp + m);
        if q == 0.25 { c + w } else { c - w }
    };
    let a = [
        (0.000325, 299.77 + 0.107408 * k - 0.009173 * t2),
        (0.000165, 251.88 + 0.016321 * k),
        (0.000164, 251.83 + 26.651886 * k),
        (0.000126, 349.42 + 36.412478 * k),
        (0.000110, 84.66 + 18.206239 * k),
        (0.000062, 141.74 + 53.303771 * k),
        (0.000060, 207.14 + 2.453732 * k),
        (0.000056, 154.84 + 7.306860 * k),
        (0.000047, 34.52 + 27.261239 * k),
        (0.000042, 207.19 + 0.121824 * k),
        (0.000040, 291.34 + 1.844379 * k),
        (0.000037, 161.72 + 24.198154 * k),
        (0.000035, 239.56 + 25.513099 * k),
        (0.000023, 331.55 + 3.592518 * k),
    ];
    for (w, arg) in a {
        c += w * sind(arg);
    }
    let jde = jde0 + c;
    // TT -> UT: Espenak-Meeus Delta T polynomial (seconds; well under a minute of error here).
    let yr = (jde - 2451545.0) / 365.25; // years since 2000
    let delta_t = 62.92 + 0.32217 * yr + 0.005589 * yr * yr;
    ((jde - delta_t / 86400.0 - 2440587.5) * MIN_PER_DAY as f64).round() as i64
}

/// Lunation number whose new moon is at or before `minutes`.
fn lunation_at(minutes: i64) -> i64 {
    // k = 0 is the new moon of 2000-01-06; one mean synodic month later each step.
    let mean = ((minutes as f64 / MIN_PER_DAY as f64) - 10962.75) / 29.530588861;
    let mut k = mean.floor() as i64;
    while meeus_minutes(k, 0.0) > minutes {
        k -= 1;
    }
    while meeus_minutes(k + 1, 0.0) <= minutes {
        k += 1;
    }
    k
}

/// The nearest instant of `p` whose UTC calendar day is within +-1 of `day`.
fn principal_near(day: i64, p: Principal) -> Option<i64> {
    let k = lunation_at(day * MIN_PER_DAY + MIN_PER_DAY / 2);
    (k - 1..=k + 1)
        .map(|k| meeus_minutes(k, p.quarter()))
        .filter(|x| (x.div_euclid(MIN_PER_DAY) - day).abs() <= 1)
        .min_by_key(|x| (x.div_euclid(MIN_PER_DAY) - day).abs())
}

/// The Moon on a local calendar date (see the module note on time zones).
pub fn phase(y: i32, m: u32, d: u32) -> Phase {
    let day = days_from_civil(y, m, d);
    let noon = day * MIN_PER_DAY + MIN_PER_DAY / 2;
    let k = lunation_at(noon);
    let marks = [
        meeus_minutes(k, 0.0),
        meeus_minutes(k, 0.25),
        meeus_minutes(k, 0.5),
        meeus_minutes(k, 0.75),
        meeus_minutes(k + 1, 0.0),
    ];
    let seg = (0..4).rev().find(|&j| marks[j] <= noon).unwrap_or(0);
    let frac = (noon - marks[seg]) as f64 / (marks[seg + 1] - marks[seg]) as f64;
    let angle = 90.0 * (seg as f64 + frac);
    let illumination = ((1.0 - cosd(angle)) / 2.0).clamp(0.0, 1.0);
    let principal = Principal::ALL
        .into_iter()
        .filter_map(|p| principal_near(day, p).map(|x| ((x.div_euclid(MIN_PER_DAY) - day).abs(), p)))
        .min_by_key(|(dist, _)| *dist)
        .map(|(_, p)| p);
    let index = principal.map(Principal::index).unwrap_or(2 * seg as u8 + 1);
    Phase { index, illumination, age_days: (noon - marks[0]) as f64 / MIN_PER_DAY as f64, principal }
}

/// If the date is within +-1 day of a full moon, that full moon's UTC date: the
/// window id, so each full-moon window counts once. Reads the generated table
/// for 2026-2040 and falls back to Meeus outside it.
pub fn full_moon_window(y: i32, m: u32, d: u32) -> Option<(i32, u32, u32)> {
    let day = days_from_civil(y, m, d);
    let table = (EVENTS_FIRST_YEAR..=EVENTS_LAST_YEAR).contains(&y);
    let hit = if table {
        events::FULL_MOONS
            .iter()
            .copied()
            .find(|x| (x.div_euclid(MIN_PER_DAY) - day).abs() <= 1)
    } else {
        principal_near(day, Principal::Full)
    };
    hit.map(|x| civil_from_days(x.div_euclid(MIN_PER_DAY)))
}

/// True within +-1 day of an April full moon (the window may spill into 31 Mar or 1 May).
pub fn is_april_full_moon_window(y: i32, m: u32, d: u32) -> bool {
    matches!(full_moon_window(y, m, d), Some((_, 4, _)))
}

/// True within +-1 day of a total lunar eclipse in the table (2026-2040 only:
/// outside the table this is always false; astro-check warns before it ends).
pub fn is_total_lunar_eclipse_window(y: i32, m: u32, d: u32) -> bool {
    full_moon_window(y, m, d).is_some_and(|id| events::TOTAL_LUNAR_ECLIPSES.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NASA_PHASES: &str = include_str!("../tests/fixtures/astro/nasa-phases-2026-2028.txt");

    /// (principal, y, m, d, minutes-since-epoch) for every instant in the NASA rows.
    fn nasa_rows() -> Vec<(Principal, i32, u32, u32, i64)> {
        const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let mut out = Vec::new();
        let mut year = 0;
        let mut last_month = 0;
        for line in NASA_PHASES.lines() {
            if line.starts_with('#') || line.trim().is_empty() || line.contains("New Moon") {
                continue;
            }
            if let Ok(y) = line.get(0..5).unwrap_or("").trim().parse::<i32>() {
                year = y;
                last_month = 0;
            }
            for (col, p) in [8usize, 26, 44, 62].into_iter().zip(Principal::ALL) {
                let Some(cell) = line.get(col..col + 13) else { continue };
                let parts: Vec<&str> = cell.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let m = MON.iter().position(|x| *x == parts[0]).expect("month") as u32 + 1;
                let d: u32 = parts[1].parse().unwrap();
                let (hh, mm) = parts[2].split_once(':').unwrap();
                // The last row of a year can only roll into the next year (e.g. "Dec 31" then none).
                let y = if m < last_month && m == 1 { year + 1 } else { year };
                last_month = last_month.max(m);
                let mins = days_from_civil(y, m, d) * MIN_PER_DAY + hh.parse::<i64>().unwrap() * 60 + mm.parse::<i64>().unwrap();
                out.push((p, y, m, d, mins));
            }
        }
        out
    }

    fn table(p: Principal) -> &'static [i64] {
        match p {
            Principal::New => events::NEW_MOONS,
            Principal::FirstQuarter => events::FIRST_QUARTERS,
            Principal::Full => events::FULL_MOONS,
            Principal::LastQuarter => events::LAST_QUARTERS,
        }
    }

    fn shift(y: i32, m: u32, d: u32, by: i64) -> (i32, u32, u32) {
        civil_from_days(days_from_civil(y, m, d) + by)
    }

    #[test]
    fn at_t7_phases_match_nasa_2026_2028() {
        let rows = nasa_rows();
        assert!(rows.len() >= 24, "only {} NASA rows", rows.len());
        for p in Principal::ALL {
            assert!(rows.iter().filter(|r| r.0 == p).count() >= 30, "{p:?} under-sampled");
        }
        for &(p, y, m, d, mins) in &rows {
            let got = phase(y, m, d);
            assert_eq!(got.principal, Some(p), "{y}-{m}-{d}: expected {p:?}, got {got:?}");
            assert_eq!(got.index, p.index());
            for by in [-1, 1] {
                let (yy, mm, dd) = shift(y, m, d, by);
                assert_eq!(phase(yy, mm, dd).principal, Some(p), "{p:?} window at {yy}-{mm}-{dd}");
            }
            for by in [-2, 2] {
                let (yy, mm, dd) = shift(y, m, d, by);
                assert_eq!(phase(yy, mm, dd).principal, None, "no principal at {yy}-{mm}-{dd}");
            }
            // The generated table: same UTC day as NASA, and within a few minutes.
            let ours = *table(p).iter().min_by_key(|x| (*x - mins).abs()).unwrap();
            assert_eq!(ours.div_euclid(MIN_PER_DAY), mins.div_euclid(MIN_PER_DAY), "{p:?} {y}-{m}-{d} day");
            assert!((ours - mins).abs() <= 3, "{p:?} {y}-{m}-{d}: table {ours} vs NASA {mins}");
        }
    }

    #[test]
    fn rust_meeus_matches_generated_table() {
        for p in Principal::ALL {
            for &x in table(p) {
                let k = lunation_at(x + 3 * MIN_PER_DAY);
                let near = (k - 1..=k + 1).map(|k| meeus_minutes(k, p.quarter())).min_by_key(|y| (y - x).abs()).unwrap();
                assert!((near - x).abs() <= 1, "{p:?}: rust {near} vs table {x}");
            }
        }
    }

    #[test]
    fn phase_shape() {
        // Full moon 2026-01-03, last quarter 01-10, new 01-18, first quarter 01-26.
        let full = phase(2026, 1, 3);
        assert!(full.illumination > 0.98 && full.index == 4);
        let new = phase(2026, 1, 18); // instant 19:52 UTC, after noon: still the old lunation
        assert!(new.illumination < 0.02 && new.index == 0 && new.age_days > 28.5);
        let after = phase(2026, 1, 19);
        assert!(after.index == 0 && after.age_days < 1.0);
        assert_eq!(phase(2026, 1, 22).index, 1); // waxing crescent
        assert_eq!(phase(2026, 1, 30).index, 3); // waxing gibbous
        assert_eq!(phase(2026, 1, 6).index, 5); // waning gibbous
        assert_eq!(phase(2026, 1, 14).index, 7); // waning crescent
        let q = phase(2026, 1, 26);
        assert!((q.illumination - 0.5).abs() < 0.1);
        for d in 1..=28 {
            let p = phase(2026, 2, d);
            assert!(p.index < 8 && (0.0..=1.0).contains(&p.illumination) && (0.0..30.0).contains(&p.age_days));
        }
    }

    #[test]
    fn at_t8_table_covers_2026_2040() {
        assert_eq!(EVENTS_FIRST_YEAR, 2026);
        assert_eq!(EVENTS_LAST_YEAR, 2040);
        for y in EVENTS_FIRST_YEAR..=EVENTS_LAST_YEAR {
            let n = events::FULL_MOONS.iter().filter(|x| civil_from_days(x.div_euclid(MIN_PER_DAY)).0 == y).count();
            assert!(n >= 12, "{y}: {n} full moons");
        }
        assert!(events::FULL_MOONS.iter().any(|x| civil_from_days(x.div_euclid(MIN_PER_DAY)).0 == 2040));
        assert_eq!(events::APRIL_FULL_MOONS.len(), 15);
        for &(y, m, d) in events::APRIL_FULL_MOONS {
            assert!(is_april_full_moon_window(y, m, d));
        }
    }

    #[test]
    fn total_lunar_eclipses() {
        assert!(is_total_lunar_eclipse_window(2026, 3, 3));
        assert!(is_total_lunar_eclipse_window(2026, 3, 2));
        assert!(is_total_lunar_eclipse_window(2026, 3, 4));
        assert!(!is_total_lunar_eclipse_window(2026, 3, 6));
        // 2026-08-28 is a partial eclipse; 2026-04-02 no eclipse at all.
        assert!(full_moon_window(2026, 8, 28).is_some() && !is_total_lunar_eclipse_window(2026, 8, 28));
        assert!(full_moon_window(2026, 4, 2).is_some() && !is_total_lunar_eclipse_window(2026, 4, 2));
        assert!(is_total_lunar_eclipse_window(2040, 11, 18));
        assert_eq!(events::TOTAL_LUNAR_ECLIPSES.len(), 13);
    }

    #[test]
    fn sky_band_boundaries() {
        let cases = [
            ((4, 59), Band::Night),
            ((5, 0), Band::Sunrise),
            ((7, 59), Band::Sunrise),
            ((8, 0), Band::Day),
            ((16, 59), Band::Day),
            ((17, 0), Band::Sunset),
            ((19, 59), Band::Sunset),
            ((20, 0), Band::Night),
            ((0, 0), Band::Night),
            ((23, 59), Band::Night),
        ];
        for ((h, m), b) in cases {
            assert_eq!(sky_band(h, m), b, "{h:02}:{m:02}");
        }
    }

    #[test]
    fn april_full_moon() {
        // April 2026 full moon: 2026-04-02 02:12 UTC.
        for d in 1..=3 {
            assert!(is_april_full_moon_window(2026, 4, d), "Apr {d}");
        }
        assert!(!is_april_full_moon_window(2026, 4, 5));
        // March 2026 full moon (03-03) is not April.
        assert!(!is_april_full_moon_window(2026, 3, 3));
        assert!(!is_april_full_moon_window(2026, 5, 1)); // May 1 full moon
    }

    #[test]
    fn full_moon_window_id_is_stable() {
        for &x in events::FULL_MOONS {
            let (y, m, d) = civil_from_days(x.div_euclid(MIN_PER_DAY));
            if !(EVENTS_FIRST_YEAR..=EVENTS_LAST_YEAR).contains(&y) {
                continue;
            }
            for by in -1..=1 {
                let (yy, mm, dd) = shift(y, m, d, by);
                assert_eq!(full_moon_window(yy, mm, dd), Some((y, m, d)), "{yy}-{mm}-{dd}");
            }
            for by in [-2, 2] {
                let (yy, mm, dd) = shift(y, m, d, by);
                assert_eq!(full_moon_window(yy, mm, dd), None);
            }
        }
        // Outside the table, Meeus still answers, and agrees with the table's edge neighbours.
        assert_eq!(full_moon_window(2025, 12, 5), Some((2025, 12, 4)));
        assert_eq!(full_moon_window(2041, 1, 18), Some((2041, 1, 17)));
    }

    #[test]
    fn civil_round_trip() {
        for z in (days_from_civil(2025, 1, 1)..days_from_civil(2042, 1, 1)).step_by(13) {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }
}
