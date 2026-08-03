#!/bin/bash
# The ONE gate battery. Every step fails the whole gate loudly — this
# script exists because two inline chains lied: a grep swallowed cargo's
# exit code (build 111 era), then an empty $REDS slipped a [ -le ] test
# after pipefail skipped the e2e (bank 67). No inline chains for gates.
set -euo pipefail
cd "$(dirname "$0")/.."
LOG="${TMPDIR:-/tmp}/spell-gate-e2e.log"

echo "== gate: manifest check (schema + D5 + Done #7 audits)"
python3 tools/manifest_check.py

echo "== gate: reports read-only boundary (CC-REPORTS I1)"
if grep -nE "storage::set|note_attempt|\.record\(|save\(" src/reports.rs; then
  echo "GATE FAIL: ReportsQuery wrote to state"; exit 1
fi

echo "== gate: family-voices V2 symbol scan (BD-4 Done #7)"
if grep -inE "URLSession|CFNetwork|train|upload" "ios/App/App/NativeLanguageKitPlugin+FamilyVoices.swift" src/family_voices.rs; then
  echo "GATE FAIL: V2/network symbol in the V1 voice path"; exit 1
fi

echo "== gate: calendar store boundary (CAL I1) + kid-only goals (CAL I7)"
if grep -rn "spell_journal_\|spell_plan_\|spell_goal_\|spell_cheer_" src/ | grep -v "src/journal.rs\|src/calendar.rs"; then
  echo "GATE FAIL: a calendar/journal store key leaked outside its module"; exit 1
fi
if grep -nE "select_goal|week_hand|plan_word|unplan_word" src/guardian_dash.rs; then
  echo "GATE FAIL: a goal-writing symbol reached the parent surface (I7)"; exit 1
fi

echo "== gate: translator closed space (TR I1) + wave-3 zero-code (TR acceptance 12)"
if grep -rniE "deepl|libretranslate|mlkit.?translat|translate\.googleapis|MTModel|machine.?translat" src/ ios/App/App/; then
  echo "GATE FAIL: a machine-translation symbol exists (the closed space is the whole safety story)"; exit 1
fi
# (loanword_explorer, not loanword_ — "loanword_spelling" is a learner SKILL id, prior art)
if grep -rniE "word_globe|language_detective|loanword_explorer|loanword_pack|false_friend|camera_lookup" src/; then
  echo "GATE FAIL: an unsigned Wave-3 translator tool has executable code"; exit 1
fi

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
