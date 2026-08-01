#!/usr/bin/env python3
"""CC-PICTURE-BANK feature 2 — culture batch 2, drawn.

Eric's "next five" (2026-08-01): calavera (es — named by the spec
itself), Taj Mahal (hi), white stork on its nest (pl), caravel (pt),
Big Ben (en). Same idiom as every drawn wave: solid subject on white,
features as holes, micro features in the (40, 130) arc band at 512.

Field rules: seams >= 34px, hinge overlaps >= 20px, detached micro
>= 10px off the silhouette, one-unbroken-outline unless a seam is wide
enough to live (the crane's lesson stands).

Run: python3 tools/draw_wave7.py
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


def flower_hole(d, cx, cy, r):
    """A petaled hole: the calavera's flower eye — a white disc with
    eight petal bumps, one scalloped interior contour."""
    circle(d, cx, cy, r, WHITE)
    for k in range(8):
        a = k * math.pi / 4
        circle(d, cx + math.cos(a) * r, cy + math.sin(a) * r, int(r * 0.42), WHITE)


def calavera():
    """Day of the Dead skull: cranium and jaw as one silhouette, flower
    eyes as scalloped holes, the heart nose and chin dots in the micro
    band — sugar-skull decoration by the snowman's-eyes machinery."""
    im, d = canvas(760, 880)
    circle(d, 380, 340, 280, BLACK)                                    # cranium
    d.polygon([(180, 500), (580, 500), (540, 700), (460, 800), (300, 800), (220, 700)],
              fill=BLACK)                                              # jaw
    # eyes sized twice: r=78 flagged against the cranium edge, r=64
    # merged into one double-flower at the centreline; r=56 breathes both ways
    flower_hole(d, 272, 320, 56)                                       # left flower eye
    flower_hole(d, 488, 320, 56)                                       # right flower eye
    # heart nose (inverted): two lobes + point, as a hole
    circle(d, 358, 480, 26, WHITE)
    circle(d, 402, 480, 26, WHITE)
    d.polygon([(334, 492), (426, 492), (380, 548)], fill=WHITE)
    # stitched smile: a white bar with tooth notches
    d.rectangle([260, 620, 500, 650], fill=WHITE)
    for x in range(285, 480, 48):
        d.rectangle([x, 596, x + 20, 674], fill=WHITE)
    # forehead + chin marigold dots (micro)
    circle(d, 380, 130, 30, WHITE)
    circle(d, 300, 730, 22, WHITE)
    circle(d, 460, 730, 22, WHITE)
    return im


def tajmahal():
    """The mausoleum: great onion dome on its drum, the iwan arch as a
    tall white hole, two flanking minarets with 40px seams, all fused at
    the plinth (the oak's ground rule)."""
    im, d = canvas(940, 760)
    # main block
    d.rectangle([260, 330, 680, 600], fill=BLACK)
    # great dome: bulb + pinched tip on a drum
    d.rectangle([390, 300, 550, 340], fill=BLACK)
    d.ellipse([360, 130, 580, 350], fill=BLACK)
    d.polygon([(470, 60), (455, 145), (485, 145)], fill=BLACK)         # finial
    # corner chhatris (small domed kiosks) on the roofline
    for cx in (300, 640):
        circle(d, cx, 305, 34, BLACK)
        d.rectangle([cx - 26, 305, cx + 26, 340], fill=BLACK)
    # iwan: the great arch doorway as a hole
    d.rectangle([420, 430, 520, 600], fill=WHITE)
    d.ellipse([420, 380, 520, 480], fill=WHITE)
    # minarets: 44px shafts with cap domes, 40px clear of the block
    for cx in (150, 790):
        d.rectangle([cx - 22, 260, cx + 22, 600], fill=BLACK)
        circle(d, cx, 250, 30, BLACK)
    d.rectangle([80, 600, 860, 680], fill=BLACK)                       # plinth
    return im


