//! CC-ACCOUNTS Phase 2 -- the account sheet's pure state. No DOM, no network, no storage.
//!
//! The sheet walks one short road: ask whether a grown-up is here, offer two doors (Apple or
//! an emailed code), take the code, finish. Everything that decides something lives here so a
//! native test can drive it: who is old enough, what each step may do next, what the server's
//! answer means, and which local keys a sign-in is allowed to touch (I5: none that hold progress).

use serde::Deserialize;

/// I7: accounts are adult-only in v1. A different bar from the 13-based age gate that decides Spell Jr.
pub const ADULT_AGE: u32 = 18;

pub fn is_adult(age: u32) -> bool {
    age >= ADULT_AGE
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Closed,
    /// The neutral age screen. No hint of which answer opens the door.
    Age,
    /// Under 18: no account, no further steps, a pointer to a parent (D14).
    UnderAge,
    Method,
    Email,
    Code,
    Working,
    Failed(Problem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The network or the service is not there. Solo play is untouched (I3).
    Offline,
    BadCode,
    Rejected,
    /// The signed server flag is off, or the signing key is not provisioned.
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Open,
    AgeAnswered { age: u32 },
    PickApple,
    PickEmail,
    CodeSent,
    Submit,
    Fail(Problem),
    SignedIn,
    Back,
    Close,
}

/// The next step. `Closed` is where a finished or abandoned sheet lands, and it is absorbing
/// except for `Open`, so a late network answer cannot reopen a sheet the player dismissed.
pub fn next(step: &Step, ev: &Event) -> Step {
    use Event::*;
    use Step::*;
    match (step, ev) {
        (_, Close) => Closed,
        (Closed, Open) => Age,
        (Closed, _) => Closed,
        (Age, AgeAnswered { age }) => {
            if is_adult(*age) {
                Method
            } else {
                UnderAge
            }
        }
        (Method, PickApple) => Working,
        (Method, PickEmail) => Email,
        (Email, CodeSent) => Code,
        (Email, Back) | (Code, Back) => Method,
        (Code, Submit) => Working,
        (Working, SignedIn) => Closed,
        (Working, Fail(p)) | (Email, Fail(p)) | (Code, Fail(p)) => Failed(*p),
        (Failed(_), Back) => Method,
        (s, _) => s.clone(),
    }
}

/// A code is exactly six digits. Anything else never leaves the device.
pub fn clean_code(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    (digits.len() == 6 && digits.chars().all(|c| c.is_ascii_digit())).then_some(digits)
}

/// Loose on purpose: the server decides. This only stops obvious non-addresses from costing a request.
pub fn plausible_email(s: &str) -> bool {
    let s = s.trim();
    s.len() <= 254
        && s.split_once('@')
            .is_some_and(|(l, d)| !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !s.contains(char::is_whitespace))
}

/// The server's sign-in answer (`/api/accounts/email/verify`, `/siwa`, `/refresh`).
#[derive(Deserialize, Debug, Clone)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: i64,
}

pub fn parse_session(body: &str) -> Option<Session> {
    serde_json::from_str(body).ok().filter(|s: &Session| !s.access_token.is_empty() && !s.refresh_token.is_empty())
}

/// What an HTTP status means to the sheet.
pub fn problem_for_status(status: u16) -> Problem {
    match status {
        400 | 401 => Problem::BadCode,
        403 => Problem::Rejected,
        0 | 429 | 500..=599 => {
            if status == 503 {
                Problem::Unavailable
            } else {
                Problem::Offline
            }
        }
        _ => Problem::Rejected,
    }
}

/// The `exp` claim of an access token, read WITHOUT verifying it. Only used to decide when to
/// refresh; the server is the one that trusts or distrusts a token.
pub fn token_expiry(jwt: &str) -> Option<f64> {
    let body = jwt.split('.').nth(1)?;
    let bytes = b64url_decode(body)?;
    serde_json::from_slice::<serde_json::Value>(&bytes).ok()?.get("exp")?.as_f64()
}

fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.trim_end_matches('=').bytes() {
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Seconds of life a token needs left before it is used as-is.
pub const REFRESH_MARGIN_SECS: f64 = 60.0;

pub fn needs_refresh(access: &str, now_secs: f64) -> bool {
    token_expiry(access).is_none_or(|exp| exp - now_secs < REFRESH_MARGIN_SECS)
}

/// The only localStorage keys account code may write (I5/I6). None holds progress, and none holds a token.
pub const LOCAL_KEYS: &[&str] = &["spell_acct_user_v1", "spell_acct_adopt_v1"];

/// F2: on a player's first sign-in the device's progress is marked "adopt me"; Phase 3's sync does the
/// merge. Marking never copies, moves, or deletes anything. Returns the marker to store, or `None` when
/// this device has already been adopted into that same account.
pub fn adoption_marker(prev_user: Option<&str>, user_id: i64) -> Option<String> {
    let id = user_id.to_string();
    (prev_user != Some(id.as_str())).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(events: &[Event]) -> Step {
        events.iter().fold(Step::Closed, |s, e| next(&s, e))
    }

    #[test]
    fn adults_reach_the_doors_and_nobody_else_does() {
        assert_eq!(run(&[Event::Open, Event::AgeAnswered { age: 18 }]), Step::Method);
        assert_eq!(run(&[Event::Open, Event::AgeAnswered { age: 17 }]), Step::UnderAge);
        assert_eq!(run(&[Event::Open, Event::AgeAnswered { age: 12 }]), Step::UnderAge);
        assert_eq!(run(&[Event::Open, Event::AgeAnswered { age: 0 }]), Step::UnderAge);
    }

    #[test]
    fn under_age_is_a_dead_end() {
        let dead = run(&[Event::Open, Event::AgeAnswered { age: 15 }]);
        for ev in [Event::PickApple, Event::PickEmail, Event::CodeSent, Event::Submit, Event::SignedIn, Event::Back] {
            assert_eq!(next(&dead, &ev), Step::UnderAge, "{ev:?} must not leave UnderAge");
        }
    }

    #[test]
    fn nothing_skips_the_age_screen() {
        for ev in [Event::PickApple, Event::PickEmail, Event::CodeSent, Event::Submit, Event::SignedIn] {
            assert_eq!(next(&Step::Age, &ev), Step::Age);
            assert_eq!(next(&Step::Closed, &ev), Step::Closed);
        }
    }

    #[test]
    fn email_road_end_to_end() {
        let s = run(&[
            Event::Open,
            Event::AgeAnswered { age: 40 },
            Event::PickEmail,
            Event::CodeSent,
            Event::Submit,
            Event::SignedIn,
        ]);
        assert_eq!(s, Step::Closed);
    }

    #[test]
    fn failure_is_recoverable_and_a_dismissed_sheet_stays_dismissed() {
        let f = run(&[Event::Open, Event::AgeAnswered { age: 30 }, Event::PickEmail, Event::Fail(Problem::Offline)]);
        assert_eq!(f, Step::Failed(Problem::Offline));
        assert_eq!(next(&f, &Event::Back), Step::Method);
        // "Not now", then a slow answer arrives: it must not resurrect the sheet.
        let closed = next(&Step::Working, &Event::Close);
        assert_eq!(next(&closed, &Event::SignedIn), Step::Closed);
        assert_eq!(next(&closed, &Event::Fail(Problem::BadCode)), Step::Closed);
    }

    #[test]
    fn codes_are_six_digits_only() {
        assert_eq!(clean_code("123456").as_deref(), Some("123456"));
        assert_eq!(clean_code(" 123 456 ").as_deref(), Some("123456"));
        assert_eq!(clean_code("123-456").as_deref(), Some("123456"));
        for bad in ["", "12345", "1234567", "12345a", "abcdef", "١٢٣٤٥٦"] {
            assert_eq!(clean_code(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn email_plausibility() {
        assert!(plausible_email("a@example.com"));
        assert!(plausible_email(" a@b.co "));
        for bad in ["", "a", "a@", "@b.co", "a@b", "a@.co", "a@b.", "a b@c.de"] {
            assert!(!plausible_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn session_parsing_and_statuses() {
        let ok = r#"{"access_token":"a.b.c","refresh_token":"rt_x","user_id":7,"expires_in":900}"#;
        assert_eq!(parse_session(ok).unwrap().user_id, 7);
        assert!(parse_session(r#"{"access_token":"","refresh_token":"x","user_id":1}"#).is_none());
        assert!(parse_session("not json").is_none());
        assert_eq!(problem_for_status(400), Problem::BadCode);
        assert_eq!(problem_for_status(403), Problem::Rejected);
        assert_eq!(problem_for_status(503), Problem::Unavailable);
        assert_eq!(problem_for_status(0), Problem::Offline);
        assert_eq!(problem_for_status(500), Problem::Offline);
    }

    #[test]
    fn token_expiry_reads_exp_and_refresh_has_a_margin() {
        // {"exp":2000}
        let jwt = "e30.eyJleHAiOjIwMDB9.sig";
        assert_eq!(token_expiry(jwt), Some(2000.0));
        assert!(!needs_refresh(jwt, 1000.0));
        assert!(needs_refresh(jwt, 1950.0));
        assert!(needs_refresh(jwt, 2500.0));
        assert!(needs_refresh("garbage", 0.0));
        assert!(needs_refresh("a.%%%.c", 0.0));
    }

    #[test]
    fn adoption_marks_once_and_touches_nothing_else() {
        assert_eq!(adoption_marker(None, 7).as_deref(), Some("7"));
        assert_eq!(adoption_marker(Some("7"), 7), None);
        assert_eq!(adoption_marker(Some("7"), 9).as_deref(), Some("9"));
        // I5: the keys account code may write hold neither progress nor tokens.
        for k in LOCAL_KEYS {
            assert!(k.starts_with("spell_acct_"));
            assert!(!k.contains("token") && !k.contains("word") && !k.contains("miss"));
        }
    }
}
