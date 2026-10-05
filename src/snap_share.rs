//! CC-SNAP-SHARE 3.3 — the payload a teacher's QR code carries.
//!
//! A teacher captures a list once; every family scans it. This owns the
//! bytes and nothing else: not the QR rendering, not the camera, not the
//! review screen it lands on.
//!
//! # Scope, and one thing deliberately not done
//!
//! D7 is SIGNED: words only. No photo, no student identity, no progress.
//! That is enforced here by construction rather than by review -- the
//! payload struct has three fields and there is nowhere for anything else
//! to ride along. `d7_payload_carries_nothing_else` holds the encoded
//! length to exactly its parts, so a fourth field cannot be added without
//! a test going red.
//!
//! **The roadmap says "a signed list payload" and this is NOT signed.** It
//! is checksummed. The difference matters and is not mine to decide:
//!
//! - A CHECKSUM answers "did this arrive intact", and catches a mis-scan,
//!   a truncated code or a typo. That is what is here.
//! - A SIGNATURE answers "did a teacher issue this, and has nobody edited
//!   the word list since". That needs a key the scanner can verify against,
//!   which needs key distribution, which needs the accounts 3.3's own gate
//!   defers to CC-ONBOARD-JR Phase B.
//!
//! Shipping a checksum and calling it a signature would be the worse
//! outcome of the two, so it is named for what it is. If Eric wants real
//! signing, it arrives with Phase B and changes the version byte.
//!
//! # Bytes, not text
//!
//! `encode` returns bytes because that is what a QR carries best. A QR in
//! byte mode spends 8 bits per byte of payload; base64 in the same mode
//! spends 8 bits to carry 6, and base32 in QR's uppercase alphanumeric mode
//! spends 5.5 to carry 5. Base-encoding is a cost worth paying only for the
//! SHORT CODE path -- a thing a person types -- and that path is deferred
//! to Phase B with the accounts it needs. So: bytes now, and the text
//! encoding arrives with the feature that needs it.

/// Unit separator. A control character, so it cannot occur in a word the
/// bank or a keyboard produced -- but `encode` checks rather than trusting,
/// because a payload that silently loses a word on a separator would show
/// up as a short list on someone else's phone.
const SEP: u8 = 0x1f;

/// Bumped by any wire change. A decoder refuses a version it does not know
/// rather than guessing, so an old app meets a new code with a clear error.
pub const VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePayload {
    pub lang: String,
    /// The roadmap's optional test date, as a day index. Not a timestamp:
    /// a date is what a teacher means, and a timestamp would carry a
    /// timezone and a time of day that nobody chose.
    pub test_day: Option<u32>,
    pub words: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// A list with no words is not a thing to share.
    NoWords,
    /// A word (or the language) contained the separator byte.
    SeparatorInText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    /// Carries the version seen, so the message can say what it was.
    UnknownVersion(u8),
    /// Shorter than a version byte plus a checksum.
    Truncated,
    /// Intact-arrival check failed: a mis-scan or a typo.
    Checksum,
    NotUtf8,
    Malformed(&'static str),
}

/// CRC-32/IEEE, computed rather than pulled in.
///
/// Public so a test can check it against the standard check value
/// (`crc32(b"123456789") == 0xCBF4_3926`). A hand-rolled checksum that is
/// merely stable, rather than actually CRC-32, would pass every round-trip
/// test in the suite and fail the first time anything else had to verify
/// one of these codes.
///
/// A dependency for fifteen lines of table-free arithmetic is the larger
/// risk in a wasm bundle, and the crate already hand-rolls `det_ln` and
/// `det_exp` in `learner.rs` for the same kind of reason.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn has_sep(s: &str) -> bool {
    s.as_bytes().contains(&SEP)
}

/// `version | lang SEP day SEP word SEP word ... | crc32(everything before)`
pub fn encode(p: &SharePayload) -> Result<Vec<u8>, EncodeError> {
    if p.words.is_empty() {
        return Err(EncodeError::NoWords);
    }
    if has_sep(&p.lang) || p.words.iter().any(|w| has_sep(w)) {
        return Err(EncodeError::SeparatorInText);
    }
    let mut out = Vec::with_capacity(32 + p.words.iter().map(|w| w.len() + 1).sum::<usize>());
    out.push(VERSION);
    out.extend_from_slice(p.lang.as_bytes());
    out.push(SEP);
    if let Some(d) = p.test_day {
        out.extend_from_slice(d.to_string().as_bytes());
    }
    for w in &p.words {
        out.push(SEP);
        out.extend_from_slice(w.as_bytes());
    }
    let crc = crc32(&out);
    out.extend_from_slice(&crc.to_be_bytes());
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<SharePayload, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Empty);
    }
    if bytes[0] != VERSION {
        return Err(DecodeError::UnknownVersion(bytes[0]));
    }
    if bytes.len() < 6 {
        return Err(DecodeError::Truncated);
    }
    let (body, tail) = bytes.split_at(bytes.len() - 4);
    let want = u32::from_be_bytes([tail[0], tail[1], tail[2], tail[3]]);
    if crc32(body) != want {
        return Err(DecodeError::Checksum);
    }
    let text = std::str::from_utf8(&body[1..]).map_err(|_| DecodeError::NotUtf8)?;
    let mut parts = text.split(SEP as char);
    let lang = parts.next().ok_or(DecodeError::Malformed("no language"))?.to_string();
    if lang.is_empty() {
        return Err(DecodeError::Malformed("empty language"));
    }
    let day_raw = parts.next().ok_or(DecodeError::Malformed("no date field"))?;
    let test_day = if day_raw.is_empty() {
        None
    } else {
        Some(day_raw.parse::<u32>().map_err(|_| DecodeError::Malformed("date is not a day index"))?)
    };
    let words: Vec<String> = parts.map(|w| w.to_string()).collect();
    if words.is_empty() || words.iter().any(|w| w.is_empty()) {
        return Err(DecodeError::Malformed("a list with no words, or an empty word"));
    }
    Ok(SharePayload { lang, test_day, words })
}

/// What a QR would have to carry, so a caller can decide before rendering.
/// Exposed because "does a 20-word Russian list fit in a code a parent can
/// scan across a classroom" is a question about this number.
pub fn encoded_len(p: &SharePayload) -> Option<usize> {
    encode(p).ok().map(|b| b.len())
}
