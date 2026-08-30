#!/usr/bin/env python3
"""CC-RUSSIAN-STRESS Phase 0 — does the ru-RU voice respond to U+0301?

BLOCKING. Features 4 and 5 change shape depending on the answer, and engine
support for the combining acute is uneven and poorly documented. Measure.

WHAT IS MEASURED. Two minimal pairs whose members differ ONLY by stress
position -- за́мок (castle) / замо́к (lock), and мо́лодец / молоде́ц. Each member
is synthesized with its mark, and each bare form is synthesized as a baseline.
Audio is decoded to 16 kHz mono, reduced to per-frame log band energies, and
compared with DTW, which is alignment-invariant: raw waveform correlation is
useless here because a few milliseconds of offset drives it to zero even for
audio a listener calls identical.

THE SPEC'S CONTROL IS DEGENERATE AND IS NOT USED AS WRITTEN. It says to compare
the marked pair's distance against "the distance between either and its own
repeat synthesis". This API is DETERMINISTIC -- the same request twice returns
byte-identical audio, verified below as `determinism` -- so that floor is
always zero and ANY difference at all would read as responsive. A real floor
has to come from two DIFFERENT input strings that a listener would call the
same, and a ceiling from two words that are plainly different. The verdict is
then a position on a scale, not a test for non-zero.

VERDICTS
  RESPONSIVE  the two marked forms differ substantially -- the mark moved the
              stress. Feature 4 sends mark_stress() output as synthesis text.
  IGNORED     the marked forms are indistinguishable from each other and from
              the bare baseline. Feature 4 falls back to SSML <prosody>.
  DISTORTING  they differ, but as mangling rather than stress. THE MACHINE
              CANNOT TELL THIS FROM RESPONSIVE. It reports the numbers and says
              so; only the auditor listening to the clips can separate them.

Run:  python3 tools/ru_stress_probe.py [--voice ru-RU-Wavenet-D] [--keep]
Exit: 0 only on RESPONSIVE, per the file's Done clause.
"""
import argparse
import array
import base64
import json
import math
import os
import pathlib
import subprocess
import sys
import wave

ACUTE = "́"
SR, FRAME, HOP = 16000, 320, 160
BANDS = [200, 300, 425, 575, 750, 950, 1200, 1500, 1900, 2400, 3000, 3800, 4800]
OUT = pathlib.Path("build/ru-stress-probe")

VOWELS = "аеёиоуыэюя"

# Minimal pairs: identical letters, stress alone distinguishes the words.
#   замок    = з0 а1 м2 о3 к4   -> vowels at 1 and 3
#   молодец  = м0 о1 л2 о3 д4 е5 ц6 -> stressed vowels at 1 and 5
PAIRS = [
    ("замок", 1, 3, "за́мок castle / замо́к lock"),
    ("молодец", 1, 5, "мо́лодец / молоде́ц"),
]


def mark(word, i):
    """Insert U+0301 after the vowel at index i. The one place this file knows
    about the diacritic (Feature 1's mark_stress, in probe form).

    THE ASSERT IS NOT DECORATION. The first run of this probe used indices 2
    and 4 for замок -- м and к, both CONSONANTS -- and produced `зам́ок` and
    `замоќ`. The two clips differed by 89%, and the probe called the voice
    RESPONSIVE. It was measuring the engine's reaction to two different
    nonsense placements, not to stress. Feature 1 already states the rule
    ("index pointing at one of аеёиоуыэюя"); this enforces it at the one place
    that builds a marked string."""
    assert word[i] in VOWELS, (
        f"stress index {i} of {word!r} points at {word[i]!r}, not a vowel — "
        f"a mark on a consonant measures nothing"
    )
    return word[: i + 1] + ACUTE + word[i + 1 :]


