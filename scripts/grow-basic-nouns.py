#!/usr/bin/env python3
"""Basic-concrete-noun bank growth (Eric's ruling, 2026-07-28).

The Leipzig news-frequency corpora left the banks without kid-basic concrete
nouns (ru had no word for cat/dog/tree/star/moon/fish; zh none; ar/hi ~1 in
6) — discovered by the CC-WORD-PICTURE coverage matrix, but a gap for EVERY
mode. Per Eric: grow the banks first, through the standard pipeline.

For each concept in the Word Picture mapping table whose candidates are all
absent from a language's bank, the first space-free candidate is added to the
EASY tier (they are, definitionally, basic vocabulary). zh additions carry
the bank's "pinyin|hanzi" shape. All additions ride the standing audit round
like every other non-English word; `build-wordlists.py --check` remains the
gate of record and must stay green after this runs.

Idempotent: words already present are never duplicated.
"""
import importlib.util
import os
import re
import sys
import unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location(
    "matrix", os.path.join(ROOT, "scripts", "build-wordpic-matrix.py"))
matrix = importlib.util.module_from_spec(spec)
spec.loader.exec_module(matrix)

NFC = lambda s: unicodedata.normalize("NFC", s)

# zh needs full bank-shaped entries (pinyin|hanzi), keyed by concept.
ZH_ADD = {
    "sun": "tai4yang2|太阳", "moon": "yue4liang4|月亮", "star": "xing1xing5|星星",
    "cloud": "yun2duo3|云朵", "sky": "tian1kong1|天空", "rain": "xia4yu3|下雨",
    "snow": "xue3hua1|雪花", "tree": "shu4mu4|树木", "leaf": "ye4zi5|叶子",
    "flower": "hua1duo3|花朵", "grass": "xiao3cao3|小草", "mountain": "gao1shan1|高山",
    "sea": "da4hai3|大海", "water": "he2shui3|河水", "fire": "huo3yan4|火焰",
    "house": "fang2zi5|房子", "door": "da4men2|大门", "window": "chuang1hu5|窗户",
    "cat": "xiao3mao1|小猫", "dog": "xiao3gou3|小狗", "fish": "xiao3yu2|小鱼",
    "bird": "xiao3niao3|小鸟", "egg": "ji1dan4|鸡蛋", "eye": "yan3jing1|眼睛",
    "ear": "er3duo5|耳朵", "nose": "bi2zi5|鼻子", "mouth": "zui3ba5|嘴巴",
    "tail": "wei3ba5|尾巴", "wing": "chi4bang3|翅膀", "hat": "mao4zi5|帽子",
    "book": "tu2shu1|图书", "boat": "xiao3chuan2|小船", "heart": "ai4xin1|爱心",
    "ball": "pi2qiu2|皮球", "night": "ye4wan3|夜晚", "foot": "jiao3ya1|脚丫",
    "smoke": "chui1yan1|炊烟", "king": "guo2wang2|国王",
}


def ok_charset(lang, w):
    if " " in w:
        return False
    for ch in w:
        cat = unicodedata.category(ch)
        if not (cat.startswith("L") or cat in ("Mn", "Mc") or ch in "'-’ʼ"):
            return False
    return True


def main():
    write = "--write" in sys.argv
    added = {}
    for lang in matrix.LANGS:
        b = matrix.bank(lang)
        adds = []
        for concept, per in matrix.C.items():
            cands = per.get(lang, [])
            if any(NFC(c) in b for c in cands):
                continue  # already mapped — nothing to add
            if lang == "zh":
                entry = ZH_ADD.get(concept)
                if entry:
                    adds.append(entry)
                continue
            pick = next((NFC(c) for c in cands if ok_charset(lang, NFC(c))), None)
            if pick:
                adds.append(pick)
        added[lang] = sorted(set(adds))
        print(f"  {lang}: +{len(added[lang])}  {added[lang][:6]}{'…' if len(added[lang]) > 6 else ''}")

    if not write:
        print("(dry run — pass --write to apply)")
        return

    for lang, adds in added.items():
        if not adds:
            continue
        if lang == "zh":
            p = os.path.join(ROOT, "src", "words.rs")
            src = open(p, encoding="utf-8").read()
            m = re.search(r"(ZH_EASY: &\[&str\] = &\[)(.*?)(\];)", src, re.S)
            body = m.group(2).rstrip()
            if not body.endswith(","):
                body += ","
            existing = set(re.findall(r'"([^"]+)"', m.group(2)))
            new = [e for e in adds if e not in existing]
            body += "\n    // Basic concrete nouns (Eric's bank-growth ruling, 2026-07-28) — audit-riding.\n"
            body += "".join(f'    "{e}",\n' for e in new)
            src = src[:m.start(2)] + body + src[m.end(2):]
            open(p, "w", encoding="utf-8").write(src)
            print(f"zh: +{len(new)} entries into ZH_EASY (src/words.rs)")
        else:
            p = os.path.join(ROOT, "assets", "words", lang, "easy.txt")
            existing = {NFC(w.strip()) for w in open(p, encoding="utf-8") if w.strip()}
            new = [w for w in adds if w not in existing]
            with open(p, "a", encoding="utf-8") as f:
                for w in new:
                    f.write(w + "\n")
            print(f"{lang}: +{len(new)} words into easy.txt")


if __name__ == "__main__":
    main()
