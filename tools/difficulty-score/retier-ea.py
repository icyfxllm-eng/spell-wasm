# -*- coding: utf-8 -*-
"""Layer-1 re-tier for the East Asian languages (zh/ja/ko/th).

The EA tiers were graded by length/topic, so every tier felt like the same kind
of challenge (and zh Expert even held trivial words like 你好/谢谢). This re-tiers
each language by ITS OWN spelling-difficulty axis — the traps that survive that
language's actual input method — so each rank introduces a new class of trap:

  中文 (pinyin typed): ü, retroflex zh/ch/sh, -ng finals, syllable count.
     Character difficulty can't survive pinyin, so this is the honest ceiling
     until drawing mode ships; ultra-common greetings are forced down to Easy.
  日本語 (kana typed): long vowels (おう/えい/おお), small っ, small ゃゅょ, づ/ぢ.
  한국어 (Hangul typed): 받침, sound-change (연음), double batchim ㄲㄳㄼ…, ㅐ/ㅔ.
  ไทย (Thai typed): การันต์ silent letters, clusters, Sanskrit/Pali rare consonants.

Re-tiering is a pure reorder of ALREADY-VETTED words — no new content, counts per
tier preserved (balance gate stays green), each word in exactly one tier. It can
only concentrate the traps that EXIST; the real spike needs harder words added
under native review (Layer 2).

  python3 tools/difficulty-score/retier-ea.py            # report only
  python3 tools/difficulty-score/retier-ea.py --apply    # rewrite pools
"""
from __future__ import annotations
import argparse, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
WORDS = ROOT / "assets" / "words"
WORDS_RS = ROOT / "src" / "words.rs"
TIERS = ["easy", "medium", "hard", "expert"]

# ---------------- per-language difficulty scorers ----------------

def zh_score(entry: str) -> float:
    pinyin, _, hanzi = entry.partition("|")
    if hanzi in {"你好", "谢谢", "很好", "一起", "不对", "再见", "谢谢"}:
        return -10.0  # ultra-common greetings/function words → Easy
    s = 0.0
    if "ü" in pinyin:
        s += 4.0  # the jqx+u / lü-nü rule — the classic pinyin trap
    sylls = re.findall(r"[a-zü]+[1-5]", pinyin)
    for syl in sylls:
        base = syl.rstrip("12345")
        if base.startswith(("zh", "ch", "sh", "r")):
            s += 1.0  # retroflex vs dental
        if base.endswith("ng"):
            s += 0.8  # -n vs -ng
        if any(f in base for f in ("iong", "uan", "üan", "iang", "uang")):
            s += 0.5
    s += 0.5 * len(sylls)
    return s

def ja_score(w: str) -> float:
    s = 0.0
    if "っ" in w:
        s += 3.0
    if "づ" in w or "ぢ" in w:
        s += 5.0  # the four-kana づ/ぢ vs ず/じ trap
    for ch in w:
        if ch in "ゃゅょ":
            s += 1.5
        if ch in "がぎぐげござじずぜぞだぢづでどばびぶべぼぱぴぷぺぽ":
            s += 0.4
    if "ー" in w:
        s += 2.0
    for i in range(len(w) - 1):
        a, b = w[i], w[i + 1]
        if b == "う" and a in "おこそとのほもよろごぞどぼぽしちにひみりじぎ":
            s += 2.0  # long o (-ou)
        elif b == "い" and a in "えけせてねへめれげぜでべぺ":
            s += 1.5  # long e (-ei)
        elif w[i:i + 2] in ("おお", "ええ", "うう"):
            s += 2.0
    s += 0.4 * len(w)
    return s

_KO_DOUBLE = {3, 5, 6, 9, 10, 11, 12, 13, 14, 15, 18}   # ㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ
_KO_AMBIG_VOWEL = {1, 5, 10, 11, 15}                     # ㅐㅔㅙㅚㅞ
def ko_score(w: str) -> float:
    s = 0.0
    prev_final = 0
    for ch in w:
        u = ord(ch)
        if 0xAC00 <= u <= 0xD7A3:
            idx = u - 0xAC00
            init, med, fin = idx // 588, (idx // 28) % 21, idx % 28
            if fin != 0:
                s += 0.8
            if fin in _KO_DOUBLE:
                s += 4.0  # 겹받침
            if med in _KO_AMBIG_VOWEL:
                s += 1.0
            if prev_final != 0 and init == 11:  # ㅇ onset after a batchim → 연음
                s += 1.5
            elif prev_final != 0 and init in {0, 3, 7, 9, 12}:  # tensification/보이는 sound-change
                s += 0.8
            prev_final = fin
        else:
            prev_final = 0
    s += 0.5 * len(w)
    return s

