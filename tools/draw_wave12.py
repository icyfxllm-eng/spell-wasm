#!/usr/bin/env python3
"""Wave 12 — Egyptian symbols (Eric, 2026-08-01: "are there versions of
these on the table?"). His references were google images and stay
untraced (the wolf rule); the DESIGNS are ancient — millennia past any
copyright — so these are drawn originals of the same subjects, the
anubis/gyenyame precedent.

eyeofhorus — the wedjat: brow, almond eye with its iris, the spiral
             curl and the teardrop stalk beneath.
ankh       — the loop (a hostable ring), arms and stem.

Run: python3 tools/draw_wave12.py
"""
import math
import pathlib

from PIL import Image, ImageDraw

REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"
BLACK, WHITE = 0, 255


def canvas(w, h):
    im = Image.new("L", (w, h), WHITE)
    return im, ImageDraw.Draw(im)


def eyeofhorus():
    im, d = canvas(980, 760)
    # brow: a thick sweeping bar, detached 40px above the eye
    d.polygon([(180, 150), (420, 80), (700, 96), (760, 150), (700, 160),
               (430, 148), (210, 205)], fill=BLACK)
    # the almond eye: filled, then opened to leave a thick rim
    d.polygon([(160, 320), (330, 230), (520, 210), (680, 250), (760, 320),
               (660, 390), (480, 420), (300, 400)], fill=BLACK)
    d.polygon([(250, 320), (360, 265), (520, 250), (650, 285), (700, 320),
               (620, 360), (470, 380), (330, 368)], fill=WHITE)
    # iris riding in the opening (micro island)
    # iris sized into the micro band — full-size it flagged the rim
    d.ellipse([438, 280, 514, 352], fill=BLACK)
    # kohl tail sweeping right off the eye corner
    d.polygon([(760, 320), (930, 300), (940, 336), (770, 352)], fill=BLACK)
    # teardrop stalk straight down from under the eye
    d.polygon([(430, 416), (490, 416), (480, 620), (440, 620)], fill=BLACK)
    # the spiral curl: an arc sweeping left then hooking
    pts = []
    for k in range(20):
        a = math.pi * 0.55 + k * 0.11
        r = 190 - k * 7
        pts.append((295 + r * math.cos(a), 500 + r * math.sin(a) * 0.62))
    d.line(pts, fill=BLACK, width=40, joint="curve")
    return im


def ankh():
    im, d = canvas(720, 980)
    d.ellipse([210, 60, 510, 420], fill=BLACK)          # the loop
    d.ellipse([280, 130, 440, 340], fill=WHITE)         # opened: a hostable ring
    d.rectangle([120, 420, 600, 500], fill=BLACK)       # arms
    d.rectangle([310, 380, 410, 920], fill=BLACK)       # stem
    return im


SUBJECTS = {"eyeofhorus": eyeofhorus, "ankh": ankh}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        fn().save(REF / f"{name}.png")
        print("wrote", name)
