#!/usr/bin/env python3
"""v8.2 acceptance #2 — the OBB gate applied to stored plans. The four
red-circled v8.1 renders must FAIL with named word pairs."""
import argparse, json, math, pathlib, sys
ROOT = pathlib.Path(__file__).resolve().parents[1]

def obbs(pl):
    bl = pl["baseline"]
    cum = [0.0]
    for a, b in zip(bl, bl[1:]):
        cum.append(cum[-1] + math.hypot(b[0]-a[0], b[1]-a[1]))
    tot = cum[-1]
    if tot <= 0: return []
    n = max(1, round(tot / pl["advance"]))
    out = []
    for k in range(n):
        want = (k + 0.5) * tot / n
        for i in range(1, len(bl)):
            if cum[i] >= want:
                seg = cum[i] - cum[i-1]
                f = (want - cum[i-1]) / seg if seg else 0
                a, b = bl[i-1], bl[i]
                d = (b[0]-a[0], b[1]-a[1]); m = math.hypot(*d) or 1e-6
                out.append({"c": (a[0]+(b[0]-a[0])*f, a[1]+(b[1]-a[1])*f),
                            "hw": tot/n*0.5*0.92, "hh": pl["size"]*0.5,
                            "u": (d[0]/m, d[1]/m)})
                break
    return out

def corners(o):
    ux = (o["u"][0]*o["hw"], o["u"][1]*o["hw"])
    vy = (-o["u"][1]*o["hh"], o["u"][0]*o["hh"])
    c = o["c"]
    return [(c[0]+ux[0]+vy[0], c[1]+ux[1]+vy[1]), (c[0]+ux[0]-vy[0], c[1]+ux[1]-vy[1]),
            (c[0]-ux[0]-vy[0], c[1]-ux[1]-vy[1]), (c[0]-ux[0]+vy[0], c[1]-ux[1]+vy[1])]

def sat(a, b):
    for ax, ay in [a["u"], (-a["u"][1], a["u"][0]), b["u"], (-b["u"][1], b["u"][0])]:
        pa = [x*ax+y*ay for x, y in corners(a)]
        pb = [x*ax+y*ay for x, y in corners(b)]
        if max(pa) < min(pb) or max(pb) < min(pa): return False
    return True

ap = argparse.ArgumentParser()
ap.add_argument("--renders", required=True)
ap.add_argument("--expect", default=None)
args = ap.parse_args()
fails = tot = 0
for f in sorted(pathlib.Path(args.renders).glob("*.json")):
    if f.name == "README.md": continue
    j = json.loads(f.read_text())
    pls = j.get("placements", [])
    scan = json.loads((ROOT/f"content-pipeline/wordpic/scans/{f.stem}.json").read_text())
    ARCS = {i: e["arc"] for i, e in enumerate(scan["paths"])}
    if not pls: continue
    boxes = [(i, obbs(p)) for i, p in enumerate(pls)]
    hits = []
    for a in range(len(boxes)):
        for b in range(a+1, len(boxes)):
            pa, pb = pls[a], pls[b]
            # Exempt ONLY arc-adjacent runs (they abut by construction).
            # Arc-DISTANT runs on a self-pinching ring can still collide
            # in space — that is exactly Eric's red circles.
            if pa["path"] == pb["path"]:
                arc = ARCS.get(pa["path"], 1.0)
                tol = (1.0 + 0.75) * pa["size"] * 0.5257 + 0.75 * 2 * pa["size"]
                seam = ((1 - pa["t1"]) + pb["t0"]) * arc < tol or ((1 - pb["t1"]) + pa["t0"]) * arc < tol
                adjacent = abs(pa["t1"]-pb["t0"])*arc < tol or abs(pb["t1"]-pa["t0"])*arc < tol or seam
                if adjacent:
                    continue
            if any(sat(x, y) for x in boxes[a][1] for y in boxes[b][1]):
                hits.append((pa["word"], pb["word"]))
    tot += 1
    fails += bool(hits)
    status = f'FAIL ({len(hits)} pairs, e.g. {hits[0]})' if hits else "pass (CHECK: eval may be wrong)"
    print(f'{f.stem:10} words={len(pls):3} -> {status}')
print(f"\n{fails}/{tot} stored renders FAIL the OBB gate")
sys.exit(0 if (args.expect != "FAIL" or fails == tot) else 1)
