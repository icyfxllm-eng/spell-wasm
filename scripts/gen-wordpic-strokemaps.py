#!/usr/bin/env python3
"""CC-WORD-PICTURE v5 P1 — starter-pack stroke-map generator (calligrams).

A picture is a manifest of WORD-PATHS: geometry the player's spelled words
are typeset along (D1). Modes: "flow" (textPath; complex-shaping scripts
auto-degrade to chord-angled straight words per Eric's D4 ruling — a RUNTIME
rule, not data), "stack" (vertical letter stack — script-universal). Budgets
are typing-unit ranges the word feed filters to (D2); wide by design (D15a),
with per-language overrides as data when I1 flags shortfalls.

Fill order is choreography (D6): outline → masses → details → focal LAST.
Picture #1 is Eric's notebook smiley (fidelity bar): stacked-word eyes,
arced-word mouth — the mouth is the finale.

Output: config/wordpic/pictures.json (+ ghost-render SVGs for review via
--renders, written to the scratchpad only — review artifacts, not assets).
Expert stroke map (Mona Lisa) is hand-traced against
content-pipeline/wordpic/ref/mona-lisa.jpg (PD; never ships — I4 scan).
"""
import json
import math
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Default budget ranges by tier (typing units). Deliberately wide (D15a);
# zh reads dense — global per-language override halves maxima at runtime? NO:
# overrides are DATA per picture; the starter uses one budget vocabulary and
# the I1 check emits suggestions where a language falls short.
B_EASY = [2, 9]
B_MED = [3, 12]
B_HARD = [3, 12]
B_EXP = [3, 14]


def arc(x0, y0, qx, qy, x1, y1):
    return f"M{x0} {y0} Q{qx} {qy} {x1} {y1}"


def line(x0, y0, x1, y1):
    return f"M{x0} {y0} L{x1} {y1}"


def flow(d, order, budget, band=2, arch="arc", segs=None):
    p = {"mode": "flow", "d": d, "order": order, "budget": budget, "band": band, "arch": arch}
    if segs:
        p["segs"] = segs
    return p


STACK_BUDGET = {"easy": [2, 9], "medium": [3, 12], "hard": [3, 12], "expert": [3, 14]}
_CUR_TIER = ["easy"]  # set per tier section below (data-gen convenience)


def stack(x, y, size, units, order, band=2):
    """A vertical letter stack at (x,y); `units` sizes the ghost, the BUDGET
    is tier-wide (D15a) — the renderer scales glyphs to the region."""
    return {"mode": "stack", "x": x, "y": y, "size": size,
            "order": order, "budget": list(STACK_BUDGET[_CUR_TIER[0]]),
            "band": band, "arch": "stack", "ghost": units}


def pic(pid, tier, subject, icon, kid, paths, wash=False, prov=None, free_hint=None):
    d = {"id": pid, "tier": tier, "subject": subject, "icon": icon, "kid": kid,
         "wash": wash, "paths": sorted(paths, key=lambda p: p["order"])}
    if prov:
        d["provenance"] = prov
    return d


P = []

_CUR_TIER[0] = "easy"
# 1) SMILEY — the notebook sketch (fidelity bar). 4 paths / up to 6 words.
P.append(pic("smiley", "easy", "faces", "🙂", True, [
    flow("M96 256 A160 160 0 0 1 416 256", 1, B_EASY, 1, "outline", 2),   # top of face (2 words)
    flow("M96 256 A160 160 0 1 0 416 256", 2, B_EASY, 1, "outline", 2),   # bottom of face (2 words)
    stack(196, 176, 26, 5, 3),                                            # left eye — "S M I L E"
    stack(316, 176, 26, 5, 4),                                            # right eye — "H A P P Y"
    flow(arc(168, 318, 256, 402, 344, 318), 5, B_EASY, 1, "arc", 2),     # THE MOUTH — finale
]))

# 2) STAR (easy, v6): the outer 10-edge outline as ONE path — the corner
# split (L4) yields ten crisp straight slots; a word per edge.
def star_outline(cx, cy, r):
    pts = []
    for i in range(10):
        rr = r if i % 2 == 0 else r * 0.45
        a = -math.pi / 2 + i * math.pi / 5
        pts.append((round(cx + rr * math.cos(a)), round(cy + rr * math.sin(a))))
    pts.append(pts[0])
    return "M" + " L".join(f"{x} {y}" for x, y in pts)

