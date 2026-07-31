#!/usr/bin/env python3
"""Hand-authored line-art references.

The standing lesson from the peacock fan, the Mona painting and Duerer's
woodcut: when the reference fights the tracer, change the reference. Two
subjects had no clean line art to reach for, so they are drawn here as
plain silhouettes -- solid subject, white holes for the features that must
survive (rhino eye, rocket window). Nothing generative: these are literal
coordinate lists, versioned like any other source file.

Run: python3 tools/draw_ref.py   (writes content-pipeline/wordpic/ref/*.png)
"""
import pathlib
from PIL import Image, ImageDraw

REF = pathlib.Path(__file__).resolve().parents[1] / "content-pipeline/wordpic/ref"
BLACK, WHITE = 0, 255


def _canvas(w, h):
    im = Image.new("L", (w, h), WHITE)
    return im, ImageDraw.Draw(im)


def rhino():
    """Side view facing right, head lowered the way a rhino carries it.
    Duerer's plate is the right animal and the wrong reference: its
    hatching is so dense that every threshold floods to a solid mass, and
    the legs vanish into the ground shadow (Eric: 'I wish you could see
    its legs and eyes'). Here the legs are drawn with real gaps between
    them and the eye is a hole, so both survive the trace by construction.
    """
    W, H = 980, 620
    im, d = _canvas(W, H)

    # Body, neck, head and horns as one closed outline, traced clockwise
    # from the rump. Kept deliberately blunt -- a rhino is a barrel.
    body = [
        (120, 300), (135, 250), (170, 214), (225, 192), (300, 180),
        (380, 176), (455, 180), (520, 192),               # back
        (565, 196), (600, 186), (640, 168), (676, 150),   # shoulder hump
        (706, 152), (726, 172), (734, 200),               # nape
        (752, 190), (774, 196), (786, 216), (784, 244),   # ear, solid
        (798, 268), (822, 300), (856, 320),               # brow to nose bridge
        (868, 322), (890, 288), (906, 250), (922, 258),   # FRONT HORN, the
        (926, 296), (916, 342), (898, 378),               # one thing that
        (908, 402), (904, 424), (884, 440), (856, 446),   # says "rhino"
        (826, 444), (802, 432), (786, 414),               # muzzle and jaw
        (726, 424), (700, 418), (680, 400), (668, 376),   # cheek / jowl
        (640, 372), (612, 380), (586, 396), (566, 408),   # throat
        (548, 420),
        # --- legs: four of them, with air in between ---
        (540, 470), (536, 520), (540, 556), (528, 566),   # front near leg
        (496, 566), (486, 552), (492, 516), (494, 470),
        (486, 448),
        (452, 452), (448, 500), (446, 544), (434, 556),   # front off leg
        (404, 556), (396, 542), (404, 500), (408, 452),
        (386, 444),
        (300, 448),
        (296, 496), (292, 540), (280, 552),               # hind off leg
        (250, 552), (242, 538), (250, 496), (254, 448),
        (232, 442),
        (206, 470), (202, 516), (206, 552), (194, 564),   # hind near leg
        (162, 564), (152, 550), (158, 512), (160, 464),
        (148, 430), (130, 386), (120, 340),
    ]
    d.polygon(body, fill=BLACK)

    # Second horn: rhinos have two, and the small one behind the big one
    # is most of what makes the head read as a rhino rather than a boar.
    # Broad at the base so the gap between the horns is a shallow V and
    # not a slot cut out of the head.
    d.polygon([(796, 276), (816, 234), (836, 226), (854, 248),
               (874, 300), (862, 318)], fill=BLACK)

    # Tail, hung well clear of the rump: drawn any closer, its corridor and
    # the body's overlap and the whole picture is refused.
    d.line([(120, 300), (86, 332), (78, 386), (92, 414)], fill=BLACK, width=13)
    d.polygon([(84, 406), (102, 406), (98, 440), (82, 434)], fill=BLACK)

    # Eye -- a hole, so it traces as its own closed contour. The ear stays
    # solid: hollowed out it read as a second eye.
    d.ellipse([762, 286, 794, 316], fill=WHITE)
    d.ellipse([770, 294, 786, 310], fill=BLACK)

    # No plate creases. A slot narrow enough to read as a crease puts its
    # own two sides inside one corridor width of each other, which the
    # layout law refuses -- and Eric asked for legs and eyes, not armour.
    return im


