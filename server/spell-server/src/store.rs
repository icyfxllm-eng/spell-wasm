//! F2 + §9 — the only persistent state: anonymous players.
//!
//! The database holds exactly the §9 columns. The plaintext token exists in
//! memory for the length of one request and is never written anywhere; only
//! its SHA-256 is stored (A2.1).

use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

/// Tokens expire after this many days without use (sliding, F2).
pub const TOKEN_TTL_DAYS: i64 = 90;
/// Rows inactive this long are deleted (§9).
pub const RETENTION_DAYS: i64 = 365;

/// Wall-clock days since the Unix epoch, with a test offset.
#[derive(Clone, Default)]
pub struct Clock {
    offset_days: Arc<AtomicI64>,
}

impl Clock {
    pub fn today(&self) -> i64 {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        secs / 86_400 + self.offset_days.load(Ordering::Relaxed)
    }

    pub fn unix_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Tests only: move the calendar forward.
    pub fn advance_days(&self, days: i64) {
        self.offset_days.fetch_add(days, Ordering::Relaxed);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Auth {
    Ok { player_id: String },
    Expired,
    Revoked,
    Unknown,
}

pub struct Store {
    conn: Mutex<Connection>,
    pub clock: Clock,
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("OS randomness");
    b
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

impl Store {
    pub fn open(path: &str, clock: Clock) -> rusqlite::Result<Store> {
        if let Some(dir) = Path::new(path).parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS players (
               player_id     TEXT PRIMARY KEY,
               token_hash    BLOB NOT NULL UNIQUE,
               created_day   INTEGER NOT NULL,
               last_seen_day INTEGER NOT NULL,
               revoked       INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS _probe (id INTEGER PRIMARY KEY, at INTEGER NOT NULL);",
        )?;
        Ok(Store { conn: Mutex::new(conn), clock })
    }

    pub fn in_memory() -> Store {
        Store::open(":memory:", Clock::default()).expect("in-memory sqlite")
    }

    /// `POST /v1/session`: a fresh anonymous identity.
    pub fn mint(&self) -> rusqlite::Result<(String, String)> {
        let player_id = hex::encode(random_bytes::<16>());
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>());
        let today = self.clock.today();
        self.conn.lock().unwrap().execute(
            "INSERT INTO players(player_id, token_hash, created_day, last_seen_day, revoked) VALUES (?1, ?2, ?3, ?3, 0)",
            params![player_id, hash_token(&token), today],
        )?;
        Ok((player_id, token))
    }

    /// Checks a token, sliding its expiry on success.
    pub fn authenticate(&self, token: &str) -> rusqlite::Result<Auth> {
        let today = self.clock.today();
        let c = self.conn.lock().unwrap();
        let row: Option<(String, i64, i64)> = c
            .query_row(
                "SELECT player_id, last_seen_day, revoked FROM players WHERE token_hash = ?1",
                params![hash_token(token)],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((player_id, last_seen, revoked)) = row else { return Ok(Auth::Unknown) };
        if revoked != 0 {
            return Ok(Auth::Revoked);
        }
        if today - last_seen > TOKEN_TTL_DAYS {
            return Ok(Auth::Expired);
        }
        if last_seen != today {
            c.execute("UPDATE players SET last_seen_day = ?1 WHERE player_id = ?2", params![today, player_id])?;
        }
        Ok(Auth::Ok { player_id })
    }

    /// `POST /admin/revoke/{player_id}`. Returns whether a row changed.
    pub fn revoke(&self, player_id: &str) -> rusqlite::Result<bool> {
        let n = self
            .conn
            .lock()
            .unwrap()
            .execute("UPDATE players SET revoked = 1 WHERE player_id = ?1", params![player_id])?;
        Ok(n > 0)
    }

    /// §9 retention: delete rows inactive for 12 months.
    pub fn sweep(&self) -> rusqlite::Result<usize> {
        let cutoff = self.clock.today() - RETENTION_DAYS;
        self.conn.lock().unwrap().execute("DELETE FROM players WHERE last_seen_day < ?1", params![cutoff])
    }

    /// The healthz probe: the database accepts a write.
    pub fn probe(&self) -> bool {
        let now = self.clock.unix_ms() as i64;
        self.conn
            .lock()
            .map(|c| c.execute("INSERT OR REPLACE INTO _probe(id, at) VALUES (1, ?1)", params![now]).is_ok())
            .unwrap_or(false)
    }

    /// `POST /admin/snapshot`: a consistent copy for backup.sh.
    pub fn snapshot(&self, dest: &str) -> rusqlite::Result<()> {
        if let Some(dir) = Path::new(dest).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::remove_file(dest);
        self.conn.lock().unwrap().execute("VACUUM INTO ?1", params![dest])?;
        Ok(())
    }

    pub fn player_count(&self) -> i64 {
        self.conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM players", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// A-DATA.1 helper: `(table, [columns])` for every table.
    pub fn schema(&self) -> Vec<(String, Vec<String>)> {
        let c = self.conn.lock().unwrap();
        let mut tables: Vec<String> = c
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        tables.retain(|t: &String| !t.starts_with("sqlite_"));
        tables
            .into_iter()
            .map(|t| {
                let cols: Vec<String> = c
                    .prepare(&format!("PRAGMA table_info({t})"))
                    .unwrap()
                    .query_map([], |r| r.get(1))
                    .unwrap()
                    .map(Result::unwrap)
                    .collect();
                (t, cols)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_then_authenticate() {
        let s = Store::in_memory();
        let (id, token) = s.mint().unwrap();
        assert_eq!(id.len(), 32, "128-bit hex player id");
        assert_eq!(token.len(), 43, "256-bit base64url token");
        assert_eq!(s.authenticate(&token).unwrap(), Auth::Ok { player_id: id });
        assert_eq!(s.authenticate("nope").unwrap(), Auth::Unknown);
    }

    #[test]
    fn a2_3_expiry_slides_and_revocation_sticks() {
        let s = Store::in_memory();
        let (id, token) = s.mint().unwrap();
        s.clock.advance_days(80);
        assert!(matches!(s.authenticate(&token).unwrap(), Auth::Ok { .. }), "use at day 80 slides the window");
        s.clock.advance_days(80);
        assert!(matches!(s.authenticate(&token).unwrap(), Auth::Ok { .. }), "day 160 is 80 after last use");
        s.clock.advance_days(TOKEN_TTL_DAYS + 1);
        assert_eq!(s.authenticate(&token).unwrap(), Auth::Expired);
        let (id2, token2) = s.mint().unwrap();
        assert!(s.revoke(&id2).unwrap());
        assert_eq!(s.authenticate(&token2).unwrap(), Auth::Revoked);
        assert!(!s.revoke("missing").unwrap());
        let _ = id;
    }

    #[test]
    fn a_data_1_schema_is_exactly_the_inventory() {
        let s = Store::in_memory();
        let schema = s.schema();
        assert_eq!(
            schema,
            vec![
                ("_probe".to_string(), vec!["id".to_string(), "at".to_string()]),
                (
                    "players".to_string(),
                    ["player_id", "token_hash", "created_day", "last_seen_day", "revoked"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                ),
            ]
        );
    }

    #[test]
    fn retention_sweep_drops_a_year_of_silence() {
        let s = Store::in_memory();
        let (_, token) = s.mint().unwrap();
        s.clock.advance_days(RETENTION_DAYS + 1);
        assert_eq!(s.sweep().unwrap(), 1);
        assert_eq!(s.authenticate(&token).unwrap(), Auth::Unknown);
    }
}
