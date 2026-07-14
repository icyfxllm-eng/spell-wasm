#!/usr/bin/env bash
#
# Build the web assets WITH the observation-only E2E test seam
# (window.__spelltest) into dist-test/, for the Playwright suite. NEVER used for
# production/TestFlight — that's scripts/build-web.sh, which omits the feature.
#
# AUDIT VARIANT (for the audit E2E spec): set AUDIT_LANGS to also turn on the
# `audit_build` cfg and emit to dist-test-audit/ instead:
#   AUDIT_LANGS=fil bash scripts/build-web-test.sh
# This is the seam-enabled twin of the audit build — it lets the E2E suite drive
# the Feature-7 preselect/pin/banner and a full Filipino round. Unset, it builds
# the plain (production-parity) seam bundle into dist-test/.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

AUDIT_LANGS="${AUDIT_LANGS:-}"
FEATURES="testseam"
if [[ -n "$AUDIT_LANGS" ]]; then
  export AUDIT_LANGS
  FEATURES="testseam,audit"
  DIST="$ROOT/dist-test-audit"
  PKG="$ROOT/pkg-test-audit"
  echo "==> AUDIT TEST BUILD (AUDIT_LANGS=$AUDIT_LANGS, --features $FEATURES) -> dist-test-audit/"
else
  DIST="$ROOT/dist-test"
  PKG="$ROOT/pkg-test"
fi

echo "==> cargo build (release, wasm32, --features $FEATURES)"
cargo build --release --target wasm32-unknown-unknown --features "$FEATURES"

echo "==> wasm-bindgen -> ${PKG##*/}/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$PKG" --target web --no-typescript

echo "==> assembling ${DIST##*/}/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r "$PKG" "$DIST/pkg"

echo "==> ${DIST##*/}/ ready (has __spelltest seam)"
