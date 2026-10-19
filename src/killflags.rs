//! CC-IMPOSSIBLE-COMPLETE F4 -- the remote KILL-ONLY flag file.
//!
//! Turning a feature ON stays a deliberate act inside a build (D-C6). Turning one OFF in an
//! emergency should not wait for app review. The app fetches one small signed document from our
//! own backend (`/api/kill-flags`), verifies it against the Ed25519 public key compiled in below,
//! and caches it. The document can only NAME flags to switch off. There is no way to say "on", so
//! the file can never turn anything on (I6), and anything unreadable, unreachable, unsigned or
//! mis-signed leaves the build's own values exactly as they were.
//!
//! Document:  {"v":1,"issued":"2026-10-19T12:00:00Z","kill":["impossible_fr"],"sig":"<128 hex>"}
//! Signed message: "spell-kill-v1\n<issued>\n<kill names, sorted, one per line>".
//! The private key lives on the server, outside git (~/spellgame-server/keys/kill-flags.pem), and
//! `tools/killflags/sign.py` is the one way to produce a document. No third-party remote-config SDK
//! is involved, the request carries no data, and the fetch is to our own origin.
//!
//! Replay: a document older than the cached one is refused, so replaying an old "all clear" cannot
//! quietly re-enable a flag that was killed since.

use std::cell::RefCell;
use std::collections::HashSet;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// The public half of the signing key (raw 32 bytes, hex).
pub const PUBLIC_KEY_HEX: &str = "361cc272a35ef4348ad1040beef7be1b79e76b9618f7892e2ceed45c98b3deb9";

/// localStorage key holding the last verified document, verbatim.
pub const STORE_KEY: &str = "spell_kill_flags_v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub issued: String,
    pub kill: Vec<String>,
}

fn hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// A flag name: lowercase letters, digits and underscores, bounded. Anything else is refused so a
/// signed document cannot smuggle odd keys into storage lookups.
fn plausible_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 64 && n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The exact bytes that are signed.
pub fn message(issued: &str, kill: &[String]) -> String {
    let mut names: Vec<&str> = kill.iter().map(String::as_str).collect();
    names.sort_unstable();
    format!("spell-kill-v1\n{issued}\n{}", names.join("\n"))
}

/// Verify a document against `key_hex`. `None` for anything that is not a well-formed,
/// correctly signed v1 document. Fields other than v / issued / kill / sig are never read, so a
/// document cannot carry an "on" list even by accident.
pub fn verify_with(doc: &str, key_hex: &str) -> Option<Verified> {
    let v: serde_json::Value = serde_json::from_str(doc).ok()?;
    if v.get("v")?.as_u64()? != 1 {
        return None;
    }
    let issued = v.get("issued")?.as_str()?.to_string();
    if issued.is_empty() || issued.len() > 40 || issued.contains('\n') {
        return None;
    }
    let kill: Vec<String> = v.get("kill")?.as_array()?.iter().map(|x| x.as_str().map(str::to_string)).collect::<Option<_>>()?;
    if kill.len() > 200 || !kill.iter().all(|n| plausible_name(n)) {
        return None;
    }
    let sig: [u8; 64] = hex(v.get("sig")?.as_str()?)?.try_into().ok()?;
    let key: [u8; 32] = hex(key_hex)?.try_into().ok()?;
    let key = VerifyingKey::from_bytes(&key).ok()?;
    key.verify(message(&issued, &kill).as_bytes(), &Signature::from_bytes(&sig)).ok()?;
    Some(Verified { issued, kill })
}

pub fn verify(doc: &str) -> Option<Verified> {
    verify_with(doc, PUBLIC_KEY_HEX)
}

thread_local! {
    static STATE: RefCell<Option<Verified>> = const { RefCell::new(None) };
}

/// Is this flag switched off by the signed file? Pure memory read; the flags module calls it on
/// every flag read.
pub fn is_killed(name: &str) -> bool {
    STATE.with(|s| s.borrow().as_ref().is_some_and(|v| v.kill.iter().any(|k| k == name)))
}

/// Offer a document (fetched, or read back from the cache). Returns true when it was adopted:
/// valid, and not older than what we already hold. Returns false and changes nothing otherwise.
pub fn adopt_with(doc: &str, key_hex: &str) -> bool {
    let Some(v) = verify_with(doc, key_hex) else { return false };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.as_ref().is_some_and(|cur| v.issued < cur.issued) {
            return false;
        }
        *s = Some(v);
        true
    })
}

pub fn adopt(doc: &str) -> bool {
    adopt_with(doc, PUBLIC_KEY_HEX)
}

/// Test seam: forget everything.
#[cfg(test)]
pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = None);
}

/// Boot: load and re-verify the cached document. A cache that no longer verifies (a rotated key, a
/// corrupted entry) is ignored, never trusted.
pub fn init() {
    if let Some(doc) = crate::storage::get_raw(STORE_KEY) {
        adopt(&doc);
    }
}

/// Fetch the current document from our own backend and adopt it if it verifies. Any failure
/// (offline, 204, bad signature) leaves the build's values and the cached document untouched.
pub async fn refresh() {
    let url = format!("{}/api/kill-flags", crate::api::api_base());
    let Ok(text) = crate::storage::fetch_text(&url).await else { return };
    if adopt(&text) {
        crate::storage::set_raw(STORE_KEY, &text);
    }
}

#[cfg(test)]
mod tests;
