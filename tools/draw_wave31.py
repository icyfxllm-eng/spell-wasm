#!/usr/bin/env python3
"""Batch 23 — palmtree, birdnest, tornado (EXPERT), crystalball, clock."""
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

# palmtree: curved trunk, seven bold fronds, coconuts, island mound
im, d = canvas()
d.ellipse([60, 428, 452, 502], fill=BLACK)                        # island mound
d.polygon([(226, 470), (238, 350), (262, 240), (296, 150), (352, 158),
           (322, 250), (302, 352), (296, 470)], fill=BLACK)       # curved trunk
cx, cy = 322, 168                                                  # crown point
for ang, ln in [(200, 150), (235, 160), (270, 150), (305, 160), (340, 150), (165, 140), (130, 130)]:
    a = math.radians(ang)
    tx, ty = cx + ln*math.cos(a), cy + ln*math.sin(a)
    d.line([(cx, cy), (tx, ty)], fill=BLACK, width=34)             # fronds
    d.ellipse([tx-17, ty-17, tx+17, ty+17], fill=BLACK)
d.ellipse([cx-40, cy-40, cx+40, cy+40], fill=BLACK)                # crown boss
d.ellipse([272, 196, 316, 240], fill=BLACK)                        # coconut (welded)
save(im, "palmtree")

# birdnest: woven bowl, rim twigs, two eggs
im, d = canvas()
d.ellipse([76, 210, 436, 470], fill=BLACK)                        # bowl
d.rectangle([40, 150, 472, 250], fill=WHITE)                      # cut the top open
d.rectangle([96, 236, 416, 280], fill=BLACK)                      # rim band
for x0, y0, x1, y1 in [(60, 200, 130, 250), (382, 200, 452, 250),
                       (120, 180, 180, 252), (332, 180, 392, 252)]:
    d.line([(x0, y0), (x1, y1)], fill=BLACK, width=24)            # rim twigs, welded into the band
d.ellipse([140, 262, 234, 380], fill=WHITE)                       # egg 1 (hostable ring)
d.ellipse([278, 262, 372, 380], fill=WHITE)                       # egg 2 (44px between)
save(im, "birdnest")

# tornado (EXPERT): five stacked funnel rings narrowing down + dust cloud
im, d = canvas()
def band(cx, cy, rx, ry, w):
    d.ellipse([cx-rx, cy-ry, cx+rx, cy+ry], fill=BLACK)
    d.ellipse([cx-rx+w, cy-ry+w//2+6, cx+rx-w, cy+ry-w//2-6], fill=WHITE)
band(256, 84, 214, 60, 40)                                        # ring 1 (widest)
band(242, 226, 158, 52, 40)                                       # ring 2 (30px below ring 1)
band(260, 352, 102, 44, 40)                                       # ring 3
d.polygon([(236, 380), (280, 380), (272, 476), (242, 476)], fill=BLACK)  # funnel tip, welded in ring 3
d.ellipse([150, 446, 370, 502], fill=BLACK)                       # dust cloud (tip welds in)
save(im, "tornado")

# crystalball: thick sphere ring on a stepped base, micro sparkles in the base
im, d = canvas()
d.ellipse([76, 40, 436, 400], fill=BLACK)                         # sphere
d.ellipse([120, 84, 392, 356], fill=WHITE)                        # inner void (big hostable ring)
d.polygon([(150, 360), (362, 360), (402, 440), (110, 440)], fill=BLACK)  # base (welds over sphere bottom)
d.rectangle([90, 432, 422, 478], fill=BLACK)                      # base step
d.ellipse([240, 392, 272, 420], fill=WHITE)                       # base gem (micro)
save(im, "crystalball")

# clock: ring band, twelve welded hour lugs, bold hands from hub
im, d = canvas()
d.ellipse([46, 46, 466, 466], fill=BLACK)                         # outer
d.ellipse([98, 98, 414, 414], fill=WHITE)                         # face (band 52 thick)
for k in range(12):
    a = math.radians(k * 30)
    bx, by = 256 + 158*math.cos(a), 256 + 158*math.sin(a)
    d.ellipse([bx-15, by-15, bx+15, by+15], fill=BLACK)           # hour lugs welded to band
d.ellipse([226, 226, 286, 286], fill=BLACK)                       # hub
d.line([(256, 256), (256, 138)], fill=BLACK, width=30)            # minute hand (12)
d.line([(256, 256), (336, 300)], fill=BLACK, width=30)            # hour hand (~4), well off the band
save(im, "clock")
