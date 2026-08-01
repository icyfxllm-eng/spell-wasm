#!/usr/bin/env python3
"""CC-PICTURE-BANK wave 1, batch 1 — the starter tier, drawn.

Eight new starter pictures (the ninth, snail, already ships). Drawn
originals in the draw_ref.py idiom: literal coordinate lists, solid subject
on white, features that must survive tracing are HOLES by construction —
the mug's handle, the ladybug's spots, the mushroom's flecks. Nothing
generative, provenance is "Original artwork (SpellGame)" by construction.

Starter means one session and a bold outline: each subject lands as 1-4
contours, which is exactly the depth the tier ladder wants at the bottom.

Run: python3 tools/draw_wave1.py   (writes content-pipeline/wordpic/ref/)
Then: ./target/release/suggest content-pipeline/wordpic/ref/<name>.png ...
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


def balloon():
    """Round balloon, tied neck, a wavy string. The knot triangle keeps the
    neck readable at starter stroke widths."""
    im, d = canvas(600, 860)
    d.ellipse([140, 80, 460, 480], fill=BLACK)                     # envelope
    d.polygon([(280, 460), (320, 460), (335, 520), (265, 520)], fill=BLACK)  # neck
    d.polygon([(270, 520), (330, 520), (300, 560)], fill=BLACK)    # knot
    pts = [(300 + 26 * math.copysign(1, math.cos(k)) * abs(math.cos(k)) ** 0.7, 560 + k * 36)
           for k in range(8)]
    d.line([(300, 560)] + [(int(x), int(y)) for x, y in pts], fill=BLACK, width=9)
    return im


def mug():
    """The handle is a HOLE — the single feature that makes a mug a mug and
    not a bucket. Eric (2026-08-01): lower the cup, steam off the top. The
    first cut skipped steam as "thin curls die at starter widths" — the fix
    is not to skip steam but to draw it THICK: 20px wisps hold their own
    contours and clear the corridor floor."""
    im, d = canvas(760, 900)
    d.rounded_rectangle([120, 420, 480, 830], radius=42, fill=BLACK)  # body, lowered
    d.ellipse([470, 480, 690, 720], fill=BLACK)                       # handle outer
    d.ellipse([520, 530, 640, 670], fill=WHITE)                       # handle hole
    # Three steam wisps: fat S-curves, gapped 30px above the rim so each
    # traces as its own clear candidate.
    for x0, amp, h0 in [(190, 30, 0), (295, 36, -40), (400, 30, 0)]:
        pts = [(x0, 390 + h0), (x0 - amp, 320 + h0), (x0 + amp, 220 + h0), (x0, 130 + h0)]
        d.line(pts, fill=BLACK, width=20, joint="curve")
    return im


def kite():
    """Diamond, spars as white creases wide enough to hold a word, and a
    tail with three bows — the bows give the starter its extra contours."""
    im, d = canvas(640, 900)
    d.polygon([(320, 60), (560, 300), (320, 560), (80, 300)], fill=BLACK)
    # Spars at 26px: the first cut used 13px and every candidate came back
    # flagged -- a crease narrower than the corridor floor puts its own two
    # sides inside one corridor, the same lesson the rhino's armour taught.
    # 34px: at 26 the QUADRANTS deduped as decorative across the spar
    # (scaled gap ~14.5px < the 16px dedup tolerance) and the kite blew the
    # 20% decorative-arc budget a second time. Same design, wider crease.
    d.line([(320, 60), (320, 560)], fill=WHITE, width=34)             # spar
    d.line([(80, 300), (560, 300)], fill=WHITE, width=34)             # spar
    # The tail starts 24px BELOW the tip: attached, the lower quadrants and
    # the tail pinch to zero clearance at the join and all three flag red.
    # A small visual gap reads fine and every contour clears the floor.
    # Tail as a 32px ribbon: at 16px the far side deduped as decorative and
    # the kite blew the 20% decorative-arc budget (solver law, not corridor).
    # Thick enough, BOTH sides host words and nothing is decoration.
    d.line([(316, 584), (262, 680), (330, 780)], fill=BLACK, width=32)
    for cx, cy in [(265, 682), (328, 778)]:
        d.polygon([(cx - 34, cy - 12), (cx + 34, cy + 12), (cx, cy)], fill=BLACK)
        d.polygon([(cx - 34, cy + 12), (cx + 34, cy - 12), (cx, cy)], fill=BLACK)
    return im


def mushroom():
    """Toadstool: cap with three flecks as holes, stout stem. The flecks
    are the starter's micro features, same trick as the snowman's eyes."""
    im, d = canvas(760, 720)
    d.pieslice([80, 60, 680, 560], 180, 360, fill=BLACK)              # cap
    d.rounded_rectangle([300, 300, 460, 640], radius=48, fill=BLACK)  # stem
    circle(d, 240, 210, 34, WHITE)
    circle(d, 400, 150, 40, WHITE)
    circle(d, 545, 230, 30, WHITE)
    return im


def sailboat():
    """Hull, mast as a white seam, two sails. Four honest contours."""
    im, d = canvas(800, 760)
    d.polygon([(120, 520), (680, 520), (600, 640), (200, 640)], fill=BLACK)  # hull
    d.polygon([(390, 80), (390, 490), (150, 490)], fill=BLACK)               # main
    d.polygon([(430, 140), (430, 490), (640, 490)], fill=BLACK)              # jib
    return im


def cactus():
    """Saguaro joined to its pot (Eric 2026-08-01: "connect the bottom of
    the cactus to the pot"), with six detached thorn triangles. Thorns are
    sized into the micro band (closed, arc < 130 on the 512 canvas): drawn
    always, filled, and NEVER hosting a word -- the snowman's-eyes machinery,
    which is exactly what "non word thorns" asks for."""
    im, d = canvas(680, 900)
    d.rounded_rectangle([280, 120, 400, 720], radius=60, fill=BLACK)   # trunk, into the pot
    d.rounded_rectangle([120, 250, 220, 450], radius=50, fill=BLACK)   # left arm rise
    d.rounded_rectangle([120, 380, 300, 470], radius=45, fill=BLACK)   # left arm join
    d.rounded_rectangle([460, 180, 560, 400], radius=50, fill=BLACK)   # right arm rise
    d.rounded_rectangle([380, 320, 560, 410], radius=45, fill=BLACK)   # right arm join
    d.polygon([(200, 700), (480, 700), (450, 860), (230, 860)], fill=BLACK)  # pot
    # Thorns: ~45px sides -> arc ≈ 80 on the 512 canvas, inside the micro
    # band; 10px off the silhouette so each stays its own contour.
    for tip, base1, base2 in [
        ((252, 200), (270, 168), (270, 232)),   # trunk left
        ((428, 288), (410, 256), (410, 320)),   # trunk right (clear of the arm join)
        ((252, 520), (270, 488), (270, 552)),   # trunk left low
        ((92, 300),  (110, 268), (110, 332)),   # left arm outer
        ((588, 240), (570, 208), (570, 272)),   # right arm outer
        ((428, 610), (410, 578), (410, 642)),   # trunk right low
    ]:
        d.polygon([tip, base1, base2], fill=BLACK)
    return im


def ladybug():
    """Dome with a head, centre seam as a white line, four spots as holes,
    two stub antennae. The spots are the earned detail."""
    im, d = canvas(760, 700)
    d.ellipse([120, 160, 640, 620], fill=BLACK)                        # dome
    circle(d, 380, 150, 85, BLACK)                                     # head
    # Seam widened 12->26px and spots enlarged: at 12px the facing dome
    # edges sat inside one keep-out and a path starved (hosted zero words).
    # Antennae dropped -- too small to host, pure starvation risk.
    d.line([(380, 245), (380, 620)], fill=WHITE, width=26)             # wing seam
    circle(d, 265, 330, 46, WHITE)
    circle(d, 495, 330, 46, WHITE)
    circle(d, 235, 490, 40, WHITE)
    circle(d, 525, 490, 40, WHITE)
    return im


def moonstar():
    """Crescent moon and a star — two separate subjects on one canvas, the
    starter that teaches 'a picture can be more than one thing'."""
    im, d = canvas(820, 700)
    circle(d, 330, 350, 260, BLACK)                                    # full disc
    circle(d, 440, 350, 235, WHITE)                                    # bite
    cx, cy, r1, r2 = 640, 200, 110, 44
    pts = []
    for k in range(10):
        r = r1 if k % 2 == 0 else r2
        a = -math.pi / 2 + k * math.pi / 5
        # coordinates only; math.cos here draws a reference image, it is
        # never inside any determinism-audited pipeline
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    d.polygon(pts, fill=BLACK)
    return im


SUBJECTS = {
    "balloon": balloon,
    "mug": mug,
    "kite": kite,
    "mushroom": mushroom,
    "sailboat": sailboat,
    "cactus": cactus,
    "ladybug": ladybug,
    "moonstar": moonstar,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
