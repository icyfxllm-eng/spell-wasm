#!/usr/bin/env python3
"""Culture batch 12 (Eric, 2026-08-02: "next five").

shinkansen     — ja: the duck-nose bullet train
chichenitza    — worldart: El Castillo, ancient and stepped
montsaintmichel— fr: the island abbey, a basil-style skyline
maasaishield   — sw: pointed oval and crossed spears
janggu         — ko: the hourglass drum on its side

Run: python3 tools/draw_wave20.py
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


def shinkansen():
    im, d = canvas(1180, 660)
    # the duck nose sweeping into the body
    d.polygon([(60, 460), (150, 400), (300, 330), (480, 290), (1120, 290),
               (1120, 460), (60, 460)], fill=BLACK)
    d.rounded_rectangle([460, 250, 1120, 470], radius=40, fill=BLACK)
    # cab window: the swept slit
    d.polygon([(268, 396), (380, 352), (410, 368), (308, 412)], fill=WHITE)   # 39px under the roof line
    # side windows
    for k in range(4):
        d.rounded_rectangle([525 + k * 148, 320, 620 + k * 148, 400], radius=20, fill=WHITE)
    # the rail across an aligned gap
    d.rectangle([40, 520, 1140, 566], fill=BLACK)
    return im


def chichenitza():
    im, d = canvas(1120, 820)
    cx = 560
    # four stepped tiers
    for k, w in enumerate([460, 360, 260, 160]):
        y1 = 700 - k * 110
        d.polygon([(cx - w, y1), (cx + w, y1), (cx + w - 55, y1 - 110), (cx - w + 55, y1 - 110)],
                  fill=BLACK)
    # the temple with its doorway
    d.rectangle([cx - 105, 160, cx + 105, 270], fill=BLACK)
    d.rectangle([cx - 130, 140, cx + 130, 176], fill=BLACK)
    d.rounded_rectangle([cx - 34, 190, cx + 34, 270], radius=16, fill=WHITE)
    # ground
    d.rectangle([70, 740, 1050, 792], fill=BLACK)
    return im


def montsaintmichel():
    im, d = canvas(940, 1040)
    cx = 470
    # the mount: broad island base
    d.polygon([(90, 780), (850, 780), (740, 640), (200, 640)], fill=BLACK)
    # the walls band
    d.rectangle([230, 560, 710, 660], fill=BLACK)
    # village step
    d.polygon([(280, 560), (660, 560), (600, 440), (340, 440)], fill=BLACK)
    # abbey block
    d.rectangle([380, 320, 560, 460], fill=BLACK)
    # the spire with its tip
    d.polygon([(470, 60), (420, 340), (520, 340)], fill=BLACK)
    circle(d, 470, 52, 16, BLACK)
    # abbey windows (micro holes)
    d.ellipse([412, 360, 448, 408], fill=WHITE)
    d.ellipse([492, 360, 528, 408], fill=WHITE)
    # the sea across an aligned gap
    d.rectangle([60, 830, 880, 886], fill=BLACK)
    return im


def maasaishield():
    im, d = canvas(880, 1120)
    # crossed spears behind: shafts with leaf blades and butts
    for sgn in (-1, 1):
        x0, y0 = 440 - sgn * 240, 100
        x1, y1 = 440 + sgn * 240, 1020
        d.line([(x0, y0), (x1, y1)], fill=BLACK, width=34)
        # leaf blade at the top end
        ang = math.atan2(y0 - y1, x0 - x1)
        bx, by = x0 + 50 * math.cos(ang), y0 + 50 * math.sin(ang)
        d.polygon([(x0 - 34, y0 + 10), (bx, by - 60), (x0 + 34, y0 + 10)], fill=BLACK)
        circle(d, x1, y1, 26, BLACK)
    # the shield: pointed oval over the crossing
    d.polygon([(440, 220), (600, 400), (640, 620), (600, 840), (440, 1000),
               (280, 840), (240, 620), (280, 400)], fill=BLACK)
    # pattern: two curved holes
    d.ellipse([330, 420, 420, 640], fill=WHITE)
    d.ellipse([460, 600, 550, 820], fill=WHITE)
    return im


def janggu():
    im, d = canvas(1060, 700)
    # two heads as rings
    for cx in (240, 820):
        circle(d, cx, 350, 190, BLACK)
        circle(d, cx, 350, 120, WHITE)
    # the waist barrel joining them
    # corners driven deep into both rings (5px misses read as separate)
    d.polygon([(340, 250), (720, 250), (600, 320), (600, 380), (720, 450),
               (340, 450), (460, 380), (460, 320)], fill=BLACK)
    return im


SUBJECTS = {
    "shinkansen": shinkansen,
    "chichenitza": chichenitza,
    "montsaintmichel": montsaintmichel,
    "maasaishield": maasaishield,
    "janggu": janggu,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
