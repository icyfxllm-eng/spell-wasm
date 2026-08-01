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


def poly_ring(d, cx, cy, r_out, r_in, fill=BLACK):
    circle(d, cx, cy, r_out, fill)
    circle(d, cx, cy, r_in, WHITE)


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
    """Body, tail sweep, comb and wattle as detached micro bits, beak,
    an eye hole, and two thick legs."""
    im, d = canvas(880, 900)
    d.ellipse([200, 300, 640, 700], fill=BLACK)                    # body
    d.ellipse([460, 180, 660, 420], fill=BLACK)                    # head+neck blend
    # tail: three sweeping fat arcs
    d.line([(240, 460), (110, 300), (150, 170)], fill=BLACK, width=44, joint="curve")
    d.line([(250, 520), (90, 430), (60, 300)], fill=BLACK, width=40, joint="curve")
    d.polygon([(640, 300), (730, 330), (640, 360)], fill=BLACK)    # beak
    circle(d, 590, 280, 30, WHITE)                                 # eye hole
    circle(d, 590, 280, 13, BLACK)                                 # pupil (micro)
    # comb: detached bumps above the head (micro band)
    for cx in (520, 566, 612):
        circle(d, cx, 132, 26, BLACK)
    circle(d, 648, 460, 30, BLACK)                                 # wattle (micro)
    d.line([(380, 690), (380, 820)], fill=BLACK, width=30)         # legs
    d.line([(470, 690), (470, 820)], fill=BLACK, width=30)
    d.line([(340, 830), (420, 830)], fill=BLACK, width=24)         # feet
    d.line([(430, 830), (510, 830)], fill=BLACK, width=24)
    return im


def seahorse():
    """Curled S body, long snout, a crown, belly ridges as wide white
    notches, an eye hole, curled tail."""
    im, d = canvas(640, 920)
    d.ellipse([200, 200, 460, 560], fill=BLACK)                    # torso
    d.ellipse([230, 440, 470, 760], fill=BLACK)                    # lower curl
    d.ellipse([320, 560, 560, 800], fill=WHITE)                    # carve the curl
    d.ellipse([280, 640, 440, 820], fill=BLACK)                    # tail mass
    d.ellipse([250, 700, 380, 830], fill=WHITE)                    # carve tail spiral
    circle(d, 380, 740, 52, BLACK)                                 # tail tip curl
    d.rounded_rectangle([160, 220, 330, 290], radius=34, fill=BLACK)  # snout
    d.polygon([(360, 130), (410, 210), (310, 210)], fill=BLACK)    # crown fin
    circle(d, 330, 300, 30, WHITE)                                 # eye
    circle(d, 330, 300, 13, BLACK)                                 # pupil (micro)
    d.line([(470, 330), (510, 430)], fill=BLACK, width=36)         # dorsal fin
    return im


def trex_skeleton():
    """A museum-mount silhouette as ONE connected mass -- skull, spine,
    torso, tail and legs all joined, so no bone-to-bone corridor exists to
    flag. Detail lives in the holes: eye socket, jaw gap, three rib slots.
    The famous tiny arm is a detached micro bit."""
    im, d = canvas(1000, 760)
    d.polygon([(660, 160), (940, 200), (950, 270), (830, 290), (660, 280)], fill=BLACK)
    # Jaw HINGED to the skull at the back (overlapping 900..940), gap open
    # at the front only: a floating jaw 5px under the skull flagged both.
    d.polygon([(700, 292), (940, 260), (940, 300), (890, 360), (720, 340)], fill=BLACK)
    d.polygon([(705, 288), (880, 296), (707, 308)], fill=WHITE)              # jaw gap (front)
    circle(d, 800, 225, 26, WHITE)                                # eye socket hole
    d.line([(660, 236), (540, 296), (460, 316)], fill=BLACK, width=56, joint="curve")
    d.ellipse([260, 300, 560, 540], fill=BLACK)                   # torso over hips
    # Rib slots start BELOW the neck band: cutting into it made knife-edge
    # geometry where slot wall met outer boundary at ~zero distance.
    for x in (350, 420, 490):                                     # rib slots (holes)
        d.rounded_rectangle([x, 386, x + 34, 496], radius=16, fill=WHITE)
    d.line([(290, 430), (150, 470), (40, 430)], fill=BLACK, width=52, joint="curve")  # tail
    d.line([(430, 500), (475, 625), (445, 715)], fill=BLACK, width=46, joint="curve")
    d.line([(420, 715), (540, 715)], fill=BLACK, width=28)
    d.line([(340, 500), (320, 630), (355, 710)], fill=BLACK, width=42, joint="curve")
    d.line([(600, 330), (645, 372)], fill=BLACK, width=24)        # tiny arm (micro)
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
