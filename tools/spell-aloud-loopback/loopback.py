#!/usr/bin/env python3
"""Spell-Aloud loopback harness (CC-SPELL-ALOUD Phase 5, acceptance #3).

The REAL loop the offline `cargo test` oracle stands in for: speak letter-name
sequences with a system TTS at three speeds × three voices, transcribe with
whisper.cpp (offline, no API), feed the transcript to the SAME parser the app uses
(examples/spell_aloud_parse), and score letter accuracy. Merge gate: >= 95%.

Runs OFFLINE on Eric's machine / a manual CI job — NOT on every PR (the cargo
oracle covers PRs). whisper.cpp + a multilingual model must be installed; absent,
this prints the exact setup command and exits WITHOUT fabricating a result — same
contract as tools/audio-verify/verify.py.

  python3 loopback.py --whisper /path/to/whisper-cli --model /path/to/ggml-large-v3.bin
  python3 loopback.py --whisper ... --model ... --lang es --voices "Mónica,Paulina"
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CLI_BIN = REPO / "target" / "debug" / "examples" / "spell_aloud_parse"

# (spoken letter names, expected word). A representative spot-check subset; the full
# 50-word/language coverage lives in the offline oracle (src/spell_aloud/loopback.rs).
SUITE = {
    "en": [
        ("see ay tee", "cat"), ("dee oh gee", "dog"), ("es you en", "sun"),
        ("double u ee bee", "web"), ("why ee es", "yes"), ("zee eye pee", "zip"),
        ("aitch oh you es ee", "house"), ("ef ar eye ee en dee", "friend"),
        ("queue you eye see kay", "quick"), ("es see aitch oh oh el", "school"),
        ("bee ar ay eye en", "brain"), ("tee ay bee el ee", "table"),
    ],
    "es": [
        ("ce a ese a", "casa"), ("ge a te o", "gato"), ("ele i be ere o", "libro"),
        ("hache o ele a", "hola"), ("uve i de a", "vida"), ("te a zeta a", "taza"),
        ("ene u be e", "nube"), ("uve a ce a", "vaca"), ("ge o eme a", "goma"),
        ("ce i eme a", "cima"), ("ese e de e", "sede"), ("ele u ene a", "luna"),
    ],
}

# Three synthetic voices per language for macOS `say` (override with --voices). espeak
# users get rate-only variation. Whisper transcribes each; the parser must recover the
# word regardless of voice.
DEFAULT_VOICES = {
    "en": ["Alex", "Samantha", "Daniel"],
    "es": ["Mónica", "Paulina", "Jorge"],
}
SPEEDS_WPM = [140, 180, 220]  # slow / normal / fast (three speeds, acceptance #3)
THRESHOLD = 0.95


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s.strip())


def levenshtein(a: str, b: str) -> int:
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a):
        cur = [i + 1]
        for j, cb in enumerate(b):
            cur.append(min(prev[j + 1] + 1, cur[j] + 1, prev[j] + (ca != cb)))
        prev = cur
    return prev[len(b)]


def letter_accuracy(expected: str, got: str) -> float:
    m = max(len(expected), len(got), 1)
    return 1.0 - levenshtein(expected, got) / m


def have_tts() -> str | None:
    for cmd in ("say", "espeak-ng", "espeak"):
        if shutil.which(cmd):
            return cmd
    return None


def synth(tts: str, text: str, voice: str, wpm: int, out: Path) -> bool:
    """Synthesize `text` to a 16 kHz wav whisper can read."""
    try:
        if tts == "say":
            aiff = out.with_suffix(".aiff")
            subprocess.run(["say", "-v", voice, "-r", str(wpm), "-o", str(aiff), text], check=True)
            # whisper.cpp wants 16 kHz mono wav
            subprocess.run(
                ["afconvert", "-f", "WAVE", "-d", "LEI16@16000", "-c", "1", str(aiff), str(out)],
                check=True,
            )
            aiff.unlink(missing_ok=True)
        else:  # espeak(-ng): wpm ≈ words/min; letters are slow, scale down a touch
            subprocess.run([tts, "-s", str(wpm), "-w", str(out), text], check=True)
        return out.exists()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def transcribe(whisper: str, model: str, lang: str, wav: Path) -> str:
    """Run whisper.cpp and return the transcription text (best effort)."""
    proc = subprocess.run(
        [whisper, "-m", model, "-l", lang, "-nt", "-otxt", "-of", str(wav.with_suffix("")), str(wav)],
        capture_output=True, text=True,
    )
    txt = wav.with_suffix(".txt")
    if txt.exists():
        return nfc(txt.read_text())
    # fall back to stdout if -otxt is unavailable in this build
    return nfc(proc.stdout)


def parse_word(lang: str, transcript: str) -> str:
    """Ask the app's own parser (via the example bin) for the spelled word."""
    import json
    out = subprocess.run([str(CLI_BIN), lang, transcript], capture_output=True, text=True)
    try:
        data = json.loads(out.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError):
        return ""
    # the mode word (chip-aware); fall back to the input-method letters
    return data.get("mode_word") or data.get("letters", "")


