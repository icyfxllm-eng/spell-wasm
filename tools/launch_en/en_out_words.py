#!/usr/bin/env python3
"""CC-LAUNCH-EN rev 4 second look (Eric 2026-11-09): the 18 English words he marked "out"
("Out means leave it out of the game completely!") are quarantined through CC-BANK-PURITY,
the same machinery as tools/ortho/ja_purity_batch2.py, so the restore round-trip holds."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO = "2026-11-09", "Eric 2026-11-09"
WORDS = "alcohol armed beer bomb casualties cult executed execution gay god gun guns sex sexual violence weapon weapons wine".split()
EV = "CC-LAUNCH-EN rev 4 second look: out of the game completely (not for a children's spelling game); " + WHO
moved = ps.apply_cuts("en", set(WORDS), "auditor_cut", EV, D)
assert moved == set(WORDS), sorted(set(WORDS) - moved)
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
    csv.writer(f, lineterminator="\n").writerows(["en", w, "auditor_cut", WHO, EV] for w in sorted(moved))
ps.rebuild_and_rehash()
print(len(moved), "en rows quarantined")
