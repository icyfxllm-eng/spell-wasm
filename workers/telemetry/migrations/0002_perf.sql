-- CC-TELEMETRY-FOUNDATION v1.1 F5: histogram cells per day, kept (they are
-- already aggregates). source = 'events' (standard, with lang) or 'aggregate'
-- (Jr / unknown / Education: lang is always '').
CREATE TABLE perf_counts (
  day TEXT NOT NULL,
  build TEXT NOT NULL,
  platform TEXT NOT NULL,
  source TEXT NOT NULL,
  metric TEXT NOT NULL,
  bucket TEXT NOT NULL,
  lang TEXT NOT NULL,
  n INTEGER NOT NULL,
  PRIMARY KEY (day, build, platform, source, metric, bucket, lang)
);
