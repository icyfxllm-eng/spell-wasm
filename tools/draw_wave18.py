#!/usr/bin/env python3
"""Culture batch 10 (Eric, 2026-08-02: "next five").

pagoda    — zh: five flared eaves under the finial
daruma    — ja: the wishing doll, one eye painted, one waiting
balalaika — ru: the triangle with its voice
sombrero  — es: brim, dome, band
djembe    — sw: the goblet drum

Run: python3 tools/draw_wave18.py
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


def pagoda():
    im, d = canvas(940, 1140)
    cx = 470
    # five eaves, wide to narrow, with body segments between — all welded
    widths = [380, 330, 280, 230, 180]
    y = 960
    for w in widths:
        # body segment below the eave
        # the body reaches THROUGH its eave to the tier above (the five
        # stories traced as five separate contours with 26px gaps)
        d.rectangle([cx - int(w * 0.42), y - 192, cx + int(w * 0.42), y], fill=BLACK)
        # the flared eave: a wide low trapezoid with upswept tips
        d.polygon([(cx - w, y - 90), (cx + w, y - 90), (cx + int(w * 0.72), y - 150),
                   (cx - int(w * 0.72), y - 150)], fill=BLACK)
        d.polygon([(cx - w, y - 90), (cx - w - 34, y - 130), (cx - int(w * 0.8), y - 118)], fill=BLACK)
        d.polygon([(cx + w, y - 90), (cx + w + 34, y - 130), (cx + int(w * 0.8), y - 118)], fill=BLACK)
        y -= 176
    # finial, ball welded to its rod
    d.rectangle([cx - 16, y - 60, cx + 16, y - 76 + 90], fill=BLACK)
    circle(d, cx, y - 76, 26, BLACK)
    # base, fused to the lowest story (the oak rule)
    d.rectangle([cx - 160, 950, cx + 160, 1000], fill=BLACK)
    d.rectangle([cx - 420, 990, cx + 420, 1050], fill=BLACK)
    return im


def daruma():
    im, d = canvas(880, 940)
    d.ellipse([160, 100, 720, 760], fill=BLACK)                        # the doll
    d.ellipse([250, 200, 630, 620], fill=WHITE)                        # face patch
    # the eyes: one painted, one waiting to be
    circle(d, 360, 372, 54, BLACK)                                     # painted
    circle(d, 520, 372, 54, BLACK)
    circle(d, 520, 372, 31, WHITE)                                     # the empty one
    # brows and mustache strokes (micro marks inside the face)
    d.ellipse([326, 218, 398, 248], fill=BLACK)
    d.ellipse([484, 218, 556, 248], fill=BLACK)
    d.ellipse([352, 480, 424, 540], fill=BLACK)
    d.ellipse([458, 480, 530, 540], fill=BLACK)
    # the base it rocks on
    d.rectangle([230, 810, 650, 866], fill=BLACK)
    return im


def balalaika():
    im, d = canvas(820, 1040)
    # triangle body
    d.polygon([(410, 300), (110, 860), (710, 860)], fill=BLACK)
    circle(d, 410, 640, 62, WHITE)                                     # soundhole
    # neck and head
    d.rectangle([360, 90, 460, 340], fill=BLACK)
    d.polygon([(350, 100), (470, 100), (450, 22), (370, 22)], fill=BLACK)
    # three peg dots on the head (micro holes)
    for py in (46, 74):
        d.ellipse([394, py - 11, 426, py + 11], fill=WHITE)
    # bridge slit near the base edge
    d.rectangle([350, 790, 470, 816], fill=WHITE)
    return im


def sombrero():
    im, d = canvas(1060, 720)
    # the great brim, tips curled up
    d.ellipse([80, 380, 980, 600], fill=BLACK)
    d.ellipse([150, 300, 330, 460], fill=BLACK)
    d.ellipse([730, 300, 910, 460], fill=BLACK)
    d.ellipse([208, 348, 292, 422], fill=WHITE)                        # curl hollows, walls >= 40px
    d.ellipse([772, 348, 856, 422], fill=WHITE)
    # dome crown with its band seam
    d.ellipse([380, 130, 680, 470], fill=BLACK)
    d.rectangle([368, 372, 692, 414], fill=WHITE)                      # the band
    return im


def djembe():
    im, d = canvas(800, 1000)
    # the bowl
    d.ellipse([160, 120, 640, 480], fill=BLACK)
    d.rectangle([160, 120, 640, 300], fill=BLACK)
    # rim band seam
    d.rectangle([148, 168, 652, 208], fill=WHITE)
    # the head above the rim
    d.rounded_rectangle([170, 90, 630, 172], radius=30, fill=BLACK)
    # waist and flared foot
    d.polygon([(330, 460), (470, 460), (450, 640), (350, 640)], fill=BLACK)
    d.polygon([(350, 630), (450, 630), (560, 900), (240, 900)], fill=BLACK)
    d.ellipse([240, 860, 560, 940], fill=BLACK)
    # rope diamonds around the bowl's skirt (micro holes)
    for k in range(4):
        hx = 260 + k * 94
        d.polygon([(hx, 330), (hx + 26, 375), (hx, 420), (hx - 26, 375)], fill=WHITE)
    return im


SUBJECTS = {
    "pagoda": pagoda,
    "daruma": daruma,
    "balalaika": balalaika,
    "sombrero": sombrero,
    "djembe": djembe,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
