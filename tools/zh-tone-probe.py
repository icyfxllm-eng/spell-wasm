#!/usr/bin/env python3
"""CC-ZH-TONE Done 6 — verify the SYNTHESIZED TONE, by listening to the pitch.

The first attempt at this gate ran the audio through an ASR and compared the
result to the intended pinyin. That cannot work, and the reason is worth
keeping: the chain was audio -> whisper -> hanzi -> pypinyin -> pinyin, and the
last hop is a DICTIONARY LOOKUP OF THE CHARACTERS. For a fixed transcription the
pinyin comes out the same whatever was actually spoken, so the test measured the
ASR's spelling and was blind to the very thing the feature is about.

Tone in Mandarin IS the pitch contour of the syllable, and the clip is raw
16 kHz PCM, so the contour can simply be measured:

    1  high level        flat
    2  rising            climbs
    3  low dipping       falls then rises, and sits low
    4  high falling      drops

No transcription anywhere. The ASR is off the critical path.

**The calibration is the point.** Forcing tone N on ONE syllable and checking
the contour comes back as N tests the lever against ground truth we control --
if forcing 2 yields a rise and forcing 4 yields a fall, then <phoneme> governs
tone, which is the whole claim. A classifier validated against its own guesses
would prove nothing, so it is validated against what was ASKED FOR.

Run:  GOOGLE_TTS_API_KEY=... python3 tools/zh-tone-probe.py
"""
from __future__ import annotations

import base64
import html
import json
import math
import os
import pathlib
import struct
import sys
import urllib.request
import wave

ROOT = pathlib.Path(__file__).resolve().parents[1]
VOICE, LANG = "cmn-CN-Wavenet-A", "cmn-CN"
# One open syllable per tone, all on the same rime, so the only thing differing
# between clips is the tone itself.
CALIBRATION = [("妈", "ma", 1), ("麻", "ma", 2), ("马", "ma", 3), ("骂", "ma", 4),
               ("拔", "ba", 2), ("把", "ba", 3), ("爸", "ba", 4), ("八", "ba", 1)]
# HELD OUT. The thresholds above were tuned on CALIBRATION, so reporting
# CALIBRATION accuracy alone would be a classifier grading its own homework --
# the same circularity that made the sandhi audit worth hand-deriving. These
# syllables have different rimes and were never used to pick a threshold.
VALIDATION = [("低", "di", 1), ("敌", "di", 2), ("底", "di", 3), ("地", "di", 4),
              ("书", "shu", 1), ("熟", "shu", 2), ("鼠", "shu", 3), ("树", "shu", 4),
              ("发", "fa", 1), ("罚", "fa", 2), ("法", "fa", 3), ("话", "hua", 4)]


