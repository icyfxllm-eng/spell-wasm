#!/bin/bash
# CC-TELEMETRY-FOUNDATION D6 retention, run daily on the Mac mini by the
# net.spellgame.telemetry-purge LaunchAgent (install-telemetry-purge.sh).
#
#   1. Raw telemetry events older than 90 days are deleted from D1
#      (the per-day counts are kept). This replaces the Worker's cron, which
#      Cloudflare refused on this account (code 10063, 2026-09-18).
#   2. R8 /api/speak metric files older than 90 days are pruned.
#
# Uses the Wrangler OAuth login in ~/Library/Preferences/.wrangler. If that
# login lapses, step 1 fails loudly in the log and step 2 still runs.
set -uo pipefail
export PATH=/opt/homebrew/bin:/usr/bin:/bin
export WRANGLER_SEND_METRICS=false
export CLOUDFLARE_ACCOUNT_ID=0e7f9ab5e7c6eb54035967ba6a9cd242
SRV="$HOME/spellgame-server"
ts() { date -u +%FT%TZ; }
rc=0

out=$(npx --yes wrangler@4 d1 execute spell-telemetry --remote --json \
  --command "DELETE FROM events WHERE day < date('now','-90 day')" 2>&1)
if n=$(printf '%s' "$out" | python3 -c 'import sys,json; print(json.load(sys.stdin)[0]["meta"]["changes"])' 2>/dev/null); then
  echo "$(ts) telemetry-purge: deleted $n raw events older than 90 days"
else
  echo "$(ts) telemetry-purge: D1 purge FAILED"
  printf '%s\n' "$out" | tail -5
  rc=1
fi

if [ -d "$SRV/logs/speak" ]; then
  python3 "$SRV/bin/speak_report.py" "$SRV/logs/speak" --days 0 --prune > /dev/null 2>"$SRV/logs/.speak-prune.tmp" \
    && echo "$(ts) telemetry-purge: speak metrics pruned ($(grep -c pruned "$SRV/logs/.speak-prune.tmp" || true) files)" \
    || { echo "$(ts) telemetry-purge: speak prune FAILED"; rc=1; }
  rm -f "$SRV/logs/.speak-prune.tmp"
fi
exit $rc
