#!/usr/bin/env python3
"""Wave 11 — world art five (Eric, 2026-08-01: "next strong world art 5").

Five civilizations, all drawn originals after ancient or long-PD
traditions — the extraction lessons said bold flat shapes win, and
these five ARE bold flat shapes:

anubis     — after Egyptian tomb painting (ancient): the seated jackal
athenaowl  — after the Athenian owl motif (ancient): the coin's bird
bamboo     — after the Chinese literati ink tradition (Zheng Xie et al.)
girih      — after Islamic geometric interlace: the eight-point star
gyenyame   — after the Adinkra symbol (Ghana): "except for God"

Field rules as ever: seams >= 34px, micro band (40, 130) at 512,
hinge overlaps >= 20px.

Run: python3 tools/draw_wave11.py
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


def anubis():
    """The seated jackal in profile, after the tomb paintings: tall ears,
    long snout, deep chest, forelegs, the curled tail along the base.
    The eye is the micro hole; the collar is a white band seam."""
    im, d = canvas(860, 960)
    # head: snout wedge + skull
    d.polygon([(150, 210), (330, 150), (345, 205), (185, 245)], fill=BLACK)   # snout
    d.polygon([(300, 130), (420, 150), (430, 260), (320, 250)], fill=BLACK)   # skull
    # the tall ears
    d.polygon([(320, 145), (300, 30), (360, 120)], fill=BLACK)
    d.polygon([(380, 150), (400, 24), (430, 140)], fill=BLACK)
    # neck sweeping into the deep chest — rooted UP INTO the skull
    # (hinge overlap; the first cut left a hairline and the head floated)
    d.polygon([(330, 180), (430, 180), (470, 420), (450, 640), (330, 640), (360, 420)],
              fill=BLACK)
    # collar: a white band across the neck
    d.polygon([(338, 300), (444, 300), (450, 344), (336, 344)], fill=WHITE)
    # haunches and body
    d.polygon([(330, 470), (560, 430), (640, 560), (650, 780), (330, 780)], fill=BLACK)
    # forelegs to the base
    d.rectangle([350, 640, 420, 880], fill=BLACK)
    d.rectangle([440, 640, 500, 880], fill=BLACK)
    # hind paw + base
    d.rectangle([540, 780, 660, 880], fill=BLACK)
    d.rectangle([300, 862, 700, 910], fill=BLACK)
    # the tail curling along the base
    d.line([(640, 800), (740, 850), (720, 905)], fill=BLACK, width=40)
    # the eye (micro hole)
    d.ellipse([352, 196, 392, 226], fill=WHITE)
    return im


def athenaowl():
    """The little owl of Athena, frontal like the tetradrachm: two big
    eye rings (hostable holes) with dark pupils riding inside as micro
    islands, the wing seam, and the olive sprig at its shoulder."""
    im, d = canvas(860, 960)
    d.ellipse([220, 120, 640, 480], fill=BLACK)                        # head
    d.polygon([(250, 400), (610, 400), (580, 800), (280, 800)], fill=BLACK)  # body
    d.ellipse([280, 760, 580, 860], fill=BLACK)
    # brow tufts
    d.polygon([(240, 150), (300, 60), (340, 140)], fill=BLACK)
    d.polygon([(620, 150), (560, 60), (520, 140)], fill=BLACK)
    # the great eyes: holes with pupils inside
    circle(d, 350, 290, 88, WHITE)
    circle(d, 510, 290, 88, WHITE)
    circle(d, 350, 290, 30, BLACK)
    circle(d, 510, 290, 30, BLACK)
    # beak between them
    d.polygon([(414, 330), (446, 330), (430, 396)], fill=WHITE)
    # wing seam down the body
    d.line([(540, 440), (560, 740)], fill=WHITE, width=36)
    # feet
    d.rectangle([330, 850, 380, 920], fill=BLACK)
    d.rectangle([470, 850, 520, 920], fill=BLACK)
    # olive sprig: stem + three leaf lances, detached from the shoulder
    d.line([(150, 320), (190, 520)], fill=BLACK, width=22)
    for (lx, ly, a) in [(150, 350, -0.9), (170, 430, -0.6), (185, 500, -0.8)]:
        ex, ey = lx - int(90 * math.cos(a)), ly + int(90 * math.sin(a))
        d.line([(lx, ly), (ex, ey)], fill=BLACK, width=30)
    return im


def bamboo():
    """Two stalks after the literati ink manner: fat segments with the
    node gaps the brush leaves, and three leaf fans welded to the culms.
    Every leaf is a lance the corridor can live with."""
    im, d = canvas(860, 1060)

    def stalk(x, w, segs):
        y = 1000
        for (h) in segs:
            d.rounded_rectangle([x - w // 2, y - h, x + w // 2, y], radius=w // 3, fill=BLACK)
            y -= h + 42                                                # the node gap
    stalk(300, 56, [220, 240, 260, 180])
    stalk(560, 44, [180, 200, 220, 230])

    def leaf_fan(cx, cy, angles, L=170, w=44):
        for a in angles:
            ex, ey = cx + int(L * math.cos(a)), cy - int(L * math.sin(a))
            d.line([(cx, cy), (ex, ey)], fill=BLACK, width=w)
            circle(d, ex, ey, w // 2, BLACK)
    leaf_fan(300, 320, [2.4, 2.9, 3.4])
    leaf_fan(560, 420, [0.3, -0.2, 5.6])
    leaf_fan(430, 150, [1.0, 0.45, 2.2])
    # a twig joining the top fan to the tall stalk
    d.line([(300, 260), (430, 150)], fill=BLACK, width=26)
    return im


def girih():
    """The eight-pointed star of the girih tradition: two squares turned
    through 45 degrees, the inner octagon opened as a hostable ring, and
    eight micro kites orbiting the points."""
    im, d = canvas(920, 920)
    cx, cy, R = 460, 460, 330

    def square(rot):
        pts = []
        for k in range(4):
            a = rot + k * math.pi / 2
            pts.append((cx + R * math.cos(a), cy + R * math.sin(a)))
        d.polygon(pts, fill=BLACK)
    square(0)
    square(math.pi / 4)
    # the inner octagon hole — a ring wide enough to host
    r = 165
    d.polygon([(cx + r * math.cos(k * math.pi / 4 + math.pi / 8),
                cy + r * math.sin(k * math.pi / 4 + math.pi / 8)) for k in range(8)], fill=WHITE)
    # eight micro kites orbiting between the points
    for k in range(8):
        a = k * math.pi / 4
        kx, ky = cx + 262 * math.cos(a), cy + 262 * math.sin(a)
        d.polygon([(kx, ky - 30), (kx + 22, ky), (kx, ky + 30), (kx - 22, ky)], fill=WHITE)
    return im


def gyenyame():
    """Gye Nyame, "except for God" — the Adinkra swirl drawn with
    respect: the great S-form with its two spiral knobs and the combed
    ridges along each back."""
    im, d = canvas(900, 900)
    # the S body: two lobes around a waist
    d.ellipse([180, 140, 560, 520], fill=BLACK)
    d.ellipse([340, 380, 720, 760], fill=BLACK)
    d.ellipse([300, 130, 570, 360], fill=WHITE)                        # open the upper lobe
    d.ellipse([330, 540, 600, 770], fill=WHITE)                        # open the lower lobe
    # the spiral knobs at each heart
    circle(d, 400, 250, 52, BLACK)
    circle(d, 470, 650, 52, BLACK)
    # combed ridges: teeth along the outer backs
    for k in range(4):
        a = 2.4 + k * 0.28
        tx, ty = 370 + int(230 * math.cos(a)), 330 - int(230 * math.sin(a))
        d.line([(370 + int(160 * math.cos(a)), 330 - int(160 * math.sin(a))), (tx, ty)],
               fill=BLACK, width=34)
    for k in range(4):
        a = 5.5 + k * 0.28
        tx, ty = 530 + int(230 * math.cos(a)), 570 - int(230 * math.sin(a))
        d.line([(530 + int(160 * math.cos(a)), 570 - int(160 * math.sin(a))), (tx, ty)],
               fill=BLACK, width=34)
    return im


SUBJECTS = {
    "anubis": anubis,
    "athenaowl": athenaowl,
    "bamboo": bamboo,
    "girih": girih,
    "gyenyame": gyenyame,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