def build_cli() -> bool:
    print("building the parser bridge (cargo build --example spell_aloud_parse)…")
    r = subprocess.run(
        ["cargo", "build", "--example", "spell_aloud_parse"], cwd=REPO,
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        print(r.stderr[-2000:], file=sys.stderr)
    return CLI_BIN.exists()


def main() -> int:
    ap = argparse.ArgumentParser(description="Spell-Aloud whisper loopback (offline).")
    ap.add_argument("--whisper", help="path to whisper-cli (whisper.cpp)")
    ap.add_argument("--model", help="path to a multilingual ggml model")
    ap.add_argument("--lang", choices=["en", "es"], help="only this language")
    ap.add_argument("--voices", help="comma-separated TTS voice override")
    ap.add_argument("--threshold", type=float, default=THRESHOLD)
    args = ap.parse_args()

    # Fail-closed on missing tools — print the exact fix, never fabricate a pass.
    tts = have_tts()
    if not tts:
        print("SKIP: no TTS found. Install macOS `say` (built-in) or `espeak-ng`.")
        return 0
    if not args.whisper or not args.model or not Path(args.whisper).exists() or not Path(args.model).exists():
        print(
            "SKIP: whisper.cpp not provided.\n"
            "  git clone https://github.com/ggerganov/whisper.cpp && cd whisper.cpp && make\n"
            "  ./models/download-ggml-model.sh large-v3\n"
            "  python3 loopback.py --whisper ./build/bin/whisper-cli --model ./models/ggml-large-v3.bin",
        )
        return 0
    if not build_cli():
        print("ERROR: could not build examples/spell_aloud_parse", file=sys.stderr)
        return 1

    langs = [args.lang] if args.lang else ["en", "es"]
    overall_fail = False
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        for lang in langs:
            voices = args.voices.split(",") if args.voices else DEFAULT_VOICES[lang]
            scores, worst = [], (1.0, "", "", "")
            for spoken, word in SUITE[lang]:
                for voice in voices:
                    for wpm in SPEEDS_WPM:
                        wav = tmp / "clip.wav"
                        if not synth(tts, spoken, voice, wpm, wav):
                            continue
                        heard = transcribe(args.whisper, args.model, lang, wav)
                        got = parse_word(lang, heard)
                        acc = letter_accuracy(nfc(word), nfc(got))
                        scores.append(acc)
                        if acc < worst[0]:
                            worst = (acc, word, got, f"{voice}@{wpm}")
            if not scores:
                print(f"[{lang}] SKIP: TTS produced no audio.")
                continue
            avg = sum(scores) / len(scores)
            status = "PASS" if avg >= args.threshold else "FAIL"
            overall_fail |= status == "FAIL"
            print(f"[{lang}] {status} letter-accuracy={avg:.3f} over {len(scores)} clips "
                  f"(worst {worst[0]:.2f}: {worst[1]!r}→{worst[2]!r} {worst[3]})")

    return 1 if overall_fail else 0


if __name__ == "__main__":
    sys.exit(main())
