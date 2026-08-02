#!/usr/bin/env python3
"""Pilot: digits 0-9 through the canonical-glyph pipeline (DejaVu Sans
Bold for sturdier strokes)."""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import os

S = os.path.dirname(os.path.abspath(__file__))
FONT = f"{S}/DejaVuSans.ttf"
K = 13

for n in range(10):
    ch = str(n)
    f = ImageFont.truetype(FONT, 760)
    x0, y0, x1, y1 = f.getbbox(ch)
    big = Image.new("L", (x1-x0+80, y1-y0+80), 255)
    ImageDraw.Draw(big).text((40-x0, 40-y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(K))
    bb = big.point(lambda v: 255-v).getbbox()
    big = big.crop(bb)
    s = 430 / max(big.size)
    big = big.resize((max(1, round(big.size[0]*s)), max(1, round(big.size[1]*s))), Image.LANCZOS)
    big = big.point(lambda v: 0 if v < 128 else 255)
    out = Image.new("L", (512, 512), 255)
    # OPTICAL centering: put the ink's center of MASS on screen center —
    # a bare bbox-center leaves 7/4/9 looking shoved to one side. Clamped
    # so the pad never drops below 26px.
    px = big.load()
    sx = sy = m = 0
    for yy in range(big.size[1]):
        for xx in range(big.size[0]):
            if px[xx, yy] < 128:
                sx += xx; sy += yy; m += 1
    cx, cy = sx / m, sy / m
    ox = min(max(round(256 - cx), 26), 512 - big.size[0] - 26)
    oy = min(max(round(256 - cy), 26), 512 - big.size[1] - 26)
    out.paste(big, (ox, oy))
    out.save(f"{S}/digit{n}.png")
print("digits rendered")
