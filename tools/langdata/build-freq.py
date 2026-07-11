# -*- coding: utf-8 -*-
"""Phase 1 data pipeline — Korean (ko) / Thai (th), frequency-tiered.

The ideal graded sources (TOPIK vocab, Thai school-grade lists) aren't freely
redistributable, so per the spec's substitution rule we tier by **OpenSubtitles
frequency bands** (spoken-language bias, which fits a "hear it" app) and document
it here. Thai words are additionally validated against the PyThaiNLP dictionary.

KNOWN GATE-1 GAPS (flagged, not hidden):
  * No English definitions — no open ko-en / th-en dictionary was reachable in
    this environment. `definition_en` is empty; to be sourced later (Wiktionary/
    Volubilis) or dropped from the in-app meaning card for these languages.
  * No POS data — some high-frequency entries are function words; a native
    reviewer should prune them (this is exactly what Gate-1 is for).

Rarer = harder: tiers are frequency-rank bands, capped so we take quality words,
not the noisy 50k tail. reading = the word itself (native script).

  python3 tools/langdata/build-freq.py ko
  python3 tools/langdata/build-freq.py th
"""
from __future__ import annotations
import json, sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "out"; OUT.mkdir(exist_ok=True)

CFG = {
    "ko": {"lo": 0xAC00, "hi": 0xD7A3, "min_len": 2},   # Hangul syllables
    "th": {"lo": 0x0E00, "hi": 0x0E7F, "min_len": 2},   # Thai block
}
# rank bands (after filtering): easy, medium, hard, expert — then stop (cap)
BANDS = [("easy", 600), ("medium", 1200), ("hard", 2200), ("expert", 3000)]

def in_script(w: str, lo: int, hi: int) -> bool:
    return w != "" and all(lo <= ord(c) <= hi for c in w)

def main():
    lang = sys.argv[1]
    cfg = CFG[lang]
    src = ROOT / "sources" / lang
    # optional Thai dictionary validation
    valid = None
    wl = src / "words.txt"
    if wl.exists():
        valid = {l.strip() for l in wl.read_text(encoding="utf-8").splitlines() if l.strip()}

    # frequency file: "word count", already rank-ordered (most frequent first)
    filtered = []
    for line in (src / "freq.txt").read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) != 2:
            continue
        w = parts[0]
        if not in_script(w, cfg["lo"], cfg["hi"]) or len(w) < cfg["min_len"]:
            continue
        if valid is not None and w not in valid:
            continue
        filtered.append(w)

    records, i = [], 0
    for tier, n in BANDS:
        for w in filtered[i:i + n]:
            records.append({
                "word": w,
                "reading": w,               # native script is its own reading
                "definition_en": "",        # GATE-1 GAP: no open dict reachable
                "difficulty_tier": tier,
                "freq_rank": filtered.index(w) + 1,
                "char_count": len(w),
                "lang": lang,
                "audio_key": w,
            })
        i += n
    (OUT / f"{lang}.json").write_text(json.dumps(records, ensure_ascii=False, indent=1), encoding="utf-8")
    counts = Counter(r["difficulty_tier"] for r in records)
    print(f"{lang}: {len(records)} words | tiers {dict(counts)} | (definitions: GATE-1 GAP)")

    md = [f"# Phase 1 Gate-1 sample — {lang}", "",
          f"{len(records)} words, tiered by OpenSubtitles frequency bands (documented",
          "substitute for TOPIK/school-grade lists). **Gaps to confirm:** no English",
          "definitions (no open dict reachable), and high-freq function words may need",
          "native pruning. Spot-check the tier feel.", ""]
    for tier, _ in BANDS:
        pool = [r for r in records if r["difficulty_tier"] == tier]
        sample = " · ".join(r["word"] for r in pool[len(pool) // 3: len(pool) // 3 + 12])
        md.append(f"## {tier}  ({counts[tier]} words)\n{sample}\n")
    (OUT / f"{lang}-gate1-sample.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print(f"Gate-1 sample → tools/langdata/out/{lang}-gate1-sample.md")

if __name__ == "__main__":
    main()
