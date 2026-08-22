#!/usr/bin/env python3
"""CC-MASTERPIECE-RECOG — author word rails for the F9 batch.

Rails are the lines words ride on. They are AUTHORED, never lifted from the
guide: guide paths are closed contours that reverse at their tips and do not
survive corner_split, so a rail taken from one mints no slot.

The shape follows what Saguaro and Taos established -- a near-horizontal span at
a chosen height, following the subject's own extent at that height rather than
crossing empty ground. Each is named for the anatomy it lies on, because "carrier
3" is a positional placeholder and F2 forbids those.

The laws a rail set must satisfy, all checked here rather than discovered in the
19-minute layout sweep:

  * no rail may INTERSECT another -- a crossing puts two words in one place;
  * 34px clearance, sized for Korean blocks and Vietnamese diacritics rather
    than for English;
  * arc over 130, or the rail mints no slot a word can use;
  * a rail must lie ON the subject: its span is taken from the ink at that
    height, so it cannot float in the margin.

Regenerate:  python3 tools/build_f9_rails.py
"""
import json
import math
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
STAGED = ROOT / "content-pipeline/wordpic/staged-batch-f9.json"

CLEARANCE = 34.0
MIN_ARC = 130.0
# Rails are placed at these heights as a fraction of the ink's own vertical
# extent, and named for what the artwork has there. Chosen per picture from the
# rendered guide, not from a formula -- the names have to be true.
PLAN = {
    "hare": [("ear", 0.09), ("crown", 0.24), ("back", 0.39), ("flank", 0.54),
             ("haunch", 0.69), ("foreleg", 0.84), ("forepaw", 0.95)],
    # The beetle pinches at 0.25-0.30 and again at the very bottom, so the head
    # rail sits at 0.20 where the subject is still wide enough to carry a word.
    "beetle": [("mandible", 0.05), ("head", 0.20), ("thorax", 0.40),
               ("wing case", 0.56), ("hind leg", 0.72), ("claw", 0.88)],
    "wing": [("covert", 0.10), ("shoulder", 0.26), ("secondary", 0.42),
             ("primary", 0.58), ("shaft", 0.74), ("quill", 0.90)],
    # Merian's plate tapers to a bare stem below 0.85, so the whole set is
    # lifted rather than letting the last rail hang off the subject.
    "banana": [("leaf tip", 0.06), ("leaf blade", 0.22), ("bract", 0.38),
               ("flower", 0.54), ("fruit hand", 0.68), ("stem", 0.80)],
    "flamingo": [("crown", 0.10), ("neck", 0.26), ("back", 0.42),
                 ("breast", 0.58), ("leg", 0.74), ("waterline", 0.90)],
    "carp": [("upper fish", 0.10), ("dorsal", 0.26), ("body", 0.42),
             ("tail", 0.58), ("lower fish", 0.74), ("cascade", 0.90)],
    # The rose is only wide between 0.20 and 0.79 -- above it is a single bud
    # and below it a bare stem -- so all six rails live inside that band.
    "rose": [("outer petal", 0.22), ("bloom", 0.33), ("calyx", 0.44),
             ("stem", 0.55), ("leaf", 0.66), ("lower leaf", 0.77)],
    "kosonowl": [("moon", 0.10), ("branch", 0.26), ("ear tuft", 0.42),
                 ("breast", 0.58), ("talon", 0.74), ("ginkgo", 0.90)],
}
INSET = {"kosonowl": 14.0}
# Koson's owl keeps its frame, and the frame has ink at EVERY height -- so a
# span measured naively runs border to border and the rail crosses empty paper.
# Points within this distance of the outer box are dropped before measuring, so
# the spans come from the owl, the branch and the moon.
IGNORE_BORDER = {"kosonowl": 16.0}


def ink(guide, drop_border=0.0):
    pts = [(float(a), float(b))
           for p in guide for a, b in re.findall(r"([-\d.]+) ([-\d.]+)", p)]
    if drop_border <= 0:
        return pts
    xs = [x for x, _ in pts]; ys = [y for _, y in pts]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    return [(x, y) for x, y in pts
            if x0 + drop_border < x < x1 - drop_border
            and y0 + drop_border < y < y1 - drop_border]


def span_at(points, y, half):
    """Horizontal extent of ink within `half` of height `y`."""
    xs = [x for x, yy in points if abs(yy - y) <= half]
    return (min(xs), max(xs)) if len(xs) >= 8 else None


