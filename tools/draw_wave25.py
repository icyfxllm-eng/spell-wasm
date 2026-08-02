#!/usr/bin/env python3
"""Batch 17 — penguin, anchor, stoplight, snowflake (bold), dartboard.
Weld-first; interior white details keep >=20px dark frames; roster checked
BEFORE drawing this time."""
from PIL import Image, ImageDraw
import math, os

OUT = os.path.dirname(os.path.abspath(__file__))
BLACK, WHITE = 0, 255

def canvas():
    im = Image.new("L", (512, 512), WHITE)
    return im, ImageDraw.Draw(im)

def save(im, name):
    im.save(f"{OUT}/{name}.png")
    print(name, "ok")

# ---------------- penguin ----------------
im, d = canvas()
d.ellipse([146, 90, 366, 470], fill=BLACK)          # body egg
d.ellipse([118, 230, 178, 410], fill=BLACK)         # left flipper (30 into body)
d.ellipse([334, 230, 394, 410], fill=BLACK)         # right flipper
d.ellipse([176, 450, 256, 488], fill=BLACK)         # left foot
d.ellipse([256, 450, 336, 488], fill=BLACK)         # right foot
d.ellipse([186, 226, 326, 444], fill=WHITE)         # belly (big hostable ring)
d.ellipse([217, 118, 241, 142], fill=WHITE)         # left eye (micro)
d.ellipse([271, 118, 295, 142], fill=WHITE)         # right eye (micro)
d.polygon([(242, 168), (270, 168), (256, 194)], fill=WHITE)  # beak (micro)
save(im, "penguin")

# ---------------- anchor ----------------
im, d = canvas()
d.ellipse([126, 210, 386, 470], fill=BLACK)         # fluke arc outer
d.ellipse([166, 250, 346, 430], fill=WHITE)         # arc inner (40px band)
d.rectangle([110, 190, 402, 340], fill=WHITE)       # cut: keep bottom semicircle
d.polygon([(112, 296), (172, 344), (130, 388)], fill=BLACK)  # left barb, welded
d.polygon([(400, 296), (340, 344), (382, 388)], fill=BLACK)  # right barb
d.rectangle([176, 120, 336, 156], fill=BLACK)       # stock (crossbar)
d.rectangle([238, 66, 274, 460], fill=BLACK)        # shank (36 wide, 30 into arc)
d.ellipse([222, 30, 290, 98], fill=BLACK)           # ring outer
d.ellipse([246, 54, 266, 74], fill=WHITE)           # ring hole (micro)
save(im, "anchor")

# ---------------- stoplight ----------------
im, d = canvas()
d.rounded_rectangle([166, 56, 346, 424], radius=28, fill=BLACK)  # housing
for cy in (126, 238, 350):                           # three lights, 28px bands between
    d.ellipse([214, cy - 42, 298, cy + 42], fill=WHITE)
d.rectangle([238, 398, 274, 484], fill=BLACK)        # pole (26 into housing)
d.rectangle([196, 464, 316, 488], fill=BLACK)        # base (20 over pole)
save(im, "stoplight")

# ---------------- snowflake (bold) ----------------
im, d = canvas()
cx = cy = 256
d.ellipse([cx - 46, cy - 46, cx + 46, cy + 46], fill=BLACK)      # hub
for k in range(6):
    a = math.radians(k * 60)
    tx, ty = cx + 210 * math.cos(a), cy + 210 * math.sin(a)
    d.line([(cx, cy), (tx, ty)], fill=BLACK, width=40)           # arm
    d.ellipse([tx - 20, ty - 20, tx + 20, ty + 20], fill=BLACK)  # tip cap
    for sgn in (1, -1):                                          # chevron branchlets
        bx, by = cx + 120 * math.cos(a), cy + 120 * math.sin(a)
        ba = a + sgn * math.radians(55)
        ex, ey = bx + 62 * math.cos(ba), by + 62 * math.sin(ba)
        d.line([(bx, by), (ex, ey)], fill=BLACK, width=30)
        d.ellipse([ex - 15, ey - 15, ex + 15, ey + 15], fill=BLACK)
save(im, "snowflake")

# ---------------- dartboard ----------------
im, d = canvas()
def ring(r, col):
    d.ellipse([256 - r, 256 - r, 256 + r, 256 + r], fill=col)
ring(214, BLACK)   # outer band 178..214 (36px, a spelling ring)
ring(178, WHITE)
ring(144, BLACK)   # middle band 108..144
ring(108, WHITE)
ring(72, BLACK)    # inner band 36..72
ring(36, WHITE)
ring(16, BLACK)    # bullseye (micro; RED pends the tint extension)
save(im, "dartboard")
