"""CC-ORTHO-SWEEP-DE-JA T2: no active German row is a struck or pre-reform spelling.

Active = in assets/words/de/*.txt. Quarantined rows live in assets/words-quarantine/de and are not read.
Run: python3 -m pytest tests/ortho/test_de_struck.py
"""
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TIERS = ("easy", "medium", "hard", "expert")


def rows():
    for t in TIERS:
        for line in open(os.path.join(ROOT, "assets/words/de", t + ".txt"), encoding="utf-8"):
            w = line.split("\t")[0].strip()
            if w and not w.startswith("#"):
                yield t, w


def struck():
    out = set()
    for line in open(os.path.join(ROOT, "data/de/struck-variants.txt"), encoding="utf-8"):
        if line.strip() and not line.startswith("#"):
            out.add(line.split("\t")[0].strip().lower())
    return out


# ß after a short vowel where the 1996 reform requires ss (Fluß, muß, daß), and the pre-reform two-letter triple
# consonants (Schiffahrt, Schnellauf is fine). The short-vowel test is the Hunspell-checked census; this is the guard.
PRE_REFORM = re.compile(r"^(daß|muß|laß|naß|miß\w*|Fluß|Haß|Schluß|Kuß|Schiffahrt|Balettheater|Brennessel)$", re.I)


def test_no_struck_form_is_active():
    s = struck()
    hits = [(t, w) for t, w in rows() if w.lower() in s]
    assert not hits, hits


def test_no_pre_reform_pattern_is_active():
    hits = [(t, w) for t, w in rows() if PRE_REFORM.match(w) or "daß" in w.lower()]
    assert not hits, hits


def test_struck_file_has_a_source_header():
    head = open(os.path.join(ROOT, "data/de/struck-variants.txt"), encoding="utf-8").read().splitlines()[:3]
    assert any("Source" in l for l in head)


def test_every_struck_row_cites_a_page():
    for line in open(os.path.join(ROOT, "data/de/struck-variants.txt"), encoding="utf-8"):
        if line.strip() and not line.startswith("#"):
            assert re.search(r"\tp\. \d+$", line.rstrip("\n")), line
