#!/usr/bin/env python3
"""Culture batch 7 (Eric, 2026-08-01: "next five").

jeepney    — fil: the spec's OTHER Tagalog name, staged like the
             carabao so Paul's one sign-off opens the whole pack
manekineko — ja third icon: the beckoning cat
acacia     — sw: the flat-top tree under Kilimanjaro's snow
samovar    — ru third icon: the tea urn
lantern    — zh third icon: the red lantern with its tassel

Run: python3 tools/draw_wave15.py
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


def jeepney():
    im, d = canvas(1120, 720)
    # long body with the flat hood
    d.rounded_rectangle([80, 220, 1040, 540], radius=60, fill=BLACK)
    d.rounded_rectangle([40, 300, 140, 540], radius=40, fill=BLACK)   # hood nose
    # windshield + three passenger windows (hostable holes)
    d.rounded_rectangle([170, 270, 330, 420], radius=36, fill=WHITE)
    for k in range(3):
        d.rounded_rectangle([400 + k * 200, 270, 560 + k * 200, 420], radius=36, fill=WHITE)
    # the roof rack
    d.rectangle([260, 150, 900, 200], fill=BLACK)
    d.rectangle([320, 190, 360, 230], fill=BLACK)
    d.rectangle([800, 190, 840, 230], fill=BLACK)
    # grille bars on the nose: two micro slits
    d.rectangle([60, 340, 116, 368], fill=WHITE)
    d.rectangle([60, 400, 116, 428], fill=WHITE)
    # wheels
    for cx in (300, 860):
        circle(d, cx, 590, 90, BLACK)
        circle(d, cx, 590, 48, WHITE)
        circle(d, cx, 590, 18, BLACK)
    return im


def manekineko():
    im, d = canvas(880, 1000)
    circle(d, 430, 330, 210, BLACK)                                    # head
    d.polygon([(300, 160), (250, 40), (380, 130)], fill=BLACK)         # ear
    d.polygon([(560, 160), (610, 40), (480, 130)], fill=BLACK)         # ear
    d.ellipse([230, 480, 630, 900], fill=BLACK)                        # body
    # the beckoning paw, raised beside the head
    d.rounded_rectangle([640, 260, 740, 560], radius=50, fill=BLACK)
    circle(d, 690, 250, 62, BLACK)
    # face: eyes and nose as micro holes
    d.ellipse([340, 280, 400, 330], fill=WHITE)
    d.ellipse([460, 280, 520, 330], fill=WHITE)
    d.ellipse([405, 360, 455, 400], fill=WHITE)
    # bell only — the collar band hole rode the head-body seam and found
    # a new pinch every time it moved; the bell carries the collar read
    circle(d, 430, 560, 40, WHITE)
    d.ellipse([320, 680, 540, 850], fill=WHITE)                        # koban bib
    # tucked paw at the base
    d.ellipse([250, 830, 400, 920], fill=BLACK)
    return im


def acacia():
    im, d = canvas(1080, 880)
    # Kilimanjaro: broad shield cone behind, snowcap across the wolf-gap
    d.polygon([(540, 300), (300, 420), (60, 560), (1020, 560), (780, 420)], fill=BLACK)
    zig = [(400, 372), (450, 398), (500, 370), (550, 400), (600, 368), (650, 396), (680, 372)]
    d.polygon(zig + [(680, 330), (400, 330)], fill=WHITE)
    cap = [(x, y - 52) for (x, y) in zig]
    d.polygon(cap + [(640, 258), (540, 240), (440, 258)], fill=BLACK)
    # the acacia: flat-top canopy on forking trunk, in front on the plain
    d.ellipse([150, 604, 610, 720], fill=BLACK)   # canopy slab, 44px under the mountain's base edge
    d.polygon([(350, 690), (330, 780), (310, 840), (360, 840), (372, 760), (390, 690)],
              fill=BLACK)                                              # trunk
    d.polygon([(360, 700), (450, 760), (470, 700)], fill=BLACK)        # fork
    d.polygon([(455, 755), (470, 840), (510, 840), (480, 745)], fill=BLACK)
    # the plain
    d.rectangle([60, 828, 1020, 878], fill=BLACK)
    return im


def samovar():
    im, d = canvas(820, 1020)
    d.polygon([(280, 240), (540, 240), (580, 380), (580, 620), (520, 740), (300, 740),
               (240, 620), (240, 380)], fill=BLACK)                    # the urn
    # crown chimney and its little teapot
    d.rectangle([360, 150, 460, 250], fill=BLACK)
    d.ellipse([340, 90, 480, 170], fill=BLACK)
    d.ellipse([375, 108, 445, 152], fill=WHITE)                        # teapot ring
    # side handles: ring holes
    for sgn in (-1, 1):
        cx = 410 + sgn * 240
        d.ellipse([cx - 70, 360, cx + 70, 540], fill=BLACK)
        d.ellipse([cx - 34, 400, cx + 34, 500], fill=WHITE)
    # the spigot with its key
    d.line([(410, 640), (410, 700)], fill=BLACK, width=0)
    d.polygon([(380, 740), (440, 740), (430, 810), (390, 810)], fill=BLACK)
    circle(d, 410, 830, 30, BLACK)
    # base and feet
    d.rectangle([300, 850, 520, 900], fill=BLACK)
    d.rectangle([260, 892, 560, 940], fill=BLACK)
    return im


def lantern():
    im, d = canvas(800, 1000)
    # hanging cord and top cap
    d.rectangle([380, 40, 420, 140], fill=BLACK)
    d.rounded_rectangle([280, 130, 520, 200], radius=24, fill=BLACK)
    # the body with three rib seams
    d.ellipse([160, 190, 640, 700], fill=BLACK)
    d.line([(400, 205), (400, 690)], fill=WHITE, width=38)
    # outer ribs shortened and pulled inward: at the ellipse edge their
    # outer slivers tapered to flagged pinches
    d.line([(302, 290), (302, 600)], fill=WHITE, width=34)
    d.line([(498, 290), (498, 600)], fill=WHITE, width=34)
    # bottom cap and the tassel
    d.rounded_rectangle([300, 690, 500, 750], radius=24, fill=BLACK)
    d.rectangle([378, 750, 422, 810], fill=BLACK)
    circle(d, 400, 830, 34, BLACK)
    d.polygon([(366, 852), (434, 852), (420, 960), (380, 960)], fill=BLACK)
    return im


SUBJECTS = {
    "jeepney": jeepney,
    "manekineko": manekineko,
    "acacia": acacia,
    "samovar": samovar,
    "lantern": lantern,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
