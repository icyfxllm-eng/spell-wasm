#!/usr/bin/env python3
"""CC-IMPOSSIBLE-COMPLETE v1 F1 -- the exclusion_only gate's data, one file per language.

    python3 tools/impossible/build_exclusion.py           # write data/impossible/exclusion/<lang>.json
    python3 tools/impossible/build_exclusion.py --check   # gate: committed files match a fresh build

A language with an exclusion file is `exclusion_only`: Impossible may open, nothing else (D-C1: no
cues, no sound-alike bonus; the SENSE-CUE table stays `missing`). The excluded set is the union of
  1. the RULES NET (tools/impossible/rules_nets.py): Expert words sharing a coarse sound key with
     another bank word, and
  2. the LOOPBACK NET as Eric reviewed it: every KEEP row of the reviewed verdict sheets (a true
     sound-alike, an accent merger or an accept-both variant spelling), together with the bank word
     it was flagged against when that word is itself an Expert row (a pair is excluded whole).
REJECT rows (recogniser noise) stay in the pool. Machines never decide KEEP: the verdict sheets in
reports/ are the signed input, and a bank edit that changes the Expert tier changes the file, which
the gate then flags as stale.

Held languages (hi, vi, ar) get no file: their loopback recogniser was too weak to trust, so they
stay locked as no_exclusion_gate until a stronger check exists. Languages with a verified
SENSE-CUE table (en, es, ru) never read these files.

Loopback results are keyed by (word, voice, voice version) (D-C16): the file records the voice and
the clip-cache version it was reviewed against, so a voice change is visible here.
"""
import csv
import json
import pathlib
import re
import sys
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "impossible"))
import rules_nets  # noqa: E402
import soundalikes as sa  # noqa: E402

OUT = ROOT / "data" / "impossible" / "exclusion"
# Words a listener ruled out by ear (an audio review), kept apart from the sound-alike gates because they
# are a human verdict about specific words, not a net: data/impossible/reviewed/<lang>.json.
REVIEWED = ROOT / "data" / "impossible" / "reviewed"
AUDIO_REVIEWS = {"en": "reports/impossible-audio-review-en.csv"}
GATED = ("fr", "de", "pt", "pl", "ko", "fil", "sw")
HELD = ("hi", "vi", "ar")  # no gate: recogniser too weak (whisper-small heard 14/287 hi clips)
REVIEWS = (
    "reports/impossible-loopback-keep-ko-sw.csv",
    "reports/impossible-loopback-verdicts-batch2.csv",
)


def nfc(s):
    return unicodedata.normalize("NFC", s)


def tiers(lang):
    txt = (ROOT / "src" / "word_data.rs").read_text(encoding="utf-8")
    out = {}
    for t in ("EASY", "MEDIUM", "HARD", "EXPERT"):
        m = re.search(rf"pub const {lang.upper()}_{t}: &\[&str\] = &\[(.*?)\];", txt, re.S)
        out[t] = [nfc(w) for w in json.loads("[" + m.group(1) + "]")]
    return out


def reviewed(lang):
    keep = {}
    for rel in REVIEWS:
        path = ROOT / rel
        for row in csv.DictReader(path.open(encoding="utf-8-sig")):
            if row["lang"] == lang and row["verdict"].startswith("KEEP"):
                keep[nfc(row["spoken_word"])] = [nfc(x.strip()) for x in row["flagged_against"].split(";") if x.strip()]
    return keep


def cache_version():
    m = re.search(r'^CACHE_VERSION\s*=\s*"([^"]+)"', (ROOT / "backend" / "app.py").read_text(encoding="utf-8"), re.M)
    return m.group(1) if m else None


def build(lang):
    t = tiers(lang)
    expert = list(dict.fromkeys(t["EXPERT"]))
    whole = list(dict.fromkeys(w for v in t.values() for w in v))
    rules = rules_nets.flagged(lang, expert, whole)
    keep = reviewed(lang)
    expert_by_lower = {w.lower(): w for w in expert}
    loop = {w for w in keep if w in set(expert)}
    partners = {expert_by_lower[p.lower()] for ps in keep.values() for p in ps if p.lower() in expert_by_lower}
    excluded = sorted(rules | loop | partners)
    return {
        "lang": lang,
        "status": "exclusion_only",
        "voice": sa.layer1_voices().get(lang),
        "clipCacheVersion": cache_version(),
        "recogniser": "whisper.cpp ggml-small, one transcript per clip (top-1)",
        "expertRows": len(expert),
        "rulesNet": sorted(rules),
        "loopbackKeep": sorted(loop),
        "loopbackPartners": sorted(partners - loop),
        "excluded": excluded,
    }


def build_reviewed(lang):
    """Words the audio review marked `other` (a different real spelling fits the sound): out of the pool."""
    path = ROOT / AUDIO_REVIEWS[lang]
    rows = list(csv.DictReader(path.open(encoding="utf-8-sig")))
    return {
        "lang": lang,
        "source": AUDIO_REVIEWS[lang],
        "rule": "verdict 'other' = another real spelling fits the sound, so the word cannot be spelled from audio alone (I1)",
        "excluded": sorted(nfc(r["word"]) for r in rows if r["verdict"] == "other"),
    }


def render(doc):
    return json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


def main(argv):
    check = "--check" in argv
    bad = []
    for lang in GATED:
        path = OUT / f"{lang}.json"
        text = render(build(lang))
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                bad.append(f"{path.relative_to(ROOT)} is stale: run python3 tools/impossible/build_exclusion.py")
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    for lang in AUDIO_REVIEWS:
        path = REVIEWED / f"{lang}.json"
        text = render(build_reviewed(lang))
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                bad.append(f"{path.relative_to(ROOT)} is stale: run python3 tools/impossible/build_exclusion.py")
        else:
            REVIEWED.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    for lang in HELD:
        if (OUT / f"{lang}.json").exists():
            bad.append(f"{lang} is a held language: it must not have an exclusion file")
    if bad:
        print("impossible exclusion: FAILED", *bad, sep="\n  ", file=sys.stderr)
        return 1
    if check:
        print("impossible exclusion: OK")
    else:
        for lang in GATED:
            d = json.loads((OUT / f"{lang}.json").read_text(encoding="utf-8"))
            print(f"{lang:4} expert={d['expertRows']:5} rules={len(d['rulesNet']):4} loopback={len(d['loopbackKeep']):3} "
                  f"+partners={len(d['loopbackPartners']):3} excluded={len(d['excluded']):4} pool={d['expertRows'] - len(d['excluded'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
