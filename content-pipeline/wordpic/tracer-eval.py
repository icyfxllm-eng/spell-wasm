#!/usr/bin/env python3
"""v7.5 F6 — calibration run: grade the FROZEN corpus with the new
three-gate eval. The gate: all 13 must FAIL, matching Eric's notebook."""
import json, pathlib, re, subprocess, sys
from PIL import Image, ImageOps
ROOT = pathlib.Path(__file__).resolve().parents[2]
BIN = ROOT / "target/release/ink-eval"
REF = pathlib.Path(__file__).parent / "ref"
CORPUS = pathlib.Path(__file__).parent / "frozen-corpus-v75"
CANVAS = 512
SUBJ = {"dog":"dog.png","butterfly":"butterfly.png","duck":"duck.png","owl":"owl.png",
        "turtle":"turtle.png","elephant":"elephant.png","horse":"horse.png","fish":"fish.png",
        "snowman":"snowman.png","eiffel":"eiffel.png","mona":"mona-lisa.jpg",
        "peacock":"peacock.jpg","dragon":"dragon.jpg"}
def pgm(path):
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (255,255,255,255)); bg.alpha_composite(im)
    im = ImageOps.autocontrast(bg.convert("L"), cutoff=1)
    im.thumbnail((CANVAS-44, CANVAS-44), Image.LANCZOS)
    c = Image.new("L", (CANVAS, CANVAS), 255)
    c.paste(im, ((CANVAS-im.width)//2, (CANVAS-im.height)//2))
    return b"P5 %d %d 255\n" % c.size + c.tobytes()
def paths_from_svg(svg):
    out = []
    for d in re.findall(r'\bd="(M[^"]+)"', svg):
        pts = re.findall(r'(-?[\d.]+)[ ,](-?[\d.]+)', d)
        pts = [(float(x), float(y)) for x, y in pts]
        if len(pts) >= 2: out.append(pts)
    return out
fails = 0
for sub, ref in SUBJ.items():
    svg = (CORPUS / f"{sub}-outline.svg").read_text()
    pf = "/tmp/paths.json"
    json.dump(paths_from_svg(svg), open(pf, "w"))
    r = json.loads(subprocess.run([str(BIN), pf], input=pgm(REF/ref), capture_output=True).stdout)
    ok = r["ink_recall"] >= 0.97 and r["path_precision"] >= 0.97 and r["component_coverage"]
    if not ok: fails += 1
    print(f'{sub:10} class={r["class"]:5} recall={r["ink_recall"]:.3f} precision={r["path_precision"]:.3f} '
          f'comps={r["components"]} unmatched={r["unmatched"]} -> {"FAIL" if not ok else "would-pass (CALIBRATION BUG)"}')
print(f"\ncalibration gate: {fails}/13 FAIL " + ("— HOLDS (matches the notebook)" if fails == 13 else "— DOES NOT HOLD"))
sys.exit(0 if fails == 13 else 1)
