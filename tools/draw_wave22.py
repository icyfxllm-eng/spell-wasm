#!/usr/bin/env python3
"""Culture batch 14 (Eric, 2026-08-02: "nutcracker/ dragon boat fail next 5").

riceterraces — vi: four stepped bands over aligned gaps
copacabana   — pt: the Ipanema wave pavement, three ribbons
alhambra     — ar: the horseshoe arch with its star lights
kokeshi      — ja: the little doll
compassrose  — worldart: the navigator's star

Run: python3 tools/draw_wave22.py
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


def riceterraces():
    im, d = canvas(1100, 880)
    # four terrace bands, each a curved shelf, stacked over aligned gaps
    # bands undulate IN PHASE — the phase-shifted first cut brought one
    # band's crest into the next band's trough at 11.7px
    for k, (x0, x1, y, h) in enumerate([(340, 780, 150, 90), (240, 880, 300, 100),
                                        (150, 970, 460, 110), (80, 1040, 640, 120)]):
        pts = []
        for i in range(21):
            t = i / 20
            x = x0 + (x1 - x0) * t
            pts.append((x, y + 24 * math.sin(math.pi * t * 2)))
        bottom = [(x1, y + h), (x0, y + h)]
        d.polygon(pts + bottom, fill=BLACK)
    return im


def copacabana():
    im, d = canvas(1120, 800)
    # three wave ribbons, 44px apart
    for k in range(3):
        y0 = 180 + k * 220
        top = []
        for i in range(41):
            t = i / 40
            x = 60 + 1000 * t
            top.append((x, y0 + 70 * math.sin(t * math.pi * 3)))
        bot = [(x, y + 130) for (x, y) in reversed(top)]
        d.polygon(top + bot, fill=BLACK)
    return im


def alhambra():
    im, d = canvas(920, 1040)
    # the wall
    d.rounded_rectangle([120, 100, 800, 940], radius=40, fill=BLACK)
    # the horseshoe arch: circle overhanging its doorway
    d.ellipse([260, 260, 660, 660], fill=WHITE)
    d.polygon([(300, 460), (620, 460), (590, 880), (330, 880)], fill=WHITE)
    # star lights in the spandrels (micro)
    for (sx, sy) in [(200, 220), (720, 220)]:
        pts = []
        for k in range(8):
            r = 30 if k % 2 == 0 else 13
            a = k * math.pi / 4
            pts.append((sx + r * math.cos(a), sy + r * math.sin(a)))
        d.polygon(pts, fill=WHITE)
    return im


def kokeshi():
    im, d = canvas(760, 1060)
    circle(d, 380, 260, 190, BLACK)                                    # head
    d.ellipse([250, 130, 510, 250], fill=BLACK)                        # hair cap
    d.ellipse([280, 250, 480, 420], fill=WHITE)                        # face patch
    d.ellipse([320, 300, 360, 336], fill=BLACK)                        # eyes (micro)
    d.ellipse([400, 300, 440, 336], fill=BLACK)
    d.ellipse([362, 366, 398, 394], fill=BLACK)                        # the little mouth
    d.rounded_rectangle([260, 440, 500, 960], radius=70, fill=BLACK)   # body
    circle(d, 380, 640, 66, WHITE)                                     # the painted flower
    circle(d, 380, 640, 24, BLACK)
    d.rectangle([230, 960, 530, 1010], fill=BLACK)                     # base
    return im


def compassrose():
    im, d = canvas(980, 980)
    cx, cy = 490, 490
    # four cardinal points, long
    for k in range(4):
        a = k * math.pi / 2
        tip = (cx + 420 * math.cos(a), cy + 420 * math.sin(a))
        l = (cx + 90 * math.cos(a - 0.5), cy + 90 * math.sin(a - 0.5))
        r = (cx + 90 * math.cos(a + 0.5), cy + 90 * math.sin(a + 0.5))
        d.polygon([l, tip, r], fill=BLACK)
    # four ordinal points, short
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        tip = (cx + 260 * math.cos(a), cy + 260 * math.sin(a))
        l = (cx + 80 * math.cos(a - 0.45), cy + 80 * math.sin(a - 0.45))
        r = (cx + 80 * math.cos(a + 0.45), cy + 80 * math.sin(a + 0.45))
        d.polygon([l, tip, r], fill=BLACK)
    # the hub with its ring
    circle(d, cx, cy, 120, BLACK)
    circle(d, cx, cy, 66, WHITE)
    circle(d, cx, cy, 22, BLACK)
    return im


SUBJECTS = {
    "riceterraces": riceterraces,
    "copacabana": copacabana,
    "alhambra": alhambra,
    "kokeshi": kokeshi,
    "compassrose": compassrose,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
