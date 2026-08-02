#!/usr/bin/env python3
"""Culture batch 5 (Eric, 2026-08-01: "next 5 culturtal photos").

arcdetriomphe — fr finally gets its drawn icon (the Eiffel guards the
                core bank); the vault is a hostable arch hole
rickshaw      — hi second icon: the auto-rickshaw, windows as holes
dhow          — sw second icon: the lateen trader on its waterline
galo          — pt: the Barcelos rooster, tail fan and comb
pyramids      — ar: Giza's three, aligned seams between them

Field rules, all learned the hard way: nothing thinner than 34px,
holes host or sit in the micro band, hinges overlap >= 20px, no
slit-shaped holes (the owl's wing seam), nothing sub-floor (the
cuckoo's chains, the owl's sprig).

Run: python3 tools/draw_wave13.py
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


def arcdetriomphe():
    im, d = canvas(920, 860)
    d.rectangle([140, 120, 780, 780], fill=BLACK)                     # the mass
    # the great vault: a hostable arch hole
    d.rectangle([330, 420, 590, 780], fill=WHITE)
    d.ellipse([330, 300, 590, 540], fill=WHITE)
    # attic band seam and cornice steps
    d.rectangle([120, 120, 800, 200], fill=BLACK)
    d.rectangle([100, 80, 820, 130], fill=BLACK)
    # side niches (micro holes)
    d.ellipse([200, 470, 268, 560], fill=WHITE)
    d.ellipse([652, 470, 720, 560], fill=WHITE)
    return im


def rickshaw():
    im, d = canvas(980, 780)
    # cab: rounded body
    d.rounded_rectangle([160, 160, 800, 560], radius=120, fill=BLACK)
    d.polygon([(160, 380), (90, 520), (160, 520)], fill=BLACK)        # front cowl
    # windscreen + door window: hostable holes
    d.rounded_rectangle([220, 230, 430, 420], radius=48, fill=WHITE)
    d.rounded_rectangle([500, 230, 740, 420], radius=48, fill=WHITE)
    # roof rack bar
    d.rectangle([300, 96, 660, 140], fill=BLACK)
    d.rectangle([340, 130, 380, 170], fill=BLACK)
    d.rectangle([580, 130, 620, 170], fill=BLACK)
    # wheels: rings with hub dots
    for cx in (300, 660):
        circle(d, cx, 620, 96, BLACK)
        circle(d, cx, 620, 52, WHITE)
        circle(d, cx, 620, 20, BLACK)
    return im


def dhow():
    im, d = canvas(1000, 820)
    # lateen sail: the great triangle, luffed corner clipped
    d.polygon([(300, 60), (760, 160), (420, 560), (330, 560), (250, 300)], fill=BLACK)
    # the yard: a long spar along the sail's head
    d.line([(220, 120), (820, 200)], fill=BLACK, width=34)
    # mast: welds sail to hull (the sail floated without it)
    d.rectangle([356, 540, 396, 620], fill=BLACK)
    # hull: swept sheer with raked prow
    d.polygon([(120, 600), (840, 600), (900, 540), (760, 700), (220, 700), (140, 640)], fill=BLACK)
    d.polygon([(120, 600), (860, 600), (780, 700), (200, 700)], fill=BLACK)
    # waterline
    d.line([(80, 760), (920, 760)], fill=BLACK, width=36)
    return im


def galo():
    im, d = canvas(860, 980)
    d.ellipse([260, 380, 620, 720], fill=BLACK)                       # body
    d.polygon([(420, 420), (330, 200), (430, 240), (400, 430)], fill=BLACK)  # neck
    circle(d, 380, 190, 82, BLACK)                                    # head
    # comb: three lobes standing on the crown
    for (cx, cy) in [(330, 110), (380, 90), (430, 110)]:
        circle(d, cx, cy, 34, BLACK)
    d.polygon([(300, 150), (460, 150), (430, 200), (330, 200)], fill=BLACK)
    d.polygon([(290, 210), (230, 240), (290, 260)], fill=BLACK)       # beak
    d.polygon([(360, 260), (400, 260), (380, 330)], fill=BLACK)       # wattle
    d.ellipse([352, 168, 392, 200], fill=WHITE)                       # eye (micro)
    # the great tail fan: five plumes sweeping up and back
    for k in range(5):
        a = 1.05 - k * 0.24
        ex = 560 + int(300 * math.cos(a))
        ey = 430 - int(300 * math.sin(a))
        d.line([(560, 470), (ex, ey)], fill=BLACK, width=52)
        circle(d, ex, ey, 26, BLACK)
    # legs to the base
    d.rectangle([380, 700, 424, 850], fill=BLACK)
    d.rectangle([470, 700, 514, 850], fill=BLACK)
    d.rectangle([320, 838, 580, 900], fill=BLACK)                     # plinth
    # heart on the flank: the Barcelos paint, a hole
    hx, hy = 440, 540
    circle(d, hx - 30, hy - 20, 36, WHITE)
    circle(d, hx + 30, hy - 20, 36, WHITE)
    d.polygon([(hx - 62, hy - 6), (hx + 62, hy - 6), (hx, hy + 78)], fill=WHITE)
    return im


def pyramids():
    im, d = canvas(1060, 700)
    # three DISTINCT triangles with sky between the flanks, floating a
    # 38px aligned gap above the ground bar (the wolf-perch answer — the
    # first cut fused everything through the ground and read as one
    # mountain ridge, not Giza)
    d.polygon([(300, 90), (70, 550), (530, 550)], fill=BLACK)
    d.polygon([(690, 200), (480, 550), (900, 550)], fill=BLACK)
    d.polygon([(940, 370), (830, 550), (1050, 550)], fill=BLACK)
    d.rectangle([50, 588, 1010, 646], fill=BLACK)
    return im


SUBJECTS = {
    "arcdetriomphe": arcdetriomphe,
    "rickshaw": rickshaw,
    "dhow": dhow,
    "galo": galo,
    "pyramids": pyramids,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
