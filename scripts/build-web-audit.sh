#!/usr/bin/env bash
#
# Build the AUDIT PREVIEW web bundle into dist-audit/ — a private build where the
# ComingSoon languages (ar/fa/ur/ru + the already-banked es/fr/de/…) are unlocked
# and playable, so native speakers can review rendering, keyboards, feedback, and
# the DRAFT word banks (assets/words-draft/, unverified CC BY-SA).
#
# NEVER production. The `audit_preview` feature is what unlocks everything; with it
# off (scripts/build-web.sh, the TestFlight build) RTL_SUPPORTED is false, the
# languages stay gated, and the draft banks are not compiled in. This script also
# stamps a persistent "UNVERIFIED PREVIEW" banner into the page so no one mistakes
# an audit build for the real app.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist-audit"

echo "==> regenerate draft banks -> src/word_data_audit.rs"
python3 scripts/build-draft-wordbanks.py

echo "==> cargo build (release, wasm32, --features audit_preview)"
cargo build --release --target wasm32-unknown-unknown --features audit_preview

echo "==> wasm-bindgen -> pkg-audit/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg-audit" --target web --no-typescript

echo "==> assembling dist-audit/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js native-language-kit.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
cp -r pkg-audit "$DIST/pkg"

# Stamp the banner: a fixed, unmissable strip + a <title> marker. Injected here (not
# in index.html) so it exists ONLY in the audit bundle, never in the shipped page.
echo "==> stamping UNVERIFIED PREVIEW banner"
python3 - "$DIST/index.html" <<'PY'
import sys
p = sys.argv[1]
html = open(p, encoding="utf-8").read()
banner = (
  '<style>#auditPreviewBanner{position:fixed;top:0;left:0;right:0;z-index:99999;'
  'background:#b03a5b;color:#fff;font:600 12px/1.4 -apple-system,system-ui,sans-serif;'
  'text-align:center;padding:5px 10px;letter-spacing:.02em}'
  'body{padding-top:26px!important}</style>'
  '<div id="auditPreviewBanner">AUDIT PREVIEW · UNVERIFIED DRAFT CONTENT · NOT FOR RELEASE</div>'
)
html = html.replace("<body>", "<body>" + banner, 1) if "<body>" in html else banner + html
html = html.replace("<title>", "<title>[AUDIT] ", 1)
open(p, "w", encoding="utf-8").write(html)
print("   banner + title marker stamped")
PY

echo "==> dist-audit/ ready — UNVERIFIED PREVIEW, do not distribute as release"
