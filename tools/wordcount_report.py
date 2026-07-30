#!/usr/bin/env python3
"""v8.1 acceptance #4 — emergent word-count scaling report."""
import json, math, pathlib, subprocess, sys, unicodedata
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
POOLS = ROOT / "content-pipeline/wordpic/pools"
BIN = ROOT / "target/release/scanlock-render"
BAND_MAX = {"easy": 40.0, "medium": 32.0, "hard": 24.0}
gate = json.loads((ROOT / "out/gate.json").read_text())
out = []
for f in sorted(SCANS.glob("*.json")):
    sub = f.stem
    if gate.get(sub, {}).get("status") == "BLOCKED":
        continue
    doc = json.loads(f.read_text())
    arc_total = sum(e["arc"] for e in doc["paths"] if not e["sub_floor"] and not e["decorative_thin"])
    pool = json.loads((POOLS/f"en-{doc['tier']}.json").read_text()) + json.loads((POOLS/"en-easy.json").read_text())
    lines = [f"WORD {w} {len(unicodedata.normalize('NFC', w))}" for w in pool if " " not in w]
    inp = [f"PARAMS 13.0 {BAND_MAX[doc['tier']]} 0.5257 1.0 1"]
    for e in doc["paths"]:
        segs = " ".join(str(t) for t in e["segments"])
        pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
        inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} | {segs} | {pts}")
    j = json.loads(subprocess.run([str(BIN)], input="\n".join(inp+lines).encode(), capture_output=True).stdout)
    n = len(j.get("placements", []))
    out.append({"subject": sub, "arc": round(arc_total), "words": n,
                "size": j["placements"][0]["size"] if n else None})
out.sort(key=lambda r: r["arc"])
mono = all(a["words"] <= b["words"] * 1.35 for a, b in zip(out, out[1:]))  # scaling with slack
for r in out:
    print(f'{r["subject"]:10} arc={r["arc"]:6} size={r["size"]}  words={r["words"]}')
json.dump(out, open(ROOT/"out/counts.json", "w"), indent=1)
print("scaling with arc:", "OK" if mono else "CHECK")
