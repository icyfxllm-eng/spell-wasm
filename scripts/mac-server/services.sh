#!/bin/bash
# Install + start the three LaunchAgents for the Mac server (no sudo needed):
#   net.spellgame.backend — gunicorn on 127.0.0.1:8000 (Flask word-server)
#   net.spellgame.caddy   — Caddy on :8080 (static web + /api proxy, tunnel mode)
#   net.spellgame.tunnel  — cloudflared (spellgame.net -> localhost:8080)
# Also installs net.spellgame.awake (caffeinate) so the mini never naps on duty.
# Requires: setup.sh + deploy.sh done, a real GOOGLE_TTS_API_KEY in
# ~/spellgame-server/.env, and ~/.cloudflared credentials (tunnel login/creation).
set -euo pipefail
SRV="$HOME/spellgame-server"
LA="$HOME/Library/LaunchAgents"
mkdir -p "$LA"

[ -s "$SRV/.env" ] || { echo "services: $SRV/.env missing"; exit 1; }
grep -q "^GOOGLE_TTS_API_KEY=.\+" "$SRV/.env" || { echo "services: GOOGLE_TTS_API_KEY is empty in $SRV/.env"; exit 1; }

TUNNEL_NAME="${SPELL_TUNNEL_NAME:-spellgame-mac}"

plist() { # name, program-args-xml, extra-env-xml, workdir
  cat > "$LA/$1.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$1</string>
  <key>ProgramArguments</key><array>$2</array>
  <key>WorkingDirectory</key><string>$4</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>$SRV/logs/$1.log</string>
  <key>StandardErrorPath</key><string>$SRV/logs/$1.log</string>
  <key>EnvironmentVariables</key><dict>$3</dict>
</dict></plist>
PLIST
}

# Read secrets from .env into plist env (launchd agents don't source shells).
G_KEY=$(grep "^GOOGLE_TTS_API_KEY=" "$SRV/.env" | cut -d= -f2-)
A_KEY=$(grep "^AZURE_SPEECH_KEY=" "$SRV/.env" | cut -d= -f2- || true)
A_REG=$(grep "^AZURE_SPEECH_REGION=" "$SRV/.env" | cut -d= -f2- || true)
# Sign-up codes go out through Resend (backend/auth.py send_email). Without the
# key the backend logs "email not sent" and still reports success, so a missing
# key here means codes silently never arrive.
R_KEY=$(grep "^RESEND_API_KEY=" "$SRV/.env" | cut -d= -f2- || true)

ENVX="<key>GOOGLE_TTS_API_KEY</key><string>$G_KEY</string>
<key>CLIMB_DB_PATH</key><string>$SRV/data/climb.db</string>
<key>AUDIO_CACHE_DIR</key><string>$SRV/cache</string>"
[ -n "$A_KEY" ] && ENVX="$ENVX<key>AZURE_SPEECH_KEY</key><string>$A_KEY</string>"
[ -n "$A_REG" ] && ENVX="$ENVX<key>AZURE_SPEECH_REGION</key><string>$A_REG</string>"
[ -n "$R_KEY" ] && ENVX="$ENVX<key>RESEND_API_KEY</key><string>$R_KEY</string>"
[ -n "$R_KEY" ] || echo "services: WARNING no RESEND_API_KEY in $SRV/.env, sign-up codes will not be emailed"

plist net.spellgame.backend \
  "<string>$SRV/venv/bin/gunicorn</string><string>--bind</string><string>127.0.0.1:8000</string><string>--workers</string><string>2</string><string>app:app</string>" \
  "$ENVX" "$SRV/backend"

plist net.spellgame.caddy \
  "<string>/opt/homebrew/bin/caddy</string><string>run</string><string>--config</string><string>$SRV/Caddyfile</string><string>--adapter</string><string>caddyfile</string>" \
  "" "$SRV"

plist net.spellgame.tunnel \
  "<string>/opt/homebrew/bin/cloudflared</string><string>tunnel</string><string>run</string><string>$TUNNEL_NAME</string>" \
  "" "$SRV"

# Sleep-proofing without sudo: caffeinate holds the system awake while loaded.
plist net.spellgame.awake \
  "<string>/usr/bin/caffeinate</string><string>-simd</string>" \
  "" "$SRV"

for svc in net.spellgame.backend net.spellgame.caddy net.spellgame.tunnel net.spellgame.awake; do
  launchctl bootout "gui/$(id -u)/$svc" 2>/dev/null || true
  # bootout returns before launchd finishes tearing the job down; bootstrapping
  # into that window fails with "5: Input/output error" (2026-09-18: it left the
  # backend unloaded and the API down). Retry for a few seconds instead.
  for try in 1 2 3 4 5 6 7 8 9 10; do
    launchctl bootstrap "gui/$(id -u)" "$LA/$svc.plist" 2>/dev/null && break
    [ "$try" = 10 ] && { echo "services: could not start $svc"; exit 1; }
    sleep 1
  done
  echo "services: started $svc"
done
echo "services: OK — logs in $SRV/logs/"