P.append(pic("star", "easy", "nature", "🌟", True, [
    flow(star_outline(256, 246, 180), 1, B_EASY, 1, "outline"),
    stack(70, 100, 26, 4, 2),
    stack(442, 120, 24, 3, 3),
    flow(line(150, 470, 362, 470), 4, B_EASY, 2, "line"),
]))

# 3) FISH (easy): body arcs, tail, fin, bubbles stack.
P.append(pic("fish", "easy", "animals", "🐟", True, [
    flow(arc(120, 256, 256, 130, 380, 256), 1, B_EASY, 1, "outline", 2),
    flow(arc(120, 256, 256, 382, 380, 256), 2, B_EASY, 1, "outline", 2),
    flow(line(380, 256, 462, 196), 3, B_EASY, 2, "line"),
    flow(line(380, 256, 462, 316), 4, B_EASY, 2, "line"),
    stack(120, 130, 24, 4, 5),                                             # bubbles
]))

# 4) HOUSE (easy): roof, walls, base, door.
P.append(pic("house", "easy", "objects", "🏠", True, [
    flow(line(120, 250, 256, 130), 1, B_EASY, 1, "line"),
    flow(line(256, 130, 392, 250), 2, B_EASY, 1, "line"),
    stack(140, 270, 30, 4, 3),                                             # left wall
    stack(372, 270, 30, 4, 4),                                             # right wall
    flow(line(120, 400, 392, 400), 5, B_EASY, 1, "line", 2),
    flow(arc(226, 400, 256, 330, 286, 400), 6, B_EASY, 3, "arc"),          # door — finale
]))

_CUR_TIER[0] = "medium"
# 5) CAT (medium): ears, head, whiskers, body, tail, face.
P.append(pic("cat", "medium", "animals", "🐱", True, [
    flow("M150 240 A106 106 0 0 1 362 240", 1, B_MED, 1, "outline", 2),    # head top
    flow("M150 240 A106 106 0 1 0 362 240", 2, B_MED, 1, "outline", 2),    # head bottom
    flow(line(170, 190, 196, 120), 3, B_EASY, 2, "line"),                  # ear L up
    flow(line(196, 120, 226, 180), 4, B_EASY, 2, "line"),                  # ear L down
    flow(line(286, 180, 316, 120), 5, B_EASY, 2, "line"),                  # ear R up
    flow(line(316, 120, 342, 190), 6, B_EASY, 2, "line"),                  # ear R down
    flow(arc(160, 360, 130, 430, 220, 470), 7, B_MED, 2, "arc"),           # body L
    flow(arc(352, 360, 382, 430, 292, 470), 8, B_MED, 2, "arc"),           # body R
    flow(line(220, 470, 292, 470), 9, B_EASY, 2, "line"),                  # base
    flow(arc(352, 430, 452, 420, 442, 330), 10, B_MED, 2, "wave"),         # tail
    flow(line(130, 262, 196, 268), 11, B_EASY, 3, "line"),                 # whisker L
    flow(line(316, 268, 382, 262), 12, B_EASY, 3, "line"),                 # whisker R
    stack(226, 232, 22, 3, 13),                                            # eye L
    stack(296, 232, 22, 3, 14),                                            # eye R
    flow(arc(226, 292, 256, 322, 286, 292), 15, B_EASY, 3, "arc"),         # smile — finale
]))

