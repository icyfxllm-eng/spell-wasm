# -*- coding: utf-8 -*-
"""Emit app-format word banks from the approved Phase 1 data (zh + ja).

Takes a SANE SUBSET (the game needs variety, not 10k words — rolling exclusion
caps at 50) of the highest-signal words per tier and writes them into the app:

  zh → src/words.rs ZH_* consts, format "pinyin|hanzi" (pinyin space-stripped,
       tone-numbered, ü — matching the existing format the pinyin matcher expects)
  ja → assets/words/ja/{tier}.txt, KANA-answer words only (kanji words route to
       drawing in Phase 2, not typeable yet); the build pipeline then regenerates
       word_data.rs and enforces the kana-keyboard charset gate.

Held behind the zh/ja availability flag + native review before public release.

  python3 tools/langdata/emit-wordbanks.py            # report
  python3 tools/langdata/emit-wordbanks.py --apply
"""
from __future__ import annotations
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "out"
REPO = ROOT.parent.parent
TIERS = ["easy", "medium", "hard", "expert"]
CAP = {"easy": 150, "medium": 150, "hard": 200, "expert": 200}

def load(lang):
    return json.loads((OUT / f"{lang}.json").read_text(encoding="utf-8"))

def zh_bank():
    recs = load("zh")
    banks = {}
    for t in TIERS:
        pool = [r for r in recs if r["difficulty_tier"] == t]
        pool.sort(key=lambda r: r["freq_rank"])  # most frequent (best) first
        out = []
        for r in pool:
            py = r["reading"].replace(" ", "")           # "ping2 guo3" -> "ping2guo3"
            hz = r["word"]
            if not py or not hz or "|" in hz:
                continue
            if len(hz) < 2:                                # drop single-char function words (的/了/之)
                continue
            if not re.fullmatch(r"[a-zü1-5]+", py):        # clean tone-numbered pinyin only
                continue
            out.append(f"{py}|{hz}")
            if len(out) >= CAP[t]:
                break
        banks[t] = out
    return banks

def ja_kana_bank():
    recs = load("ja")
    banks = {}
    for t in TIERS:
        pool = [r for r in recs if r["difficulty_tier"] == t and r["answer_script"] == "kana"]
        pool.sort(key=lambda r: (r["kanji_grade"], r["char_count"]))
        out = []
        for r in pool:
            w = r["word"]
            if re.fullmatch(r"[぀-ゟ゠-ヿー]+", w):          # pure kana only (hiragana/katakana/ー)
                out.append(w)
            if len(out) >= CAP[t]:
                break
        banks[t] = out
    return banks

def write_zh(banks):
    src = (REPO / "src" / "words.rs").read_text(encoding="utf-8")
    for t in TIERS:
        name = f"ZH_{t.upper()}"
        body = ",\n    ".join(f'"{w}"' for w in banks[t])
        repl = f"pub const {name}: &[&str] = &[\n    {body},\n];"
        src = re.sub(rf"pub const {name}: &\[&str\] = &\[.*?\];", lambda _m: repl, src, count=1, flags=re.S)
    (REPO / "src" / "words.rs").write_text(src, encoding="utf-8")

def write_ja(banks):
    for t in TIERS:
        (REPO / "assets" / "words" / "ja" / f"{t}.txt").write_text("\n".join(banks[t]) + "\n", encoding="utf-8")

def main():
    apply = "--apply" in sys.argv
    zh, ja = zh_bank(), ja_kana_bank()
    print("zh (pinyin|hanzi):", {t: len(zh[t]) for t in TIERS})
    print("  easy sample:", " ".join(w.split("|")[1] for w in zh["easy"][:8]))
    print("  expert sample:", " ".join(w.split("|")[1] for w in zh["expert"][:8]))
    print("ja (kana only):", {t: len(ja[t]) for t in TIERS})
    print("  easy sample:", " ".join(ja["easy"][:10]))
    print("  expert sample:", " ".join(ja["expert"][:10]))
    if apply:
        write_zh(zh)
        # ja typed pool is NOT rewritten: the JLPT-kana subset tiers vocabulary,
        # not kana-spelling difficulty (ダム/ガム as "expert"), so it's no better
        # than the existing kana-trap-tiered pool. ja's Phase 1 value is its 6,379
        # KANJI words, reserved for Phase 2 drawing — not the typed kana pool.
        print("\nAPPLIED — src/words.rs (zh) rewritten. ja HELD (see note; kanji → Phase 2).")
    else:
        print("\nREPORT ONLY (pass --apply to write)")

if __name__ == "__main__":
    main()
