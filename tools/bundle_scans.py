#!/usr/bin/env python3
"""Bundle the scan library into the shipping manifest (path data only —
references never ship, I6). Coordinates quantized to 0.5px: below the
0.05px residual epsilon's visibility and it halves the payload."""
import json, pathlib
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
OUT = ROOT / "config/wordpic/scans.json"
TIER_ORDER = {"easy": 0, "medium": 1, "hard": 2, "expert": 3}
subjects = {}
for f in sorted(SCANS.glob("*.json")):
    d = json.loads(f.read_text())
    paths = []
    for e in d["paths"]:
        pts = [[round(x * 2) / 2, round(y * 2) / 2] for x, y in e["points"]]
        ded = [pts[0]]
        for p in pts[1:]:
            if p != ded[-1]:
                ded.append(p)
        if len(ded) < 2:
            continue
        paths.append({
            "p": ded,
            "s": [round(t, 4) for t in e["segments"]],
            "f": (1 if e["sub_floor"] else 0) | (2 if e["decorative_thin"] else 0)
                 | (4 if e.get("micro_feature") else 0),
        })
    subjects[d["subject"]] = {
        "tier": d["tier"],
        "req": d.get("required_micro", []),
        "paths": paths,
    }
OUT.write_text(json.dumps({"v": "8.2", "subjects": subjects}, separators=(",", ":")))
kb = OUT.stat().st_size / 1024
print(f"bundled {len(subjects)} subjects -> {OUT} ({kb:.0f} KB)")
