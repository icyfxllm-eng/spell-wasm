#!/usr/bin/env python3
"""v7 F8 driver: every reference through tracer-core -> outline SVG +
contour overlay + measured gates. Output feeds Eric's outline review."""
import base64, io, json, pathlib, subprocess, sys
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
BIN = ROOT / "target/release/trace-pgm"
REF = pathlib.Path(__file__).parent / "ref"
OUT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "trace-out")
OUT.mkdir(exist_ok=True)
CANVAS = 512
SUBJECTS = {  # subject -> (file, budget)
    "dog": ("dog.png", "standard"), "butterfly": ("butterfly.png", "standard"),
    "snail": ("snail.png", "standard"), "duck": ("duck.png", "standard"),
    "owl": ("owl.png", "standard"), "turtle": ("turtle.png", "standard"),
    "elephant": ("elephant.png", "standard"), "horse": ("horse.png", "expert"),
    "peacock": ("peacock.jpg", "expert"), "fish": ("fish.png", "standard"),
    "snowman": ("snowman.png", "standard"), "eiffel": ("eiffel.png", "standard"),
    "dragon": ("dragon.jpg", "expert"), "mona": ("mona-lisa.jpg", "expert"),
}

def canvas_gray(path, gamma=1.0, channel=None, invert=False):
    from PIL import ImageOps
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    bg.alpha_composite(im)
    im = bg.convert("RGB").getchannel(channel) if channel else bg.convert("L")
    if invert:
        from PIL import ImageOps as _io
        im = _io.invert(im)
    corner = im.getpixel((2, 2))
    if 60 < corner < 235:
        lo, hi = corner - 18, corner + 18
        im = im.point(lambda v: 255 if lo <= v <= hi else v)
    im = ImageOps.autocontrast(im, cutoff=1)
    if gamma != 1.0:
        lut = [min(255, int(255 * (v / 255) ** gamma)) for v in range(256)]
        im = im.point(lut)
    im.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    canvas = Image.new("L", (CANVAS, CANVAS), 255)
    ox, oy = (CANVAS - im.width) // 2, (CANVAS - im.height) // 2
    canvas.paste(im, (ox, oy))
    return canvas

metrics = []
GAMMA = {"mona": 1.45}
CHANNEL = {}
INVERT = set()

def dragon_gray():
    """Hokusai panel: subject = everything NOT the red ground. Red dominance
    R - (G+B)/2 is high on the ground, low/negative on the dragon."""
    im = Image.open(REF / "dragon.jpg").convert("RGB")
    r, g, b = [list(ch.getdata()) for ch in im.split()]
    px = [255 if (rv - (gv + bv) // 2) > 40 or (rv + gv + bv) // 3 > 195 else 40
          for rv, gv, bv in zip(r, g, b)]
    out = Image.new("L", im.size)
    out.putdata(px)
    out.thumbnail((CANVAS - 44, CANVAS - 44), Image.LANCZOS)
    canvas = Image.new("L", (CANVAS, CANVAS), 255)
    canvas.paste(out, ((CANVAS - out.width) // 2, (CANVAS - out.height) // 2))
    return canvas
for sub, (fname, budget) in SUBJECTS.items():
    g = dragon_gray() if sub == "dragon" else canvas_gray(REF / fname, GAMMA.get(sub, 1.0), CHANNEL.get(sub), sub in INVERT)
    pgm = b"P5 %d %d 255\n" % g.size + g.tobytes()
    def run(args):
        return json.loads(subprocess.run(args, input=pgm, capture_output=True).stdout)
    r = run([str(BIN), budget])
    lasso_file = REF / f"{sub}-lasso.json"
    unseeded_fail = not (r["deviation"] <= 0.015 and r["coverage"] >= 0.95 and r.get("smooth_viol", 0) == 0)
    if lasso_file.exists() and (unseeded_fail or sub == "peacock"):
        # v7.2 routing: a failed unseeded trace is never an output — the
        # lasso (Eric's rough loop / the future finger-circle) seeds the
        # subject mask. Low-contrast subjects are lasso-first by default.
        pts = json.load(open(lasso_file))
        arg = ";".join(f"{x},{y}" for x, y in pts)
        r = run([str(BIN), budget, arg])
        r["seeded"] = True
    if sub == "mona":
        # v7.1: figure mask (background interiors filtered per D6) + ONE
        # frame path (the only legal non-mask contour class).
        keep = []
        for p in r["paths"]:
            ys = [y for _, y in p["points"]]
            if p["silhouette"] or (min(ys) > 215):
                keep.append(p)
        r["paths"] = keep
        r["paths"].append({"points": [[143, 40], [372, 40], [372, 484], [143, 484], [143, 40]],
                           "band": 2, "scale": "long", "silhouette": False, "frame": True})
    if sub == "snail":
        # Eric's round-2 direction: outline only — outer shell + body (the
        # eyes and smile get authored as labeled features in curation).
        r["paths"] = [p for p in r["paths"] if p["silhouette"]]
    # outline SVG
    def pts(p):
        return "M" + " L".join(f"{x:.0f} {y:.0f}" for x, y in p["points"])
    paths = "".join(
        f'<path d="{pts(p)}" fill="none" stroke="{"#e8ecf5" if p["silhouette"] else "#9fb3d9"}"'
        f' stroke-opacity="{0.9 if p["silhouette"] else 0.55}" stroke-width="{4 if p["silhouette"] else 2.5}"'
        ' stroke-linecap="round" stroke-linejoin="round"/>'
        for p in r["paths"])
    open(OUT / f"{sub}-outline.svg", "w").write(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512">'
        f'<rect width="512" height="512" fill="#101623"/>{paths}</svg>')
    # contour overlay: faded reference under accent paths
    buf = io.BytesIO()
    canvas_rgb = Image.merge("RGB", [g, g, g])
    canvas_rgb.save(buf, "JPEG", quality=70)
    b64 = base64.b64encode(buf.getvalue()).decode()
    opaths = "".join(
        f'<path d="{pts(p)}" fill="none" stroke="#e8b44f" stroke-opacity="0.95" stroke-width="2.5"/>'
        for p in r["paths"])
    open(OUT / f"{sub}-overlay.svg", "w").write(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512">'
        f'<image href="data:image/jpeg;base64,{b64}" width="512" height="512" opacity="0.42"/>{opaths}</svg>')
    metrics.append({"subject": sub, "budget": budget, "paths": len(r["paths"]),
                    "deviation": r["deviation"], "coverage": r["coverage"],
                    "dev_ok": r["deviation"] <= 0.015, "cov_ok": r["coverage"] >= 0.95})
    print(f'{sub:10} paths {len(r["paths"]):3}  dev {r["deviation"]*100:5.2f}%  cov {r["coverage"]*100:5.1f}%  '
          f'{"PASS" if r["deviation"] <= 0.015 and r["coverage"] >= 0.95 else "FAIL"}')
json.dump(metrics, open(OUT / "metrics.json", "w"), indent=1)
