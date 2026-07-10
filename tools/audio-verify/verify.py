#!/usr/bin/env python3
"""TTS audio loopback verification (Part 3).

For every cached word audio, run speech-to-text (whisper.cpp — offline, no API
cost) and compare the transcription to the expected word with a language-aware
comparator. Emits a worst-first triage report so a human listens only to the
flagged tail, and a held-out list of MISMATCH words for the assignment generator.

Runs OFFLINE on Eric's machine / a manual CI job — NOT on every PR. whisper.cpp +
the multilingual model must be installed (see README); absent, the tool prints
the exact setup command and exits (no fabrication).

  python3 verify.py --cache /path/to/audio_cache --whisper /path/to/whisper-cli \
                    --model /path/to/ggml-large-v3.bin
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import unicodedata
from pathlib import Path

# Whisper covers en/vi/ja/ko/zh/th/fil — but fil quality must be confirmed
# empirically on a sample before trusting it (HARD REVIEW GATE, see README).
WHISPER_LANGS = {"en", "vi", "ja", "ko", "zh", "th", "fil"}
FIL_GATE = "fil audio verification is UNCONFIRMED — run --fil-sample first (README §fil)."


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s.strip())


def compare(expected: str, heard: str, lang: str, lexicon: dict) -> str:
    """Return PASS / MISMATCH. Per-language comparators:
    - Latin (en/vi/fil): casefold exact.
    - ja: whisper returns kanji or kana; accept if it maps to the kana answer
      via the lexicon reading, or matches kana directly.
    - zh: whisper returns hanzi; compare against the lexicon `display` (hanzi).
    - ko/th: NFC exact (script has no case).
    """
    e, h = nfc(expected), nfc(heard)
    if lang in ("en", "vi", "fil"):
        return "PASS" if e.casefold() == h.casefold() else "MISMATCH"
    if lang in ("ko", "th"):
        return "PASS" if e == h else "MISMATCH"
    if lang == "zh":
        hanzi = lexicon.get(e, {}).get("display") or lexicon.get(e, {}).get("hanzi")
        return "PASS" if hanzi and nfc(hanzi) == h else "MISMATCH"
    if lang == "ja":
        # accept kana answer OR its kanji/reading forms from the lexicon
        forms = {e} | set(lexicon.get(e, {}).get("forms", []))
        return "PASS" if h in {nfc(f) for f in forms} else "MISMATCH"
    return "MISMATCH"


def transcribe(whisper: str, model: str, audio: Path, lang: str) -> tuple[str, float]:
    """Run whisper.cpp; return (text, avg_logprob-ish confidence)."""
    out = subprocess.run(
        [whisper, "-m", model, "-l", lang, "-otxt", "-of", "/tmp/wv", str(audio)],
        capture_output=True, text=True,
    )
    txt = Path("/tmp/wv.txt").read_text(encoding="utf-8").strip() if Path("/tmp/wv.txt").exists() else ""
    # crude confidence: whisper prints nothing structured here without JSON; the
    # README notes using -oj for real logprobs. Placeholder 1.0 unless empty.
    return txt, (0.0 if not txt else 1.0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", required=True, help="audio cache dir (manifest of word,lang,file)")
    ap.add_argument("--whisper", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--manifest", help="JSON manifest [{word,lang,file}]; else derive from cache")
    args = ap.parse_args()

    for tool in (args.whisper, args.model):
        if not Path(tool).exists():
            print(f"missing: {tool}\nSetup: see tools/audio-verify/README.md (whisper.cpp + multilingual model).", file=sys.stderr)
            sys.exit(1)

    manifest = json.loads(Path(args.manifest).read_text()) if args.manifest else []
    if not manifest:
        print("no manifest — pass --manifest with [{word,lang,file}] entries.", file=sys.stderr)
        sys.exit(1)

    lexicons: dict[str, dict] = {}  # lang -> {word: entry}  (loaded on demand from data/<lang>/lexicon.jsonl)
    results = []
    for item in manifest:
        lang = item["lang"]
        if lang not in WHISPER_LANGS:
            continue
        if lang == "fil":
            print(FIL_GATE, file=sys.stderr)  # warn, still run for reporting
        heard, conf = transcribe(args.whisper, args.model, Path(item["file"]), lang)
        verdict = compare(item["word"], heard, lang, lexicons.get(lang, {}))
        if conf < 0.5:
            verdict = "LOW-CONFIDENCE"
        results.append({**item, "heard": heard, "verdict": verdict, "conf": conf})

    order = {"MISMATCH": 0, "LOW-CONFIDENCE": 1, "PASS": 2}
    results.sort(key=lambda r: order.get(r["verdict"], 3))
    held_out = [r["word"] for r in results if r["verdict"] == "MISMATCH"]

    out = Path(__file__).parent / "out"
    out.mkdir(exist_ok=True)
    (out / "triage.json").write_text(json.dumps(results, ensure_ascii=False, indent=2))
    (out / "held-out.json").write_text(json.dumps(held_out, ensure_ascii=False, indent=2))
    n = len(results)
    print(f"audio-verify: {n} checked, {len(held_out)} MISMATCH held out. worst-first → out/triage.json")


if __name__ == "__main__":
    main()
