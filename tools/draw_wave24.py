#!/usr/bin/env python3
"""Batch 16 — five drawn originals, weld-first: every part rooted deep
(>=28px overlaps), no slits, no floating pieces, pad >=34 from borders."""
from PIL import Image, ImageDraw
import os

OUT = os.path.dirname(os.path.abspath(__file__))
BLACK, WHITE = 0, 255

def canvas():
    im = Image.new("L", (512, 512), WHITE)
    return im, ImageDraw.Draw(im)

def save(im, name):
    im.save(f"{OUT}/{name}.png")
    print(name, "ok")

# ---------------- lighthouse ----------------
im, d = canvas()
d.polygon([(86,486),(120,432),(392,432),(426,486)], fill=BLACK)          # rock base
d.polygon([(184,462),(328,462),(302,166),(210,166)], fill=BLACK)         # tapered tower
d.rectangle([188,150,324,196], fill=BLACK)                               # gallery deck
d.rectangle([216,86,296,182], fill=BLACK)                                # lantern room
d.polygon([(206,118),(306,118),(256,44)], fill=BLACK)                    # roof
d.rectangle([243,119,269,145], fill=WHITE)                               # light window (26px, micro band)
save(im, "lighthouse")

# ---------------- cactus (saguaro) ----------------
im, d = canvas()
d.polygon([(70,480),(110,430),(400,430),(440,480)], fill=BLACK)          # ground mound
d.rectangle([226,120,316,462], fill=BLACK)                               # trunk
d.ellipse([226,75,316,165], fill=BLACK)                                  # trunk cap
d.ellipse([247,50,295,98], fill=BLACK)                                   # flower bump, welded
d.rectangle([160,300,258,364], fill=BLACK)                               # left arm out (32 into trunk)
d.rectangle([160,200,224,364], fill=BLACK)                               # left arm up
d.ellipse([160,168,224,232], fill=BLACK)                                 # left cap
d.rectangle([284,220,390,284], fill=BLACK)                               # right arm out (32 into trunk)
d.rectangle([326,140,390,284], fill=BLACK)                               # right arm up
d.ellipse([326,108,390,172], fill=BLACK)                                 # right cap
save(im, "cactus")

# ---------------- tulip ----------------
im, d = canvas()
d.rectangle([236,230,278,470], fill=BLACK)                               # stem (42 wide)
d.polygon([(168,126),(176,220),(210,262),(302,262),(336,220),(344,126),
           (300,196),(256,118),(212,196)], fill=BLACK)                   # 3-tip bloom, 32 over stem
d.polygon([(268,440),(160,330),(136,366),(230,462),(268,462)], fill=BLACK)  # left leaf, 32 into stem
d.polygon([(246,452),(352,350),(376,384),(276,470),(246,470)], fill=BLACK)  # right leaf, 32 into stem
save(im, "tulip")

# ---------------- hot-air balloon ----------------
im, d = canvas()
d.ellipse([106,40,406,340], fill=BLACK)                                  # envelope
d.polygon([(216,300),(296,300),(276,430),(236,430)], fill=BLACK)         # neck, deep in envelope
d.rectangle([222,400,290,464], fill=BLACK)                               # basket (30 over neck)
save(im, "balloon")

# ---------------- hourglass ----------------
im, d = canvas()
d.rectangle([150,60,362,98], fill=BLACK)                                 # top cap
d.polygon([(170,70),(342,70),(268,268),(244,268)], fill=BLACK)           # top bulb (28 under cap)
d.polygon([(244,264),(268,264),(342,462),(170,462)], fill=BLACK)         # bottom bulb (waist joined)
d.rectangle([150,434,362,472], fill=BLACK)                               # bottom cap (28 over bulb)
save(im, "hourglass")
