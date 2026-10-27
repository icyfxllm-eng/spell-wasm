#!/usr/bin/env bash
# CC-GAME-SERVER F14 / G-D16 — deploy one image: drain, point the unit at the
# new digest, restart, wait for health, print status. Never auto-update.
# Run as spellsrv. Eric runs this; CC never does.
#
#   server/deploy/deploy.sh sha256:<hex>
set -euo pipefail

digest="${1:-}"
case "${digest}" in
  sha256:*) ;;
  *) echo "usage: deploy.sh sha256:<hex>   (build.sh prints it)"; exit 2 ;;
esac

unit="${HOME}/.config/containers/systemd/spellgame.container"
image="localhost/spellgame-server@${digest}"
admin="http://127.0.0.1:8081"
public="http://127.0.0.1:8080"

if [ ! -f "${unit}" ]; then
  echo "no unit at ${unit} (HOST-RUNBOOK.md step 12)"; exit 2
fi
if grep -q '<MAC_MINI_TAILNET_IP>' "${unit}"; then
  echo "SPELL_TTS_BASE in ${unit} still holds its placeholder"; exit 2
fi
# G-D1: refuse a unit that would publish beyond loopback.
if grep -E '^PublishPort=' "${unit}" | grep -vqE '^PublishPort=127\.0\.0\.1:'; then
  echo "${unit} publishes a port beyond 127.0.0.1; refusing"; exit 2
fi
if ! grep -qE '^StopTimeout=900$' "${unit}"; then
  echo "${unit} has no StopTimeout=900: podman would kill the drain after 10 s (G-D16)"; exit 2
fi
if grep -qE '^AutoUpdate=' "${unit}"; then
  echo "${unit} enables podman auto-update (G-D16); refusing"; exit 2
fi
if ! podman image exists "${image}"; then
  echo "no local image ${image}; run server/deploy/build.sh first"; exit 2
fi

# 1. Drain: live matches finish, new rooms are refused (F11).
if curl -fsS -m 5 -X POST "${admin}/admin/drain" > /dev/null; then
  echo "draining: live matches may take up to 15 minutes to finish"
else
  echo "no running server to drain (first deploy?)"
fi

# 2. The unit now names the new image.
sed -i -E "s|^Image=.*|Image=${image}|" "${unit}"
systemctl --user daemon-reload

# 3. Restart. systemd sends SIGTERM; the server finishes draining and exits 0
#    (TimeoutStopSec=960 covers the full 15-minute drain).
systemctl --user restart spellgame.service

# 4. Health, up to 120 s.
for _ in $(seq 1 120); do
  if curl -fsS -m 2 "${public}/healthz" > /dev/null; then
    echo "healthy"
    systemctl --user --no-pager --lines=0 status spellgame.service || true
    curl -fsS -m 5 "${admin}/admin/readyz" || true
    echo
    exit 0
  fi
  sleep 1
done
echo "NOT healthy after 120 s"
systemctl --user --no-pager --lines=20 status spellgame.service || true
exit 1
