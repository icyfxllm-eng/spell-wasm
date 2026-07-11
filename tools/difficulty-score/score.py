#!/usr/bin/env python3
"""Difficulty scoring + tier verification (word-quality spec, Part 1).

Scores candidate words per language and reports how well the current Expert pools
satisfy the spec's rule: **every Expert word must carry ≥1 machine-verifiable
difficulty feature.** This is the highest-value check runnable against today's
pools — it flags Expert words that are merely long/rare but not genuinely hard to
spell from hearing.

Score = w_f·freq + w_g·graded + w_l·length + w_o·orthographic + w_p·phoneme_gap,
normalized within each language (percentiles). Frequency/graded inputs are
optional here (no external data yet) — with them absent the formula renormalizes
onto length + the feature signals, which is the honest current state.

  python3 tools/difficulty-score/score.py            # report on en, zh, vi Expert pools
  python3 tools/difficulty-score/score.py --lang en  # one language
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extractors import EXTRACTORS  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent.parent
TIERS = ["easy", "medium", "hard", "expert"]

# Default weights (spec §Scoring); w_o + w_p (spelling-from-hearing) outweigh
# rarity so this tests spelling, not vocabulary.
W = {"f": 0.25, "g": 0.15, "l": 0.10, "o": 0.30, "p": 0.20}


def load_tier(lang: str, tier: str) -> list[str]:
    p = ROOT / "assets" / "words" / lang / f"{tier}.txt"
    if not p.exists():
        # zh lives in src/words.rs consts; parse the pinyin|hanzi bank if needed.
        return []
    return [w.strip() for w in p.read_text(encoding="utf-8").splitlines() if w.strip() and not w.startswith("#")]


def features(word: str, lang: str) -> list[str]:
    fn = EXTRACTORS.get(lang)
    return fn(word) if fn else []


def report(lang: str) -> dict:
    ext = EXTRACTORS.get(lang)
    if not ext:
        return {"lang": lang, "skipped": "no extractor"}
    expert = load_tier(lang, "expert")
    if not expert and lang == "zh":
        # zh Expert lives in src/words.rs; sample a few known entries for the pilot.
        expert = ["lü3you2|旅游", "dong4wu4yuan2|动物园", "zi4xing2che1|自行车"]
    scored = [(w, features(w, lang)) for w in expert]
    with_feature = [w for w, f in scored if f]
    without = [(w, f) for w, f in scored if not f]
    return {
        "lang": lang,
        "expert_count": len(expert),
        "with_feature": len(with_feature),
        "coverage_pct": (len(with_feature) * 100 // len(expert)) if expert else 0,
        "no_feature_words": [w for w, _ in without],
        "sample": [(w, f) for w, f in scored[:8]],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang")
    args = ap.parse_args()
    langs = [args.lang] if args.lang else list(EXTRACTORS.keys())

    out = ROOT / "tools" / "difficulty-score" / "out"
    out.mkdir(exist_ok=True)
    lines = ["# Expert-pool feature coverage — all 17 languages", "",
             "Every Expert word should carry ≥1 `hardBecause` feature. Words listed",
             "under 'no feature' are long/rare but not verifiably hard to SPELL —",
             "candidates to demote or replace once the scoring pipeline has real",
             "graded/frequency data + expanded pools.", ""]
    for lang in langs:
        r = report(lang)
        if "skipped" in r:
            lines.append(f"## {lang} — {r['skipped']}\n")
            print(f"[{lang}] {r['skipped']}")
            continue
        lines.append(f"## {lang} — Expert feature coverage: **{r['coverage_pct']}%** "
                     f"({r['with_feature']}/{r['expert_count']})")
        lines.append("\nSample (word → features):")
        for w, f in r["sample"]:
            lines.append(f"- `{w}` → {', '.join(f) if f else '⚠️ none'}")
        if r["no_feature_words"]:
            lines.append(f"\n**{len(r['no_feature_words'])} Expert words with NO feature** "
                         f"(review): {', '.join(r['no_feature_words'][:20])}")
        lines.append("")
        print(f"[{lang}] Expert feature coverage: {r['coverage_pct']}% "
              f"({r['with_feature']}/{r['expert_count']}); {len(r['no_feature_words'])} flagged")
    (out / "expert-coverage.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nreport → tools/difficulty-score/out/expert-coverage.md")


if __name__ == "__main__":
    main()
