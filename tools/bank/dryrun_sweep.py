#!/usr/bin/env python3
"""CC-BANK-PURITY C4/C8 helper — apply the PROVISIONAL Phase A sweep to a scratch copy.

    python3 tools/bank/dryrun_sweep.py <scratch-repo-root>

Reads reports/bank-purity-rows.csv (from this checkout) and rewrites assets/words/<lang>/<tier>.txt
under the scratch root only:
  - quarantine classes, P11 duplicates, and hi unconfirmed (P8) are removed;
  - misspelled rows are corrected in place; zh traditional rows get the simplified form;
  - de noun_shown_lowercase rows are stored capitalised (F5).
Never run it against a real checkout.  No threshold or test is touched.
"""
import csv, os, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, ALL, TIERS, load_bank, ROOT

target = os.path.abspath(sys.argv[1])
assert target != ROOT and os.path.isdir(target + "/assets/words"), "give the scratch copy, not this checkout"
nfc = lambda s: unicodedata.normalize("NFC", s)
rows = list(csv.DictReader(open(f"{ROOT}/reports/bank-purity-rows.csv", encoding="utf-8")))
bank = {l: {nfc(w) for _, w in load_bank(l)} for l in ALL}
action = {}   # (lang, word) -> None (remove) | new word
for r in rows:
    l, w, c, ev = r["lang"], nfc(r["word"]), r["class"], r["evidence"]
    if c in QUARANTINE or (c == "not_in_reference_dictionary" and l == "hi"):
        action[(l, w)] = None
    elif c == "misspelled_missing_diacritic":
        new = nfc(ev[2:].strip())
        action[(l, w)] = None if new in bank[l] else new
    elif c == "traditional_character":
        py, _, han = w.partition("|")
        new = f"{py}|{ev}" if ev and "|" not in ev else None
        action[(l, w)] = None if (new is None or nfc(new) in bank[l]) else new
    elif c == "noun_shown_lowercase":
        action[(l, w)] = w[:1].upper() + w[1:]
summary = {}
for l in ALL:
    for t in TIERS:
        p = f"{target}/assets/words/{l}/{t}.txt"
        if not os.path.exists(p):
            continue
        out, seen, before = [], set(), 0
        for line in open(p, encoding="utf-8").read().split("\n"):
            w = line.strip()
            if not w:
                continue
            before += 1
            a = action.get((l, nfc(w)), w)
            if a is None or nfc(a) in seen:
                continue
            seen.add(nfc(a)); out.append(a)
        open(p, "w", encoding="utf-8").write("\n".join(out) + "\n")
        summary[(l, t)] = (before, len(out))
for l in ALL:
    print(l, " ".join(f"{t}:{summary[(l,t)][0]}->{summary[(l,t)][1]}" for t in TIERS if (l, t) in summary))
