#!/usr/bin/env python3
"""Batch 19 take two — the twelve zodiac glyphs at their CANONICAL
typographic proportions (the same forms Apple's emoji render), from
DejaVu Sans (Bitstream Vera license, free), stroke-thickened to field
spec. Per-glyph dilation keeps every enclosed counter hostable."""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import os

S = os.path.dirname(os.path.abspath(__file__))
FONT = f"{S}/DejaVuSans.ttf"
SIGNS = [("aries", 0x2648), ("taurus", 0x2649), ("gemini", 0x264A), ("cancer", 0x264B),
         ("leo", 0x264C), ("virgo", 0x264D), ("libra", 0x264E), ("scorpio", 0x264F),
         ("sagittarius", 0x2650), ("capricorn", 0x2651), ("aquarius", 0x2652), ("pisces", 0x2653)]
# dilation kernel per glyph (odd; grows ink (k-1)/2 px each side)
K = {"aries": 15, "taurus": 13, "gemini": 15, "cancer": 13, "leo": 13, "virgo": 13,
     "libra": 15, "scorpio": 13, "sagittarius": 15, "capricorn": 13, "aquarius": 15, "pisces": 15}

for name, cp in SIGNS:
    ch = chr(cp)
    # render big, then fit into 512 with margins
    f = ImageFont.truetype(FONT, 760)
    x0, y0, x1, y1 = f.getbbox(ch)
    w, h = x1 - x0, y1 - y0
    big = Image.new("L", (w + 80, h + 80), 255)
    ImageDraw.Draw(big).text((40 - x0, 40 - y0), ch, font=f, fill=0)
    big = big.filter(ImageFilter.MinFilter(K[name]))       # thicken the ink
    bb = big.point(lambda v: 255 - v).getbbox()
    big = big.crop(bb)
    # scale longest side to 430, center on 512
    s = 430 / max(big.size)
    big = big.resize((max(1, round(big.size[0] * s)), max(1, round(big.size[1] * s))), Image.LANCZOS)
    big = big.point(lambda v: 0 if v < 128 else 255)
    out = Image.new("L", (512, 512), 255)
    out.paste(big, ((512 - big.size[0]) // 2, (512 - big.size[1]) // 2))
    out.save(f"{S}/{name}.png")
    print(name, "ok", big.size)
