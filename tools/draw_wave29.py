#!/usr/bin/env python3
"""Batch 21 — castle, ferris wheel, whale, robot, ice cream. Weld-first."""
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

# castle: wall + battlements, two towers, center keep, open gate arch
im, d = canvas()
d.rectangle([76, 300, 436, 470], fill=BLACK)                    # wall
for x in range(96, 420, 64):
    d.rectangle([x, 262, x + 40, 310], fill=BLACK)              # crenellations
d.rectangle([76, 180, 160, 470], fill=BLACK)                    # left tower
d.polygon([(60, 190), (118, 96), (176, 190)], fill=BLACK)       # left roof
d.rectangle([352, 180, 436, 470], fill=BLACK)                   # right tower
d.polygon([(336, 190), (394, 96), (452, 190)], fill=BLACK)      # right roof
d.rectangle([216, 200, 296, 470], fill=BLACK)                   # keep
d.polygon([(200, 210), (256, 120), (312, 210)], fill=BLACK)     # keep roof
d.rectangle([252, 42, 262, 130], fill=BLACK)                    # flag pole (welded in roof)
d.polygon([(262, 42), (322, 62), (262, 84)], fill=BLACK)        # flag
d.polygon([(226, 470), (226, 396), (256, 368), (286, 396), (286, 470)], fill=WHITE)  # gate (open at base)
d.ellipse([104, 224, 132, 252], fill=WHITE)                     # tower window (micro)
d.ellipse([380, 224, 408, 252], fill=WHITE)                     # tower window (micro)
save(im, "castle")

# ferris wheel: ring, 8 spokes, hub, cabins as welded bumps, A-legs, base
im, d = canvas()
cx, cy = 256, 226
d.arc([cx-190, cy-190, cx+190, cy+190], 0, 360, fill=BLACK, width=34)  # ring
for k in range(8):
    a = math.radians(k * 45 + 22.5)
    d.line([(cx, cy), (cx + 176*math.cos(a), cy + 176*math.sin(a))], fill=BLACK, width=26)  # spokes
d.ellipse([cx-32, cy-32, cx+32, cy+32], fill=BLACK)             # hub
for k in range(8):
    a = math.radians(k * 45 + 22.5)
    bx, by = cx + 190*math.cos(a), cy + 190*math.sin(a)
    d.ellipse([bx-24, by-24, bx+24, by+24], fill=BLACK)         # cabins on the ring
d.line([(cx, cy), (cx-110, 476)], fill=BLACK, width=32)         # left leg
d.line([(cx, cy), (cx+110, 476)], fill=BLACK, width=32)         # right leg
d.rectangle([96, 460, 416, 490], fill=BLACK)                    # ground bar
save(im, "ferriswheel")

# whale: body, tail flukes, fin, welded spout, micro eye
im, d = canvas()
d.ellipse([60, 210, 400, 420], fill=BLACK)                      # body
d.polygon([(360, 300), (470, 230), (450, 315), (470, 400)], fill=BLACK)  # tail flukes
d.polygon([(200, 380), (260, 470), (300, 392)], fill=BLACK)     # fin
d.rectangle([120, 130, 152, 250], fill=BLACK)                   # spout column (deep in head)
d.ellipse([76, 78, 200, 150], fill=BLACK)                       # spout cloud
d.ellipse([120, 280, 150, 310], fill=WHITE)                     # eye (micro)
save(im, "whale")

# robot: boxy welded, micro eyes + mouth
im, d = canvas()
d.rectangle([176, 80, 336, 200], fill=BLACK)                    # head
d.rectangle([246, 30, 266, 96], fill=BLACK)                     # antenna stem
d.ellipse([236, 8, 276, 48], fill=BLACK)                        # antenna ball
d.rectangle([146, 216, 366, 400], fill=BLACK)                   # torso
d.rectangle([216, 190, 296, 226], fill=BLACK)                   # neck (welds head+torso)
d.rectangle([86, 226, 156, 268], fill=BLACK)                    # left arm out
d.rectangle([86, 226, 128, 370], fill=BLACK)                    # left arm down
d.rectangle([356, 226, 426, 268], fill=BLACK)                   # right arm out
d.rectangle([384, 226, 426, 370], fill=BLACK)                   # right arm down
d.rectangle([176, 390, 226, 480], fill=BLACK)                   # left leg
d.rectangle([286, 390, 336, 480], fill=BLACK)                   # right leg
d.ellipse([210, 114, 238, 142], fill=WHITE)                     # left eye (micro)
d.ellipse([274, 114, 302, 142], fill=WHITE)                     # right eye (micro)
d.rectangle([234, 164, 278, 182], fill=WHITE)                   # mouth slot (micro)
save(im, "robot")

# ice cream: cone + two scoops + cherry, one welded stack
im, d = canvas()
d.polygon([(166, 250), (346, 250), (256, 484)], fill=BLACK)     # cone
d.ellipse([146, 160, 366, 300], fill=BLACK)                     # lower scoop
d.ellipse([176, 80, 336, 216], fill=BLACK)                      # upper scoop
d.ellipse([232, 34, 280, 82], fill=BLACK)                       # cherry (welded)
save(im, "icecream")
