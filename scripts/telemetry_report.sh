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
#
# The queries go in through --command, not --file: wrangler sends a --file
# through D1's IMPORT api, which refused this account's OAuth login with
# "Authentication error [code: 10000]" (2026-09-22) while every other D1 call
# worked. Comment lines are stripped first, or wrangler reads a leading "--"
# as one of its own flags.
set -euo pipefail
cd "$(dirname "$0")/.."
W=workers/telemetry
export WRANGLER_SEND_METRICS=false

# Run a .sql file and print its rows as a table.
query() {
  local sql
  sql=$(grep -v '^[[:space:]]*--' "$W/reports/$1.sql" | tr '\n' ' ')
  (cd "$W" && npx wrangler d1 execute spell-telemetry --remote --json --command "$sql") | python3 -c '
import json, sys
try:
    rows = json.load(sys.stdin)[0]["results"]
except Exception as e:
    sys.exit(f"  (could not read the answer: {e})")
if not rows:
    print("  (no rows)")
    sys.exit()
cols = list(rows[0])
w = [max(len(c), *(len(str(r[c])) for r in rows)) for c in cols]
print("  " + "  ".join(c.ljust(n) for c, n in zip(cols, w)))
print("  " + "  ".join("-" * n for n in w))
for r in rows:
    print("  " + "  ".join(str(r[c]).ljust(n) for c, n in zip(cols, w)))
print(f"  ({len(rows)} rows)")
'
}

echo "== crashes (14 days)"
query crashes
echo
echo "== client performance (14 days)"
query perf
echo
echo "== server TTS health (14 days)"
python3 scripts/speak_report.py "${SPEAK_METRICS_DIR:-$HOME/spellgame-server/logs/speak}" --days 14
