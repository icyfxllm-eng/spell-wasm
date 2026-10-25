#!/usr/bin/env bash
# CC-GAME-SERVER F14 — build the room server image on the host and print the
# digest deploy.sh takes. Run as spellsrv from the repository checkout.
#
#   server/deploy/build.sh            -> prints the tag, then sha256:<hex>
set -euo pipefail

cd "$(dirname "$0")/../.."
rev="$(git rev-parse --short HEAD)"
tag="localhost/spellgame-server:${rev}"

podman build --pull=newer -f server/deploy/Containerfile -t "${tag}" .

digest="$(podman image inspect --format '{{.Digest}}' "${tag}")"
echo "${tag}"
echo "${digest}"
echo "next: server/deploy/deploy.sh ${digest}"
