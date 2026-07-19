#!/usr/bin/env python3
"""Grow assets/words/<lang>/<tier>.txt from the Leipzig Corpora Collection.

Leipzig (wortschatz-leipzig.de) is monolingual, frequency-ranked, CC BY 4.0, and
covers every language uniformly — the clean, scalable source for large diverse
banks. This AUGMENTS the hand-curated seed words (keeps the concrete "sol luna
casa" core) with corpus words for volume, tiered by spelling difficulty (length).

Every candidate is filtered through the SAME gates production uses:
  * typeable on the language's keyboard (reachable_chars, reused from
    build-wordlists.py — so the ko-syllable and vi-tone expansions match),
  * NFC, length-bounded, deduped, and — for cased scripts — proper-noun-filtered
    (a Title-case word never seen lowercased is dropped).

The corpora are LARGE (~20 MB) build inputs, cached under .corpus-cache/ and NOT
committed (per NOTICES.md: raw datasets are build inputs, not redistributed). Only
the emitted, filtered word lists ship. Re-run build-wordlists.py afterwards to
regenerate word_data.rs and re-check every gate.

Usage:  python3 scripts/build-bigbank.py [--target N]   (default 200 per tier)
"""
import os, sys, json, tarfile, unicodedata, urllib.request, importlib.util

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, ".corpus-cache")
TIERS = ["easy", "medium", "hard", "expert"]
# (min,max) letters per tier — length is the spelling-difficulty proxy.
TIER_LEN = {"easy": (2, 4), "medium": (5, 6), "hard": (7, 8), "expert": (9, 15)}

# Verified Leipzig files (Wikipedia = diverse/neutral vocab). Corpus is a build
# input; the year is pinned here for reproducibility.
SOURCES = {
    "en": "eng_wikipedia_2016_100K", "es": "spa_wikipedia_2021_100K",
    "fr": "fra_wikipedia_2021_100K", "de": "deu_wikipedia_2021_100K",
    "pt": "por_wikipedia_2021_100K", "pl": "pol_wikipedia_2021_100K",
    "vi": "vie_wikipedia_2021_100K", "ko": "kor_wikipedia_2021_100K",
    "fil": "tgl_wikipedia_2021_100K", "ja": "jpn_wikipedia_2021_100K",
}
# Languages we lowercase + apply the case-based proper-noun filter to. German is
# EXCLUDED: it capitalises every common noun (Haus, Baum), so case can't tell a
# common noun from a proper one, and lowercasing would misspell them. German keeps
# its corpus case and takes the residual proper nouns (like the caseless scripts).
CASED = {"en", "es", "fr", "pt", "pl", "vi", "fil"}
PRESERVE_CASE = {"de"}  # keep as-is, no lowercase, no case-based PN filter

# Reuse the production charset gate (identical ko/vi handling).
_spec = importlib.util.spec_from_file_location("bw", os.path.join(ROOT, "scripts", "build-wordlists.py"))
_bw = importlib.util.module_from_spec(_spec)
try:
    _spec.loader.exec_module(_bw)
except SystemExit:
    pass
reachable_chars = _bw.reachable_chars


def corpus_words(name):
    """Download+cache a Leipzig archive; yield (word, freq) in frequency order."""
    os.makedirs(CACHE, exist_ok=True)
    tgz = os.path.join(CACHE, name + ".tar.gz")
    if not os.path.exists(tgz):
        url = f"https://downloads.wortschatz-leipzig.de/corpora/{name}.tar.gz"
        print(f"    fetching {name} …", flush=True)
        urllib.request.urlretrieve(url, tgz)
    with tarfile.open(tgz) as t:
        member = next(m for m in t.getmembers() if m.name.endswith("-words.txt"))
        rows = []
        for line in t.extractfile(member).read().decode("utf-8").splitlines():
            p = line.split("\t")
            if len(p) >= 3:
                try:
                    rows.append((p[1], int(p[2])))
                except ValueError:
                    pass
    rows.sort(key=lambda x: -x[1])
    return rows


def strict_fold(s):
    return unicodedata.normalize("NFC", s).lower()


def build(lang, target):
    reach = reachable_chars(lang)
    rows = corpus_words(SOURCES[lang])
    # Genuine lowercase entries: a word the corpus actually writes lowercased. A
    # Title-case word whose lowercase is NOT one of these is only ever capitalised
    # — i.e. a proper noun (London), so it's dropped.
    lower_seen = {w for w, _ in rows if w and not w[0].isupper()} if lang in CASED else set()

    # Keep the curated seed words in their existing tiers (they are good, concrete).
    seed = {}
    seen = set()
    for tier in TIERS:
        path = os.path.join(ROOT, "assets", "words", lang, f"{tier}.txt")
        seed[tier] = []
        if os.path.exists(path):
            for w in open(path, encoding="utf-8"):
                w = w.strip()
                if w and not w.startswith("#"):
                    seed[tier].append(w)
                    seen.add(strict_fold(w))

    # Augment each tier with corpus words of the right length, in frequency order.
    banks = {t: list(seed[t]) for t in TIERS}
    for w0, _ in rows:
        if lang in CASED:
            if w0[:1].isupper() and w0.lower() not in lower_seen:
                continue  # proper noun: only ever seen capitalised
            w = unicodedata.normalize("NFC", w0.lower())
        else:
            w = unicodedata.normalize("NFC", w0)  # de + caseless scripts keep case
        key = strict_fold(w)
        if not w or key in seen or any(c not in reach for c in w):
            continue
        n = sum(1 for c in w if unicodedata.category(c) not in ("Mn", "Mc"))
        tier = next((t for t, (lo, hi) in TIER_LEN.items() if lo <= n <= hi), None)
        if not tier or len(banks[tier]) >= target:
            continue
        banks[tier].append(w)
        seen.add(key)

    outdir = os.path.join(ROOT, "assets", "words", lang)
    os.makedirs(outdir, exist_ok=True)
    for tier in TIERS:
        open(os.path.join(outdir, f"{tier}.txt"), "w", encoding="utf-8").write("\n".join(banks[tier]) + "\n")
    return {t: len(banks[t]) for t in TIERS}, {t: len(seed[t]) for t in TIERS}


def main():
    target = 200
    if "--target" in sys.argv:
        target = int(sys.argv[sys.argv.index("--target") + 1])
    print(f"  growing to {target}/tier from Leipzig (Wikipedia, CC BY):")
    for lang in SOURCES:
        counts, seed = build(lang, target)
        c = counts
        print(f"    {lang:4} {c['easy']}/{c['medium']}/{c['hard']}/{c['expert']}  "
              f"(seed kept: {sum(seed.values())})")


if __name__ == "__main__":
    main()
