"""SQLite data layer for The Climb (player accounts + leaderboard).

Cloudflare D1 is SQLite under the hood, so this schema ports directly if the
backend ever moves to Workers + D1 (the spec's alternative). The DB file lives
on a Docker volume (see docker-compose.yml) so it survives container rebuilds.

Only the leaderboard categories medium/hard/expert exist — there is deliberately
NO 'easy' board (enforced by the CHECK constraint below). Phone numbers are
stored only when provided and are never returned to clients.
"""

import os
import sqlite3
import threading

DB_PATH = os.environ.get("CLIMB_DB_PATH", "/data/climb/climb.db")

_local = threading.local()

SCHEMA = """
CREATE TABLE IF NOT EXISTS users (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  username            TEXT NOT NULL,
  username_lc         TEXT NOT NULL UNIQUE,        -- case-insensitive uniqueness
  display_name        TEXT,
  email               TEXT NOT NULL,
  email_lc            TEXT NOT NULL UNIQUE,
  email_verified      INTEGER NOT NULL DEFAULT 0,
  phone               TEXT,                        -- only if provided; never displayed
  phone_verified      INTEGER NOT NULL DEFAULT 0,
  pw_hash             TEXT NOT NULL,               -- bcrypt; plaintext never stored/logged
  created_at          REAL NOT NULL,
  username_changed_at REAL
);

-- Old usernames are reserved for 30 days after a rename so they can't be
-- immediately grabbed to impersonate the previous holder.
CREATE TABLE IF NOT EXISTS reserved_usernames (
  username_lc    TEXT PRIMARY KEY,
  reserved_until REAL NOT NULL
);

-- Opaque random session tokens (httpOnly cookie on web; Capacitor Preferences
-- in the app). 90-day rolling expiry — refreshed on use.
CREATE TABLE IF NOT EXISTS sessions (
  token      TEXT PRIMARY KEY,
  user_id    INTEGER NOT NULL,
  created_at REAL NOT NULL,
  expires_at REAL NOT NULL,
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS email_verifications (
  token      TEXT PRIMARY KEY,
  user_id    INTEGER NOT NULL,
  expires_at REAL NOT NULL,
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Email (link) and SMS (OTP) password resets share this table. Email rows have
-- code_hash NULL; SMS rows store a bcrypt hash of the 6-digit code.
CREATE TABLE IF NOT EXISTS password_resets (
  token      TEXT PRIMARY KEY,
  user_id    INTEGER NOT NULL,
  kind       TEXT NOT NULL CHECK (kind IN ('email','sms')),
  code_hash  TEXT,
  expires_at REAL NOT NULL,
  attempts   INTEGER NOT NULL DEFAULT 0,
  used       INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- One row per player per category (upsert on a new record). NO 'easy'.
CREATE TABLE IF NOT EXISTS leaderboard_entries (
  user_id     INTEGER NOT NULL,
  difficulty  TEXT NOT NULL CHECK (difficulty IN ('medium','hard','expert')),
  best_chain  INTEGER NOT NULL,
  achieved_at REAL NOT NULL,
  run_meta    TEXT,                                -- JSON: word_count, duration_ms, ...
  PRIMARY KEY (user_id, difficulty),
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS name_reports (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  reported_user_id INTEGER NOT NULL,
  reporter_user_id INTEGER,
  created_at       REAL NOT NULL,
  FOREIGN KEY(reported_user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Per-account submission log, for rate-limiting chain submissions.
CREATE TABLE IF NOT EXISTS submit_log (
  user_id INTEGER NOT NULL,
  ts      REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_submit_log_user ON submit_log(user_id, ts);
-- Ranking: highest chain first, ties broken by earliest achieved.
CREATE INDEX IF NOT EXISTS idx_lb_rank ON leaderboard_entries(difficulty, best_chain DESC, achieved_at ASC);
"""


def conn() -> sqlite3.Connection:
    """A per-thread connection (Flask/gunicorn workers are threaded)."""
    c = getattr(_local, "conn", None)
    if c is None:
        c = sqlite3.connect(DB_PATH)
        c.row_factory = sqlite3.Row
        c.execute("PRAGMA foreign_keys = ON")
        c.execute("PRAGMA journal_mode = WAL")
        _local.conn = c
    return c


def init() -> None:
    """Create the schema if needed. Safe to call on every startup."""
    parent = os.path.dirname(DB_PATH)
    if parent:
        os.makedirs(parent, exist_ok=True)
    c = conn()
    c.executescript(SCHEMA)
    c.commit()
