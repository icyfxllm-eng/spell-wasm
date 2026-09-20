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

echo "==> cargo build (release, wasm32, --features \"testseam dev_preview\" $EXTRA)"
# shellcheck disable=SC2086
cargo build --release --target wasm32-unknown-unknown --features "testseam dev_preview" $EXTRA

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
# CC-HUMAN-AUDIO D13: verified human clips ship with the app and the site at
# human-audio/<lang>/<sha256>.m4a. Only the .m4a files: verdicts, provenance and
# the runtime manifest are build inputs (runtime.json is compiled into the wasm).
for d in assets/human-audio/*/; do
  lang=$(basename "$d")
  ls "$d"*.m4a >/dev/null 2>&1 || continue
  mkdir -p "$DIST/human-audio/$lang"
  cp "$d"*.m4a "$DIST/human-audio/$lang/"
done
cp -r pkg-test "$DIST/pkg"
# CC-HUMAN-AUDIO e2e: the one clip the testseam fixture manifest points every
# English word at (build.rs). Inert unless a spec arms it.
mkdir -p "$DIST/human-audio/en"
cp tests/fixtures/human-audio/fixture.m4a "$DIST/human-audio/en/fixture.m4a"

echo "==> dist-test/ ready (has __spelltest seam)"
