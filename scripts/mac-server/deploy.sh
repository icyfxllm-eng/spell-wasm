#!/bin/bash
# Deploy the CURRENT repo state to the local Mac server (~/spellgame-server):
# backend code + built web frontend, then restart whatever services are loaded.
# The one-command replacement for the old "ssh to the box and rebuild" flow.
set -euo pipefail
SRV="$HOME/spellgame-server"
REPO="$(cd "$(dirname "$0")/../.." && pwd)"

[ -d "$SRV/venv" ] || { echo "deploy: run setup.sh first"; exit 1; }

rsync -a --delete "$REPO/backend/" "$SRV/backend/"
# Frontend: serve the same dist/ the iOS bundle uses (build with `npm run build`).
if [ -d "$REPO/dist" ]; then
  # web/ also holds things dist/ never has, which --delete must not touch:
  # packs/ is the server-hosted offline language packs (built on the server,
  # about 1 GB; nothing restores it), and privacy.html + .well-known/ are
  # copied in below. Excluded, they are never deleted, not even briefly.
  rsync -a --delete --exclude /packs/ --exclude /.well-known/ --exclude /privacy.html "$REPO/dist/" "$SRV/web/"
  # privacy.html + well-known live at the repo root, outside dist.
  [ -f "$REPO/privacy.html" ] && cp "$REPO/privacy.html" "$SRV/web/privacy.html"
  [ -d "$REPO/.well-known" ] && rsync -a "$REPO/.well-known/" "$SRV/web/.well-known/"
fi
"$SRV/venv/bin/pip" install --quiet -r "$SRV/backend/requirements.txt"

# Restart loaded services (no-op for any not yet installed).
for svc in net.spellgame.backend net.spellgame.caddy net.spellgame.tunnel; do
  launchctl kickstart -k "gui/$(id -u)/$svc" 2>/dev/null && echo "deploy: restarted $svc" || true
done
echo "deploy: OK"
