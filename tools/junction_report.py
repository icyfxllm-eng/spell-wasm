#!/usr/bin/env python3
"""v8.2 acceptance #1 — junction detection + keep-out inventory."""
import json, math, pathlib, sys
ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
RADIUS = 10.0
KEEPOUT_RATIO = 0.75
out = {}
for f in sorted(SCANS.glob("*.json")):
    doc = json.loads(f.read_text())
    P = [e for e in doc["paths"] if not e["sub_floor"] and not e["decorative_thin"]]
    js = []
    for i, e in enumerate(P):
        pts = e["points"]
        cum = [0.0]
        for a, b in zip(pts, pts[1:]):
            cum.append(cum[-1] + math.hypot(b[0]-a[0], b[1]-a[1]))
        tot = cum[-1] or 1.0
        hits = []
        for j, o in enumerate(P):
            if i == j: continue
            for ep in (o["points"][0], o["points"][-1]):
                for k, pt in enumerate(pts):
                    if math.hypot(pt[0]-ep[0], pt[1]-ep[1]) <= RADIUS:
                        hits.append(round(cum[k]/tot, 3)); break
            for t_end, ep in ((0.0, pts[0]), (1.0, pts[-1])):
                if any(math.hypot(q[0]-ep[0], q[1]-ep[1]) <= RADIUS for q in o["points"]):
                    hits.append(t_end)
        # self-junctions: arc-distant, space-near (the red circles)
        step = max(1, len(pts)//400)
        idx = list(range(0, len(pts), step))
        for ai in range(len(idx)):
            for bi in range(ai+1, len(idx)):
                a, b = idx[ai], idx[bi]
                gap = min(cum[b]-cum[a], tot - (cum[b]-cum[a]))
                if gap < RADIUS*4: continue
                if math.hypot(pts[a][0]-pts[b][0], pts[a][1]-pts[b][1]) <= RADIUS:
                    hits.append(round(cum[a]/tot, 2)); hits.append(round(cum[b]/tot, 2))
        hits = sorted(set(round(h, 2) for h in hits))
        if hits:
            js.append({"path": i, "arc": e["arc"], "junctions": hits})
    out[f.stem] = js
    print(f'{f.stem:10} paths_with_junctions={len(js):3} total_junctions={sum(len(x["junctions"]) for x in js)}')
json.dump(out, open(ROOT/"out/junctions.json", "w"), indent=1)
