#!/usr/bin/env bash
#
# Build the web assets WITH the observation-only E2E test seam
# (window.__spelltest) into dist-test/, for the Playwright suite. NEVER used for
# production/TestFlight — that's scripts/build-web.sh, which omits the feature.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist-test"

echo "==> cargo build (release, wasm32, --features testseam)"
cargo build --release --target wasm32-unknown-unknown --features testseam

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
