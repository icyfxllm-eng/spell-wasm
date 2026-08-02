#!/usr/bin/env python3
"""Culture batch 11 (Eric, 2026-08-02: "all pass next five").

liberty    — worldart: Bartholdi died 1904; the lady is long PD
angkorwat  — worldart: the five lotus-bud towers
accordion  — fr: bellows pleats, the fan lesson applied
quetzal    — es: the resplendent bird and its impossible tail
grandpiano — pl: for Chopin

Run: python3 tools/draw_wave19.py
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


def liberty():
    im, d = canvas(900, 1160)
    cx = 430
    # the robe: A-line mass
    d.polygon([(cx - 40, 320), (cx + 60, 320), (cx + 150, 900), (cx - 150, 900)], fill=BLACK)
    # head and crown spikes
    circle(d, cx, 270, 70, BLACK)
    for k in range(7):
        a = math.pi * (0.18 + 0.64 * k / 6)
        x0, y0 = cx + 62 * math.cos(a), 265 - 62 * math.sin(a)
        x1, y1 = cx + 130 * math.cos(a), 265 - 130 * math.sin(a)
        d.line([(x0, y0), (x1, y1)], fill=BLACK, width=26)
    # the torch arm, raised
    d.line([(cx + 40, 340), (cx + 190, 120)], fill=BLACK, width=52)
    d.rectangle([cx + 160, 80, cx + 220, 130], fill=BLACK)
    d.polygon([(cx + 165, 84), (cx + 215, 84), (cx + 190, 20)], fill=BLACK)   # flame
    # the tablet arm
    d.line([(cx - 30, 400), (cx - 170, 520)], fill=BLACK, width=48)
    d.rectangle([cx - 260, 440, cx - 150, 600], fill=BLACK)
    # pedestal across an aligned gap
    d.rectangle([cx - 210, 940, cx + 210, 1020], fill=BLACK)
    d.rectangle([cx - 280, 1012, cx + 280, 1090], fill=BLACK)
    return im


def angkorwat():
    im, d = canvas(1160, 800)
    def tower(cx, base_y, h, w):
        # stepped lotus bud: three shoulders and the tip
        d.polygon([(cx - w, base_y), (cx - int(w * 0.72), base_y - int(h * 0.45)),
                   (cx - int(w * 0.45), base_y - int(h * 0.75)), (cx, base_y - h),
                   (cx + int(w * 0.45), base_y - int(h * 0.75)),
                   (cx + int(w * 0.72), base_y - int(h * 0.45)), (cx + w, base_y)], fill=BLACK)
    tower(580, 560, 430, 110)
    tower(330, 560, 320, 90)
    tower(830, 560, 320, 90)
    tower(140, 560, 230, 75)
    tower(1020, 560, 230, 75)
    # the gallery joining all five
    d.rectangle([60, 550, 1100, 640], fill=BLACK)
    # causeway across an aligned gap
    d.rectangle([420, 680, 740, 736], fill=BLACK)
    return im


def accordion():
    im, d = canvas(1040, 780)
    # left box with its buttons, right box with the key column
    d.rounded_rectangle([80, 180, 280, 620], radius=40, fill=BLACK)
    d.rounded_rectangle([760, 180, 960, 620], radius=40, fill=BLACK)
    for by in (280, 380, 480):
        d.ellipse([148, by - 22, 212, by + 22], fill=WHITE)            # buttons
    # keys as three micro slots (the 44px column was a slit hole that
    # could not host — the owl's wing-seam failure, third sighting)
    for ky in (280, 380, 480):
        d.rectangle([872, ky - 18, 932, ky + 18], fill=WHITE)
    # the bellows: pleats ARE the silhouette — zigzag top and bottom
    # edges, no interior slits for the solver to refuse
    top = [(280, 250)]
    for k in range(4):
        top += [(340 + k * 120, 190), (400 + k * 120, 250)]
    bot = [(760, 550)]
    for k in range(4):
        bot += [(700 - k * 120, 610), (640 - k * 120, 550)]
    d.polygon(top + [(760, 250)] + bot + [(280, 550)], fill=BLACK)
    return im


def quetzal():
    im, d = canvas(800, 1140)
    # head with its round crest, body
    circle(d, 380, 220, 100, BLACK)
    d.polygon([(280, 200), (230, 250), (300, 265)], fill=BLACK)        # beak
    d.ellipse([300, 280, 520, 560], fill=BLACK)                        # body
    # the wing fold as a hole
    d.ellipse([380, 350, 480, 500], fill=WHITE)
    # eye (micro)
    d.ellipse([352, 180, 392, 212], fill=WHITE)
    # the impossible tail: two long ribbons sweeping down
    d.line([(390, 540), (330, 800), (390, 1060)], fill=BLACK, width=44, joint="curve")
    d.line([(450, 540), (510, 780), (450, 1040)], fill=BLACK, width=44, joint="curve")
    # the perch across an aligned gap... no — quetzals perch; weld it
    d.rectangle([200, 560, 620, 610], fill=BLACK)
    return im


def grandpiano():
    im, d = canvas(1100, 900)
    # the body in profile: flat front, the great curve of the bent side
    d.polygon([(140, 380), (760, 380), (900, 420), (960, 500), (940, 600), (140, 600)],
              fill=BLACK)
    # the lid, propped open on its stick
    d.polygon([(150, 360), (700, 360), (900, 140), (240, 200)], fill=BLACK)
    d.line([(650, 400), (740, 220)], fill=BLACK, width=30)   # the stick welds lid AND body
    # the key band along the front
    d.rectangle([190, 432, 620, 470], fill=WHITE)   # left wall 50px
    # three legs and their castors
    for x in (220, 520, 820):
        d.rectangle([x, 600, x + 52, 760], fill=BLACK)
        circle(d, x + 26, 780, 26, BLACK)
    return im


SUBJECTS = {
    "liberty": liberty,
    "angkorwat": angkorwat,
    "accordion": accordion,
    "quetzal": quetzal,
    "grandpiano": grandpiano,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
