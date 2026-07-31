#!/usr/bin/env python3
"""v8 scan library: pinned batch traces -> render-library scan files.

Paths are VERBATIM (the tracer is deterministic; the pin hash proves it).
Scan-acceptance work happens here, once: pre-marked segment boundaries
(F2's ONLY split points), D-A sub-floor merge tags, and the raw
measurements F4's gate reads. Layout never touches geometry again.
"""
import json, math, pathlib, subprocess, sys
from PIL import Image, ImageOps

ROOT = pathlib.Path(__file__).resolve().parents[1]
TP = ROOT / "target/release/trace-pgm"
REF = ROOT / "content-pipeline/wordpic/ref"
OUT = ROOT / "content-pipeline/wordpic/scans"
OUT.mkdir(exist_ok=True)
CANVAS = 512
# Proposed constants (D1-D3 pending Eric):
FLOOR = 13.0
MIN_WORD_CHARS = 3
CORNER_DEG = 35.0
SMALL_FEATURE_MAX = 130.0
SEG_MAX = 100000.0  # v8.1: pack per path; corner marks only   # a segment longer than this gets an interior mark

# Eric's per-path sign-off (D4): features that MUST be present, named.
# Position is (x, y) in canvas coords with a tolerance, so the gate binds
# to the feature, not to a path index that could renumber.
REQUIRED_MICRO = {
    # Verified against the recovered closed features (build output):
    # eyes at (233,137)/(267,138), nose wedge at (258,160), buttons below.
    "snowman": [
        {"name": "left eye", "near": [233, 137], "tol": 20},
        {"name": "right eye", "near": [267, 138], "tol": 20},
        {"name": "nose", "near": [258, 160], "tol": 24},
    ],
}

SUBJ = {  # subject -> (ref, mode, tier)
 "dog": ("dog.png", "ink", "easy"), "butterfly": ("butterfly.png", "ink", "easy"),
 "duck": ("duck.png", "ink", "easy"), "turtle": ("turtle.png", "ink", "easy"),
 "owl": ("owl.png", "ink-boundary", "medium"), "elephant": ("elephant.png", "ink", "medium"),
 "snowman": ("snowman.png", "ink", "medium"), "horse": ("horse.png", "ink", "hard"),
 "fish": ("fish.png", "ink", "easy"), "eiffel": ("eiffel.png", "ink", "hard"),
 "dragon": ("dragon.jpg", "ink", "hard"), "peacock": ("peacock.jpg", "ink", "hard"),
}

