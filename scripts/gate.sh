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

echo "== gate: family-voices network scan (BD-D4: local Mac only)"
# BD-D4 DECIDED 2026-08-05 (Eric: "Local Mac companion"): V2 custom
# voices are UNBLOCKED, so `train` is no longer a forbidden symbol —
# local training is the sanctioned path. What tightened instead is the
# network ban: household recordings never leave the home, so NO network
# symbol may appear in the voice path at all. Full-line comments are
# stripped first (the files must be free to DISCUSS the ban without
# tripping it); an inline URL in real code still matches.
for f in "ios/App/App/NativeLanguageKitPlugin+FamilyVoices.swift" src/family_voices.rs; do
  # Judge SHIPPED code: strip full-line comments, and stop at the test
  # module — the test that ASSERTS this ban necessarily names the very
  # symbols it forbids (`no_upload_path_exists`), and that is the ban
  # working, not breaking.
  if sed -E '/^#\[cfg\(test\)\]/,$d; /^[[:space:]]*\/\//d' "$f" \
       | grep -inE "URLSession|CFNetwork|upload|fetch_post|fetch_json|https?:"; then
    echo "GATE FAIL: network symbol in the voice path ($f) — BD-D4 says recordings stay home"; exit 1
  fi
done

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
# WAVE 3 SIGNED 2026-08-05 (D3/D4/D6/D7/D8): the tools may exist, so the
# zero-code scan is retired. What the signed specs STILL require is that
# no UNAUDITED content ships: D3's per-pair tables and D8's per-pack
# lists stay empty until each one's audit lands, and the closed space
# holds. Those are asserted in-engine (translate.rs
# `authored_tables_are_dark_until_audited`); here we keep the guard that
# never relaxes — no machine translation, ever (checked just above) —
# and verify the audited-content accessors are still the ONLY source.
if grep -rniE "fn (pair_table|loanword_packs)\b" src/ | grep -v "^src/translate.rs"; then
  echo "GATE FAIL: audited-table accessors exist outside the one resolver"; exit 1
fi

echo "== gate: composite pin law (CC-BANK-COMPLETE F2/D2)"
python3 tools/bank/verify_pins.py || { echo "GATE FAIL: composite pin law violated"; exit 1; }

echo "== gate: cargo test"
cargo test 2>&1 | tail -n 20 | grep -E "test result: ok" >/dev/null || { echo "GATE FAIL: cargo test"; cargo test 2>&1 | grep -E "FAILED|panicked" | head; exit 1; }

echo "== gate: app build"
npm run build 2>&1 | grep bundled

echo "== gate: e2e (app + site)"
if ! npm run e2e > "$LOG" 2>&1; then
  # SHIP 135: the three tolerated hub reds are GONE — they were stale
  # tests asserting a pre-D5 world, not app bugs. With the board clean
  # the tolerance clause retires too: it let any NEW hub failure hide
  # behind "just the known three", which is the whole cost of a
  # tolerated red. Zero means zero now.
  REDS=$(grep -c "✗" "$LOG" || true)
  if [ "${REDS:-99}" -ne 0 ]; then
    echo "GATE FAIL: e2e has reds"; grep "✗" "$LOG"; exit 1
  fi
  echo "e2e: clean board (zero reds)"
fi
grep -E "E2E:" "$LOG"

echo "== gate: web wall scan"
npm run build:web 2>&1 | tail -n 1 | grep "web-picture-wall-scan: OK"

echo "== gate: restore app dist"
npm run build 2>&1 | grep bundled

echo "GATE PASS"
