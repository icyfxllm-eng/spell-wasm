#!/usr/bin/env bash
# Pinned fetch for the Spanish (es) word-list source: rla-es v2.9.
#
# Deterministic by construction: it downloads a COMMIT-pinned release tarball
# and refuses to proceed unless its sha256 matches the value recorded here (and
# mirrored in ../registry.json). A moved branch head or a mutated upstream
# artifact is therefore a HARD ERROR, never a silent substitution.
#
# Usage:  sources/es/fetch.sh <workdir>
#   Downloads + verifies the tarball, extracts the packaged Hunspell dictionary
#   (es_ANY.dic / es_ANY.aff) into <workdir>/, and prints their paths.
#
# D5: this NEVER scrapes an authority site. It fetches one pinned open-source
# release from github.com/sbosio/rla-es and nothing else.
set -euo pipefail

# --- Pins (mirror of sources/registry.json -> sources.es) --------------------
TARBALL_URL="https://codeload.github.com/sbosio/rla-es/tar.gz/refs/tags/v2.9"
TARBALL_SHA256="3930b1e5d9fdf8ddc19247798a77ae2b9efcfe6848555df80bd13f8c9597211e"
DIC_PATH="rla-es-2.9/source-code/hispalabras-0.1/hispalabras/es_ANY.dic"
AFF_PATH="rla-es-2.9/source-code/hispalabras-0.1/hispalabras/es_ANY.aff"
# -----------------------------------------------------------------------------

WORKDIR="${1:?usage: fetch.sh <workdir>}"
mkdir -p "$WORKDIR"
TARBALL="$WORKDIR/rla-es-v2.9.tar.gz"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}';
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}

# Download only if we don't already have a byte-correct copy (idempotent).
if [ ! -f "$TARBALL" ] || [ "$(sha256_of "$TARBALL")" != "$TARBALL_SHA256" ]; then
  echo "fetch(es): downloading $TARBALL_URL" >&2
  curl -sSL --fail -o "$TARBALL" "$TARBALL_URL"
fi

GOT="$(sha256_of "$TARBALL")"
if [ "$GOT" != "$TARBALL_SHA256" ]; then
  echo "fetch(es): CHECKSUM MISMATCH" >&2
  echo "  expected $TARBALL_SHA256" >&2
  echo "  got      $GOT" >&2
  echo "  Refusing to proceed. Re-pin sources/es/fetch.sh + registry.json after review." >&2
  exit 1
fi
echo "fetch(es): checksum OK ($GOT)" >&2

# Extract ONLY the two dictionary files we consume.
tar -xzf "$TARBALL" -C "$WORKDIR" "$DIC_PATH" "$AFF_PATH"
cp "$WORKDIR/$DIC_PATH" "$WORKDIR/es_ANY.dic"
cp "$WORKDIR/$AFF_PATH" "$WORKDIR/es_ANY.aff"

echo "$WORKDIR/es_ANY.dic"
echo "$WORKDIR/es_ANY.aff"
