#!/usr/bin/env python3
"""Re-render the 12 shipped zodiac glyphs with the same OPTICAL centering
the rest of the program uses — identical forms, translated only."""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import os

S = os.path.dirname(os.path.abspath(__file__))
SIGNS = [("aries", 0x2648), ("taurus", 0x2649), ("gemini", 0x264A), ("cancer", 0x264B),
         ("leo", 0x264C), ("virgo", 0x264D), ("libra", 0x264E), ("scorpio", 0x264F),
         ("sagittarius", 0x2650), ("capricorn", 0x2651), ("aquarius", 0x2652), ("pisces", 0x2653)]
K = {"aries": 15, "taurus": 13, "gemini": 15, "cancer": 13, "leo": 13, "virgo": 13,
     "libra": 15, "scorpio": 13, "sagittarius": 15, "capricorn": 13, "aquarius": 15, "pisces": 15}

for name, cp in SIGNS:
    ch = chr(cp)
    f = ImageFont.truetype(f"{S}/DejaVuSans.ttf", 760)
    x0, y0, x1, y1 = f.getbbox(ch)
    big = Image.new("L", (x1-x0+80, y1-y0+80), 255)
    ImageDraw.Draw(big).text((40-x0, 40-y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(K[name]))
    bb = big.point(lambda v: 255-v).getbbox()
    big = big.crop(bb)
    s = 430 / max(big.size)
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
    out.save(f"{S}/{name}.png")
    print(name, "recentered")
