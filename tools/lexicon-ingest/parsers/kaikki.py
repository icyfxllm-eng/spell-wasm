"""kaikki.org machine-readable Wiktionary extract — the uniform fallback for
POS, glosses, and pronunciations across every language. One JSON object per
line. License: CC BY-SA (Wiktionary) — record per language in SOURCES.md.

For ja the answer is the kana reading; for others the headword. Only the
primary reading/sense is taken (multiple standalone spellings would each need
their own reviewed row, out of scope for the scaffold).
"""
from __future__ import annotations

import json
import re
import unicodedata
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

# F5 "word stories": one first-hop origin sentence, NFC, <= this many chars.
# Mirrors src/word_stories.rs::compress so the pipeline output and the runtime
# defensive re-compression agree.
ETY_MAX_LEN = 120
_HOP_STOP = re.compile(r"[—–;:.!?]")  # em/en dash, ; : . ! ?


def compress_etymology(text: str | None) -> str | None:
    """Compress a raw Wiktionary etymology to one first-hop sentence.

    origin language + root + gloss, truncated at the FIRST hop (no chains) and
    hard-capped at ETY_MAX_LEN chars on a word boundary. Returns None if there
    is nothing usable. NFC-normalized. Language-agnostic.
    """
    if not text:
        return None
    nfc = unicodedata.normalize("NFC", text)
    collapsed = " ".join(nfc.split())
    if not collapsed:
        return None
    # First hop: cut at the first sentence terminator / hop separator.
    m = _HOP_STOP.search(collapsed)
    first = (collapsed[: m.start()] if m else collapsed).strip()
    # Drop chains: keep up to (excluding) a second "from" token.
    out, seen_from = [], 0
    for tok in first.split():
        bare = re.sub(r"[^\w]", "", tok, flags=re.UNICODE).lower()
        if bare == "from":
            seen_from += 1
            if seen_from >= 2:
                break
        out.append(tok)
    hop = " ".join(out).strip()
    # Hard length cap at a word boundary, with an ellipsis.
    if len(hop) > ETY_MAX_LEN:
        cut = hop[: ETY_MAX_LEN - 1]
        if " " in cut:
            cut = cut[: cut.rfind(" ")]
        hop = cut.rstrip() + "…"
    hop = hop.strip().rstrip(",;:-–— ").strip()
    return hop or None


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
        # F5 word stories: kaikki exposes the raw Wiktionary etymology prose as
        # `etymology_text`; compress it to one first-hop sentence. CC BY-SA —
        # attribution recorded in data/<lang>/SOURCES.md.
        etymology = compress_etymology(obj.get("etymology_text"))
        yield Entry(
            word=word,
            lang=lang,
            pos=[pos] if pos else [],
            gloss=gloss,
            pron=pron,
            etymology=etymology,
            sources=[f"kaikki-{lang}"],
        )
