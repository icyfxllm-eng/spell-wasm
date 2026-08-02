#!/usr/bin/env python3
"""Culture batch 8 (Eric, 2026-08-02: "next 5").

sphinx      — worldart/ar: the couchant guardian in profile
moai        — es: the Rapa Nui profile, unmistakable
sitar       — hi third icon: gourd, wide neck, curled pegbox
brandenburg — de third icon: the column comb under its attic
colosseum   — worldart: the spec's own expert-list name, arch rows

Run: python3 tools/draw_wave16.py
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


def sphinx():
    im, d = canvas(1140, 720)
    # lion body couchant, profile facing left
    d.polygon([(300, 420), (900, 380), (1010, 420), (1030, 560), (280, 560)], fill=BLACK)
    # haunch swell
    d.ellipse([820, 380, 1050, 560], fill=BLACK)
    # forepaws stretched forward
    d.rounded_rectangle([120, 490, 560, 560], radius=34, fill=BLACK)
    d.rounded_rectangle([150, 520, 590, 580], radius=30, fill=BLACK)
    # chest rising to the head
    d.polygon([(300, 430), (330, 300), (420, 300), (430, 430)], fill=BLACK)
    # the nemes headdress: flared triangle with the face notched in front
    d.polygon([(250, 300), (370, 120), (470, 120), (500, 300), (430, 330), (310, 330)],
              fill=BLACK)
    # face profile carved from the front edge
    d.polygon([(250, 300), (310, 210), (330, 260), (300, 300)], fill=WHITE)
    # eye (micro)
    d.ellipse([330, 200, 366, 228], fill=WHITE)
    # base slab across an aligned gap
    d.rectangle([80, 620, 1080, 676], fill=BLACK)
    return im


def moai():
    im, d = canvas(780, 1020)
    # the head in profile facing left: forehead, brow, the long nose,
    # lips, jaw — the silhouette IS the statue
    d.polygon([
        (300, 100), (480, 80), (600, 130), (640, 300), (640, 760),
        (560, 860), (300, 880), (250, 800), (270, 700),
        (250, 660), (270, 620),                                        # chin steps
        (230, 560), (300, 540),                                        # lips ledge
        (210, 500), (300, 430),                                        # the nose
        (250, 400), (300, 360), (270, 300),                            # brow ridge
    ], fill=BLACK)
    # the long ear: a hostable slot hole
    d.rounded_rectangle([480, 380, 560, 660], radius=36, fill=WHITE)
    # eye shadow under the brow (micro)
    d.ellipse([320, 372, 380, 412], fill=WHITE)
    # the ahu platform
    d.rectangle([120, 930, 700, 986], fill=BLACK)
    return im


def sitar():
    im, d = canvas(800, 1100)
    # gourd
    d.ellipse([220, 620, 620, 1020], fill=BLACK)
    circle(d, 420, 800, 66, WHITE)                                     # rosette
    d.rectangle([360, 950, 480, 976], fill=WHITE)                      # bridge slit
    # wide neck
    d.rectangle([370, 160, 470, 700], fill=BLACK)
    # curled pegbox
    d.polygon([(370, 180), (470, 180), (470, 90), (390, 40), (310, 60), (300, 130), (370, 140)],
              fill=BLACK)
    circle(d, 350, 95, 26, WHITE)                                      # the curl's eye
    # the small resonating gourd behind the neck's top
    d.ellipse([180, 200, 340, 360], fill=BLACK)
    d.rectangle([320, 250, 380, 300], fill=BLACK)                      # its weld
    return im


def brandenburg():
    im, d = canvas(1120, 820)
    # steps, entablature, attic — the columns comb between them
    d.rectangle([60, 180, 1060, 280], fill=BLACK)                      # entablature
    d.rectangle([300, 80, 820, 190], fill=BLACK)                       # attic block
    for k in range(6):
        x = 120 + k * 160
        d.polygon([(x, 280), (x + 80, 280), (x + 92, 640), (x - 12, 640)], fill=BLACK)
    d.rectangle([50, 640, 1070, 700], fill=BLACK)                      # stylobate
    d.rectangle([90, 700, 1030, 750], fill=BLACK)                      # steps
    return im


def colosseum():
    im, d = canvas(1140, 780)
    # the broken ring in elevation: tall section left, low section right
    d.polygon([(90, 620), (90, 160), (200, 110), (430, 90), (620, 110), (700, 150),
               (700, 320), (1050, 320), (1050, 620)], fill=BLACK)
    # upper arch row (tall section)
    for k in range(4):
        x = 160 + k * 140
        d.rounded_rectangle([x, 170, x + 76, 290], radius=38, fill=WHITE)
    # middle arch row
    for k in range(4):
        x = 160 + k * 140
        d.rounded_rectangle([x, 340, x + 76, 460], radius=38, fill=WHITE)
    # lower row runs the full ellipse — re-spaced: at 490-592 it sat
    # 30px under the middle row and 20px over the wall's foot
    for k in range(6):
        x = 160 + k * 150
        d.rounded_rectangle([x - 8, 498, x + 88, 580], radius=40, fill=WHITE)  # wide enough to stay hostable
    # ground
    d.rectangle([60, 660, 1080, 716], fill=BLACK)
    return im


SUBJECTS = {
    "sphinx": sphinx,
    "moai": moai,
    "sitar": sitar,
    "brandenburg": brandenburg,
    "colosseum": colosseum,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
