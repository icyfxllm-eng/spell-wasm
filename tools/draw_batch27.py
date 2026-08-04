#!/usr/bin/env python3
"""Sky + masterpiece expansion (Eric, 2026-08-04: "carry on with the sky
and masterpiece expansion"). Seven ORIGINALS in the weld-first idiom —
overlaps >=28px, white cuts enter from the silhouette edge, detached
pieces >=30px apart, micro discs r>=15 — plus D2 ladder siblings (wave,
sunflower) drawn as their own subjects. The three PD masterpieces ride
the trace pipeline separately (never traced from unlicensed images —
these are Wikimedia PD-Art scans with provenance recorded).
"""
import math
import pathlib

from PIL import Image, ImageDraw

HERE = pathlib.Path(__file__).parent
REF = HERE / "ref"
BLACK, WHITE = 0, 255


def canvas(w, h):
    im = Image.new("L", (w, h), WHITE)
    return im, ImageDraw.Draw(im)


def circle(d, cx, cy, r, fill):
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=fill)


def sun():
    """Solid disc, eight welded TRAPEZOID rays (bases buried 40px into the
    disc), a face hole trio: two eye discs + smile arc, all hostable.
    Rays are blunt on purpose (L9): a pointed ray's outline is two sides
    converging to zero gap, and the facing words collide near the tip —
    the trapezoid keeps the sides ~90px apart end to end."""
    im, d = canvas(900, 900)
    cx, cy, r = 450, 450, 210
    for i in range(8):
        a = math.pi / 4 * i
        base1 = (cx + (r - 40) * math.cos(a - 0.26), cy + (r - 40) * math.sin(a - 0.26))
        base2 = (cx + (r - 40) * math.cos(a + 0.26), cy + (r - 40) * math.sin(a + 0.26))
        tip2 = (cx + (r + 185) * math.cos(a + 0.115), cy + (r + 185) * math.sin(a + 0.115))
        tip1 = (cx + (r + 185) * math.cos(a - 0.115), cy + (r + 185) * math.sin(a - 0.115))
        d.polygon([base1, base2, tip2, tip1], fill=BLACK)
    circle(d, cx, cy, r, BLACK)
    # face holes (hostable): eyes + smile
    circle(d, cx - 75, cy - 50, 34, WHITE)
    circle(d, cx + 75, cy - 50, 34, WHITE)
    d.arc([cx - 110, cy - 60, cx + 110, cy + 120], 25, 155, fill=WHITE, width=34)
    return im


def rainbow():
    """Three nested arc bands landing in two cloud masses — every band
    WELDED into both clouds (no floating arcs), the white gaps between
    bands open at both ends by construction. Gaps widened to ~58px (L9):
    facing band-edge words collided across the old ~26px gap."""
    im, d = canvas(940, 720)
    cx, base = 470, 640
    for i, r in enumerate([428, 320, 212]):
        d.arc([cx - r, base - r, cx + r, base + r], 180, 360, fill=BLACK, width=50)
    # cloud anchors swallowing all band ends (weld)
    for ccx in (cx - 330, cx + 330):
        for dx, dy, r in [(-60, 0, 70), (30, -28, 82), (95, 8, 62), (10, 34, 74)]:
            circle(d, ccx + dx, base - 40 + dy, r, BLACK)
    return im


def comet():
    """Head disc with ONE fat swept trapezoid tail (blunt end), welded;
    two micro star discs riding clear of the tail. Channel law (L9): the
    old three skinny lobes tapered to points — converging outline sides
    host colliding words."""
    im, d = canvas(940, 760)
    hx, hy, hr = 700, 250, 120
    circle(d, hx, hy, hr, BLACK)
    d.polygon([
        (hx - hr + 34, hy - 120),
        (hx - hr + 34, hy + 120),
        (hx - 620, hy + 235),
        (hx - 620, hy + 105),
    ], fill=BLACK)
    circle(d, 180, 130, 26, BLACK)   # micro star
    circle(d, 320, 620, 26, BLACK)   # micro star
    return im


