#!/usr/bin/env python3
"""v8.1 acceptance #1 — capacity math vs hand-computed fixtures."""
import json, math, sys
doc = json.loads(open(sys.argv[1]).read())
AVG = doc["avg_advance"]
measured = json.loads(open("out/avg_advance.json").read())
assert abs(measured["avg_advance_em"] - AVG) < 0.005, "AVG_ADVANCE drifted from font measurement"
bad = 0
for c in doc["cases"]:
    pts = c["points"]
    arc = sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(pts, pts[1:]))
    cap = arc / (c["size"] * AVG)
    if abs(cap - c["expected_cap"]) > 1.0:
        print("FAIL", c["name"], c["size"], cap, "!=", c["expected_cap"]); bad += 1
print(f"AVG_ADVANCE report: {AVG} em (font-measured: {measured['avg_advance_em']} from {measured['chars']} chars)")
print(f"{len(doc['cases'])-bad}/{len(doc['cases'])} capacity cases within ±1 char")
sys.exit(1 if bad else 0)
