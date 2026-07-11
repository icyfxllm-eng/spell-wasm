#!/usr/bin/env python3
"""Job A — re-tier the hard∪expert union so Expert holds the genuinely
hardest-to-SPELL words, using only self-contained signals (no external data).

Why not just count extractor features? Because the extractors have
false-negatives: they don't detect *why* `onomatopoeia`/`isthmus`/`paradigm`
are hard (rare vowel clusters, unusual letter runs). Re-tiering on feature-count
alone would DEMOTE those out of Expert — making it easier. So the score below
adds a **bigram-rarity** term computed against each language's OWN easy+medium
pool: letter patterns that never occur in common words are, by construction,
hard to spell from hearing. That term is what keeps `onomatopoeia` in Expert.

Rules that make this safe:
  * Only the hard∪expert union is touched. easy/medium (familiarity tiers) are
    never reordered.
  * Expert keeps its exact original count → the ±20% balance gate stays green.
  * Each word ends up in exactly one tier → the no-repeat invariant holds.
  * A word only moves when its score beats the boundary by a MARGIN, so ties
    and scorer noise don't churn already-good curation.

    python3 tools/difficulty-score/retier.py            # report only (no writes)
    python3 tools/difficulty-score/retier.py --apply    # rewrite hard/expert .txt
"""
from __future__ import annotations

import argparse
import math
import sys
import unicodedata
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extractors import EXTRACTORS  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent.parent
WORDS = ROOT / "assets" / "words"
LANGS = ["en", "es", "fr", "de", "pt", "it", "nl", "pl", "sv", "nb", "tr", "vi", "th", "fil", "ko", "ja"]
# Score margin (in composite-score units) a lower-tier word must EXCEED the
# displaced upper-tier word by before we swap. Higher = more conservative.
MARGIN = 0.35


def load(lang: str, tier: str) -> list[str]:
    p = WORDS / lang / f"{tier}.txt"
    if not p.exists():
        return []
    return [w.strip() for w in p.read_text(encoding="utf-8").splitlines()
            if w.strip() and not w.startswith("#")]


def bigrams(word: str) -> list[str]:
    w = f"^{word.lower()}$"
    return [w[i:i + 2] for i in range(len(w) - 1)]


def rarity_model(common: list[str]):
    """-log frequency of each bigram across the language's common words.
    Bigrams never seen in easy/medium get the max penalty."""
    c = Counter()
    for w in common:
        c.update(bigrams(w))
    total = sum(c.values()) or 1
    max_pen = math.log(total + 1)
    def rar(word: str) -> float:
        bs = bigrams(word)
        if not bs:
            return 0.0
        return sum(math.log(total / (c[b] + 0.5)) for b in bs) / len(bs) / max_pen
    return rar


def composite(word: str, lang: str, rar) -> float:
    ext = EXTRACTORS.get(lang)
    feats = ext(word) if ext else []
    # Strip a leading "pinyin|hanzi" style guard just in case (file langs don't).
    plain = word.split("|")[-1]
    vowels = "aeiouyáàâãäéèêëíìîïóòôõöúùûüåæøœ"
    # longest run of consecutive vowels (rare clusters like 'oeia' → hard)
    run = best = 0
    for ch in plain.lower():
        run = run + 1 if ch in vowels else 0
        best = max(best, run)
    vowel_cluster = max(0, best - 2)  # 0,1,2 vowels normal; 3+ counts
    length_norm = min(len(plain), 16) / 16.0
    return (3.0 * len(feats)       # extractor-detected features (strong)
            + 2.5 * rar(plain)     # bigram rarity vs common words (false-neg killer)
            + 1.2 * vowel_cluster  # unusual vowel runs
            + 0.5 * length_norm)   # mild length tiebreak


def retier(lang: str) -> dict:
    easy, medium = load(lang, "easy"), load(lang, "medium")
    hard, expert = load(lang, "hard"), load(lang, "expert")
    if not hard or not expert:
        return {"lang": lang, "skipped": "missing hard/expert"}
    rar = rarity_model(easy + medium)
    union = [(w, "hard") for w in hard] + [(w, "expert") for w in expert]
    scored = sorted(((composite(w, lang, rar), w, src) for w, src in union),
                    key=lambda t: t[0], reverse=True)
    n_expert = len(expert)
    # Provisional new-expert = top n_expert by score. But only ACT on a word
    # crossing the boundary if it beats the boundary score by MARGIN, so we
    # don't churn near-ties.
    boundary = scored[n_expert - 1][0] if n_expert <= len(scored) else 0.0
    new_expert, new_hard = [], []
    for i, (s, w, src) in enumerate(scored):
        want_expert = i < n_expert
        if src == "expert" and not want_expert:
            # would be demoted; keep in expert unless clearly below boundary
            if boundary - s > MARGIN:
                new_hard.append(w)
            else:
                new_expert.append(w)
        elif src == "hard" and want_expert:
            if s - boundary > MARGIN:
                new_expert.append(w)
            else:
                new_hard.append(w)
        else:
            (new_expert if want_expert else new_hard).append(w)
    # Enforce exact counts (margin rules can drift by a couple); rebalance by
    # moving boundary-nearest words, never dropping any.
    def rebalance():
        while len(new_expert) > n_expert:
            # move lowest-scoring expert word down
            w = min(new_expert, key=lambda x: composite(x, lang, rar))
            new_expert.remove(w); new_hard.append(w)
        while len(new_expert) < n_expert:
            w = max(new_hard, key=lambda x: composite(x, lang, rar))
            new_hard.remove(w); new_expert.append(w)
    rebalance()
    up = sorted(set(new_expert) - set(expert))     # hard → expert
    down = sorted(set(expert) - set(new_expert))   # expert → hard
    return {"lang": lang, "n_expert": n_expert, "up": up, "down": down,
            "new_expert": sorted(new_expert), "new_hard": sorted(new_hard)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()

    out = ROOT / "tools" / "difficulty-score" / "out"
    out.mkdir(exist_ok=True)
    md = ["# Job A — Expert re-tier diff (all file-based languages)", "",
          "Swaps within the hard∪expert union only. Expert counts unchanged →",
          "balance + no-repeat invariants preserved. `↑` promoted into Expert",
          "(was hard, is genuinely harder to spell); `↓` demoted out of Expert",
          "(was Expert but easier to spell than the promoted word).", ""]
    total_moves = 0
    for lang in LANGS:
        r = retier(lang)
        if "skipped" in r:
            md.append(f"## {lang} — {r['skipped']}\n")
            continue
        moves = len(r["up"])
        total_moves += moves
        status = "no change (already well-tiered)" if not moves else f"{moves} swap(s)"
        md.append(f"## {lang} — {status}")
        if moves:
            for u in r["up"]:
                md.append(f"- ↑ `{u}`  (hard → **expert**)")
            for d in r["down"]:
                md.append(f"- ↓ `{d}`  (expert → hard)")
        md.append("")
        print(f"[{lang}] {status}")
        if args.apply and moves:
            (WORDS / lang / "expert.txt").write_text(
                "\n".join(r["new_expert"]) + "\n", encoding="utf-8")
            (WORDS / lang / "hard.txt").write_text(
                "\n".join(r["new_hard"]) + "\n", encoding="utf-8")
    md.insert(5, f"**Total promotions across all languages: {total_moves}**\n")
    (out / "retier-diff.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print(f"\n{'APPLIED' if args.apply else 'REPORT ONLY'} — {total_moves} promotions; "
          f"diff → tools/difficulty-score/out/retier-diff.md")


if __name__ == "__main__":
    main()