# 6) ROCKET (medium).
P.append(pic("rocket", "medium", "vehicles", "🚀", True, [
    stack(226, 180, 28, 6, 1),                                             # body L
    stack(292, 180, 28, 6, 2),                                             # body R
    flow(arc(226, 170, 256, 60, 292, 170), 3, B_MED, 1, "arc"),            # nose
    flow(line(226, 340, 170, 420), 4, B_EASY, 2, "line"),
    flow(line(170, 420, 226, 400), 5, B_EASY, 2, "line"),
    flow(line(292, 340, 342, 420), 6, B_EASY, 2, "line"),
    flow(line(342, 420, 292, 400), 7, B_EASY, 2, "line"),
    flow(arc(226, 262, 256, 232, 292, 262), 8, B_EASY, 3, "arc"),          # window top
    flow(arc(226, 262, 256, 292, 292, 262), 9, B_EASY, 3, "arc"),          # window bottom
    flow(line(120, 60, 180, 110), 10, B_EASY, 3, "line"),                  # star streak 1
    flow(line(392, 90, 442, 130), 11, B_EASY, 3, "line"),                  # star streak 2
    flow(arc(226, 420, 256, 500, 292, 420), 12, B_EASY, 2, "arc"),         # flame outer — finale-ish
    stack(256, 430, 24, 4, 13),                                            # flame core — finale
]))

# 7) SNOWMAN (medium).
P.append(pic("snowman", "medium", "holidays", "⛄", True, [
    flow("M170 360 A86 86 0 0 1 342 360", 1, B_MED, 1, "outline", 2),
    flow("M170 360 A86 86 0 1 0 342 360", 2, B_MED, 1, "outline", 2),
    flow("M204 220 A52 52 0 0 1 308 220", 3, B_MED, 1, "arc"),
    flow("M204 220 A52 52 0 1 0 308 220", 4, B_MED, 1, "arc"),
    flow(line(196, 150, 316, 150), 5, B_EASY, 2, "line"),                  # hat brim
    stack(238, 90, 22, 3, 6),                                              # hat side L
    stack(278, 90, 22, 3, 7),                                              # hat side R
    flow(line(170, 330, 96, 280), 8, B_EASY, 2, "line"),                   # arm L
    flow(line(342, 330, 416, 280), 9, B_EASY, 2, "line"),                  # arm R
    stack(256, 320, 22, 4, 10),                                            # buttons
    stack(232, 196, 18, 2, 11),                                            # eye L
    stack(282, 196, 18, 2, 12),                                            # eye R
    flow(arc(230, 246, 256, 268, 282, 246), 13, B_EASY, 3, "arc"),         # smile — finale
]))

_CUR_TIER[0] = "hard"
# 8) DRAGON (hard): 26 paths.
_dr = []
_dr.append(flow(arc(120, 350, 100, 250, 190, 240), 1, B_HARD, 1, "outline"))
_dr.append(flow(arc(190, 240, 300, 170, 380, 250), 2, B_HARD, 1, "outline", 2))
_dr.append(flow(arc(380, 250, 460, 260, 440, 310), 3, B_HARD, 1, "outline"))   # head
_dr.append(flow(arc(440, 310, 420, 350, 360, 350), 4, B_HARD, 1, "arc"))       # jaw
_dr.append(flow(arc(360, 350, 300, 420, 170, 400), 5, B_HARD, 1, "outline", 2))  # belly
_dr.append(flow(arc(170, 400, 130, 390, 120, 350), 6, B_EASY, 1, "arc"))
_dr.append(flow(arc(120, 350, 60, 330, 50, 270), 7, B_HARD, 2, "wave"))        # tail 1
_dr.append(flow(arc(50, 270, 45, 220, 80, 210), 8, B_EASY, 2, "wave"))         # tail 2
_dr.append(flow(line(60, 235, 30, 210), 9, B_EASY, 2, "line"))                 # spade a
_dr.append(flow(line(30, 210, 62, 196), 10, B_EASY, 2, "line"))                # spade b
for k in range(5):                                                              # wing ribs
    x0 = 210 + k * 26
    _dr.append(flow(line(x0, 236 - (k % 2) * 8, 200 + k * 34, 120 + k * 6),
                    11 + k, B_EASY, 2, "radial"))
_dr.append(flow(arc(200, 120, 300, 90, 370, 150), 16, B_HARD, 2, "arc", 2))    # wing top
for k in range(4):                                                              # back spikes
    x0 = 210 + k * 40
    _dr.append(flow(line(x0, 212 - (k % 2) * 6, x0 + 18, 184), 17 + k, B_EASY, 3, "zigzag"))
