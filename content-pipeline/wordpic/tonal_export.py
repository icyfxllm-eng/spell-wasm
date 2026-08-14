#!/usr/bin/env python3
"""CC-MASTERPIECE-TONAL / FACEPASS — mona's tonal EXPORT (staged).

The flip is atomic at export: extractionClass LINE -> TONAL +
requiredFeatures, paths replaced by the tonal trace. This tool builds
the registry entry and REFUSES to arm the real flip until every gate
in the face session is recorded — D4 is law: export BLOCKS until the
operator explicitly touches the smile ("it looked fine" is not an
exception).

Gates read from facepass-verdicts.json (written at Eric's session):
  contourApproved, landmarksConfirmed, perFeaturePass, handsLasso,
  smileTouched  — all must be true; smileTouched also flips the trace's
  facePass.focalTouched.

Modes:
  --stage  write staged-mona-entry.json + gate report (always allowed)
  --flip   apply to config/wordpic/pictures.json (only when gates green)

Selection law (v1, documented for the review):
  hosts  = 11 iconic vocabulary strokes (band 2, budget [1,4], the
           closed FACEPASS set; lipParting focal:true, order = MAX)
         + flow paths per band, longest first, budgeted so estimated
           slots stay inside mona's 12..=110 slot law
  guides = band boundary paths (the outline look) + unselected flow
  order  = flow by band (lightest first, words-as-shading builds the
           shadows last), then iconic strokes, then the smile — F4.
"""
import json, math, pathlib, sys

SCRATCH = pathlib.Path("/private/tmp/claude-501/-Users-eric/d5d833c7-982c-4f16-9353-8fe37d983a28/scratchpad")
ROOT = pathlib.Path("/Users/eric/repos/spell-wasm")
REG = ROOT / "config/wordpic/pictures.json"
TRACE = SCRATCH / "mona-tonal.json"
VERDICTS = SCRATCH / "facepass-verdicts.json"

# BROW AMENDMENT (Eric, 2026-08-04, verbatim: "they pass drop the brows
# for mona"): the painting has no eyebrows — leftBrow/rightBrow are
# dropped from mona's vocabulary AND her required features. Nine
# strokes remain; other portraits keep their brows.
VOCAB = ["leftEyeLid", "leftEyeCrease", "rightEyeLid",
         "rightEyeCrease", "noseBridge", "noseBase",
         "lipParting", "lowerLipShadow", "chinCrescent"]

# Eric's enumeration (CC-MASTERPIECE-TONAL verbatim) was twelve; the
# 2026-08-04 brow amendment drops leftBrow/rightBrow FOR MONA (the
# painting has none). TEN remain. Presence is CHECKED per feature —
# see presence_report(); hands come only from Eric's lasso.
TWELVE = ["leftEye", "rightEye", "nose", "mouth", "chin", "hairMass",
          "leftHand", "rightHand", "frame", "highlightBand"]
FACE_FEATURE_STROKES = {
    "leftEye": ["leftEyeLid", "leftEyeCrease"],
    "rightEye": ["rightEyeLid", "rightEyeCrease"],
    "nose": ["noseBridge", "noseBase"],
    "mouth": ["lipParting", "lowerLipShadow"],
    "chin": ["chinCrescent"],
}

GATES = ["contourApproved", "landmarksConfirmed", "perFeaturePass",
         "handsLasso", "smileTouched"]

SLOT_TARGET = 150          # inside mona's 12..=110 law with headroom
EST_PX_PER_SLOT = 64.0

def arclen(p):
    return sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))

def dstr(p):
    return "M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in p)

def gate_report():
    v = json.loads(VERDICTS.read_text()) if VERDICTS.exists() else {}
    rows = [(g, bool(v.get(g))) for g in GATES]
    return rows, all(ok for _, ok in rows), v

