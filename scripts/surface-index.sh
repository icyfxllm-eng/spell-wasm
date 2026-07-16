#!/usr/bin/env bash
# Build the RAW provenance surface index for a language (CC-WORDLIST-SOURCES).
#
#   scripts/surface-index.sh es       # fetch -> verify -> unmunch -> emit index
#   make surface-index LANG=es        # same thing
#
# Pipeline: fetch (pinned + checksum-verified, sources/<lang>/fetch.sh) -> unmunch
# the Hunspell dic/aff -> NFC + lowercase + dedupe the FULL surface expansion ->
# emit sources/<lang>/surface-index.txt.
#
# This is the raw EXISTENCE index used for provenance validation. It is emitted
# BEFORE the game-eligibility filter (unlike wordlists/<lang>.txt), so a long or
# otherwise game-ineligible-but-real headword still counts as source-backed.
#
# NO silent fallback: a missing registry entry, a failed fetch, a checksum
# mismatch, or a missing `unmunch` tool is a HARD ERROR — never a stale index.
# Determinism: two consecutive runs produce a byte-identical surface-index.txt.
set -euo pipefail

LANG_CODE="${1:?usage: surface-index.sh <lang>   (e.g. surface-index.sh es)}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REG="$ROOT/sources/registry.json"
SRC_DIR="$ROOT/sources/$LANG_CODE"

# --- Registry completeness gate (mirrors scripts/license-gate.mjs) -----------
[ -f "$REG" ] || { echo "surface-index: missing sources/registry.json" >&2; exit 1; }
if ! python3 -c "import json,sys;d=json.load(open('$REG'));sys.exit(0 if '$LANG_CODE' in d['sources'] else 1)"; then
  echo "surface-index: no registry entry for '$LANG_CODE' in sources/registry.json." >&2
  exit 1
fi
for f in fetch.sh LICENSE PROVENANCE.md; do
  [ -f "$SRC_DIR/$f" ] || { echo "surface-index: missing sources/$LANG_CODE/$f (incomplete registry)." >&2; exit 1; }
done

# --- Tooling gate ------------------------------------------------------------
if ! command -v unmunch >/dev/null 2>&1; then
  echo "surface-index: 'unmunch' not found. Install hunspell (brew install hunspell /" >&2
  echo "  apt-get install hunspell-tools) or run this step in CI. NOT faking output." >&2
  exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- 1) fetch + verify checksum ---------------------------------------------
echo "surface-index($LANG_CODE): fetch + verify" >&2
bash "$SRC_DIR/fetch.sh" "$WORK" | tail -2 > "$WORK/fetched_paths"
DIC="$(sed -n '1p' "$WORK/fetched_paths")"
AFF="$(sed -n '2p' "$WORK/fetched_paths")"
[ -f "$DIC" ] && [ -f "$AFF" ] || { echo "surface-index: fetch did not yield dic/aff" >&2; exit 1; }

# --- 2) unmunch --------------------------------------------------------------
echo "surface-index($LANG_CODE): unmunch" >&2
RAW="$WORK/raw.txt"
unmunch "$DIC" "$AFF" > "$RAW"

# --- 3) NFC + lowercase + dedupe -> sources/<lang>/surface-index.txt ---------
echo "surface-index($LANG_CODE): emit index" >&2
python3 "$ROOT/scripts/surface_index.py" "$LANG_CODE" "$RAW"

echo "surface-index($LANG_CODE): done -> sources/$LANG_CODE/surface-index.txt" >&2
