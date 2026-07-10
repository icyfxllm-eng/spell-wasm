"""JMdict (Japanese) parser — the gold standard: kana readings (our answer
strings), POS, glosses, and priority tags (news/ichi/spec/nfXX → freq_rank).
Expects the JSON build of JMdict (jmdict-eng, one entry per record). License:
EDRDG / CC BY-SA — record the required attribution in data/ja/SOURCES.md.

Answer = the primary kana reading. Entries whose reading isn't kana-only are
skipped (out of the kana keyboard's charset).
"""
from __future__ import annotations

import json
from pathlib import Path

from schema import Entry
from . import SourceMissing

URL = "https://github.com/scriptin/jmdict-simplified/releases  (jmdict-eng JSON)"
POS_MAP = {"n": "noun", "v": "verb", "adj": "adj", "adv": "adv"}


def _kana_only(s: str) -> bool:
    return all(0x3040 <= ord(c) <= 0x30FF for c in s)


def _nf_rank(tags) -> int | None:
    # JMdict priority tag nfXX buckets frequency in blocks of 500.
    for t in tags or []:
        if t.startswith("nf") and t[2:].isdigit():
            return (int(t[2:]) - 1) * 500 + 1
    return None


def parse(path, lang):
    p = Path(path) if path else None
    if not p or not p.exists():
        raise SourceMissing("jmdict", URL, str(p))
    data = json.loads(p.read_text(encoding="utf-8"))
    for word_entry in data.get("words", []):
        kana_forms = word_entry.get("kana", [])
        if not kana_forms:
            continue
        primary = next((k for k in kana_forms if k.get("common")), kana_forms[0])
        reading = primary.get("text", "")
        if not reading or not _kana_only(reading):
            continue
        senses = word_entry.get("sense", [])
        pos, gloss = [], None
        if senses:
            for raw in senses[0].get("partOfSpeech", []):
                mapped = POS_MAP.get(raw[:3]) or POS_MAP.get(raw[:1])
                if mapped:
                    pos.append(mapped)
            gl = senses[0].get("gloss", [])
            if gl:
                gloss = gl[0].get("text")
        yield Entry(
            word=reading,
            lang="ja",
            pron=reading,
            pos=pos,
            gloss=gloss,
            freq_rank=_nf_rank(primary.get("tags")),
            sources=["jmdict"],
        )
