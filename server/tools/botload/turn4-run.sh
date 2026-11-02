#!/usr/bin/env bash
# CC-ONLINE-TURN Phase D / A9: 50 four-seat rooms for 10 minutes, against a
# throwaway server on its own loopback ports. It never touches the live
# container, its database, or the tunnel. Pass extra botload options through.
#
#   bash server/tools/botload/turn4-run.sh | tee ~/turn4.txt
set -u
ulimit -n 16384
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
bin="$repo/target/release"
work="$(mktemp -d)"
"$bin/botload" tts-stub --listen 127.0.0.1:9191 > "$work/stub.log" 2>&1 &
stub=$!
SPELL_TTS_BASE=http://127.0.0.1:9191 SPELL_ONLINE_TURN=1 SPELL_DB_PATH="$work/s.db" \
  SPELL_BIND_PUBLIC=127.0.0.1:8180 SPELL_BIND_ADMIN=127.0.0.1:8181 SPELL_LOG=warn \
  SPELL_ALLOWED_ORIGINS=capacitor://localhost "$bin/spell-server" > "$work/server.log" 2>&1 &
server=$!
sleep 2
"$bin/botload" run --target http://127.0.0.1:8180 --admin http://127.0.0.1:8181 --profile turn4 "$@" | grep -vE '^(JSON|hb)'
code=${PIPESTATUS[0]}
# -9: a SIGTERM makes the server drain and hold its port for up to 15 minutes.
kill -9 "$server" "$stub" 2>/dev/null
rm -rf "$work"
echo "exit=$code"
exit "$code"
