#!/usr/bin/env python3
"""Can a canonical string serve as DOC-1's bank anchor?

The measurement behind docs/CC-SNAP-DOC1-census.md.

word_index::canonical() returns the bank's own spelling for a word, and the
tempting move is to call that string the anchor. This checks whether it can
IDENTIFY a bank row, by mirroring the real fold and the real index key:

    comparable(entry) = fold_strict(entry.split('|')[0])
    fold_strict(s)    = NFC, lowercase, minus whitespace, minus silent marks

A collision means two distinct bank rows share one index key, so canonical()
can return either one and the string names neither.

    python3 tools/doc1_census.py
"""
import collections
import pathlib
import unicodedata

BANKS = pathlib.Path(__file__).resolve().parent.parent / "assets" / "words"
TIERS = ("easy", "medium", "hard", "expert")


def is_silent_mark(c):
    """src/norm.rs is_silent_mark: tashkeel, dagger alef, tatweel, ZWNJ."""
    o = ord(c)
    return (0x064B <= o <= 0x065F) or o in (0x0670, 0x0640, 0x200C)


def fold_strict(s):
    s = unicodedata.normalize("NFC", s).lower()
    return "".join(c for c in s if not c.isspace() and not is_silent_mark(c))


def comparable(entry):
    """word_index::comparable -- zh is keyed on the reading, left of the pipe."""
    return fold_strict(entry.split("|")[0])


def rows(lang_dir):
    out = []
    for tier in TIERS:
        p = lang_dir / f"{tier}.txt"
        if p.exists():
            out += [l.strip() for l in p.read_text(encoding="utf-8").splitlines()
                    if l.strip() and not l.startswith("#")]
    return sorted(set(out))


def main():
    total = 0
    print(f"{'lang':6} {'rows':>7} {'collide':>8} {'unreachable':>12} {'%':>6}")
    for d in sorted(p for p in BANKS.iterdir() if p.is_dir()):
        r = rows(d)
        if not r:
            continue
        total += len(r)
        by = collections.defaultdict(set)
        for w in r:
            by[comparable(w)].add(w)
        coll = {k: v for k, v in by.items() if len(v) > 1}
        lost = sum(len(v) - 1 for v in coll.values())
        if coll:
            print(f"{d.name:6} {len(r):>7} {len(coll):>8} {lost:>12} {lost/len(r)*100:>5.1f}%")
            for k, v in list(coll.items())[:3]:
                print(f"       e.g. {sorted(v)} -> {k!r}")
        else:
            print(f"{d.name:6} {len(r):>7} {0:>8} {0:>12} {0.0:>5.1f}%")
    print(f"\n{total:,} distinct bank entries examined.")
    print("A nonzero 'unreachable' count is the number of rows canonical() can")
    print("never name, because another row answers their key first.")


if __name__ == "__main__":
    main()
