#!/bin/bash
# CC-SPELLUZZLE: rebuild the verified-seed file for Medium, Hard and Expert.
#
#   scripts/spelluzzle-seeds.sh /path/to/en_US.dic [target-per-tier]
#
# The dictionary is a BUILD INPUT: it is read here, recorded by name and sha256 in
# assets/spelluzzle/seeds-en.json, and never committed or shipped (data/LICENSES.md).
# Re-run whenever the English bank, the collision table or GEN_VERSION changes;
# `cargo test spelluzzle::tests::seeds_are_current` says when it is needed.
set -euo pipefail
DIC="${1:?usage: spelluzzle-seeds.sh /path/to/en_US.dic [target]}"
[ -f "$DIC" ] || { echo "no such file: $DIC" >&2; exit 1; }
cd "$(dirname "$0")/.."
SPZ_VALIDITY_FILE="$DIC" SPZ_VALIDITY_SHA256="$(shasum -a 256 "$DIC" | cut -d' ' -f1)" SPZ_TARGET="${2:-4000}" \
  cargo test --release --lib spelluzzle::seedgen::build_seeds -- --ignored --nocapture