# ---- per-subject AUTHORING recipes (content work, not renderer tuning:
# the renderer stays picture-agnostic; this is how a scan is authored) ----
def author_horse(im):
    """v8.2.1 horse re-author (Eric): the block was MicroBudget 0.234 —
    grass tufts and mane/tail wisps fragment into arcs too small to host
    words. Recipe: drop the grass by colour, then close the subject mask
    so mane and tail spikes merge into solid mass. Interior identity
    lines (leg separations, mane edge, jaw, eye) survive untouched."""
    rgb = im.convert("RGB")
    W, H = rgb.size
    px = rgb.load()
    # 1. subject = not near-white, not green-dominant (grass)
    mask = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            r, g, b = px[x, y]
            if r > 235 and g > 235 and b > 235:
                continue
            if g > r + 12 and g > b + 12:      # grass green
                continue
            mask[y][x] = True
    # 2. keep the largest component (drops isolated grass blades)
    seen = [[False]*W for _ in range(H)]
    best, bestsz = None, 0
    for y0 in range(H):
        for x0 in range(W):
            if not mask[y0][x0] or seen[y0][x0]:
                continue
            comp, stack = [], [(x0, y0)]
            seen[y0][x0] = True
            while stack:
                x, y = stack.pop()
                comp.append((x, y))
                for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = True
                        stack.append((nx, ny))
            if len(comp) > bestsz:
                best, bestsz = comp, len(comp)
    keep = [[False]*W for _ in range(H)]
    for x, y in best:
        keep[y][x] = True
    # 3. morphological CLOSE (r=5): mane/tail spikes merge into mass
    R = 5
    dil = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if not keep[y][x]:
                continue
            for dy in range(-R, R+1):
                for dx in range(-R, R+1):
                    if dx*dx + dy*dy > R*R:
                        continue
                    nx, ny = x+dx, y+dy
                    if 0 <= nx < W and 0 <= ny < H:
                        dil[ny][nx] = True
    closed = [[False]*W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if not dil[y][x]:
                continue
            ok = True
            for dy in range(-R, R+1):
                for dx in range(-R, R+1):
                    if dx*dx + dy*dy > R*R:
                        continue
                    nx, ny = x+dx, y+dy
                    if not (0 <= nx < W and 0 <= ny < H and dil[ny][nx]):
                        ok = False
                        break
                if not ok:
                    break
            closed[y][x] = ok
    # 4. paint: subject ink dark, filled gaps dark, everything else white
    out = Image.new("L", (W, H), 255)
    o = out.load()
    g = ImageOps.autocontrast(rgb.convert("L"), cutoff=1).load()
    for y in range(H):
        for x in range(W):
            if closed[y][x] or keep[y][x]:
                o[x, y] = min(g[x, y], 90) if keep[y][x] else 40
    return out

AUTHOR = {"horse": author_horse}


def rings(path, subject=None):
    """Ring-based scan source for boundary art: threshold -> boundary
    pixels -> connected components -> ordered rings (greedy walk). The
    outer ring is the word path; interior rings are the strokes' far
    sides / hole contours."""
    im0 = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im0.size, (255, 255, 255, 255)); bg.alpha_composite(im0)
    if subject in AUTHOR:
        im = AUTHOR[subject](bg)
    else:
        im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS - im.width) // 2, (CANVAS - im.height) // 2))
    px = c.load()
    W = H = CANVAS
    hist = [0]*256
    for y in range(H):
        for x in range(W):
            hist[px[x, y]] += 1
    total = W * H
    sum_all = sum(i * h for i, h in enumerate(hist))
    sum_b = w_b = best = 0.0
    t_best = 127
    for t in range(256):
        w_b += hist[t]
        if w_b == 0: continue
        w_f = total - w_b
        if w_f == 0: break
        sum_b += t * hist[t]
        m_b, m_f = sum_b / w_b, (sum_all - sum_b) / w_f
        between = w_b * w_f * (m_b - m_f) ** 2
        if between > best: best, t_best = between, t
    T = min(t_best, 160)
    ink = [[px[x, y] <= T for x in range(W)] for y in range(H)]
    bnd = set()
    for y in range(H):
        for x in range(W):
            if not ink[y][x]: continue
            if x == 0 or y == 0 or x == W-1 or y == H-1 or not (ink[y][x-1] and ink[y][x+1] and ink[y-1][x] and ink[y+1][x]):
                bnd.add((x, y))
    comps = []
    seen = set()
    for start in sorted(bnd):
        if start in seen: continue
        comp = []
        stack = [start]; seen.add(start)
        while stack:
            (x, y) = stack.pop()
            comp.append((x, y))
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    n = (x+dx, y+dy)
                    if n in bnd and n not in seen:
                        seen.add(n); stack.append(n)
        comps.append(comp)
    ordered = []
    for comp in comps:
        if len(comp) < 8: continue
        rem = set(comp)
        cur = min(rem)
        ring = [cur]; rem.discard(cur)
        while rem:
            nxt = min(rem, key=lambda p: (p[0]-cur[0])**2 + (p[1]-cur[1])**2)
            d2 = (nxt[0]-cur[0])**2 + (nxt[1]-cur[1])**2
            if d2 > 36:  # ring walk jumped — separate strand
                break
            ring.append(nxt); rem.discard(nxt); cur = nxt
        if len(ring) >= 8:
            ring.append(ring[0])  # close the loop
            ordered.append([(float(x), float(y)) for x, y in ring[::2]])
    return ordered

