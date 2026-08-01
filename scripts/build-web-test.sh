#!/usr/bin/env bash
#
# Build the web assets WITH the observation-only E2E test seam
# (window.__spelltest) into dist-test/, for the Playwright suite. NEVER used for
# production/TestFlight — that's scripts/build-web.sh, which omits the feature.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
# SPELL_WEB=1 builds the SITE configuration (English only) into its own dir,
# so the platform-specific specs run against the platform they describe
# instead of asserting web behaviour against an app build.
if [ "${SPELL_WEB:-0}" = "1" ]; then
  DIST="$ROOT/dist-test-web"
  EXTRA="--features web"
else
  DIST="$ROOT/dist-test"
  EXTRA=""
fi

echo "==> cargo build (release, wasm32, --features testseam $EXTRA)"
# shellcheck disable=SC2086
cargo build --release --target wasm32-unknown-unknown --features testseam $EXTRA

echo "==> wasm-bindgen -> pkg-test/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg-test" --target web --no-typescript

echo "==> assembling dist-test/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js native-language-kit.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
cp -r pkg-test "$DIST/pkg"

echo "==> dist-test/ ready (has __spelltest seam)"
