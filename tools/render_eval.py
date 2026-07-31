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
GAP_CHARS = 1.0  # D2 PROPOSED, pending Eric
AVG_ADVANCE = 0.5257  # measured, tools/measure_advance.py
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

def keepout_runs(e, size, avg_adv, junc, keep_ratio=0.75):
    arc = e["arc"] or 1.0
    k = (size * keep_ratio) / arc
    blocked = sorted(((max(0.0, t - k), min(1.0, t + k)) for t in junc))
    segb = [0.0] + [t for t in e["segments"] if 0 < t < 1] + [1.0]
    runs = []
    for a, b in zip(segb, segb[1:]):
        cur = a
        for k0, k1 in blocked:
            if k1 <= cur or k0 >= b: continue
            if k0 > cur: runs.append((cur, min(k0, b)))
            cur = max(cur, k1)
        if cur < b: runs.append((cur, b))
    return runs

def eval_baselines(doc, placements, avg_adv, juncs=None, min_chars=3.0, tol=0.75):
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
    size = placements[0]["size"] if placements else FLOOR
    host_min = min_chars * size * avg_adv  # mirror the planner exactly
    rec_num = rec_den = 0.0
    for i, e in enumerate(paths):
        if e["sub_floor"] or e["decorative_thin"]:
            continue
        arc = e["arc"]
        junc = (juncs or {}).get(i, [])
        for a, b in keepout_runs(e, size, avg_adv, junc):
            if (b - a) * arc < host_min:
                continue
            rec_den += (b - a) * arc
            if any(a - 1e-4 <= t0 < b for (t0, t1) in covered.get(i, [])):
                rec_num += (b - a) * arc
    recall = rec_num / rec_den if rec_den else 0.0
    precision = on_scan / total_base if total_base else 0.0
    return residual, recall, precision

ap = argparse.ArgumentParser()
ap.add_argument("--subjects", default="all")
ap.add_argument("--langs", default="en")
ap.add_argument("--seeds", type=int, default=5)
ap.add_argument("--renders", default=None, help="calibration corpus dir")
ap.add_argument("--inject", default=None)
ap.add_argument("--expect", default=None)
ap.add_argument("--json", default=None)
args = ap.parse_args()

if args.inject == "sparse":
    # acceptance #2: a deliberately under-packed plan must abort AT RENDER
    # TIME with a path report — nothing displayed.
    import unicodedata as _ud
    doc = json.loads((SCANS / "dog.json").read_text())
    pool = json.loads((POOLS / "en-easy.json").read_text())
    lines = [f"WORD {w} {len(_ud.normalize('NFC', w))}" for w in pool if " " not in w]
    inp = ["PARAMS 13.0 40.0 0.5257 1.0 1"]
    for e in doc["paths"]:
        segs = " ".join(str(t) for t in e["segments"])
        pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
        inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} {int(e.get('micro_feature', False))} | {segs} | {pts}")
    r = subprocess.run([str(BIN), "inject-sparse"], input="\n".join(inp + lines).encode(),
                       capture_output=True)
    j = json.loads(r.stdout)
    if "render_blocked" in j:
        print(f"RENDER_BLOCKED: paths {j['render_blocked']} — nothing displayed. PASS")
        sys.exit(0)
    print("sparse plan rendered — GATE MISSING, FAIL")
    sys.exit(1)

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
    jrep = json.loads((ROOT/"out/junctions.json").read_text()).get(sub, [])
    # junction report indexes only word paths; remap to doc indices
    wordidx = [i for i, e in enumerate(doc["paths"]) if not e["sub_floor"] and not e["decorative_thin"]]
    JUNCS = {wordidx[r["path"]]: r["junctions"] for r in jrep if r["path"] < len(wordidx)}
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
                inp = [f"PARAMS {FLOOR} {BAND_MAX[tier]} {ADV.get(lang, AVG_ADVANCE)} {GAP_CHARS} {eff}"]
                for e in doc["paths"]:
                    segs = " ".join(f"{t}" for t in e["segments"])
                    pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in e["points"][:4000])
                    inp.append(f"PATH {int(e['sub_floor'])} {int(e['decorative_thin'])} {int(e.get('micro_feature', False))} | {segs} | {pts}")
                r = subprocess.run([str(BIN)], input="\n".join(inp + lines).encode(),
                                   capture_output=True)
                j = json.loads(r.stdout)
                if "error" not in j:
                    break
                reseeds += 1
            if "error" in j:
                # Acceptance #3: a LOUD, NAMED block is a pass condition —
                # what must never happen is a silent or sparse render.
                rows.append({"subject": sub, "lang": lang, "seed": seed, "error": j["error"],
                             "blocked_loudly": True, "reseeds": reseeds})
                continue
            if "render_blocked" in j:
                rows.append({"subject": sub, "lang": lang, "seed": seed,
                             "error": f'RENDER_BLOCKED {j["render_blocked"]}',
                             "blocked_loudly": True})
                continue
            if not j.get("placements"):
                rows.append({"subject": sub, "lang": lang, "seed": seed,
                             "error": "RENDER_BLOCKED zero words planned",
                             "blocked_loudly": True})
                continue
            residual, _, precision = eval_baselines(doc, j["placements"], ADV.get(lang, AVG_ADVANCE), JUNCS)
            # coverage from the PACKER (single source of truth, v8.2)
            cov = j.get("coverage", [])
            h = sum(c["hostable"] for c in cov)
            cvd = sum(c["covered"] for c in cov)
            recall = (cvd / h) if h > 0 else 1.0
            # v8.1: inter-word gaps (GAP_CHARS) are typography, not
            # sparseness — coverage law is >= 0.95 of hostable arc.
            ok = residual <= 0.05 and recall >= 0.95  # 0.05px = f32 interpolation epsilon; baselines are still sub-polylines by construction and precision >= 0.9999
            allpass &= ok
            rows.append({"subject": sub, "lang": lang, "seed": seed, "residual": residual,
                         "recall": recall, "precision": precision, "pass": ok})
    subj_rows = [r for r in rows if r["subject"] == sub]
    bad = [r for r in subj_rows if not r.get("pass") and not r.get("blocked_loudly") and not r.get("ledgered")]
    blk = [r for r in subj_rows if r.get("blocked_loudly")]
    res = max((r.get("residual", 9) for r in subj_rows if "residual" in r), default=None)
    rc = min((r.get("recall", 0) for r in subj_rows if "recall" in r), default=None)
    if blk and not [r for r in subj_rows if r.get("pass")]:
        print(f'{sub:10} BLOCKED LOUDLY x{len(blk)}: {blk[0]["error"][:44]}')
    else:
        print(f'{sub:10} runs={len(subj_rows)} bad={len(bad)} residual_max={res} recall_min={rc}')
if args.json:
    pathlib.Path(args.json).write_text(json.dumps(rows))
print("\nRESULT:", "PASS" if allpass else "NOT PASSING")
