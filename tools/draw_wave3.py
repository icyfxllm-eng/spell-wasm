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
    """v5. What each round taught: v3's THICK tail stroke read fine (the
    thorns were the thin spikes); v4's long muzzle was right but its
    in-outline tail loop smoothed to a nub. So: v4 head + neck + back on
    one spline, tail as one thick overlapping stroke, no spikes anywhere.
    Sharp ear via a plain polygon on top (splines round ears off)."""
    im, d = canvas(900, 960)
    smooth_poly(d, [
        (230, 80),                        # nose high
        (298, 108), (368, 168),           # long muzzle
        (398, 218), (430, 252),           # stop, skull
        (462, 320), (505, 430),           # nape, shoulders
        (548, 560), (578, 690),           # sloping back
        (598, 800), (565, 890),           # haunch
        (470, 925), (310, 925),           # ground
        (295, 780), (305, 620),           # foreleg column
        (268, 500), (278, 395),           # chest
        (242, 300), (220, 205),           # throat
        (206, 132),                       # underjaw
    ], samples=12)
    d.polygon([(408, 232), (466, 196), (452, 282)], fill=BLACK)  # ear, SHARP
    # Jaw wedge sized into the micro band: 6 units bigger it was a
    # word-hosting contour 5px off the outer boundary and flagged both.
    d.polygon([(233, 96), (302, 138), (238, 162)], fill=WHITE)   # open jaw
    tapered_stroke(d, [(560, 850), (668, 892), (760, 862), (792, 775)], 48, 13)  # tail
    circle(d, 362, 200, 21, WHITE)                                # eye
    return im


def koi():
    """v5. The in-outline fan smoothed to a bowling pin; the detached fan
    left a gap. Answer from the wolf: body spline + fan as its own SOLID
    poly whose root plunges 70px INTO the body, so the join cannot gap and
    the fan corners stay sharp (drawn plain, not splined)."""
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
        circle(d, cx + f * 35, cy - f * 262, 25, WHITE)

    # ±225: at ±195 the inner curves sat 11.8px apart -- the water between
    # the fish is the composition AND the corridor.
    fish(520 - 225, 470, 1)
    fish(520 + 225, 470, -1)
    return im


SUBJECTS = {"wolf": wolf, "koi": koi}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
