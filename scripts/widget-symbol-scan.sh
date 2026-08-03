#!/bin/bash
# BD-1 acceptance #2 — the widget extension binary links no networking
# symbols. Same doctrine as the web wall scan: absence proven, not assumed.
set -euo pipefail
BIN="$1" # path to the built SpellWidgets binary
if nm -u "$BIN" 2>/dev/null | grep -iE "URLSession|CFNetwork|NSURLConnection|CFSocket" ; then
  echo "SYMBOL SCAN FAIL: networking symbols in the widget extension"
  exit 1
fi
echo "widget-symbol-scan: OK — no networking symbols in $(basename "$BIN")"
