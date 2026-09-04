#!/usr/bin/env python3
"""CC-BUILD219-FIXES F3 — the continuity gate.

Recall and precision are AGGREGATES, and an aggregate hides the defect Eric
actually photographed. A trace can cover 92% of the reference ink and still
leave the dallah's spout detached from its body, because the missing 8% is one
continuous stretch at exactly the join that makes the object read. Averages
answer "how much ink is covered"; they never answer "is any single gap big
enough to break the drawing".

THE RULE (F3): along each reference ink component, the longest untraced run
must be <= 3% of that component's length. Reported per asset as the worst run
across its components.

MEASUREMENT, stated plainly so the number can be argued with:
  * ink      = reference pixels darker than the tracer's own threshold, built
               through the same 512-canvas pipeline the tracer is fed, so the
               two agree on where the ink is.
  * covered  = within TAU of a traced path. TAU is 1% of the canvas diagonal,
               the same tolerance run-ink-batch grades with.
  * a RUN    = one connected blob of uncovered pixels inside a component.
  * length   = pixel COUNT, used as a proxy for arc length. Honest about
               itself: for the thin ink this gate is aimed at the two are
               within a few percent, and for a thick blob the proxy overstates
               a gap, which errs toward failing rather than passing.

Run:  python3 tools/ink_continuity.py <subject> [subject...]
"""
import json, math, pathlib, re, subprocess, sys
from collections import deque
from PIL import Image, ImageOps, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
REF = ROOT / "content-pipeline/wordpic/ref"
CANVAS, THRESH = 512, 128
TAU = 0.01 * math.sqrt(2 * CANVAS ** 2)   # ~7.2px, same as run-ink-batch
MAX_RUN_FRAC = 0.03                        # F3
MIN_COMPONENT = 40                         # ignore specks: below this a "run" is noise

REFS = {"daruma": "daruma.png", "dallah": "dallah.png", "taos": "taos-source.jpg",
        "dog": "dog.png", "fish": "fish.png", "eiffel": "eiffel.png"}


def canvas_gray(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    bg.alpha_composite(im)
    im = bg.convert("L")
    corner = im.getpixel((2, 2))
    if 60 < corner < 235:
        lo, hi = corner - 18, corner + 18
        im = im.point(lambda v: 255 if lo <= v <= hi else v)
    im = ImageOps.autocontrast(im, cutoff=1)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS - im.width) // 2, (CANVAS - im.height) // 2))
    return c


def components(mask, w, h, within=None):
    """8-connected blobs of set pixels, optionally restricted to `within`."""
    seen = bytearray(w * h)
    out = []
    for s in range(w * h):
        if not mask[s] or seen[s] or (within is not None and not within[s]):
            continue
        q, blob = deque([s]), []
        seen[s] = 1
        while q:
            i = q.popleft()
            blob.append(i)
            x, y = i % w, i // w
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < w and 0 <= ny < h:
                        j = ny * w + nx
                        if mask[j] and not seen[j] and (within is None or within[j]):
                            seen[j] = 1
                            q.append(j)
        out.append(blob)
    return out


def traced_paths(sub):
    scans = json.load(open(ROOT / "config/wordpic/scans.json"))["subjects"]
    pics = {x["id"]: x for x in json.load(open(ROOT / "config/wordpic/pictures.json"))["pictures"]}
    p = pics.get(sub, {})
    if "masters" in (p.get("categories") or []) and p.get("guide"):
        # A master carries its drawing in the guide; the carriers are rails.
        out = []
        for d in p["guide"]:
            n = [float(x) for x in re.findall(r"-?\d+\.?\d*", d)]
            out.append(list(zip(n[0::2], n[1::2])))
        return out, "guide"
    return [[tuple(q) for q in e["p"]] for e in scans[sub]["paths"]], "carriers"


def report(sub):
    g = canvas_gray(REF / REFS[sub])
    w, h = g.size
    px = g.load()
    solid = bytearray(1 if px[i % w, i // w] < THRESH else 0 for i in range(w * h))
    # THE INK IS THE EDGE, NOT THE FILL. The first cut of this gate measured
    # every dark pixel and reported dog and fish -- both perfect 1.000/1.000
    # traces -- as 73% untraced. The references are filled silhouettes, so the
    # interior of a solid dog is "ink" that no boundary trace will ever sit on.
    # What a trace is answerable for is the OUTLINE, so the ink for continuity
    # purposes is the set of ink pixels touching non-ink. Thin-stroke art is
    # unaffected: a 2px stroke is all edge.
    def edge(i):
        x, y = i % w, i // w
        for nx, ny in ((x-1, y), (x+1, y), (x, y-1), (x, y+1)):
            if not (0 <= nx < w and 0 <= ny < h) or not solid[ny * w + nx]:
                return True
        return False
    ink = bytearray(1 if solid[i] and edge(i) else 0 for i in range(w * h))

    paths, layer = traced_paths(sub)
    cov = Image.new("1", (w, h), 0)
    d = ImageDraw.Draw(cov)
    for p in paths:
        if len(p) >= 2:
            d.line([(float(x), float(y)) for x, y in p], fill=1, width=int(2 * TAU) + 1)
    cv = cov.load()
    uncovered = bytearray(1 if ink[i] and not cv[i % w, i // w] else 0 for i in range(w * h))

    worst, worst_n, comps = 0.0, 0, 0
    for blob in components(ink, w, h):
        if len(blob) < MIN_COMPONENT:
            continue
        comps += 1
        inblob = bytearray(w * h)
        for i in blob:
            inblob[i] = 1
        runs = components(uncovered, w, h, within=inblob)
        longest = max((len(r) for r in runs), default=0)
        frac = longest / len(blob)
        if frac > worst:
            worst, worst_n = frac, longest
    ok = worst <= MAX_RUN_FRAC
    print(f"  {sub:8} {layer:9} components={comps:3}  worst untraced run = "
          f"{worst*100:6.2f}% ({worst_n} px)   {'PASS' if ok else 'FAIL'} (limit {MAX_RUN_FRAC*100:.0f}%)")
    return ok


if __name__ == "__main__":
    subs = sys.argv[1:] or ["daruma", "dallah", "taos"]
    print(f"CC-BUILD219-FIXES F3 continuity — longest untraced run per reference ink component")
    bad = sum(0 if report(s) else 1 for s in subs if s in REFS)
    raise SystemExit(1 if bad else 0)
