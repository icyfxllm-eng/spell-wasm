import os
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


# ---- v7.5 CURATED TRACES (Eric passed all 13 on outline review) ----
# Source: content-pipeline/wordpic/curated-traces-v75.json (from the ink
# tracer; fish stays frozen per D2). Paths are polylines (M/L) riding the
# reference's own ink. The sweep remains the law.
import json as _json
_CUR = _json.load(open(os.path.join(os.path.dirname(__file__), "..",
    "content-pipeline", "wordpic", "curated-traces-v75.json")))

def _curated_guide(sub):
    g = _CUR[sub].get("guide", [])
    return ["M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in path) for path in g]

def _curated_paths(sub, budget):
    out = []
    for i, q in enumerate(_CUR[sub]["paths"]):
        dstr = "M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in q["points"])
        out.append(flow(dstr, i + 1, budget, q["band"], "line"))
    return out

def _curated_features(sub):
    return [q["feature"] for q in _CUR[sub]["paths"]]

def tuft(x, y, h):
    """Small grass tuft: a shallow ~55px arc, the minimum fillable stroke."""
    return arc(x - 28, y, x, y - h, x + 28, y)


# v7 F2 — stroke attribution: every path names the feature it draws. A
# stroke nobody can name is noise (round-2 lesson: fish blob, tower sides).
BANNED_FEATURES = {"decorative", "filler", "background texture"}
FEATURES = {
    "smiley": ["head", "head", "left eye", "right eye", "smile"],
    "star": ["star outline"],
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


def pic(pid, tier, subject, icon, kid, paths, wash=False, prov=None, free_hint=None, pack="starter", guide=None):
    ordered = sorted(paths, key=lambda p: p["order"])
    for k, q in enumerate(ordered):
        q["order"] = k + 1  # normalize: authoring gaps are fine, output is 1..n
    feats = FEATURES[pid]
    assert len(feats) == len(paths), f"{pid}: {len(paths)} paths but {len(feats)} feature labels"
    for q, f in zip(ordered, feats):
        assert f and f.lower() not in BANNED_FEATURES, f"{pid}: banned/empty feature {f!r}"
        q["feature"] = f
    d = {"id": pid, "tier": tier, "subject": subject, "icon": icon, "kid": kid,
         "wash": wash, "pack": pack, "paths": ordered,
         "guide": guide if guide is not None else _curated_guide(pid) if pid in _CUR else []}
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
# v8 F6 (Eric could not identify these on device): the two vertical
# letter-stacks flanking the star read as dashed UI rails and the ground
# line as a stray rule. A star is a star — deleted, per D7 and the same
# reasoning that removed the fish blob (D2) and tower sides (D3).
P.append(pic("star", "easy", "nature", "🌟", True, [
    flow(star_outline(256, 250, 180), 1, B_EASY, 1, "outline"),
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
FEATURES["snowman"] = _curated_features("snowman")
P.append(pic("snowman", "medium", "holidays", "⛄", True, _curated_paths("snowman", B_MED)))

# 8) DRAGON (hard) — fanned ribs, spread fire, no interior spikes.
_CUR_TIER[0] = "hard"
FEATURES["dragon"] = _curated_features("dragon")
P.append(pic("dragon", "hard", "fantasy", "🐲", True, _curated_paths("dragon", B_HARD)))

# 9) EIFFEL (hard) — single diagonals, no crossing braces.
FEATURES["eiffel"] = _curated_features("eiffel")
P.append(pic("eiffel", "hard", "landmarks", "🗼", False, _curated_paths("eiffel", B_HARD)))

# 10) MONA LISA (expert) — rebuilt: fewer, LONGER strokes (every stroke must
# host an expert-tier word legibly); the eyes live in the brow line and
# negative space — a 12px eye stroke cannot carry a 10-letter word (L10 note
# for Eric); the smile is the final word.
_CUR_TIER[0] = "expert"
_CUR_TIER[0] = "expert"
FEATURES["mona"] = _curated_features("mona")
P.append(pic("mona", "expert", "masterpieces", "🖼️", False, _curated_paths("mona", B_EXP),
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
_CUR_TIER[0] = "easy"
FEATURES["dog"] = _curated_features("dog")
P.append(pic("dog", "easy", "dog", "🐶", True, _curated_paths("dog", B_EASY), pack="animals"))

_CUR_TIER[0] = "easy"
FEATURES["butterfly"] = _curated_features("butterfly")
P.append(pic("butterfly", "easy", "butterfly", "🦋", True, _curated_paths("butterfly", B_EASY), pack="animals"))

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

_CUR_TIER[0] = "easy"
FEATURES["duck"] = _curated_features("duck")
P.append(pic("duck", "easy", "duck", "🦆", True, _curated_paths("duck", B_EASY), pack="animals"))

_CUR_TIER[0] = "medium"
_CUR_TIER[0] = "medium"
FEATURES["owl"] = _curated_features("owl")
P.append(pic("owl", "medium", "owl", "🦉", True, _curated_paths("owl", B_MED), pack="animals"))

_CUR_TIER[0] = "easy"
FEATURES["turtle"] = _curated_features("turtle")
P.append(pic("turtle", "easy", "turtle", "🐢", True, _curated_paths("turtle", B_EASY), pack="animals"))

_CUR_TIER[0] = "medium"
FEATURES["elephant"] = _curated_features("elephant")
P.append(pic("elephant", "medium", "elephant", "🐘", True, _curated_paths("elephant", B_MED), pack="animals"))

_CUR_TIER[0] = "hard"
_CUR_TIER[0] = "hard"
FEATURES["horse"] = _curated_features("horse")
P.append(pic("horse", "hard", "horse", "🐴", True, _curated_paths("horse", B_HARD), pack="animals"))

FEATURES["peacock"] = _curated_features("peacock")
P.append(pic("peacock", "hard", "peacock", "🦚", True, _curated_paths("peacock", B_HARD), pack="animals"))

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

BANDS = {"easy": (1, 8), "medium": (1, 20), "hard": (1, 45), "expert": (1, 200)}


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
