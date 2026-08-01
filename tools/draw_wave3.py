#!/usr/bin/env python3
"""CC-PICTURE-BANK wave 1, batch 3 — the advanced tier, drawn.

Two subjects (dragon, the third advanced, already ships): the wolf howling
and the koi pair. Advanced is the texture/flow tier, so the silhouettes
carry texture in their EDGES — fur tufts breaking the wolf's ruff and
back, the koi's fins trailing like wash — rather than interior noise the
solver would have to fight. Tier "hard": the pools bring the long words.

Same contract as batches 1-2: literal coordinates, drawn originals,
features that must survive are holes, micro bits detached and sized into
the (40,130) band.
"""
import math
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from draw_wave2 import BLACK, WHITE, canvas, circle, smooth_poly, tapered_stroke

REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"


def wolf():
    """v6, to Eric's reference pose: STANDING on a ridge, head thrown all
    the way back, muzzle near vertical, ears flat to the neck, chest ruff
    breaking into fur, bushy tail hanging down-back. All four legs in the
    outline with V-notches between; the ridge is its own bar below.
    (Drawn from the pose, never traced -- the reference stays out of the
    pipeline.)"""
    im, d = canvas(1040, 900)
    smooth_poly(d, [
        (355, 55),                        # nose, thrown back
        (405, 75), (420, 140),            # muzzle top edge (near vertical)
        (438, 190),                       # stop
        (472, 210), (505, 200),           # ear flat back
        (505, 245),                       # ear rear edge
        (540, 300), (600, 345),           # thick neck into withers
        (700, 360), (790, 365),           # level back
        (830, 400),                       # rump
        (850, 480), (830, 560),           # near hind leg back edge
        (845, 650),                       # hock down
        (800, 655), (795, 560),           # hind paw and front edge
        (770, 480),                       # up to belly...
        (735, 560), (740, 650),           # far hind leg
        (695, 652), (690, 540),           # its paw, up
        (660, 460),                       # belly line
        (560, 470),                       # belly forward
        (545, 560), (548, 655),           # far foreleg
        (505, 655), (505, 545),           # paw, up
        (488, 470),                       # chest bottom
        (452, 560), (455, 660),           # near foreleg
        (410, 660), (412, 540),           # paw, up
        (395, 440),                       # chest rising
        (350, 400), (368, 330),           # ruff break (jagged chest)
        (320, 290), (345, 225),           # second ruff break
        (318, 160), (335, 95),            # throat, taut, up to the jaw
    ], samples=10)
    # bushy tail: thick stroke off the rump, hanging down-back
    tapered_stroke(d, [(838, 415), (930, 480), (975, 580), (958, 665)], 50, 15)
    # open jaw wedge (micro band)
    d.polygon([(357, 68), (410, 108), (362, 128)], fill=WHITE)
    # the ridge underfoot: a separate bar with a broken edge
    d.polygon([(360, 700), (960, 700), (985, 760), (420, 770), (335, 735)], fill=BLACK)
    return im


def koi():
    """v9. "Just the outline" meant no INTERIOR detail (the reference's
    scales and ornament), not finless fish -- misread, corrected. So:
    outline silhouettes WITH their fins. The v5 body spline (the one that
    read as a fish), a sharp buried-root fan tail, and one flowing
    pectoral per fish. No scales, no eyes, no interior anything."""
    im, d = canvas(1060, 940)

    def fish(cx, cy, f):
        smooth_poly(d, [
            (cx - f * 15, cy - f * 325),           # nose toward centre
            (cx + f * 95, cy - f * 295),           # head
            (cx + f * 175, cy - f * 190),          # shoulder
            (cx + f * 190, cy - f * 30),           # outer flank
            (cx + f * 145, cy + f * 110),          # bend
            (cx + f * 75, cy + f * 215),           # tail waist
            (cx - f * 5, cy + f * 230),            # waist inner
            (cx - f * 55, cy + f * 130),           # inner belly
            (cx - f * 72, cy - f * 30),            # deep comma curve
            (cx - f * 60, cy - f * 185),           # throat
            (cx - f * 45, cy - f * 285),           # chin
        ], samples=12)
        d.polygon([
            (cx + f * 55, cy + f * 160),
            (cx + f * 150, cy + f * 330),
            (cx + f * 85, cy + f * 380),
            (cx + f * 45, cy + f * 440),
            (cx - f * 15, cy + f * 375),
            (cx - f * 75, cy + f * 400),
            (cx - f * 10, cy + f * 170),
        ], fill=BLACK)
        d.polygon([
            (cx + f * 160, cy - f * 175),
            (cx + f * 250, cy - f * 205),
            (cx + f * 285, cy - f * 160),
            (cx + f * 235, cy - f * 120),
            (cx + f * 178, cy - f * 128),
        ], fill=BLACK)

    fish(530 - 225, 470, 1)
    fish(530 + 225, 470, -1)
    return im


SUBJECTS = {"wolf": wolf, "koi": koi}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
