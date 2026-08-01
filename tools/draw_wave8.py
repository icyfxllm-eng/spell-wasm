#!/usr/bin/env python3
"""CC-PICTURE-BANK feature 2 — culture batch 3, drawn.

Eric's "next 5" (2026-08-01): turtle ship (ko — named by the spec),
croissant (fr), baobab (sw), nón lá (vi), and a German retry — the
cuckoo clock — since the pretzel left that language's slot open.
Same idiom, same field rules as every drawn wave.

Run: python3 tools/draw_wave8.py
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


def turtleship():
    """The geobukseon: dragon head at the prow, the spiked shell roof,
    hull with oar row as a hostable slit, on its waterline."""
    im, d = canvas(1000, 760)
    # shell roof: a broad dome with spike studs riding its back
    d.ellipse([220, 210, 800, 470], fill=BLACK)
    # spikes are the identity: tall studs standing proud of the shell,
    # rooted 24px into it (first cut half-buried them and they melted)
    for k in range(6):
        cx = 290 + k * 90
        t = math.pi * (k + 0.5) / 6
        cy = 340 - int(130 * math.sin(t))
        d.polygon([(cx - 17, cy + 30), (cx + 17, cy + 30), (cx, cy - 62)], fill=BLACK)
    # hull under the shell
    d.polygon([(180, 400), (840, 400), (790, 560), (230, 560)], fill=BLACK)
    # oar-deck slit: one long hostable hole in the hull
    d.rectangle([300, 440, 720, 490], fill=WHITE)
    # dragon head on its neck, jaw open
    d.rectangle([120, 300, 230, 380], fill=BLACK)
    circle(d, 110, 320, 62, BLACK)
    d.polygon([(64, 296), (14, 258), (70, 344)], fill=BLACK)           # open jaw
    circle(d, 106, 314, 17, WHITE)   # eye (micro) — centred deep so it clears the head outline
    # stern tail fin
    d.polygon([(800, 430), (950, 360), (920, 500), (800, 500)], fill=BLACK)  # rooted into the hull
    # waterline
    d.line([(120, 640), (900, 640)], fill=BLACK, width=36)
    return im


def croissant():
    """The croissant: crescent body with the two tapering horns, crimp
    seams as wide white creases across the body (the kite-spar width)."""
    im, d = canvas(860, 700)
    # crescent: fat ring minus a bite from above
    circle(d, 430, 430, 300, BLACK)
    circle(d, 430, 180, 260, WHITE)
    # horn tapers: curl OUTWARD and down, not up (the first cut's upward
    # peaks read as cat ears on the review render)
    d.polygon([(160, 380), (80, 430), (140, 520), (240, 470)], fill=BLACK)
    d.polygon([(700, 380), (780, 430), (720, 520), (620, 470)], fill=BLACK)
    # crimp seams: two wide creases dividing the body into three lobes
    d.line([(330, 380), (300, 620)], fill=WHITE, width=38)
    d.line([(530, 380), (560, 620)], fill=WHITE, width=38)
    return im


def baobab():
    """The upside-down tree: massive bottle trunk, root-like crown of
    thick bare branches, honestly fused to its ground line."""
    im, d = canvas(900, 860)
    d.polygon([(330, 760), (350, 500), (340, 380), (560, 380), (550, 500), (570, 760)],
              fill=BLACK)                                              # bottle trunk
    # crown: thick tapering branches reaching up and out
    for (x0, y0, x1, y1, w) in [
        (380, 400, 180, 240, 46), (180, 240, 90, 190, 30),
        (180, 240, 200, 130, 28), (450, 390, 450, 150, 44),
        (450, 170, 340, 90, 28), (450, 170, 560, 80, 28),
        (520, 400, 720, 240, 46), (720, 240, 810, 190, 30),
        (720, 240, 700, 120, 28),
    ]:
        d.line([(x0, y0), (x1, y1)], fill=BLACK, width=w)
    # ground
    d.line([(180, 780), (720, 780)], fill=BLACK, width=40)
    return im


def nonla():
    """Nón lá over an áo dài figure: the conical hat, the flowing tunic
    with its side slits as white seams, one graceful silhouette."""
    im, d = canvas(700, 960)
    d.polygon([(350, 90), (130, 300), (570, 300)], fill=BLACK)         # the cone
    d.line([(350, 300), (350, 360)], fill=BLACK, width=40)             # neck under the brim
    # áo dài: fitted top flowing into two panels
    d.polygon([(280, 360), (420, 360), (450, 520), (470, 840), (380, 840),
               (370, 600), (330, 600), (320, 840), (230, 840), (250, 520)],
              fill=BLACK)
    # sleeves sweeping out
    d.polygon([(280, 380), (170, 520), (210, 560), (300, 460)], fill=BLACK)
    d.polygon([(420, 380), (530, 520), (490, 560), (400, 460)], fill=BLACK)
    return im


def cuckoo():
    """The Black Forest cuckoo clock: chalet gable, the dial as a hole
    with micro hands, the little door above it, two hanging pinecone
    weights and the pendulum on their chains."""
    im, d = canvas(760, 980)
    d.polygon([(380, 60), (110, 330), (650, 330)], fill=BLACK)         # gable roof
    d.rectangle([170, 330, 590, 620], fill=BLACK)                      # chalet body
    circle(d, 380, 470, 88, WHITE)                                     # dial hole
    d.line([(380, 470), (380, 415)], fill=BLACK, width=12)             # hour hand
    d.line([(380, 470), (425, 452)], fill=BLACK, width=9)              # minute hand
    d.rectangle([345, 240, 415, 320], fill=WHITE)                      # cuckoo door
    # chains with pinecone weights and the pendulum
    d.line([(280, 620), (280, 800)], fill=BLACK, width=16)
    d.line([(480, 620), (480, 740)], fill=BLACK, width=16)
    d.line([(380, 620), (380, 850)], fill=BLACK, width=16)
    d.ellipse([250, 790, 310, 900], fill=BLACK)                        # pinecone
    d.ellipse([450, 730, 510, 840], fill=BLACK)                        # pinecone
    circle(d, 380, 890, 52, BLACK)                                     # pendulum bob
    return im


SUBJECTS = {
    "turtleship": turtleship,
    "croissant": croissant,
    "baobab": baobab,
    "nonla": nonla,
    "cuckoo": cuckoo,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
