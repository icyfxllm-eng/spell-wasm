#!/bin/bash
# CC-PHOTO-IMPORT Phase 7 — run the OCR precision/recall eval + merge gate.
#
# Scores the on-device recognizer over the 78-image fixture set and FAILS if
# any language drops >0.02 below Resources/ocr-fixtures/ocr-baseline.json.
# Regenerate fixtures after changing the word banks or adding styles:
#   node scripts/ocr-fixtures/extract-spec.mjs
#   swift scripts/ocr-fixtures/gen-ocr-fixtures.swift
# To re-record the baseline after a deliberate improvement, paste the JSON the
# test prints into ocr-baseline.json.
set -euo pipefail
cd "$(dirname "$0")/../ios/NativeLanguageKit"
DEST="${OCR_EVAL_DEST:-platform=iOS Simulator,name=iPhone 17 Pro}"
LOG="$(mktemp)"
xcodebuild test \
  -scheme NativeLanguageKitCore \
  -destination "$DEST" \
  -only-testing:NativeLanguageKitCoreTests/OcrEvalTests \
  > "$LOG" 2>&1 || true
grep -E "OCR-EVAL|regressed" "$LOG" || true
if grep -q "Test Suite 'OcrEvalTests' passed" "$LOG"; then
  echo "ocr-eval: PASS (no language below baseline - 0.02)"
else
  echo "ocr-eval: FAIL — see log above"
  exit 1
fi
