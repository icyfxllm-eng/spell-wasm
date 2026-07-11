# -*- coding: utf-8 -*-
"""Phase 1 data pipeline — Japanese (ja).

Sources (open, fetched build-time, gitignored):
  * JLPT word lists (elzup/jlpt-word-list) — word + reading + meaning + JLPT level
  * KANJIDIC2 (EDRDG)                       — kyōiku school grade per kanji (metadata)

Tier = JLPT band (N5→easy … N1→expert), the educator grade. Each word is
classified answerScript = "kanji" | "kana" (codepoint test) — this drives the
Phase 2 drawing routing (kanji words → 書き取り/drawing; kana words → typed
dictation). Build-time only; emits out/ja.json + a Gate-1 sample.

  python3 tools/langdata/build-ja.py
"""
from __future__ import annotations
import csv, json, re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "sources" / "ja"
OUT = ROOT / "out"; OUT.mkdir(exist_ok=True)

# process easiest→hardest so a word in several levels keeps its beginner tier
JLPT_ORDER = [("n5", "easy"), ("n4", "medium"), ("n3", "hard"), ("n2", "expert"), ("n1", "expert")]

def is_kanji(s: str) -> bool:
    return any(0x4E00 <= ord(c) <= 0x9FFF for c in s)  # CJK Unified Ideographs

def kanji_grade() -> dict[str, int]:
    grade = {}
    lit = None
    for line in (SRC / "kanjidic2.xml").read_text(encoding="utf-8").splitlines():
        m = re.search(r"<literal>(.)</literal>", line)
        if m:
            lit = m.group(1)
        g = re.search(r"<grade>(\d+)</grade>", line)
        if g and lit:
            grade[lit] = int(g.group(1))
    return grade

def main():
    grades = kanji_grade()
    seen, records = set(), []
    for fname, tier in JLPT_ORDER:
        with open(SRC / f"jlpt-{fname}.csv", encoding="utf-8") as f:
            for row in csv.DictReader(f):
                word = row["expression"].strip()
                if not word or word in seen or not re.fullmatch(r"[぀-ヿ一-鿿ー]+", word):
                    continue  # kana/kanji only (drop punctuation/latin/particles-with-marks)
                seen.add(word)
                script = "kanji" if is_kanji(word) else "kana"
                # hardest kyōiku grade among the word's kanji (metadata / ja-only labels)
                gr = max((grades.get(c, 0) for c in word if is_kanji(c)), default=0)
                records.append({
                    "word": word,
                    "reading": row["reading"].strip(),
                    "definition_en": row["meaning"].strip(),
                    "difficulty_tier": tier,
                    "jlpt": fname.upper(),
                    "answer_script": script,      # kanji → drawing (Phase 2); kana → typed
                    "kanji_grade": gr,
                    "char_count": len(word),
                    "lang": "ja",
                    "audio_key": word,
                })
    records.sort(key=lambda r: (["easy", "medium", "hard", "expert"].index(r["difficulty_tier"]),
                                r["kanji_grade"], r["char_count"]))
    (OUT / "ja.json").write_text(json.dumps(records, ensure_ascii=False, indent=1), encoding="utf-8")
    counts = Counter(r["difficulty_tier"] for r in records)
    scripts = Counter(r["answer_script"] for r in records)
    print(f"ja: {len(records)} words | tiers {dict(counts)} | scripts {dict(scripts)}")

    md = ["# Phase 1 Gate-1 sample — 日本語 (ja)", "",
          f"{len(records)} words from JLPT lists + KANJIDIC2. Tier = JLPT band; `script`",
          "= kanji (→drawing, Phase 2) or kana (→typed dictation). Spot-check.", ""]
    for tier in ["easy", "medium", "hard", "expert"]:
        pool = [r for r in records if r["difficulty_tier"] == tier and r["definition_en"]]
        md.append(f"## {tier}  ({counts[tier]} words)")
        md.append("| word | reading | script | definition |")
        md.append("|---|---|---|---|")
        for r in pool[len(pool) // 3: len(pool) // 3 + 8]:
            md.append(f"| {r['word']} | {r['reading']} | {r['answer_script']} | {r['definition_en'][:44]} |")
        md.append("")
    (OUT / "ja-gate1-sample.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("Gate-1 sample → tools/langdata/out/ja-gate1-sample.md")

if __name__ == "__main__":
    main()
