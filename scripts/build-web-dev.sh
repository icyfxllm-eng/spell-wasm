#!/usr/bin/env bash
# CC-LEARNING-ENGINE-L0 R4 — a local build WITH the learner inspector.
#
# Mirrors build-web-test.sh minus the test seam: --features dev_preview only.
# Produces dist-dev/, for Eric's device pass (acceptance 13). Never shipped:
# release, auditor and education builds don't carry the feature, and
# scripts/seam-absence-check.mjs greps dist/ for the inspector's marker.
#
#   bash scripts/build-web-dev.sh
#   python3 -m http.server 8141 --directory dist-dev    # then five-tap the logo
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist-dev"

echo "==> cargo build (release, wasm32, --features dev_preview)"
cargo build --release --target wasm32-unknown-unknown --features dev_preview

echo "==> wasm-bindgen -> pkg-dev/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg-dev" --target web --no-typescript

echo "==> assembling dist-dev/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js native-language-kit.js telemetry-schema.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
for d in assets/human-audio/*/; do
  lang=$(basename "$d")
  ls "$d"*.m4a >/dev/null 2>&1 || continue
  mkdir -p "$DIST/human-audio/$lang"
  cp "$d"*.m4a "$DIST/human-audio/$lang/"
done
cp -r pkg-dev "$DIST/pkg"
echo "==> dist-dev/ ready (learner inspector: five taps on the SPELL logo)"
