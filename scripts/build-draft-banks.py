#!/usr/bin/env python3
"""Generate assets/words-draft/<lang>/*.txt from Leipzig (CC BY) — AUDIT drafts.

Rebuilds the ar/fa/ur/ru audit drafts from Leipzig Wikipedia corpora (CC BY 4.0),
REPLACING the earlier OpenSubtitles CC BY-SA drafts. Two wins:
  * Licence — CC BY is attribution-only, so shipping the reviewed result needs no
    §4 copyleft decision; the share-alike question simply stops applying.
  * Content — Wikipedia is more neutral than subtitles, which carry proper nouns
    and adult vocabulary (the old Hindi draft surfaced "kidnapping"/"hatred").

Still AUDIT-ONLY. This only makes the drafts cleaner and correctly-licensed; a
native still judges naturalness, register, and kid-appropriateness (that review is
the real gate — see assets/words-draft/README.md). These never reach production
until promoted + the RTL flip.

Per language: fetch the corpus, apply the script normalization fold, keep only
keyboard-typeable words (the HARD guarantee — this gate is what rejected ~17k
Persian words mis-encoded with Arabic yeh), NFC, length-tier by letter count.

Run this, then scripts/build-draft-wordbanks.py to re-emit src/word_data_audit.rs.

Usage:  python3 scripts/build-draft-banks.py [--target N]   (default 200 per tier)
"""
import os, sys, unicodedata, importlib.util

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DRAFT = os.path.join(ROOT, "assets", "words-draft")
TIERS = ["easy", "medium", "hard", "expert"]
# Letter-count tiers (combining marks don't count). Arabic-script words run short,
# so easy reaches to 4 to stay populated; expert is the long tail.
TIER_LEN = {"easy": (2, 4), "medium": (5, 6), "hard": (7, 8), "expert": (9, 14)}
SOURCES = {
    "ar": "ara_wikipedia_2021_100K",
    "fa": "pes_wikipedia_2021_100K",   # pes = Iranian Persian (fa-IR per D3), not Dari
    "ur": "urd_wikipedia_2021_100K",
    "ru": "rus_wikipedia_2021_100K",
}

# Reuse the corpus fetch/cache and the production charset gate.
_spec = importlib.util.spec_from_file_location("bb", os.path.join(ROOT, "scripts", "build-bigbank.py"))
_bb = importlib.util.module_from_spec(_spec)
try:
    _spec.loader.exec_module(_bb)
except SystemExit:
    pass
corpus_words = _bb.corpus_words
reachable_chars = _bb.reachable_chars

# Arabic diacritics an unvocalized keyboard can't type: tatweel (kashida, a pure
# elongation glyph), the harakat/tanwin/shadda/sukun range, and superscript alef.
# Stripped so a vocalized corpus spelling collapses to the bare orthography the
# keyboard produces. The charset gate is still the backstop for anything missed.
STRIP = {0x0640, 0x0670} | set(range(0x064B, 0x0653))


def fold(lang, w):
    w = unicodedata.normalize("NFC", w)
    w = "".join(c for c in w if ord(c) not in STRIP)
    if lang in ("fa", "ur"):
        # The same letter, different codepoints: Persian/Urdu keyboards produce
        # U+06CC / U+06A9, the corpus often carries the Arabic U+064A / U+0643.
        w = w.replace("ي", "ی").replace("ك", "ک")
    return unicodedata.normalize("NFC", w)


def letters(w):
    return sum(1 for c in w if unicodedata.category(c) not in ("Mn", "Mc"))


def build(lang, target):
    reach = reachable_chars(lang)
    banks = {t: [] for t in TIERS}
    seen = set()
    for w0, _ in corpus_words(SOURCES[lang]):
        w = fold(lang, w0)
        if not w or w in seen or any(c not in reach for c in w):
            continue
        n = letters(w)
        tier = next((t for t, (lo, hi) in TIER_LEN.items() if lo <= n <= hi), None)
        if not tier or len(banks[tier]) >= target:
            continue
        banks[tier].append(w)
        seen.add(w)
    outdir = os.path.join(DRAFT, lang)
    os.makedirs(outdir, exist_ok=True)
    for tier in TIERS:
        open(os.path.join(outdir, f"{tier}.txt"), "w", encoding="utf-8").write("\n".join(banks[tier]) + "\n")
    return {t: len(banks[t]) for t in TIERS}


def main():
    target = 200
    if "--target" in sys.argv:
        target = int(sys.argv[sys.argv.index("--target") + 1])
    print(f"  rebuilding ar/fa/ur/ru drafts from Leipzig (Wikipedia, CC BY), {target}/tier:")
    for lang in SOURCES:
        c = build(lang, target)
        print(f"    {lang}  {c['easy']}/{c['medium']}/{c['hard']}/{c['expert']}")


if __name__ == "__main__":
    main()
