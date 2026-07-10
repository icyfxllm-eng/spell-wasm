"""kaikki.org machine-readable Wiktionary extract — the uniform fallback for
POS, glosses, and pronunciations across every language. One JSON object per
line. License: CC BY-SA (Wiktionary) — record per language in SOURCES.md.

For ja the answer is the kana reading; for others the headword. Only the
primary reading/sense is taken (multiple standalone spellings would each need
their own reviewed row, out of scope for the scaffold).
"""
from __future__ import annotations

import json
from pathlib import Path

from schema import Entry
from . import SourceMissing

URL = "https://kaikki.org/dictionary/rawdata.html"  # per-language dump index

# Wiktionary POS -> shared tagset.
POS_MAP = {
    "noun": "noun", "verb": "verb", "adj": "adj", "adv": "adv", "num": "num",
    "pron": "pron", "det": "det", "prep": "prep", "conj": "conj",
    "particle": "part", "intj": "interj",
}


def parse(path, lang):
    p = Path(path) if path else None
    if not p or not p.exists():
        raise SourceMissing(f"kaikki-{lang}", URL, str(p))
    for line in p.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        obj = json.loads(line)
        word = obj.get("word")
        if not word:
            continue
        # ja: prefer the kana reading as the answer string.
        if lang == "ja":
            forms = obj.get("forms", [])
            kana = next((f["form"] for f in forms if "hiragana" in f.get("tags", [])), None)
            word = kana or word
        pos = POS_MAP.get(obj.get("pos", ""))
        gloss = None
        senses = obj.get("senses", [])
        if senses:
            gl = senses[0].get("glosses")
            if gl:
                gloss = gl[0]
        pron = None
        for s in obj.get("sounds", []):
            if "ipa" in s:
                pron = s["ipa"]
                break
        yield Entry(
            word=word,
            lang=lang,
            pos=[pos] if pos else [],
            gloss=gloss,
            pron=pron,
            sources=[f"kaikki-{lang}"],
        )
