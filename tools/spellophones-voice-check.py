#!/usr/bin/env python3
"""CC-SPELL-O-PHONES — does the PINNED VOICE actually merge each set?

D13 pinned this mode to Pack -> ServerCache (never NativeTts), and that pin
opened an obligation nothing else in the file covers.

F0's sets came from CMUdict. CMUdict is a phonemic dictionary of General
American -- IT IS NOT THE VOICE. It says `pair`, `pear` and `pare` share a
pronunciation. It does not promise the pinned voice renders them
indistinguishably, and "the player cannot tell them apart by ear" is the entire
premise of the mode. A set the dictionary calls homophonous and the voice
renders distinctly is an unwinnable question that looks winnable -- F2's
failure arriving through the audio path, which neither F0 nor F2 catches.

WHAT IS MEASURED. Each member is synthesized through the pinned source
(/api/speak, the same clip the app plays), decoded to 16 kHz mono, reduced to
per-frame log band energies, and compared pairwise with DTW.

DTW, not waveform correlation. Raw sample correlation was tried first and is
useless here: a few milliseconds of offset drives it to zero even for audio a
listener would call identical (`to` vs `too` measured -0.054 while sounding the
same). DTW compares SHAPE and is alignment-invariant.

THE THRESHOLD IS CALIBRATED EVERY RUN, NEVER HARDCODED. Two different input
strings never produce byte-identical audio from this engine, so "distance 0" is
the wrong bar. The controls below fix the real one:

  SAME-SOUND controls   gray/grey, center/centre, color/colour, disc/disk.
                        Two spellings of ONE word -- any listener calls these
                        identical, so whatever they measure IS the floor for
                        "indistinguishable". Observed 0.485-0.545.
  DIFFERENT controls    pair/dog, right/ocean. The ceiling. Observed 0.92-1.00.

A set passes if every pair sits at or below the same-sound floor (plus a
margin). It is SUSPECT above that and BROKEN near the different-word band.
Deriving the bar from controls on every run means a change of voice moves the
bar with it, instead of silently invalidating a hardcoded number.

Run:  python3 tools/spellophones-voice-check.py [--limit N]
"""
import argparse
import array
import itertools
import math
import pathlib
import re
import subprocess
import sys
import time
import urllib.parse
import wave
from collections import defaultdict

ROOT = pathlib.Path(__file__).resolve().parent.parent
CACHE = ROOT / "build" / "voice-check"
CMU = ROOT / "tools/wordpipe/sources/cmudict.dict"
API = "https://spellgame.net/api/speak"
SR, FRAME, HOP = 16000, 320, 160
BANDS = [200, 300, 425, 575, 750, 950, 1200, 1500, 1900, 2400, 3000, 3800, 4800]
STRESS = re.compile(r"\d")

SAME_SOUND = [("gray", "grey"), ("center", "centre"), ("color", "colour"), ("disc", "disk")]
DIFFERENT = [("pair", "dog"), ("right", "ocean")]


def bank():
    src = (ROOT / "src/words.rs").read_text() + (ROOT / "src/word_data.rs").read_text()
    out = {}
    for tier in ("easy", "medium", "hard", "expert"):
        m = re.search(rf"pub const EN_{tier.upper()}: &\[&str\] = &\[(.*?)\];", src, re.S)
        if m:
            for e in re.findall(r'"([^"]+)"', m.group(1)):
                out.setdefault(e.split("|")[0].lower(), tier)
    return out


def sets_from_bank():
    """The same grouping the inventory tool uses: a set IS its banked members."""
    B = bank()
    groups = defaultdict(set)
    for line in CMU.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line or line.startswith(";;;"):
            continue
        parts = line.split()
        w = parts[0].lower()
        if w.endswith(")"):
            w = w[: w.rindex("(")]
        if w in B:
            groups[" ".join(STRESS.sub("", p) for p in parts[1:])].add(w)
    # Dedupe by MEMBERS, not by pronunciation string. A word with two cmudict
    # entries can put the same member set under two keys -- hour/our arrived
    # twice in the first run for exactly that reason.
    uniq = {tuple(sorted(v)) for v in groups.values() if len(v) > 1}
    return sorted((list(t) for t in uniq), key=len, reverse=True)


def is_mp3(p):
    """A 200 with a body is not proof of audio. Cloudflare answers a throttled
    request with a 17-byte `error code: 1015` body, and the first version of
    this tool CACHED that as if it were a clip -- reporting 28 sets as having
    no audio when the truth was that it had hammered the endpoint with a
    hundred rapid requests. Check the actual frame header."""
    try:
        head = p.read_bytes()[:3]
    except OSError:
        return False
    return p.stat().st_size >= 512 and (head[:3] == b"ID3" or head[0] == 0xFF)


