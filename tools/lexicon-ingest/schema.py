"""Unified lexicon schema — one entry format for all languages.

An entry is the structured record the assignment/difficulty/curation pipeline
reads instead of guessing from a bare word string. See data/LEXICON.md.

The `word` field is the ANSWER string (kana for ja, pinyin for zh, hangul for
ko, plain for Latin), already normalized. `display` is what is shown after
answering when it differs (zh: the hanzi). All fields except `word`/`lang` are
optional / nullable so partial-coverage sources still contribute.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field, asdict

# Shared, small POS tagset. Source-specific tags are mapped into these; anything
# unmapped is dropped from `pos` (and, if a word has ONLY function-word POS, it
# gets the `function_word` flag and is excluded from pools by default).
POS_TAGS = {"noun", "verb", "adj", "adv", "num", "pron", "det", "prep", "conj", "part", "interj"}
CONTENT_POS = {"noun", "verb", "adj", "adv"}          # good spelling words
FUNCTION_POS = {"pron", "det", "prep", "conj", "part", "interj"}


@dataclass
class Entry:
    word: str                       # answer string, normalized
    lang: str
    display: str | None = None      # shown after answering if != word (zh hanzi)
    freq_rank: int | None = None    # 1 = most common; None if unknown
    pos: list[str] = field(default_factory=list)
    domains: list[str] = field(default_factory=list)
    pron: str | None = None         # collision key (romanization/reading)
    grade: str | None = None        # HSK-1 / JLPT-N5 / TOPIK-1 / … ; None if none
    gloss: str | None = None        # short English gloss (internal)
    etymology: str | None = None    # one first-hop origin story (F5 word stories)
    kid_ok: bool | None = None      # tri-state: reviewed-in / -out / unreviewed
    sources: list[str] = field(default_factory=list)
    flags: list[str] = field(default_factory=list)  # homophone|sandhi|loanword|function_word

    def to_json(self) -> str:
        # Deterministic key order + compact separators for byte-stable output.
        d = asdict(self)
        d["pos"] = sorted(set(d["pos"]))
        d["domains"] = sorted(set(d["domains"]))
        d["sources"] = sorted(set(d["sources"]))
        d["flags"] = sorted(set(d["flags"]))
        return json.dumps(d, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def validate(e: Entry, charset_ok) -> list[str]:
    """Return a list of problems (empty = valid). `charset_ok(word)->bool` is the
    language's answer-charset check."""
    problems = []
    if not e.word:
        problems.append("empty word")
    elif not charset_ok(e.word):
        problems.append(f"charset: {e.word!r}")
    for p in e.pos:
        if p not in POS_TAGS:
            problems.append(f"unknown POS {p!r}")
    if e.freq_rank is not None and e.freq_rank < 1:
        problems.append("freq_rank < 1")
    return problems


def merge(a: Entry, b: Entry) -> Entry:
    """Merge two entries for the same normalized word: union list fields, prefer
    the more-informative scalar (first non-None), keep the best (lowest) rank."""
    def best_rank(x, y):
        ranks = [r for r in (x, y) if r is not None]
        return min(ranks) if ranks else None

    return Entry(
        word=a.word,
        lang=a.lang,
        display=a.display or b.display,
        freq_rank=best_rank(a.freq_rank, b.freq_rank),
        pos=a.pos + b.pos,
        domains=a.domains + b.domains,
        pron=a.pron or b.pron,
        grade=a.grade or b.grade,
        gloss=a.gloss or b.gloss,
        etymology=a.etymology or b.etymology,
        kid_ok=a.kid_ok if a.kid_ok is not None else b.kid_ok,
        sources=a.sources + b.sources,
        flags=a.flags + b.flags,
    )
