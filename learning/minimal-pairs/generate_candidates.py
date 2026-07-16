#!/usr/bin/env python3
"""
Minimal-pair CANDIDATE generator for CC-LEARNING-MODES feature 3.

Pure-text, deterministic. For each language with a word list, finds every pair of
words in the SAME difficulty tier whose GRAPHEME-CLUSTER edit distance == 1
(NFC-normalized). A grapheme is a user-perceived character (UAX #29 extended
grapheme cluster) -- NOT a code point -- so Thai stacked vowels/tone marks,
Korean syllable blocks, and Vietnamese precomposed tone+diacritic letters each
count as a single unit.

THESE ARE PRE-GATE CANDIDATES. The real feature applies a whisper.cpp loopback
distinctness gate at build time (synthesize + transcribe both words, drop the
pair if whisper confuses them). That gate needs the TTS + whisper infra which is
NOT available here. This script produces the SUPERSET that the whisper gate (a
separate CI/device step) will later prune. It invents no whisper results.

Deps: `grapheme` (pip) -- the only added dependency, a UAX#29 segmenter.
Output: learning/minimal-pairs/<lang>.candidates.tsv
        columns: word1 <TAB> word2 <TAB> tier <TAB> diff (op@1-based-index:detail)
"""
import os
import sys
import unicodedata

import grapheme

LANGS = ["en", "es", "fr", "de", "pt", "it", "nl", "pl",
         "sv", "nb", "tr", "th", "fil", "vi", "ko", "ja"]
TIERS = ["easy", "medium", "hard", "expert"]

# Per-tier safety valve (brief: "if a tier explodes ... cap ... never silently
# truncate ... LOG what was capped"). Set ABOVE the current largest tier
# (ko/easy = 362) so it does NOT fire on today's audited lists -- see README for
# why we deliberately retain the full Korean single-syllable superset rather than
# discard genuine-but-unrankable pairs. If a future list exceeds this, the
# machinery keeps the most-useful (most shared context, then shortest) and logs
# exactly how many were dropped.
PER_TIER_CAP = 1000

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
WORDS_DIR = os.path.join(REPO_ROOT, "assets", "words")
OUT_DIR = os.path.dirname(os.path.abspath(__file__))


def clusters(word):
    """NFC-normalize then split into extended grapheme clusters."""
    return list(grapheme.graphemes(unicodedata.normalize("NFC", word)))


def diff_at_distance_one(a, b):
    """
    a, b are grapheme-cluster lists. Return (op, index, detail, shared) describing
    the single edit that turns a into b, or None if their grapheme edit distance
    != 1. index is 0-based into the relevant word; detail is human-readable;
    `shared` = count of grapheme clusters common to both (a usefulness proxy: more
    shared context = tighter, more genuinely-confusable minimal pair).
    """
    la, lb = len(a), len(b)
    if abs(la - lb) > 1:
        return None

    if la == lb:
        # Same length -> must be exactly one substitution.
        mismatches = [i for i in range(la) if a[i] != b[i]]
        if len(mismatches) != 1:
            return None
        i = mismatches[0]
        return ("sub", i, f"{a[i]}→{b[i]}", la - 1)

    # Length differs by exactly 1 -> must be a single insert/delete.
    lo, hi = (a, b) if la < lb else (b, a)  # lo is shorter
    # longest common prefix
    p = 0
    while p < len(lo) and lo[p] == hi[p]:
        p += 1
    # longest common suffix (not overlapping the prefix)
    s = 0
    while s < (len(lo) - p) and lo[len(lo) - 1 - s] == hi[len(hi) - 1 - s]:
        s += 1
    if p + s != len(lo):
        return None  # more than one edit
    extra = hi[p]  # the single grapheme present only in the longer word
    shared = len(lo)  # every grapheme of the shorter word survives the indel
    # Describe relative to a->b. If b is longer, it's an insertion into a; else deletion.
    if lb > la:
        return ("ins", p, f"+{extra}", shared)
    return ("del", p, f"-{extra}", shared)


def load_words(path):
    """One word per line; NFC; dedupe preserving nothing but sorted determinism."""
    seen = set()
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            w = line.strip()
            if not w or w.startswith("#"):
                continue
            seen.add(unicodedata.normalize("NFC", w))
    return sorted(seen)


def main():
    grand_total = 0
    summary = []  # (lang, {tier: count}, total, [cap notes])
    for lang in LANGS:
        lang_dir = os.path.join(WORDS_DIR, lang)
        rows = []
        per_tier = {}
        cap_notes = []
        for tier in TIERS:
            path = os.path.join(lang_dir, f"{tier}.txt")
            if not os.path.isfile(path):
                per_tier[tier] = 0
                continue
            words = load_words(path)
            # precompute clusters once
            gc = {w: clusters(w) for w in words}
            pairs = []
            for i in range(len(words)):
                for j in range(i + 1, len(words)):
                    w1, w2 = words[i], words[j]
                    d = diff_at_distance_one(gc[w1], gc[w2])
                    if d is None:
                        continue
                    op, idx, detail, shared = d
                    diff_str = f"{op}@{idx + 1}:{detail}"
                    # usefulness rank: MORE shared context first (tighter minimal
                    # pair), then shorter words, then lexicographic. Deterministic.
                    key = (-shared, len(gc[w1]) + len(gc[w2]), w1, w2)
                    pairs.append((key, w1, w2, tier, diff_str))
            pairs.sort(key=lambda t: t[0])
            if len(pairs) > PER_TIER_CAP:
                dropped = len(pairs) - PER_TIER_CAP
                cap_notes.append(
                    f"{tier}: {len(pairs)} pairs -> capped to {PER_TIER_CAP} "
                    f"(dropped {dropped} least-shared-context pairs)")
                pairs = pairs[:PER_TIER_CAP]
            per_tier[tier] = len(pairs)
            rows.extend((key, w1, w2, t, ds) for key, w1, w2, t, ds in pairs)

        # deterministic final ordering: tier order, then usefulness key
        # (most shared context first), so auditors see the tightest pairs on top.
        tier_rank = {t: i for i, t in enumerate(TIERS)}
        rows = [(w1, w2, t, ds) for _k, w1, w2, t, ds in
                sorted(rows, key=lambda r: (tier_rank[r[3]], r[0]))]

        out_path = os.path.join(OUT_DIR, f"{lang}.candidates.tsv")
        with open(out_path, "w", encoding="utf-8") as fh:
            fh.write("word1\tword2\ttier\tdiff\n")
            for w1, w2, tier, ds in rows:
                fh.write(f"{w1}\t{w2}\t{tier}\t{ds}\n")
        total = len(rows)
        grand_total += total
        summary.append((lang, per_tier, total, cap_notes))

    # emit a machine-readable summary to stdout
    print("lang\teasy\tmedium\thard\texpert\ttotal\tcaps")
    for lang, per_tier, total, cap_notes in summary:
        caps = "; ".join(cap_notes) if cap_notes else "-"
        print(f"{lang}\t{per_tier.get('easy',0)}\t{per_tier.get('medium',0)}\t"
              f"{per_tier.get('hard',0)}\t{per_tier.get('expert',0)}\t{total}\t{caps}")
    print(f"# GRAND TOTAL candidate pairs (all langs): {grand_total}")
    print("# zh: SKIPPED (no word list -- assets/words/zh does not exist)")


if __name__ == "__main__":
    main()
