-- Acceptance 9 — crash counts by error_code x language x build, last 14 days.
-- Standard players only have a language; Jr / unknown / Education tallies are
-- reported on their own line with lang '(aggregate)'.
SELECT build,
       error_code,
       CASE WHEN source = 'aggregate' THEN '(aggregate)' ELSE lang END AS lang,
       SUM(n) AS n
FROM daily_counts
WHERE day >= date('now', '-14 day')
GROUP BY build, error_code, 3
ORDER BY n DESC, build, error_code, lang;
