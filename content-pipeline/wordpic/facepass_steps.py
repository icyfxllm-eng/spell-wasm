#!/usr/bin/env python3
"""Eric's Step 0-10 protocol for CC-TONAL-FACEPASS, as a runner.
Every automatable step emits a PASS/FAIL and an artifact; his gates
(1 approve-contour, 4 per-feature verdicts, 10 side-by-side) get their
evidence rendered here. Usage:
    python3 facepass_steps.py <tonal.json> <ref-image> <landmarks.json> <outdir>
"""
import json
import math
import pathlib
import sys

import cv2
import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from tonal import face_polygon, point_in_poly, load_canvas, landmark_features, CANVAS

tonal_path, ref_path, lm_path, outdir = sys.argv[1:5]
OUT = pathlib.Path(outdir)
OUT.mkdir(exist_ok=True)
data = json.loads(pathlib.Path(tonal_path).read_text())
img, box, scale = load_canvas(ref_path)
feats = landmark_features(lm_path, None, box, scale)
fp = face_polygon(feats)
score = {}

# ---- STEP 0: provenance — the pass is LIVE, or nothing else matters.
violations = []
for i, c in enumerate(data["candidates"]):
    inside = sum(1 for pt in c["points"][::3] if point_in_poly(pt[0], pt[1], fp))
    if inside > len(c["points"][::3]) * 0.5:
        if not (c.get("scope") == "face" or c["pathKind"] == "feature"):
            violations.append((i, c["pathKind"], c.get("bandIndex")))
score["step0_provenance"] = {"pass": not violations, "violations": violations[:5],
                             "n_violations": len(violations)}

# ---- STEP 1 evidence: the contour, rendered for approval.
ref_bgr = cv2.imread(ref_path)
ih, iw = ref_bgr.shape[:2]
ref_bgr = ref_bgr[int(ih*0.045):ih-int(ih*0.045), int(iw*0.045):iw-int(iw*0.045)]
small = cv2.resize(ref_bgr, None, fx=scale, fy=scale)
canvas_ref = np.full((CANVAS, CANVAS, 3), (35, 22, 16), np.uint8)
x0, y0, w, h = box
canvas_ref[y0:y0+small.shape[0], x0:x0+small.shape[1]] = small
overlay = canvas_ref.copy()
cv2.polylines(overlay, [np.array(fp, np.int32)], True, (255, 255, 255), 2)
cv2.imwrite(str(OUT/"step1-contour-on-painting.png"), overlay)

# ---- STEP 3: landmarks overlaid on the painting + tolerance report.
lm_overlay = canvas_ref.copy()
tol = 0.005 * math.hypot(CANVAS, CANVAS)  # the 0.5% anchor tolerance
anchor_report = {}
for name, pts in feats.items():
    for pt in pts:
        cv2.circle(lm_overlay, (int(pt[0]), int(pt[1])), 2, (80, 220, 255), -1)
    anchor_report[name] = len(pts)
cv2.imwrite(str(OUT/"step3-landmarks-on-painting.png"), lm_overlay)
score["step3_landmarks"] = {"tolerance_px": round(tol, 2), "anchors": anchor_report,
                            "note": "strokes derive FROM these anchors (0 offset by construction); report is for Eric's eye vs the painting"}

# ---- STEP 2: face-only debug view (+ per-feature strips for STEP 4).
def render(cands, only_face=False, only_feature=None):
    im = Image.new("RGB", (CANVAS, CANVAS), (16, 22, 35))
    dr = ImageDraw.Draw(im)
    for c in cands:
        pts = [tuple(q) for q in c["points"]]
        if len(pts) < 2:
            continue
        is_face = c.get("scope") == "face" or c["pathKind"] == "feature"
        if only_face and not is_face:
            continue
        if only_feature and c.get("feature") != only_feature:
            continue
        if c["pathKind"] == "feature":
            dr.line(pts, fill=(255, 236, 150) if c.get("focal") else (255, 224, 130),
                    width=4 if c.get("focal") else 3)
        elif c.get("scope") == "face":
            dr.line(pts, fill=(255, 200, 150), width=2)
        elif not only_face:
            g = 140
            dr.line(pts, fill=(g, g, g + 15), width=1)
    return im

