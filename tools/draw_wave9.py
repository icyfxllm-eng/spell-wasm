#!/usr/bin/env python3
"""CC-PICTURE-BANK feature 2 — culture batch 4, drawn.

Eric's "next 5" (2026-08-01): second icons for the strong languages,
two of them named by the spec's own wave-1 list — torii gate (ja) and
hanbok (ko) — plus matryoshka (ru), the Great Wall (zh) and the oud
(ar). Same idiom, same field rules as every drawn wave.

Run: python3 tools/draw_wave9.py
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


def torii():
    """The gate: two slightly inclined pillars, the curved kasagi beam
    with upswept ends, the nuki tie-beam through both pillars, and the
    centre strut — one welded frame."""
    im, d = canvas(940, 800)
    # kasagi: top beam, thicker at centre, ends sweeping up
    d.polygon([(80, 190), (140, 150), (470, 120), (800, 150), (860, 190),
               (820, 230), (470, 200), (120, 230)], fill=BLACK)
    # shimaki: the straight cap under it
    d.rectangle([150, 216, 790, 262], fill=BLACK)
    # pillars, inclined inward
    d.polygon([(200, 240), (270, 240), (300, 740), (230, 740)], fill=BLACK)
    d.polygon([(670, 240), (740, 240), (710, 740), (640, 740)], fill=BLACK)
    # nuki: the tie-beam passing through both pillars
    d.rectangle([150, 380, 790, 440], fill=BLACK)
    # gakuzuka: centre strut between shimaki and nuki
    d.rectangle([440, 262, 500, 380], fill=BLACK)
    return im


def matryoshka():
    """Nesting dolls, the big one and her little sister: bell body with
    scarf-round head, the face an oval hole, the apron a big hostable
    hole with a painted flower as a micro dot inside it."""
    im, d = canvas(920, 900)

    def doll(cx, base_y, s):
        head_r = int(95 * s)
        body_rx = int(180 * s)
        body_h = int(430 * s)
        head_cy = base_y - body_h - int(head_r * 0.6)
        # body bell
        d.polygon([(cx - body_rx, base_y), (cx - int(body_rx * 0.92), base_y - int(body_h * 0.55)),
                   (cx - int(head_r * 0.9), head_cy + head_r),
                   (cx + int(head_r * 0.9), head_cy + head_r),
                   (cx + int(body_rx * 0.92), base_y - int(body_h * 0.55)),
                   (cx + body_rx, base_y)], fill=BLACK)
        d.ellipse([cx - body_rx, base_y - int(140 * s), cx + body_rx, base_y + int(60 * s)],
                  fill=BLACK)
        circle(d, cx, head_cy, head_r, BLACK)
        # face hole
        # face ratio 0.55: at 0.62 the little sister's scarf ring fell
        # below the corridor floor at her scale
        d.ellipse([cx - int(head_r * 0.55), head_cy - int(head_r * 0.55),
                   cx + int(head_r * 0.55), head_cy + int(head_r * 0.60)], fill=WHITE)
        # apron hole with its painted flower (micro)
        ar = int(body_rx * 0.62)
        acy = base_y - int(body_h * 0.32)
        d.ellipse([cx - ar, acy - ar, cx + ar, acy + int(ar * 1.05)], fill=WHITE)
        circle(d, cx, acy, int(26 * s), BLACK)

    doll(280, 820, 0.96)
    doll(660, 820, 0.75)
    return im


def greatwall():
    """The wall winding over two ridges, crenellated all along its back,
    with a watchtower riding each crest — one continuous mass, the way
    the wall itself is."""
    im, d = canvas(1060, 820)
    # the ribbon: top spine points over two humps, then the same spine
    # displaced downward for the wall's thickness
    spine = [(60, 620), (170, 520), (300, 380), (430, 420), (560, 520),
             (690, 480), (820, 340), (1000, 300)]
    band = 130
    top = spine
    bottom = [(x, y + band) for (x, y) in reversed(spine)]
    d.polygon(top + bottom, fill=BLACK)
    # crenellations: teeth standing on the spine
    for i in range(len(spine) - 1):
        (x0, y0), (x1, y1) = spine[i], spine[i + 1]
        seg = math.hypot(x1 - x0, y1 - y0)
        n = max(2, int(seg / 64))
        for k in range(n):
            t = (k + 0.5) / n
            x, y = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            d.rectangle([x - 14, y - 34, x + 14, y + 6], fill=BLACK)
    # watchtowers on the two crests
    for (cx, cy) in [(300, 380), (820, 340)]:
        d.rectangle([cx - 64, cy - 150, cx + 64, cy + 40], fill=BLACK)
        for k in (-44, 0, 44):
            d.rectangle([cx + k - 14, cy - 186, cx + k + 14, cy - 144], fill=BLACK)
        # tower gate as a hole
        d.rectangle([cx - 24, cy - 96, cx + 24, cy - 10], fill=WHITE)
        d.ellipse([cx - 24, cy - 118, cx + 24, cy - 72], fill=WHITE)
    return im


def hanbok():
    """The women's hanbok: jeogori with its wide curved sleeves, the
    great bell of the chima, and the otgoreum ribbon falling from the
    knot — one flowing silhouette."""
    im, d = canvas(860, 960)
    # chima: the bell skirt
    d.polygon([(430, 330), (560, 360), (660, 500), (720, 700), (740, 880),
               (120, 880), (140, 700), (200, 500), (300, 360)], fill=BLACK)
    d.ellipse([120, 820, 740, 920], fill=BLACK)
    # jeogori: fitted top
    d.polygon([(320, 200), (540, 200), (560, 340), (300, 340)], fill=BLACK)
    # collar V as a white notch, well inside the jeogori (10px off the
    # top edge flagged)
    d.polygon([(430, 236), (386, 316), (474, 316)], fill=WHITE)
    # sleeves: wide curves out to the cuffs
    d.polygon([(330, 210), (150, 300), (120, 380), (230, 380), (330, 300)], fill=BLACK)
    d.polygon([(530, 210), (710, 300), (740, 380), (630, 380), (530, 300)], fill=BLACK)
    # otgoreum: the two ribbon tails from the knot
    d.polygon([(400, 340), (440, 340), (420, 560), (386, 560)], fill=BLACK)
    d.polygon([(450, 340), (486, 350), (470, 500), (440, 495)], fill=BLACK)
    return im


def oud():
    """The oud: deep pear body with its rosette as a hole, short wide
    neck, and the sharply bent-back pegbox that makes it an oud and not
    a guitar."""
    im, d = canvas(760, 980)
    # body: deep pear
    d.ellipse([160, 380, 600, 900], fill=BLACK)
    d.polygon([(250, 470), (330, 330), (430, 330), (510, 470)], fill=BLACK)
    # rosette hole
    circle(d, 380, 620, 78, WHITE)
    # bridge slit near the tail (micro)
    d.rectangle([320, 800, 440, 828], fill=WHITE)
    # neck
    d.polygon([(330, 340), (430, 340), (410, 160), (350, 160)], fill=BLACK)
    # pegbox: bent sharply back at ~60 degrees
    d.polygon([(350, 172), (410, 172), (300, 60), (250, 96)], fill=BLACK)
    return im


SUBJECTS = {
    "torii": torii,
    "matryoshka": matryoshka,
    "greatwall": greatwall,
    "hanbok": hanbok,
    "oud": oud,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