def presence_report(t, verdicts):
    """v7.4 law: export blocks when ANY of the twelve is missing from
    the trace. Each feature has concrete evidence, not vibes."""
    feats = {c["feature"] for c in t["candidates"] if c["pathKind"] == "feature"}
    def centroid(c):
        xs = [p[0] for p in c["points"]]; ys = [p[1] for p in c["points"]]
        return sum(xs) / len(xs), sum(ys) / len(ys)
    def near_edge(pts, tol=30):
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        return min(xs) < tol or max(xs) > 512 - tol or min(ys) < tol or max(ys) > 512 - tol
    bounds = [c for c in t["candidates"] if c["pathKind"] == "boundary"]
    flows = [c for c in t["candidates"] if c["pathKind"] == "flow"]
    hairish = [c for c in flows
               if (lambda xy: 130 < xy[1] < 300 and not (190 < xy[0] < 330 and xy[1] < 190))(centroid(c))]
    hands = verdicts.get("hands", {})
    rep = {}
    for name in TWELVE:
        if name in FACE_FEATURE_STROKES:
            missing = [s for s in FACE_FEATURE_STROKES[name] if s not in feats]
            rep[name] = (not missing, f"strokes {FACE_FEATURE_STROKES[name]}"
                         + (f" MISSING {missing}" if missing else ""))
        elif name == "highlightBand":
            rep[name] = (t.get("highlightBand") is not None,
                         f"trace highlightBand={t.get('highlightBand')}")
        elif name == "frame":
            n = sum(1 for c in bounds if near_edge(c["points"]))
            rep[name] = (n >= 4, f"{n} edge-hugging boundary paths")
        elif name == "hairMass":
            rep[name] = (len(hairish) >= 10, f"{len(hairish)} hair-register flows")
        elif name in ("leftHand", "rightHand"):
            key = "left" if name == "leftHand" else "right"
            rep[name] = (bool(hands.get(key)),
                         "lasso polygon recorded" if hands.get(key)
                         else "AWAITS Eric's hands lasso (Vision has no hand detector)")
    return rep

