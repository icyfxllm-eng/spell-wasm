"""Frequency ranks. Reads a plain "word<TAB>count" or "word count" list (as
exported from the wordfreq package or a Leipzig corpus) and yields entries
carrying only freq_rank — the driver merges these onto the dictionary entries.
Rank is 1-based by descending frequency. License: verify per source
(wordfreq data is redistributable; Leipzig is research-licensed).
"""
from __future__ import annotations

from pathlib import Path

from schema import Entry
from . import SourceMissing

URL = "https://github.com/rspeer/wordfreq  (or Leipzig Corpora Collection)"


def parse(path, lang):
    p = Path(path) if path else None
    if not p or not p.exists():
        raise SourceMissing(f"wordfreq-{lang}", URL, str(p))
    rows = []
    for line in p.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.replace("\t", " ").split()
        word = parts[0]
        count = int(parts[1]) if len(parts) > 1 and parts[1].isdigit() else 0
        rows.append((word, count))
    rows.sort(key=lambda r: (-r[1], r[0]))
    for rank, (word, _count) in enumerate(rows, start=1):
        yield Entry(word=word, lang=lang, freq_rank=rank, sources=[f"wordfreq-{lang}"])
