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

echo "== gate: reachability laws — scroll + modal nesting (AUDITPASS F1/F4)"
node scripts/scroll-check.mjs || { echo "GATE FAIL: scroll law"; exit 1; }

echo "== gate: element IDs are unique (CC-SPELLPIC F0)"
node scripts/dom-id-check.mjs || { echo "GATE FAIL: duplicate element ID"; exit 1; }

echo "== gate: every word in exactly one tier (BD-G4)"
node scripts/tier-partition-check.mjs || { echo "GATE FAIL: a word is in two tiers"; exit 1; }

echo "== gate: one zh grading path (CC-ZH-TONE F2)"
node scripts/zh-grading-path-check.mjs || { echo "GATE FAIL: a second zh grading path"; exit 1; }

echo "== gate: pinned pinyin syllable inventory (CC-ZH-TONE F1a)"
node scripts/pinyin-inventory-check.mjs || { echo "GATE FAIL: pinyin inventory does not match its pin"; exit 1; }

echo "== gate: no tracking on joined scripts (CC-LOCALE-TYPESET F2)"
node scripts/tracking-check.mjs || { echo "GATE FAIL: tracking on a joining script"; exit 1; }

echo "== gate: masterpiece outline hierarchy (CC-MASTERPIECE-RECOG F2)"
python3 tools/masterpiece_lint.py || { echo "GATE FAIL: masterpiece outline hierarchy"; exit 1; }
python3 tools/masterpiece_lint.py --selftest || { echo "GATE FAIL: masterpiece-lint selftest — the gate no longer bites"; exit 1; }

echo "== gate: export/play neutral parity (CC-PICTURE-COLOR)"
node scripts/wordpic-export-parity.mjs || { echo "GATE FAIL: screen and export neutrals disagree"; exit 1; }
node scripts/wordpic-export-parity.mjs --selftest || { echo "GATE FAIL: export-parity selftest — the gate no longer bites"; exit 1; }

echo "== gate: masterpiece density floor + bank-wide ratchet (CC-MASTERPIECE-RECOG)"
python3 tools/density_check.py || { echo "GATE FAIL: a trace is too thin, or thinner than it was"; exit 1; }
python3 tools/density_check.py --selftest || { echo "GATE FAIL: density-check selftest — the gate no longer bites"; exit 1; }

echo "== gate: modes registry has implementations"
# This check was CORRECT and UNREAD. It had been reporting that calendar,
# translate and reports were registry entries with no flag — three live
# modes whose tiles therefore never rendered — and it was not in the gate,
# so nobody saw it. A check nobody runs is a check that does not exist.
node scripts/modes-check.mjs || { echo "GATE FAIL: a mode has no implementation"; exit 1; }

echo "== gate: settings truth (AUDITPASS F8)"
node scripts/settings-truth-check.mjs || { echo "GATE FAIL: settings truth"; exit 1; }
node scripts/settings-truth-check.mjs --selftest || { echo "GATE FAIL: settings-truth selftest — the gate no longer bites"; exit 1; }

echo "== gate: cargo test"
# Keep the OUTPUT, then judge it. The old form re-ran cargo and grepped
# only FAILED|panicked, so a test-profile compile error printed nothing at
# all — ship 141 failed with a bare "GATE FAIL: cargo test" and no clue.
cargo test > "$LOG.cargo" 2>&1 || true
if ! grep -qE "^test result: ok" "$LOG.cargo" || grep -qE "^test result: FAILED" "$LOG.cargo"; then
  echo "GATE FAIL: cargo test"
  grep -E "^error|^test .* FAILED|panicked at" "$LOG.cargo" | head -20
  exit 1
fi

echo "== gate: app build"
# `npm run build | grep bundled` failed CORRECTLY under pipefail, but printed
# NOTHING when it failed — grep dropped every line that was not the bundle
# summary, including the error. Two red gates on 2026-08-08 read as silence:
# a stale word_data.rs, then a web-feature compile break. Same family as the
# swallowed exit code this file's header warns about — that one lied about
# the verdict, this one told the truth and hid the reason.
if ! npm run build > "$LOG.build" 2>&1; then
  echo "GATE FAIL: app build"
  tail -30 "$LOG.build"
  exit 1
fi
grep bundled "$LOG.build" || true

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
