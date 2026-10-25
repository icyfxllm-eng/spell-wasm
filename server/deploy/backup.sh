#!/usr/bin/env bash
# CC-GAME-SERVER F14 — nightly backup of the players database (the only
# persistent state, §9). Run by spell-backup.timer as spellsrv.
#
# Reads its secrets from an env file that lives on the host only, never in
# the repository (O5: Backblaze B2 or Cloudflare R2, Eric's choice):
#   ~/.config/spellgame/backup.env
#     RESTIC_REPOSITORY=<RESTIC_REPOSITORY_URL>
#     RESTIC_PASSWORD_FILE=<PATH_TO_PASSWORD_FILE>
#     plus the provider's keys (B2_ACCOUNT_ID/B2_ACCOUNT_KEY, or AWS_* for R2)
set -euo pipefail

env_file="${SPELL_BACKUP_ENV:-${HOME}/.config/spellgame/backup.env}"
if [ ! -f "${env_file}" ]; then
  echo "no ${env_file} (HOST-RUNBOOK.md step 13)"; exit 2
fi
set -a
# shellcheck source=/dev/null
. "${env_file}"
set +a
: "${RESTIC_REPOSITORY:?set in ${env_file}}"
: "${RESTIC_PASSWORD_FILE:?set in ${env_file}}"

state="${XDG_STATE_HOME:-${HOME}/.local/state}/spellgame"
mkdir -p "${state}"

# 1. A consistent copy, made by the server itself (VACUUM INTO).
curl -fsS -m 30 -X POST http://127.0.0.1:8081/admin/snapshot > /dev/null

# 2. Read it out of the rootless volume and stream it to restic.
vol="$(podman volume inspect spell-data --format '{{.Mountpoint}}')"
podman unshare cat "${vol}/backup/snapshot.db" \
  | restic backup --stdin --stdin-filename spell.db --tag spellgame --host "$(hostname)"

# 3. Keep two weeks of dailies, two months of weeklies, half a year of monthlies.
restic forget --tag spellgame --keep-daily 14 --keep-weekly 8 --keep-monthly 6 --prune

# 4. The stamp verify-host.sh H12 reads.
date -u +%s > "${state}/last-backup"
echo "backup ok"
