# Spell-Aloud loopback (CC-SPELL-ALOUD Phase 5)

Closes the loop on the voice-spelling parser with **real** speech: a system TTS speaks
letter-name sequences at three speeds × three voices, **whisper.cpp** transcribes them
offline, and the transcript is fed to the *same* parser the app uses
(`examples/spell_aloud_parse`). Scores **letter accuracy**; the merge gate is **≥ 95%**
(acceptance #3).

This is the real counterpart to the **offline oracle** in
`src/spell_aloud/loopback.rs`, which runs on every PR (deterministic, no audio) over a
50-word suite per language. This harness runs **offline, manually / on a dedicated CI
job** — like `tools/audio-verify/` — because it needs whisper + a model and is slow.

## Why this shape

- **The app is device-only** (on-device `SFSpeechRecognizer`); this exercises the
  *parser* over synthetic ASR, which is where the linguistic logic lives (Invariant I4:
  one lexicon, no scattered letter maps). It does **not** test the iOS speech stack.
- **Fail-closed:** with no whisper/model (or no TTS) it prints the exact setup command
  and exits without fabricating a pass — same contract as `tools/audio-verify/verify.py`.

## Setup

```bash
# whisper.cpp + a multilingual model
git clone https://github.com/ggerganov/whisper.cpp && cd whisper.cpp && make
./models/download-ggml-model.sh large-v3
```

TTS: macOS `say` is built in (best quality here); otherwise install `espeak-ng`.

## Run

```bash
python3 tools/spell-aloud-loopback/loopback.py \
  --whisper /path/to/whisper.cpp/build/bin/whisper-cli \
  --model   /path/to/whisper.cpp/models/ggml-large-v3.bin
# one language / custom voices:
python3 tools/spell-aloud-loopback/loopback.py --whisper ... --model ... \
  --lang es --voices "Mónica,Paulina,Jorge"
```

Output is one PASS/FAIL line per language with the mean letter-accuracy and the
worst-scoring clip (word → parsed, at which voice/speed), so you listen only to the
flagged tail.

## Notes

- The 12-word/language subset here is a **spot-check** for real ASR quality; the
  exhaustive 50-word parser coverage is the `cargo test` oracle. Keep the two suites in
  rough sync when adding words.
- `CONFUSABLE_CONFIDENCE` (confusable chips) and the es b/v chip are parser behaviours
  the oracle asserts; this harness measures end-to-end letter accuracy only.