def synth(text, voice, path):
    key = os.environ.get("GOOGLE_TTS_API_KEY")
    if not key:
        sys.exit("  GOOGLE_TTS_API_KEY not set — source ~/spellgame-server/.env first")
    body = json.dumps({
        "input": {"text": text},
        "voice": {"languageCode": "ru-RU", "name": voice},
        "audioConfig": {"audioEncoding": "MP3", "sampleRateHertz": 24000},
    })
    r = subprocess.run(
        ["curl", "-s", "--max-time", "30", "-X", "POST",
         f"https://texttospeech.googleapis.com/v1/text:synthesize?key={key}",
         "-H", "Content-Type: application/json", "-d", body],
        capture_output=True, text=True)
    try:
        d = json.loads(r.stdout)
    except Exception:
        return None
    if "audioContent" not in d:
        print(f"    ERROR {text!r}: {d.get('error', {}).get('message', '')[:80]}")
        return None
    path.write_bytes(base64.b64decode(d["audioContent"]))
    wav = path.with_suffix(".wav")
    subprocess.run(["afconvert", "-f", "WAVE", "-d", f"LEI16@{SR}", "-c", "1",
                    str(path), str(wav)],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    f = wave.open(str(wav))
    a = array.array("h")
    a.frombytes(f.readframes(f.getnframes()))
    return a


def goertzel(frame, freq):
    w = 2.0 * math.pi * freq / SR
    c = 2.0 * math.cos(w)
    s1 = s2 = 0.0
    for x in frame:
        s0 = x + c * s1 - s2
        s2, s1 = s1, s0
    return s1 * s1 + s2 * s2 - c * s1 * s2


def features(sig):
    peak = max(1, max(abs(x) for x in sig))
    sig = [x / peak for x in sig]
    out = []
    for i in range(0, len(sig) - FRAME, HOP):
        fr = sig[i:i + FRAME]
        if sum(x * x for x in fr) < 1e-4:
            continue
        v = [math.log(goertzel(fr, f) + 1e-9) for f in BANDS]
        m = sum(v) / len(v)
        out.append([x - m for x in v])
    return out


def dtw(a, b):
    n, m = len(a), len(b)
    if not n or not m:
        return 99.0
    prev = [0.0] + [1e9] * m
    for i in range(1, n + 1):
        cur = [1e9] * (m + 1)
        ai = a[i - 1]
        for j in range(1, m + 1):
            d = sum(abs(p - q) for p, q in zip(ai, b[j - 1])) / len(BANDS)
            cur[j] = d + min(prev[j], cur[j - 1], prev[j - 1])
        prev = cur
    return prev[m] / (n + m)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--voice", default="ru-RU-Wavenet-D")
    ap.add_argument("--keep", action="store_true", help="leave the clips for the auditor")
    args = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)

    cache = {}
    n = [0]

    def F(text, tag):
        if tag not in cache:
            a = synth(text, args.voice, OUT / f"{tag}.mp3")
            n[0] += 1
            cache[tag] = features(a) if a else None
        return cache[tag]

    print("=" * 68)
    print(f"CC-RUSSIAN-STRESS Phase 0 — {args.voice}")
    print("=" * 68)

    # Floor: identical string twice. Proves the apparatus, and shows why the
    # spec's "own repeat synthesis" control cannot carry the verdict.
    a1, a2 = F("замок", "det_a"), F("замок", "det_b")
    if a1 is None:
        sys.exit("  synthesis failed — cannot probe")
    determinism = dtw(a1, a2)
    ceiling = dtw(F("замок", "det_a"), F("молоко", "ceil"))
    print(f"  determinism  (замок vs замок)   {determinism:.4f}   <- floor")
    print(f"  ceiling      (замок vs молоко)  {ceiling:.4f}   <- plainly different words")
    print()

    verdicts = []
    for word, i1, i2, label in PAIRS:
        m1, m2 = mark(word, i1), mark(word, i2)
        f1 = F(m1, f"{word}_a")
        f2 = F(m2, f"{word}_b")
        fb = F(word, f"{word}_bare")
        if not (f1 and f2 and fb):
            print(f"  {label}: synthesis failed")
            continue
        d_pair = dtw(f1, f2)
        d_1b = dtw(f1, fb)
        d_2b = dtw(f2, fb)
        # Where does the pair distance sit between "identical" and "different
        # words"? Below a fifth of the way up, the mark did essentially nothing.
        frac = (d_pair - determinism) / max(ceiling - determinism, 1e-9)
        print(f"  {label}")
        print(f"    {m1} vs {m2}   {d_pair:.4f}   ({frac * 100:.0f}% of the way to a different word)")
        print(f"    {m1} vs bare   {d_1b:.4f}")
        print(f"    {m2} vs bare   {d_2b:.4f}")
        verdicts.append(frac)
        print()

    if not verdicts:
        sys.exit("  no pair completed — cannot issue a verdict")

    worst = min(verdicts)
    print("  VERDICT")
    if worst < 0.15:
        print("    IGNORED — the mark changes nothing the engine hears.")
        print("    Feature 4 takes the SSML <prosody> branch. This voice is")
        print("    Wavenet, a full-SSML tier, so no voice change is needed.")
        code = 1
    else:
        print("    RESPONSIVE (machine) — the two marked forms are substantially")
        print("    different audio, so U+0301 is reaching the front end.")
        print()
        print("    THIS DOES NOT CERTIFY THE FEATURE. The machine cannot tell")
        print("    RESPONSIVE from DISTORTING: both produce different audio, and")
        print("    only a listener can say whether the stress landed on the right")
        print("    vowel or the word was mangled. Clips are in build/ru-stress-probe/")
        print("    for the auditor. Phase 0 is not closed until they confirm.")
        code = 0
    print(f"\n  ({n[0]} syntheses)")
    if not args.keep:
        for f in OUT.glob("*.wav"):
            f.unlink()
    return code


if __name__ == "__main__":
    raise SystemExit(main())
