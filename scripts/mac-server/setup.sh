#!/bin/bash
# spellgame.net on THIS Mac (Eric's ruling 2026-07-25: "no pi just use the mac").
# Builds the local server layout under ~/spellgame-server:
#   backend/   Flask app (rsynced from the repo by deploy.sh)
#   web/       static frontend (rsynced from dist/ by deploy.sh)
#   data/      climb.db (accounts/leaderboard) — CLIMB_DB_PATH points here
#   cache/     synthesized TTS clips — AUDIO_CACHE_DIR points here
#   venv/      Python env with backend/requirements.txt
#   .env       secrets (GOOGLE_TTS_API_KEY etc.) — NEVER committed
#   Caddyfile  :8080 tunnel-mode config (repo Caddyfile with local paths)
#   logs/
# Services run as LaunchAgents (no sudo): gunicorn, caddy, cloudflared —
# installed by services.sh once .env has a real key.
set -euo pipefail
SRV="$HOME/spellgame-server"
REPO="$(cd "$(dirname "$0")/../.." && pwd)"

mkdir -p "$SRV"/{data,cache,logs,web}

# Python env (fresh each run is cheap and deterministic).
# Backend deps need Python >=3.10 (google-cloud-texttospeech); Homebrew 3.12.
PY=/opt/homebrew/bin/python3.12
"$PY" -m venv "$SRV/venv"
"$SRV/venv/bin/pip" install --quiet --upgrade pip
"$SRV/venv/bin/pip" install --quiet -r "$REPO/backend/requirements.txt"

# .env scaffold (only if absent — never clobber real secrets).
if [ ! -f "$SRV/.env" ]; then
  cat > "$SRV/.env" <<'ENV'
# spellgame backend secrets — fill in and run services.sh
GOOGLE_TTS_API_KEY=
# Optional (Swahili voices):
AZURE_SPEECH_KEY=
AZURE_SPEECH_REGION=
ENV
fi

# Caddyfile: repo config with container paths swapped for local ones.
sed -e "s|reverse_proxy backend:8000|reverse_proxy 127.0.0.1:8000|g" \
    -e "s|root \* /srv|root * $SRV/web|g" \
    "$REPO/Caddyfile" > "$SRV/Caddyfile"

echo "setup: OK — $SRV ready. Next: deploy.sh, then fill $SRV/.env, then services.sh"
