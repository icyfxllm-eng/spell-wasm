#!/usr/bin/env python3
"""Rerun the 18 stragglers with a wider dilation search, both directions."""
import json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import importlib.util
S = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("gen", f"{S}/gen_alphabets.py")
# reuse render/audit by re-defining minimal copies
from PIL import Image, ImageDraw, ImageFont, ImageFilter
SUGGEST = "/Users/eric/repos/spell-wasm/target/release/suggest"
FONT_OF = {"cyr": "DejaVuSans.ttf", "arb": "NotoSansArabic.ttf", "hir": "NotoSansJP.ttf"}

def render(ch, fontfile, k, fit=430):
    f = ImageFont.truetype(f"{S}/{fontfile}", 700)
    x0, y0, x1, y1 = f.getbbox(ch)
    big = Image.new("L", (x1-x0+100, y1-y0+100), 255)
    ImageDraw.Draw(big).text((50-x0, 50-y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(k))
    bb = big.point(lambda v: 255-v).getbbox()
    big = big.crop(bb)
    s = fit / max(big.size)
    big = big.resize((max(1, round(big.size[0]*s)), max(1, round(big.size[1]*s))), Image.LANCZOS)
    big = big.point(lambda v: 0 if v < 128 else 255)
    out = Image.new("L", (512, 512), 255)
    px = big.load(); sx = sy = m = 0
    for yy in range(big.size[1]):
        for xx in range(big.size[0]):
            if px[xx, yy] < 128:
                sx += xx; sy += yy; m += 1
    ox = min(max(round(256 - sx/m), 26), 512 - big.size[0] - 26)
    oy = min(max(round(256 - sy/m), 26), 512 - big.size[1] - 26)
    out.paste(big, (ox, oy))
    return out

def audit(sub, outdir):
    d = json.load(open(f"{outdir}/{sub}.json"))
    probs = []
    for c in d["candidates"]:
        if c["flagged"]: probs.append("flag")
        if not c["micro"] and c["id"] > 0 and 130 < c["arc"] < 250: probs.append(f"dz{round(c['arc'])}")
        if c["clearance"] < 20: probs.append(f"tight{c['clearance']:.0f}")
    return probs

man = json.load(open(f"{S}/alpha_manifest.json"))
dirty = {k: v for k, v in man.items() if v["problems"]}
for sub, info in dirty.items():
    setid, ch = info["set"], info["char"]
    outdir = f"{S}/sugg_{setid}"
    fixed = False
    for k, fit in [(7,430),(9,430),(17,430),(19,430),(21,430),(9,460),(7,460),(23,430),(25,430)]:
        im = render(ch, FONT_OF[setid], k, fit)
        im.save(f"{S}/{sub}.png")
        if os.path.exists(f"{outdir}/{sub}.json"): os.remove(f"{outdir}/{sub}.json")
        subprocess.run([SUGGEST, f"{S}/{sub}.png", outdir], capture_output=True)
        probs = audit(sub, outdir)
        if not probs:
            man[sub] = {"set": setid, "char": ch, "k": k, "problems": []}
            print(f"{sub} {ch!r}: CLEAN at k={k} fit={fit}")
            fixed = True
            break
    if not fixed:
        print(f"{sub} {ch!r}: STILL DIRTY -> {probs}")
json.dump(man, open(f"{S}/alpha_manifest.json", "w"), ensure_ascii=False, indent=1)
left = [k for k, v in man.items() if v["problems"]]
print("remaining dirty:", left if left else "none — all 213 clean")
