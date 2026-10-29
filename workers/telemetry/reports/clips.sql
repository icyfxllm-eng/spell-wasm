-- CLIP-REPORT: clips by number of "Can't tell" reports, last 90 days.
-- scripts/clip_report.py turns item_hash into words.
SELECT lang, item_hash, variant, source, COUNT(*) AS reports
FROM clip_reports
WHERE day >= date('now', '-90 days')
GROUP BY lang, item_hash, variant, source
ORDER BY reports DESC, lang, item_hash
LIMIT 500;
