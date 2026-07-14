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
#
# AUDIT BUILD (review-gated Filipino audit, NOT for production/TestFlight):
#   AUDIT_LANGS=fil bash scripts/build-web.sh
# When AUDIT_LANGS is set, the crate's build.rs turns on the `audit_build` cfg
# (marking those langs Active + enabling the Feature-7 preselect/pin/banner) and
# output goes to a SEPARATE dist-audit/ — never the prod webroot. When AUDIT_LANGS
# is UNSET this behaves exactly as before and the dist/ bytes are unchanged.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

AUDIT_LANGS="${AUDIT_LANGS:-}"
FEATURE_ARGS=""
if [[ -n "$AUDIT_LANGS" ]]; then
  export AUDIT_LANGS
  FEATURE_ARGS="--features audit"
  DIST="$ROOT/dist-audit"
  PKG="$ROOT/pkg-audit"
  echo "==> AUDIT BUILD (AUDIT_LANGS=$AUDIT_LANGS) -> dist-audit/ (NOT production)"
else
  DIST="$ROOT/dist"
  PKG="$ROOT/pkg"
fi

echo "==> cargo build (release, wasm32-unknown-unknown)"
# shellcheck disable=SC2086 -- FEATURE_ARGS is intentionally word-split (empty in prod).
cargo build --release --target wasm32-unknown-unknown $FEATURE_ARGS

echo "==> wasm-bindgen -> ${PKG##*/}/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$PKG" --target web --no-typescript

echo "==> assembling ${DIST##*/}/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r "$PKG" "$DIST/pkg"

echo "==> ${DIST##*/}/ ready:"
ls -1 "$DIST"
