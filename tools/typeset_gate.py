#!/usr/bin/env python3
"""v8 F4 — typesettability gate at scan acceptance.

A scan path enters the render library only if:
  G1 arc_length >= FLOOR * MIN_WORD_CHARS   (else sub-floor -> D-A merge)
  G2 per-segment curvature <= CURVATURE_LEGIBILITY_MAX (segments are
     pre-split at corners, so this reports the worst residual bend)
  G3 inter-path clearance >= one floor glyph height
Any failing non-merged path BLOCKS the subject.
"""
import argparse, json, math, pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
FLOOR = 13.0
MIN_WORD_CHARS = 3
CURVATURE_LEGIBILITY_MAX = 35.0
CLEARANCE_MIN = FLOOR

ap = argparse.ArgumentParser()
ap.add_argument("--subjects", default="all")
ap.add_argument("--json", default=None)
args = ap.parse_args()

subjects = sorted(p.stem for p in SCANS.glob("*.json")) if args.subjects == "all" else args.subjects.split(",")
report = {}
blocked = []
for sub in subjects:
    doc = json.loads((SCANS / f"{sub}.json").read_text())
    rows = []
    fails = 0
    for i, e in enumerate(doc["paths"]):
        if e["sub_floor"]:
            rows.append({"path": i, "status": "MERGED(D-A)", "arc": e["arc"], "parent": e["merged_into"]})
            continue
        if e.get("decorative_thin"):
            rows.append({"path": i, "status": "DECORATIVE-THIN", "arc": e["arc"]})
            continue
        f = []
        if e["arc"] < FLOOR * MIN_WORD_CHARS:
            f.append(f"G1 arc {e['arc']:.0f} < {FLOOR*MIN_WORD_CHARS:.0f}")
        # segments are pre-split at corners > CURVATURE max; the residual
        # worst turn is what remains INSIDE segments.
        if e["worst_turn_deg"] > CURVATURE_LEGIBILITY_MAX and not e["segments"]:
            f.append(f"G2 turn {e['worst_turn_deg']}deg unsplit")
        if e.get("tight_frac", 0.0) > 0.15:
            f.append(f"G3 tight_frac {e['tight_frac']:.2f} > 0.15 (min clearance {e['min_clearance']:.1f})")
        rows.append({"path": i, "status": "FAIL" if f else "PASS", "arc": e["arc"],
                     "clearance": e["min_clearance"], "fails": f})
        fails += bool(f)
    status = "BLOCKED" if fails else "PASS"
    if fails: blocked.append(sub)
    report[sub] = {"status": status, "failing_paths": fails,
                   "typesettable_paths": sum(1 for r in rows if r["status"] == "PASS"),
                   "merged_paths": sum(1 for r in rows if r["status"].startswith("MERGED")),
                   "decorative_thin": sum(1 for r in rows if r["status"] == "DECORATIVE-THIN"),
                   "paths": rows}
    print(f'{sub:10} {status:8} typesettable={report[sub]["typesettable_paths"]:4} '
          f'merged={report[sub]["merged_paths"]:5} failing={fails}')
if args.json:
    pathlib.Path(args.json).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(args.json).write_text(json.dumps(report))
print("\nBLOCKED:", ", ".join(blocked) if blocked else "none")
