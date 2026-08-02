#!/usr/bin/env python3
"""Culture batch 6 (Eric, 2026-08-01: "next five").

waweldragon — pl second icon: Krakow's spiky fire-breather
doubledecker — en second icon: the red bus, windows as holes
halongbay   — vi second icon: karst stacks and a junk sail
carabao     — fil: the spec's own wave-1 name, drawn NOW but
              release-gated with the jeepney until Paul's audit
fleurdelis  — fr second icon: the heraldic lily

Run: python3 tools/draw_wave14.py
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


def waweldragon():
    """Krakow's dragon in clean profile: arched neck, horned head with
    open jaw, four back spikes, a folded wing as a hole, tail with its
    spade — standing on the rock across an aligned gap."""
    im, d = canvas(1060, 900)
    # body: long horizontal mass
    d.ellipse([260, 380, 820, 640], fill=BLACK)
    # neck arching up-left, rooted deep
    d.polygon([(300, 520), (250, 300), (330, 240), (420, 300), (400, 460)], fill=BLACK)
    # head: skull + horn + open jaw, welded to the neck top
    d.polygon([(230, 310), (370, 240), (360, 160), (240, 180), (180, 250)], fill=BLACK)
    d.polygon([(250, 180), (200, 90), (300, 160)], fill=BLACK)          # horn
    d.polygon([(190, 230), (60, 220), (180, 280)], fill=BLACK)          # upper snout
    d.polygon([(225, 280), (80, 340), (228, 328)], fill=BLACK)          # lower jaw, rooted in the skull
    # eye (micro hole)
    d.ellipse([268, 226, 304, 254], fill=WHITE)
    # four back spikes rooted 30px into the body
    for k in range(4):
        cx = 430 + k * 100
        cy = 402 - int(24 * (1 if k in (1, 2) else 0.4))
        d.polygon([(cx - 22, cy + 30), (cx + 22, cy + 30), (cx, cy - 58)], fill=BLACK)
    # folded wing: a hole in the body's shoulder
    d.polygon([(430, 470), (560, 430), (640, 470), (540, 540)], fill=WHITE)
    # legs to the rock line
    d.rectangle([380, 600, 450, 700], fill=BLACK)
    d.rectangle([640, 600, 710, 700], fill=BLACK)
    # tail sweeping right, spade welded
    d.line([(800, 500), (930, 540), (990, 460)], fill=BLACK, width=46)
    d.polygon([(970, 420), (1040, 450), (985, 505)], fill=BLACK)
    # the rock, aligned 40px below the paws
    d.polygon([(300, 740), (800, 740), (880, 870), (230, 870)], fill=BLACK)
    return im


def doubledecker():
    im, d = canvas(1000, 820)
    d.rounded_rectangle([120, 90, 880, 640], radius=70, fill=BLACK)
    # upper deck windows
    for k in range(4):
        d.rounded_rectangle([170 + k * 180, 140, 310 + k * 180, 280], radius=32, fill=WHITE)
    # lower deck: two windows + the doorway
    d.rounded_rectangle([170, 360, 360, 520], radius=32, fill=WHITE)
    d.rounded_rectangle([420, 360, 610, 520], radius=32, fill=WHITE)
    d.rounded_rectangle([680, 360, 830, 600], radius=32, fill=WHITE)   # door
    # wheels
    for cx in (300, 700):
        circle(d, cx, 680, 92, BLACK)
        circle(d, cx, 680, 50, WHITE)
        circle(d, cx, 680, 19, BLACK)
    return im


def halongbay():
    im, d = canvas(1040, 860)
    # three karst stacks, distinct, rounded crowns
    d.polygon([(140, 700), (170, 380), (220, 300), (290, 360), (320, 700)], fill=BLACK)
    d.ellipse([165, 270, 300, 400], fill=BLACK)
    d.polygon([(420, 700), (440, 260), (520, 160), (600, 250), (620, 700)], fill=BLACK)
    d.ellipse([435, 140, 605, 300], fill=BLACK)
    d.polygon([(760, 700), (780, 460), (840, 400), (900, 450), (920, 700)], fill=BLACK)
    d.ellipse([770, 380, 910, 490], fill=BLACK)
    # the junk: on the open water LEFT of the stacks — no gap between
    # stacks is wide enough for hull plus two corridors
    d.polygon([(20, 640), (115, 640), (102, 688), (32, 688)], fill=BLACK)
    d.polygon([(38, 505), (96, 505), (106, 622), (28, 622)], fill=BLACK)
    d.rectangle([48, 615, 88, 650], fill=BLACK)
    # the water: one band under everything (aligned 40px gap)
    d.rectangle([80, 740, 960, 800], fill=BLACK)
    return im


def carabao():
    im, d = canvas(1060, 860)
    # the great crescent horns — the identity
    d.line([(340, 240), (240, 120), (120, 100)], fill=BLACK, width=46)
    d.line([(480, 240), (580, 110), (700, 90)], fill=BLACK, width=46)
    circle(d, 122, 102, 23, BLACK)
    circle(d, 698, 92, 23, BLACK)
    # head, dewlap, stocky body
    d.polygon([(330, 220), (490, 220), (520, 330), (460, 420), (360, 420), (300, 330)],
              fill=BLACK)
    d.polygon([(360, 420), (460, 420), (440, 500), (380, 500)], fill=BLACK)     # muzzle drop
    d.polygon([(420, 300), (900, 280), (960, 420), (940, 560), (420, 560), (380, 420)],
              fill=BLACK)
    # legs + ground
    for x in (470, 610, 750, 870):
        d.rectangle([x, 540, x + 56, 740], fill=BLACK)
    d.rectangle([420, 726, 980, 780], fill=BLACK)
    # tail: rooted INSIDE the flank, angled away
    d.line([(895, 310), (1035, 385)], fill=BLACK, width=36)
    circle(d, 1040, 398, 24, BLACK)
    # eye + nostril (micro holes)
    d.ellipse([386, 268, 424, 298], fill=WHITE)
    d.ellipse([394, 444, 428, 470], fill=WHITE)
    return im


def fleurdelis():
    im, d = canvas(860, 980)
    cx = 430
    # centre petal: lance with swelling waist
    d.polygon([(cx, 60), (cx - 70, 240), (cx - 46, 470), (cx + 46, 470), (cx + 70, 240)],
              fill=BLACK)
    # side petals: crescents by the moonstar construction — the first
    # cut's parametric polygons self-intersected and even-odd fill left
    # white slivers that traced as ghost contours
    for sgn in (-1, 1):
        px0 = cx + sgn * 60
        d.ellipse([min(px0, px0 + sgn * 260), 120, max(px0, px0 + sgn * 260), 470], fill=BLACK)
        bite_x0 = cx + sgn * 30
        d.ellipse([min(bite_x0, bite_x0 + sgn * 190), 90, max(bite_x0, bite_x0 + sgn * 190), 380],
                  fill=WHITE)
        d.rectangle([min(cx + sgn * 60, cx + sgn * 240), 430, max(cx + sgn * 60, cx + sgn * 240), 470],
                    fill=BLACK)
    # the band binding the three
    d.rounded_rectangle([cx - 210, 470, cx + 210, 560], radius=34, fill=BLACK)
    # the three feet below the band
    d.polygon([(cx - 40, 560), (cx + 40, 560), (cx + 24, 760), (cx, 800), (cx - 24, 760)],
              fill=BLACK)
    for s in (-1, 1):
        d.polygon([(cx + s * 190, 560), (cx + s * 90, 560), (cx + s * 150, 700),
                   (cx + s * 210, 680)], fill=BLACK)
    return im


SUBJECTS = {
    "waweldragon": waweldragon,
    "doubledecker": doubledecker,
    "halongbay": halongbay,
    "carabao": carabao,
    "fleurdelis": fleurdelis,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
