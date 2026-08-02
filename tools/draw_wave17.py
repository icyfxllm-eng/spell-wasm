#!/usr/bin/env python3
"""Culture batch 9 (Eric, 2026-08-02: "next 5").

goldengate — worldart: the spec's expert list named it from the start
fan        — es: the flamenco fan, ribs ending short of the pivot
tram       — pt third icon: Lisbon's 28
lotus      — vi third icon: the national flower on its pad
stonehenge — en third icon: two trilithons and a fallen stone

Run: python3 tools/draw_wave17.py
"""
import math
import pathlib

from PIL import Image, ImageDraw

REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"
BLACK, WHITE = 0, 255


def canvas(w, h):
    im = Image.new("L", (w, h), WHITE)
    return im, ImageDraw.Draw(im)


def circle(d, cx, cy, r, fill):
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=fill)


def goldengate():
    im, d = canvas(1180, 820)
    # towers: two, each with twin portal holes
    for tx in (330, 850):
        d.rectangle([tx - 70, 130, tx + 70, 560], fill=BLACK)   # walls 40px each side of the portals
        d.rounded_rectangle([tx - 30, 240, tx + 30, 335], radius=28, fill=WHITE)
        d.rounded_rectangle([tx - 30, 385, tx + 30, 480], radius=28, fill=WHITE)
    # the deck
    d.rectangle([40, 520, 1140, 570], fill=BLACK)
    # ONE continuous cable polyline — separate spans traced as separate
    # contours and pinched each other and the portals
    pts = []
    for (x0, x1, ytop, ysag) in [(40, 330, 104, 540), (330, 850, 108, 400), (850, 1140, 104, 540)]:
        for k in range(25):
            t = k / 24
            x = x0 + (x1 - x0) * t
            pts.append((x, ysag - (ysag - ytop) * (abs(2 * t - 1)) ** 1.6))
    d.line(pts, fill=BLACK, width=44, joint="curve")   # the cable IS two corridor walls
    # the water
    d.rectangle([30, 640, 1150, 692], fill=BLACK)
    return im


def fan():
    im, d = canvas(1020, 800)
    cx, cy = 510, 700
    # the open leaf: a wedge from the pivot, scalloped along the top
    d.pieslice([cx - 460, cy - 460, cx + 460, cy + 460], 205, 335, fill=BLACK)
    for k in range(5):
        a = math.radians(205 + 26 + k * 26)
        px, py = cx + 470 * math.cos(a), cy + 470 * math.sin(a)
        circle(d, px, py, 44, WHITE)
    # rib seams: from NEAR the pivot outward, both ends inset (the Orion
    # lesson — converging ends pinch each other at the hub)
    for k in range(4):
        a = math.radians(205 + 26 + k * 26)
        x0, y0 = cx + 150 * math.cos(a), cy + 150 * math.sin(a)
        x1, y1 = cx + 390 * math.cos(a), cy + 390 * math.sin(a)
        d.line([(x0, y0), (x1, y1)], fill=WHITE, width=34)
    # pivot boss
    circle(d, cx, cy - 20, 46, BLACK)
    return im


def tram():
    im, d = canvas(1000, 900)
    d.rounded_rectangle([160, 260, 840, 640], radius=48, fill=BLACK)
    # windows and the door
    d.rounded_rectangle([200, 310, 360, 460], radius=28, fill=WHITE)
    d.rounded_rectangle([420, 310, 580, 460], radius=28, fill=WHITE)
    d.rounded_rectangle([640, 310, 800, 600], radius=28, fill=WHITE)   # door
    # the pantograph: fat zig to the wire
    d.line([(430, 260), (530, 150)], fill=BLACK, width=30)
    d.line([(530, 150), (470, 80)], fill=BLACK, width=30)
    d.line([(300, 70), (700, 62)], fill=BLACK, width=26)               # the wire
    # wheels + rail
    for cx in (320, 680):
        circle(d, cx, 690, 62, BLACK)
        circle(d, cx, 690, 26, WHITE)
    d.rectangle([80, 790, 920, 836], fill=BLACK)
    return im


def lotus():
    im, d = canvas(1000, 800)
    cx = 500
    # centre petal
    d.polygon([(cx, 90), (cx - 90, 300), (cx - 60, 480), (cx + 60, 480), (cx + 90, 300)],
              fill=BLACK)
    # inner side petals
    for sgn in (-1, 1):
        d.polygon([(cx + sgn * 150, 160), (cx + sgn * 40, 330), (cx + sgn * 80, 490),
                   (cx + sgn * 210, 430), (cx + sgn * 230, 280)], fill=BLACK)
    # outer petals sweeping low
    for sgn in (-1, 1):
        d.polygon([(cx + sgn * 330, 280), (cx + sgn * 160, 420), (cx + sgn * 170, 520),
                   (cx + sgn * 360, 470), (cx + sgn * 400, 360)], fill=BLACK)
    # the receptacle bowl welding all five
    d.ellipse([cx - 240, 440, cx + 240, 580], fill=BLACK)
    # the pad: a wide leaf bar across an aligned gap
    d.ellipse([140, 640, 860, 760], fill=BLACK)
    return im


def stonehenge():
    im, d = canvas(1120, 800)
    # two trilithons
    for x0 in (150, 630):
        d.rectangle([x0, 240, x0 + 110, 620], fill=BLACK)
        d.rectangle([x0 + 230, 240, x0 + 340, 620], fill=BLACK)
        d.rectangle([x0 - 20, 150, x0 + 360, 250], fill=BLACK)          # lintel
    # the fallen stone between them
    d.polygon([(534, 566), (590, 556), (598, 606), (542, 618)], fill=BLACK)  # centred, 40px off both uprights
    # the earth
    d.rectangle([80, 660, 1040, 712], fill=BLACK)
    return im


SUBJECTS = {
    "goldengate": goldengate,
    "fan": fan,
    "tram": tram,
    "lotus": lotus,
    "stonehenge": stonehenge,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
