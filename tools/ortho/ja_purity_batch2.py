#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-DE-JA v1.1 second ja PURITY batch: applies Eric's verdicts (reports/ja-quarantine-verdicts.tsv, 2026-11-04).
batch1 rows are all `cut` (already quarantined; nothing to restore). batch2 `cut` rows are quarantined; `unsure` rows are quarantined too and stay
there until the ja auditor rules; `keep` rows stay in the bank."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO, TAG = "2026-11-04", "Eric 2026-11-04", "CC-ORTHO-SWEEP-DE-JA v1.1 batch 2: "
rows = list(csv.DictReader(open(f"{REPO}/reports/ja-quarantine-verdicts.tsv", encoding="utf-8"), delimiter="\t"))
assert all(r["verdict"] in ("cut", "keep", "unsure") for r in rows if r["word"])
assert all(r["verdict"] == "cut" for r in rows if r["group"].startswith("batch1")), "a batch 1 restore needs a separate script"
moved = set(); ov = []
for r in rows:
    if not r["group"].startswith("batch2") or r["verdict"] == "keep": continue
    k = r["proposed_class"]; why = r["note"] or r["group"].split(": ")[1]
    ev = TAG + why + ("; UNSURE, auditor to rule" if r["verdict"] == "unsure" else "") + "; " + WHO
    got = ps.apply_cuts("ja", {r["word"]}, k, ev, D)
    if got: moved |= got; ov.append(["ja", r["word"], k, WHO, ev])
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
    csv.writer(f, lineterminator="\n").writerows(ov)
ps.rebuild_and_rehash()
print(len(moved), "ja rows quarantined")
