#!/usr/bin/env python3
"""CC-ORTHO-FIX-FIL-ROMANCE A: sunud-sunod quarantined as non-standard (Eric, 2026-11-06). Run once. It is a miss for now; a close
list entry follows when Phase B generalizes close feedback."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO = "2026-11-06", "Eric 2026-11-06"
ev = "CC-ORTHO-SWEEP-ALL: old spelling; the 2014 rules keep the o in repeated roots, so sunod-sunod is standard; not accepted, to become a close-list entry in Phase B; " + WHO
assert ps.apply_cuts("fil", {"sunud-sunod"}, "misspelled_nonstandard", ev, D)
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
    csv.writer(f, lineterminator="\n").writerow(["fil", "sunud-sunod", "misspelled_nonstandard", WHO, ev])
ps.rebuild_and_rehash()
print("done")
