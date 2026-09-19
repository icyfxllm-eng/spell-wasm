#!/usr/bin/env python3
"""The shared loudness normalizer (CC-HUMAN-AUDIO I7).

One measure, one target, for every clip the app plays: human recordings now,
TTS once it adopts this tool. Loudness is EBU R128 integrated loudness (LUFS)
via pyloudnorm; the target lives in config/audio-loudness.json and nowhere else.

Decoding and encoding use macOS afconvert (no ffmpeg on the build Mac), so this
runs on Eric's machine. Needs a venv with numpy, soundfile, pyloudnorm.

  measure FILE...                 print integrated LUFS and sample peak
  tts-target --cache DIR --bank TSV --lang en [--sample N] [--write]
                                  measure cached server TTS for that language and
                                  (with --write) set the target to its median
  normalize IN OUT [--target L]   trim silence, gain to target, encode AAC .m4a,
                                  re-measure the ENCODED file, report the result
"""
import argparse
import hashlib
import json
import os
import pathlib
import random
import statistics
import subprocess
import sys
import tempfile

import numpy as np
import pyloudnorm as pyln
import soundfile as sf

ROOT = pathlib.Path(__file__).resolve().parents[2]
CONFIG = ROOT / "config" / "audio-loudness.json"

PEAK_CEILING_DB = -1.5     # limiter ceiling before AAC, dBFS (AAC overshoots a little)
LIMIT_FLAG_DB = 6.0        # more gain reduction than this is flagged for a listen
TOLERANCE_LU = 1.0         # I7 / acceptance test 5: within +/-1 LU of target
TRIM_THRESHOLD_DB = -45.0  # frames this far below the clip's loudest frame are silence
TRIM_PAD_S = 0.06          # silence kept at each end after trimming
OUT_RATE = 24000           # matches the server TTS (24 kHz mono)
OUT_BITRATE = 48000        # AAC-LC mono; ~6 KB for a one-second word


def target():
    return json.loads(CONFIG.read_text())["target_lufs"]


def decode(path):
    """Any afconvert-readable file -> (mono float32 samples, rate)."""
    path = str(path)
    if path.lower().endswith(".wav"):
        data, rate = sf.read(path, dtype="float32", always_2d=True)
    else:
        with tempfile.TemporaryDirectory() as td:
            wav = os.path.join(td, "d.wav")
            subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEF32", path, wav],
                           check=True, capture_output=True)
            data, rate = sf.read(wav, dtype="float32", always_2d=True)
    return data.mean(axis=1), rate


def lufs(samples, rate):
    """Integrated loudness. A one-word clip can be shorter than R128's 400 ms
    gating block; padding with silence is loudness-neutral (the absolute gate at
    -70 LUFS drops it), so short clips are padded rather than rejected."""
    need = int(0.4 * rate) + 1
    if len(samples) < need:
        samples = np.concatenate([samples, np.zeros(need - len(samples), dtype=samples.dtype)])
    return pyln.Meter(rate).integrated_loudness(samples)


def peak_db(samples):
    p = float(np.max(np.abs(samples))) if len(samples) else 0.0
    return 20 * np.log10(p) if p > 0 else -float("inf")


def trim(samples, rate):
    """Cut leading/trailing silence, judged per 10 ms frame against the clip's
    loudest frame, keeping TRIM_PAD_S of room at each end."""
    hop = max(1, int(0.01 * rate))
    n = len(samples) // hop
    if n == 0:
        return samples
    frames = samples[: n * hop].reshape(n, hop)
    rms = np.sqrt(np.mean(frames ** 2, axis=1) + 1e-12)
    db = 20 * np.log10(rms)
    loud = np.where(db > db.max() + TRIM_THRESHOLD_DB)[0]
    pad = int(TRIM_PAD_S * rate)
    start = max(0, loud[0] * hop - pad)
    end = min(len(samples), (loud[-1] + 1) * hop + pad)
    return samples[start:end]


