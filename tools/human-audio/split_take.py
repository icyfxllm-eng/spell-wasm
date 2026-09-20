#!/usr/bin/env python3
"""CC-HUMAN-AUDIO F4: cut one Gig C take into one clip per word.

The speaker reads a word list top to bottom, one word at a time with a pause
between. This splits the recording on those pauses, transcribes each piece with
whisper.cpp, and walks the list in order, so it can tell a retake from the next
word:

  matches the word expected next   -> that word's clip
  matches the word just recorded   -> a retake; the LATER take wins
  matches neither                  -> set aside (a cough, a false start, chat)

Nothing is guessed: a word the walk never matched is reported as missing, and
its entry simply has no recording. Alignment errors are the one failure that
would attach a clip to the wrong word, so the transcript of every clip is kept
and the auditor still hears each one (I2).

Output is a work folder shaped like harvest.py's, so qc.py, sheet.py, ingest.py
and bundle.py all run on it unchanged.

Run: <venv>/bin/python tools/human-audio/split_take.py --take take.m4a \\
       --list gig-c-record-list.tsv --out <dir> --model ggml-base.en.bin
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tempfile

import numpy as np
import soundfile as sf

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "audio-loudness"))
sys.path.insert(0, str(HERE))
import loudness  # noqa: E402
from qc import WHISPER, norm_text  # noqa: E402

GAP_S = 0.35        # silence this long separates two words
KEEP_S = 0.12       # room kept either side of a word
MIN_WORD_S = 0.12   # anything shorter is a click, not a word
SILENCE_DB = -38.0  # below this, relative to the take's loudest frame, is silence


def segments(samples, rate):
    """[(start, end)] sample ranges of speech, split on GAP_S of silence."""
    hop = max(1, int(0.01 * rate))
    n = len(samples) // hop
    rms = np.sqrt(np.mean(samples[: n * hop].reshape(n, hop) ** 2, axis=1) + 1e-12)
    db = 20 * np.log10(rms)
    loud = db > db.max() + SILENCE_DB
    out, start, gap = [], None, 0
    for i, is_loud in enumerate(loud):
        if is_loud:
            if start is None:
                start = i
            gap = 0
        elif start is not None:
            gap += 1
            if gap * 0.01 >= GAP_S:
                out.append((start, i - gap))
                start = None
    if start is not None:
        out.append((start, n))
    pad = int(KEEP_S / 0.01)
    return [((max(0, s - pad)) * hop, min(len(samples), (e + pad) * hop))
            for s, e in out if (e - s) * 0.01 >= MIN_WORD_S]


def transcribe(model, wavs):
    args = [WHISPER, "-m", str(model), "-l", "en", "-nt", "-np", "-otxt"]
    for w in wavs:
        args += ["-f", str(w)]
    subprocess.run(args, check=True, capture_output=True)
    return [norm_text(pathlib.Path(str(w) + ".txt").read_text()) if pathlib.Path(str(w) + ".txt").exists()
            else "" for w in wavs]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--take", required=True)
    ap.add_argument("--list", required=True, help="tier<TAB>word list the speaker read")
    ap.add_argument("--out", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--speaker", default="eric")
    a = ap.parse_args()
    out = pathlib.Path(a.out).expanduser()
    raw = out / "clips" / "raw"
    raw.mkdir(parents=True, exist_ok=True)

    words = []
    for line in pathlib.Path(a.list).expanduser().read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        tier, word = line.split("\t")[:2]
        words.append((tier, word.strip()))

    samples, rate = loudness.decode(pathlib.Path(a.take).expanduser())
    segs = segments(samples, rate)
    print(f"{len(segs)} spoken pieces for {len(words)} words", file=sys.stderr)

    with tempfile.TemporaryDirectory() as td:
        wavs = []
        for i, (s, e) in enumerate(segs):
            w = pathlib.Path(td) / f"s{i:04d}.wav"
            sf.write(w, samples[s:e], rate, subtype="PCM_16")
            subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEI16@16000", "-c", "1",
                            str(w), str(w) + ".16k.wav"], check=True, capture_output=True)
            wavs.append(str(w) + ".16k.wav")
        heard = transcribe(a.model, wavs)

        # Walk the list in order; a piece that says the word just done is a retake.
        got, extra, i = {}, [], 0
        last = None
        for (s, e), text in zip(segs, heard):
            said = re.sub(r"[^\w\s']", " ", text).strip()
            if i < len(words) and said == norm_text(words[i][1]):
                got[words[i][1]] = (s, e, said)
                last = words[i][1]
                i += 1
            elif last is not None and said == norm_text(last):
                got[last] = (s, e, said)  # retake: the later one wins
            else:
                extra.append((said, round((e - s) / rate, 2)))

        clips = []
        for tier, word in words:
            if word not in got:
                continue
            s, e, said = got[word]
            data = samples[s:e]
            with tempfile.TemporaryDirectory() as td2:
                f32 = pathlib.Path(td2) / "w.wav"
                sf.write(f32, data, rate, subtype="PCM_16")
                digest = hashlib.sha1(f32.read_bytes()).hexdigest()
                (raw / f"{digest}.wav").write_bytes(f32.read_bytes())
            clips.append({
                "entry": word, "key": word, "tiers": [tier],
                "commons_sha1": digest, "raw_path": f"clips/raw/{digest}.wav",
                "license": "Gig C", "speaker": a.speaker,
                "page_url": "", "title": f"Gig C take: {word}",
                "source_url": "", "duration": round((e - s) / rate, 3),
                "heard": said,
            })

    (out / "manifest.json").write_text(json.dumps(
        {"lang": "en", "source": "gig-c", "speaker": a.speaker, "clips": clips},
        ensure_ascii=False, indent=1))
    missing = [w for _, w in words if w not in got]
    (out / "split-report.txt").write_text(
        f"{len(clips)} words cut from {len(segs)} pieces\n"
        + ("missing (never heard in the take): " + ", ".join(missing) + "\n" if missing else "")
        + ("set aside: " + "; ".join(f"{t!r} ({d}s)" for t, d in extra) + "\n" if extra else ""))
    print(f"{len(clips)} clips; {len(missing)} missing; {len(extra)} pieces set aside", file=sys.stderr)
    if missing:
        print("missing: " + ", ".join(missing), file=sys.stderr)


if __name__ == "__main__":
    main()
