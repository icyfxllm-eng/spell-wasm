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
import time

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
  locale      TEXT NOT NULL DEFAULT 'en',         -- word language of the run (§4.4)
  best_chain  INTEGER NOT NULL,
  achieved_at REAL NOT NULL,
  run_meta    TEXT,                                -- JSON: word_count, duration_ms, ...
  PRIMARY KEY (user_id, difficulty, locale),
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- CC-ATTEMPTS-SHIELDS / PD1 season reset: when shields ship, The Climb's active
-- leaderboard is reset so every ranked score sits under the shield ruleset. That
-- reset is an ARCHIVE, never a deletion — prior scores are copied here first.
-- One row per (player, board, season); `season` distinguishes archived eras.
CREATE TABLE IF NOT EXISTS leaderboard_archive (
  user_id     INTEGER NOT NULL,
  difficulty  TEXT NOT NULL,
  locale      TEXT NOT NULL DEFAULT 'en',
  best_chain  INTEGER NOT NULL,
  achieved_at REAL NOT NULL,
  run_meta    TEXT,
  season      INTEGER NOT NULL,
  archived_at REAL NOT NULL,
  PRIMARY KEY (user_id, difficulty, locale, season),
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
-- Ranking within a (difficulty, locale) board: highest chain first, ties by earliest.
CREATE INDEX IF NOT EXISTS idx_lb_rank ON leaderboard_entries(difficulty, locale, best_chain DESC, achieved_at ASC);

-- Async 1v1 "Spell Off" matches (see matches.py). Two friends spell the SAME
-- words — derived deterministically client-side from the server-owned `seed` —
-- on their own time; the server compares results and declares the winner. The
-- seed lives server-side so it's the integrity foundation for a later
-- server-side re-derivation + answer/timing audit (see the TODO in matches.py).
-- Account-gated only (no anonymous/Kid-Mode/stranger play) — COPPA-safe.
CREATE TABLE IF NOT EXISTS matches (
  code        TEXT PRIMARY KEY,               -- short share code (8 chars)
  seed        TEXT NOT NULL,                  -- server-generated hex (the shared PRNG seed)
  lang        TEXT NOT NULL,
  tier        TEXT NOT NULL,
  word_count  INTEGER NOT NULL,
  player_a    INTEGER NOT NULL,               -- creator (user id)
  player_b    INTEGER,                        -- joiner (user id), NULL while open
  status      TEXT NOT NULL DEFAULT 'open'    -- open | active | complete
                CHECK (status IN ('open','active','complete')),
  score_a     INTEGER, correct_a INTEGER, elapsed_a INTEGER,
  score_b     INTEGER, correct_b INTEGER, elapsed_b INTEGER,
  winner      TEXT,                           -- 'a' | 'b' | 'tie' (once complete)
  created_at  REAL NOT NULL,
  expires_at  REAL NOT NULL,
  FOREIGN KEY(player_a) REFERENCES users(id) ON DELETE CASCADE,
  FOREIGN KEY(player_b) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_matches_players ON matches(player_a, player_b);
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
    # Migrate BEFORE running SCHEMA: the SCHEMA index references the locale
    # column, which an old table won't have until this rebuild adds it.
    _migrate_leaderboard_locale(c)
    c.executescript(SCHEMA)
    c.commit()


def archive_active_season(c=None, now=None) -> int:
    """PD1 season reset — ARCHIVE, never delete.

    Copies every current `leaderboard_entries` row into `leaderboard_archive`
    under a fresh season number, then clears the active table so the live
    leaderboard returns zero entries while all prior scores survive in the
    archive. A brand-new run posts and ranks normally afterward.

    IDEMPOTENT: with the active table already empty (a second run, or a
    re-invocation), it copies nothing, allocates no new season, and returns 0 —
    a safe no-op. Returns the number of rows archived.

    NOT wired into `init()` or any request path: this runs exactly once, by hand,
    at the v2 (shields) release. Importing/creating the schema never triggers it.
    """
    c = c or conn()
    now = time.time() if now is None else now
    active = c.execute("SELECT COUNT(*) AS n FROM leaderboard_entries").fetchone()["n"]
    if active == 0:
        return 0  # nothing live to archive -> no-op (idempotent)
    season = (c.execute("SELECT COALESCE(MAX(season), 0) AS s FROM leaderboard_archive").fetchone()["s"]) + 1
    c.execute(
        "INSERT INTO leaderboard_archive"
        "(user_id, difficulty, locale, best_chain, achieved_at, run_meta, season, archived_at) "
        "SELECT user_id, difficulty, locale, best_chain, achieved_at, run_meta, ?, ? "
        "FROM leaderboard_entries",
        (season, now),
    )
    c.execute("DELETE FROM leaderboard_entries")
    c.commit()
    return active


def _migrate_leaderboard_locale(c) -> None:
    """Add the per-locale segmentation (§4.4) to a leaderboard_entries table that
    predates it. Existing rows backfill as locale='en'. SQLite can't alter a
    primary key in place, so rebuild the table when the column is missing."""
    exists = c.execute(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='leaderboard_entries'"
    ).fetchone()
    if not exists:
        return  # fresh DB — SCHEMA will create it with the locale column
    cols = [r["name"] for r in c.execute("PRAGMA table_info(leaderboard_entries)").fetchall()]
    if "locale" in cols:
        return
    c.executescript(
        """
        DROP INDEX IF EXISTS idx_lb_rank;
        ALTER TABLE leaderboard_entries RENAME TO leaderboard_entries_old;
        CREATE TABLE leaderboard_entries (
          user_id     INTEGER NOT NULL,
          difficulty  TEXT NOT NULL CHECK (difficulty IN ('medium','hard','expert')),
          locale      TEXT NOT NULL DEFAULT 'en',
          best_chain  INTEGER NOT NULL,
          achieved_at REAL NOT NULL,
          run_meta    TEXT,
          PRIMARY KEY (user_id, difficulty, locale),
          FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        INSERT INTO leaderboard_entries(user_id, difficulty, locale, best_chain, achieved_at, run_meta)
          SELECT user_id, difficulty, 'en', best_chain, achieved_at, run_meta FROM leaderboard_entries_old;
        DROP TABLE leaderboard_entries_old;
        CREATE INDEX IF NOT EXISTS idx_lb_rank ON leaderboard_entries(difficulty, locale, best_chain DESC, achieved_at ASC);
        """
    )
    c.commit()