_dr.append(stack(408, 282, 20, 3, 21, 3))                                      # eye
_dr.append(flow(line(452, 320, 500, 300), 22, B_EASY, 3, "line"))               # fire 1
_dr.append(flow(line(452, 334, 504, 334), 23, B_EASY, 3, "line"))               # fire 2
_dr.append(flow(line(452, 348, 500, 366), 24, B_EASY, 3, "line"))               # fire 3
_dr.append(flow(arc(180, 430, 256, 460, 330, 430), 25, B_HARD, 3, "arc"))       # ground shadow
_dr.append(flow(arc(360, 300, 396, 316, 372, 336), 26, B_EASY, 4, "arc"))       # nostril-smile — finale
P.append(pic("dragon", "hard", "fantasy", "🐲", True, _dr))

# 9) EIFFEL (hard): 30 paths.
_ei = []
_ei.append(flow(arc(150, 400, 180, 320, 224, 292), 1, B_HARD, 1, "outline"))
_ei.append(flow(arc(362, 400, 332, 320, 288, 292), 2, B_HARD, 1, "outline"))
_ei.append(flow(arc(178, 400, 256, 320, 334, 400), 3, B_HARD, 1, "arc", 2))    # arch
_ei.append(flow(line(186, 292, 326, 292), 4, B_HARD, 1, "line", 2))            # platform 1
_ei.append(flow(line(204, 292, 232, 200), 5, B_EASY, 2, "line"))
_ei.append(flow(line(308, 292, 280, 200), 6, B_EASY, 2, "line"))
for k in range(6):                                                              # lattice X mid
    y = 280 - k * 13
    if k % 2 == 0:
        _ei.append(flow(line(212 + k * 2, y, 300 - k * 2, y - 26), 7 + k, B_EASY, 3, "zigzag"))
    else:
        _ei.append(flow(line(300 - k * 2, y, 212 + k * 2, y - 26), 7 + k, B_EASY, 3, "zigzag"))
_ei.append(flow(line(222, 200, 290, 200), 13, B_EASY, 2, "line"))               # platform 2
_ei.append(flow(line(236, 200, 248, 120), 14, B_EASY, 2, "line"))
_ei.append(flow(line(276, 200, 264, 120), 15, B_EASY, 2, "line"))
for k in range(4):                                                              # upper lattice
    y = 190 - k * 17
    if k % 2 == 0:
        _ei.append(flow(line(240 + k, y, 272 - k, y - 17), 16 + k, B_EASY, 3, "zigzag"))
    else:
        _ei.append(flow(line(272 - k, y, 240 + k, y - 17), 16 + k, B_EASY, 3, "zigzag"))
_ei.append(flow(line(246, 120, 266, 120), 20, B_EASY, 2, "line"))
_ei.append(stack(256, 68, 20, 3, 21, 2))                                        # spire
for k in range(3):                                                               # ground
    _ei.append(flow(line(80 + k * 20, 430 - k * 4, 200 + k * 30, 430 - k * 4), 22 + k, B_EASY, 3, "line"))
_ei.append(flow(arc(60, 120, 110, 80, 160, 120), 25, B_EASY, 3, "arc"))          # cloud L
_ei.append(flow(arc(360, 90, 410, 55, 460, 90), 26, B_EASY, 3, "arc"))           # cloud R
_ei.append(flow(line(410, 400, 470, 400), 27, B_EASY, 3, "line"))
_ei.append(flow(line(50, 400, 110, 400), 28, B_EASY, 3, "line"))
_ei.append(flow(arc(230, 110, 256, 92, 282, 110), 29, B_EASY, 3, "arc"))
_ei.append(stack(256, 34, 18, 2, 30, 4))                                         # beacon — finale
P.append(pic("eiffel", "hard", "landmarks", "🗼", False, _ei))

_CUR_TIER[0] = "expert"
# 10) MONA LISA (expert): 56 paths, hand-traced bands over the PD reference
# (content-pipeline/wordpic/ref/mona-lisa.jpg — never ships). Canvas 512x512,
# subject centered as in the painting: head upper-center, hands lower-left.
_ml = []
o = 0
def nx():
    global o
    o += 1
    return o
