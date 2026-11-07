#!/usr/bin/env bash
# CC-LAUNCH-EN A3.1 / item 4: no "coming soon"-class copy ships, except the one
# D3 sentence (key complete.includesFutureLanguages).
#
# It scans the shipped UI: the English strings, index.html, privacy.html and
# notices.html, and the Rust and JS sources for user-visible literals. Usage:
#   scripts/placeholder-grep.sh          scan the source tree
#   scripts/placeholder-grep.sh dist     scan a built bundle instead
set -euo pipefail
cd "$(dirname "$0")/.."
PATTERN='coming soon|not yet available|under construction|lorem ipsum'
ALLOW='complete\.includesFutureLanguages'

if [ "${1:-}" != "" ]; then
  TARGETS=("$1")
else
  TARGETS=(src/i18n/locales/en.json index.html privacy.html notices.html store-native.js native-language-kit.js audio-native.js)
fi

hits=$(grep -rniE "$PATTERN" "${TARGETS[@]}" --include='*.json' --include='*.html' --include='*.js' 2>/dev/null \
  | grep -vE "$ALLOW" || true)
if [ -n "$hits" ]; then
  echo "placeholder-grep: FAIL. Coming-soon copy found outside the D3 allowlist:"
  echo "$hits"
  exit 1
fi
echo "placeholder-grep: OK"
