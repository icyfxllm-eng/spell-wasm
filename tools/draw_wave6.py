#!/usr/bin/env python3
"""CC-PICTURE-BANK feature 2 — the culture five, drawn.

One icon per language from Eric's "strongest five" pick (2026-08-01):
pretzel (de), origami crane (ja), dallah (ar), Saint Basil's (ru),
panda (zh). Drawn originals in the draw_ref.py idiom: solid subject on
white, features that must survive tracing are HOLES by construction,
micro features sized into the (40, 130) arc band on the 512 canvas.
The panda marks you know are trademarks, which is one more reason these
are originals. Culture packs are ADDITIVE per language (the spec's own
rule) — every language still gets the full core bank.

Field rules honored (learned the hard way across waves 1-2):
seams >= 34px, hinge overlaps >= 20px, detached micro >= 10px off the
silhouette, and nothing thinner than the corridor floor hosts anything.

Run: python3 tools/draw_wave6.py   (writes content-pipeline/wordpic/ref/)
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


def pretzel():
    """The three-hole knot. Body first, then the holes — each hole is big
    enough to be its own interior contour (the mug-handle lesson), and the
    crossed feet are stubby capsules so nothing knife-edges."""
    im, d = canvas(760, 660)
    d.ellipse([70, 120, 690, 600], fill=BLACK)                        # the loop mass
    d.ellipse([170, 320, 330, 480], fill=WHITE)                       # left hole
    d.ellipse([430, 320, 590, 480], fill=WHITE)                       # right hole
    d.ellipse([300, 170, 460, 320], fill=WHITE)                       # top hole
    # crossed feet: two fat diagonals through the lower body
    d.line([(250, 560), (470, 380)], fill=BLACK, width=52)
    d.line([(290, 380), (510, 560)], fill=BLACK, width=52)
    # bite the outer bottom so the feet read as ends, not a solid base
    d.ellipse([330, 585, 430, 655], fill=WHITE)
    return im


def crane():
    """Origami crane: every edge is a fold, so the silhouette is all
    angles — ONE unbroken outline (the field rule; the first cut tried
    40px fold seams and they severed the bird into floating pieces).
    Folds live in the angles, not in creases."""
    im, d = canvas(860, 700)
    d.polygon([(180, 470), (420, 350), (640, 470), (400, 580)], fill=BLACK)  # body
    d.polygon([(180, 470), (110, 240), (150, 215), (275, 400)], fill=BLACK)  # neck
    d.polygon([(112, 250), (60, 205), (130, 200), (150, 215)], fill=BLACK)   # head + beak
    d.polygon([(340, 380), (430, 120), (520, 380), (430, 425)], fill=BLACK)  # standing wing
    d.polygon([(600, 445), (800, 380), (660, 525)], fill=BLACK)              # tail spike
    return im


def dallah():
    """The Arabic coffee pot: bulb body, waisted neck, flared lid with a
    crescent finial, one sweeping spout, and the handle hole that makes it
    a dallah and not a vase."""
    im, d = canvas(720, 900)
    d.polygon([(250, 380), (470, 380), (430, 560), (450, 700), (270, 700), (290, 560)],
              fill=BLACK)                                             # waisted body
    d.polygon([(240, 700), (480, 700), (510, 800), (210, 800)], fill=BLACK)  # flared base
    d.polygon([(280, 380), (440, 380), (400, 300), (320, 300)], fill=BLACK)  # shoulder
    d.polygon([(300, 300), (420, 300), (390, 240), (330, 240)], fill=BLACK)  # lid
    d.polygon([(340, 240), (380, 240), (362, 168)], fill=BLACK)              # lid peak
    # crescent finial ATTACHED by its stem — a floating crescent flagged
    # against the lid peak; attached, finial and pot are one contour
    d.line([(362, 172), (360, 130)], fill=BLACK, width=18)
    circle(d, 360, 110, 42, BLACK)
    circle(d, 378, 98, 36, WHITE)
    # spout: rooted DEEP in the shoulder (hinge overlap >= 20px), 46px wide
    d.line([(330, 370), (200, 260), (150, 170)], fill=BLACK, width=46, joint="curve")
    d.polygon([(120, 130), (185, 165), (150, 205)], fill=BLACK)              # spout mouth
    # handle: outer arm with the HOLE
    d.ellipse([430, 380, 610, 620], fill=BLACK)
    d.ellipse([480, 430, 570, 570], fill=WHITE)
    return im


def basil():
    """Saint Basil's as a skyline: the central tent spire flanked by four
    onion domes at staggered heights, 36px seams between towers, one
    honest plinth fusing them at the base (the oak's ground rule)."""
    im, d = canvas(940, 780)

    def onion(cx, tip_y, r):
        # onion dome: bulb + pinched tip, on its tower below
        d.polygon([(cx, tip_y), (cx - 12, tip_y + 40), (cx + 12, tip_y + 40)], fill=BLACK)
        d.ellipse([cx - r, tip_y + 26, cx + r, tip_y + 26 + 2 * r], fill=BLACK)
        d.rectangle([cx - int(r * 0.72), tip_y + 26 + r, cx + int(r * 0.72), 640], fill=BLACK)

    onion(150, 240, 62)
    onion(320, 150, 74)
    onion(620, 150, 74)
    onion(790, 240, 62)
    # central tent spire
    d.polygon([(470, 60), (400, 340), (540, 340)], fill=BLACK)
    d.rectangle([405, 340, 535, 640], fill=BLACK)
    circle(d, 470, 52, 16, BLACK)
    # seams between towers stay open sky by construction (gaps >= 36px)
    d.rectangle([60, 640, 880, 730], fill=BLACK)                      # plinth
    return im


def panda():
    """Sitting panda: head with ears and body as one silhouette; the eye
    patches and nose are white holes sized into the micro band, so the
    renderer draws them FILLED — dark patches on the face, which is
    exactly what a panda is (the snowman's-eyes machinery). The belly
    ring is a big hole: an interior contour that hosts words."""
    im, d = canvas(780, 760)
    circle(d, 390, 280, 205, BLACK)                                   # head
    circle(d, 235, 130, 80, BLACK)                                    # ear
    circle(d, 545, 130, 80, BLACK)                                    # ear
    d.ellipse([160, 380, 620, 720], fill=BLACK)                       # body
    # eye patches + nose: micro holes (filled dark in the render)
    d.ellipse([290, 230, 350, 310], fill=WHITE)
    d.ellipse([430, 230, 490, 310], fill=WHITE)
    d.ellipse([362, 330, 418, 378], fill=WHITE)
    # belly: the hostable interior ring
    d.ellipse([280, 460, 500, 660], fill=WHITE)
    return im


SUBJECTS = {
    "pretzel": pretzel,
    "crane": crane,
    "dallah": dallah,
    "basil": basil,
    "panda": panda,
}

if __name__ == "__main__":
    for name, fn in SUBJECTS.items():
        p = REF / f"{name}.png"
        fn().save(p)
        print("wrote", p)