# Band 1 (background, large/light): horizon + landscape, 10 paths.
_ml.append(flow(line(30, 150, 180, 146), nx(), B_EXP, 1, "line", 3))
_ml.append(flow(line(332, 146, 482, 150), nx(), B_EXP, 1, "line", 3))
_ml.append(flow(arc(30, 190, 110, 170, 180, 186), nx(), B_EXP, 1, "wave", 3))
_ml.append(flow(arc(332, 186, 410, 168, 482, 190), nx(), B_EXP, 1, "wave", 3))
_ml.append(flow(arc(30, 230, 100, 210, 170, 226), nx(), B_EXP, 1, "wave", 3))
_ml.append(flow(arc(342, 226, 412, 208, 482, 230), nx(), B_EXP, 1, "wave", 3))
_ml.append(flow(line(30, 270, 160, 268), nx(), B_EXP, 1, "line", 3))
_ml.append(flow(line(352, 268, 482, 270), nx(), B_EXP, 1, "line", 3))
_ml.append(flow(arc(30, 110, 256, 90, 482, 110), nx(), B_EXP, 1, "line", 3))
_ml.append(flow(arc(30, 60, 256, 40, 482, 60), nx(), B_EXP, 1, "line", 3))
# Band 2 (veil + hair falls, dark/bold): 12 paths.
_ml.append(flow(arc(196, 96, 256, 66, 316, 96), nx(), B_EXP, 2, "arc", 3))          # veil crown
_ml.append(flow(arc(186, 130, 178, 200, 186, 270), nx(), B_EXP, 2, "wave", 3))      # hair L1
_ml.append(flow(arc(200, 140, 194, 210, 202, 280), nx(), B_EXP, 2, "wave", 3))      # hair L2
_ml.append(flow(arc(214, 150, 210, 216, 218, 286), nx(), B_EXP, 2, "wave", 3))      # hair L3
_ml.append(flow(arc(326, 130, 334, 200, 326, 270), nx(), B_EXP, 2, "wave", 3))      # hair R1
_ml.append(flow(arc(312, 140, 318, 210, 310, 280), nx(), B_EXP, 2, "wave", 3))      # hair R2
_ml.append(flow(arc(298, 150, 302, 216, 294, 286), nx(), B_EXP, 2, "wave", 3))      # hair R3
_ml.append(flow(arc(186, 96, 176, 160, 186, 224), nx(), B_EXP, 2, "arc", 3))        # veil L
_ml.append(flow(arc(326, 96, 336, 160, 326, 224), nx(), B_EXP, 2, "arc", 3))        # veil R
_ml.append(flow(arc(196, 84, 256, 58, 316, 84), nx(), B_EXP, 2, "arc", 3))
_ml.append(flow(arc(206, 108, 256, 92, 306, 108), nx(), B_EXP, 2, "arc", 3))        # hairline
_ml.append(flow(arc(226, 300, 256, 316, 286, 300), nx(), B_EXP, 2, "arc", 3))       # chin
# Band 3 (face + neck, smallest): 12 paths (features late).
_ml.append(flow(arc(206, 130, 198, 200, 216, 268), nx(), B_EASY, 3, "outline", 2))  # face L
_ml.append(flow(arc(306, 130, 314, 200, 296, 268), nx(), B_EASY, 3, "outline", 2))  # face R
_ml.append(flow(arc(216, 268, 256, 306, 296, 268), nx(), B_EASY, 3, "arc", 2))      # jaw
_ml.append(flow(line(238, 320, 274, 320), nx(), B_EASY, 3, "line", 2))              # neck base
_ml.append(flow(line(232, 296, 232, 336), nx(), B_EASY, 3, "line", 2))              # neck L
_ml.append(flow(line(280, 296, 280, 336), nx(), B_EASY, 3, "line", 2))              # neck R
_ml.append(flow(arc(222, 176, 238, 168, 252, 176), nx(), B_EASY, 3, "arc", 2))      # brow L
_ml.append(flow(arc(260, 176, 274, 168, 290, 176), nx(), B_EASY, 3, "arc", 2))      # brow R
_ml.append(stack(238, 192, 12, 2, nx(), 3))                                      # eye L
_ml.append(stack(274, 192, 12, 2, nx(), 3))                                      # eye R
_ml.append(flow(line(256, 200, 256, 236), nx(), B_EASY, 3, "line", 2))              # nose
_ml.append(flow(arc(244, 224, 256, 230, 268, 224), nx(), B_EASY, 3, "arc", 2))      # nostril line
# Band 2/3 (dress + shoulders): 14 paths.
_ml.append(flow(arc(186, 300, 150, 340, 140, 400), nx(), B_EXP, 2, "arc", 3))       # shoulder L
_ml.append(flow(arc(326, 300, 362, 340, 372, 400), nx(), B_EXP, 2, "arc", 3))       # shoulder R
for k in range(5):                                                                # drapery falls L
    x = 156 + k * 14
    _ml.append(flow(arc(x, 340, x - 6, 400, x + 4, 468), nx(), B_EXP, 2, "wave", 3))
