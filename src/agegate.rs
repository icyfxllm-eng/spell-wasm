//! First-launch age gate. We ask date of birth once, compute age locally, and
//! store ONLY the verdict ("kid"/"full") — never the birth date itself (data
//! minimization: the verdict is all we need). Under-cutoff devices are locked
//! into Kid Mode; leaving it requires the parent gate (a worded-math challenge
//! in lib.rs). Clearing app data wipes the stored verdict, so the prompt
//! reappears on next launch — acceptable, and the only way to reset it.

use serde::{Deserialize, Serialize};

use crate::dom;
use crate::storage;

/// Minimum age for the full app. Default **13, not 12**, on purpose: US COPPA
/// treats under-13 as children for data-collection purposes, and the account
/// system (The Climb) collects email/phone. Letting 12-year-olds into the full
/// app and its signup flow would create COPPA exposure. The owner may lower
/// this to 12, but 13 is the safe default.
pub const MIN_FULL_APP_AGE: u32 = 13;

const AGE_GATE_KEY: &str = "byear_agegate_v1";

#[derive(Serialize, Deserialize, Clone)]
pub struct AgeVerdict {
    /// "kid" or "full" — the verdict only; the birth date is never stored.
    pub verdict: String,
    #[serde(rename = "checkedAt")]
    pub checked_at: f64,
}

/// The stored verdict, if the gate has already been answered on this device.
pub fn stored() -> Option<AgeVerdict> {
    storage::get_json::<AgeVerdict>(AGE_GATE_KEY)
}

pub fn is_kid_locked() -> bool {
    stored().map(|v| v.verdict == "kid").unwrap_or(false)
}

/// Persist the verdict for `age` against the cutoff (verdict only). Returns
/// true if this is a full-app verdict.
pub fn save(age: u32) -> bool {
    let is_full = age >= MIN_FULL_APP_AGE;
    let v = AgeVerdict {
        verdict: if is_full { "full".into() } else { "kid".into() },
        checked_at: js_sys::Date::now(),
    };
    storage::set_json(AGE_GATE_KEY, &v);
    is_full
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Today's (year, month 1-12, day) from the device clock.
fn today() -> (i32, u32, u32) {
    let d = js_sys::Date::new_0();
    (d.get_full_year() as i32, d.get_month() as u32 + 1, d.get_date() as u32)
}

/// A real, non-future calendar date after 1900.
pub fn valid_date(y: i32, m: u32, d: u32) -> bool {
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) || y <= 1900 {
        return false;
    }
    (y, m, d) <= today()
}

/// Whole-years age as of today. A Feb-29 birthday ticks over on Mar 1 in
/// non-leap years, because tuple ordering puts (3,1) after (2,29).
pub fn age_from(y: i32, m: u32, d: u32) -> u32 {
    let (ty, tm, td) = today();
    let mut age = ty - y;
    if (tm, td) < (m, d) {
        age -= 1;
    }
    age.max(0) as u32
}

/// Fill the month/day/year selectors. Year starts at a neutral 2000 (never a
/// value that hints at the cutoff); range is the last ~100 years.
pub fn populate_selects() {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ];
    let mut mo = String::new();
    for (i, name) in MONTHS.iter().enumerate() {
        mo.push_str(&format!("<option value=\"{}\">{}</option>", i + 1, name));
    }
    dom::set_html("ageMonth", &mo);

    let mut da = String::new();
    for d in 1..=31 {
        da.push_str(&format!("<option value=\"{d}\">{d}</option>"));
    }
    dom::set_html("ageDay", &da);

    let (ty, _, _) = today();
    let mut ye = String::new();
    for y in (ty - 100..=ty).rev() {
        ye.push_str(&format!("<option value=\"{y}\">{y}</option>"));
    }
    dom::set_html("ageYear", &ye);

    dom::select("ageMonth").set_value("1");
    dom::select("ageDay").set_value("1");
    dom::select("ageYear").set_value("2000");
}

/// Read the current (year, month, day) from the selectors.
pub fn read_selection() -> (i32, u32, u32) {
    let y = dom::select("ageYear").value().parse().unwrap_or(0);
    let m = dom::select("ageMonth").value().parse().unwrap_or(0);
    let d = dom::select("ageDay").value().parse().unwrap_or(0);
    (y, m, d)
}

/// A worded multiplication challenge for the parent gate, and its answer.
/// Worded (not digits) so it's a real speed bump for a young child — in the
/// active UI language.
pub fn parent_problem() -> (String, i32) {
    let a = 3 + (js_sys::Math::random() * 6.0) as i32; // 3..=8
    let b = 3 + (js_sys::Math::random() * 6.0) as i32;
    let words = number_words(&crate::i18n::current());
    let q = crate::i18n::tp("parent.question", &[("a", words[a as usize]), ("b", words[b as usize])]);
    (q, a * b)
}

/// Spelled-out 0–9 for each supported UI locale (only 3–8 are ever used).
fn number_words(locale: &str) -> [&'static str; 10] {
    match locale {
        "es" => ["cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve"],
        "fr" => ["z\u{e9}ro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf"],
        "de" => ["null", "eins", "zwei", "drei", "vier", "f\u{fc}nf", "sechs", "sieben", "acht", "neun"],
        "pt" => ["zero", "um", "dois", "tr\u{ea}s", "quatro", "cinco", "seis", "sete", "oito", "nove"],
        "pl" => ["zero", "jeden", "dwa", "trzy", "cztery", "pi\u{119}\u{107}", "sze\u{15b}\u{107}", "siedem", "osiem", "dziewi\u{119}\u{107}"],
        "tr" => ["s\u{131}f\u{131}r", "bir", "iki", "\u{fc}\u{e7}", "d\u{f6}rt", "be\u{15f}", "alt\u{131}", "yedi", "sekiz", "dokuz"],
        _ => ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"],
    }
}
