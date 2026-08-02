#!/usr/bin/env python3
"""Batch 18 — spiderweb, puzzlepiece, volcano, zodiacwheel. Roster checked."""
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

# ---------------- spiderweb ----------------
im, d = canvas()
cx = cy = 256
d.ellipse([cx-30, cy-30, cx+30, cy+30], fill=BLACK)                # hub
for k in range(6):
    a = math.radians(k*60 + 30)
    tx, ty = cx + 215*math.cos(a), cy + 215*math.sin(a)
    d.line([(cx,cy),(tx,ty)], fill=BLACK, width=34)                # radial strand
    d.ellipse([tx-17,ty-17,tx+17,ty+17], fill=BLACK)               # anchor point
d.arc([cx-155,cy-155,cx+155,cy+155], 0, 360, fill=BLACK, width=30) # one spiral ring, welded
save(im, "spiderweb")

# ---------------- puzzlepiece ----------------
im, d = canvas()
d.rounded_rectangle([120,120,392,392], radius=22, fill=BLACK)      # body
d.ellipse([212,64,300,152], fill=BLACK)                            # top knob (32 in)
d.ellipse([360,212,448,300], fill=BLACK)                           # right knob (32 in)
d.ellipse([212,360,300,448], fill=WHITE)                           # bottom blank (32 carved)
save(im, "puzzlepiece")

# ---------------- volcano ----------------
im, d = canvas()
d.polygon([(86,470),(216,150),(296,150),(426,470)], fill=BLACK)    # cone
d.polygon([(216,150),(296,150),(256,210)], fill=WHITE)             # crater notch
d.rectangle([236,90,276,240], fill=BLACK)                          # plume column (30 into cone)
d.ellipse([166,36,346,132], fill=BLACK)                            # ash cloud (40 over column)
save(im, "volcano")

# ---------------- zodiacwheel ----------------
im, d = canvas()
def ring(r, col):
    d.ellipse([256-r,256-r,256+r,256+r], fill=col)
ring(214, BLACK)                                                    # outer band 178..214
ring(178, WHITE)                                                    # (glyphs live in this gap, guide ink)
ring(108, BLACK)                                                    # inner band 72..108
ring(72, WHITE)
for k in range(8):                                                  # sun rays, welded into inner band
    a = math.radians(k*45)
    d.line([(256,256),(256+100*math.cos(a),256+100*math.sin(a))], fill=BLACK, width=26)
ring(48, BLACK)                                                     # sun disc (moat cells are micro)
save(im, "zodiacwheel")
