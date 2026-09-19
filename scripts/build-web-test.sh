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
# Same sentinel strip as build-web.sh: the wall spec tests the RENDERED site,
# so the test artifact must be cut the same way the deployed one is.
if [ "${SPELL_WEB:-0}" = "1" ]; then
  python3 - "$DIST" <<'STRIP'
import pathlib, sys
src = pathlib.Path("index.html").read_text()
for pair in (("<!-- SPELL-PICTURE:BEGIN", "<!-- SPELL-PICTURE:END -->"),
             ("/* SPELL-PICTURE:BEGIN", "/* SPELL-PICTURE:END */")):
    while pair[0] in src:
        a = src.index(pair[0]); b = src.index(pair[1], a) + len(pair[1])
        src = src[:a] + src[b:]
pathlib.Path(sys.argv[1], "index.html").write_text(src)
STRIP
else
  cp index.html "$DIST/"
fi
cp audio-native.js native-language-kit.js telemetry-schema.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
cp -r pkg-test "$DIST/pkg"

echo "==> dist-test/ ready (has __spelltest seam)"
