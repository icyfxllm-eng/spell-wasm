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
    """Krakow's dragon, take two — Eric: "more realistic if possible".
    (The 1972 Chromy statue is still-copyrighted sculpture, so this is a
    drawn original.) One smooth Catmull-Rom body — sinuous neck, deep
    chest, coiling tail — with a membraned wing (white spoke seams, the
    kite-spar width), ridge scales rooted like the turtle ship's spikes,
    and a crisp angular head welded on: realism lives in the curves,
    identity in the head and wing."""
    im, d = canvas(1120, 920)

    def smooth_closed(knots, per=10):
        def cr(p0, p1, p2, p3, t):
            t2, t3 = t * t, t * t * t
            return (0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
                    0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3))
        pts, n = [], len(knots)
        for i in range(n):
            for k in range(per):
                pts.append(cr(knots[(i - 1) % n], knots[i], knots[(i + 1) % n], knots[(i + 2) % n], k / per))
        return pts

    # the beast: one flowing outline, clockwise from the chest
    body = [
        (350, 700), (300, 620), (285, 520), (300, 430), (280, 360),   # chest up the throat
        (255, 300), (270, 250),                                        # throat to jaw hinge
        (330, 230), (390, 260),                                        # skull back
        (420, 330), (470, 390), (560, 420), (660, 420), (760, 450),    # nape and back
        (850, 480), (950, 520), (1030, 560), (1060, 600),              # tail tapering out
        (1000, 620), (900, 610), (820, 620),                           # under the tail
        (760, 640), (700, 680), (660, 740),                            # haunch
        (560, 760), (470, 750), (420, 720),                            # belly
    ]
    d.polygon(smooth_closed(body), fill=BLACK)
    # crisp head: brow, snout, open jaw with its fang notch
    d.polygon([(255, 295), (170, 235), (70, 235), (160, 295), (255, 340)], fill=BLACK)   # upper jaw, fuller
    d.polygon([(310, 335), (110, 380), (200, 395), (315, 390)], fill=BLACK)              # lower jaw, 40px into the throat
    d.polygon([(300, 245), (280, 160), (345, 225)], fill=BLACK)                          # horn
    d.polygon([(360, 240), (370, 150), (415, 245)], fill=BLACK)                          # horn
    d.ellipse([292, 268, 330, 298], fill=WHITE)                                          # eye
    # the wing: ONE falcate bat-wing polygon — the outline itself does
    # the realism (spokes and carved scallops kept severing the membrane
    # into flagged strips; the cusps are in the polygon now)
    d.polygon([(520, 460), (470, 300), (500, 150), (640, 80),
               (760, 150), (700, 230), (850, 250), (790, 330),
               (900, 370), (800, 430), (660, 470)], fill=BLACK)
    # ridge scales rooted on the spine, standing proud
    for (cx, cy) in [(460, 380), (560, 405), (660, 408), (770, 445), (865, 500)]:
        d.polygon([(cx - 20, cy + 34), (cx + 20, cy + 34), (cx, cy - 46)], fill=BLACK)
    # legs with three-toed feet
    for x in (430, 640):
        d.rectangle([x, 700, x + 64, 800], fill=BLACK)
        d.polygon([(x - 14, 800), (x + 78, 800), (x + 88, 836), (x - 26, 836)], fill=BLACK)
    # tail spade
    d.polygon([(1040, 560), (1110, 545), (1100, 640), (1045, 615)], fill=BLACK)
    # the rock, aligned gap below the feet
    d.polygon([(340, 872), (820, 872), (900, 910), (260, 910)], fill=BLACK)
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
