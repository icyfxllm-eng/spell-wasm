#!/usr/bin/env python3
"""Batch 20 — the retakes. spiderweb aims at EXPERT: big, dense, every
cell a real spelling ring; volcano gets drama. All welds >=28px."""
from PIL import Image, ImageDraw
import math, os

OUT = os.path.dirname(os.path.abspath(__file__))
BLACK, WHITE = 0, 255
CX = CY = 256

def canvas():
    im = Image.new("L", (512, 512), WHITE)
    return im, ImageDraw.Draw(im)

def save(im, name):
    im.save(f"{OUT}/{name}.png")
    print(name, "ok")

def pol(r, deg):
    a = math.radians(deg)
    return (CX + r * math.cos(a), CY + r * math.sin(a))

# ---------------- spiderweb (expert) ----------------
im, d = canvas()
ANGLES = [k * 60 + 30 for k in range(6)]
for deg in ANGLES:                                        # six strands
    d.line([pol(0, deg), pol(215, deg)], fill=BLACK, width=30)
    x, y = pol(215, deg)
    d.ellipse([x-16, y-16, x+16, y+16], fill=BLACK)       # anchor knots
d.ellipse([CX-24, CY-24, CX+24, CY+24], fill=BLACK)       # hub
for R, sag in ((148, 16), (207, 18)):                     # two scalloped orbits
    for i in range(6):
        a1, a2 = ANGLES[i], ANGLES[i] + 60
        pts = []
        for t in range(0, 11):
            f = t / 10
            deg = a1 + (a2 - a1) * f
            rr = R - sag * math.sin(math.pi * f)          # sag toward the hub
            pts.append(pol(rr, deg))
        d.line(pts, fill=BLACK, width=30, joint="curve")
for i in range(6):                                        # a dew drop on EVERY scallop
    for R, sag in ((148, 16), (207, 18)):
        x, y = pol(R - sag, ANGLES[i] + 30)
        d.ellipse([x-15, y-15, x+15, y+15], fill=BLACK)
save(im, "spiderweb")

# ---------------- volcano (retake) ----------------
im, d = canvas()
d.polygon([(60, 470), (96, 468), (150, 380), (188, 300), (210, 232), (222, 186),
           (240, 186), (250, 214), (262, 186), (282, 186), (292, 224), (302, 186),
           (318, 186), (330, 244), (352, 310), (400, 396), (452, 470)], fill=BLACK)  # concave flanks, jagged rim
d.ellipse([56, 444, 214, 502], fill=BLACK)                # left foothill, merged into the flank
d.ellipse([300, 444, 462, 502], fill=BLACK)               # right foothill, merged
d.rectangle([244, 96, 296, 226], fill=BLACK)              # eruption column into the throat
d.ellipse([160, 40, 300, 128], fill=BLACK)                # cloud lobe west
d.ellipse([240, 26, 388, 118], fill=BLACK)                # cloud lobe east
d.ellipse([206, 14, 330, 92], fill=BLACK)                 # cloud crown
d.polygon([(206, 216), (240, 192), (128, 134)], fill=BLACK)  # lava spurt left, rooted in the rim
d.polygon([(330, 252), (358, 276), (426, 198)], fill=BLACK)  # lava spurt right, low on the flank
# lava rivers: meandering channels carved into the rock, OPEN at the rim
# (they meet the sky beside the eruption column) so they are silhouette
# cuts, never enclosed slits. Dark rock stays >=34 on every side.
d.polygon([(222, 180), (246, 190), (238, 246), (220, 296), (234, 344),
           (206, 348), (196, 294), (214, 244)], fill=WHITE)   # west river
d.polygon([(298, 182), (322, 192), (330, 246), (312, 296), (326, 346),
           (298, 352), (288, 296), (304, 244)], fill=WHITE)   # east river
save(im, "volcano")