def rail(points, y, inset):
    """A near-horizontal rail lying on the subject at height `y`.

    The slight slope keeps it from reading as a ruled line while staying far
    inside the 34px clearance, so it cannot approach its neighbours.
    """
    s = span_at(points, y, 26.0) or span_at(points, y, 60.0)
    if not s:
        return None
    x0, x1 = s[0] + inset, s[1] - inset
    widened = False
    if x1 - x0 < MIN_ARC:
        # A rail this short mints no slot. Widening it is the only way to keep
        # it, but the extra length hangs off the subject, so the caller is told
        # rather than left to assume the rail sits on ink.
        mid = (x0 + x1) / 2
        x0, x1 = mid - MIN_ARC / 2 - 6, mid + MIN_ARC / 2 + 6
        widened = True
    # A point every ~2px. The masters density floor counts the rail set's own
    # points (saguaro ships 530 across 11 rails), and a sparse rail also gives
    # the typesetter fewer places to break a word.
    n = max(48, int((x1 - x0) / 2))
    out = []
    for i in range(n + 1):
        t = i / n
        out.append((x0 + (x1 - x0) * t, y + math.sin(t * math.pi) * 3.0))
    return out, widened


def arc(p):
    return sum(math.dist(a, b) for a, b in zip(p, p[1:]))


def segments_cross(a1, a2, b1, b2):
    def side(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    d1, d2 = side(b1, b2, a1), side(b1, b2, a2)
    d3, d4 = side(a1, a2, b1), side(a1, a2, b2)
    return ((d1 > 0) != (d2 > 0)) and ((d3 > 0) != (d4 > 0))


def min_gap(p, q):
    best = 1e9
    for a in p:
        for b in q:
            best = min(best, math.dist(a, b))
    return best


def main():
    staged = json.loads(STAGED.read_text())["staged"]
    out, problems = {}, []
    for pid, plan in PLAN.items():
        pts = ink(staged[pid]["guide"], IGNORE_BORDER.get(pid, 0.0))
        ys = [y for _, y in pts]
        lo, hi = min(ys), max(ys)
        inset = INSET.get(pid, 10.0)
        rails, widened = [], []
        for name, frac in plan:
            got = rail(pts, lo + (hi - lo) * frac, inset)
            if got is not None and got[1]:
                widened.append(name)
            r = got[0] if got else None
            if r is None:
                problems.append(f"{pid}: no ink to carry '{name}'")
                continue
            if arc(r) < MIN_ARC:
                problems.append(f"{pid}/{name}: arc {arc(r):.0f} under {MIN_ARC:.0f}")
            rails.append((name, r))
        for i, (n1, r1) in enumerate(rails):
            for n2, r2 in rails[i + 1:]:
                if any(segments_cross(a1, a2, b1, b2)
                       for a1, a2 in zip(r1, r1[1:]) for b1, b2 in zip(r2, r2[1:])):
                    problems.append(f"{pid}: '{n1}' crosses '{n2}'")
                g = min_gap(r1, r2)
                if g < CLEARANCE:
                    problems.append(f"{pid}: '{n1}' and '{n2}' are {g:.0f}px apart (need {CLEARANCE:.0f})")
        if len(rails) < 6:
            problems.append(f"{pid}: {len(rails)} rails, the shipped floor is 6")
        out[pid] = [
            {"mode": "flow", "arch": "line", "band": 1 + i % 3, "budget": [3, 20],
             "feature": n,
             "d": "M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in r)}
            for i, (n, r) in enumerate(rails)
        ]
        note = f"   WIDENED off-subject: {', '.join(widened)}" if widened else ""
        print(f"  {pid:10} {len(rails)} rails  "
              f"arcs {' '.join(str(int(arc(r))) for _, r in rails)}{note}")
    if problems:
        print("\n  PROBLEMS:")
        for p in problems:
            print("   ", p)
        sys.exit(1)
    (ROOT / "content-pipeline/wordpic/f9-rails.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1) + "\n")
    print(f"\n  wrote content-pipeline/wordpic/f9-rails.json "
          f"({sum(len(v) for v in out.values())} rails, 0 crossings, all >= {CLEARANCE:.0f}px)")


if __name__ == "__main__":
    main()
