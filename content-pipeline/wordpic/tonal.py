#!/usr/bin/env python3
"""CC-MASTERPIECE-TONAL — the posterize-then-trace pipeline (T0, tool-side).

Paintings extract by TONE: luminance bands are the real information
structure, band boundaries are the only honest lines a painting has, and
form-following flow paths (structure tensor) do what brushstrokes did.
Words-as-shading is enforced monotonically: darker band => denser flow.

Usage:
    python3 tonal.py ref/mona-lisa.jpg --bands 5 --landmarks landmarks.json \
            --out mona-tonal.json --preview mona-tonal.png

Everything here is candidate material for Eric's keep/exclude review
(v7.3 semantics). References never ship; paths + band metadata do.
"""
import argparse
import json
import math
import pathlib
import subprocess
import tempfile

import cv2
import numpy as np
from PIL import Image, ImageDraw

CANVAS = 512
# Flow spacing per band rank (0 = darkest): the monotonic law as data.
FLOW_SPACING = [8, 12, 18, 26, 36, 48]
FLOW_STEP = 2.0
FLOW_MAX_LEN = 140


def load_canvas(path):
    img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    ih, iw = img.shape
    img = img[int(ih * 0.045):ih - int(ih * 0.045), int(iw * 0.045):iw - int(iw * 0.045)]
    img = cv2.createCLAHE(clipLimit=2.5, tileGridSize=(8, 8)).apply(img)
    img = cv2.medianBlur(img, 5)
    scale = (CANVAS - 44) / max(img.shape)
    small = cv2.resize(img, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA)
    pad = np.full((CANVAS, CANVAS), 255, np.uint8)
    y0 = (CANVAS - small.shape[0]) // 2
    x0 = (CANVAS - small.shape[1]) // 2
    pad[y0:y0 + small.shape[0], x0:x0 + small.shape[1]] = small
    return pad, (x0, y0, small.shape[1], small.shape[0]), scale


def bands_of(img, n, box):
    """Posterize into n luminance bands over the painting region only."""
    x0, y0, w, h = box
    inside = img[y0:y0 + h, x0:x0 + w]
    qs = np.quantile(inside, np.linspace(0, 1, n + 1)[1:-1])
    idx = np.digitize(img.astype(float), qs)  # 0 = darkest
    mask_outside = np.ones_like(img, bool)
    mask_outside[y0:y0 + h, x0:x0 + w] = False
    idx[mask_outside] = n  # outside the painting: no band
    return idx, qs


def potrace_polys(mask):
    with tempfile.TemporaryDirectory() as td:
        pbm = pathlib.Path(td) / "b.pbm"
        svg = pathlib.Path(td) / "b.svg"
        h, w = mask.shape
        pbm.write_bytes(f"P4\n{w} {h}\n".encode() + np.packbits(mask > 0, axis=1).tobytes())
        subprocess.run(["potrace", str(pbm), "-s", "-o", str(svg), "--turdsize", "24",
                        "--alphamax", "1.1", "--opttolerance", "0.5"],
                       check=True, capture_output=True)
        return parse_potrace_svg(svg.read_text(), w, h)


