#!/usr/bin/env python3
"""CC-SENSE-CUE v1.1 F5 -- the voice loopback, FLAG-ONLY (S2, Eric 2026-10-16).

    python3 tools/soundalike_loopback.py --lang en      (also en, es, ru; needs the local server on :8080)

For every kind "true" set (and English's re-tier table), each member is synthesized
by the Layer-1 voice through /api/speak -- the clip the app plays -- decoded to
16 kHz mono and compared pairwise with DTW over log band energies. The method and
the per-run calibration are tools/spellophones-voice-check.py's: a SAME-SOUND
control (two spellings any listener calls identical) sets the floor, a DIFFERENT
control (two unrelated words) the ceiling, and the bar sits a fifth of the way up.

A set whose worst pair is over the bar gets loopbackFlag "voice-may-distinguish"
and goes to the auditor sheet (F6). The loopback NEVER changes a set's kind (I9):
whisper-style or acoustic evidence on isolated words is not proof, and a false
demotion would drop a real sound-alike out of accept-all grading.

Results: assets/words/<lang>/soundalike-loopback.json, keyed by setId, with the
voice they were measured against. tools/build_collisions.py applies them, and a
table whose every true set has a result for its current voice reads "verified".
"""
import argparse
import importlib.util
import itertools
import json
import pathlib
import subprocess
import sys
import time
import urllib.parse
import wave
import array

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from soundalikes import layer1_voices, soundalikes  # noqa: E402

_spec = importlib.util.spec_from_file_location("vc", ROOT / "tools" / "spellophones-voice-check.py")
vc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(vc)          # features(), dtw(), is_mp3(), SR: one implementation

API = "http://127.0.0.1:8080/api/speak"
CACHE = ROOT / "build" / "loopback"
CONTROLS = {
    "en": {"same": [("gray", "grey"), ("center", "centre"), ("color", "colour"), ("disc", "disk")],
           "different": [("pair", "dog"), ("right", "ocean")]},
    # b/v and silent h are homophones in every Spanish accent, Castilian included.
    "es": {"same": [("basto", "vasto"), ("hola", "ola"), ("bello", "vello")],
           "different": [("casa", "perro"), ("mesa", "ventana")]},
    # Final devoicing: pairs every Russian listener hears as one word.
    "ru": {"same": [("пруд", "прут"), ("код", "кот"), ("луг", "лук")],
           "different": [("дом", "окно"), ("стол", "рыба")]},
}
STORE = lambda lang: ROOT / "assets" / "words" / lang / "soundalike-loopback.json"


def audio(word, lang):
    d = CACHE / lang
    d.mkdir(parents=True, exist_ok=True)
    safe = urllib.parse.quote(word, safe="")
    mp3, wav = d / f"{safe}.mp3", d / f"{safe}.wav"
    if not wav.exists():
        if not vc.is_mp3(mp3):
            mp3.unlink(missing_ok=True)
            url = f"{API}?word={urllib.parse.quote(word)}&variant=normal&lang={lang}"
            for attempt in range(3):
                if attempt:
                    time.sleep(2 ** attempt)
                subprocess.run(["curl", "-s", "--max-time", "30", url, "-o", str(mp3)])
                if vc.is_mp3(mp3):
                    break
                mp3.unlink(missing_ok=True)
            else:
                return None
        r = subprocess.run(["afconvert", "-f", "WAVE", "-d", f"LEI16@{vc.SR}", "-c", "1", str(mp3), str(wav)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if r.returncode:
            return None
    f = wave.open(str(wav))
    a = array.array("h")
    a.frombytes(f.readframes(f.getnframes()))
    return a


def run(lang):
    feats = {}

    def F(w):
        if w not in feats:
            s = audio(w, lang)
            feats[w] = vc.features(s) if s else None
        return feats[w]

    same = [vc.dtw(F(a), F(b)) for a, b in CONTROLS[lang]["same"] if F(a) and F(b)]
    diff = [vc.dtw(F(a), F(b)) for a, b in CONTROLS[lang]["different"] if F(a) and F(b)]
    if not same or not diff:
        sys.exit(f"{lang}: calibration failed -- could not synthesize the control words (is the server on :8080?)")
    floor, ceiling = max(same), min(diff)
    bar = floor + (ceiling - floor) * 0.20
    sets = {}
    tables = [soundalikes(lang)] + ([soundalikes(lang, variant="retier")] if lang == "en" else [])
    for t in tables:
        for s in t["sets"]:
            if s["kind"] != "true" or s["setId"] in sets:
                continue
            ms = [m["memberId"] for m in s["members"]]
            worst, pair, missing = 0.0, None, []
            for a, b in itertools.combinations(ms, 2):
                fa, fb = F(a), F(b)
                if not fa or not fb:
                    missing += [w for w, f in ((a, fa), (b, fb)) if not f]
                    continue
                d = vc.dtw(fa, fb)
                if d > worst:
                    worst, pair = d, [a, b]
            if missing:
                sets[s["setId"]] = {"members": ms, "flag": "no-audio", "missing": sorted(set(missing))}
            else:
                sets[s["setId"]] = {"members": ms, "worst": round(worst, 4), "worstPair": pair,
                                    "flag": "voice-may-distinguish" if worst > bar else None}
    store = {
        "$doc": "CC-SENSE-CUE v1.1 F5 loopback results (flag-only). Written by tools/soundalike_loopback.py; "
                "read by tools/build_collisions.py. Never changes a set's kind.",
        "lang": lang, "voice": layer1_voices().get(lang), "measured": time.strftime("%Y-%m-%d"),
        "calibration": {"sameSoundFloor": round(floor, 4), "differentCeiling": round(ceiling, 4), "bar": round(bar, 4),
                        "controls": CONTROLS[lang]},
        "sets": dict(sorted(sets.items())),
    }
    STORE(lang).write_text(json.dumps(store, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    flagged = [v for v in sets.values() if v["flag"]]
    print(f"{lang}: {len(sets)} true sets measured; floor {floor:.4f}, ceiling {ceiling:.4f}, bar {bar:.4f}; "
          f"{len(flagged)} flagged")
    return store


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", required=True, choices=sorted(CONTROLS))
    run(ap.parse_args().lang)