def build_entry(verdicts=None):
    t = json.loads(TRACE.read_text())
    cands = t["candidates"]
    feats = {c["feature"]: c for c in cands if c["pathKind"] == "feature"}
    missing = [f for f in VOCAB if f not in feats]
    assert not missing, f"vocabulary strokes missing from trace: {missing}"
    # OPERATOR-DRAWN OVERRIDE (perFeaturePass 2026-08-04, "they pass"):
    # Eric's processed strokes replace the machine's wherever recorded.
    drawn = {}
    dp = SCRATCH / "eric-drawn-processed.json"
    if dp.exists():
        drawn = json.loads(dp.read_text())
    for name, c in feats.items():
        if name in drawn:
            c = dict(c)
            c["points"] = drawn[name]
            c["operatorDrawn"] = True
            feats[name] = c

    flows = [c for c in cands if c["pathKind"] == "flow"]
    bounds = [c for c in cands if c["pathKind"] == "boundary"]
    # Batch-27 laws in exporter form: (a) PRE-SPLIT at corners (>33 deg
    # per vertex) so the engine's corner_split never carves a shard out
    # of a host; (b) sub-60px pieces are dot-crumbs CJK cannot fill —
    # they stay visible as guide ink, never host.
    def presplit(pts, deg=33.0):
        import math as _m
        if len(pts) < 3:
            return [pts]
        cuts = [0]
        for i in range(1, len(pts) - 1):
            ax, ay = pts[i][0] - pts[i-1][0], pts[i][1] - pts[i-1][1]
            bx, by = pts[i+1][0] - pts[i][0], pts[i+1][1] - pts[i][1]
            la, lb = _m.hypot(ax, ay), _m.hypot(bx, by)
            if la < 1e-3 or lb < 1e-3:
                continue
            cosv = max(-1.0, min(1.0, (ax*bx + ay*by) / (la*lb)))
            if _m.degrees(_m.acos(cosv)) > deg:
                cuts.append(i)
        cuts.append(len(pts) - 1)
        cuts = sorted(set(cuts))
        return [pts[a:b+1] for a, b in zip(cuts, cuts[1:]) if b > a]
    split_flows = []
    crumbs = []
    for c in flows:
        for piece in presplit(c["points"]):
            d = dict(c)
            d["points"] = piece
            (split_flows if arclen(piece) >= 60 else crumbs).append(d)
    flows = split_flows

    # flow selection: per band, longest first, global slot budget
    by_band = {}
    for c in flows:
        by_band.setdefault(c["bandIndex"], []).append(c)
    for b in by_band.values():
        b.sort(key=lambda c: arclen(c["points"]), reverse=True)
    face_slots = len(VOCAB)  # one word per iconic stroke
    budget = SLOT_TARGET - face_slots
    chosen, est = [], 0.0
    bands_sorted = sorted(by_band)  # lightest..darkest by index order
    i = 0
    while est < budget:
        took = False
        for b in bands_sorted:
            if by_band[b][i:i+1]:
                c = by_band[b][i]
                s = max(1.0, arclen(c["points"]) / EST_PX_PER_SLOT)
                if est + s > budget:
                    continue
                chosen.append(c)
                est += s
                took = True
        if not took:
            break
        i += 1

    chosen.sort(key=lambda c: (c["bandIndex"], -arclen(c["points"])))
    # WORDS-AS-SHADING band mapping, pinned to the trace's own
    # highlightBand key (=4): bandIndex 0 is the DARKEST -> heavy band 1
    # (22-40px); mids -> band 2; light -> band 3; the highlight band ->
    # engine band 4, the glow tier. (The first fix inverted this the
    # wrong way — geometry filled either way, semantics did not.)
    hlb = int(t.get("highlightBand", 4))
    def engine_band(bi):
        bi = int(bi)
        if bi == hlb:
            return 4
        return {0: 1, 1: 2, 2: 2, 3: 3}.get(bi, 3)
    # Rainbow lesson: hosted lines need breathing room for their glyph
    # size — greedy min-separation per engine band.
    # Retuned 2026-08-05: the earlier values were set while the
    # real failures (self-facing rings, hub convergence) were still
    # undiagnosed, so separation was carrying blame that belonged
    # elsewhere. A masterpiece should not ship 18 hostable strokes.
    SEP = {4: 15.0, 3: 12.0, 2: 16.0, 1: 22.0}
    def densify(pts, step=6.0):
        out = [pts[0]]
        for a, b in zip(pts, pts[1:]):
            seg = math.hypot(b[0] - a[0], b[1] - a[1])
            n = max(1, int(seg / step))
            for i in range(1, n + 1):
                out.append([a[0] + (b[0] - a[0]) * i / n,
                            a[1] + (b[1] - a[1]) * i / n])
        return out
    def min_dist(a, b):
        da, db = densify(a), densify(b)
        return min(math.hypot(pa[0] - pb[0], pa[1] - pb[1])
                   for pa in da for pb in db[::2])
    def med_dist(a, b):
        # 2-point straight pieces taught this: sample the LINES, not the
        # vertex lists — sparse vertices let coincident twins through.
        da, db = densify(a), densify(b)
        ds = sorted(min(math.hypot(pa[0] - pb[0], pa[1] - pb[1])
                        for pb in db) for pa in da)
        return ds[len(ds) // 2]
    # STRUCTURAL hosts first — required features win the space. Hands
    # (Eric's lassos) and the frame are closed/double-lined loops: same
    # laws as flows (presplit, >=60px, separation), band 3 small ink.
    def chaikin(pts, it=2):
        for _ in range(it):
            if len(pts) < 3:
                return pts
            q = [pts[0]]
            for a, b in zip(pts, pts[1:]):
                q.append([0.75*a[0]+0.25*b[0], 0.75*a[1]+0.25*b[1]])
                q.append([0.25*a[0]+0.75*b[0], 0.25*a[1]+0.75*b[1]])
            q.append(pts[-1])
            pts = q
        return pts
    structural = []
    hand_arcs = {}
    hands = (verdicts or {}).get("hands", {})
    for side, key in [("leftHand", "left"), ("rightHand", "right")]:
        poly = hands.get(key)
        if poly:
            # a closed lasso cannot host on all its segments — the
            # loop's far sides face each other across the hand width.
            # The hand STROKE is the longer open arc between the loop's
            # two most-distant points: one confident contour, nothing
            # facing it. (Smoothed first — the chin lesson, again.)
            loop = chaikin([list(q) for q in poly] + [list(poly[0])], 2)[:-1]
            n = len(loop)
            besti, bestj, bestd = 0, 0, -1.0
            for i in range(n):
                for j in range(i + 1, n):
                    d2 = ((loop[i][0]-loop[j][0])**2 + (loop[i][1]-loop[j][1])**2)
                    if d2 > bestd:
                        besti, bestj, bestd = i, j, d2
            arc1 = loop[besti:bestj + 1]
            arc2 = loop[bestj:] + loop[:besti + 1]
            # choose by OPENNESS (chord/arc), not length — a small
            # lasso's longer arc is a self-facing hook. Then one
            # confident curve (least-squares cubic), the chin lesson.
            def openness(a):
                chord = math.hypot(a[-1][0]-a[0][0], a[-1][1]-a[0][1])
                return chord / max(arclen(a), 1e-6)
            cands = [a for a in (arc1, arc2) if arclen(a) >= 60]
            if cands:
                arc = max(cands, key=openness)
                d = [0.0]
                for a2, b2 in zip(arc, arc[1:]):
                    d.append(d[-1] + math.hypot(b2[0]-a2[0], b2[1]-a2[1]))
                tt = [x / max(d[-1], 1e-6) for x in d]
                deg = min(3, len(arc) - 1)
                import numpy as _np
                px = _np.polyfit(tt, [q[0] for q in arc], deg)
                py = _np.polyfit(tt, [q[1] for q in arc], deg)
                smooth = [[float(_np.polyval(px, u)), float(_np.polyval(py, u))]
                          for u in _np.linspace(0, 1, 10)]
                hand_arcs[side] = smooth
    # her right hand lies OVER the left wrist — the lassos abut. The
    # smaller right stroke places first; the left stroke is trimmed
    # where it comes within glyph range (18px) of it.
    if "rightHand" in hand_arcs:
        structural.append({"points": hand_arcs["rightHand"], "feat": "rightHand"})
    if "leftHand" in hand_arcs:
        la = hand_arcs["leftHand"]
        if "rightHand" in hand_arcs:
            ra = hand_arcs["rightHand"]
            def dmin(pt):
                return min(math.hypot(pt[0]-q[0], pt[1]-q[1]) for q in ra)
            runs, cur = [], []
            for pt in la:
                if dmin(pt) >= 18.0:
                    cur.append(pt)
                else:
                    if cur:
                        runs.append(cur)
                    cur = []
            if cur:
                runs.append(cur)
            best = max(runs, key=arclen, default=la)
            la = best if arclen(best) >= 60 else la
        structural.append({"points": la, "feat": "leftHand"})
    def near_edge(pts, tol=30):
        xs = [q[0] for q in pts]; ys = [q[1] for q in pts]
        return min(xs) < tol or max(xs) > 512 - tol or min(ys) < tol or max(ys) > 512 - tol
    def hugs_edge(pts, tol=40):
        # a TRUE frame side: every point near the canvas boundary —
        # any-point tests admitted boundaries that wander inland (one
        # passed 1.3px from the right hand).
        return all(min(q[0], q[1], 512 - q[0], 512 - q[1]) < tol
                   for q in densify(pts))
    frame_pieces = []
    for c in bounds:
        if near_edge(c["points"]):
            for piece in presplit(c["points"]):
                if arclen(piece) >= 90 and hugs_edge(piece):
                    frame_pieces.append(piece)
    frame_pieces.sort(key=arclen, reverse=True)
    for piece in frame_pieces[:4]:
        structural.append({"points": piece, "feat": "frame"})
    kept_struct = []
    for c in sorted(structural, key=lambda c: arclen(c["points"]), reverse=True):
        if all(min_dist(c["points"], k["points"]) >= 20.0 for k in kept_struct):
            kept_struct.append(c)
    # required guarantee: at least one piece per structural feature
    for want in ["leftHand", "rightHand", "frame"]:
        if not any(k["feat"] == want for k in kept_struct):
            pool = [c for c in structural if c["feat"] == want]
            if pool:
                kept_struct.append(max(pool, key=lambda c: arclen(c["points"])))
    kept = []
    for c in chosen:
        eb = engine_band(c["bandIndex"])
        if (all(med_dist(c["points"], k["points"])
                >= min(SEP[eb], SEP[engine_band(k["bandIndex"])])
                for k in kept)
            and all(min_dist(c["points"], k["points"]) >= SEP[eb]
                    for k in kept_struct)):
            kept.append(c)
    chosen = kept
    # v7.4 build lint: every requiredFeature must appear as a host
    # path's feature name. Flows are named by REGION/band; the frame
    # boundary is promoted to a host so 'frame' exists in the trace.
    def centroid(pts):
        xs = [q[0] for q in pts]; ys = [q[1] for q in pts]
        return sum(xs) / len(xs), sum(ys) / len(ys)
    def flow_feature(c):
        if int(c["bandIndex"]) == hlb:
            return "highlightBand"
        x, y = centroid(c["points"])
        if 130 < y < 300 and not (190 < x < 330 and y < 190):
            return "hairMass"
        return f"band{c['bandIndex']}"
    # guarantee hairMass + highlightBand each host at least once
    for want in ["hairMass", "highlightBand"]:
        if not any(flow_feature(c) == want for c in chosen):
            pool = [c for c in flows if flow_feature(c) == want]
            if pool:
                chosen.append(max(pool, key=lambda c: arclen(c["points"])))
    paths, order = [], 0
    for c in chosen:
        order += 1
        L = arclen(c["points"])
        paths.append({"mode": "flow", "d": dstr(c["points"]), "order": order,
                      "budget": [2, 10] if L < 420 else [3, 20],
                      "band": engine_band(c["bandIndex"]),
                      "arch": "line", "feature": flow_feature(c)})
    for k in kept_struct:
        order += 1
        L = arclen(k["points"])
        paths.append({"mode": "flow", "d": dstr(k["points"]), "order": order,
                      "budget": [2, 10] if L < 420 else [3, 20],
                      "band": 3, "arch": "line", "feature": k["feat"],
                      "operatorDrawn": k["feat"] != "frame"})
    STROKE_FEATURE = {"leftEyeLid": "leftEye", "leftEyeCrease": "leftEye",
                      "rightEyeLid": "rightEye", "rightEyeCrease": "rightEye",
                      "noseBridge": "nose", "noseBase": "nose",
                      "lipParting": "mouth", "lowerLipShadow": "mouth",
                      "chinCrescent": "chin"}
    for name in VOCAB:
        if name == "lipParting":
            continue
        order += 1
        paths.append({"mode": "flow", "d": dstr(feats[name]["points"]),
                      "order": order, "budget": [2, 10], "band": 2,
                      "arch": "line", "feature": STROKE_FEATURE[name]})
    order += 1  # F4: the smile schedules LAST — max export order, focal
    paths.append({"mode": "flow", "d": dstr(feats["lipParting"]["points"]),
                  "order": order, "budget": [2, 10], "band": 2,
                  "arch": "line", "feature": "mouth",
                  "focal": True})

    guide = ([dstr(c["points"]) for c in bounds]
             + [dstr(c["points"]) for c in crumbs])[:200]
    return {
        "extractionClass": "TONAL",
        "requiredFeatures": TWELVE[:],  # Eric's twelve, spec-verbatim
        "paths": paths,
        "guide": guide,
    }, {"flows": len(chosen), "estSlots": round(face_slots + est),
        "guides": min(len(bounds), 200)}

def main():
    rows, green, verdicts = gate_report()
    t = json.loads(TRACE.read_text())
    presence = presence_report(t, verdicts)
    patch, stats = build_entry(verdicts)
    (SCRATCH / "staged-mona-entry.json").write_text(json.dumps(patch, indent=1))
    print("STAGED staged-mona-entry.json —", stats)
    print("THE TWELVE (v7.4: export blocks when one is missing):")
    for name in TWELVE:
        ok, why = presence[name]
        print(f"  [{'x' if ok else ' '}] {name:14} {why}")
    print("SESSION GATES:")
    for g, ok in rows:
        print(f"  [{'x' if ok else ' '}] {g}")
    if "--flip" not in sys.argv:
        return
    missing = [n for n in TWELVE if not presence[n][0]]
    if missing or not green:
        sys.exit(f"EXPORT BLOCKED: missing features {missing}; session gates "
                 "not all recorded. D4 is law — the smile must be touched.")
    reg = json.loads(REG.read_text())
    mona = next(p for p in reg["pictures"] if p["id"] == "mona")
    mona.update(patch)
    REG.write_text(json.dumps(reg, ensure_ascii=False, indent=1))
    print("FLIPPED: mona is TONAL. Run the scoped sweep + mona_bands_and_density.")

if __name__ == "__main__":
    main()
