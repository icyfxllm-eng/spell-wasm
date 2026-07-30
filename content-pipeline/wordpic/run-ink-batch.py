#!/usr/bin/env python3
"""v7.5 — re-render the 13-subject batch through the ink branch (mona =
photo branch vs pinned annotation) and grade with the three-gate eval."""
import json, pathlib, re, subprocess, sys, base64, io
from PIL import Image, ImageOps
ROOT = pathlib.Path(__file__).resolve().parents[2]
TP = ROOT / "target/release/trace-pgm"
IE = ROOT / "target/release/ink-eval"
REF = pathlib.Path(__file__).parent / "ref"
OUT = pathlib.Path(sys.argv[1])
OUT.mkdir(exist_ok=True)
CANVAS = 512
SUBJ = {"dog":"dog.png","butterfly":"butterfly.png","duck":"duck.png","owl":"owl.png",
        "turtle":"turtle.png","elephant":"elephant.png","horse":"horse.png","fish":"fish.png",
        "snowman":"snowman.png","eiffel":"eiffel.png","dragon":"dragon.jpg","peacock":"peacock.jpg",
        "mona":"mona-lisa.jpg"}
def canvas_im(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255,255,255,255)); bg.alpha_composite(im)
    im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS-44, CANVAS-44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS-im.width)//2, (CANVAS-im.height)//2))
    return c
def as_pgm(c):
    return b"P5 %d %d 255\n" % c.size + c.tobytes()
results = []
for sub, ref in SUBJ.items():
    c = canvas_im(REF/ref); pgm = as_pgm(c)
    if sub == "mona":
        # photo branch: v7.4 trace, graded against the PINNED annotation
        r = json.loads(subprocess.run([str(TP), "expert", "frame=143,40,372,484"], input=pgm, capture_output=True).stdout)
        paths = [p["points"] for p in r["paths"]]
        ex = json.load(open(REF/"mona-exemplar.json"))
        truth = [ex["red_frame"], ex["red_figure"], ex["red_hands"], ex["blue_hair_face"], ex["blue_neck_shoulders"]]
        json.dump(paths, open("/tmp/p.json", "w"))
        # grade by sampling annotation as skeleton: use ink-eval on synthetic skel? simplest: python distances
        import math
        def sd(p, a, b):
            vx, vy = b[0]-a[0], b[1]-a[1]; l2 = vx*vx+vy*vy
            t = 0 if l2 == 0 else max(0, min(1, ((p[0]-a[0])*vx+(p[1]-a[1])*vy)/l2))
            return math.hypot(p[0]-(a[0]+t*vx), p[1]-(a[1]+t*vy))
        tau = 0.01 * (2 * CANVAS**2) ** 0.5
        tpts = []
        for line in truth:
            for a, b in zip(line, line[1:]):
                n = max(1, int(math.hypot(b[0]-a[0], b[1]-a[1]) / 4))
                tpts += [(a[0]+(b[0]-a[0])*k/n, a[1]+(b[1]-a[1])*k/n) for k in range(n+1)]
        near = lambda pt, lines: any(sd(pt, a, b) <= tau for l in lines for a, b in zip(l, l[1:]))
        rec = sum(near(t, paths) for t in tpts) / len(tpts)
        ppts = []
        for l in paths:
            for a, b in zip(l, l[1:]):
                n = max(1, int(math.hypot(b[0]-a[0], b[1]-a[1]) / 4))
                ppts += [(a[0]+(b[0]-a[0])*k/n, a[1]+(b[1]-a[1])*k/n) for k in range(n+1)]
        prec = sum(near(p, truth) for p in ppts) / max(1, len(ppts))
        m = {"subject": sub, "class": "photo(annotation)", "ink_recall": rec, "path_precision": prec,
             "components": len(truth), "unmatched": sum(1 for l in truth if not all(near((x, y), paths) for x, y in l[:3])), "component_coverage": rec >= 0.97}
        residual = [t for t in tpts if not near(t, paths)]
    else:
        r = json.loads(subprocess.run([str(TP), "ink"], input=pgm, capture_output=True).stdout)
        paths = r["paths"]
        json.dump(paths, open("/tmp/p.json", "w"))
        m = json.loads(subprocess.run([str(IE), "/tmp/p.json", "/tmp/res.json"], input=pgm, capture_output=True).stdout)
        m["subject"] = sub
        residual = json.load(open("/tmp/res.json"))
    # renders
    def d(p):
        return "M" + " L".join(f"{x:.0f} {y:.0f}" for x, y in p)
    stroke_paths = "".join(f'<path d="{d(p)}" fill="none" stroke="#e8ecf5" stroke-opacity="0.9" stroke-width="2.4" stroke-linecap="round"/>' for p in paths)
    open(OUT/f"{sub}-outline.svg", "w").write(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512"><rect width="512" height="512" fill="#101623"/>{stroke_paths}</svg>')
    buf = io.BytesIO(); Image.merge("RGB", [c, c, c]).save(buf, "JPEG", quality=68)
    b64 = base64.b64encode(buf.getvalue()).decode()
    opaths = "".join(f'<path d="{d(p)}" fill="none" stroke="#e8b44f" stroke-opacity="0.9" stroke-width="2"/>' for p in paths)
    dots = "".join(f'<circle cx="{x}" cy="{y}" r="1.6" fill="#ff5d5d"/>' for x, y in residual[::3])
    open(OUT/f"{sub}-overlay.svg", "w").write(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512">'
        f'<image href="data:image/jpeg;base64,{b64}" width="512" height="512" opacity="0.35"/>{opaths}{dots}</svg>')
    m["paths"] = len(paths); m["residual_px"] = len(residual)
    results.append(m)
    gates = m["ink_recall"] >= 0.97 and m["path_precision"] >= 0.97 and m["component_coverage"]
    print(f'{sub:10} class={m["class"]:6} paths={len(paths):4} recall={m["ink_recall"]:.3f} precision={m["path_precision"]:.3f} '
          f'unmatched={m["unmatched"]} residual={len(residual):5} -> {"gates met" if gates else "gates NOT met"}')
json.dump(results, open(OUT/"ink-metrics.json", "w"), indent=1)