def synth(text_or_ssml: str, is_ssml: bool, path: pathlib.Path) -> None:
    key = os.environ["GOOGLE_TTS_API_KEY"]
    body = {
        "input": ({"ssml": text_or_ssml} if is_ssml else {"text": text_or_ssml}),
        "voice": {"languageCode": LANG, "name": VOICE},
        "audioConfig": {"audioEncoding": "LINEAR16", "sampleRateHertz": 16000},
    }
    req = urllib.request.Request(
        f"https://texttospeech.googleapis.com/v1/text:synthesize?key={key}",
        data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        path.write_bytes(base64.b64decode(json.load(r)["audioContent"]))


def read_pcm(path: pathlib.Path):
    with wave.open(str(path), "rb") as w:
        assert w.getsampwidth() == 2 and w.getnchannels() == 1, "expected 16-bit mono"
        sr = w.getframerate()
        raw = w.readframes(w.getnframes())
    return sr, list(struct.unpack(f"<{len(raw)//2}h", raw))


def f0_contour(sr, x, fmin=70.0, fmax=400.0):
    """Voiced pitch per frame, by autocorrelation.

    Deliberately plain: the question is whether the contour rises or falls, not
    what the speaker's exact hertz was, so a robust cheap estimator beats a
    precise fragile one.
    """
    import numpy as np
    a = np.asarray(x, dtype=np.float64)
    if a.size == 0:
        return []
    a /= max(1.0, np.abs(a).max())
    win, hop = int(sr * 0.040), int(sr * 0.010)
    lo, hi = int(sr / fmax), int(sr / fmin)
    out = []
    for s in range(0, max(0, len(a) - win), hop):
        f = a[s:s + win]
        e = float(np.sqrt(np.mean(f * f)))
        if e < 0.04:                      # silence / unvoiced
            out.append(None); continue
        f = f - f.mean()
        c = np.correlate(f, f, mode="full")[len(f) - 1:]
        if c[0] <= 0:
            out.append(None); continue
        seg = c[lo:hi]
        if seg.size == 0:
            out.append(None); continue
        k = int(np.argmax(seg)) + lo
        # A weak peak means no real periodicity: unvoiced rather than a guess.
        out.append(sr / k if c[k] / c[0] > 0.30 else None)
    return out


def classify(f0):
    """Name the contour of ONE syllable: 1 level, 2 rising, 3 dipping, 4 falling."""
    v = [p for p in f0 if p]
    if len(v) < 6:
        return None, {}
    import numpy as np
    a = np.asarray(v, dtype=float)
    # Semitones relative to the syllable's own median: speaker-independent, and
    # what "rising" actually means to an ear.
    st = 12.0 * np.log2(a / np.median(a))
    n = len(st)
    head, tail = st[: max(2, n // 3)].mean(), st[-max(2, n // 3):].mean()
    slope = tail - head
    dip = st.min() - min(head, tail)          # how far it sags below its ends
    feat = {"slope_st": round(float(slope), 2), "dip_st": round(float(dip), 2),
            "range_st": round(float(st.max() - st.min()), 2), "frames": n}
    # Order matters, and getting it wrong is instructive: testing the dip first
    # called a RISING tone a dipping one, because tone 2 commonly sags a little
    # before it climbs. Tone 4 is the steepest fall in the language, so it is
    # separated by magnitude rather than by sign -- tone 3 also ends lower than
    # it began, just far less steeply.
    if slope < -4.0:
        return 4, feat
    if slope > 1.6:
        return 2, feat
    if dip < -1.6 and st.argmin() not in (0, n - 1):
        return 3, feat
    return 1, feat


def voiced_runs(f0, min_len=6, gap=3):
    """Split a contour into syllables at the unvoiced gaps between them."""
    runs, cur, hole = [], [], 0
    for p in f0:
        if p:
            if hole >= gap and cur:
                runs.append(cur); cur = []
            cur.append(p); hole = 0
        else:
            hole += 1
    if cur:
        runs.append(cur)
    return [r for r in runs if len(r) >= min_len]


def classify_in_word(syl, word_median):
    """Tone of one syllable INSIDE a word, judged against the word's register.

    An isolated-syllable classifier scores badly here and the reason is
    phonetic, not acoustic: the HALF-THIRD. Tone 3 shows its dip-and-rise only
    in isolation or utterance-finally; inside a word it is simply a LOW tone
    that falls a little. Judged by contour alone it looks like a 4, or like a 1.

    So the register matters as much as the shape, which is what Chao's tone
    letters encode: 3 is the low one, 1 the high level one, 4 the high one that
    drops. Normalizing to the WORD's median rather than the syllable's is what
    makes "low" a meaningful word at all.
    """
    import numpy as np
    a = np.asarray(syl, dtype=float)
    if a.size < 5:
        return None
    st = 12.0 * np.log2(a / word_median)
    n = len(st)
    head, tail = st[: max(2, n // 3)].mean(), st[-max(2, n // 3):].mean()
    slope, level = tail - head, float(np.median(st))
    if slope < -3.5 and level > -1.5:
        return 4                      # high, and drops steeply
    if slope > 1.6:
        return 2                      # climbs
    if level < -1.0:
        return 3                      # sits low: the half-third
    if slope < -2.5:
        return 4
    return 1


def syllable_tones(f0, n):
    """Tones of the n syllables in a word.

    Prefer the real unvoiced gaps between syllables. Mandarin does not always
    give you one -- two open syllables can run together with no break at all --
    so an equal split of the voiced material is the fallback rather than a
    refusal to answer.
    """
    runs = voiced_runs(f0)
    if len(runs) != n:
        flat = [p for p in f0 if p]
        if len(flat) < n * 6:
            return [None] * n
        k = len(flat) // n
        runs = [flat[i * k:(i + 1) * k] for i in range(n)]
    import numpy as np
    flat = [p for p in f0 if p]
    wm = float(np.median(np.asarray(flat, dtype=float))) if flat else 1.0
    return [classify_in_word(r, wm) for r in runs]


def probe_words() -> int:
    """The 30 polyphone words, measured by pitch rather than transcribed."""
    words = json.loads((ROOT / "config/zh-polyphone-set.json").read_text())
    out = ROOT / "content-pipeline/wordpic/.tone-probe/words"
    out.mkdir(parents=True, exist_ok=True)
    import re
    rows = graded = ok = 0, 0, 0
    rows, graded, ok, neutral = [], 0, 0, 0
    for i, w in enumerate(words, 1):
        # TTS speaks the SURFACE form (F5), so that is what the audio should show.
        syls = re.findall(r"[a-zü]+[0-5]", w["surface"])
        ph = " ".join(s.replace("v", "ü") for s in syls)
        ssml = (f'<speak><phoneme alphabet="pinyin" ph="{html.escape(ph, quote=True)}">'
                f'{html.escape(w["hanzi"])}</phoneme></speak>')
        f = out / f"{i:02d}.wav"
        synth(ssml, True, f)
        sr, x = read_pcm(f)
        got = syllable_tones(f0_contour(sr, x), len(syls))
        want = [int(s[-1]) for s in syls]
        per = []
        for g, t in zip(got, want):
            if t == 5:
                # Neutral is short, low and toneless by definition; a contour
                # classifier has nothing to read. Counted, never graded.
                neutral += 1; per.append("neutral"); continue
            graded += 1
            hit = g == t
            ok += hit
            per.append("OK" if hit else f"want {t} heard {g}")
        rows.append({"hanzi": w["hanzi"], "surface": w["surface"], "asked": ph,
                     "want": want, "heard": got, "per_syllable": per})
        print(f'  {i:2}/{len(words)} {w["hanzi"]:6} {ph:20} {" ".join(per)}')
    print(f"\n  TONED SYLLABLES {ok}/{graded} correct   ({neutral} neutral, not gradable)")
    (ROOT / "config/zh-tone-probe-words.json").write_text(
        json.dumps({"ran": "2026-08-22", "voice": VOICE,
                    "method": "F0 autocorrelation per syllable, no ASR",
                    "toned_correct": ok, "toned_total": graded,
                    "neutral_skipped": neutral, "rows": rows},
                   ensure_ascii=False, indent=1) + "\n")
    return 0 if ok == graded else 1


def main() -> int:
    out = ROOT / "content-pipeline/wordpic/.tone-probe"
    out.mkdir(parents=True, exist_ok=True)
    rows, ok = [], 0
    for hanzi, seg, tone in CALIBRATION + VALIDATION:
        ph = f"{seg}{tone}"
        ssml = (f'<speak><phoneme alphabet="pinyin" ph="{html.escape(ph, quote=True)}">'
                f'{html.escape(hanzi)}</phoneme></speak>')
        p = out / f"{seg}{tone}.wav"
        synth(ssml, True, p)
        sr, x = read_pcm(p)
        got, feat = classify(f0_contour(sr, x))
        hit = got == tone
        ok += hit
        held = (hanzi, seg, tone) in VALIDATION
        rows.append({"hanzi": hanzi, "asked": ph, "heard_tone": got, "match": hit,
                     "held_out": held, **feat})
        print(f"  {'[held] ' if held else '       '}{hanzi} forced {ph:5} -> tone {got}"
              f"  {'OK' if hit else 'MISS'}   slope {feat.get('slope_st')} st, "
              f"dip {feat.get('dip_st')} st")
    cal = sum(r["match"] for r in rows if not r["held_out"])
    val = sum(r["match"] for r in rows if r["held_out"])
    print(f"\n  tuned    {cal}/{len(CALIBRATION)}")
    print(f"  HELD OUT {val}/{len(VALIDATION)} — this is the number that means anything")
    (ROOT / "config/zh-tone-probe-result.json").write_text(
        json.dumps({"ran": "2026-08-22", "voice": VOICE,
                    "method": "F0 autocorrelation, contour in semitones, no ASR",
                    "calibration": rows}, ensure_ascii=False, indent=1) + "\n")
    return 0 if val == len(VALIDATION) else 1


if __name__ == "__main__":
    raise SystemExit(probe_words() if "--words" in sys.argv else main())