_TH_CONS = set("กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรลวศษสหฬอฮ")
_TH_RARE = set("ฆฌญฎฏฐฑฒณธภศษฬฤฦ")  # Sanskrit/Pali-loan consonants
def th_score(w: str) -> float:
    s = 0.0
    if "์" in w:
        s += 5.0  # การันต์ (thanthakhat, silent letter)
    if "ๆ" in w:
        s += 1.0
    for ch in w:
        if ch in "่้๊๋":
            s += 0.4
    for i in range(len(w) - 1):
        if w[i] in _TH_CONS and w[i + 1] in "รลว":
            s += 1.2  # ควบกล้ำ cluster
    s += 2.0 * sum(1 for ch in w if ch in _TH_RARE)
    s += 0.3 * sum(1 for ch in w if ch in _TH_CONS)
    return s

SCORERS = {"ja": ja_score, "ko": ko_score, "th": th_score}

# ---------------- re-tier engine ----------------

def retier(words_by_tier: dict[str, list[str]], score) -> tuple[dict[str, list[str]], list[str]]:
    counts = {t: len(words_by_tier[t]) for t in TIERS}
    old_tier = {w: t for t in TIERS for w in words_by_tier[t]}
    # Dedupe the union (some source files had a word in two tiers — the build
    # pipeline hid it via first-tier-wins). Keep one copy; a word lands in
    # exactly one tier.
    seen, allw = set(), []
    for t in TIERS:
        for w in words_by_tier[t]:
            if w not in seen:
                seen.add(w)
                allw.append(w)
    allw.sort(key=lambda w: (score(w), len(w), w))
    # Fill easy/medium/hard at their original sizes; expert takes the remainder
    # (a couple fewer if dupes were removed).
    out, moves, i = {}, [], 0
    for t in TIERS[:-1]:
        out[t] = allw[i:i + counts[t]]
        i += counts[t]
    out[TIERS[-1]] = allw[i:]
    for t in TIERS:
        for w in out[t]:
            if old_tier[w] != t:
                moves.append(f"{w}: {old_tier[w]} → {t}")
    return out, moves

# ---------------- io: ja/ko/th (assets/words) ----------------

def load_txt(lang: str, tier: str) -> list[str]:
    p = WORDS / lang / f"{tier}.txt"
    return [l.strip() for l in p.read_text(encoding="utf-8").splitlines() if l.strip() and not l.startswith("#")]

def write_txt(lang: str, tier: str, words: list[str]):
    (WORDS / lang / f"{tier}.txt").write_text("\n".join(words) + "\n", encoding="utf-8")

# ---------------- io: zh (src/words.rs consts) ----------------

def load_zh() -> dict[str, list[str]]:
    src = WORDS_RS.read_text(encoding="utf-8")
    out = {}
    for tier in TIERS:
        name = f"ZH_{tier.upper()}"
        m = re.search(rf"pub const {name}: &\[&str\] = &\[(.*?)\];", src, re.S)
        out[tier] = re.findall(r'"([^"]+)"', m.group(1))
    return out

def write_zh(new: dict[str, list[str]]):
    src = WORDS_RS.read_text(encoding="utf-8")
    for tier in TIERS:
        name = f"ZH_{tier.upper()}"
        body = ",\n    ".join(f'"{w}"' for w in new[tier])
        repl = f"pub const {name}: &[&str] = &[\n    {body},\n];"
        src = re.sub(rf"pub const {name}: &\[&str\] = &\[.*?\];", lambda _m: repl, src, count=1, flags=re.S)
    WORDS_RS.write_text(src, encoding="utf-8")

# ---------------- main ----------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    out_md = ["# Layer-1 East-Asian re-tier diff", "",
              "Reorder of already-vetted words by each script's spelling-difficulty",
              "axis (traps that survive the actual input method). Counts per tier",
              "unchanged; no new content. Real spikes need Layer-2 (harder words,",
              "native-reviewed).", ""]

    # zh
    zh = load_zh()
    zh_new, zh_moves = retier(zh, zh_score)
    out_md.append(f"## zh — {len(zh_moves)} moves (pinyin-difficulty axis; character difficulty needs drawing)")
    for t in TIERS:
        sample = ", ".join(w.split("|")[1] for w in zh_new[t][:8])
        out_md.append(f"- **{t}**: {sample}")
    out_md += [f"  - moved: {m.split('|')[1] if '|' in m else m}" for m in zh_moves[:12]] + [""]
    if args.apply:
        write_zh(zh_new)

    # ja/ko/th
    for lang, score in SCORERS.items():
        cur = {t: load_txt(lang, t) for t in TIERS}
        new, moves = retier(cur, score)
        out_md.append(f"## {lang} — {len(moves)} moves")
        for t in TIERS:
            out_md.append(f"- **{t}**: {', '.join(new[t][:10])}")
        out_md += [f"  - moved: {m}" for m in moves[:12]] + [""]
        if args.apply:
            for t in TIERS:
                write_txt(lang, t, new[t])
        print(f"[{lang}] {len(moves)} moves")

    print(f"[zh] {len(zh_moves)} moves")
    outp = ROOT / "tools" / "difficulty-score" / "out"
    outp.mkdir(exist_ok=True)
    (outp / "retier-ea.md").write_text("\n".join(out_md) + "\n", encoding="utf-8")
    print(f"\n{'APPLIED' if args.apply else 'REPORT ONLY'} — diff → tools/difficulty-score/out/retier-ea.md")

if __name__ == "__main__":
    main()