def stork():
    """The Polish white stork standing on its nest: body, S-neck, spike
    beak and tail as one silhouette; legs are honest 26px strokes rooted
    in the nest mass (the wolf-perch answer to 'standing on')."""
    im, d = canvas(800, 900)
    d.ellipse([260, 330, 560, 530], fill=BLACK)                        # body
    d.polygon([(300, 380), (200, 300), (168, 190), (210, 175), (270, 300), (340, 370)],
              fill=BLACK)                                              # neck
    circle(d, 190, 165, 42, BLACK)                                     # head
    d.polygon([(160, 150), (40, 185), (162, 190)], fill=BLACK)         # beak
    d.polygon([(540, 400), (660, 330), (620, 470), (540, 470)], fill=BLACK)  # tail
    # legs: rooted 24px into the body, landing 24px into the nest
    d.line([(380, 500), (370, 680)], fill=BLACK, width=26)
    d.line([(450, 500), (470, 680)], fill=BLACK, width=26)
    # the nest: a woven mound
    d.ellipse([220, 660, 620, 790], fill=BLACK)
    d.line([(180, 740), (660, 740)], fill=BLACK, width=44)
    return im


def caravel():
    """The Portuguese caravel: crescent hull with fore and stern castles,
    two billowing square sails and the lateen mizzen, every sail rooted
    to the hull by its 26px mast (welded, one contour per sail+mast)."""
    im, d = canvas(880, 800)
    # hull: crescent with raised castles
    d.polygon([(110, 560), (770, 560), (700, 700), (200, 700)], fill=BLACK)
    d.polygon([(110, 560), (60, 470), (150, 470), (190, 560)], fill=BLACK)   # forecastle
    d.polygon([(690, 560), (720, 480), (820, 480), (770, 560)], fill=BLACK)  # sterncastle
    # main mast + billowing mainsail (40px clear of the foresail)
    d.line([(410, 560), (410, 120)], fill=BLACK, width=26)
    d.polygon([(290, 160), (550, 160), (580, 300), (550, 420), (290, 420), (260, 290)],
              fill=BLACK)
    d.ellipse([280, 130, 560, 200], fill=BLACK)                        # yard curve
    # foresail (smaller, forward)
    d.line([(160, 560), (160, 260)], fill=BLACK, width=24)
    d.polygon([(85, 290), (220, 290), (235, 375), (220, 455), (85, 455), (70, 370)],
              fill=BLACK)
    # lateen mizzen aft: one clean triangle on its mast, well off the
    # sterncastle (the first cut's raked yard knife-edged against it)
    d.line([(660, 560), (660, 280)], fill=BLACK, width=24)
    d.polygon([(600, 400), (660, 250), (745, 420)], fill=BLACK)
    # pennant (micro) at the main masthead
    d.polygon([(400, 78), (470, 95), (400, 112)], fill=BLACK)
    return im


def bigben():
    """Elizabeth Tower: shaft, the wider clock stage with the face as a
    white hole, then the tiered spire. Window slits ride the shaft as
    micro holes."""
    im, d = canvas(620, 1000)
    d.rectangle([220, 380, 400, 880], fill=BLACK)                      # shaft
    d.rectangle([190, 260, 430, 400], fill=BLACK)                      # clock stage
    circle(d, 310, 330, 58, WHITE)                                     # the face
    d.polygon([(310, 330), (310, 288)], fill=BLACK)
    d.line([(310, 330), (310, 292)], fill=BLACK, width=10)             # hour hand
    d.line([(310, 330), (340, 318)], fill=BLACK, width=8)              # minute hand
    d.rectangle([200, 220, 420, 268], fill=BLACK)                      # cornice
    d.polygon([(200, 220), (420, 220), (390, 150), (230, 150)], fill=BLACK)  # roof base
    d.polygon([(230, 155), (390, 155), (310, 40)], fill=BLACK)         # spire
    d.rectangle([300, 14, 320, 46], fill=BLACK)                        # finial
    d.rectangle([180, 880, 440, 950], fill=BLACK)                      # base
    # window slits down the shaft (micro holes)
    for y in (460, 580, 700):
        d.rectangle([290, y, 330, y + 56], fill=WHITE)
    return im


SUBJECTS = {
    "calavera": calavera,
    "tajmahal": tajmahal,
    "stork": stork,
    "caravel": caravel,
    "bigben": bigben,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
