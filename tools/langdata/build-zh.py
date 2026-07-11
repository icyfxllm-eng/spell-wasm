# -*- coding: utf-8 -*-
"""Phase 1 data pipeline — Mandarin (zh) pilot.

Ingests three open datasets (fetched to sources/zh/, gitignored) and emits a
SpellGame word list with per-word metadata + a Gate-1 review sample:

  * HSK 3.0 wordlists  → educator-graded tier anchor + tone-numbered pinyin
  * CC-CEDICT          → English definitions
  * Unihan             → total stroke count (character-recall difficulty signal)

Tiering is anchored on HSK band (the educator grade), per the master plan:
  HSK1 → easy · HSK2 → medium · HSK3-4 → hard · HSK5-9 → expert
character complexity (stroke total) orders words *within* a tier.

Build-time only; nothing here ships. Emits out/zh.json (full, gitignored) and
out/zh-gate1-sample.md (30 words for spot-check). Licenses in NOTICES.md.

  python3 tools/langdata/build-zh.py
"""
from __future__ import annotations
import json, re, gzip
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "sources" / "zh"
OUT = ROOT / "out"
OUT.mkdir(exist_ok=True)

TIER_FOR_HSK = {1: "easy", 2: "medium", 3: "hard", 4: "hard", 5: "expert", 6: "expert", 7: "expert"}

# ---- CC-CEDICT: simplified → (pinyin, best English gloss) ----
# A char/word can have several entries (multi-reading 多音字, sense splits). We
# score each entry's best gloss and keep the highest — so 蛋 picks "egg", not the
# surname; 说 picks "to speak" (shuo1), not "to persuade" (shui4). CC-CEDICT's
# pinyin for the chosen sense is used (more reliable here than HSK's form[0]).
_SKIP = ("variant of", "see ", "surname ", "abbr. for", "old variant", "(bound form)")
def _gloss_quality(glosses: list[str]) -> tuple[int, str]:
    for g in glosses:
        g = g.strip()
        if not g or g.lower().startswith(_SKIP):
            continue
        if g[:1].isupper() and " " not in g.rstrip("."):  # bare proper noun
            continue
        return (2, g)  # a clean common-word gloss
    for g in glosses:  # else any non-xref gloss
        g = g.strip()
        if g and not g.lower().startswith(("variant of", "see ")):
            return (1, g)
    return (0, glosses[0].strip() if glosses else "")

def load_cedict() -> dict[str, tuple[str, str]]:
    from collections import defaultdict
    entries = defaultdict(list)
    line_re = re.compile(r"^(\S+)\s+(\S+)\s+\[([^\]]*)\]\s+/(.+)/\s*$")
    for line in (SRC / "cedict.txt").read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            continue
        m = line_re.match(line)
        if m:
            entries[m.group(2)].append((m.group(3), m.group(4).split("/")))
    out = {}
    for simp, ents in entries.items():
        best = max(ents, key=lambda e: _gloss_quality(e[1])[0])
        pinyin = best[0].lower().replace("u:", "ü")  # tone-numbered, lowercased
        out[simp] = (pinyin, _gloss_quality(best[1])[1])
    return out

# ---- Unihan: char → total strokes ----
def load_strokes() -> dict[str, int]:
    strokes = {}
    for line in (SRC / "Unihan_IRGSources.txt").read_text(encoding="utf-8").splitlines():
        if "kTotalStrokes" not in line or line.startswith("#"):
            continue
        cp, _, val = line.split("\t")
        strokes[chr(int(cp[2:], 16))] = int(val.split()[0])
    return strokes

# ---- HSK 3.0: simplified → (level, numeric-pinyin, freq-rank) ----
def load_hsk() -> dict[str, tuple[int, str, int]]:
    out = {}
    for lvl in range(1, 8):
        for e in json.loads((SRC / f"hsk{lvl}.json").read_text(encoding="utf-8")):
            simp = e["simplified"]
            if simp in out:
                continue
            forms = e.get("forms", [{}])
            numeric = forms[0].get("transcriptions", {}).get("numeric", "") if forms else ""
            out[simp] = (lvl, numeric, e.get("frequency", 999999))
    return out

def main():
    cedict, strokes, hsk = load_cedict(), load_strokes(), load_hsk()
    records = []
    missing_def = 0
    for word, (lvl, hsk_pinyin, freq) in hsk.items():
        ced = cedict.get(word)
        pinyin = ced[0] if ced else hsk_pinyin.lower()
        definition = ced[1] if ced else ""
        if not definition:
            missing_def += 1
        stroke_total = sum(strokes.get(c, 0) for c in word)
        records.append({
            "word": word,
            "reading": pinyin,                # tone-numbered pinyin (matches app format)
            "definition_en": definition,
            "difficulty_tier": TIER_FOR_HSK[lvl],
            "hsk_level": lvl,
            "stroke_total": stroke_total,
            "char_count": len(word),
            "freq_rank": freq,
            "lang": "zh",
            "audio_key": word,
        })
    # order within tier by character complexity (stroke total), then rarity
    records.sort(key=lambda r: (["easy", "medium", "hard", "expert"].index(r["difficulty_tier"]),
                                r["stroke_total"], r["freq_rank"]))
    (OUT / "zh.json").write_text(json.dumps(records, ensure_ascii=False, indent=1), encoding="utf-8")

    # tier counts
    from collections import Counter
    counts = Counter(r["difficulty_tier"] for r in records)
    print(f"zh: {len(records)} words | tiers {dict(counts)} | {missing_def} missing definition")

    # ---- Gate-1 sample: ~30 words spread across tiers (mid-band of each) ----
    sample_md = ["# Phase 1 Gate-1 sample — 中文 (zh)", "",
                 f"{len(records)} words ingested from HSK 3.0 + CC-CEDICT + Unihan. Tier = HSK band;",
                 "within-tier order = stroke complexity. Spot-check tiers/readings/definitions.", ""]
    for tier in ["easy", "medium", "hard", "expert"]:
        pool = [r for r in records if r["difficulty_tier"] == tier and r["definition_en"]]
        # mid-band slice so it's representative, not edge cases
        mid = pool[len(pool) // 3: len(pool) // 3 + 8]
        sample_md.append(f"## {tier}  ({counts[tier]} words)")
        sample_md.append("| word | pinyin | strokes | HSK | definition |")
        sample_md.append("|---|---|---|---|---|")
        for r in mid:
            sample_md.append(f"| {r['word']} | {r['reading']} | {r['stroke_total']} | {r['hsk_level']} | {r['definition_en'][:48]} |")
        sample_md.append("")
    (OUT / "zh-gate1-sample.md").write_text("\n".join(sample_md) + "\n", encoding="utf-8")
    print(f"Gate-1 sample → tools/langdata/out/zh-gate1-sample.md")

if __name__ == "__main__":
    main()
