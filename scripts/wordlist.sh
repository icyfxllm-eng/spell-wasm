#!/usr/bin/env bash
# One-command word-list build for a single language (CC-WORDLIST-SOURCES).
#
#   scripts/wordlist.sh es            # fetch -> verify -> unmunch -> filter
#   make wordlist LANG=es             # same thing
#
# Pipeline: fetch (pinned + checksum-verified) -> unmunch the Hunspell dic/aff
# -> NFC-normalize + dedupe + game-eligibility filter -> emit wordlists/<lang>.txt
# + wordlists/<lang>.manifest.json.
#
# NO silent fallback: a missing registry entry, a failed fetch, a checksum
# mismatch, or a missing unmunch tool is a HARD ERROR. It never falls back to a
# stale list.
#
# Determinism: two consecutive runs produce byte-identical output + manifest.
set -euo pipefail

LANG_CODE="${1:?usage: wordlist.sh <lang>   (e.g. wordlist.sh es)}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REG="$ROOT/sources/registry.json"
SRC_DIR="$ROOT/sources/$LANG_CODE"

# --- Registry completeness gate (mirrors scripts/license-gate.mjs) -----------
[ -f "$REG" ] || { echo "wordlist: missing sources/registry.json" >&2; exit 1; }
if ! python3 -c "import json,sys;d=json.load(open('$REG'));sys.exit(0 if '$LANG_CODE' in d['sources'] else 1)"; then
  echo "wordlist: no registry entry for '$LANG_CODE' in sources/registry.json." >&2
  echo "  A language without a COMPLETE registry entry cannot produce a list." >&2
  exit 1
fi
for f in fetch.sh LICENSE PROVENANCE.md; do
  [ -f "$SRC_DIR/$f" ] || { echo "wordlist: missing sources/$LANG_CODE/$f (incomplete registry)." >&2; exit 1; }
done

# --- Tooling gate ------------------------------------------------------------
if ! command -v unmunch >/dev/null 2>&1; then
  echo "wordlist: 'unmunch' not found. Install hunspell (brew install hunspell /" >&2
  echo "  apt-get install hunspell-tools) or run this step in CI. NOT faking output." >&2
  exit 1
fi
HUNSPELL_VER="$(hunspell -vv 2>/dev/null | head -1 | tr -d '\r' || echo unknown)"
[ -n "$HUNSPELL_VER" ] || HUNSPELL_VER="unknown"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- 1) fetch + verify checksum ---------------------------------------------
echo "wordlist($LANG_CODE): fetch + verify" >&2
bash "$SRC_DIR/fetch.sh" "$WORK" | tail -2 > "$WORK/fetched_paths"
DIC="$(sed -n '1p' "$WORK/fetched_paths")"
AFF="$(sed -n '2p' "$WORK/fetched_paths")"
[ -f "$DIC" ] && [ -f "$AFF" ] || { echo "wordlist: fetch did not yield dic/aff" >&2; exit 1; }

# --- 2) unmunch --------------------------------------------------------------
echo "wordlist($LANG_CODE): unmunch" >&2
RAW="$WORK/raw.txt"
unmunch "$DIC" "$AFF" > "$RAW"

# --- 3) NFC + lowercase + dedupe -> raw provenance surface index -------------
# Emitted from the SAME unmunch expansion, BEFORE the game filter, so the
# provenance index and the playable list are provably the same source run.
echo "wordlist($LANG_CODE): surface index" >&2
python3 "$ROOT/scripts/surface_index.py" "$LANG_CODE" "$RAW"

# --- 4) NFC + dedupe + game-eligibility filter -> outputs + manifest ---------
echo "wordlist($LANG_CODE): filter" >&2
python3 "$ROOT/scripts/wordlist_filter.py" "$LANG_CODE" "$RAW" \
  --hunspell "$HUNSPELL_VER" --unmunch-tool "unmunch (hunspell)"

echo "wordlist($LANG_CODE): done -> wordlists/$LANG_CODE.txt (+ manifest, surface-index)" >&2
