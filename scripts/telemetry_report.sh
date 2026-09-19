#!/usr/bin/env bash
# CC-TELEMETRY-FOUNDATION v1.1 acceptance 9 — the usefulness report.
#
#   bash scripts/telemetry_report.sh            # after the Worker is deployed
#
# 1. Crash counts by error_code x language x build (D1, last 14 days).
# 2. Client performance buckets per metric and language (D1, last 14 days).
# 3. Mac mini TTS p50/p95, cache hit rate and error rate per language (R8 lines).
# If (1) is empty for a language with players, or (3) is empty, the pipeline is
# broken (acceptance 9).
set -euo pipefail
cd "$(dirname "$0")/.."
W=workers/telemetry
echo "== crashes (14 days)"
(cd "$W" && npx wrangler d1 execute spell-telemetry --remote --file reports/crashes.sql)
echo "== client performance (14 days)"
(cd "$W" && npx wrangler d1 execute spell-telemetry --remote --file reports/perf.sql)
echo "== server TTS health (14 days)"
python3 scripts/speak_report.py "${SPEAK_METRICS_DIR:-$HOME/spellgame-server/logs/speak}" --days 14
