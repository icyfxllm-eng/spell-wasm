//! The real binary: A4.2 (refuses a public bind with trust-proxy, non-zero,
//! with a clear message) and A11.3's last clause (SIGTERM drains and the
//! process exits 0).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_spell-server");

fn temp_db() -> String {
    let dir = std::env::temp_dir().join(format!("spell-server-proc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("spell.db").to_string_lossy().into_owned()
}

#[test]
fn a4_2_a_public_bind_with_trust_proxy_exits_non_zero_with_a_reason() {
    let out = Command::new(BIN)
        .env_clear()
        .env("SPELL_TTS_BASE", "http://127.0.0.1:9")
        .env("SPELL_BIND_PUBLIC", "0.0.0.0:18080")
        .env("SPELL_TRUST_PROXY", "true")
        .env("SPELL_DB_PATH", temp_db())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("not loopback") && err.contains("CF-Connecting-IP"), "{err}");
}

#[test]
fn a_missing_tts_base_exits_non_zero() {
    let out = Command::new(BIN).env_clear().env("SPELL_DB_PATH", temp_db()).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("SPELL_TTS_BASE"));
}

#[test]
fn a11_3_sigterm_drains_and_exits_zero() {
    let port = 18_000 + (std::process::id() % 1000) as u16;
    let mut child = Command::new(BIN)
        .env_clear()
        .env("SPELL_TTS_BASE", "http://127.0.0.1:9")
        .env("SPELL_BIND_PUBLIC", format!("127.0.0.1:{port}"))
        .env("SPELL_BIND_ADMIN", format!("127.0.0.1:{}", port + 1))
        .env("SPELL_DB_PATH", temp_db())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Wait for the listener.
    let start = Instant::now();
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(start.elapsed() < Duration::from_secs(10), "server never listened");
        std::thread::sleep(Duration::from_millis(50));
    }
    let ok = Command::new("kill").arg("-TERM").arg(child.id().to_string()).status().unwrap();
    assert!(ok.success());
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no exit after SIGTERM with no live matches");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(status.code(), Some(0));
}
