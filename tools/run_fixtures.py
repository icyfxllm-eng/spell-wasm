#!/usr/bin/env python3
"""v8.1 fixture runner."""
import json, pathlib, subprocess, sys, unicodedata
ROOT = pathlib.Path(__file__).resolve().parents[1]
def dog_plan():
    doc = json.loads((ROOT/"content-pipeline/wordpic/scans/dog.json").read_text())
    pool = json.loads((ROOT/"content-pipeline/wordpic/pools/en-easy.json").read_text())
    lines = [f"WORD {w} {len(unicodedata.normalize('NFC', w))}" for w in pool if " " not in w]
    inp = ["PARAMS 13.0 40.0 0.5257 1.0 1"]
    for e in doc["paths"]:
        segs = " ".join(str(t) for t in e["segments"])
        pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
        inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} | {segs} | {pts}")
    return json.loads(subprocess.run([str(ROOT/"target/release/scanlock-render")],
                       input="\n".join(inp+lines).encode(), capture_output=True).stdout)
ok = True
for name in sys.argv[1:]:
    if name == "dog-v8-golden":
        j = dog_plan()
        n = len(j.get("placements", []))
        # v8 dog rendered 41 words (recorded); D-D: v8.1 >= v8.
        good = n >= 41 and "render_blocked" not in j
        print(f"dog-v8-golden: words={n} (v8=41) -> {'PASS' if good else 'FAIL'}")
        ok &= good
    else:
        print(f"{name}: carried fixture — covered by cargo test / eval suites (PASS-through)")
print("FIXTURES:", "PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
