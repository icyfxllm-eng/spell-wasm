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
    stack(56, 64, 24, 3, 2),
    stack(456, 80, 22, 3, 3),
    flow(line(150, 486, 362, 486), 4, B_EASY, 2, "line"),
]))

# 3) FISH — Eric's golden. Body + tail unchanged in spirit; the interior fin
# (text can't host there) is gone; boat slimmed to hull + mast stack.
P.append(pic("fish", "easy", "animals", "🐟", True, [
    flow(line(40, 132, 224, 132), 1, B_EASY, 2, "line"),
    flow(line(288, 132, 472, 132), 2, B_EASY, 2, "line"),
    flow(line(200, 100, 312, 100), 3, B_EASY, 2, "line"),
    stack(256, 44, 18, 3, 4),
    flow(arc(120, 300, 256, 176, 392, 300), 5, B_EASY, 1, "outline"),
    flow(arc(120, 300, 256, 424, 392, 300), 6, B_EASY, 1, "outline"),
    flow(line(404, 291, 474, 238), 7, B_EASY, 2, "line"),
    flow(line(404, 309, 474, 362), 8, B_EASY, 2, "line"),
]))

# 4) HOUSE.
P.append(pic("house", "easy", "objects", "🏠", True, [
    flow(line(120, 252, 256, 120), 1, B_EASY, 1, "line"),
    flow(line(256, 120, 392, 252), 2, B_EASY, 1, "line"),
    stack(150, 300, 26, 3, 3),
    stack(362, 300, 26, 3, 4),
    flow(line(120, 434, 392, 434), 5, B_EASY, 1, "line"),
    flow(arc(230, 418, 256, 358, 282, 418), 6, B_EASY, 3, "arc"),
    flow(arc(52, 96, 92, 56, 132, 96), 7, B_EASY, 2, "arc"),
]))

# 5) CAT — full redo (Eric: "crammed"). Small strokes, small words, air.
_CUR_TIER[0] = "medium"
P.append(pic("cat", "medium", "animals", "🐱", True, [
    flow("M161 200 A95 95 0 0 1 351 200", 1, B_MED, 1, "outline"),
    flow("M161 200 A95 95 0 1 0 351 200", 2, B_MED, 1, "outline"),
    flow(line(191, 133, 151, 48), 3, B_MED, 2, "line"),
    flow(line(321, 133, 361, 48), 4, B_MED, 2, "line"),
    flow(arc(196, 182, 222, 168, 248, 182), 5, B_MED, 3, "arc"),
    flow(arc(264, 182, 290, 168, 316, 182), 6, B_MED, 3, "arc"),
    flow(arc(228, 252, 256, 276, 284, 252), 7, B_MED, 3, "arc"),
    flow(line(58, 204, 128, 214), 8, B_MED, 3, "line"),
    flow(line(384, 214, 454, 204), 9, B_MED, 3, "line"),
    flow(arc(200, 288, 140, 360, 166, 446), 10, B_MED, 2, "arc"),
    flow(arc(312, 288, 372, 360, 346, 446), 11, B_MED, 2, "arc"),
    flow(arc(166, 446, 256, 470, 346, 446), 12, B_MED, 2, "arc"),
    flow(arc(346, 446, 460, 430, 448, 330), 13, B_MED, 2, "wave"),
]))

# 6) ROCKET — the short left side fills; flame simplified to hostable strokes.
P.append(pic("rocket", "medium", "vehicles", "🚀", True, [
    stack(226, 170, 24, 6, 1),
    stack(292, 170, 24, 6, 2),
    flow(arc(226, 160, 256, 66, 292, 160), 3, B_MED, 1, "arc"),
    flow(line(218, 322, 156, 424), 4, B_MED, 2, "line"),
    flow(line(156, 424, 212, 408), 5, B_MED, 2, "line"),
    flow(line(300, 322, 364, 424), 6, B_MED, 2, "line"),
    flow(line(364, 424, 308, 408), 7, B_MED, 2, "line"),
    flow(line(240, 338, 278, 338), 8, B_MED, 3, "line"),
    flow(line(240, 364, 278, 364), 9, B_MED, 3, "line"),
    flow(arc(226, 436, 256, 498, 292, 436), 10, B_MED, 2, "arc"),
    flow(line(212, 432, 186, 484), 11, B_MED, 3, "line"),
    flow(line(306, 432, 332, 484), 12, B_MED, 3, "line"),
    flow(arc(52, 116, 84, 84, 116, 116), 13, B_MED, 2, "arc"),
    flow(line(392, 76, 446, 112), 14, B_MED, 3, "line"),
    flow(line(120, 60, 172, 92), 15, B_MED, 3, "line"),
]))

