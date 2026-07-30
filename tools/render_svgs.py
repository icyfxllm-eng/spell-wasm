#!/usr/bin/env python3
"""Word-picture SVGs from scanlock placements (Eric's re-grade set)."""
import json, math, pathlib, subprocess, sys, unicodedata
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
POOLS = ROOT / "content-pipeline/wordpic/pools"
BIN = ROOT / "target/release/scanlock-render"
OUT = pathlib.Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)
BAND_MAX = {"easy": 40.0, "medium": 32.0, "hard": 24.0}
for f in sorted(SCANS.glob("*.json")):
    sub = f.stem
    doc = json.loads(f.read_text())
    tier = doc["tier"]
    pool = json.loads((POOLS/f"en-{tier}.json").read_text()) + json.loads((POOLS/"en-easy.json").read_text())
    lines = [f"WORD {w} {len(unicodedata.normalize('NFC', w))}" for w in pool if " " not in w]
    inp = [f"PARAMS 13.0 {BAND_MAX[tier]} 0.54 2.6 1"]
    for e in doc["paths"]:
        segs = " ".join(str(t) for t in e["segments"])
        pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
        inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} | {segs} | {pts}")
    r = subprocess.run([str(BIN)], input="\n".join(inp+lines).encode(), capture_output=True)
    j = json.loads(r.stdout)
    if "error" in j:
        print(sub, "ERROR", j["error"]); continue
    svg = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512"><rect width="512" height="512" fill="#101623"/>']
    for k, pl in enumerate(j["placements"]):
        d = "M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in pl["baseline"])
        svg.append(f'<defs><path id="b{k}" d="{d}"/></defs>')
        # justified: per-glyph placement via textLength over the full segment
        seg_len = sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(pl["baseline"], pl["baseline"][1:]))
        svg.append(f'<text font-size="{pl["size"]:.1f}" fill="#e8ecf5" font-family="Helvetica">'
                   f'<textPath href="#b{k}" textLength="{seg_len:.1f}" lengthAdjust="spacing">{pl["word"]}</textPath></text>')
    svg.append("</svg>")
    (OUT/f"{sub}-scanlock.svg").write_text("".join(svg))
    print(sub, "ok", len(j["placements"]), "words")
