#!/usr/bin/env python3
"""CC-BANK-PURITY §0 C1 — row-level comparison of this checker against the evidence CSV.

Both sides are reduced to the same "would leave the bank in Phase A" set: quarantine classes,
P11 duplicates (misspelled whose correct form is already a row) and hi unconfirmed (P8).
Writes reports/bank-purity-c1-diff.csv (only-evidence / only-now rows) and prints a table.
"""
import csv, os, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, ALL, load_bank, ROOT
nfc = lambda s: unicodedata.normalize("NFC", s)

def removal(path):
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    bank = {l: {nfc(w) for _, w in load_bank(l)} for l in ALL}
    out, cls = {l: set() for l in ALL}, {}
    for r in rows:
        l, w, c = r["lang"], nfc(r["word"]), r["class"]
        cls[(l, w)] = c
        if c in QUARANTINE or (c == "not_in_reference_dictionary" and l == "hi"):
            out[l].add(w)
        elif c == "misspelled_missing_diacritic" and nfc(r["evidence"][2:].strip()) in bank[l]:
            out[l].add(w)
    return out, cls

ev, evc = removal(f"{ROOT}/reports/bank-check-2026-10-07.csv")
now, nowc = removal(f"{ROOT}/reports/bank-purity-rows.csv")
diff, table = [], []
for l in ALL:
    a, b = ev[l], now[l]
    only_ev, only_now = sorted(a - b), sorted(b - a)
    for w in only_ev: diff.append((l, "evidence_only", w, evc.get((l, w), ""), nowc.get((l, w), "pass")))
    for w in only_now: diff.append((l, "now_only", w, evc.get((l, w), "pass"), nowc.get((l, w), "")))
    d = len(only_ev) + len(only_now)
    table.append((l, len(a), len(b), len(a & b), len(only_ev), len(only_now), 100.0 * d / max(len(a), 1)))
with open(f"{ROOT}/reports/bank-purity-c1-diff.csv", "w", newline="", encoding="utf-8") as f:
    cw = csv.writer(f, lineterminator="\n"); cw.writerow(["lang", "side", "word", "evidence_class", "now_class"]); cw.writerows(diff)
print("lang evidence now both only_ev only_now  symdiff/evidence")
for t in table: print("%-4s %6d %6d %6d %6d %6d   %5.1f%%" % t)