for k in range(5):                                                                # drapery falls R
    x = 300 + k * 14
    _ml.append(flow(arc(x, 340, x + 6, 400, x - 4, 468), nx(), B_EXP, 2, "wave", 3))
_ml.append(flow(arc(230, 330, 256, 344, 282, 330), nx(), B_EASY, 2, "arc", 3))      # neckline
_ml.append(flow(arc(210, 350, 256, 372, 302, 350), nx(), B_EASY, 2, "arc", 3))      # bodice
# Band 4 (hands, then THE SMILE last): 8 paths.
_ml.append(flow(arc(190, 470, 240, 450, 290, 470), nx(), B_EXP, 3, "arc", 2))       # forearm
_ml.append(flow(arc(250, 470, 300, 456, 344, 476), nx(), B_EXP, 3, "arc", 2))       # hand over hand
for k in range(4):                                                                # fingers
    _ml.append(flow(line(296 + k * 12, 470 + (k % 2) * 4, 330 + k * 10, 486), nx(), B_EASY, 3, "line", 2))
_ml.append(flow(arc(236, 246, 256, 252, 276, 246), nx(), B_EASY, 4, "arc", 2))      # upper lip shadow
_ml.append(flow(arc(234, 256, 256, 270, 278, 256), nx(), B_EASY, 4, "arc", 2))      # THE SMILE — final word
P.append(pic("mona", "expert", "masterpieces", "🖼️", False, _ml,
             prov={
                 "title": "Mona Lisa",
                 "artist": "Leonardo da Vinci (1452–1519)",
                 "source": "Wikimedia Commons",
                 "sourceUrl": "https://commons.wikimedia.org/wiki/Special:FilePath/Mona_Lisa,_by_Leonardo_da_Vinci,_from_C2RMF_retouched.jpg",
                 "pdBasis": "Public domain: painting c. 1503–1506, artist died 1519; stroke map hand-traced at content time over the PD reference (reference image not shipped)",
                 "retrieved": "2026-07-28"}))

BANDS = {"easy": (3, 8), "medium": (10, 20), "hard": (25, 45), "expert": (50, 90)}


def main():
    for p in P:
        n = len(p["paths"])
        lo, hi = BANDS[p["tier"]]
        assert lo <= n <= hi, f"{p['id']}: {n} paths outside {p['tier']} band {lo}-{hi}"
        orders = [q["order"] for q in p["paths"]]
        assert orders == list(range(1, n + 1)), f"{p['id']}: fill order not 1..{n}: {orders}"
    manifest = {
        "$doc": "CC-WORD-PICTURE v5 starter pack — calligram stroke maps. Words typeset along paths; ghost guides from word 1; focal stroke last (D6).",
        "pack": "starter",
        "flowMs": 1000,
        "minFont": 12,
        "recencyBias": 3,
        "free": ["smiley", "house", "cat"],
        "pictures": P,
    }
    out = os.path.join(ROOT, "config", "wordpic", "pictures.json")
    with open(out, "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=1)
    total_words = 0
    for p in P:
        words = sum(q.get("segs", 1) for q in p["paths"])
        total_words += words
        print(f"  {p['id']:<9} {p['tier']:<7} {len(p['paths'])} paths, ~{words} words")
    print(f"gen-wordpic-strokemaps: 10 pictures, ~{total_words} words total → {out}")


if __name__ == "__main__":
    main()