def limit(x, rate, ceiling_db=PEAK_CEILING_DB):
    """Lookahead peak limiter. Human recordings carry plosive spikes that TTS
    does not, so hitting the loudness target by gain alone would clip them.
    This turns down only those few milliseconds: the gain envelope is computed
    per 1 ms block (fast attack via 5 ms lookahead, 60 ms release), then
    interpolated per sample. Returns (limited samples, max reduction dB)."""
    ceil = 10 ** (ceiling_db / 20)
    blk = max(1, rate // 1000)
    n = -(-len(x) // blk)
    padded = np.pad(np.abs(x), (0, n * blk - len(x)))
    need = np.minimum(1.0, ceil / np.maximum(padded.reshape(n, blk).max(axis=1), 1e-9))
    look = 5
    need = np.array([need[max(0, i - look): i + look + 1].min() for i in range(n)])
    rel = np.exp(-1.0 / 60.0)  # per-1 ms-block release toward unity
    env = np.empty(n)
    g = 1.0
    for i in range(n):
        g = need[i] if need[i] < g else min(need[i], 1.0 - (1.0 - g) * rel)
        env[i] = g
    per_sample = np.interp(np.arange(len(x)), np.arange(n) * blk + blk / 2, env)
    y = x * per_sample
    y = np.clip(y, -ceil, ceil)  # belt and braces: interpolation can leave a hair over
    return y, float(-20 * np.log10(env.min()))


def normalize(src, dst, tgt=None):
    """Returns a report dict. The loudness that counts is the ENCODED file's:
    AAC changes levels slightly, so the result is re-measured after encoding."""
    tgt = target() if tgt is None else tgt
    samples, rate = decode(src)
    raw_lufs = lufs(samples, rate)
    samples = trim(samples, rate)
    gain = tgt - lufs(samples, rate)
    dst = pathlib.Path(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    # Limiting and AAC both move the level a little, so converge on the loudness
    # of the ENCODED file, the one that ships: encode, measure, correct the gain.
    for _ in range(4):
        out, reduction = limit(samples * (10 ** (gain / 20)), rate)
        with tempfile.TemporaryDirectory() as td:
            wav = os.path.join(td, "n.wav")
            sf.write(wav, out, rate, subtype="FLOAT")
            subprocess.run(["afconvert", "-f", "m4af", "-d", f"aac@{OUT_RATE}", "-c", "1",
                            "-b", str(OUT_BITRATE), wav, str(dst)], check=True, capture_output=True)
        enc, enc_rate = decode(dst)
        final = lufs(enc, enc_rate)
        if abs(final - tgt) <= 0.25:
            break
        gain += tgt - final
    return {"raw_lufs": round(raw_lufs, 2), "gain_db": round(gain, 2),
            "limit_db": round(reduction, 2), "heavy_limiting": bool(reduction > LIMIT_FLAG_DB),
            "final_lufs": round(final, 2),
            "final_peak_db": round(peak_db(enc), 2),
            "within_tolerance": bool(abs(final - tgt) <= TOLERANCE_LU),
            "duration": round(len(enc) / enc_rate, 3), "bytes": dst.stat().st_size}


def tts_target(cache, bank, lang, sample, write):
    words = [l.split("\t")[2] for l in pathlib.Path(bank).read_text().splitlines()
             if l.split("\t")[0] == lang]
    random.Random(7).shuffle(words)
    vals = []
    for w in words:
        key = w if lang == "en" else f"{lang}:{w}"
        p = pathlib.Path(cache) / f"v3_{hashlib.md5(key.encode()).hexdigest()}_normal.mp3"
        if p.exists():
            s, r = decode(p)
            vals.append(lufs(s, r))
        if len(vals) >= sample:
            break
    if not vals:
        sys.exit(f"no cached {lang} TTS found in {cache}")
    med = statistics.median(vals)
    q = statistics.quantiles(vals, n=10)
    print(f"{lang} TTS: n={len(vals)} median {med:.2f} LUFS, p10 {q[0]:.2f}, p90 {q[-1]:.2f}")
    if write:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps({
            "target_lufs": round(med, 1),
            "tolerance_lu": TOLERANCE_LU,
            "measured_from": f"median of {len(vals)} cached {lang} server TTS clips (normal speed)",
            "p10_p90": [round(q[0], 2), round(q[-1], 2)],
            "decided": "Eric, 2026-09-18: match today's TTS loudness (CC-HUMAN-AUDIO I7)",
        }, indent=1) + "\n")
        print(f"wrote {CONFIG}")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("measure")
    m.add_argument("files", nargs="+")
    t = sub.add_parser("tts-target")
    t.add_argument("--cache", required=True)
    t.add_argument("--bank", required=True)
    t.add_argument("--lang", default="en")
    t.add_argument("--sample", type=int, default=400)
    t.add_argument("--write", action="store_true")
    n = sub.add_parser("normalize")
    n.add_argument("src")
    n.add_argument("dst")
    n.add_argument("--target", type=float)
    a = ap.parse_args()
    if a.cmd == "measure":
        for f in a.files:
            s, r = decode(f)
            print(f"{lufs(s, r):7.2f} LUFS  peak {peak_db(s):6.2f} dBFS  {f}")
    elif a.cmd == "tts-target":
        tts_target(a.cache, a.bank, a.lang, a.sample, a.write)
    else:
        print(json.dumps(normalize(a.src, a.dst, a.target)))


if __name__ == "__main__":
    main()
