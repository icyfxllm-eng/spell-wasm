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
# Self-hosted web fonts (FIX 1): index.html references ./fonts/*.woff2 locally
# instead of Google Fonts / jsdelivr, and sw.js precaches them. Must be copied
# into dist/ or the app falls back to system fonts offline.
cp -r fonts "$DIST/fonts"
cp -r pkg "$DIST/pkg"

# Precompress the ~2.3MB wasm (FIX 2) so Caddy's `precompressed br gzip` can
# serve it. Best-effort locally: skip a codec that isn't installed (the
# production build always has both — see Dockerfile). Kept alongside the raw
# .wasm; instantiateStreaming still works because Content-Encoding is separate
# from Content-Type (application/wasm).
WASM="$DIST/pkg/spell_wasm_bg.wasm"
if [ -f "$WASM" ]; then
  command -v brotli >/dev/null 2>&1 && brotli -q 11 -f "$WASM" && echo "==> brotli: $WASM.br" || echo "==> (brotli not found — skipping .br)"
  command -v gzip   >/dev/null 2>&1 && gzip -9 -c "$WASM" > "$WASM.gz" && echo "==> gzip:   $WASM.gz" || echo "==> (gzip not found — skipping .gz)"
fi

echo "==> dist/ ready:"
ls -1 "$DIST"
