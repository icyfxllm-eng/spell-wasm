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

Output: config/wordpic/pictures.json (+ outline-render SVGs for review via
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
B_EASY = [2, 10]
B_MED = [3, 14]
B_HARD = [3, 14]
B_EXP = [3, 20]


def arc(x0, y0, qx, qy, x1, y1):
    return f"M{x0} {y0} Q{qx} {qy} {x1} {y1}"


def line(x0, y0, x1, y1):
    return f"M{x0} {y0} L{x1} {y1}"


def flow(d, order, budget, band=2, arch="arc", segs=None):
    p = {"mode": "flow", "d": d, "order": order, "budget": budget, "band": band, "arch": arch}
    if segs:
        p["segs"] = segs
    return p


STACK_BUDGET = {"easy": [2, 10], "medium": [3, 14], "hard": [3, 14], "expert": [3, 20]}
_CUR_TIER = ["easy"]  # set per tier section below (data-gen convenience)


def stack(x, y, size, units, order, band=2):
    """A vertical letter stack at (x,y); `units` sizes the column, the BUDGET
    is tier-wide (D15a) — the renderer scales glyphs to the region."""
    return {"mode": "stack", "x": x, "y": y, "size": size,
            "order": order, "budget": list(STACK_BUDGET[_CUR_TIER[0]]),
            "band": band, "arch": "stack", "column": units}


def tuft(x, y, h):
    """Small grass tuft: a shallow ~55px arc, the minimum fillable stroke."""
    return arc(x - 28, y, x, y - h, x + 28, y)


# v7 F2 — stroke attribution: every path names the feature it draws. A
# stroke nobody can name is noise (round-2 lesson: fish blob, tower sides).
BANNED_FEATURES = {"decorative", "filler", "background texture"}
FEATURES = {
    "smiley": ["head", "head", "left eye", "right eye", "smile"],
    "star": ["star outline", "top-left sparkle", "top-right sparkle", "ground"],
    "fish": ["back", "belly", "upper tail fin", "lower tail fin"],
    "house": ["left roof slope", "right roof slope", "left window", "right window",
              "left base wall", "right base wall", "door", "sun"],
    "cat": ["head", "chin", "left ear", "right ear", "left eye", "right eye", "smile",
            "left whiskers", "right whiskers", "left flank", "right flank", "haunch", "tail"],
    "rocket": ["left fuselage", "right fuselage", "nose cone", "left fin edge", "left fin base",
               "right fin edge", "right fin base", "engine nozzle", "flame plume",
               "left flame jet", "right flame jet", "cloud", "star streak", "star streak"],
    "snowman": ["body", "head", "head", "left shoulder", "right shoulder", "hat brim",
                "hat top", "hat band", "brow", "smile", "buttons", "left arm", "right arm",
                "snow bank", "left drift", "right drift"],
    "dragon": ["chest", "back", "head", "jaw", "belly", "crest spike", "crest spike",
               "crest spike", "crest ridge", "tail", "tail barb", "tail barb", "snout",
               "fire breath", "fire breath", "fire breath", "mountain slope", "mountain slope",
               "ground", "river", "cloud", "cloud", "star streak", "wing", "bird"],
    "eiffel": ["left leg", "right leg", "base arch", "first platform", "left pillar",
               "right pillar", "arch crown", "second platform", "spire", "spire edge",
               "spire edge", "beacon", "left ground", "right ground", "cloud", "cloud",
               "cloud", "left lawn", "left lawn", "right lawn", "right lawn"],
    "mona": ["picture frame"] + ["loggia colonnade"] * 12
            + ["hair cascade"] * 6 + ["crown of head", "left cheek", "right cheek", "jaw",
               "brow line", "nose", "left neck", "right neck", "left shoulder",
               "right shoulder", "bodice fold", "bodice fold", "bodice fold"]
            + ["sleeve fold"] * 6 + ["left hand", "right hand"]
            + ["parapet rail"] * 4 + ["drapery"] + ["parapet rail"] * 4 + ["smile"],
    "dog": ["crown", "jaw", "left ear", "right ear", "nose", "tongue", "bone"],
    "butterfly": ["body", "upper left wing", "lower left wing", "upper right wing",
                  "lower right wing", "left antenna", "right antenna"],
    "snail": ["shell whorl", "foot", "neck", "left eye stalk", "right eye stalk",
              "left eye", "right eye", "smile"],
    "duck": ["head", "beak", "back", "breast", "waterline", "waterline"],
    "owl": ["left body side", "right body side", "belly", "left ear tuft", "right ear tuft",
            "left eye ring", "left eye ring", "right eye ring", "right eye ring",
            "chest band", "branch", "moon"],
    "turtle": ["shell dome", "shell base", "neck", "head", "front leg", "hind leg", "tail",
               "grass tuft", "grass tuft", "sun", "cloud"],
    "elephant": ["back", "forehead", "trunk", "ear", "belly", "front leg", "hind leg",
                 "tusk", "tail", "grass tuft", "horizon", "cloud"],
    "horse": ["back", "neck", "head", "muzzle", "ear", "mane", "chest", "belly",
              "hind leg", "hind leg", "front leg", "front leg", "tail", "tail strand",
              "ground", "ground", "fence rail", "fence rail", "sun", "cloud", "cloud",
              "bird", "bird", "grass", "grass"],
    "peacock": ["tail feather"] * 9 + ["feather eyespot"] * 4 + ["outer plume arc"] * 2
               + ["shoulders", "left body side", "right body side", "head",
                  "outer plume arc", "outer plume arc", "ground",
                  "path", "path", "wing plume", "wing plume"],
    "rhino": ["woodcut caption"] * 4 + ["back", "rump", "tail", "belly",
              "front leg", "front leg", "hind leg", "hind leg", "forehead", "jaw", "mouth",
              "ear", "dorsal hornlet", "gorget fold", "shoulder plate", "shoulder rivets",
              "shoulder rivets", "body plate", "body rivets", "haunch plate",
              "haunch rivets", "haunch rivets", "haunch fold", "belly scales",
              "belly scales", "belly scales", "ground", "ground", "cloud", "cloud",
              "cloud", "cloud", "bird", "bird", "horn"],
}


def pic(pid, tier, subject, icon, kid, paths, wash=False, prov=None, free_hint=None, pack="starter"):
    ordered = sorted(paths, key=lambda p: p["order"])
    for k, q in enumerate(ordered):
        q["order"] = k + 1  # normalize: authoring gaps are fine, output is 1..n
    feats = FEATURES[pid]
    assert len(feats) == len(paths), f"{pid}: {len(paths)} paths but {len(feats)} feature labels"
    for q, f in zip(ordered, feats):
        assert f and f.lower() not in BANNED_FEATURES, f"{pid}: banned/empty feature {f!r}"
        q["feature"] = f
    d = {"id": pid, "tier": tier, "subject": subject, "icon": icon, "kid": kid,
         "wash": wash, "pack": pack, "paths": ordered}
    if prov:
        d["provenance"] = prov
    return d


P = []

# v6 re-authoring rule (Eric: "no words overlap; words fill the line"):
# parallel strokes keep >= 0.42*(sizeA+sizeB) clearance for their bands;
# interior decorations that cannot host text at clearance are REMOVED, not
# nudged. Stroke endpoints may meet (junction contact is legal).

# 1) SMILEY — the notebook sketch. Circle, stacked-word eyes, arced smile last.
_CUR_TIER[0] = "easy"
P.append(pic("smiley", "easy", "faces", "🙂", True, [
    flow("M76 256 A180 180 0 0 1 436 256", 1, B_EASY, 1, "outline"),
    flow("M76 256 A180 180 0 1 0 436 256", 2, B_EASY, 1, "outline"),
    stack(196, 170, 24, 4, 3),
    stack(316, 170, 24, 4, 4),
    flow(arc(168, 330, 256, 412, 344, 330), 5, B_EASY, 1, "arc"),
]))

def bird(cx, cy, sz):
    return (f"M{cx - 2 * sz} {cy} q{sz} -{sz} {2 * sz} 0 q{sz} -{sz} {2 * sz} 0")


def pine(cx, gy, h):
    w = h * 0.55
    return (f"M{cx - w * 0.75:.0f} {gy} L{cx} {gy - h} L{cx + w * 0.75:.0f} {gy}")


def star_outline(cx, cy, r):
    pts = []
    for i in range(10):
        rr = r if i % 2 == 0 else r * 0.45
        a = -math.pi / 2 + i * math.pi / 5
        pts.append((round(cx + rr * math.cos(a)), round(cy + rr * math.sin(a))))
    pts.append(pts[0])
    return "M" + " L".join(f"{x} {y}" for x, y in pts)


# 2) STAR — outer 10-edge outline (L4 corner split → crisp points).
P.append(pic("star", "easy", "nature", "🌟", True, [
    flow(star_outline(256, 250, 180), 1, B_EASY, 1, "outline"),
    stack(56, 56, 26, 4, 2),
    stack(456, 72, 26, 4, 3),
    flow(line(150, 486, 362, 486), 4, B_EASY, 2, "line"),
]))

# 3) FISH — Eric's golden. Body + tail unchanged in spirit; the interior fin
# (text can't host there) is gone; boat slimmed to hull + mast stack.
P.append(pic("fish", "easy", "animals", "🐟", True, [
    flow(arc(146, 282, 256, 176, 392, 300), 5, B_EASY, 1, "outline"),
    flow(arc(146, 318, 256, 424, 392, 300), 6, B_EASY, 1, "outline"),
    flow(line(404, 291, 474, 238), 7, B_EASY, 2, "line"),
    flow(line(404, 309, 474, 362), 8, B_EASY, 2, "line"),
]))

# 4) HOUSE.
P.append(pic("house", "easy", "objects", "🏠", True, [
    flow(line(120, 252, 248, 128), 1, B_EASY, 1, "line"),
    flow(line(264, 128, 392, 252), 2, B_EASY, 1, "line"),
    stack(150, 296, 28, 4, 3),
    stack(362, 296, 28, 4, 4),
    flow(line(120, 434, 214, 434), 5, B_EASY, 1, "line"),
    flow(line(298, 434, 392, 434), 6, B_EASY, 1, "line"),
    flow(arc(238, 404, 256, 366, 274, 404), 7, B_EASY, 3, "arc"),
    flow(arc(52, 96, 92, 56, 132, 96), 8, B_EASY, 2, "arc"),
]))

# 5) CAT — full redo (Eric: "crammed"). Small strokes, small words, air.
_CUR_TIER[0] = "medium"
P.append(pic("cat", "medium", "animals", "🐱", True, [
    flow("M161 200 A95 95 0 0 1 351 200", 1, B_MED, 1, "outline"),
    flow("M161 200 A95 95 0 1 0 351 200", 2, B_MED, 1, "outline"),
    flow(line(168, 20, 198, 96), 3, B_MED, 2, "line"),
    flow(line(314, 96, 344, 20), 4, B_MED, 2, "line"),
    flow(arc(196, 182, 222, 168, 248, 182), 5, B_MED, 3, "arc"),
    flow(arc(264, 182, 290, 168, 316, 182), 6, B_MED, 3, "arc"),
    flow(arc(228, 252, 256, 276, 284, 252), 7, B_MED, 3, "arc"),
    flow(line(30, 238, 100, 248), 8, B_MED, 3, "line"),
    flow(line(412, 248, 482, 238), 9, B_MED, 3, "line"),
    flow(arc(204, 304, 140, 368, 166, 446), 10, B_MED, 2, "arc"),
    flow(arc(308, 304, 372, 368, 346, 446), 11, B_MED, 2, "arc"),
    flow(arc(166, 446, 256, 470, 346, 446), 12, B_MED, 2, "arc"),
    flow(arc(346, 446, 460, 430, 448, 330), 13, B_MED, 2, "wave"),
]))

# 6) ROCKET — the short left side fills; flame simplified to hostable strokes.
P.append(pic("rocket", "medium", "vehicles", "🚀", True, [
    stack(226, 170, 24, 6, 1),
    stack(292, 170, 24, 6, 2),
    flow(arc(226, 160, 256, 66, 292, 160), 3, B_MED, 1, "arc"),
    flow(line(218, 322, 156, 424), 4, B_MED, 2, "line"),
    flow(line(150, 432, 208, 416), 5, B_MED, 2, "line"),
    flow(line(300, 322, 364, 424), 6, B_MED, 2, "line"),
    flow(line(304, 416, 362, 432), 7, B_MED, 2, "line"),
    flow(line(256, 330, 256, 414), 8, B_MED, 3, "line"),
    flow(arc(226, 436, 256, 498, 292, 436), 10, B_MED, 2, "arc"),
    flow(line(212, 432, 186, 484), 11, B_MED, 3, "line"),
    flow(line(306, 432, 332, 484), 12, B_MED, 3, "line"),
    flow(arc(52, 116, 84, 84, 116, 116), 13, B_MED, 2, "arc"),
    flow(line(384, 68, 456, 116), 14, B_MED, 3, "line"),
    flow(line(100, 52, 180, 100), 15, B_MED, 3, "line"),
]))

# 7) SNOWMAN.
P.append(pic("snowman", "medium", "holidays", "⛄", True, [
    flow("M176 340 A82 86 0 1 0 336 340", 1, B_MED, 1, "outline"),
    flow("M201 205 A55 55 0 0 1 311 205", 2, B_MED, 1, "arc"),
    flow("M201 205 A55 55 0 1 0 311 205", 3, B_MED, 1, "arc"),
    flow(arc(176, 332, 196, 292, 226, 284), 4, B_MED, 2, "arc"),
    flow(arc(286, 284, 316, 292, 336, 332), 17, B_MED, 2, "arc"),
    flow(line(204, 128, 308, 128), 5, B_MED, 2, "line"),
    flow(line(206, 62, 306, 62), 6, B_MED, 2, "line"),
    flow(line(206, 95, 306, 95), 16, B_MED, 3, "line"),
    flow(arc(204, 192, 256, 178, 308, 192), 7, B_MED, 3, "arc"),
    flow(arc(230, 232, 256, 244, 282, 232), 9, B_MED, 3, "arc"),
    flow(line(256, 296, 256, 392), 10, B_MED, 3, "line"),
    flow(line(176, 330, 92, 288), 11, B_MED, 2, "line"),
    flow(line(336, 330, 420, 288), 12, B_MED, 2, "line"),
    flow(arc(60, 474, 256, 458, 452, 474), 13, B_MED, 2, "line"),
    flow(arc(70, 432, 120, 420, 170, 432), 14, B_MED, 3, "arc"),
    flow(arc(342, 432, 392, 420, 442, 432), 15, B_MED, 3, "arc"),
]))

# 8) DRAGON (hard) — fanned ribs, spread fire, no interior spikes.
_CUR_TIER[0] = "hard"
_d = []
_o = [0]
def dn():
    _o[0] += 1
    return _o[0]
_d.append(flow(arc(150, 380, 120, 300, 180, 272), dn(), B_HARD, 1, "outline"))
_d.append(flow(arc(180, 272, 240, 210, 320, 224), dn(), B_HARD, 1, "outline"))
_d.append(flow(arc(320, 224, 392, 232, 408, 280), dn(), B_HARD, 1, "outline"))
_d.append(flow(arc(408, 280, 400, 330, 350, 342), dn(), B_HARD, 2, "arc"))
_d.append(flow(arc(160, 388, 256, 400, 344, 352), dn(), B_HARD, 2, "arc"))
_d.append(flow(line(248, 190, 210, 116), dn(), B_HARD, 2, "radial"))
_d.append(flow(line(266, 188, 262, 104), dn(), B_HARD, 2, "radial"))
_d.append(flow(line(284, 190, 316, 114), dn(), B_HARD, 2, "radial"))
_d.append(flow(arc(206, 112, 262, 68, 320, 110), dn(), B_HARD, 2, "arc"))
_d.append(flow(arc(150, 380, 66, 336, 78, 238), dn(), B_HARD, 2, "wave"))
_d.append(flow(line(34, 186, 84, 244), dn(), B_HARD, 2, "line"))
_d.append(flow(line(40, 192, 118, 174), dn(), B_HARD, 2, "line"))
_d.append(flow(arc(348, 280, 382, 262, 416, 280), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(420, 306, 480, 288), dn(), B_HARD, 3, "line"))
_d.append(flow(line(424, 322, 492, 322), dn(), B_HARD, 3, "line"))
_d.append(flow(line(420, 338, 480, 356), dn(), B_HARD, 3, "line"))
_d.append(flow(line(20, 448, 64, 376), dn(), B_HARD, 2, "line"))
_d.append(flow(line(64, 376, 108, 448), dn(), B_HARD, 2, "line"))
_d.append(flow(arc(140, 452, 300, 440, 482, 452), dn(), B_HARD, 2, "line"))
_d.append(flow(line(320, 396, 470, 396), dn(), B_HARD, 3, "line"))
_d.append(flow(arc(150, 64, 196, 40, 242, 64), dn(), B_HARD, 3, "arc"))
_d.append(flow(arc(322, 84, 366, 58, 410, 86), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(350, 34, 430, 70), dn(), B_HARD, 3, "line"))
_d.append(flow(arc(322, 186, 358, 170, 394, 208), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(118, 134, 194, 92), dn(), B_HARD, 3, "line"))
P.append(pic("dragon", "hard", "fantasy", "🐲", True, _d))

# 9) EIFFEL (hard) — single diagonals, no crossing braces.
_e = []
_o2 = [0]
def en():
    _o2[0] += 1
    return _o2[0]
_e.append(flow(arc(150, 400, 180, 324, 218, 304), en(), B_HARD, 1, "outline"))
_e.append(flow(arc(362, 400, 332, 324, 294, 304), en(), B_HARD, 1, "outline"))
_e.append(flow(arc(178, 400, 256, 330, 334, 400), en(), B_HARD, 1, "arc"))
_e.append(flow(line(186, 284, 326, 284), en(), B_HARD, 1, "line"))
_e.append(flow(line(212, 284, 236, 204), en(), B_HARD, 2, "line"))
_e.append(flow(line(300, 284, 276, 204), en(), B_HARD, 2, "line"))
_e.append(flow(arc(216, 320, 256, 306, 296, 320), en(), B_HARD, 3, "arc"))
_e.append(flow(line(222, 190, 290, 190), en(), B_HARD, 2, "line"))
_e.append(flow(line(256, 180, 256, 104), en(), B_HARD, 2, "line"))
_e.append(flow(line(154, 44, 256, 96), en(), B_HARD, 3, "line"))
_e.append(flow(line(256, 96, 358, 44), en(), B_HARD, 3, "line"))
_e.append(flow(arc(220, 52, 256, 26, 292, 52), en(), B_HARD, 2, "arc"))
_e.append(flow(line(60, 430, 200, 430), en(), B_HARD, 2, "line"))
_e.append(flow(line(312, 430, 452, 430), en(), B_HARD, 2, "line"))
_e.append(flow(arc(50, 120, 100, 84, 150, 120), en(), B_HARD, 3, "arc"))
_e.append(flow(arc(360, 96, 410, 60, 460, 96), en(), B_HARD, 3, "arc"))
_e.append(flow(arc(86, 54, 120, 28, 154, 54), en(), B_HARD, 3, "arc"))
_e.append(flow(line(44, 462, 214, 462), en(), B_HARD, 3, "line"))
_e.append(flow(line(44, 486, 214, 486), en(), B_HARD, 3, "line"))
_e.append(flow(line(298, 486, 468, 486), en(), B_HARD, 3, "line"))
_e.append(flow(line(298, 462, 468, 462), en(), B_HARD, 3, "line"))
P.append(pic("eiffel", "hard", "landmarks", "🗼", False, _e))

# 10) MONA LISA (expert) — rebuilt: fewer, LONGER strokes (every stroke must
# host an expert-tier word legibly); the eyes live in the brow line and
# negative space — a 12px eye stroke cannot carry a 10-letter word (L10 note
# for Eric); the smile is the final word.
_CUR_TIER[0] = "expert"
_m = []
_o3 = [0]
def mn():
    _o3[0] += 1
    return _o3[0]
_m.append(flow(arc(30, 30, 256, 14, 482, 30), mn(), B_EXP, 1, "line"))
for y in (60, 104, 148, 192, 236, 280):
    _m.append(flow(line(30, y, 112, y - 4), mn(), B_EXP, 3, "line"))
    _m.append(flow(line(400, y - 4, 482, y), mn(), B_EXP, 3, "line"))
for k, x in enumerate((180, 152, 124)):
    _m.append(flow(arc(x, 96 + k * 8, x - 10, 190, x + 4, 284 - k * 6), mn(), B_EXP, 2, "wave"))
for k, x in enumerate((332, 360, 388)):
    _m.append(flow(arc(x, 96 + k * 8, x + 10, 190, x - 4, 284 - k * 6), mn(), B_EXP, 2, "wave"))
_m.append(flow(arc(180, 88, 256, 46, 332, 88), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(208, 128, 200, 200, 216, 264), mn(), B_EXP, 3, "outline"))
_m.append(flow(arc(304, 128, 312, 200, 296, 264), mn(), B_EXP, 3, "outline"))
_m.append(flow(arc(216, 264, 256, 300, 296, 264), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(214, 158, 256, 142, 298, 158), mn(), B_EXP, 3, "arc"))
_m.append(flow(line(256, 166, 256, 240), mn(), B_EXP, 3, "line"))
_m.append(flow(line(224, 288, 224, 362), mn(), B_EXP, 3, "line"))
_m.append(flow(line(288, 288, 288, 362), mn(), B_EXP, 3, "line"))
_m.append(flow(arc(186, 310, 140, 348, 130, 408), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(326, 310, 372, 348, 382, 408), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(208, 344, 256, 368, 304, 344), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(204, 384, 256, 408, 308, 384), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(190, 424, 256, 452, 322, 424), mn(), B_EXP, 2, "arc"))
for x in (66, 92, 118):
    _m.append(flow(arc(x, 426, x - 7, 466, x - 2, 508), mn(), B_EXP, 3, "wave"))
for x in (346, 372, 398):
    _m.append(flow(arc(x, 426, x + 7, 466, x + 2, 508), mn(), B_EXP, 3, "wave"))
_m.append(flow(arc(150, 424, 200, 448, 248, 456), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(248, 456, 292, 464, 330, 452), mn(), B_EXP, 3, "arc"))
_m.append(flow(line(48, 332, 136, 326), mn(), B_EXP, 3, "line"))
_m.append(flow(line(376, 326, 464, 332), mn(), B_EXP, 3, "line"))
_m.append(flow(line(48, 376, 132, 370), mn(), B_EXP, 3, "line"))
_m.append(flow(line(380, 370, 464, 376), mn(), B_EXP, 3, "line"))
_m.append(flow(line(152, 468, 242, 496), mn(), B_EXP, 3, "line"))
_m.append(flow(line(38, 300, 38, 396), mn(), B_EXP, 3, "line"))
_m.append(flow(line(474, 300, 474, 396), mn(), B_EXP, 3, "line"))
_m.append(flow(line(48, 414, 140, 408), mn(), B_EXP, 3, "line"))
_m.append(flow(line(372, 408, 464, 414), mn(), B_EXP, 3, "line"))
_m.append(flow(arc(206, 246, 256, 276, 306, 246), mn(), B_EXP, 4, "arc"))
P.append(pic("mona", "expert", "masterpieces", "🖼️", False, _m,
             prov={
                 "title": "Mona Lisa",
                 "artist": "Leonardo da Vinci (1452–1519)",
                 "source": "Wikimedia Commons",
                 "sourceUrl": "https://commons.wikimedia.org/wiki/Special:FilePath/Mona_Lisa,_by_Leonardo_da_Vinci,_from_C2RMF_retouched.jpg",
                 "pdBasis": "Public domain: painting c. 1503–1506, artist died 1519; stroke map hand-traced at content time over the PD reference (reference image not shipped)",
                 "retrieved": "2026-07-28"}))


# ============================ ANIMALS PACK (D13) ============================
# 4 easy / 3 medium / 2 hard / 1 expert. Every picture its own subject tag
# (D11). Clearance vocabulary: strokes connect only at shared endpoints
# (seam <8px) or stay >0.38x(sizes) apart; tips may approach bodies to
# 0.2x(sizes). The sweep is the law.

_CUR_TIER[0] = "easy"
P.append(pic("dog", "easy", "dog", "🐶", True, [
    flow(arc(166, 210, 256, 118, 346, 210), 1, B_EASY, 2, "outline"),
    flow(arc(166, 210, 256, 290, 346, 210), 2, B_EASY, 2, "outline"),
    flow(arc(150, 108, 106, 168, 136, 238), 3, B_EASY, 2, "arc"),
    flow(arc(362, 108, 406, 168, 376, 238), 4, B_EASY, 2, "arc"),
    flow(arc(236, 196, 256, 210, 276, 196), 5, B_EASY, 3, "arc"),
    flow(arc(238, 306, 256, 322, 274, 306), 6, B_EASY, 3, "arc"),
    flow(line(180, 420, 332, 420), 7, B_EASY, 2, "line"),
], pack="animals"))

P.append(pic("butterfly", "easy", "butterfly", "🦋", True, [
    flow(line(256, 160, 256, 300), 1, B_EASY, 2, "line"),
    flow(arc(226, 170, 130, 100, 110, 220), 2, B_EASY, 2, "arc"),
    flow(arc(110, 240, 130, 360, 226, 300), 3, B_EASY, 2, "arc"),
    flow(arc(286, 170, 382, 100, 402, 220), 4, B_EASY, 2, "arc"),
    flow(arc(402, 240, 382, 360, 286, 300), 5, B_EASY, 2, "arc"),
    flow(line(244, 140, 208, 74), 6, B_EASY, 3, "line"),
    flow(line(268, 140, 304, 74), 7, B_EASY, 3, "line"),
], pack="animals"))

# Round-2 direction (Eric): outline only — outer shell, body, eyes, smile.
P.append(pic("snail", "easy", "snail", "🐌", True, [
    flow("M220 260 A80 80 0 1 1 300 340", 1, B_EASY, 2, "spiral"),
    flow(arc(110, 352, 250, 372, 408, 352), 2, B_EASY, 2, "line"),
    flow(arc(112, 340, 96, 300, 122, 268), 3, B_EASY, 2, "arc"),
    flow(line(118, 262, 90, 208), 4, B_EASY, 2, "line"),
    flow(line(134, 262, 152, 206), 5, B_EASY, 2, "line"),
    flow(arc(76, 200, 92, 184, 108, 200), 6, B_EASY, 4, "arc"),
    flow(arc(136, 198, 152, 182, 168, 198), 7, B_EASY, 4, "arc"),
    flow(arc(122, 298, 144, 312, 138, 338), 8, B_EASY, 4, "arc"),
], pack="animals"))

P.append(pic("duck", "easy", "duck", "🦆", True, [
    flow("M138 200 A46 46 0 0 1 230 200", 1, B_EASY, 2, "arc"),
    flow(line(134, 210, 82, 226), 2, B_EASY, 2, "line"),
    flow(arc(200, 244, 300, 208, 384, 262), 3, B_EASY, 2, "arc"),
    flow(arc(184, 264, 280, 344, 380, 274), 4, B_EASY, 2, "arc"),
    flow(line(60, 384, 200, 384), 5, B_EASY, 3, "line"),
    flow(line(300, 384, 440, 384), 6, B_EASY, 3, "line"),
], pack="animals"))

_CUR_TIER[0] = "medium"
P.append(pic("owl", "medium", "owl", "🦉", True, [
    flow(arc(256, 110, 144, 140, 154, 300), 1, B_MED, 2, "outline"),
    flow(arc(256, 110, 368, 140, 358, 300), 2, B_MED, 2, "outline"),
    flow(arc(154, 300, 256, 400, 358, 300), 3, B_MED, 2, "arc"),
    flow(line(192, 96, 172, 46), 4, B_MED, 3, "line"),
    flow(line(320, 96, 340, 46), 5, B_MED, 3, "line"),
    flow("M206 204 A20 20 0 0 1 246 204", 6, B_MED, 3, "arc"),
    flow("M206 204 A20 20 0 1 0 246 204", 7, B_MED, 3, "arc"),
    flow("M266 204 A20 20 0 0 1 306 204", 8, B_MED, 3, "arc"),
    flow("M266 204 A20 20 0 1 0 306 204", 9, B_MED, 3, "arc"),
    flow(arc(210, 294, 256, 314, 302, 294), 10, B_MED, 3, "arc"),
    flow(line(150, 430, 362, 430), 11, B_MED, 2, "line"),
    flow(arc(60, 84, 96, 56, 132, 84), 12, B_MED, 4, "arc"),
], pack="animals"))

P.append(pic("turtle", "medium", "turtle", "🐢", True, [
    flow(arc(140, 300, 256, 160, 372, 300), 1, B_MED, 2, "outline"),
    flow(arc(140, 300, 256, 330, 372, 300), 2, B_MED, 2, "arc"),
    flow(arc(372, 300, 412, 286, 426, 252), 3, B_MED, 2, "arc"),
    flow(arc(400, 246, 428, 218, 456, 246), 4, B_MED, 3, "arc"),
    flow(line(140, 300, 118, 356), 5, B_MED, 3, "line"),
    flow(line(372, 300, 394, 356), 6, B_MED, 3, "line"),
    flow(line(140, 300, 96, 326), 7, B_MED, 3, "line"),
    flow(tuft(80, 400, 18), 8, B_MED, 4, "wave"),
    flow(tuft(430, 400, 16), 9, B_MED, 4, "wave"),
    flow(arc(60, 80, 100, 52, 140, 80), 10, B_MED, 4, "arc"),
    flow(arc(330, 70, 376, 46, 422, 72), 11, B_MED, 4, "arc"),
], pack="animals"))

P.append(pic("elephant", "medium", "elephant", "🐘", True, [
    flow(arc(120, 220, 260, 150, 390, 230), 1, B_MED, 2, "outline"),
    flow(arc(390, 230, 432, 242, 434, 282), 2, B_MED, 2, "arc"),
    flow(arc(434, 282, 456, 360, 416, 430), 3, B_MED, 2, "wave"),
    flow(arc(302, 238, 272, 276, 302, 314), 4, B_MED, 3, "arc"),
    flow(arc(140, 430, 260, 448, 380, 430), 5, B_MED, 2, "arc"),
    flow(line(176, 330, 184, 406), 6, B_MED, 2, "line"),
    flow(line(350, 332, 358, 404), 7, B_MED, 2, "line"),
    flow(arc(388, 312, 414, 330, 398, 364), 8, B_MED, 3, "arc"),
    flow(arc(120, 226, 92, 280, 110, 330), 9, B_MED, 3, "wave"),
    flow(tuft(70, 442, 18), 10, B_MED, 4, "wave"),
    flow(line(120, 70, 220, 58), 11, B_MED, 4, "line"),
    flow(arc(330, 70, 376, 46, 422, 72), 12, B_MED, 4, "arc"),
], pack="animals"))

_CUR_TIER[0] = "hard"
_h = []
_oh = [0]
def hn():
    _oh[0] += 1
    return _oh[0]
_h.append(flow(arc(150, 240, 250, 206, 330, 240), hn(), B_HARD, 2, "outline"))
_h.append(flow(arc(330, 240, 370, 200, 380, 150), hn(), B_HARD, 2, "arc"))
_h.append(flow(arc(380, 150, 420, 138, 448, 172), hn(), B_HARD, 2, "arc"))
_h.append(flow(arc(448, 172, 458, 202, 422, 214), hn(), B_HARD, 3, "arc"))
_h.append(flow(arc(376, 148, 350, 106, 368, 72), hn(), B_HARD, 4, "arc"))
_h.append(flow(arc(344, 144, 320, 172, 336, 212), hn(), B_HARD, 3, "wave"))
_h.append(flow(arc(320, 330, 344, 290, 334, 244), hn(), B_HARD, 3, "arc"))
_h.append(flow(arc(160, 330, 250, 348, 320, 330), hn(), B_HARD, 2, "arc"))
_h.append(flow(line(170, 364, 166, 436), hn(), B_HARD, 3, "line"))
_h.append(flow(line(206, 364, 202, 436), hn(), B_HARD, 3, "line"))
_h.append(flow(line(266, 364, 262, 436), hn(), B_HARD, 3, "line"))
_h.append(flow(line(298, 364, 294, 436), hn(), B_HARD, 3, "line"))
_h.append(flow(arc(146, 244, 116, 300, 130, 360), hn(), B_HARD, 3, "wave"))
_h.append(flow(arc(118, 252, 88, 306, 102, 366), hn(), B_HARD, 3, "wave"))
_h.append(flow(line(60, 452, 220, 452), hn(), B_HARD, 2, "line"))
_h.append(flow(line(292, 452, 452, 452), hn(), B_HARD, 2, "line"))
_h.append(flow(line(388, 84, 476, 84), hn(), B_HARD, 3, "line"))
_h.append(flow(line(392, 110, 476, 110), hn(), B_HARD, 3, "line"))
_h.append(flow(arc(60, 84, 108, 56, 156, 84), hn(), B_HARD, 4, "arc"))
_h.append(flow(arc(200, 60, 240, 38, 280, 62), hn(), B_HARD, 4, "arc"))
_h.append(flow(arc(64, 124, 100, 110, 136, 124), hn(), B_HARD, 4, "arc"))
_h.append(flow(arc(170, 104, 198, 86, 226, 104), hn(), B_HARD, 4, "arc"))
_h.append(flow(arc(280, 94, 308, 76, 336, 94), hn(), B_HARD, 4, "arc"))
_h.append(flow(line(64, 400, 130, 392), hn(), B_HARD, 4, "line"))
_h.append(flow(line(380, 392, 446, 400), hn(), B_HARD, 4, "line"))
P.append(pic("horse", "hard", "horse", "🐴", True, _h, pack="animals"))

_p2 = []
_op = [0]
def pn():
    _op[0] += 1
    return _op[0]
import math as _mth
for k in range(9):
    a = _mth.pi * (0.16 + 0.68 * k / 8.0)
    _p2.append(flow(line(256 - int(86 * _mth.cos(a)), 330 - int(86 * _mth.sin(a)),
                         256 - int(196 * _mth.cos(a)), 330 - int(196 * _mth.sin(a))), pn(), B_HARD, 3, "radial"))
for k in range(4):
    a0 = _mth.pi * (0.16 + 0.68 * (2 * k) / 8.0)
    a1 = _mth.pi * (0.16 + 0.68 * (2 * k + 2) / 8.0)
    am = (a0 + a1) / 2
    _p2.append(flow(arc(256 - int(212 * _mth.cos(a0)), 330 - int(212 * _mth.sin(a0)),
                        256 - int(232 * _mth.cos(am)), 330 - int(232 * _mth.sin(am)),
                        256 - int(212 * _mth.cos(a1)), 330 - int(212 * _mth.sin(a1))), pn(), B_HARD, 3, "arc"))
_p2.append(flow(arc(127, 107, 189, 78, 256, 72), pn(), B_HARD, 4, "arc"))
_p2.append(flow(arc(256, 72, 323, 78, 385, 107), pn(), B_HARD, 4, "arc"))
_p2.append(flow(arc(226, 330, 256, 312, 286, 330), pn(), B_HARD, 3, "arc"))
_p2.append(flow(arc(226, 336, 236, 400, 256, 428), pn(), B_HARD, 2, "arc"))
_p2.append(flow(arc(256, 428, 276, 400, 286, 336), pn(), B_HARD, 2, "arc"))
_p2.append(flow("M236 300 A20 20 0 0 1 276 300", pn(), B_HARD, 3, "arc"))
_p2.append(flow(arc(40, 190, 74, 148, 116, 114), pn(), B_HARD, 4, "arc"))
_p2.append(flow(arc(396, 114, 438, 148, 472, 190), pn(), B_HARD, 4, "arc"))
_p2.append(flow(line(150, 448, 362, 448), pn(), B_HARD, 2, "line"))
_p2.append(flow(line(70, 478, 200, 474), pn(), B_HARD, 4, "line"))
_p2.append(flow(line(312, 474, 442, 478), pn(), B_HARD, 4, "line"))
_p2.append(flow(arc(96, 388, 140, 366, 182, 392), pn(), B_HARD, 4, "arc"))
_p2.append(flow(arc(330, 392, 372, 366, 416, 388), pn(), B_HARD, 4, "arc"))
P.append(pic("peacock", "hard", "peacock", "🦚", True, _p2, pack="animals"))

# EXPERT — Dürer's Rhinoceros (1515), hand-traced over the PD reference
# (content-pipeline/wordpic/ref/durer-rhinoceros.jpg — never ships). Armor
# plates carry the composition as parallel rivet-arc families; the HORN is
# the final word.
_CUR_TIER[0] = "expert"
_r = []
_orh = [0]
def rn():
    _orh[0] += 1
    return _orh[0]
_r.append(flow(line(60, 40, 250, 34), rn(), B_EXP, 2, "line"))
_r.append(flow(line(266, 34, 452, 40), rn(), B_EXP, 2, "line"))
_r.append(flow(line(60, 68, 200, 64), rn(), B_EXP, 2, "line"))
_r.append(flow(line(312, 64, 452, 68), rn(), B_EXP, 2, "line"))
_r.append(flow(arc(120, 180, 250, 128, 400, 168), rn(), B_EXP, 2, "outline"))
_r.append(flow(arc(400, 168, 452, 198, 440, 268), rn(), B_EXP, 2, "arc"))
_r.append(flow(arc(440, 268, 466, 340, 446, 428), rn(), B_EXP, 3, "wave"))
_r.append(flow(arc(150, 330, 270, 350, 392, 330), rn(), B_EXP, 2, "arc"))
_r.append(flow(line(162, 358, 168, 444), rn(), B_EXP, 3, "line"))
_r.append(flow(line(208, 364, 214, 446), rn(), B_EXP, 3, "line"))
_r.append(flow(line(330, 366, 336, 446), rn(), B_EXP, 3, "line"))
_r.append(flow(line(374, 360, 380, 444), rn(), B_EXP, 3, "line"))
_r.append(flow(arc(120, 180, 74, 224, 94, 290), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(94, 292, 130, 314, 170, 320), rn(), B_EXP, 3, "arc"))
_r.append(flow(line(90, 344, 164, 366), rn(), B_EXP, 4, "line"))
_r.append(flow(line(156, 146, 114, 82), rn(), B_EXP, 3, "line"))
_r.append(flow(line(240, 114, 282, 48), rn(), B_EXP, 3, "line"))
_r.append(flow("M110 212 Q146 224 140 254 Q136 278 150 288", rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(168, 196, 206, 242, 178, 306), rn(), B_EXP, 2, "arc"))
_r.append(flow(arc(196, 202, 232, 244, 206, 304), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(224, 208, 258, 246, 234, 302), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(288, 160, 300, 234, 286, 322), rn(), B_EXP, 2, "arc"))
_r.append(flow(arc(316, 162, 328, 234, 314, 320), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(352, 170, 372, 234, 356, 320), rn(), B_EXP, 2, "arc"))
_r.append(flow(arc(380, 176, 398, 238, 384, 316), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(404, 182, 424, 238, 410, 314), rn(), B_EXP, 3, "arc"))
_r.append(flow(arc(406, 328, 434, 352, 418, 398), rn(), B_EXP, 4, "arc"))
_r.append(flow(line(232, 374, 312, 376), rn(), B_EXP, 4, "line"))
_r.append(flow(line(232, 404, 312, 407), rn(), B_EXP, 4, "line"))
_r.append(flow(line(244, 434, 322, 437), rn(), B_EXP, 4, "line"))
_r.append(flow(line(60, 470, 240, 470), rn(), B_EXP, 2, "line"))
_r.append(flow(line(276, 470, 452, 470), rn(), B_EXP, 2, "line"))
_r.append(flow(arc(48, 118, 82, 100, 118, 118), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(48, 144, 82, 128, 118, 144), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(378, 118, 414, 100, 450, 118), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(380, 144, 416, 130, 452, 144), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(154, 114, 190, 90, 226, 114), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(306, 114, 342, 90, 378, 114), rn(), B_EXP, 4, "arc"))
_r.append(flow(arc(92, 292, 62, 322, 44, 368), rn(), B_EXP, 2, "arc"))
P.append(pic("rhino", "expert", "rhinoceros", "🦏", False, _r,
             prov={
                 "title": "The Rhinoceros",
                 "artist": "Albrecht Dürer (1471–1528)",
                 "source": "Wikimedia Commons",
                 "sourceUrl": "https://commons.wikimedia.org/wiki/Special:FilePath/D%C3%BCrer's%20Rhinoceros,%201515.jpg",
                 "pdBasis": "Public domain: woodcut 1515, artist died 1528; stroke map hand-traced at content time over the PD reference (reference image not shipped)",
                 "retrieved": "2026-07-29"},
             pack="animals"))

BANDS = {"easy": (3, 8), "medium": (10, 20), "hard": (20, 45), "expert": (38, 200)}


def main():
    for p in P:
        n = len(p["paths"])
        lo, hi = BANDS[p["tier"]]
        assert lo <= n <= hi, f"{p['id']}: {n} paths outside {p['tier']} band {lo}-{hi}"
        orders = [q["order"] for q in p["paths"]]
        assert orders == list(range(1, n + 1)), f"{p['id']}: fill order not 1..{n}: {orders}"
    manifest = {
        "$doc": "CC-WORD-PICTURE v5 starter pack — calligram stroke maps. Words typeset along paths; outline guides from word 1; focal stroke last (D6).",
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
