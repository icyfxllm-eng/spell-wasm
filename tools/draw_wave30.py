#!/usr/bin/env python3
"""Batch 22 — crown, umbrella, key, campfire, submarine. Weld-first."""
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

# crown: band, five points with ball tips, micro jewels
im, d = canvas()
d.rectangle([76, 330, 436, 420], fill=BLACK)                      # band
for i, x in enumerate([130, 256, 382]):
    top = 96 if i == 1 else 156
    d.polygon([(x-46, 366), (x, top-6), (x+46, 366)], fill=BLACK)  # three points, apex buried in ball
    d.ellipse([x-26, top-26, x+26, top+26], fill=BLACK)            # ball tips
d.rectangle([64, 400, 448, 446], fill=BLACK)                       # base rim
for x in [130, 256, 382]:
    d.ellipse([x-14, 356, x+14, 384], fill=WHITE)                  # jewels (micro)
save(im, "crown")

# umbrella: canopy with scalloped hem, pole, open C handle
im, d = canvas()
d.pieslice([56, 60, 456, 460], 180, 360, fill=BLACK)              # canopy dome
for x in [106, 206, 306, 406]:
    d.ellipse([x-50, 232, x+50, 292], fill=WHITE)                 # scalloped hem
d.rectangle([56, 210, 456, 262], fill=BLACK)                      # hem bar keeps the dome whole
d.ellipse([236, 34, 276, 74], fill=BLACK)                          # ferrule knob
d.rectangle([238, 60, 274, 436], fill=BLACK)                       # pole
d.arc([166, 360, 274, 470], 0, 200, fill=BLACK, width=34)          # handle hook, end welded in the pole
save(im, "umbrella")

# key: bow ring (big hostable hole), shank, two teeth
im, d = canvas()
d.ellipse([60, 156, 260, 356], fill=BLACK)                        # bow
d.ellipse([110, 206, 210, 306], fill=WHITE)                       # bow hole
d.rectangle([236, 230, 452, 282], fill=BLACK)                     # shank (26 into bow)
d.rectangle([392, 282, 428, 344], fill=BLACK)                     # tooth 1
d.rectangle([310, 282, 346, 330], fill=BLACK)                     # tooth 2
save(im, "key")

# campfire: crossed logs, flame welded through them
im, d = canvas()
d.polygon([(80, 400), (110, 368), (430, 452), (402, 486)], fill=BLACK)   # log A
d.polygon([(402, 368), (432, 400), (110, 486), (82, 452)], fill=BLACK)   # log B
d.polygon([(256, 60), (330, 176), (352, 280), (322, 372), (256, 420),
           (190, 372), (160, 280), (182, 176)], fill=BLACK)              # outer flame, into the logs
d.polygon([(256, 190), (296, 268), (300, 330), (256, 372), (212, 330),
           (216, 268)], fill=WHITE)                                      # inner tongue (hostable ring)
save(im, "campfire")

# submarine: hull, tower, periscope, tail fin, micro portholes
im, d = canvas()
d.ellipse([48, 208, 424, 384], fill=BLACK)                        # hull
d.rectangle([196, 130, 316, 250], fill=BLACK)                     # conning tower
d.rectangle([228, 60, 256, 150], fill=BLACK)                      # periscope stem
d.rectangle([228, 60, 316, 92], fill=BLACK)                       # periscope arm
d.polygon([(400, 250), (490, 180), (472, 296), (490, 400), (398, 342)], fill=BLACK)  # tail fin
for x in [130, 206, 282]:
    d.ellipse([x-16, 280, x+16, 312], fill=WHITE)                 # portholes (micro)
save(im, "submarine")
