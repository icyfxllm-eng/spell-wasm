#!/usr/bin/env python3
"""Does the word bank hold inflected forms, or lemmas only?

The measurement behind docs/CC-SNAP-LEMMA-census.md. Counts bank pairs
(X, X+suffix) where BOTH are bank words.

This is NOT morphological analysis and must never be used as an inflection
table. It is a mechanical fact about the files -- either both strings are in
the bank or they are not -- which is why it needs no linguist to check. The
per-language suffix lists below are adequate to prove that inflected forms
are PRESENT; they are not a description of how any language works. CC-SNAP
2.3's tables come from the dictionary authority list, never from here.

    python3 tools/lemma_census.py [--lang en]
"""
import argparse
import collections
import pathlib

BANKS = pathlib.Path(__file__).resolve().parent.parent / "assets" / "words"
TIERS = ("easy", "medium", "hard", "expert")

# Orthographic inflectional suffixes. zh/ja/vi are deliberately empty: they do
# not inflect by concatenating one, and their zero is this method's own
# validity check -- it should find nothing where there is nothing to find.
SUFFIXES = {
    "en": ["s", "es", "ed", "ing", "er", "est", "ly", "d"],
    "es": ["s", "es", "a", "as", "os", "ando", "iendo", "ado", "ida", "idos"],
    "fr": ["s", "es", "e", "ent", "ait", "aient"],
    "pt": ["s", "es", "a", "as", "os", "ando", "ado", "ida"],
    "de": ["e", "en", "er", "es", "n", "s", "te", "ten"],
    "pl": ["a", "e", "y", "i", "u", "ach", "om", "ami", "ow", "ów"],
    "ru": ["а", "у", "ом", "е", "ы", "и",
           "ов", "ам", "ами",
           "ах", "ой", "ю", "я"],
    "ar": ["ة", "ات", "ين", "ون", "ها"],
    "hi": ["ों", "ें", "ा", "ी", "े"],
    "ko": ["들", "이", "가", "을", "를", "은", "는"],
    "fil": ["ng", "in", "an", "han"],
    "sw": ["ni", "wa", "ki", "vi"],
    "fa": ["ها", "ان", "ی"],
    "vi": [], "zh": [], "ja": [],
}
# Suffixes that almost never coincide, so a pair is near-certainly a real
# inflection. car+d=card and care+er=career are why the others are not here.
HIGH_CONFIDENCE = {"en": {"ing", "ed", "y->ies"}}


def bank(lang):
    """Every bank word for a language, deduped across tiers, as the app reads them."""
    seen = set()
    for tier in TIERS:
        path = BANKS / lang / f"{tier}.txt"
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            word = line.strip().lower()
            if word and not word.startswith("#"):
                seen.add(word)
    return seen


def pairs(lang, words):
    out = []
    for word in sorted(words):
        for suffix in SUFFIXES.get(lang, []):
            if suffix and word.endswith(suffix) and len(word) > len(suffix) + 1:
                stem = word[: -len(suffix)]
                if stem in words:
                    out.append((stem, word, suffix))
                    break
        else:
            if lang == "en" and word.endswith("ies") and (word[:-3] + "y") in words:
                out.append((word[:-3] + "y", word, "y->ies"))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", help="one language instead of all")
    args = ap.parse_args()
    langs = [args.lang] if args.lang else sorted(SUFFIXES)

    print(f"{'lang':5} {'bank':>6} {'pairs':>7} {'%':>7}   examples")
    for lang in langs:
        words = bank(lang)
        if not words:
            continue
        found = pairs(lang, words)
        pct = len(found) / len(words) * 100
        ex = "  ".join(f"{a}->{b}" for a, b, _ in found[:3]) or "(none)"
        print(f"{lang:5} {len(words):>6} {len(found):>7} {pct:>6.1f}%   {ex}")

        if lang in HIGH_CONFIDENCE:
            by = collections.Counter(s for _, _, s in found)
            print("      by suffix: " + ", ".join(f"{s} {n}" for s, n in by.most_common()))
            hi = [p for p in found if p[2] in HIGH_CONFIDENCE[lang]]
            print(f"      low-coincidence subset ({'/'.join(sorted(HIGH_CONFIDENCE[lang]))}): "
                  f"{len(hi)} ({len(hi)/len(words)*100:.1f}% of bank)")
            fam = collections.defaultdict(set)
            for a, b, _ in found:
                fam[a].add(b)
            multi = {k: v for k, v in fam.items() if len(v) >= 2}
            print(f"      lemmas with 2+ inflections also in the bank: {len(multi)}")


if __name__ == "__main__":
    main()
