"""CC-CEDICT (Mandarin) parser. Format lines:

    trad simp [pin1 yin1] /gloss1/gloss2/

The pinyin field (tone-numbered) IS the answer string. Apostrophe-boundary
words are excluded per Resolved Policy P3. License: CC BY-SA 4.0 — record in
data/zh/SOURCES.md before ingesting.
"""
from __future__ import annotations

import re
from pathlib import Path

from schema import Entry
from . import SourceMissing

URL = "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"
LINE = re.compile(r"^(\S+)\s+(\S+)\s+\[([^\]]+)\]\s+/(.+)/\s*$")


def _pinyin_to_answer(toned: str) -> str:
    # "Ping2 guo3" -> "ping2guo3"; drop tone 5 later in normalize().
    return "".join(toned.lower().split())


def parse(path, lang):
    p = Path(path) if path else None
    if not p or not p.exists():
        raise SourceMissing("cedict", URL, str(p))
    for line in p.read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            continue
        m = LINE.match(line)
        if not m:
            continue
        _trad, simp, pinyin, glosses = m.groups()
        answer = _pinyin_to_answer(pinyin)
        if "'" in pinyin or "·" in pinyin:
            continue  # P3: apostrophe-boundary words excluded
        yield Entry(
            word=answer,
            lang="zh",
            display=simp,
            pron=answer,
            gloss=glosses.split("/")[0],
            sources=["cc-cedict"],
        )