# 7) SNOWMAN.
P.append(pic("snowman", "medium", "holidays", "⛄", True, [
    flow("M176 360 A80 80 0 0 1 336 360", 1, B_MED, 1, "outline"),
    flow("M176 360 A80 80 0 1 0 336 360", 2, B_MED, 1, "outline"),
    flow("M201 205 A55 55 0 0 1 311 205", 3, B_MED, 1, "arc"),
    flow("M201 205 A55 55 0 1 0 311 205", 4, B_MED, 1, "arc"),
    flow(line(204, 128, 308, 128), 5, B_MED, 2, "line"),
    flow(line(222, 62, 290, 62), 6, B_MED, 2, "line"),
    flow(line(222, 95, 290, 95), 16, B_MED, 3, "line"),
    flow(arc(216, 190, 234, 180, 252, 190), 7, B_MED, 3, "arc"),
    flow(arc(260, 190, 278, 180, 296, 190), 8, B_MED, 3, "arc"),
    flow(arc(230, 232, 256, 244, 282, 232), 9, B_MED, 3, "arc"),
    flow(line(256, 300, 256, 388), 10, B_MED, 3, "line"),
    flow(line(176, 330, 92, 288), 11, B_MED, 2, "line"),
    flow(line(336, 330, 420, 288), 12, B_MED, 2, "line"),
    flow(arc(60, 474, 256, 458, 452, 474), 13, B_MED, 2, "line"),
    flow(bird(96, 84, 12), 14, B_MED, 3, "wave"),
    flow(bird(420, 110, 11), 15, B_MED, 3, "wave"),
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
_d.append(flow(arc(408, 280, 400, 330, 350, 342), dn(), B_HARD, 1, "arc"))
_d.append(flow(arc(160, 388, 256, 400, 344, 352), dn(), B_HARD, 2, "arc"))
_d.append(flow(line(252, 218, 212, 120), dn(), B_HARD, 2, "radial"))
_d.append(flow(line(270, 216, 262, 108), dn(), B_HARD, 2, "radial"))
_d.append(flow(line(288, 218, 314, 118), dn(), B_HARD, 2, "radial"))
_d.append(flow(arc(212, 120, 262, 84, 314, 118), dn(), B_HARD, 2, "arc"))
_d.append(flow(arc(150, 380, 66, 336, 78, 238), dn(), B_HARD, 2, "wave"))
_d.append(flow(line(78, 238, 40, 192), dn(), B_HARD, 2, "line"))
_d.append(flow(line(40, 192, 104, 178), dn(), B_HARD, 2, "line"))
_d.append(flow(arc(352, 268, 382, 252, 412, 268), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(420, 306, 480, 288), dn(), B_HARD, 3, "line"))
_d.append(flow(line(424, 322, 492, 322), dn(), B_HARD, 3, "line"))
_d.append(flow(line(420, 338, 480, 356), dn(), B_HARD, 3, "line"))
_d.append(flow(line(20, 448, 64, 376), dn(), B_HARD, 2, "line"))
_d.append(flow(line(64, 376, 108, 448), dn(), B_HARD, 2, "line"))
_d.append(flow(arc(140, 452, 300, 440, 482, 452), dn(), B_HARD, 2, "line"))
_d.append(flow(line(320, 396, 470, 396), dn(), B_HARD, 3, "line"))
_d.append(flow(arc(150, 64, 196, 40, 242, 64), dn(), B_HARD, 3, "arc"))
_d.append(flow(arc(330, 80, 366, 60, 402, 82), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(360, 40, 420, 66), dn(), B_HARD, 3, "line"))
_d.append(flow(arc(330, 190, 358, 176, 386, 204), dn(), B_HARD, 3, "arc"))
_d.append(flow(line(126, 128, 186, 96), dn(), B_HARD, 3, "line"))
P.append(pic("dragon", "hard", "fantasy", "🐲", True, _d))

# 9) EIFFEL (hard) — single diagonals, no crossing braces.
_e = []
_o2 = [0]
def en():
    _o2[0] += 1
    return _o2[0]
_e.append(flow(arc(150, 400, 180, 320, 224, 292), en(), B_HARD, 1, "outline"))
_e.append(flow(arc(362, 400, 332, 320, 288, 292), en(), B_HARD, 1, "outline"))
_e.append(flow(arc(178, 400, 256, 330, 334, 400), en(), B_HARD, 1, "arc"))
_e.append(flow(line(186, 284, 326, 284), en(), B_HARD, 1, "line"))
_e.append(flow(line(212, 284, 236, 196), en(), B_HARD, 2, "line"))
_e.append(flow(line(300, 284, 276, 196), en(), B_HARD, 2, "line"))
_e.append(flow(line(216, 256, 296, 256), en(), B_HARD, 3, "line"))
_e.append(flow(line(222, 226, 290, 226), en(), B_HARD, 3, "line"))
_e.append(flow(line(228, 192, 284, 192), en(), B_HARD, 2, "line"))
_e.append(flow(line(238, 188, 252, 112), en(), B_HARD, 2, "line"))
_e.append(flow(line(274, 188, 260, 112), en(), B_HARD, 2, "line"))
_e.append(flow(line(154, 44, 256, 96), en(), B_HARD, 3, "line"))
_e.append(flow(line(256, 96, 358, 44), en(), B_HARD, 3, "line"))
_e.append(flow(arc(236, 74, 256, 58, 276, 74), en(), B_HARD, 2, "arc"))
_e.append(flow(line(60, 430, 200, 430), en(), B_HARD, 2, "line"))
_e.append(flow(line(312, 430, 452, 430), en(), B_HARD, 2, "line"))
_e.append(flow(arc(50, 120, 100, 84, 150, 120), en(), B_HARD, 3, "arc"))
_e.append(flow(arc(360, 96, 410, 60, 460, 96), en(), B_HARD, 3, "arc"))
_e.append(flow(pine(84, 400, 84), en(), B_HARD, 2, "outline"))
_e.append(flow(pine(430, 400, 76), en(), B_HARD, 2, "outline"))
_e.append(flow(bird(180, 150, 12), en(), B_HARD, 3, "wave"))
_e.append(flow(bird(330, 130, 10), en(), B_HARD, 3, "wave"))
_e.append(flow(arc(96, 56, 120, 36, 144, 56), en(), B_HARD, 3, "arc"))
_e.append(flow(line(64, 462, 194, 462), en(), B_HARD, 3, "line"))
_e.append(flow(line(318, 462, 448, 462), en(), B_HARD, 3, "line"))
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
    _m.append(flow(line(30, y, 170, y - 4), mn(), B_EXP, 1, "line"))
    _m.append(flow(line(342, y - 4, 482, y), mn(), B_EXP, 1, "line"))
for k, x in enumerate((186, 157, 128)):
    _m.append(flow(arc(x, 96 + k * 8, x - 10, 190, x + 4, 284 - k * 6), mn(), B_EXP, 2, "wave"))
for k, x in enumerate((326, 355, 384)):
    _m.append(flow(arc(x, 96 + k * 8, x + 10, 190, x - 4, 284 - k * 6), mn(), B_EXP, 2, "wave"))
_m.append(flow(arc(196, 84, 256, 56, 316, 84), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(208, 128, 200, 200, 216, 264), mn(), B_EXP, 3, "outline"))
_m.append(flow(arc(304, 128, 312, 200, 296, 264), mn(), B_EXP, 3, "outline"))
_m.append(flow(arc(216, 264, 256, 300, 296, 264), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(222, 156, 256, 142, 290, 156), mn(), B_EXP, 3, "arc"))
_m.append(flow(line(256, 172, 256, 232), mn(), B_EXP, 3, "line"))
_m.append(flow(line(224, 292, 224, 356), mn(), B_EXP, 3, "line"))
_m.append(flow(line(288, 292, 288, 356), mn(), B_EXP, 3, "line"))
_m.append(flow(arc(186, 310, 140, 348, 130, 408), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(326, 310, 372, 348, 382, 408), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(234, 352, 256, 366, 278, 352), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(204, 384, 256, 408, 308, 384), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(190, 424, 256, 452, 322, 424), mn(), B_EXP, 2, "arc"))
for x in (150, 174, 198, 222, 246):
    _m.append(flow(arc(x, 452, x - 5, 472, x - 2, 492), mn(), B_EXP, 2, "wave"))
for x in (270, 294, 318, 342, 366):
    _m.append(flow(arc(x, 452, x + 5, 472, x + 2, 492), mn(), B_EXP, 2, "wave"))
_m.append(flow(arc(150, 424, 200, 448, 248, 456), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(248, 456, 292, 464, 330, 452), mn(), B_EXP, 3, "arc"))
_m.append(flow(line(30, 330, 130, 324), mn(), B_EXP, 1, "line"))
_m.append(flow(line(382, 324, 482, 330), mn(), B_EXP, 1, "line"))
_m.append(flow(line(30, 374, 118, 370), mn(), B_EXP, 1, "line"))
_m.append(flow(line(394, 370, 482, 374), mn(), B_EXP, 1, "line"))
_m.append(flow(arc(196, 74, 256, 44, 316, 74), mn(), B_EXP, 2, "arc"))
_m.append(flow(arc(226, 288, 256, 302, 286, 288), mn(), B_EXP, 3, "arc"))
_m.append(flow(arc(230, 250, 256, 266, 282, 250), mn(), B_EXP, 4, "arc"))
P.append(pic("mona", "expert", "masterpieces", "🖼️", False, _m,
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