def saturn():
    """The ringed planet: sphere + one continuous ring band crossing in
    front (the behind-half is a white cut that enters from both
    silhouette edges — open by construction)."""
    im, d = canvas(940, 760)
    cx, cy, r = 470, 380, 200
    circle(d, cx, cy, r, BLACK)
    # ring: fat ellipse band, then hollow it
    d.ellipse([cx - 430, cy - 120, cx + 430, cy + 150], fill=BLACK)
    d.ellipse([cx - 350, cy - 74, cx + 350, cy + 104], fill=WHITE)
    # restore the planet inside the hollow
    circle(d, cx, cy, r, BLACK)
    # carve the ring's BEHIND half back out (white, open at both edges)
    d.ellipse([cx - 430, cy - 132, cx + 430, cy + 8], fill=WHITE)
    d.ellipse([cx - 430, cy - 120 + 8, cx + 430, cy + 150 + 8], outline=None)
    # re-lay the planet's upper half over the carve
    d.pieslice([cx - r, cy - r, cx + r, cy + r], 180, 360, fill=BLACK)
    # and the FRONT ring arc over the planet's lower half
    d.arc([cx - 430, cy - 120, cx + 430, cy + 150], 12, 168, fill=BLACK, width=52)
    return im


def raincloud():
    """Cloud mass (five merged discs + base slab) with four fat raindrops
    below — detached pieces >=30px apart (dartboard rule), each drop a
    teardrop big enough to host."""
    im, d = canvas(920, 860)
    for dx, dy, r in [(-230, 20, 110), (-90, -60, 140), (80, -70, 150), (230, 10, 115), (0, 40, 150)]:
        circle(d, 460 + dx, 260 + dy, r, BLACK)
    d.rectangle([180, 280, 740, 380], fill=BLACK)
    for i, dx in enumerate([-210, -70, 70, 210]):
        x = 460 + dx
        y = 520 + (60 if i % 2 else 0)
        d.polygon([(x, y - 90), (x - 52, y + 10), (x + 52, y + 10)], fill=BLACK)
        circle(d, x, y + 30, 58, BLACK)
    return im


def wave():
    """The D2 ladder sibling for the Great Wave: an ORIGINAL curling
    breaker — big C-curl, three claw crests, foam base — one welded mass,
    the curl's white spiral entering from the open mouth of the wave."""
    im, d = canvas(940, 800)
    # base swell
    d.polygon([(30, 700), (60, 560), (200, 470), (420, 430), (700, 450), (910, 560), (910, 700)], fill=BLACK)
    # the curl: thick C sweeping up and over
    d.pieslice([180, 60, 780, 660], 130, 355, fill=BLACK)
    d.pieslice([300, 180, 660, 540], 130, 355, fill=WHITE)  # open spiral cut
    # claw crests welded onto the curl's lip
    for (x, y, a) in [(268, 240, -35), (350, 150, -20), (470, 100, -5)]:
        d.polygon([(x, y), (x + 120, y - 18), (x + 34, y + 78)], fill=BLACK)
    # foam toes welded to the base
    for dx in (-260, -60, 160, 330):
        circle(d, 470 + dx, 560, 66, BLACK)
    return im


def sunflower():
    """The ladder sibling for Sunflowers: one bloom — solid seed disc,
    SIX fat blunt petals, widened seed spiral, stem + one blunt leaf.
    Channel law (L9): 16 needle petals left ~7px gaps at the bases and
    tapered to points — every outline channel hosted colliding words."""
    im, d = canvas(880, 920)
    cx, cy, r = 440, 330, 130
    for i in range(6):
        a = math.tau / 6 * i + 0.26
        b1 = (cx + (r - 30) * math.cos(a - 0.32), cy + (r - 30) * math.sin(a - 0.32))
        b2 = (cx + (r - 30) * math.cos(a + 0.32), cy + (r - 30) * math.sin(a + 0.32))
        t1 = (cx + (r + 140) * math.cos(a - 0.14), cy + (r + 140) * math.sin(a - 0.14))
        t2 = (cx + (r + 140) * math.cos(a + 0.14), cy + (r + 140) * math.sin(a + 0.14))
        d.polygon([b1, t1, t2, b2], fill=BLACK)
    circle(d, cx, cy, r, BLACK)
    d.arc([cx - 74, cy - 74, cx + 74, cy + 74], 20, 300, fill=WHITE, width=42)
    d.rectangle([cx - 26, cy + r - 34, cx + 26, 860], fill=BLACK)
    d.polygon([(cx + 20, 640), (cx + 190, 555), (cx + 210, 615), (cx + 30, 700)], fill=BLACK)
    return im


SUBJECTS = {
    "sun": sun, "rainbow": rainbow, "comet": comet, "saturn": saturn,
    "raincloud": raincloud, "wave": wave, "sunflower": sunflower,
}
for name, fn in SUBJECTS.items():
    im = fn()
    im.save(REF / f"{name}.png")
    print(f"drew {name} {im.size}")
