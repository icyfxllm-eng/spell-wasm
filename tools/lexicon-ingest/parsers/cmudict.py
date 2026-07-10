"""CMUdict (English pronunciations) — feeds the P1 homophone detection. Yields
one entry per headword carrying only `pron` (the ARPAbet phoneme string, stress
digits stripped so homophones collide). Same-`pron` groups become homophone
sets. License: BSD-style (CMU) — record in data/en/SOURCES.md.
"""
from __future__ import annotations

import re
from pathlib import Path

from schema import Entry
from . import SourceMissing

URL = "https://github.com/cmusphinx/cmudict/raw/master/cmudict.dict"
STRESS = re.compile(r"\d")


def parse(path, lang):
    p = Path(path) if path else None
    if not p or not p.exists():
        raise SourceMissing("cmudict", URL, str(p))
    for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line or line.startswith(";;;"):
            continue
        parts = line.split()
        word = parts[0].lower()
        if word.endswith(")"):  # alternate pronunciation "word(2)"
            word = word[: word.rindex("(")]
        if not word.isalpha():
            continue
        phones = STRESS.sub("", " ".join(parts[1:]))  # drop stress for homophones
        yield Entry(word=word, lang="en", pron=phones, sources=["cmudict"])
