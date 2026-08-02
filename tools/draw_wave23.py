#!/usr/bin/env python3
"""Culture batch 15 (Eric, 2026-08-02: "next 5").

gramophone — worldart: the horn that played the century in
scarab     — ar: the beetle, wings spread, ancient motif
bull       — es: charging (an original — the famous roadside bull
             silhouette is a trademark, so this is our own beast)
kitsune    — ja: the fox mask
zebra      — sw: stripe seams at the kite-spar width

Run: python3 tools/draw_wave23.py
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


def gramophone():
    im, d = canvas(1000, 1000)
    # the horn: a great flare with its bell opened as a ring
    d.polygon([(560, 480), (740, 160), (860, 90), (930, 140), (900, 300), (700, 520)],
              fill=BLACK)
    d.ellipse([760, 120, 900, 260], fill=BLACK)
    d.ellipse([796, 156, 864, 224], fill=WHITE)                        # the bell mouth
    # elbow down to the box
    d.line([(600, 500), (520, 620)], fill=BLACK, width=54)
    # the box with its crank and needle arm
    d.rounded_rectangle([220, 620, 760, 860], radius=36, fill=BLACK)
    d.line([(760, 700), (860, 660)], fill=BLACK, width=34)
    circle(d, 872, 652, 34, BLACK)                                     # crank knob
    d.ellipse([300, 700, 380, 780], fill=WHITE)                        # turntable peek
    # feet
    d.rectangle([260, 860, 340, 920], fill=BLACK)
    d.rectangle([640, 860, 720, 920], fill=BLACK)
    return im


def scarab():
    """Take two: everything rooted DEEP — the first cut was an assembly
    of parts floating 2-9px from each other."""
    im, d = canvas(1000, 900)
    cx = 500
    d.ellipse([cx - 150, 320, cx + 150, 720], fill=BLACK)              # body
    d.ellipse([cx - 95, 200, cx + 95, 380], fill=BLACK)                # head, welded
    # the sun disc on a fat stem
    d.rectangle([cx - 30, 120, cx + 30, 230], fill=BLACK)
    circle(d, cx, 110, 64, BLACK)
    # wings: solid fans rooted 40px into the body
    for sgn in (-1, 1):
        d.polygon([(cx + sgn * 100, 400), (cx + sgn * 420, 250), (cx + sgn * 450, 350),
                   (cx + sgn * 370, 480), (cx + sgn * 100, 540)], fill=BLACK)
    # six legs rooted 30px inside the body edge
    for k, dy in enumerate([420, 540, 640]):
        for sgn in (-1, 1):
            x0 = cx + sgn * (120 if k < 2 else 95)   # the body narrows low
            x1 = cx + sgn * (300 + 45 * k)
            y1 = dy + 140 + 40 * k
            d.line([(x0, dy), (x1, y1)], fill=BLACK, width=38)
    # wing-case seam, fat and inset
    d.rectangle([cx - 110, 430, cx + 110, 468], fill=WHITE)
    return im


def bull():
    im, d = canvas(1180, 900)
    # the great body: hump, back, rump
    d.ellipse([260, 300, 960, 700], fill=BLACK)
    d.ellipse([220, 260, 560, 560], fill=BLACK)                        # shoulder hump
    # head lowered to charge
    d.polygon([(240, 380), (120, 430), (80, 540), (170, 590), (280, 560)], fill=BLACK)
    # the horns: crescents sweeping up
    # horns rooted 30px into the skull (they hovered at 1.8px)
    d.line([(165, 450), (90, 300), (150, 220)], fill=BLACK, width=42, joint="curve")
    d.line([(230, 440), (300, 290), (260, 200)], fill=BLACK, width=42, joint="curve")
    d.ellipse([160, 452, 202, 486], fill=WHITE)                        # eye (micro)
    # legs mid-charge
    d.polygon([(340, 640), (420, 640), (380, 850), (300, 850)], fill=BLACK)
    d.polygon([(500, 680), (580, 690), (560, 860), (480, 860)], fill=BLACK)
    d.polygon([(720, 690), (800, 680), (840, 860), (760, 860)], fill=BLACK)
    d.polygon([(855, 600), (945, 585), (1030, 800), (950, 830)], fill=BLACK)
    # tail flying
    d.line([(905, 430), (1080, 360), (1120, 260)], fill=BLACK, width=34, joint="curve")
    circle(d, 1124, 248, 26, BLACK)
    return im


def kitsune():
    im, d = canvas(880, 1000)
    cx = 440
    # the mask: fox face, pointed chin
    d.polygon([(cx - 260, 330), (cx - 180, 220), (cx, 180), (cx + 180, 220), (cx + 260, 330),
               (cx + 200, 620), (cx, 840), (cx - 200, 620)], fill=BLACK)
    # tall ears, welded at the crown
    d.polygon([(cx - 230, 300), (cx - 300, 60), (cx - 110, 210)], fill=BLACK)
    d.polygon([(cx + 230, 300), (cx + 300, 60), (cx + 110, 210)], fill=BLACK)
    # ear hollows
    # hollows inset 30px from every ear edge (they grazed at 2.3px)
    d.polygon([(cx - 206, 256), (cx - 232, 182), (cx - 178, 228)], fill=WHITE)
    d.polygon([(cx + 206, 256), (cx + 232, 182), (cx + 178, 228)], fill=WHITE)
    # the eye slits, angled (micro)
    d.polygon([(cx - 160, 380), (cx - 60, 420), (cx - 150, 440)], fill=WHITE)
    d.polygon([(cx + 160, 380), (cx + 60, 420), (cx + 150, 440)], fill=WHITE)
    # snout patch with its nose
    d.ellipse([cx - 70, 560, cx + 70, 720], fill=WHITE)
    d.polygon([(cx - 30, 640), (cx + 30, 640), (cx, 690)], fill=BLACK)
    return im


def zebra():
    """Take two: one welded beast, carabao-style — the first cut's neck,
    mane, stripes and tail all floated."""
    im, d = canvas(1160, 920)
    # body, neck and head as one polygon
    d.polygon([(300, 320), (200, 150), (110, 190), (100, 300), (200, 300), (260, 380),
               (280, 600), (920, 600), (940, 360), (860, 300), (420, 300)], fill=BLACK)
    d.ellipse([280, 280, 940, 640], fill=BLACK)
    d.polygon([(196, 168), (170, 90), (236, 148)], fill=BLACK)                # ear, rooted
    d.ellipse([140, 218, 180, 250], fill=WHITE)                               # eye (micro)
    # mane teeth rooted on the neckline
    for k in range(4):
        x = 300 + k * 52
        y = 210 + k * 32
        d.polygon([(x, y + 40), (x + 40, y + 30), (x + 24, y - 44)], fill=BLACK)
    # stripe seams: ends 45px inside the body's edges
    for k in range(4):
        x = 460 + k * 120
        d.line([(x, 345), (x - 26, 556)], fill=WHITE, width=38)
    # legs overlapping the belly line
    for x in (360, 500, 720, 860):
        d.rectangle([x, 560, x + 58, 850], fill=BLACK)
    # tail rooted in the rump
    d.line([(900, 380), (1030, 540)], fill=BLACK, width=32)
    d.polygon([(1016, 526), (1080, 640), (996, 606)], fill=BLACK)
    return im


SUBJECTS = {
    "gramophone": gramophone,
    "scarab": scarab,
    "bull": bull,
    "kitsune": kitsune,
    "zebra": zebra,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
