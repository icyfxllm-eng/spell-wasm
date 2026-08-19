#!/usr/bin/env python3
"""Guide-layer generator: mkbitmap + potrace, graded by ink-eval.

The guide is the traced artwork under the words. It never hosts words and
never collides (see guide_polys), so none of the word-carrier rules apply --
not slot length, not corner splits, not the separation rule. Only whether it
looks like the picture.

WHY POTRACE. The first generator sliced the image at five luminance levels and
contoured each region, hand-rolled. That has no notion of an edge: it draws a
line wherever a smooth gradient crosses a threshold, so an Ansel Adams sky
came back as five stacked bands that are not in the photograph. potrace does
curve fitting with corner detection and speckle suppression, and mkbitmap's
highpass removes the gradient BEFORE thresholding -- the step the old method
never had. Same subject, same budget: the false bands vanish.

-a 0 makes potrace emit polygons rather than Beziers, which is the M/L
polyline the guide field wants.

WHY INK-EVAL, AND WHY IT IS NOT A GATE. The repo ships a grader whose gates
are 0.97 recall and 0.97 precision, calibrated on full ink traces. A guide is
a budgeted subset by design and cannot score like one: the guides Eric has
accepted run 0.80-0.89 recall and 0.58-0.87 precision. Worse, recall is not
quality. Red Fuji's 40-path guide scores 0.174 and reads better than a potrace
version at 0.877, because a woodblock print's ink includes every decorative
stripe and maximising coverage buys clutter. Eric's ruling: keep the 40-path
Fuji. So the numbers are REPORTED, never enforced. Precision is the one worth
watching -- it tracks whether strokes sit on real ink.
"""
from __future__ import annotations
import argparse, json, math, pathlib, re, subprocess, sys, tempfile
from PIL import Image, ImageOps

ROOT = pathlib.Path(__file__).resolve().parents[1]
INK_EVAL = ROOT / "target/release/ink-eval"
CANVAS, LO, HI = 512, 22.0, 490.0
CAP, BUDGET, PERPATH = 220, 3200, 165
HOUSE_RECALL, HOUSE_PRECISION = (0.80, 0.89), (0.58, 0.87)


def _run(cmd, **kw):
    r = subprocess.run(cmd, capture_output=True, **kw)
    if r.returncode != 0:
        sys.exit(f"{cmd[0]} failed: {r.stderr.decode()[:300]}")
    return r


def parse_potrace(svg_path):
    """potrace writes an absolute M then RELATIVE linetos, inside a
    translate+scale group. Reading number pairs as absolute points and
    ignoring the transform turns a saguaro into four straight lines."""
    src = pathlib.Path(svg_path).read_text()
    tx = ty = 0.0; sx = sy = 1.0
    m = re.search(r'transform="translate\(([-\d.]+),([-\d.]+)\)\s*scale\(([-\d.]+),([-\d.]+)\)"', src)
    if m:
        tx, ty, sx, sy = (float(v) for v in m.groups())
    NEED = {"M": 2, "m": 2, "L": 2, "l": 2, "C": 6, "c": 6, "V": 1, "v": 1, "H": 1, "h": 1}
    out, path = [], []
    for d in re.findall(r'\sd="([^"]+)"', src):
        cmd, cur, vals = None, (0.0, 0.0), []
        for tok, num in re.findall(r"([MmLlCcVvHhZz])|(-?[\d.]+)", d):
            if tok:
                cmd, vals = tok, []
                if tok in "Zz" and len(path) >= 4:
                    out.append(path); path = []
                continue
            vals.append(float(num))
            if len(vals) < NEED.get(cmd, 2):
                continue
            if cmd in "Mm":
                if len(path) >= 4:
                    out.append(path)
                cur = (vals[0], vals[1]) if cmd == "M" else (cur[0] + vals[0], cur[1] + vals[1])
                path = [cur]
            elif cmd in "Ll":
                cur = (vals[0], vals[1]) if cmd == "L" else (cur[0] + vals[0], cur[1] + vals[1])
                path.append(cur)
            elif cmd in "Cc":
                cur = (vals[4], vals[5]) if cmd == "C" else (cur[0] + vals[4], cur[1] + vals[5])
                path.append(cur)
            elif cmd in "HhVv":
                dx = vals[0]
                cur = {"H": (dx, cur[1]), "h": (cur[0] + dx, cur[1]),
                       "V": (cur[0], dx), "v": (cur[0], cur[1] + dx)}[cmd]
                path.append(cur)
            vals = []
        if len(path) >= 4:
            out.append(path); path = []
    return [[(tx + x * sx, ty + y * sy) for x, y in p] for p in out]


def simplify(pts, eps):
    if len(pts) < 3:
        return pts
    if pts[0] == pts[-1] and len(pts) > 4:
        ring = pts[:-1]
        x0, y0 = ring[0]
        far = max(range(1, len(ring)), key=lambda i: (ring[i][0] - x0) ** 2 + (ring[i][1] - y0) ** 2)
        return simplify(ring[: far + 1], eps)[:-1] + simplify(ring[far:] + [ring[0]], eps)
    def rec(a, b):
        (x1, y1), (x2, y2) = pts[a], pts[b]
        dx, dy = x2 - x1, y2 - y1
        n = math.hypot(dx, dy) or 1e-9
        worst, wi = 0.0, -1
        for i in range(a + 1, b):
            x, y = pts[i]
            dd = abs(dy * x - dx * y + x2 * y1 - y2 * x1) / n
            if dd > worst:
                worst, wi = dd, i
        return [pts[a]] if worst <= eps or wi < 0 else rec(a, wi) + rec(wi, b)
    return rec(0, len(pts) - 1) + [pts[-1]]


