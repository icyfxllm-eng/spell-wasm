#!/bin/bash
# Is the onset mishearing sensitive to the lead silence?
#
# Four /b/-initial words come back from BOTH recognizers as /dh/-initial --
# ban -> "then", bay -> "They", bear -> "There", ben -> "Then". Four for four
# on one consonant is not four word defects. The suspicion is the clip's
# opening: F1 added a 200ms lead break precisely because MP3 encoder priming
# eats the start of a file, so the question is whether 200ms is enough.
#
# Direction is NOT predicted. If more lead helps, the padding is short. If it
# makes no difference, the onset is a property of the voice and the fix is a
# signed IPA row per word. Either answer is worth more than four guesses.
set -u
SP="$(cd "$(dirname "$0")" && pwd)"
ROOT="$HOME/spellgame-server"
PORT=8199
FAIL_WORDS="ban bay bear ben"
CTRL_WORDS="bad bed big box"
OUT="$SP/results.tsv"
printf 'lead_ms\tgroup\tword\twhisper\tgoogle_stt\n' > "$OUT"

set -a; . "$ROOT/.env"; set +a
export AUDIO_LEXICON_DIR="$ROOT/audio/lexicon"

for LEAD in 0 200 400; do
  CACHE="$SP/cache-lead$LEAD"; SCRATCH="$CACHE/_scratch"
  mkdir -p "$CACHE" "$SCRATCH/speak"
  export AUDIO_CACHE_DIR="$CACHE" CLIMB_DB_PATH="$SCRATCH/climb.db" SPEAK_METRICS_DIR="$SCRATCH/speak"
  export PAD_LEAD_MS_OVERRIDE="$LEAD"
  cd "$SP/backend" || exit 1
  CLIMB_DB_PATH="$CLIMB_DB_PATH" "$ROOT/venv/bin/python" -c "import db; db.init()" >/dev/null 2>&1
  "$ROOT/venv/bin/gunicorn" --bind "127.0.0.1:$PORT" --workers 2 --timeout 120 \
    --log-file "$SP/lead$LEAD.log" app:app &
  PID=$!
  for _ in $(seq 1 40); do curl -fsS -o /dev/null "http://127.0.0.1:$PORT/api/health" 2>/dev/null && break; sleep 0.5; done
  if ! curl -fsS -o /dev/null "http://127.0.0.1:$PORT/api/health" 2>/dev/null; then
    echo "FATAL: lead=$LEAD instance never healthy"; kill $PID 2>/dev/null; exit 1
  fi
  # Confirm the override is live rather than assuming the copy took.
  LIVE=$(curl -fsS "http://127.0.0.1:$PORT/api/health" >/dev/null 2>&1; "$ROOT/venv/bin/python" -c "
import os; os.environ['PAD_LEAD_MS_OVERRIDE']='$LEAD'
print(int(os.environ.get('PAD_LEAD_MS_OVERRIDE','200')))")
  echo "--- lead=${LEAD}ms (instance pid $PID, cache $(basename $CACHE), override reads $LIVE) ---"

  for GROUP in fail ctrl; do
    [ "$GROUP" = fail ] && WORDS="$FAIL_WORDS" || WORDS="$CTRL_WORDS"
    for W in $WORDS; do
      MP3="$SCRATCH/$W.mp3"
      CODE=$(curl -s -o "$MP3" -w '%{http_code}' "http://127.0.0.1:$PORT/api/speak?word=$W&lang=en&variant=normal")
      if [ "$CODE" != "200" ] || [ ! -s "$MP3" ]; then
        printf '%s\t%s\t%s\t(HTTP %s)\t(HTTP %s)\n' "$LEAD" "$GROUP" "$W" "$CODE" "$CODE" >> "$OUT"; continue
      fi
      WAV="$SCRATCH/$W.wav"
      rm -f "$WAV"
      # Same conversion the F2 and bake-off harnesses use, so a transcript here
      # is comparable with the ones already on record.
      afconvert -f WAVE -d LEI16@16000 "$MP3" "$WAV" 2>/dev/null
      WH=$(whisper-cli -m "$HOME/.cache/whisper/ggml-small.bin" -l en -nt "$WAV" 2>/dev/null | tr -d '\n' | sed 's/^ *//;s/ *$//')
      B64=$(base64 < "$WAV" | tr -d '\n')
      GG=$("$ROOT/venv/bin/python" - "$PORT" "$B64" <<'PY'
import sys, json, urllib.request
port, b64 = sys.argv[1], sys.argv[2]
req = urllib.request.Request(f"http://127.0.0.1:{port}/api/stt",
    data=json.dumps({"lang":"en","audio":b64,"sampleRate":16000}).encode(),
    headers={"Content-Type":"application/json"})
try:
    r = json.load(urllib.request.urlopen(req, timeout=40))
    print((r.get("transcript") or "").strip())
except Exception as e:
    print(f"(stt error: {type(e).__name__})")
PY
)
      printf '%s\t%s\t%s\t%s\t%s\n' "$LEAD" "$GROUP" "$W" "$WH" "$GG" >> "$OUT"
      printf '  %-5s %-5s %-22s | %s\n' "$GROUP" "$W" "$WH" "$GG"
    done
  done
  kill $PID 2>/dev/null; wait $PID 2>/dev/null
done
echo "results -> $OUT"
