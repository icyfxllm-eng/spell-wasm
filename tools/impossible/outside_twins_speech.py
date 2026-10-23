#!/usr/bin/env python3
"""Speech check for the outside-twin candidates (reports/outside-twins/<lang>.csv, kind=internal).

For each (pool word P, dictionary twin T) it asks the shipping voice (/api/speak on the local server) for BOTH
words and listens with whisper-small. Verdicts:
  IDENTICAL_AUDIO  the two clips are the same bytes: the voice says them identically (a real twin to the ear)
  SAME_HEARD       whisper writes the same thing for both (whole transcript, folded): they sound alike
  P_HEARD_AS_T     whisper wrote T when told P (T is what a listener would write)
  DIFFERENT        none of the above: a near-miss, not a twin
  NO_AUDIO         the voice would not say one of them
Usage: outside_twins_speech.py <lang> [max_twins_per_word]
"""
import csv, hashlib, pathlib, subprocess, sys, unicodedata, urllib.parse

lang = sys.argv[1]; per = int(sys.argv[2]) if len(sys.argv) > 2 else 3
OUT = pathlib.Path(f"/tmp/claude-501/otwins-{lang}"); OUT.mkdir(parents=True, exist_ok=True)
nfc = lambda s: unicodedata.normalize("NFC", s)

def fold(s):
    s = nfc(s.lower()).replace("ё", "е")
    return "".join(c for c in s if c.isalnum())

def clip(word):
    mp3 = OUT / (hashlib.md5(word.encode()).hexdigest()[:16] + ".mp3"); wav = mp3.with_suffix(".wav")
    if not wav.exists():
        url = f"http://127.0.0.1:8080/api/speak?word={urllib.parse.quote(word)}&variant=normal&lang={lang}"
        r = subprocess.run(["curl", "-s", "-m", "40", "-o", str(mp3), "-w", "%{http_code}", url], capture_output=True, text=True)
        if r.stdout != "200": return None, None
        subprocess.run(["afconvert", "-f", "WAVE", "-d", "LEI16@16000", "-c", "1", str(mp3), str(wav)], capture_output=True)
        if not wav.exists(): return None, None
    return mp3, wav

_heard = {}
def hear(wav):
    if wav not in _heard:
        r = subprocess.run(["whisper-cli", "-m", str(pathlib.Path.home() / ".cache/whisper/ggml-small.bin"), "-l", lang, "-nt", "-np", "-f", str(wav)],
                           capture_output=True, text=True, timeout=120)
        _heard[wav] = r.stdout.strip().replace("\n", " ")
    return _heard[wav]

rows = [r for r in csv.DictReader(open(f"reports/outside-twins/{lang}.csv", encoding="utf8")) if r["kind"] == "internal"]
out = []
for r in rows:
    p = r["pool_word"]
    for t in r["twins_sample"].split()[:per]:
        if fold(t) == fold(p): continue  # e/yo and case: already accepted
        mp, wp = clip(p); mt, wt = clip(t)
        if not wp or not wt:
            out.append((p, t, "NO_AUDIO", "", "")); continue
        hp, ht = hear(wp), hear(wt)
        if mp.read_bytes() == mt.read_bytes(): v = "IDENTICAL_AUDIO"
        elif fold(hp) == fold(ht) and fold(hp): v = "SAME_HEARD"
        elif fold(hp) == fold(t) or fold(t) in [fold(x) for x in hp.split()]: v = "P_HEARD_AS_T"
        else: v = "DIFFERENT"
        out.append((p, t, v, hp, ht))
    with open(f"reports/outside-twins/{lang}-speech.csv", "w", newline="", encoding="utf8") as f:
        w = csv.writer(f); w.writerow(["pool_word", "twin", "verdict", "heard_pool", "heard_twin"]); w.writerows(out)
from collections import Counter
print(lang, Counter(v for _, _, v, _, _ in out))
