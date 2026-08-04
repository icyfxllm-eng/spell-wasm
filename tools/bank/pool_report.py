#!/usr/bin/env python3
"""F6 pool-floor snapshot: audited rows per tier vs the signed floors.
Report-only until a language's `launched` flag flips after its wave
(F7). Reads the repo bank sources (assets/words/<lang>/<tier>.txt)."""
import json, pathlib

FLOORS = json.loads(pathlib.Path("config/bank_floors.json").read_text())
SRC = pathlib.Path("assets/words")  # the app repo IS the bank source
TIERS = ["easy", "medium", "hard", "expert"]

print("| lang | tier | rows | floor | status |")
print("|------|------|------|-------|--------|")
for lang, row in FLOORS["languages"].items():
    pools = row["poolFloors"]
    for i, floor in enumerate(pools):
        tier = TIERS[i] if i < len(TIERS) else f"t{i+1}"
        f = SRC / lang / f"{tier}.txt"
        n = sum(1 for l in f.read_text().splitlines() if l.strip()) if f.exists() else 0
        ok = "ok" if n >= floor else ("REPORT-ONLY GAP" if not row.get("launched") else "RED")
        print(f"| {lang} | {tier} | {n} | {floor} | {ok} |")