def pgm(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255)); bg.alpha_composite(im)
    im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS - im.width) // 2, (CANVAS - im.height) // 2))
    return b"P5 %d %d 255\n" % c.size + c.tobytes()

def plen(p):
    return sum(math.hypot(b[0]-a[0], b[1]-a[1]) for a, b in zip(p, p[1:]))

def turn(a, b, c):
    v1 = (b[0]-a[0], b[1]-a[1]); v2 = (c[0]-b[0], c[1]-b[1])
    m = (math.hypot(*v1) * math.hypot(*v2)) or 1e-9
    d = max(-1.0, min(1.0, (v1[0]*v2[0]+v1[1]*v2[1]) / m))
    return math.degrees(math.acos(d))

def fnv(paths):
    h = 0xcbf29ce484222325
    for p in paths:
        for x, y in p:
            for b in int(round(x*10)).to_bytes(4, "little", signed=True) + int(round(y*10)).to_bytes(4, "little", signed=True):
                h ^= b; h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h

def seg_marks(p):
    """Pre-marked boundaries: corners past CORNER_DEG plus interior marks
    keeping every segment <= SEG_MAX px. Fractions of total arc."""
    total = plen(p)
    if total <= 0: return []
    cums = [0.0]
    for a, b in zip(p, p[1:]):
        cums.append(cums[-1] + math.hypot(b[0]-a[0], b[1]-a[1]))
    marks = set()
    for i in range(1, len(p)-1):
        if turn(p[i-1], p[i], p[i+1]) > CORNER_DEG:
            marks.add(round(cums[i]/total, 4))
    bounds = sorted({0.0, 1.0} | marks)
    final = set(marks)
    for a, b in zip(bounds, bounds[1:]):
        span = (b - a) * total
        n = int(span // SEG_MAX)
        for k in range(1, n + 1):
            final.add(round(a + (b - a) * k / (n + 1), 4))
    # Corner marks are kept UNCONDITIONALLY (v6 star law: text never
    # bends around a corner). Only interior subdivision marks respect the
    # minimum span; sub-hostable slivers between corner marks are skipped
    # by the typesetter and excluded from recall (D-A / D2 territory).
    return sorted(final)

def path_min_clearance(p, others):
    """Interior-to-interior clearance: junction contact (a leg meeting the
    body at shared endpoints) is structure, not a violation — F4's G3 is
    about PARALLEL strokes without word-height space between them."""
    def sd(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    J = 18.0
    def interior(path):
        e0, e1 = path[0], path[-1]
        return [pt for pt in path
                if math.hypot(pt[0]-e0[0], pt[1]-e0[1]) > J
                and math.hypot(pt[0]-e1[0], pt[1]-e1[1]) > J]
    pi = interior(p)
    if not pi: return 999.0
    m = 1e9
    for q in others:
        qi = interior(q)
        if len(qi) < 2: continue
        for pt in pi[::3]:
            for u, v in zip(qi, qi[1:]):
                d = sd(pt, u, v)
                if d < m: m = d
    return m

for sub, (ref, mode, tier) in SUBJ.items():
    paths = rings(REF / ref, sub)
    paths.sort(key=plen, reverse=True)
    # v8.2.1 small-feature pass: a SHORT CLOSED contour is a solid source
    # feature (snowman eyes, nose; animal eyes). It is preserved intact —
    # exempt from decorative dedup, from the tight-run split, and from
    # sub-floor merging — and carries micro_feature so the renderer draws
    # it filled and the gate can require it.
    def is_small_closed(p):
        return (math.hypot(p[0][0]-p[-1][0], p[0][1]-p[-1][1]) < 3.0
                and plen(p) < SMALL_FEATURE_MAX)
    small_feats = [p for p in paths if is_small_closed(p)]
    paths = [p for p in paths if not is_small_closed(p)]
    entries = []
    lens = [plen(p) for p in paths]
    for i, p in enumerate(paths):
        L = lens[i]
        sub_floor = L < FLOOR * MIN_WORD_CHARS
        parent = None
        if sub_floor and len(paths) > 1:
            # D-A: merge tag -> nearest longer path
            best, bd = None, 1e9
            cx = sum(x for x, _ in p)/len(p); cy = sum(y for _, y in p)/len(p)
            for j, q in enumerate(paths):
                if j == i or lens[j] < FLOOR * MIN_WORD_CHARS: continue
                d = min(math.hypot(cx-x, cy-y) for x, y in q[::3])
                if d < bd: bd, best = d, j
            parent = best
        marks = seg_marks(p)
        cums = [0.0]
        for a2, b2 in zip(p, p[1:]):
            cums.append(cums[-1] + math.hypot(b2[0]-a2[0], b2[1]-a2[1]))
        totL = cums[-1] or 1.0
        bounds = [0.0] + [t for t in marks if 0 < t < 1] + [1.0]
        worst_turn = 0.0
        for k in range(1, len(p)-1):
            t_here = cums[k] / totL
            near_mark = any(abs(t_here - m) < 0.012 for m in bounds)
            if not near_mark:
                worst_turn = max(worst_turn, turn(p[k-1], p[k], p[k+1]))
        entries.append({
            "points": p, "arc": round(L, 2), "tier": tier,
            "sub_floor": sub_floor, "merged_into": parent,
            "decorative_thin": False,
            "segments": marks,
            "worst_turn_deg": round(worst_turn, 1),
            "min_clearance": None,  # filled below
        })
    # Stroke-side dedup: ink_boundary emits BOTH sides of a drawn stroke.
    # The far side (>=70% of points within 16px of a LONGER path) is
    # decorative_thin — one drawn line, one word baseline; covering one
    # side recovers the stroke. Recall excludes decorative_thin per F5.
    def near_frac(p, q, tol=22.0):
        def sd(pt, u, v):
            vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
            t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
            return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
        pts = p[::2] if len(p) > 4 else p
        hit = sum(1 for pt in pts if any(sd(pt, u, v) <= tol for u, v in zip(q, q[1:])))
        return hit / len(pts)
    order = sorted(range(len(entries)), key=lambda i: -lens[i])
    kept = []
    for i in order:
        e = entries[i]
        if e["sub_floor"]:
            continue
        if any(near_frac(e["points"], entries[j]["points"]) >= 0.7 for j in kept):
            e["decorative_thin"] = True
        else:
            kept.append(i)
    # Tight-run split (scan acceptance, geometry verbatim): runs of a word
    # path closer than one floor glyph to another word path are the far
    # sides of strokes (leg gaps, hat brims) — they become decorative
    # entries; the clear runs stay word paths. F4 then gates truthfully
    # and F3 never faces an impossible segment.
    def sd3(pt, u, v):
        vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
        t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
        return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
    word_idx = [i for i, e in enumerate(entries) if not e["sub_floor"] and not e["decorative_thin"]]
    new_entries = []
    for i, e in enumerate(entries):
        if e["sub_floor"] or e["decorative_thin"]:
            new_entries.append(e)
            continue
        others = [entries[j]["points"] for j in word_idx if j != i]
        if not others:
            new_entries.append(e)
            continue
        pts = e["points"]
        tight = [any(sd3(pt, u, v) < FLOOR * 1.15 for q in others for u, v in zip(q, q[1:])) for pt in pts]
        # smooth runs: a run flips only if >= 3 consecutive agree
        runs = []
        cur = tight[0]; start = 0
        k = 0
        while k < len(pts):
            if tight[k] != cur:
                nxt = tight[k:k+2]
                if len(nxt) == 2 and all(v == tight[k] for v in nxt):
                    runs.append((start, k, cur)); start = k; cur = tight[k]
            k += 1
        runs.append((start, len(pts), cur))
        if all(not t for _, _, t in runs) or len(runs) == 1:
            e2 = dict(e)
            if runs[0][2]:
                e2["decorative_thin"] = True
            new_entries.append(e2)
            continue
        for (a, b, is_tight) in runs:
            seg = pts[a:b+1][:]
            if len(seg) < 2 or plen(seg) < 20:
                continue
            e2 = dict(e)
            e2["points"] = seg
            e2["arc"] = round(plen(seg), 2)
            e2["decorative_thin"] = bool(is_tight)
            e2["sub_floor"] = (not is_tight) and plen(seg) < FLOOR * MIN_WORD_CHARS
            e2["segments"] = seg_marks(seg)
            e2["worst_turn_deg"] = 0.0
            marks2 = e2["segments"]
            cums2 = [0.0]
            for a3, b3 in zip(seg, seg[1:]):
                cums2.append(cums2[-1] + math.hypot(b3[0]-a3[0], b3[1]-a3[1]))
            tot2 = cums2[-1] or 1.0
            bounds2 = [0.0] + [t for t in marks2 if 0 < t < 1] + [1.0]
            for k2 in range(1, len(seg)-1):
                th = cums2[k2] / tot2
                if not any(abs(th - m) < 0.012 for m in bounds2):
                    e2["worst_turn_deg"] = max(e2["worst_turn_deg"], round(turn(seg[k2-1], seg[k2], seg[k2+1]), 1))
            new_entries.append(e2)
    entries = new_entries
    lens = [plen(e["points"]) for e in entries]
    for i, e in enumerate(entries):
        others = [q["points"] for j, q in enumerate(entries)
                  if j != i and not q["sub_floor"] and not q["decorative_thin"]]
        if not others:
            e["min_clearance"] = 999.0
            e["tight_frac"] = 0.0
            continue
        e["min_clearance"] = round(path_min_clearance(e["points"], others), 2)
        # fraction of this path's arc closer than one floor glyph to any
        # other word path — local contact zones (stacked circles touching)
        # are F3's job; only broadly-parallel geometry blocks.
        def sd2(pt, u, v):
            vx, vy = v[0]-u[0], v[1]-u[1]; l2 = vx*vx+vy*vy
            t = 0 if l2 == 0 else max(0, min(1, ((pt[0]-u[0])*vx+(pt[1]-u[1])*vy)/l2))
            return math.hypot(pt[0]-(u[0]+t*vx), pt[1]-(u[1]+t*vy))
        pts = e["points"][::2]
        tight = sum(1 for pt in pts if any(sd2(pt, u, v) < FLOOR for q in others for u, v in zip(q, q[1:])))
        e["tight_frac"] = round(tight / max(1, len(pts)), 3)
    for p in small_feats:
        entries.append({
            "points": p, "arc": round(plen(p), 2), "tier": tier,
            "sub_floor": False, "merged_into": None, "decorative_thin": False,
            "micro_feature": True, "segments": [], "worst_turn_deg": 0.0,
            "min_clearance": 999.0, "tight_frac": 0.0,
        })
    # v8.2.1 (2): required micro features — named, signed off in the scan
    # file. The render gate refuses to display a subject missing any.
    req = REQUIRED_MICRO.get(sub)
    doc = {"subject": sub, "tier": tier, "pin_hash": fnv(paths), "canvas": CANVAS,
           "required_micro": req or [], "paths": entries}
    (OUT / f"{sub}.json").write_text(json.dumps(doc))
    print(f'{sub:10} paths={len(entries):5} sub_floor={sum(e["sub_floor"] for e in entries):4} hash={doc["pin_hash"]:#x}')
