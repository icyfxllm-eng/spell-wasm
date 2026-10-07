#!/usr/bin/env python3
"""CC-BANK-PURITY §0 C7 — residual proper nouns a Wiktionary part-of-speech filter would add.

Report only.  A bank row is a "residual proper noun" when every Wiktionary entry (headword or
inflected/alternative form) that yields it has pos == "name", and the current purity run has not
already put it in a quarantine class.  Reads .corpus-cache/wikt/<Language>.jsonl.
Writes reports/bank-purity-c7.csv and prints count + 50-row sample per language.
"""
import csv, json, os, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, load_bank, ROOT, find

NAMES = {"ko": "Korean", "ja": "Japanese", "ar": "Arabic", "hi": "Hindi", "sw": "Swahili", "ru": "Russian"}
nfc = lambda s: unicodedata.normalize("NFC", s)

cls = {}
for r in csv.DictReader(open(f"{ROOT}/reports/bank-purity-rows.csv", encoding="utf-8")):
    cls[(r["lang"], r["tier"], r["word"])] = r["class"]

out, summary = [], []
for lang in (sys.argv[1:] or NAMES):
    rows = load_bank(lang)
    want = {nfc(w).lower() for _, w in rows}
    name_keys, other_keys = set(), set()
    for line in open(find(f"wikt/{NAMES[lang]}.jsonl"), encoding="utf-8"):
        d = json.loads(line)
        pos = d.get("pos")
        forms = [d.get("word", "")] + [f.get("form", "") for f in d.get("forms", [])]
        for f in forms:
            k = nfc(f).lower()
            if k in want:
                (name_keys if pos == "name" else other_keys).add(k)
    flagged = [(t, w) for t, w in rows if nfc(w).lower() in name_keys and nfc(w).lower() not in other_keys]
    new = [(t, w) for t, w in flagged if cls.get((lang, t, w), "") not in QUARANTINE]
    summary.append((lang, len(rows), len(flagged), len(new)))
    for t, w in new:
        out.append((lang, t, w, cls.get((lang, t, w), "") or "pass"))
    step = max(1, len(new) // 50)
    print(f"\n{lang}: {len(flagged)} rows are name-only in Wiktionary; {len(new)} not already quarantined")
    print("  sample:", " ".join(w for _, w in new[::step][:50]))
with open(f"{ROOT}/reports/bank-purity-c7.csv", "w", newline="", encoding="utf-8") as f:
    cw = csv.writer(f, lineterminator="\n"); cw.writerow(["lang", "tier", "word", "current_class"]); cw.writerows(out)
print("\nlang rows name_only additional"); [print(*s) for s in summary]
