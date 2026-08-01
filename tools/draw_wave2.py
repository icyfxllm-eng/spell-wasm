#!/usr/bin/env python3
"""CC-PICTURE-BANK wave 1, batch 2 — the intermediate tier, drawn.

Nine subjects with a real FEATURES layer: interior holes (the violin's
f-holes, the lighthouse's windows), micro features (eyes, hub dots), and
multi-part structure a starter silhouette doesn't have. Same idiom as
batch 1: literal coordinate lists, solid black on white, features that
must survive are holes by construction, provenance "Original artwork
(SpellGame)" by construction.

Field rules learned in batch 1, applied from the start this time:
- white creases/seams >= 34px, or the divided parts dedupe as decorative;
- detached micro bits sized into the (40, 130) arc band on the 512 canvas
  (closed + arc < 130 -> micro: drawn always, never hosting a word);
- nothing thin attached to something big (starves); gap it or fatten it.

Run: python3 tools/draw_wave2.py
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


def capsule(d, x0, y0, x1, y1, fill):
    """Rounded slot from primitives. PIL's rounded_rectangle left a 1px seam
    of the UNDERLYING colour down the middle of every trex rib slot (found
    by dumping raw pixels: '......#......'), which split each slot into two
    slivers and doubled every boundary. Two circles and a rectangle cannot
    have a seam."""
    r = (x1 - x0) // 2
    circle(d, (x0 + x1) // 2, y0 + r, r, fill)
    circle(d, (x0 + x1) // 2, y1 - r, r, fill)
    d.rectangle([x0, y0 + r, x1, y1 - r], fill=fill)


def poly_ring(d, cx, cy, r_out, r_in, fill=BLACK):
    circle(d, cx, cy, r_out, fill)
    circle(d, cx, cy, r_in, WHITE)


def smooth_poly(d, keys, fill=BLACK, samples=14):
    """Closed Catmull-Rom through the key points -> one organic polygon.
    Eric failed the rooster/seahorse/trex drawn from stacked ellipses --
    blobs read as blobs. A spline through hand-placed keys is still a
    literal coordinate list, just one with a spine."""
    n = len(keys)
    pts = []
    for i in range(n):
        p0, p1, p2, p3 = keys[(i - 1) % n], keys[i], keys[(i + 1) % n], keys[(i + 2) % n]
        for k in range(samples):
            t = k / samples
            t2, t3 = t * t, t * t * t
            x = 0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3)
            y = 0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)
            pts.append((x, y))
    d.polygon(pts, fill=fill)


def tapered_stroke(d, keys, w0, w1, fill=BLACK, samples=60):
    """Circles along a Catmull-Rom path, radius easing w0->w1: a tapering
    organic limb or tail, impossible with fixed-width line()."""
    n = len(keys)
    for i in range(n - 1):
        p0 = keys[max(i - 1, 0)]
        p1, p2 = keys[i], keys[i + 1]
        p3 = keys[min(i + 2, n - 1)]
        for k in range(samples // (n - 1) + 1):
            t = k / (samples // (n - 1) + 1)
            t2, t3 = t * t, t * t * t
            x = 0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3)
            y = 0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)
            g = (i + t) / (n - 1)
            r = w0 + (w1 - w0) * g
            circle(d, x, y, r, fill)


def bicycle():
    """Two wheel rings, frame as thick bars, seat and bars. The wheels are
    rings (hole = the inside), the classic two-triangle frame reads through
    the white creases between tubes."""
    im, d = canvas(980, 640)
    poly_ring(d, 230, 430, 170, 118)                       # rear wheel
    poly_ring(d, 750, 430, 170, 118)                       # front wheel
    circle(d, 230, 430, 26, BLACK)                          # rear hub (micro)
    circle(d, 750, 430, 26, BLACK)                          # front hub (micro)
    d.line([(230, 430), (450, 430)], fill=BLACK, width=34)  # chainstay
    d.line([(450, 430), (390, 200)], fill=BLACK, width=34)  # seat tube
    d.line([(230, 430), (390, 200)], fill=BLACK, width=34)  # seat stay
    d.line([(450, 430), (640, 210)], fill=BLACK, width=34)  # down tube
    d.line([(640, 210), (750, 430)], fill=BLACK, width=34)  # fork
    d.line([(390, 200), (640, 210)], fill=BLACK, width=34)  # top tube
    d.line([(340, 160), (440, 160)], fill=BLACK, width=30)  # seat
    d.line([(390, 200), (390, 168)], fill=BLACK, width=24)
    d.line([(640, 210), (620, 130)], fill=BLACK, width=24)  # stem
    d.line([(580, 118), (680, 128)], fill=BLACK, width=30)  # handlebars
    return im


def windmill():
    """Tapered tower, cap, four blades set diagonally so none collides
    with the tower, and a little door hole."""
    im, d = canvas(760, 900)
    d.polygon([(310, 260), (450, 260), (500, 850), (260, 850)], fill=BLACK)  # tower
    d.polygon([(290, 260), (470, 260), (380, 160)], fill=BLACK)              # cap
    d.rounded_rectangle([345, 700, 415, 850], radius=30, fill=WHITE)         # door hole
    # four blades from the hub, X arrangement; each a long trapezoid
    # Blades START inside the hub: the whole mill is ONE contour, so no
    # blade-to-blade or blade-to-cap corridor exists to flag.
    hub = (380, 235)
    for ang in (45, 135, 225, 315):
        a = math.radians(ang)
        dx, dy = math.cos(a), math.sin(a)
        px, py = -dy, dx
        tip = (hub[0] + dx * 300, hub[1] + dy * 300)
        base = (hub[0] + dx * 20, hub[1] + dy * 20)
        d.polygon([
            (base[0] + px * 16, base[1] + py * 16),
            (tip[0] + px * 52, tip[1] + py * 52),
            (tip[0] - px * 6, tip[1] - py * 6),
            (base[0] - px * 16, base[1] - py * 16),
        ], fill=BLACK)
    circle(d, *hub, 44, BLACK)
    return im


def hot_air_balloon():
    """Envelope with two gore seams (wide creases), basket joined by a
    white-gapped band, and the basket weave as a hole."""
    im, d = canvas(720, 940)
    d.ellipse([110, 60, 610, 620], fill=BLACK)                    # envelope
    d.polygon([(240, 540), (480, 540), (420, 700), (300, 700)], fill=BLACK)  # throat
    # Seams cross the WHOLE envelope (kite lesson): a seam that stops short
    # leaves a thin slot whose own walls flag; a full split makes three
    # separate gores with honest 36px corridors.
    d.line([(280, 40), (330, 720)], fill=WHITE, width=36)         # gore seam
    d.line([(440, 40), (390, 720)], fill=WHITE, width=36)         # gore seam
    d.rounded_rectangle([280, 740, 440, 870], radius=28, fill=BLACK)  # basket
    d.rounded_rectangle([310, 770, 410, 840], radius=18, fill=WHITE)  # weave hole
    return im


def lighthouse():
    """Striped tower — stripes as WIDE white bands so each black band hosts
    its own words — gallery, lamp room with a light hole, rock base."""
    im, d = canvas(700, 920)
    d.polygon([(280, 200), (420, 200), (470, 760), (230, 760)], fill=BLACK)  # tower
    d.line([(255, 380), (445, 380)], fill=WHITE, width=40)                   # stripe
    d.line([(245, 560), (455, 560)], fill=WHITE, width=40)                   # stripe
    # Lamp room floats 34px above the tower: the old 26px gallery bar sat
    # 2px off the tower top and flagged. The gallery is now the lamp room's
    # own wider base.
    d.rounded_rectangle([240, 90, 460, 166], radius=20, fill=BLACK)          # lamp room + gallery
    d.ellipse([310, 104, 390, 154], fill=WHITE)                              # the light
    d.polygon([(180, 760), (520, 760), (560, 870), (140, 870)], fill=BLACK)  # rock base
    return im


def violin():
    """The body with both f-holes, a waist, neck and scroll. The f-holes
    are the whole reason a violin reads as a violin."""
    im, d = canvas(640, 960)
    # body: two lobes + waist
    d.ellipse([150, 480, 490, 900], fill=BLACK)                   # lower bout
    d.ellipse([180, 330, 460, 620], fill=BLACK)                   # upper bout
    # waist nicks
    d.ellipse([100, 430, 200, 560], fill=WHITE)
    d.ellipse([440, 430, 540, 560], fill=WHITE)
    d.rounded_rectangle([290, 120, 350, 420], radius=24, fill=BLACK)  # neck
    circle(d, 320, 95, 42, BLACK)                                  # scroll
    # f-holes: tall S-ish slots, drawn as fat white capsules
    d.rounded_rectangle([228, 560, 262, 700], radius=17, fill=WHITE)
    d.rounded_rectangle([378, 560, 412, 700], radius=17, fill=WHITE)
    return im


def rooster():
    """v2 (Eric failed v1 -- stacked ellipses read as a blob). Side profile
    with a spined outline: upright chest, saddle dipping to a big three-
    feather tail sweep, head with ATTACHED comb, hanging wattle, one eye
    hole, two strong legs."""
    im, d = canvas(900, 940)
    smooth_poly(d, [
        (560, 240),            # crown behind comb
        (620, 300), (640, 360),        # nape
        (600, 430), (560, 500),        # back slopes down-left? no: chest right
        (610, 590), (560, 700),        # breast
        (430, 760), (330, 740),        # belly
        (250, 660), (230, 560),        # stern
        (300, 480), (280, 400),        # saddle dip to tail root
        (360, 330), (470, 250),        # neck front
    ])
    # tail: three tapered feathers sweeping up-left from the stern
    tapered_stroke(d, [(300, 520), (170, 420), (90, 250)], 26, 9)
    tapered_stroke(d, [(300, 560), (130, 520), (50, 400)], 24, 8)
    tapered_stroke(d, [(310, 600), (150, 620), (60, 560)], 22, 8)
    # comb: three bumps ATTACHED to the crown
    for cx, cy in [(520, 205), (565, 185), (610, 205)]:
        circle(d, cx, cy, 34, BLACK)
    d.polygon([(640, 330), (740, 360), (640, 392)], fill=BLACK)   # beak
    tapered_stroke(d, [(640, 400), (630, 470)], 24, 12)           # wattle
    circle(d, 590, 310, 30, WHITE)                                # eye hole
    circle(d, 590, 310, 13, BLACK)                                # pupil (micro)
    # legs: thick, with feet
    d.line([(430, 750), (430, 860)], fill=BLACK, width=32)
    d.line([(340, 745), (340, 860)], fill=BLACK, width=32)
    d.line([(390, 868), (480, 868)], fill=BLACK, width=26)
    d.line([(295, 868), (385, 868)], fill=BLACK, width=26)
    return im


def seahorse():
    """v2 (Eric failed the carved-ellipse v1). One tapered spine: head at a
    right angle, chest out, S through the belly, spiral tail -- drawn as a
    single tapering stroke so the body flows instead of lumping. Snout,
    coronet, dorsal fin, eye."""
    im, d = canvas(680, 960)
    tapered_stroke(d, [
        (390, 190),                    # crown
        (450, 250), (440, 340),        # head to nape
        (350, 420), (300, 520),        # chest curve
        (320, 640), (400, 720),        # belly swing
        (430, 800), (380, 860),        # tail drop
        (300, 870), (260, 820),        # spiral out
        (290, 770), (340, 790),        # spiral in
    ], 58, 8, samples=160)
    tapered_stroke(d, [(400, 210), (300, 230), (210, 250)], 26, 10)  # snout
    d.polygon([(360, 130), (420, 60), (440, 160)], fill=BLACK)       # coronet
    tapered_stroke(d, [(470, 380), (560, 430), (520, 540), (450, 560)], 16, 12)  # dorsal fin
    circle(d, 395, 265, 28, WHITE)                                   # eye
    circle(d, 395, 265, 12, BLACK)                                   # pupil (micro)
    return im


def trex_skeleton():
    """v2 (Eric failed the lumpy v1). A proper museum mount: horizontal
    posture, huge skull with hinged jaw, arched spine over a deep ribcage,
    z-bent hind legs, long tapering tail. One connected mass; detail in
    the holes (eye socket, jaw gap, four rib slots); the tiny arm micro."""
    im, d = canvas(1060, 720)
    # skull: deep box with a brow step, hinged jaw below
    smooth_poly(d, [
        (760, 130), (870, 120), (960, 150), (1000, 210),
        (985, 265), (900, 285), (790, 275), (740, 220),
    ], samples=10)
    d.polygon([(985, 262), (1000, 300), (900, 345), (790, 330), (770, 285), (900, 292)], fill=BLACK)  # jaw, hinged right
    d.polygon([(778, 288), (940, 296), (782, 312)], fill=WHITE)     # jaw gap
    circle(d, 880, 200, 30, WHITE)                                  # eye socket
    circle(d, 795, 185, 22, WHITE)                                  # antorbital hole
    # neck arching down to the spine
    tapered_stroke(d, [(760, 180), (660, 230), (580, 290)], 34, 26)
    # spine arc over the hips
    tapered_stroke(d, [(580, 290), (460, 310), (330, 330)], 26, 22)
    # ribcage: deep ellipse hanging from the spine
    # Deep cage: at 520 the ellipse curved up to within 2px of the slot
    # bottoms at the outer slots -- another knife edge, this time from
    # geometry curving INTO the straight slots. 60px of floor under every
    # slot now.
    d.ellipse([380, 280, 660, 560], fill=BLACK)
    # Slots start below the spine stroke (the v1 knife-edge, relearned at a
    # different y) and the gaps widened past the corridor floor at this
    # canvas's scale (0.477: 30px ref was a 14.3px corridor -- a coin flip).
    for x in (425, 497, 569):                                       # rib slots
        capsule(d, x, 356, x + 28, 470, WHITE)
    # hip mass, big enough to swallow the leg junctions -- the v2 hip left
    # an enclosed white pocket between hip, leg and ribcage, which traced as
    # an accidental word-hosting hole.
    circle(d, 335, 375, 85, BLACK)
    d.polygon([(265, 310), (430, 335), (430, 515), (280, 510)], fill=BLACK)  # weld hip to cage: no pocket
    tapered_stroke(d, [(330, 380), (390, 480), (330, 570), (380, 660)], 34, 18)
    d.line([(350, 668), (470, 668)], fill=BLACK, width=26)          # foot
    tapered_stroke(d, [(300, 390), (260, 500), (300, 600)], 26, 14) # far leg
    # tail: long taper out the back
    tapered_stroke(d, [(300, 330), (170, 340), (60, 300), (20, 250)], 26, 6)
    # the famous tiny arm (detached, micro)
    d.line([(610, 350), (655, 390)], fill=BLACK, width=22)
    return im


def hummingbird():
    """Hummingbird at a flower: body, swept wing, needle beak reaching a
    detached bloom with petal holes."""
    im, d = canvas(940, 760)
    d.ellipse([320, 260, 600, 500], fill=BLACK)                    # body
    d.ellipse([500, 200, 660, 360], fill=BLACK)                    # head
    # Tail rooted INSIDE the body ellipse -- rooted 15px outside it, the
    # tail floated as its own flagged spear.
    d.line([(380, 420), (210, 590)], fill=BLACK, width=46)         # tail
    d.polygon([(240, 540), (150, 650), (210, 630)], fill=BLACK)    # tail tip fork
    # wing swept up-back
    d.polygon([(430, 300), (250, 90), (330, 80), (520, 260)], fill=BLACK)
    circle(d, 590, 260, 26, WHITE)                                 # eye
    circle(d, 590, 260, 11, BLACK)                                 # pupil (micro)
    # Beak stops 45px short of the bloom: reaching, not touching -- touching
    # merged bird and flower into one red tangle.
    d.line([(650, 300), (760, 340)], fill=BLACK, width=26)         # beak
    circle(d, 860, 380, 70, BLACK)                                 # bloom
    circle(d, 860, 380, 30, WHITE)                                 # center hole
    d.line([(860, 448), (860, 620)], fill=BLACK, width=30)         # stem (joined)
    d.ellipse([780, 590, 866, 660], fill=BLACK)                    # leaf (joined)
    return im


SUBJECTS = {
    "bicycle": bicycle,
    "windmill": windmill,
    "hotair": hot_air_balloon,
    "lighthouse": lighthouse,
    "violin": violin,
    "rooster": rooster,
    "seahorse": seahorse,
    "trex": trex_skeleton,
    "hummingbird": hummingbird,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