def parse_potrace_svg(text, w, h):
    import re
    m = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', text)
    sx = w / float(m.group(1)) if m else 1.0
    sy = h / float(m.group(2)) if m else 1.0
    tr = re.search(r'translate\(([-\d.]+),([-\d.]+)\)\s*scale\(([-\d.]+),([-\d.]+)\)', text)
    token_re = re.compile(r"([MmCcLlZz])|(-?[\d.]+)")
    polys = []
    for pm in re.finditer(r'd="([^"]+)"', text):
        toks = [(t.group(1), t.group(2)) for t in token_re.finditer(pm.group(1))]
        i, cmd, cx, cy, cur = 0, None, 0.0, 0.0, []
        def flush(c):
            if len(c) > 4:
                polys.append(c[::2] + [c[-1]])
        while i < len(toks):
            letter, num = toks[i]
            if letter:
                cmd = letter
                if cmd in "Zz":
                    if cur:
                        cur.append(cur[0])
                    flush(cur)
                    cur = []
                i += 1
                continue
            if cmd in "Mm":
                x, y = float(num), float(toks[i + 1][1])
                if cmd == "m":
                    x, y = cx + x, cy + y
                cx, cy = x, y
                flush(cur)
                cur = [(cx, cy)]
                i += 2
                cmd = "l" if cmd == "m" else "L"
            elif cmd in "Ll":
                x, y = float(num), float(toks[i + 1][1])
                if cmd == "l":
                    x, y = cx + x, cy + y
                cx, cy = x, y
                cur.append((cx, cy))
                i += 2
            elif cmd in "Cc":
                x, y = float(toks[i + 4][1]), float(toks[i + 5][1])
                if cmd == "c":
                    x, y = cx + x, cy + y
                cx, cy = x, y
                cur.append((cx, cy))
                i += 6
            else:
                i += 1
        flush(cur)
    if tr:
        tx, ty, s1, s2 = (float(tr.group(k)) for k in range(1, 5))
        polys = [[((x * s1 + tx) * sx, (y * s2 + ty) * sy) for x, y in poly] for poly in polys]
    return polys


def structure_tensor(img, sigma=6):
    gx = cv2.Sobel(img, cv2.CV_32F, 1, 0, ksize=3)
    gy = cv2.Sobel(img, cv2.CV_32F, 0, 1, ksize=3)
    Jxx = cv2.GaussianBlur(gx * gx, (0, 0), sigma)
    Jyy = cv2.GaussianBlur(gy * gy, (0, 0), sigma)
    Jxy = cv2.GaussianBlur(gx * gy, (0, 0), sigma)
    # dominant orientation, ROTATED 90° = the flow (along the form, not
    # across it — drapery folds and hair waves come from exactly this)
    theta = 0.5 * np.arctan2(2 * Jxy, Jxx - Jyy) + math.pi / 2
    coherence = np.sqrt((Jxx - Jyy) ** 2 + 4 * Jxy ** 2) / (Jxx + Jyy + 1e-6)
    return theta, coherence


