"""Migrate the shipped flat lists (assets/words/<lang>/<tier>.txt) into lexicon
entries. This is the bridge from today's data to the schema — every current
word becomes an Entry with source=curated and its tier recorded as a domain
(so the migration diff can show tier moves later). Mandarin's "pinyin|hanzi"
entries split into word=pinyin, display=hanzi.
"""
from __future__ import annotations

from pathlib import Path

from schema import Entry

ROOT = Path(__file__).resolve().parent.parent.parent.parent  # repo root
TIERS = ["easy", "medium", "hard", "expert"]


def parse(path, lang):
    base = ROOT / "assets" / "words" / lang
    for tier in TIERS:
        f = base / f"{tier}.txt"
        if not f.exists():
            continue
        for raw in f.read_text(encoding="utf-8").splitlines():
            w = raw.strip()
            if not w or w.startswith("#"):
                continue
            display = None
            if lang == "zh" and "|" in w:
                w, display = w.split("|", 1)
            yield Entry(
                word=w,
                lang=lang,
                display=display,
                domains=[f"tier:{tier}"],
                sources=["curated"],
            )
