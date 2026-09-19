#!/usr/bin/env python3
"""CC-HUMAN-AUDIO F2: normalize and auto-QC harvested clips.

For every clip in harvest.py's manifest:
  1. clipping     runs of full-scale samples in the ORIGINAL recording
  2. noise        signal-to-noise from the recording's own quiet frames
  3. duration     trimmed length within the language's bounds
  4. loudness     trim + gain + limit + AAC encode via the shared normalizer
                  (tools/audio-loudness, I7); the encoded file must land within
                  tolerance of config/audio-loudness.json's target
  5. loopback     whisper.cpp must hear the intended word

Any failure rejects the clip WITH its reasons (and whisper's transcript), so a
reject can be reviewed. Passing is flag-only (I2): it puts a clip on the F3
sheet for a native speaker, it never makes a clip playable.

Needs the Phase B venv (numpy, soundfile, pyloudnorm), whisper-cli and a model.

Run: <venv>/bin/python tools/human-audio/qc.py --dir ~/repos/ha-census-cache/phase-b/en \\
       --model ~/repos/ha-census-cache/phase-b/models/ggml-base.en.bin
Writes <dir>/qc.json and <dir>/clips/norm/<sha1>.m4a.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import unicodedata

import numpy as np

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "audio-loudness"))
import loudness  # noqa: E402  (the shared normalizer, I7)

WHISPER = "/opt/homebrew/bin/whisper-cli"
# Trimmed-duration bounds in seconds, per language. A one-word citation form
# shorter than this is a cut-off take; longer is a phrase, a false start or a
# repeated take.
DURATION = {"en": (0.2, 2.5)}
SNR_FLOOR_DB = 20.0
CLIP_RUN = 3  # consecutive full-scale samples that count as clipping


def clipping(samples):
    """Longest run of samples at (or within a hair of) digital full scale."""
    hot = np.abs(samples) >= 0.999
    if not hot.any():
        return 0
    edges = np.diff(np.concatenate([[0], hot.astype(np.int8), [0]]))
    starts, ends = np.where(edges == 1)[0], np.where(edges == -1)[0]
    return int((ends - starts).max())


def snr_db(samples, rate):
    """Speech level (frames within 20 dB of the loudest) minus noise level (the
    quietest tenth of frames), in 20 ms frames of the untrimmed recording."""
    hop = max(1, int(0.02 * rate))
    n = len(samples) // hop
    if n < 10:
        return None
    rms = np.sqrt(np.mean(samples[: n * hop].reshape(n, hop) ** 2, axis=1) + 1e-12)
    db = 20 * np.log10(rms)
    speech = db[db > db.max() - 20]
    noise = np.sort(db)[: max(1, n // 10)]
    return float(speech.mean() - noise.mean())


def norm_text(s):
    s = unicodedata.normalize("NFKC", s).lower()
    s = re.sub(r"\[[^\]]*\]|\([^)]*\)", " ", s)  # [BLANK_AUDIO], (music)
    s = re.sub(r"[^\w\s'-]", " ", s)
    return " ".join(s.replace("-", " ").split())


def load_homophones(repo, lang):
    """The bank's generated collision sets: whisper hearing 'there' for 'their'
    is not a wrong word, it is the same sound."""
    p = repo / "assets" / "words" / lang / "homophones.txt"
    sets = {}
    if p.exists():
        for line in p.read_text().splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            words = [norm_text(w) for w in re.split(r"[\s,|]+", line) if w.strip()]
            for w in words:
                sets.setdefault(w, set()).update(words)
    return sets


def transcribe(model, clips, lang):
    """whisper-cli over many files per call (loading the model once), each
    resampled to the 16 kHz mono it expects. Returns {sha1: transcript}."""
    out = {}
    with tempfile.TemporaryDirectory() as td:
        for i in range(0, len(clips), 100):
            batch = clips[i:i + 100]
            wavs = []
            for sha, src in batch:
                w = f"{td}/{sha}.wav"
                subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEI16@16000", "-c", "1", str(src), w],
                               check=True, capture_output=True)
                wavs.append(w)
            args = [WHISPER, "-m", str(model), "-l", lang, "-nt", "-np", "-otxt"]
            for w in wavs:
                args += ["-f", w]
            subprocess.run(args, check=True, capture_output=True)
            for sha, _ in batch:
                t = pathlib.Path(f"{td}/{sha}.wav.txt")
                out[sha] = t.read_text().strip() if t.exists() else ""
            print(f"  loopback {min(i + 100, len(clips))}/{len(clips)}", file=sys.stderr)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--repo", default=str(HERE.parents[1]))
    a = ap.parse_args()
    d = pathlib.Path(a.dir).expanduser()
    man = json.loads((d / "manifest.json").read_text())
    lang = man["lang"]
    lo, hi = DURATION[lang]
    tgt = loudness.target()
    homs = load_homophones(pathlib.Path(a.repo), lang)
    prev = json.loads((d / "qc.json").read_text()) if (d / "qc.json").exists() else {}
    results = {}

    todo = []
    for rec in man["clips"]:
        sha = rec["commons_sha1"]
        if sha in prev.get("clips", {}):
            results[sha] = prev["clips"][sha]
            continue
        src = d / rec["raw_path"]
        samples, rate = loudness.decode(src)
        r = {"key": rec["key"], "reasons": []}
        run = clipping(samples)
        if run >= CLIP_RUN:
            r["reasons"].append(f"clipping ({run} full-scale samples in a row)")
        snr = snr_db(samples, rate)
        r["snr_db"] = None if snr is None else round(snr, 1)
        if snr is not None and snr < SNR_FLOOR_DB:
            r["reasons"].append(f"noise (SNR {snr:.1f} dB < {SNR_FLOOR_DB:.0f})")
        n = loudness.normalize(src, d / "clips" / "norm" / f"{sha}.m4a", tgt)
        r.update({"final_lufs": n["final_lufs"], "limit_db": n["limit_db"],
                  "duration": n["duration"], "bytes": n["bytes"],
                  "norm_path": f"clips/norm/{sha}.m4a"})
        if not lo <= n["duration"] <= hi:
            r["reasons"].append(f"duration {n['duration']:.2f}s outside {lo}-{hi}s")
        if not n["within_tolerance"]:
            r["reasons"].append(f"loudness {n['final_lufs']} LUFS not within ±1 of {tgt} "
                                f"(needed {n['limit_db']} dB limiting)")
        results[sha] = r
        todo.append((sha, src))
        if len(todo) % 100 == 0:
            print(f"  measured {len(todo)}", file=sys.stderr)

    heard = transcribe(a.model, todo, lang) if todo else {}
    for sha, text in heard.items():
        r = results[sha]
        r["transcript"] = text
        got, want = norm_text(text), norm_text(r["key"])
        if got != want and got not in homs.get(want, set()):
            r["reasons"].append(f"loopback heard {text!r}")

    for r in results.values():
        r["status"] = "rejected" if r["reasons"] else "to_audit"
    summary = {"to_audit": sum(r["status"] == "to_audit" for r in results.values()),
               "rejected": sum(r["status"] == "rejected" for r in results.values()),
               "target_lufs": tgt}
    (d / "qc.json").write_text(json.dumps({"lang": lang, "summary": summary, "clips": results},
                                          ensure_ascii=False, indent=1))
    print(json.dumps(summary), file=sys.stderr)


if __name__ == "__main__":
    main()