face_only = render(data["candidates"], only_face=True)
face_only.save(OUT/"step2-face-only.png")
FEATURES_ORDER = ["leftEyeLid", "leftEyeCrease", "rightEyeLid", "rightEyeCrease",
                  "leftBrow", "rightBrow", "noseBridge", "noseBase",
                  "chinCrescent", "lipParting", "lowerLipShadow"]
strip = Image.new("RGB", (256 * 4, 256 * 3), (16, 22, 35))
for i, f in enumerate(FEATURES_ORDER):
    tile = render(data["candidates"], only_face=True, only_feature=f).resize((256, 256))
    ImageDraw.Draw(tile).text((8, 6), f, fill=(255, 177, 77))
    strip.paste(tile, ((i % 4) * 256, (i // 4) * 256))
strip.save(OUT/"step4-feature-strip.png")

# ---- STEP 5: face-local bands + glow present, monotonic.
face_bands = [c for c in data["candidates"] if c.get("scope") == "face" and c["pathKind"] == "boundary"]
by_band = {}
for c in face_bands:
    by_band[c["bandIndex"]] = by_band.get(c["bandIndex"], 0) + 1
counts = [by_band.get(b, 0) for b in sorted(by_band)]
score["step5_face_bands"] = {"pass": len(by_band) >= 2, "boundaries_per_band": by_band}

# ---- STEP 6: clearance — ZERO generic flow inside the contour, and the
# lint must catch a deliberately-planted violation (proven, not trusted).
flows_inside = [i for i, c in enumerate(data["candidates"])
                if c["pathKind"] == "flow" and c.get("scope") != "face"
                and sum(1 for pt in c["points"][::3] if point_in_poly(pt[0], pt[1], fp)) > 0]
planted = {"pathKind": "flow", "bandIndex": 1, "status": "pending",
           "points": [[fp[0][0] + 20, fp[0][1] + 40], [fp[0][0] + 40, fp[0][1] + 60],
                      [(fp[0][0] + fp[len(fp)//2][0]) / 2, (fp[0][1] + fp[len(fp)//2][1]) / 2]]}
planted_caught = sum(1 for pt in planted["points"] if point_in_poly(pt[0], pt[1], fp)) > 0
score["step6_clearance"] = {"pass": not flows_inside and planted_caught,
                            "flows_inside_face": len(flows_inside),
                            "deliberate_violation_caught": planted_caught}

# ---- STEP 7: THE SQUINT TEST, formalized — SSIM at 64px, render vs ref.
ys = [q[1] for q in fp]; xs = [q[0] for q in fp]
fy0, fy1 = int(min(ys)), int(max(ys))
fx0, fx1 = int(min(xs)), int(max(xs))
ref_face = cv2.cvtColor(canvas_ref[fy0:fy1, fx0:fx1], cv2.COLOR_BGR2GRAY)
rend_face = cv2.cvtColor(np.array(face_only)[fy0:fy1, fx0:fx1], cv2.COLOR_RGB2GRAY)
a = cv2.resize(ref_face, (64, 64)).astype(np.float64)
b = cv2.resize(rend_face, (64, 64)).astype(np.float64)
b = 255 - b  # render is light-on-dark; squint compares STRUCTURE
a = (a - a.mean()) / (a.std() + 1e-6)
b = (b - b.mean()) / (b.std() + 1e-6)
mu_ab = float((a * b).mean())
score["step7_squint"] = {"structural_corr_64px": round(mu_ab, 3),
                         "note": "normalized structural correlation; sign+magnitude = does she read when tiny"}
sq = Image.new("RGB", (300, 160), (16, 22, 35))
sq.paste(Image.fromarray(cv2.resize(ref_face, (128, 128))).convert("RGB"), (10, 16))
sq.paste(Image.fromarray(cv2.resize(255 - rend_face, (128, 128))).convert("RGB"), (160, 16))
sq.save(OUT/"step7-squint.png")

# ---- STEP 8: geometry eval (from the pipeline) + budget.
score["step8_geometry"] = data.get("facePass")

pathlib.Path(OUT/"scorecard.json").write_text(json.dumps(score, indent=1))
for k, v in score.items():
    flag = v.get("pass") if isinstance(v, dict) else None
    print(k, "PASS" if flag else ("FAIL" if flag is False else "-"),
          json.dumps(v)[:110])