def flow_paths(band_idx, band, theta, coh, spacing):
    """Streamlines through one band, seeded on a grid, spaced per the
    monotonic density law."""
    H, W = band_idx.shape
    mask = band_idx == band
    taken = np.zeros_like(mask, bool)
    out = []
    for sy in range(24, H - 24, spacing):
        for sx in range(24, W - 24, spacing):
            if not mask[sy, sx] or taken[sy, sx] or coh[sy, sx] < 0.08:
                continue
            pts = []
            for direction in (1, -1):
                x, y = float(sx), float(sy)
                seg = []
                for _ in range(int(FLOW_MAX_LEN / FLOW_STEP)):
                    xi, yi = int(x), int(y)
                    if not (0 <= xi < W and 0 <= yi < H) or not mask[yi, xi]:
                        break
                    seg.append((x, y))
                    a = theta[yi, xi]
                    x += direction * FLOW_STEP * math.cos(a)
                    y += direction * FLOW_STEP * math.sin(a)
                pts = seg[::-1] + pts if direction == -1 else pts + seg
            if len(pts) > 6:
                out.append(pts[::2])
                for px, py in pts:
                    xi, yi = int(px), int(py)
                    y0c, y1c = max(0, yi - spacing // 2), min(H, yi + spacing // 2)
                    x0c, x1c = max(0, xi - spacing // 2), min(W, xi + spacing // 2)
                    taken[y0c:y1c, x0c:x1c] = True
    return out


def landmark_features(lm_path, src_shape_before_resize, box, scale):
    """Map Vision landmark pixel coords into canvas space and name them
    per the requiredFeatures vocabulary. Suggestion-only candidates."""
    lm = json.loads(pathlib.Path(lm_path).read_text())
    if lm.get("faces") != 1:
        return {}
    # landmarks were detected on the ORIGINAL file; ref cropping used the
    # same 4.5% inset — map: subtract inset, then * scale, + pad.
    iw, ih = lm["imageW"], lm["imageH"]
    ix, iy = int(iw * 0.045), int(ih * 0.045)
    x0, y0, _, _ = box
    def mapped(name):
        return [((p[0] - ix) * scale + x0, (p[1] - iy) * scale + y0) for p in lm.get(name, [])]
    feats = {
        "leftEye": mapped("leftEye"), "rightEye": mapped("rightEye"),
        "leftBrow": mapped("leftBrow"), "rightBrow": mapped("rightBrow"),
        "nose": mapped("nose") + mapped("noseCrest"),
        "mouth": mapped("mouth"),
        "chin": mapped("faceContour"),
    }
    return {k: v for k, v in feats.items() if len(v) >= 3}




# ---------------- CC-TONAL-FACEPASS ----------------
# The face is its own picture: ~8% of canvas, ~80% of recognition.
# Face-local bands, iconic strokes (a CLOSED vocabulary — landmarks
# position strokes, they never become them), flow suppression on skin,
# clearance zones, a hard path budget, and the smile as THE focal stroke
# (export blocks until explicitly touched — D4).

FACE_BANDS_RANGE = (3, 4)          # D1
FACE_CLEARANCE = 7.0                # registry config: one number
FACE_PATH_BUDGET = 28               # registry config default

def _poly_from(pts):
    return np.array(pts, np.int32)

def face_polygon(feats):
    """The approved face contour closes over the brow line — the raw
    bbox is explicitly NOT the boundary."""
    contour = feats.get("chin", [])
    brows = feats.get("leftBrow", []) + feats.get("rightBrow", [])
    if not contour or not brows:
        return None
    top = min(y for _, y in brows) - 14
    xs = [pt[0] for pt in contour]
    left, right = min(xs), max(xs)
    poly = [(left, top)] + sorted(contour, key=lambda q: q[0]) + [(right, top)]
    return poly

def point_in_poly(x, y, poly):
    inside = False
    j = len(poly) - 1
    for i in range(len(poly)):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi + 1e-9) + xi:
            inside = not inside
        j = i
    return inside

def clip_outside_poly(poly_path, face_poly):
    """Point-wise knife: global paths TERMINATE at the face boundary."""
    out, cur = [], []
    for pt in poly_path:
        if point_in_poly(pt[0], pt[1], face_poly):
            if len(cur) > 2:
                out.append(cur)
            cur = []
        else:
            cur.append(pt)
    if len(cur) > 2:
        out.append(cur)
    return out

def _upper_arc(eye_pts):
    """Upper-lid stroke: the top edge of the eye points, left to right.
    NEVER a closed loop — an eye rendered as a polygon reads as a hole."""
    pts = sorted(eye_pts, key=lambda q: q[0])
    cy = sum(q[1] for q in pts) / len(pts)
    top = [q for q in pts if q[1] <= cy + 1]
    return top if len(top) >= 3 else pts[: max(3, len(pts) // 2)]

def _offset(path, dx, dy):
    return [(x + dx, y + dy) for x, y in path]

def iconic_strokes(feats, img):
    """The closed vocabulary (D2). Landmarks position; Eric approves."""
    strokes = []
    def add(name, pts, focal=False):
        if len(pts) >= 2:
            strokes.append({"pathKind": "feature", "feature": name,
                            "iconic": True, "focal": focal,
                            "bandIndex": None, "scope": "face",
                            "status": "pending", "points": pts})
    for side in ("left", "right"):
        eye = feats.get(f"{side}Eye", [])
        if eye:
            lid = _upper_arc(eye)
            h = max(q[1] for q in eye) - min(q[1] for q in eye)
            add(f"{side}EyeLid", lid)
            add(f"{side}EyeCrease", _offset(lid, 0, -max(3.0, h * 0.5)))
        brow = feats.get(f"{side}Brow", [])
        if brow:
            add(f"{side}Brow", sorted(brow, key=lambda q: q[0]))
    nose = feats.get("nose", [])
    if nose:
        xs = sorted(nose, key=lambda q: q[0])
        mid_x = (xs[0][0] + xs[-1][0]) / 2
        # shadow side: darker mean luminance beside the bridge
        l_lum = float(np.mean([img[int(q[1]), int(q[0]) - 6] for q in nose if 0 < q[0] - 6]))
        r_lum = float(np.mean([img[int(q[1]), int(q[0]) + 6] for q in nose if q[0] + 6 < img.shape[1]]))
        shadow = [q for q in nose if (q[0] < mid_x) == (l_lum < r_lum)]
        add("noseBridge", sorted(shadow, key=lambda q: q[1]))
        base = sorted(nose, key=lambda q: -q[1])[: max(3, len(nose) // 3)]
        add("noseBase", sorted(base, key=lambda q: q[0]))
    mouth = feats.get("mouth", [])
    if mouth:
        pts = sorted(mouth, key=lambda q: q[0])
        # lip parting: x-bucketed vertical midpoints of the outer loop
        buckets = {}
        for x, y in pts:
            buckets.setdefault(round(x / 4), []).append(y)
        parting = [(k * 4.0, (min(v) + max(v)) / 2) for k, v in sorted(buckets.items())]
        add("lipParting", parting, focal=True)   # THE focal stroke (F3)
        bottom = [q for q in pts if q[1] > sum(y for _, y in pts) / len(pts)]
        add("lowerLipShadow", _offset(sorted(bottom, key=lambda q: q[0]), 0, 3))
    chin = feats.get("chin", [])
    if chin:
        c = sorted(chin, key=lambda q: q[0])
        third = max(3, len(c) // 3)
        add("chinCrescent", c[third:-third] if len(c) > 2 * third + 2 else c)
    return strokes

def de_light(img, face_poly):
    """F0 — the de-light pre-pass (Eric, 2026-08-04: "sign the de-light
    pass"). Single-scale retinex over the face: divide out the
    illumination field (face-scale Gaussian), keep reflectance. Bands
    computed on reflectance follow ANATOMY through shadow — the eye
    socket's eye, not the eye socket's shadow. Deterministic, no ML,
    nothing generated: it is division by a blur."""
    xs = [q[0] for q in face_poly]
    ys = [q[1] for q in face_poly]
    fx0, fy0 = int(max(0, min(xs))), int(max(0, min(ys)))
    fx1 = int(min(img.shape[1], max(xs)))
    fy1 = int(min(img.shape[0], max(ys)))
    out = img.copy()
    face = img[fy0:fy1, fx0:fx1].astype(np.float64) + 1.0
    if face.size == 0:
        return out
    sigma = max(face.shape) / 3.0
    illum = cv2.GaussianBlur(face, (0, 0), sigma)
    refl = np.log(face) - np.log(illum + 1.0)
    # normalize over the FACE POLYGON only — bbox corners are background
    # and drag the range, which painted a seam at the face edge
    poly_mask = np.zeros(img.shape, np.uint8)
    cv2.fillPoly(poly_mask, [np.array(face_poly, np.int32)], 1)
    pm = poly_mask[fy0:fy1, fx0:fx1] == 1
    vals = refl[pm]
    lo, hi = np.quantile(vals, 0.02), np.quantile(vals, 0.98)
    refl = np.clip((refl - lo) / (hi - lo + 1e-9) * 255, 0, 255).astype(np.uint8)
    patch = out[fy0:fy1, fx0:fx1]
    patch[pm] = cv2.medianBlur(refl, 3)[pm]
    out[fy0:fy1, fx0:fx1] = patch
    return out


def face_local_bands(img, face_poly, n):
    """Nested re-posterize over the face only — ON REFLECTANCE (F0,
    signed): skin-tone range is a narrow slice the global thresholds
    cannot see, and lighting is not anatomy."""
    img = de_light(img, face_poly)
    assert FACE_BANDS_RANGE[0] <= n <= FACE_BANDS_RANGE[1]
    ys = [q[1] for q in face_poly]; xs = [q[0] for q in face_poly]
    y0b, y1b = int(max(0, min(ys))), int(min(img.shape[0], max(ys)))
    x0b, x1b = int(max(0, min(xs))), int(min(img.shape[1], max(xs)))
    mask = np.zeros(img.shape, np.uint8)
    cv2.fillPoly(mask, [_poly_from(face_poly)], 1)
    vals = img[(mask == 1)]
    qs = np.quantile(vals, np.linspace(0, 1, n + 1)[1:-1])
    boundaries = []
    for b, q in enumerate(qs):
        m = ((img <= q) & (mask == 1)).astype(np.uint8)
        m = cv2.morphologyEx(m, cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
        for poly in potrace_polys(m):
            boundaries.append({"pathKind": "boundary", "bandIndex": b,
                               "scope": "face", "status": "pending",
                               "points": poly})
    return boundaries

def near_any_stroke(pt, strokes, margin):
    for st in strokes:
        for (ax, ay) in st["points"][::2]:
            if (pt[0] - ax) ** 2 + (pt[1] - ay) ** 2 <= margin * margin:
                return True
    return False

def geometry_eval(strokes, feats):
    """The face must become a number before it becomes an on-device
    disappointment. Per-feature report; export gates on it."""
    rep = {}
    le, re_ = feats.get("leftEye", []), feats.get("rightEye", [])
    contour = feats.get("chin", [])
    mouth = feats.get("mouth", [])
    if le and re_:
        lc = (sum(q[0] for q in le) / len(le), sum(q[1] for q in le) / len(le))
        rc = (sum(q[0] for q in re_) / len(re_), sum(q[1] for q in re_) / len(re_))
        rep["eyeLineTiltDeg"] = math.degrees(math.atan2(rc[1] - lc[1], rc[0] - lc[0]))
        if contour:
            fw = max(q[0] for q in contour) - min(q[0] for q in contour)
            rep["eyeSpacingToFaceWidth"] = math.hypot(rc[0] - lc[0], rc[1] - lc[1]) / fw
    if mouth and contour:
        mw = max(q[0] for q in mouth) - min(q[0] for q in mouth)
        jw = max(q[0] for q in contour) - min(q[0] for q in contour)
        rep["mouthToJawWidth"] = mw / jw
    rep["focalTouched"] = False  # D4: flips ONLY on Eric's explicit touch
    return rep


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ref")
    ap.add_argument("--bands", type=int, default=5)
    ap.add_argument("--landmarks")
    ap.add_argument("--out", required=True)
    ap.add_argument("--preview")
    args = ap.parse_args()
    assert 4 <= args.bands <= 6, "D1: band count is 4-6, chosen per piece"

    img, box, scale = load_canvas(args.ref)
    band_idx, qs = bands_of(img, args.bands, box)
    theta, coh = structure_tensor(img)

    candidates = []
    # band boundaries: the honest lines
    for b in range(args.bands - 1):
        mask = (band_idx <= b).astype(np.uint8)
        mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
        for poly in potrace_polys(mask):
            candidates.append({"pathKind": "boundary", "bandIndex": b,
                               "status": "pending", "points": poly})
    # band interiors: form-following flow, monotonic density
    for b in range(args.bands):
        for pts in flow_paths(band_idx, b, theta, coh, FLOW_SPACING[b]):
            candidates.append({"pathKind": "flow", "bandIndex": b,
                               "status": "pending", "points": pts})
    # landmark feature paths (suggestion-only; Eric approves each)
    features = {}
    if args.landmarks:
        features = landmark_features(args.landmarks, None, box, scale)
        for name, pts in features.items():
            candidates.append({"pathKind": "feature", "feature": name,
                               "bandIndex": None, "status": "pending", "points": pts})

    # ---- CC-TONAL-FACEPASS (auto for any TONAL subject with a face) ----
    face_report = None
    if features:
        fp = face_polygon(features)
        if fp:
            # global paths TERMINATE at the face contour
            clipped = []
            for c in candidates:
                if c["pathKind"] in ("boundary", "flow"):
                    for piece in clip_outside_poly(c["points"], fp):
                        clipped.append({**c, "points": piece})
                elif c["pathKind"] == "feature":
                    continue  # raw landmark loops are replaced by iconic strokes
                else:
                    clipped.append(c)
            candidates = clipped
            strokes = iconic_strokes(features, img)
            face_bounds = face_local_bands(img, fp, 3)
            # clearance: no generic path near a feature stroke
            kept = []
            dropped_clearance = 0
            for c in candidates:
                if c["pathKind"] == "flow" and any(
                    near_any_stroke(pt, strokes, FACE_CLEARANCE) for pt in c["points"][::3]
                ):
                    dropped_clearance += 1
                    continue
                kept.append(c)
            candidates = kept + face_bounds + strokes
            n_face = len(face_bounds) + len(strokes)
            assert n_face <= FACE_PATH_BUDGET, f"face budget: {n_face} > {FACE_PATH_BUDGET}"
            face_report = geometry_eval(strokes, features)
            face_report["facePaths"] = n_face
            face_report["clearanceDrops"] = dropped_clearance
            print("facepass:", json.dumps(face_report))

    # F4 (POLISH, signed): the smile is the last word — the focal path
    # carries the piece's MAX order, so the existing in-order word engine
    # lands its word last with zero app logic. FINALE replay inherits the
    # climax because replay is assembly-ordered.
    max_order = len(candidates) + 1
    for c in candidates:
        if c.get("focal"):
            c["exportOrder"] = max_order
        else:
            c["exportOrder"] = None  # assigned sequentially at export

    # the glow: lightest band tagged highlight (required for portraits)
    out = {
        "ref": str(args.ref), "bands": args.bands,
        "bandQuantiles": [float(q) for q in qs],
        "highlightBand": args.bands - 1,
        "features": sorted(features),
        "candidates": candidates,
        "facePass": face_report,
    }
    pathlib.Path(args.out).write_text(json.dumps(out))
    n_by = {}
    for c in candidates:
        k = f'{c["pathKind"]}{"" if c["bandIndex"] is None else c["bandIndex"]}'
        n_by[k] = n_by.get(k, 0) + 1
    print("candidates:", n_by)

    if args.preview:
        im = Image.new("RGB", (CANVAS, CANVAS), (16, 22, 35))
        dr = ImageDraw.Draw(im)
        band_grey = [230, 205, 175, 140, 110, 90]
        for c in candidates:
            pts = [tuple(q) for q in c["points"]]
            if len(pts) < 2:
                continue
            if c["pathKind"] == "flow":
                g = band_grey[c["bandIndex"]]
                dr.line(pts, fill=(g, g + 4, min(255, g + 18)), width=1 if c["bandIndex"] >= args.bands - 2 else 2)
        for c in candidates:
            pts = [tuple(q) for q in c["points"]]
            if len(pts) < 2:
                continue
            if c["pathKind"] == "boundary":
                if c.get("scope") == "face":
                    dr.line(pts, fill=(255, 200, 150), width=2)  # face bands: warm
                else:
                    dr.line(pts, fill=(200, 208, 226), width=2)
        for c in candidates:
            pts = [tuple(q) for q in c["points"]]
            if len(pts) < 2:
                continue
            if c["pathKind"] == "feature":
                w = 4 if c.get("focal") else 3
                dr.line(pts, fill=(255, 236, 150) if c.get("focal") else (255, 224, 130), width=w)
        x0, y0, w, h = box
        dr.rectangle([x0 + 2, y0 + 2, x0 + w - 2, y0 + h - 2], outline=(238, 242, 250), width=3)
        im.save(args.preview)
        print("preview:", args.preview)


if __name__ == "__main__":
    main()
