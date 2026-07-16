#!/usr/bin/env bash
#
# Build the web assets for the EDUCATION edition (CC-EDITIONS) into dist-edu/.
#
# Mirrors scripts/build-web.sh exactly, plus `--features education`. Same shape
# as scripts/build-web-test.sh, which does the same job for the E2E seam — three
# scripts, three dists, one per build variant:
#
#   build-web.sh       -> dist/       consumer      (production / TestFlight)
#   build-web-test.sh  -> dist-test/  + testseam    (Playwright only)
#   build-web-edu.sh   -> dist-edu/   + education   (schools / program buyers)
#
# WHY A SEPARATE DIST, not a flag on dist/. The iOS wrapper packages whatever is
# in its webDir; the Fastfile assumes dist/ is already built and synced and does
# not build the web itself. So a single output directory means an "education"
# archive could silently contain the CONSUMER wasm — the worst failure mode
# available here, because the artifact would look correct while carrying the
# purchase surface into a school. Distinct directories make that mistake
# impossible rather than unlikely.
#
# Capacitor: capacitor.config.json pins webDir to "dist", so syncing an education
# build needs an explicit override:
#
#   bash scripts/build-web-edu.sh
#   npx cap sync ios --web-dir=dist-edu     # NOT the default dist/
#
# NOT WIRED TO FASTLANE, deliberately. An `education` lane needs a distinct
# bundle ID that does not exist yet (Appfile and Matchfile both hardcode
# net.spellgame.app), an App Store Connect record only Eric can create, and a
# `match` run that writes new certs into the private spell-certs repo. Fastlane
# is also under the pipeline freeze — no touches without Eric's approval. This
# script is the half that needs none of that.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist-edu"

echo "==> cargo build (release, wasm32, --features education)"
cargo build --release --target wasm32-unknown-unknown --features education

echo "==> wasm-bindgen -> pkg-edu/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg-edu" --target web --no-typescript

echo "==> assembling dist-edu/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js native-language-kit.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
cp -r pkg-edu "$DIST/pkg"

# D4 as a build step, not a review note: an education artifact that carries a
# purchase surface must never leave this script. Cheap, and it runs on the bytes
# that would actually ship.
echo "==> verifying zero purchase surface (CC-EDITIONS D4)"
node scripts/edu-no-purchase-check.mjs dist-edu

echo "==> dist-edu/ ready (education edition)"
