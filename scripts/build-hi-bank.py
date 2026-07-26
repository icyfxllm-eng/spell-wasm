#!/usr/bin/env python3
"""Build the PRODUCTION Hindi bank — assets/words/hi/{tier}.txt, 800/tier.

Eric's ruling (2026-07-25): build Hindi as the 15th language. Same recipe as
the ar/ru production banks: Leipzig Corpora Collection (CC BY 4.0 —
attribution in NOTICES.md), Wikipedia corpus (hin_wikipedia_2021_100K — the
old draft's NEWS corpus surfaced junk like "kidnapping"; Wikipedia is the
neutral source the ar/ru rebuild moved to), frequency-ranked, keyboard-charset
gated (assets/keyboards/hi.json), NFC, length-tiered by BASE letters
(combining matras don't count).

Words carrying ZWJ/ZWNJ or Devanagari digits fail the keyboard-reachability
gate and drop out — a spelling word must be typeable keystroke-for-keystroke.

Run, then add "hi" to build-wordlists.py LANGS and re-run it:
    python3 scripts/build-hi-bank.py
    python3 scripts/build-wordlists.py
"""
import os, sys, unicodedata, importlib.util

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "assets", "words", "hi")
TIERS = ["easy", "medium", "hard", "expert"]
# Base-letter tiers (Mn/Mc combining marks don't count). Devanagari words run
# compact — a 2-consonant word like "घर" is a legitimate easy word.
TIER_LEN = {"easy": (2, 3), "medium": (4, 5), "hard": (6, 7), "expert": (8, 14)}
SOURCE = "hin_wikipedia_2021_100K"
TARGET = 800

_spec = importlib.util.spec_from_file_location("bb", os.path.join(ROOT, "scripts", "build-bigbank.py"))
_bb = importlib.util.module_from_spec(_spec)
try:
    _spec.loader.exec_module(_bb)
except SystemExit:
    pass
corpus_words = _bb.corpus_words
reachable_chars = _bb.reachable_chars

_wspec = importlib.util.spec_from_file_location("bw2", os.path.join(ROOT, "scripts", "build-wordlists.py"))
_bw = importlib.util.module_from_spec(_wspec)
try:
    _wspec.loader.exec_module(_bw)
except SystemExit:
    pass
load_exclusions = _bw.load_exclusions
is_excluded = _bw.is_excluded


def letters(w):
    return sum(1 for c in w if unicodedata.category(c) not in ("Mn", "Mc"))


def main():
    reach = reachable_chars("hi")
    roots, exact = load_exclusions("hi")
    banks = {t: [] for t in TIERS}
    seen = set()
    for w0, _ in corpus_words(SOURCE):
        w = unicodedata.normalize("NFC", w0)
        # Devanagari only — reject mixed-script tokens (Latin loans, digits).
        if not w or w in seen:
            continue
        if any(c not in reach for c in w):
            continue
        if is_excluded(w, roots, exact):
            continue
        # build-wordlists' production gate: 2..16 codepoints.
        if not (2 <= len(w) <= 16):
            continue
        n = letters(w)
        tier = next((t for t, (lo, hi) in TIER_LEN.items() if lo <= n <= hi), None)
        if not tier or len(banks[tier]) >= TARGET:
            continue
        banks[tier].append(w)
        seen.add(w)
        if all(len(banks[t]) >= TARGET for t in TIERS):
            break
    os.makedirs(OUT, exist_ok=True)
    for tier in TIERS:
        with open(os.path.join(OUT, f"{tier}.txt"), "w", encoding="utf-8") as f:
            f.write("\n".join(banks[tier]) + "\n")
    counts = {t: len(banks[t]) for t in TIERS}
    print(f"build-hi-bank: {counts} -> {os.path.relpath(OUT, ROOT)}")
    if any(c < TARGET for c in counts.values()):
        print("build-hi-bank: WARNING — a tier is under target; check TIER_LEN bounds")


if __name__ == "__main__":
    main()
