#!/usr/bin/env python3
"""CC-BANK-PURITY §0 C6 — bank rows matching the repo's own profanity lists.

Read-only. Mirrors src/norm.rs fold_lenient; whole-word match, zh on the hanzi half
(src/kid_filter.rs key()). Writes reports/bank-profanity-matches.csv.
"""
import csv, os, re, sys, unicodedata

ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
LANGS = "en es fr de pt pl ko ja fil zh ru ar hi sw".split()
TIERS = "easy medium hard expert".split()
SILENT = set(range(0x064B, 0x0660)) | {0x0670, 0x0640, 0x200C}

def fold_lenient(s):
    s = unicodedata.normalize("NFC", s).lower()
    s = "".join(c for c in s if ord(c) not in SILENT)
    s = "".join(c for c in unicodedata.normalize("NFD", s) if not 0x300 <= ord(c) <= 0x36F)
    s = s.replace("ß", "ss").replace("æ", "ae").replace("œ", "oe")
    s = "".join({"ł": "l", "ø": "o", "ı": "i"}.get(c, c) for c in s)
    return "".join(c for c in s if not c.isspace())

def load(path):
    if not os.path.exists(path):
        return None
    out = set()
    for l in open(path, encoding="utf-8"):
        l = l.strip()
        if l and not l.startswith("#"):
            out.add(fold_lenient(l))
    return out

def main():
    rows, summary = [], []
    for lang in LANGS:
        prof = load(f"{ROOT}/assets/words/profanity/{lang}.txt")
        kid = load(f"{ROOT}/assets/words/kid-exclude/{lang}.txt")
        n = 0
        if prof:
            for t in TIERS:
                p = f"{ROOT}/assets/words/{lang}/{t}.txt"
                if not os.path.exists(p):
                    continue
                for w in open(p, encoding="utf-8").read().split("\n"):
                    w = w.strip()
                    if w and fold_lenient(w.rsplit("|", 1)[-1]) in prof:
                        rows.append((lang, t, w)); n += 1
        summary.append((lang, "none" if prof is None else len(prof),
                        "none" if kid is None else len(kid), n))
    with open(f"{ROOT}/reports/bank-profanity-matches.csv", "w", newline="", encoding="utf-8") as f:
        cw = csv.writer(f); cw.writerow(["lang", "tier", "word"]); cw.writerows(rows)
    print("lang  profanity_list  kid_list  bank_matches")
    for s in summary: print("%-5s %-15s %-9s %s" % s)
    print("total matches:", len(rows))

if __name__ == "__main__":
    main()
