#!/usr/bin/env python3
"""Wave 10, the drawn three of Eric's "strongest five" (2026-08-01):

Mt. Fuji (ja — the spec's wave-1 partner to the shipped torii), the
Orion constellation map (PICTURE-BANK Done #6 references completing
"layer 1 of the constellation map" — the picture finally exists), and
papel picado (es — the hole machinery's showcase banner). Red Fuji and
The Scream ride the sourced pipeline instead (prep_wave10.py).

Field rules as ever: seams >= 34px, micro band (40, 130) at 512,
detached micro >= 10px off the silhouette.

Run: python3 tools/draw_wave10.py
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


def fuji():
    """The mountain and its snowcap as TWO contours across an aligned
    zigzag seam (the wolf-perch answer): fused they would be one dull
    cone; the aligned 36px gap reads as the snow line."""
    im, d = canvas(960, 700)
    # the body: broad concave-sided cone, cut below the snow line
    d.polygon([(480, 226), (388, 250), (300, 330), (206, 450), (60, 620),
               (900, 620), (754, 450), (660, 330), (572, 250)], fill=BLACK)
    # carve the zigzag snow seam into the body's top edge
    zig = [(388, 250), (420, 296), (452, 252), (484, 300), (516, 254), (548, 298), (572, 250)]
    d.polygon(zig + [(572, 214), (388, 214)], fill=WHITE)
    # the snowcap: same zigzag displaced 36px up, closed over the summit
    # 56px vertical: the zigzag runs steep, so the PERPENDICULAR gap
    # at 36px was only ~11px at scan scale
    cap = [(x, y - 56) for (x, y) in zig]
    d.polygon(cap + [(540, 130), (480, 110), (420, 130)], fill=BLACK)
    return im


def orion():
    """The hunter as a star chart: seven principal stars as detached
    micro dots, the classic stick figure as 26px chart lines that stop
    12px short of every star — exactly how the charts draw it."""
    im, d = canvas(860, 1000)
    stars = {
        "betelgeuse": (300, 180), "bellatrix": (600, 220),
        "alnitak": (390, 500), "alnilam": (450, 480), "mintaka": (510, 460),
        "saiph": (330, 820), "rigel": (620, 780),
    }
    # no belt-to-belt links: with the 34px setbacks those 60px hops
    # collapsed into flagged nubs — the three dots in a row ARE the belt
    links = [("betelgeuse", "alnitak"), ("bellatrix", "mintaka"),
             ("alnitak", "saiph"), ("mintaka", "rigel"),
             ("betelgeuse", "bellatrix")]
    for a, b in links:
        (x0, y0), (x1, y1) = stars[a], stars[b]
        L = math.hypot(x1 - x0, y1 - y0)
        # stop each end 52px short of the star's centre — at 34 the two
        # lines converging on a shared star pinched EACH OTHER at 10px
        t0, t1 = 52 / L, 1 - 52 / L
        d.line([(x0 + (x1 - x0) * t0, y0 + (y1 - y0) * t0),
                (x0 + (x1 - x0) * t1, y0 + (y1 - y0) * t1)], fill=BLACK, width=26)
    for name, (x, y) in stars.items():
        r = 20 if name in ("betelgeuse", "rigel") else 15
        circle(d, x, y, r, BLACK)
    # the sword: two small stars hanging under the belt
    circle(d, 452, 580, 12, BLACK)
    circle(d, 460, 650, 12, BLACK)
    return im


def papelpicado():
    """The fiesta banner: one rectangle full of cut-outs — a flower ring
    big enough to host words, diamond and leaf micro holes, a scalloped
    hem — hanging from its string."""
    im, d = canvas(1000, 760)
    d.line([(30, 90), (970, 90)], fill=BLACK, width=22)               # the string
    d.rectangle([170, 80, 830, 600], fill=BLACK)                      # the banner
    # scalloped hem: bites along the bottom edge
    for k in range(6):
        cx = 225 + k * 110
        circle(d, cx, 600, 46, WHITE)
    # the flower: a big scalloped ring (hostable) with its heart dot
    cx, cy, r = 500, 320, 118
    circle(d, cx, cy, r, WHITE)
    for k in range(8):
        a = k * math.pi / 4
        circle(d, cx + math.cos(a) * r, cy + math.sin(a) * r, 46, WHITE)
    circle(d, cx, cy, 34, BLACK)
    # corner diamonds and leaves (micro holes)
    for (hx, hy) in [(265, 190), (735, 190), (265, 470), (735, 470)]:
        d.polygon([(hx, hy - 34), (hx + 26, hy), (hx, hy + 34), (hx - 26, hy)], fill=WHITE)
    for (hx, hy) in [(380, 160), (620, 160)]:
        d.ellipse([hx - 34, hy - 20, hx + 34, hy + 20], fill=WHITE)
    return im


SUBJECTS = {
    "fuji": fuji,
    "orion": orion,
    "papelpicado": papelpicado,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
