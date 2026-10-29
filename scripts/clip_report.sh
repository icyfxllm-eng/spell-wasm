#!/usr/bin/env bash
# CLIP-REPORT -- words by "Can't tell" report count (last 90 days), for Eric to
# quarantine. Same wrangler/--command trick as telemetry_report.sh (a --file
# goes through D1's import API, which this OAuth login is refused for).
#
#   bash scripts/clip_report.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export WRANGLER_SEND_METRICS=false
sql=$(grep -v '^[[:space:]]*--' workers/telemetry/reports/clips.sql | tr '\n' ' ')
echo "== clips by report count (90 days)"
(cd workers/telemetry && npx wrangler d1 execute spell-telemetry --remote --json --command "$sql") \
  | python3 scripts/clip_report.py
