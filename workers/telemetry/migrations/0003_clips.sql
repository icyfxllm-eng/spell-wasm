-- CLIP-REPORT. One row per "Can't tell" report from a standard player. No
-- session id, no IP, no account: two reports from one launch are not linked.
-- item_hash is FNV-1a of "<lang>|<bank entry>"; the report script looks it up
-- against the banks. Deleted after 90 days (D6), like raw events.
CREATE TABLE clip_reports (
  day TEXT NOT NULL,          -- UTC date received, YYYY-MM-DD
  build TEXT NOT NULL,
  platform TEXT NOT NULL,
  lang TEXT NOT NULL,
  item_hash TEXT NOT NULL,
  variant TEXT NOT NULL,
  source TEXT NOT NULL
);
CREATE INDEX clip_reports_day ON clip_reports (day);
