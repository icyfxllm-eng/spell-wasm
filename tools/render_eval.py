#!/usr/bin/env python3
"""v8 F5 — shape-fidelity eval. Grades renders against the pinned scan:
residual_px (baseline vs pinned path; target 0), recall (pinned arc
covered by baselines; decorative-thin excluded; target 100%), precision
(baseline arc on-scan; target 100%), overlaps. Calibration mode grades
the broken corpus and expects FAIL."""
import argparse, json, math, pathlib, re, subprocess, sys, unicodedata

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
POOLS = ROOT / "content-pipeline/wordpic/pools"
BIN = ROOT / "target/release/scanlock-render"
FLOOR = 13.0
BAND_MAX = {"easy": 40.0, "medium": 32.0, "hard": 24.0, "expert": 30.0}
ADV = {"ja": 1.0, "ko": 0.95, "ar": 0.52, "hi": 0.58, "ru": 0.58}
MAX_JUSTIFY = 2.6  # D4 PROPOSED, pending Eric
LANGS = ["en","es","fr","de","pt","pl","ru","vi","ko","ja","zh","ar","hi","sw","fil"]
# Ledgered bank debt (carried from the engine's READINESS_EXCEPTIONS):
# zh lacks 1-syllable entries; segments <=100px cannot host long pinyin.
# Reported as LEDGERED, never as PASS.
LEDGERED = {"zh"}

def units(w):
    return len([c for c in unicodedata.normalize("NFC", w)])

def sd(pt, u, v):
    vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
    t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
    return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))

def eval_baselines(doc, placements, tol=0.75):
    paths = doc["paths"]
    residual = 0.0
    on_scan = total_base = 0.0
    covered = {}
    for pl in placements:
        pp = paths[pl["path"]]["points"]
        segs = list(zip(pp, pp[1:]))
        bl = pl["baseline"]
        for pt in bl:
            d = min(sd(pt, u, v) for u, v in segs)
            residual = max(residual, d)
        for a, b in zip(bl, bl[1:]):
            L = math.hypot(b[0]-a[0], b[1]-a[1])
            total_base += L
            mid = ((a[0]+b[0])/2, (a[1]+b[1])/2)
            if min(sd(mid, u, v) for u, v in segs) <= tol:
                on_scan += L
        covered.setdefault(pl["path"], []).append((pl["t0"], pl["t1"]))
    # recall over non-excluded paths
    rec_num = rec_den = 0.0
    for i, e in enumerate(paths):
        if e["sub_floor"] or e["decorative_thin"]:
            continue
        arc = e["arc"]
        bounds = [0.0] + [t for t in e["segments"] if 0 < t < 1] + [1.0]
        for a, b in zip(bounds, bounds[1:]):
            if (b - a) * arc >= FLOOR * 2.0:  # hostable spans only (D-A)
                rec_den += (b - a) * arc
        for (t0, t1) in covered.get(i, []):
            rec_num += (t1 - t0) * arc
    recall = rec_num / rec_den if rec_den else 0.0
    precision = on_scan / total_base if total_base else 0.0
    return residual, recall, precision

ap = argparse.ArgumentParser()
ap.add_argument("--subjects", default="all")
ap.add_argument("--langs", default="en")
ap.add_argument("--seeds", type=int, default=5)
ap.add_argument("--renders", default=None, help="calibration corpus dir")
ap.add_argument("--expect", default=None)
ap.add_argument("--json", default=None)
args = ap.parse_args()

