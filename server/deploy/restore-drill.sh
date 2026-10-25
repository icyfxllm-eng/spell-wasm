#!/usr/bin/env bash
# CC-GAME-SERVER F14 / go-no-go §10.6 — prove the backup restores: fetch the
# latest snapshot into a scratch directory, check it opens cleanly, and
# compare it with the live database. Prints PASS or FAIL. Changes nothing
# live. Run as spellsrv.
set -euo pipefail

env_file="${SPELL_BACKUP_ENV:-${HOME}/.config/spellgame/backup.env}"
set -a
# shellcheck source=/dev/null
. "${env_file}"
set +a

work="$(mktemp -d)"
cleanup() { find "${work}" -mindepth 1 -delete; rmdir "${work}"; }
trap cleanup EXIT

restic restore latest --tag spellgame --target "${work}/restored" > /dev/null
restored="${work}/restored/spell.db"
if [ ! -f "${restored}" ]; then
  echo "FAIL: the latest snapshot holds no spell.db"; exit 1
fi

curl -fsS -m 30 -X POST http://127.0.0.1:8081/admin/snapshot > /dev/null
vol="$(podman volume inspect spell-data --format '{{.Mountpoint}}')"
podman unshare cat "${vol}/backup/snapshot.db" > "${work}/live.db"

check="$(sqlite3 "${restored}" 'PRAGMA integrity_check;')"
r_schema="$(sqlite3 "${restored}" '.schema players')"
l_schema="$(sqlite3 "${work}/live.db" '.schema players')"
r_rows="$(sqlite3 "${restored}" 'SELECT COUNT(*) FROM players;')"
l_rows="$(sqlite3 "${work}/live.db" 'SELECT COUNT(*) FROM players;')"

echo "restored: ${r_rows} players   live: ${l_rows} players   integrity: ${check}"
if [ "${check}" != "ok" ]; then
  echo "FAIL: the restored database is damaged"; exit 1
fi
if [ "${r_schema}" != "${l_schema}" ]; then
  echo "FAIL: the restored schema differs from live"; exit 1
fi
# Live may have grown since the nightly backup; it may not have shrunk below it
# by more than the sweep of 12-month-idle rows could explain.
if [ "${l_rows}" -gt 0 ] && [ "${r_rows}" -eq 0 ]; then
  echo "FAIL: live has players and the backup has none"; exit 1
fi
echo "PASS"
