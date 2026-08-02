#!/usr/bin/env python3
"""Culture batch 13 — TEN (Eric, 2026-08-02: "next 10").

koinobori   — ja: carp streamers on the pole
nazcabird   — worldart: the hummingbird geoglyph, ancient line art
              that was BORN for scan-lock
terracotta  — zh: the standing warrior
hamsa       — ar: the hand, its eye watching
nutcracker  — de: the soldier
pinata      — es: the seven-point star... five, for the corridors
dragonboat  — vi: prow to tail
bagpipes    — en: bag, drones, chanter
triskele    — worldart: the triple spiral
mancala     — sw: the board whose pits are pure hole machinery

Run: python3 tools/draw_wave21.py
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


def koinobori():
    im, d = canvas(1060, 940)
    # the pole with its finial
    d.rectangle([120, 90, 164, 880], fill=BLACK)
    circle(d, 142, 70, 34, BLACK)
    # two carp flying right, welded to the pole by fat cords
    def carp(y, L, h):
        d.line([(164, y), (240, y)], fill=BLACK, width=28)
        d.polygon([(240, y - h // 2), (240 + L, y - h // 4), (240 + L - 70, y),
                   (240 + L, y + h // 4), (240, y + h // 2)], fill=BLACK)
        d.ellipse([268, y - 26, 320, y + 14], fill=WHITE)              # eye zone
    carp(240, 640, 220)
    carp(560, 520, 180)
    return im


def nazcabird():
    im, d = canvas(1180, 900)
    w = 42
    # the geoglyph as one continuous fat line: beak, head loop, body,
    # wing feathers, tail prongs — 40px between parallel runs
    d.line([(60, 300), (420, 300)], fill=BLACK, width=w)               # the long beak
    d.line([(420, 300), (500, 240), (580, 300), (500, 360), (430, 310)], fill=BLACK,
           width=w, joint="curve")                                     # head loop
    d.line([(560, 300), (700, 330)], fill=BLACK, width=w)              # neck
    # body diamond
    d.line([(700, 330), (800, 260), (900, 330), (800, 420), (700, 330)], fill=BLACK,
           width=w, joint="curve")
    # wings: three fingers up, three down
    for k in range(3):
        x = 730 + k * 62
        d.line([(x, 280), (x, 90)], fill=BLACK, width=w)
        d.line([(x, 380), (x, 570)], fill=BLACK, width=w)
    # tail prongs
    d.line([(900, 330), (1080, 300)], fill=BLACK, width=w)
    d.line([(900, 360), (1080, 430)], fill=BLACK, width=w)
    return im


def terracotta():
    im, d = canvas(820, 1120)
    circle(d, 410, 190, 110, BLACK)                                    # head
    d.polygon([(450, 90), (520, 40), (540, 110)], fill=BLACK)          # topknot
    # shoulders and robe to the base
    d.polygon([(250, 300), (570, 300), (620, 480), (600, 1000), (220, 1000), (200, 480)],
              fill=BLACK)
    # folded arms: a white seam tracing the sleeves meeting
    d.polygon([(280, 480), (540, 480), (500, 560), (410, 600), (320, 560)], fill=WHITE)
    d.ellipse([368, 520, 452, 584], fill=BLACK)                        # the clasped hands
    # belt seam
    d.rectangle([250, 660, 570, 700], fill=WHITE)
    # base slab
    d.rectangle([180, 1044, 640, 1104], fill=BLACK)   # 44px under the robe
    return im


def hamsa():
    im, d = canvas(900, 1100)
    cx = 450
    # palm
    d.ellipse([cx - 210, 420, cx + 210, 860], fill=BLACK)
    # three fingers, gaps 40px
    for k, (dx, h) in enumerate([(-120, 320), (0, 380), (120, 320)]):
        d.rounded_rectangle([cx + dx - 52, 460 - h, cx + dx + 52, 520], radius=52, fill=BLACK)
    # two thumbs sweeping out
    for sgn in (-1, 1):
        d.ellipse([cx + sgn * 160 - 60, 430, cx + sgn * 160 + 220 * sgn - 60 * -1 if False else cx + sgn * 300 - 60, 640], fill=BLACK) if False else None
        d.polygon([(cx + sgn * 140, 560), (cx + sgn * 330, 380), (cx + sgn * 360, 440),
                   (cx + sgn * 200, 640)], fill=BLACK)   # rooted in the palm
    # the eye: ring + micro pupil
    circle(d, cx, 640, 90, WHITE)
    circle(d, cx, 640, 34, BLACK)
    # fringe drop at the wrist
    d.polygon([(cx - 120, 860), (cx + 120, 860), (cx + 60, 990), (cx - 60, 990)], fill=BLACK)
    return im


def nutcracker():
    im, d = canvas(760, 1180)
    cx = 380
    d.rectangle([cx - 110, 60, cx + 110, 240], fill=BLACK)             # tall hat
    d.rectangle([cx - 130, 240, cx + 130, 290], fill=BLACK)            # brim
    d.rectangle([cx - 122, 130, cx + 122, 172], fill=WHITE)            # hat band
    d.rectangle([cx - 95, 288, cx + 95, 470], fill=BLACK)              # face block, welded to the brim
    d.ellipse([cx - 40, 340, cx + 40, 400], fill=WHITE)                # face patch
    d.rectangle([cx - 95, 470, cx + 95, 560], fill=BLACK)              # the jaw/beard
    d.rectangle([cx - 140, 558, cx + 140, 872], fill=BLACK)            # tunic, welded jaw to legs
    for by in (640, 720, 800):
        circle(d, cx, by, 18, WHITE)                                   # buttons (micro)
    d.rectangle([cx - 120, 870, cx - 30, 1080], fill=BLACK)            # legs
    d.rectangle([cx + 30, 870, cx + 120, 1080], fill=BLACK)
    d.rectangle([cx - 180, 1080, cx + 180, 1130], fill=BLACK)          # base
    return im


def pinata():
    im, d = canvas(1000, 960)
    cx, cy = 500, 480
    circle(d, cx, cy, 150, BLACK)                                      # the core
    # five cones with streamer balls at the tips
    for k in range(5):
        a = -math.pi / 2 + k * 2 * math.pi / 5
        tx, ty = cx + 330 * math.cos(a), cy + 330 * math.sin(a)
        bx1 = cx + 140 * math.cos(a - 0.32)
        by1 = cy + 140 * math.sin(a - 0.32)
        bx2 = cx + 140 * math.cos(a + 0.32)
        by2 = cy + 140 * math.sin(a + 0.32)
        d.polygon([(bx1, by1), (tx, ty), (bx2, by2)], fill=BLACK)
        circle(d, tx, ty, 34, BLACK)
    # the hanging cord
    d.line([(cx, 40), (cx, cy - 320)], fill=BLACK, width=26)
    return im


def dragonboat():
    im, d = canvas(1180, 760)
    # the long hull
    d.polygon([(160, 440), (1000, 440), (1080, 360), (1010, 560), (180, 560), (100, 380)],
              fill=BLACK)
    # scale band seam along the hull
    d.rectangle([260, 480, 900, 516], fill=WHITE)
    # dragon head prow: skull, horn, open jaw — the turtleship lessons
    d.polygon([(150, 460), (110, 300), (170, 260), (230, 320), (220, 460)], fill=BLACK)
    d.polygon([(150, 280), (80, 240), (160, 230)], fill=BLACK)         # horn
    d.polygon([(120, 310), (30, 300), (120, 360)], fill=BLACK)         # upper snout
    d.polygon([(120, 380), (40, 420), (130, 410)], fill=BLACK)         # jaw
    d.ellipse([152, 306, 190, 336], fill=WHITE)                        # eye (micro)
    # the tail sweeping up at the stern
    d.polygon([(1000, 440), (1120, 300), (1160, 340), (1060, 470)], fill=BLACK)
    # water
    d.rectangle([80, 620, 1100, 672], fill=BLACK)
    return im


def bagpipes():
    im, d = canvas(980, 1060)
    # the bag
    d.ellipse([260, 480, 720, 800], fill=BLACK)
    # three drones rising, 46px apart at the roots
    for k, (dx, h) in enumerate([(-130, 380), (0, 460), (130, 380)]):
        d.line([(490 + dx, 540), (430 + dx, 540 - h)], fill=BLACK, width=48)
        d.rounded_rectangle([430 + dx - 40, 540 - h - 70, 430 + dx + 40, 540 - h + 10],
                            radius=24, fill=BLACK)
    # the chanter hanging low with its micro fingerholes
    d.line([(500, 760), (560, 990)], fill=BLACK, width=52)
    for t in (0.35, 0.6, 0.85):
        hx, hy = 500 + 60 * t, 760 + 230 * t
        d.ellipse([hx - 11, hy - 11, hx + 11, hy + 11], fill=WHITE)
    # blowpipe
    d.line([(330, 560), (210, 430)], fill=BLACK, width=44)
    return im


def triskele():
    im, d = canvas(940, 940)
    cx, cy = 470, 470
    circle(d, cx, cy, 90, BLACK)                                       # the hub
    for k in range(3):
        base = k * 2 * math.pi / 3
        pts = []
        for i in range(22):
            t = i / 21
            a = base + t * 2.4
            r = 90 + 260 * t
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
        d.line(pts, fill=BLACK, width=52, joint="curve")
        circle(d, int(pts[-1][0]), int(pts[-1][1]), 40, BLACK)
    return im


def mancala():
    im, d = canvas(1160, 640)
    d.rounded_rectangle([60, 140, 1100, 500], radius=70, fill=BLACK)
    # two stores at the ends
    d.ellipse([100, 200, 220, 440], fill=WHITE)
    d.ellipse([940, 200, 1060, 440], fill=WHITE)
    # two rows of four pits
    for row_y in (240, 400):
        for k in range(4):
            cxp = 330 + k * 170
            circle(d, cxp, row_y, 52, WHITE)
    return im


SUBJECTS = {
    "koinobori": koinobori,
    "nazcabird": nazcabird,
    "terracotta": terracotta,
    "hamsa": hamsa,
    "nutcracker": nutcracker,
    "pinata": pinata,
    "dragonboat": dragonboat,
    "bagpipes": bagpipes,
    "triskele": triskele,
    "mancala": mancala,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