if args.renders:
    # CALIBRATION: broken renders' word paths (their curated slot geometry)
    # graded against the pinned scans. Every one must FAIL.
    fails = tot = 0
    for f in sorted(pathlib.Path(args.renders).glob("*-filled.svg")):
        sub = f.stem.replace("-filled", "")
        sp = SCANS / f"{sub}.json"
        if not sp.exists():
            continue
        doc = json.loads(sp.read_text())
        svg = f.read_text()
        base_pts = []
        for d in re.findall(r'<path id="[^"]*" d="(M[^"]+)"', svg):
            pts = [(float(x), float(y)) for x, y in re.findall(r'(-?[\d.]+)[ ,](-?[\d.]+)', d)]
            base_pts.append(pts)
        if not base_pts:
            continue
        segs_all = [list(zip(e["points"], e["points"][1:])) for e in doc["paths"] if len(e["points"]) > 1]
        worst = 0.0
        for bl in base_pts:
            for pt in bl[:: max(1, len(bl)//6) ]:
                d = min((sd(pt, u, v) for segs in segs_all for u, v in segs), default=1e9)
                worst = max(worst, d)
        ok = worst <= 0.75
        tot += 1
        fails += (not ok)
        print(f'{sub:10} residual={worst:7.2f}px -> {"FAIL" if not ok else "pass (CALIBRATION BUG)"}')
    verdict = "CALIBRATION HOLDS" if fails == tot and tot > 0 else "CALIBRATION BROKEN"
    print(f"\n{fails}/{tot} broken renders FAIL — {verdict}")
    sys.exit(0 if fails == tot else 1)

subjects = sorted(p.stem for p in SCANS.glob("*.json")) if args.subjects == "all" else args.subjects.split(",")
gate = json.loads((ROOT / "out/gate.json").read_text()) if (ROOT / "out/gate.json").exists() else {}
langs = LANGS if args.langs == "all" else args.langs.split(",")
rows = []
allpass = True
for sub in subjects:
    if gate.get(sub, {}).get("status") == "BLOCKED":
        print(f"{sub:10} BLOCKED (F4 gate) — skipped")
        continue
    doc = json.loads((SCANS / f"{sub}.json").read_text())
    tier = doc["tier"]
    for lang in langs:
        pool = json.loads((POOLS / f"{lang}-{tier}.json").read_text())
        pool += json.loads((POOLS / f"{lang}-easy.json").read_text())  # D4 borrow carried
        lines = [f"WORD {w.split('|')[0]} {units(w.split('|')[0])}" for w in pool if " " not in w]
        if lang in LEDGERED:
            rows.append({"subject": sub, "lang": lang, "ledgered": True})
            continue
        for seed in range(1, args.seeds + 1):
            j = None
            reseeds = 0
            # F3 re-seed ladder: a failing SEED re-seeds (cap 3) before the
            # subject fails typesettability.
            for attempt in range(3):
                eff = seed + attempt * 7777
                inp = [f"PARAMS {FLOOR} {BAND_MAX[tier]} {ADV.get(lang, 0.54)} {MAX_JUSTIFY} {eff}"]
                for e in doc["paths"]:
                    segs = " ".join(f"{t}" for t in e["segments"])
                    pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
                    inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} | {segs} | {pts}")
                r = subprocess.run([str(BIN)], input="\n".join(inp + lines).encode(),
                                   capture_output=True)
                j = json.loads(r.stdout)
                if "error" not in j:
                    break
                reseeds += 1
            if "error" in j:
                rows.append({"subject": sub, "lang": lang, "seed": seed, "error": j["error"],
                             "reseeds": reseeds})
                allpass = False
                continue
            residual, recall, precision = eval_baselines(doc, j["placements"])
            ok = residual <= 0.01 and recall >= 0.9999 and precision >= 0.9999
            allpass &= ok
            rows.append({"subject": sub, "lang": lang, "seed": seed, "residual": residual,
                         "recall": recall, "precision": precision, "pass": ok})
    subj_rows = [r for r in rows if r["subject"] == sub]
    bad = [r for r in subj_rows if not r.get("pass")]
    res = max((r.get("residual", 9) for r in subj_rows if "residual" in r), default=None)
    rc = min((r.get("recall", 0) for r in subj_rows if "recall" in r), default=None)
    print(f'{sub:10} runs={len(subj_rows)} bad={len(bad)} residual_max={res} recall_min={rc}')
if args.json:
    pathlib.Path(args.json).write_text(json.dumps(rows))
print("\nRESULT:", "PASS" if allpass else "NOT PASSING")
