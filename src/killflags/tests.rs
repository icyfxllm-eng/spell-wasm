//! CC-IMPOSSIBLE-COMPLETE A-7: the kill-only flag file. Unreachable, unsigned, mis-signed,
//! tampered, replayed and "on" documents all leave the build's values alone; only a correctly
//! signed list can switch a flag OFF.

use super::*;
use ed25519_dalek::{Signer, SigningKey};

/// A document produced by tools/killflags/sign.py with the REAL key, 2026-10-19: proves the shipped
/// public key verifies what the shipped signer makes (openssl on one side, ed25519-dalek on the other).
const REAL_DOC: &str = r#"{
 "v": 1,
 "issued": "2026-10-19T12:00:00Z",
 "kill": ["impossible_fr", "impossible_mix"],
 "sig": "bd2e970d4e27620794019a061fe849b2c3a9f7704f98811d4a6f3af78aa23a823657dd22db41ad6ac079cd4ff1bb3162fbf6fb739a77e09b3d59f9408199180e"
}"#;

fn test_key() -> (SigningKey, String) {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk: String = sk.verifying_key().to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    (sk, pk)
}

fn signed(sk: &SigningKey, issued: &str, kill: &[&str]) -> String {
    let names: Vec<String> = kill.iter().map(|s| s.to_string()).collect();
    let sig: String = sk.sign(message(issued, &names).as_bytes()).to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    serde_json::json!({ "v": 1, "issued": issued, "kill": kill, "sig": sig }).to_string()
}

#[test]
fn the_shipped_key_verifies_the_shipped_signer() {
    reset();
    let v = verify(REAL_DOC).expect("the real-key document must verify");
    assert_eq!(v.kill, vec!["impossible_fr", "impossible_mix"]);
    assert!(adopt(REAL_DOC));
    assert!(is_killed("impossible_fr") && is_killed("impossible_mix") && !is_killed("impossible_es"));
    reset();
}

#[test]
fn nothing_loaded_leaves_the_builds_values() {
    reset();
    assert!(!is_killed("impossible") && !is_killed("anything"));
    crate::flags::set_test_override(Some("on"));
    assert!(crate::flags::impossible(), "file unreachable: the build's own value applies");
    crate::flags::set_test_override(None);
}

#[test]
fn a_signed_list_switches_a_flag_off_and_nothing_else() {
    reset();
    let (sk, pk) = test_key();
    assert!(adopt_with(&signed(&sk, "2026-10-19T00:00:00Z", &["impossible_fr"]), &pk));
    assert!(is_killed("impossible_fr") && !is_killed("impossible_de"));
    reset();
}

#[test]
fn bad_documents_are_ignored() {
    reset();
    let (sk, pk) = test_key();
    let good = signed(&sk, "2026-10-19T00:00:00Z", &["impossible_fr"]);
    // Wrong key: a document signed by someone else.
    let other = SigningKey::from_bytes(&[9u8; 32]);
    assert!(!adopt_with(&signed(&other, "2026-10-19T00:00:00Z", &["impossible_fr"]), &pk));
    // Tampered: the kill list changed after signing.
    assert!(!adopt_with(&good.replace("impossible_fr", "impossible_es"), &pk));
    // Tampered: the issue time changed.
    assert!(!adopt_with(&good.replace("2026-10-19", "2026-10-20"), &pk));
    // Missing / short / non-hex signature, wrong version, junk.
    assert!(!adopt_with(r#"{"v":1,"issued":"x","kill":[]}"#, &pk));
    assert!(!adopt_with(&good.replace("\"v\":1", "\"v\":2"), &pk));
    assert!(!adopt_with("not json", &pk));
    assert!(!adopt_with("", &pk));
    // A name that could not be a flag.
    assert!(!adopt_with(&signed(&sk, "2026-10-19T00:00:00Z", &["Impossible FR"]), &pk));
    assert!(!is_killed("impossible_fr"), "nothing above may have been adopted");
}

/// I6: the file can never turn a flag on. An "on" list in a signed document is never read, and a
/// kill cannot lift a flag the build has off.
#[test]
fn the_file_can_never_turn_a_flag_on() {
    reset();
    let (sk, pk) = test_key();
    let mut v: serde_json::Value = serde_json::from_str(&signed(&sk, "2026-10-19T00:00:00Z", &[])).unwrap();
    v["on"] = serde_json::json!(["impossible", "impossible_fr"]);
    v["enable"] = serde_json::json!(["impossible_mix"]);
    assert!(adopt_with(&v.to_string(), &pk), "extra fields do not break a valid document");
    crate::flags::set_test_override(None);
    assert!(!crate::flags::impossible() && !crate::flags::impossible_lang("fr") && !crate::flags::impossible_mix(), "build says off, stays off");
    reset();
}

/// The file reaches real flag reads: a flag the build has ON is OFF while it is named.
#[test]
fn a_killed_flag_reads_off_through_the_flags_module() {
    reset();
    let (sk, pk) = test_key();
    crate::flags::set_test_override(Some("on"));
    assert!(crate::flags::impossible() && crate::flags::impossible_lang("fr"));
    assert!(adopt_with(&signed(&sk, "2026-10-19T00:00:00Z", &["impossible_fr", "impossible"]), &pk));
    assert!(!crate::flags::impossible_lang("fr"), "impossible_fr is killed");
    assert!(!crate::flags::impossible(), "impossible is killed");
    assert!(crate::flags::impossible_lang("de"), "an unnamed flag keeps the build's value");
    crate::flags::set_test_override(None);
    reset();
}

/// Replay: an older signed document cannot undo a newer one; a newer all-clear does.
#[test]
fn a_replayed_old_document_is_refused() {
    reset();
    let (sk, pk) = test_key();
    assert!(adopt_with(&signed(&sk, "2026-10-19T10:00:00Z", &["impossible_fr"]), &pk));
    assert!(!adopt_with(&signed(&sk, "2026-10-19T09:00:00Z", &[]), &pk), "older all-clear must not re-enable");
    assert!(is_killed("impossible_fr"));
    assert!(adopt_with(&signed(&sk, "2026-10-19T11:00:00Z", &[]), &pk), "a newer all-clear lifts the kill");
    assert!(!is_killed("impossible_fr"));
    reset();
}

/// The Rust side reads exactly what the Python signer writes: the message format is pinned.
#[test]
fn message_format_is_pinned() {
    let kill = vec!["b".to_string(), "a".to_string()];
    assert_eq!(message("T", &kill), "spell-kill-v1\nT\na\nb");
    assert_eq!(message("T", &[]), "spell-kill-v1\nT\n");
    assert!(include_str!("../../tools/killflags/sign.py").contains("\"spell-kill-v1\\n\""));
}
