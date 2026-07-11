#!/usr/bin/env bash
#
# Produce the bundled web assets Capacitor ships inside the native app.
#
# This mirrors the two build stages in the repo's Dockerfile (the source of
# truth for the deployed site):
#   1. Compile the Rust crate to wasm32-unknown-unknown.
#   2. Run wasm-bindgen (--target web) to emit pkg/spell_wasm.js + _bg.wasm.
# It then assembles the static shell into dist/ so `--web-dir=dist` picks it
# up. Keep the file list here in sync with the Dockerfile's COPY lines.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist"

echo "==> cargo build (release, wasm32-unknown-unknown)"
cargo build --release --target wasm32-unknown-unknown

echo "==> wasm-bindgen -> pkg/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg" --target web --no-typescript

echo "==> assembling dist/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r pkg "$DIST/pkg"

echo "==> dist/ ready:"
ls -1 "$DIST"
