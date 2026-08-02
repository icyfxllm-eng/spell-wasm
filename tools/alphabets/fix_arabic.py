#!/usr/bin/env python3
"""The eight Arabic stragglers: render the body at moderate thickness,
then (a) any detached small component (a dot) becomes a clean r=15 micro
disc at its centroid, and (b) any enclosed counter whose size sits in the
untrusted 130-250 band is filled solid. Bodies keep their canonical form."""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import json, math, os, subprocess

S = os.path.dirname(os.path.abspath(__file__))
SUGGEST = "/Users/eric/Desktop/xcode-backup/repos/spell-wasm/target/release/suggest"
CASES = {"arb02": "ب", "arb03": "ت", "arb05": "ج", "arb07": "خ",
         "arb19": "غ", "arb20": "ف", "arb21": "ق", "arb28": "ي"}

def components(img, val):
    w, h = img.size
    px = img.load()
    seen = [[False]*w for _ in range(h)]
    comps = []
    for y0 in range(h):
        for x0 in range(w):
            if px[x0, y0] == val and not seen[y0][x0]:
                stack = [(x0, y0)]; seen[y0][x0] = True; pts = []
                while stack:
                    x, y = stack.pop(); pts.append((x, y))
                    for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
                        nx, ny = x+dx, y+dy
                        if 0 <= nx < w and 0 <= ny < h and px[nx, ny] == val and not seen[ny][nx]:
                            seen[ny][nx] = True; stack.append((nx, ny))
                comps.append(pts)
    return comps

for sub, ch in CASES.items():
    f = ImageFont.truetype(f"{S}/NotoSansArabic.ttf", 700)
    x0, y0, x1, y1 = f.getbbox(ch)
    big = Image.new("L", (x1-x0+100, y1-y0+100), 255)
    ImageDraw.Draw(big).text((50-x0, 50-y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(13))
    bb = big.point(lambda v: 255-v).getbbox()
    big = big.crop(bb)
    s = 430 / max(big.size)
    big = big.resize((max(1, round(big.size[0]*s)), max(1, round(big.size[1]*s))), Image.LANCZOS)
    big = big.point(lambda v: 0 if v < 128 else 255)
    d = ImageDraw.Draw(big)
    # (a) dots -> micro discs
    inks = sorted(components(big, 0), key=len, reverse=True)
    for comp in inks[1:]:
        cx = sum(p[0] for p in comp) / len(comp)
        cy = sum(p[1] for p in comp) / len(comp)
        for x, y in comp:
            d.point((x, y), fill=255)
        d.ellipse([cx-15, cy-15, cx+15, cy+15], fill=0)
    # (b) enclosed counters in the untrusted band -> solid
    w, h = big.size
    whites = components(big, 255)
    for comp in whites:
        touches_border = any(x == 0 or y == 0 or x == w-1 or y == h-1 for x, y in comp)
        if touches_border:
            continue
        r_est = math.sqrt(len(comp) / math.pi)
        perim = 2 * math.pi * r_est
        if 110 < perim < 270:
            for x, y in comp:
                d.point((x, y), fill=0)
    out = Image.new("L", (512, 512), 255)
    px = big.load(); sx = sy = m = 0
    for yy in range(h):
        for xx in range(w):
            if px[xx, yy] < 128:
                sx += xx; sy += yy; m += 1
    ox = min(max(round(256 - sx/m), 26), 512 - w - 26)
    oy = min(max(round(256 - sy/m), 26), 512 - h - 26)
    out.paste(big, (ox, oy))
    out.save(f"{S}/{sub}.png")
    j = f"{S}/sugg_arb/{sub}.json"
    if os.path.exists(j):
        os.remove(j)
    subprocess.run([SUGGEST, f"{S}/{sub}.png", f"{S}/sugg_arb"], capture_output=True)
    dd = json.load(open(j))
    probs = []
    for c in dd["candidates"]:
        if c["flagged"]: probs.append("flag")
        if not c["micro"] and c["id"] > 0 and 130 < c["arc"] < 250: probs.append(f"dz{round(c['arc'])}")
        if c["clearance"] < 20: probs.append(f"tight{c['clearance']:.0f}")
    print(sub, repr(ch), "CLEAN" if not probs else probs)

man = json.load(open(f"{S}/alpha_manifest.json"))
for sub, ch in CASES.items():
    man[sub] = {"set": "arb", "char": ch, "k": "13+post", "problems": []}
json.dump(man, open(f"{S}/alpha_manifest.json", "w"), ensure_ascii=False, indent=1)
