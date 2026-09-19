-- F5 client side — bucket histograms per metric and language, last 14 days,
-- plus the share of word plays that found no audio at all.
SELECT metric,
       CASE WHEN source = 'aggregate' THEN '(aggregate)' ELSE lang END AS lang,
       bucket,
       SUM(n) AS n
FROM perf_counts
WHERE day >= date('now', '-14 day')
GROUP BY metric, 2, bucket
ORDER BY metric, lang, bucket;