def horse():
    """Standing, head up (Eric: 'a pic of a horse where its head is
    upright'). The old reference was a galloping horse with its neck
    stretched flat out in front -- the picture read fine, it just wasn't
    the pose he asked for. Standing also buys clearance: a gallop throws
    the legs together, and legs that cross cannot both hold a word."""
    W, H = 880, 820
    im, d = _canvas(W, H)

    body = [
        (806, 264),                                        # muzzle
        (790, 244), (770, 212), (744, 176), (716, 140),    # up the face
        (694, 112),                                        # poll
        (692, 84), (684, 52), (668, 58), (668, 96),        # ear
        (656, 108), (640, 120),
        (636, 124), (600, 150), (556, 190), (516, 228),    # crest of the neck
        (486, 262), (462, 288),                            # withers
        (400, 296), (340, 300), (280, 302), (222, 296),    # back
        (180, 302), (156, 316),                            # croup, tail root
        (140, 340), (134, 372), (150, 400),                # rump
        # --- four legs, each standing clear of the next ---
        (156, 440), (162, 500), (158, 560), (150, 620),    # hind, off side
        (156, 680), (150, 740), (162, 756),
        (190, 756), (196, 740), (188, 680), (196, 620),
        (200, 560), (206, 500), (216, 452),
        (252, 448),                                        # hind, near side
        (258, 500), (254, 560), (246, 620), (252, 680),
        (246, 740), (258, 756),
        (286, 756), (292, 740), (284, 680), (292, 620),
        (296, 560), (300, 500), (308, 452),
        (360, 470), (420, 476), (452, 470),                # belly
        (458, 520), (452, 580), (446, 640), (452, 700),    # fore, off side
        (446, 748), (458, 762),
        (486, 762), (492, 748), (486, 700), (492, 640),
        (498, 580), (504, 520), (508, 470),
        (548, 466),                                        # fore, near side
        (554, 520), (548, 580), (542, 640), (548, 700),
        (542, 748), (554, 762),
        (582, 762), (588, 748), (582, 700), (588, 640),
        (594, 580), (598, 520), (600, 460),
        (608, 420), (618, 386), (634, 352),                # chest
        (656, 314), (672, 276), (680, 244),                # throat
        (700, 250), (722, 264), (746, 282),                # jaw and cheek
        (770, 296), (792, 298), (806, 288), (812, 276),    # blunt muzzle
    ]
    d.polygon(body, fill=BLACK)

    # Tail, swung well clear of the hind legs for the same reason the
    # rhino's is: a tail drawn against the leg shares its corridor.
    d.polygon([(160, 306), (118, 336), (86, 400), (72, 490), (76, 570),
               (96, 630), (126, 640), (128, 596), (112, 540), (110, 470),
               (124, 400), (150, 348)], fill=BLACK)

    # Eye as a hole.
    d.ellipse([702, 178, 730, 206], fill=WHITE)
    d.ellipse([709, 185, 723, 199], fill=BLACK)
    return im


def rocket():
    """The CC0 pictogram is split by a white seam down its middle, so
    largest-component kept one half and the picture came out as a
    fragment (Eric: 'it doesn't look right'). One body, no seam."""
    W, H = 620, 900
    im, d = _canvas(W, H)

    d.polygon([                                    # nose, body, fins, tail
        (310, 70),
        (356, 138), (386, 216), (402, 300), (408, 392),   # right flank
        (470, 470), (498, 556), (500, 640),               # right fin
        (408, 520),                                       # fin root, set
        (404, 660), (392, 712),                           # high: rooted by
        (228, 712), (216, 660), (212, 520),               # the skirt it
        (120, 640), (122, 556), (150, 470), (212, 392),   # pinches shut
        (218, 300), (234, 216), (264, 138),
    ], fill=BLACK)

    # Window: a hole, so it survives as its own contour.
    d.ellipse([254, 250, 366, 362], fill=WHITE)
    d.ellipse([272, 268, 348, 344], fill=BLACK)

    # Exhaust: three flames, each its own shape, each big enough to host a
    # word rather than being dropped by the arc floor.
    d.polygon([(228, 736), (262, 736), (250, 806), (232, 786)], fill=BLACK)
    d.polygon([(276, 736), (344, 736), (322, 846), (310, 800), (286, 830)], fill=BLACK)
    d.polygon([(358, 736), (392, 736), (388, 786), (370, 806)], fill=BLACK)
    return im


if __name__ == "__main__":
    for name, fn in (("rhino", rhino), ("rocket", rocket), ("horse", horse)):
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