def audio(word):
    CACHE.mkdir(parents=True, exist_ok=True)
    mp3, wav = CACHE / f"{word}.mp3", CACHE / f"{word}.wav"
    if not wav.exists():
        if not is_mp3(mp3):
            mp3.unlink(missing_ok=True)   # never keep a poisoned cache entry
            url = f"{API}?word={urllib.parse.quote(word)}&variant=normal&lang=en"
            for attempt in range(4):
                if attempt:
                    time.sleep(2 ** attempt)      # back off a throttle
                subprocess.run(["curl", "-s", "--max-time", "25", url, "-o", str(mp3)])
                if is_mp3(mp3):
                    break
                mp3.unlink(missing_ok=True)
            else:
                return None
            time.sleep(0.35)              # stay under the rate limit
        if not is_mp3(mp3):
            return None
        r = subprocess.run(["afconvert", "-f", "WAVE", "-d", f"LEI16@{SR}", "-c", "1",
                            str(mp3), str(wav)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if r.returncode:
            return None
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
            continue                      # silence carries no identity
        v = [math.log(goertzel(fr, f) + 1e-9) for f in BANDS]
        m = sum(v) / len(v)
        out.append([x - m for x in v])    # mean-subtract: cancel loudness
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
    ap.add_argument("--limit", type=int, default=0, help="check only the first N sets")
    args = ap.parse_args()

    feats = {}

    def F(w):
        if w not in feats:
            s = audio(w)
            feats[w] = features(s) if s else None
        return feats[w]

    print("=" * 70)
    print("CC-SPELL-O-PHONES — voice check against the pinned source")
    print("=" * 70)
    print("  calibrating on controls (the bar is derived, never hardcoded)")
    same, diff = [], []
    for x, y in SAME_SOUND:
        a, b = F(x), F(y)
        if a and b:
            d = dtw(a, b)
            same.append(d)
            print(f"    same-sound  {x:>8} / {y:<8} {d:.4f}")
    for x, y in DIFFERENT:
        a, b = F(x), F(y)
        if a and b:
            d = dtw(a, b)
            diff.append(d)
            print(f"    different   {x:>8} / {y:<8} {d:.4f}")
    if not same or not diff:
        sys.exit("  calibration failed — could not synthesize the control words")

    floor, ceiling = max(same), min(diff)
    # Margin: a fifth of the gap between "identical to a listener" and
    # "obviously a different word". Wide enough to absorb the engine's
    # per-string jitter, narrow enough that a real phonetic split shows up.
    bar = floor + (ceiling - floor) * 0.20
    print(f"\n  indistinguishable floor : {floor:.4f}   (worst same-sound control)")
    print(f"  different-word ceiling  : {ceiling:.4f}")
    print(f"  PASS BAR                : {bar:.4f}\n")

    all_sets = sets_from_bank()
    if args.limit:
        all_sets = all_sets[: args.limit]
    verified, suspect, failed = [], [], []
    for ms in all_sets:
        worst, worst_pair, ok = 0.0, None, True
        for x, y in itertools.combinations(ms, 2):
            a, b = F(x), F(y)
            if not a or not b:
                ok = False
                break
            d = dtw(a, b)
            if d > worst:
                worst, worst_pair = d, (x, y)
        if not ok:
            failed.append((ms, "no audio"))
        elif worst <= bar:
            verified.append((ms, worst))
        else:
            suspect.append((ms, worst, worst_pair))

    print(f"  VERIFIED — the voice merges these ({len(verified)}):")
    for ms, d in sorted(verified, key=lambda x: x[1]):
        print(f"    {d:.4f}  {'/'.join(ms)}")
    if suspect:
        print(f"\n  SUSPECT — the voice may distinguish these ({len(suspect)}).")
        print("  Each is an unwinnable question if it ships: the player is told two")
        print("  spellings sound alike and can in fact hear which one it is.")
        for ms, d, pr in sorted(suspect, key=lambda x: -x[1]):
            print(f"    {d:.4f}  {'/'.join(ms):<28} worst pair {pr[0]}/{pr[1]}")
    if failed:
        print(f"\n  NO AUDIO ({len(failed)}) — cannot be shipped, cannot be verified:")
        for ms, why in failed:
            print(f"    {'/'.join(ms)}  ({why})")
    print(f"\n  {len(verified)} verified, {len(suspect)} suspect, {len(failed)} unusable")
    return 1 if suspect or failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
