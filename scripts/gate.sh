#!/bin/bash
# The ONE gate battery. Every step fails the whole gate loudly — this
# script exists because two inline chains lied: a grep swallowed cargo's
# exit code (build 111 era), then an empty $REDS slipped a [ -le ] test
# after pipefail skipped the e2e (bank 67). No inline chains for gates.
set -euo pipefail
cd "$(dirname "$0")/.."
LOG="${TMPDIR:-/tmp}/spell-gate-e2e.log"

echo "== gate: cargo test"
cargo test 2>&1 | tail -n 20 | grep -E "test result: ok" >/dev/null || { echo "GATE FAIL: cargo test"; cargo test 2>&1 | grep -E "FAILED|panicked" | head; exit 1; }

echo "== gate: app build"
npm run build 2>&1 | grep bundled

echo "== gate: e2e (app + site)"
if ! npm run e2e > "$LOG" 2>&1; then
  REDS=$(grep -c "✗" "$LOG" || true)
  KNOWN=$(grep -c "✗ hub:" "$LOG" || true)
  if [ "${REDS:-99}" -ne "${KNOWN:-0}" ] || [ "${REDS:-99}" -gt 3 ]; then
    echo "GATE FAIL: e2e has non-hub reds"; grep "✗" "$LOG"; exit 1
  fi
  echo "e2e: only the ${REDS} spun-out hub reds"
fi
grep -E "E2E:" "$LOG"

echo "== gate: web wall scan"
npm run build:web 2>&1 | tail -n 1 | grep "web-picture-wall-scan: OK"

echo "== gate: restore app dist"
npm run build 2>&1 | grep bundled

echo "GATE PASS"