plen = lambda p: sum(math.hypot(a[0] - b[0], a[1] - b[1]) for a, b in zip(p, p[1:]))


def fit(paths, eps, budget):
    """Normalise into the 22-490 box FIRST, then simplify, so one epsilon
    means the same thing for every subject whatever its source size."""
    xs = [x for p in paths for x, _ in p]; ys = [y for p in paths for _, y in p]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    s = min((HI - LO) / max(1e-6, x1 - x0), (HI - LO) / max(1e-6, y1 - y0))
    ox = LO + ((HI - LO) - (x1 - x0) * s) / 2 - x0 * s
    oy = LO + ((HI - LO) - (y1 - y0) * s) / 2 - y0 * s
    out = []
    for p in paths:
        q = [(x * s + ox, y * s + oy) for x, y in p]
        e = eps
        r = simplify(q, e)
        # no single contour may hoard the budget: Starry Night's swirl came
        # back as 470 points and crowded out the other 47 shapes
        while len(r) > PERPATH and e < 12:
            e *= 1.5
            r = simplify(q, e)
        if len(r) >= 4:
            out.append(r)
    out.sort(key=plen, reverse=True)
    keep, n = [], 0
    for q in out[:CAP]:
        if n + len(q) > budget:
            continue
        keep.append(q); n += len(q)
    return keep


def grade(paths, im, work):
    if not INK_EVAL.exists():
        return {"error": "ink-eval not built"}
    W, H = im.size
    k = max(W, H) / (HI - LO)
    in_px = [[((x - LO) * k - (max(W, H) - W) / 2, (y - LO) * k - (max(W, H) - H) / 2)
              for x, y in p] for p in paths]
    pj, rj = work / "p.json", work / "r.json"
    pj.write_text(json.dumps([[[round(x, 2), round(y, 2)] for x, y in p] for p in in_px]))
    pgm = b"P5 %d %d 255\n" % im.size + im.tobytes()
    r = subprocess.run([str(INK_EVAL), str(pj), str(rj)], input=pgm, capture_output=True)
    if r.returncode != 0 or not r.stdout.strip():
        return {"error": r.stderr.decode()[:160] or "no output"}
    return json.loads(r.stdout)


def generate(src, threshold, blur, filt, turd, eps, budget, work):
    im = Image.open(src).convert("L")
    im.thumbnail((900, 900), Image.LANCZOS)
    im = ImageOps.autocontrast(im, cutoff=1)
    pgm = work / "in.pgm"; im.save(pgm)
    args = ["mkbitmap", "-s", "2", "-t", str(threshold)]
    args += ["-b", str(blur)] if blur else ["-f", str(filt)]
    _run(args + [str(pgm), "-o", str(work / "b.pbm")])
    _run(["potrace", "-s", "-a", "0", "-t", str(turd), str(work / "b.pbm"),
          "-o", str(work / "b.svg")])
    raw = parse_potrace(work / "b.svg")
    if not raw:
        return [], {"error": "potrace returned no paths"}
    paths = fit(raw, eps, budget)
    return paths, grade(paths, im, work)


def render(paths, out, bg="#f4efe4", ink="rgba(26,23,18,.58)"):
    body = "".join('<path d="M' + " L".join(f"{x:.1f} {y:.1f}" for x, y in p)
                   + f'" fill="none" stroke="{ink}" stroke-width="2" '
                     'stroke-linecap="round" stroke-linejoin="round"/>' for p in paths)
    pathlib.Path(out).write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}" '
        f'width="{CANVAS}"><rect width="{CANVAS}" height="{CANVAS}" fill="{bg}"/>{body}</svg>')


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("source")
    ap.add_argument("--id", required=True)
    ap.add_argument("--threshold", type=float, default=0.45)
    ap.add_argument("--blur", type=float)
    ap.add_argument("--filter", type=int, default=4)
    ap.add_argument("--turdsize", type=int, default=60)
    ap.add_argument("--eps", type=float, default=1.25)
    ap.add_argument("--budget", type=int, default=BUDGET)
    ap.add_argument("--out")
    a = ap.parse_args()
    with tempfile.TemporaryDirectory() as td:
        paths, m = generate(a.source, a.threshold, a.blur, a.filter, a.turdsize,
                            a.eps, a.budget, pathlib.Path(td))
    pts = sum(len(p) for p in paths)
    if "error" in m:
        print(f"{a.id:12} {len(paths):3} paths {pts:5} pts   ink-eval: {m['error']}")
    else:
        rc, pr = m.get("ink_recall", 0), m.get("path_precision", 0)
        note = "" if HOUSE_RECALL[0] <= rc <= HOUSE_RECALL[1] else "  (recall outside the house band)"
        print(f"{a.id:12} {len(paths):3} paths {pts:5} pts   "
              f"recall={rc:.3f} precision={pr:.3f}{note}")
    out = a.out or f"{a.id}-guide"
    render(paths, f"{out}.svg")
    json.dump({"id": a.id, "metrics": m,
               "guide": ["M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in p) for p in paths]},
              open(f"{out}.json", "w"), indent=1)


if __name__ == "__main__":
    main()
