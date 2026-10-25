//! A1.1: a golden snapshot of every message type's exact wire form, and a
//! round trip through the decoder. A protocol change shows up here as a diff
//! someone has to read, never as a silent drift old App Store builds discover.
//!
//! Regenerate with `UPDATE_GOLDEN=1` after a deliberate change.

use spell_server::protocol::{decode, decode_server, encode_client, encode_server, samples};
use uuid::Uuid;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/protocol.txt");

fn render() -> String {
    let cid = Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap();
    let mut out = String::from("# client -> server\n");
    for m in samples::client() {
        let text = encode_client(cid, &m);
        assert_eq!(decode(&text).expect(&text).msg, m, "client round trip");
        out.push_str(&canonical(&text));
        out.push('\n');
    }
    out.push_str("# server -> client\n");
    for (i, m) in samples::server().into_iter().enumerate() {
        let text = encode_server(i as u64 + 1, &m);
        assert_eq!(decode_server(&text).expect(&text).msg, m, "server round trip");
        out.push_str(&canonical(&text));
        out.push('\n');
    }
    out
}

/// Keys sorted, so the golden is stable regardless of map ordering.
fn canonical(text: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    serde_json::to_string(&sort(v)).unwrap()
}

fn sort(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => {
            let b: std::collections::BTreeMap<_, _> = m.into_iter().map(|(k, v)| (k, sort(v))).collect();
            serde_json::to_value(b).unwrap()
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(sort).collect()),
        x => x,
    }
}

#[test]
fn a1_1_every_message_type_matches_its_golden_snapshot() {
    let now = render();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(PATH, &now).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(PATH).expect("golden exists: run with UPDATE_GOLDEN=1");
    assert_eq!(now, golden, "the wire format changed");
}
