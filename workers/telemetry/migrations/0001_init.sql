-- CC-TELEMETRY-FOUNDATION v1.1. Columns mirror schema.json; no column can
-- hold an IP, a user agent, a location, an account, or free text.

-- Raw standard-player errors. Deleted after 90 days (D6).
CREATE TABLE events (
  day TEXT NOT NULL,          -- UTC date received, YYYY-MM-DD
  build TEXT NOT NULL,
  platform TEXT NOT NULL,
  session_id TEXT NOT NULL,   -- random per app launch (D4)
  error_code TEXT NOT NULL,
  lang TEXT NOT NULL,
  mode TEXT NOT NULL,
  stack_hash TEXT NOT NULL
);
CREATE INDEX events_day ON events (day);

-- Per-day counts, kept indefinitely. source = 'events' (standard, with lang)
-- or 'aggregate' (Jr / unknown / Education: lang is always '').
CREATE TABLE daily_counts (
  day TEXT NOT NULL,
  build TEXT NOT NULL,
  platform TEXT NOT NULL,
  source TEXT NOT NULL,
  error_code TEXT NOT NULL,
  lang TEXT NOT NULL,
  n INTEGER NOT NULL,
  PRIMARY KEY (day, build, platform, source, error_code, lang)
);
