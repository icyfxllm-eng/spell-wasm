#!/usr/bin/env python3
"""CC-ORTHO-FIX-FIL-ROMANCE F1: the two held fil pairs. The left form is stored, the right one is quarantined and accepted at
every tier: komplikado (kumplikado accepted), politika (pulitika accepted). Run once, then build_saved_word_map.py."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO, TAG = "2026-11-06", "Eric 2026-11-06 (CC-ORTHO-FIX F1)", "CC-ORTHO-SWEEP-ALL: "
MERGE = [("komplikado", "kumplikado"), ("politika", "pulitika")]
ov = []
for keep, lose in MERGE:
    ev = TAG + f"same word as {keep}; {keep} kept (the left form is stored), {lose} an accepted variant; " + WHO
    assert ps.apply_cuts("fil", {lose}, "variant_duplicate", ev, D), lose
    ov.append(["fil", lose, "variant_duplicate", WHO, ev])
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f: csv.writer(f, lineterminator="\n").writerows(ov)
ps.rebuild_and_rehash()
p = f"{REPO}/data/fil/accepted-variants.tsv"
SRC = "KWF Ortograpiyang Pambansa 2014: e/i and o/u alternate in Filipino spellings; the stored form is the left one (CC-ORTHO-FIX F1, Eric 2026-11-06)"
with open(p, "a", encoding="utf-8") as f:
    for k, l in MERGE: f.write(f"{k}\t{l}\t{SRC}\te_i_o_u\n")
print("done")
