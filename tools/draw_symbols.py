#!/usr/bin/env python3
"""CC-PICTURE-BANK-SYMBOLS — the GEOMETRIC lane, centerline-emitting.

Twelve constructions, not drawings. Every subject is defined by
compass-and-straightedge relationships, so it is authored correctly by
specification rather than judged by eye — which is why this lane went
first (D3, revised: the Zodiac pack the file proposed already shipped).

The construction emits its own CENTERLINES. An earlier version drew
line art and let the tracer find paths, which was backwards: the tracer
outlines ink, so every stroke became a pair of parallel edges and the
words ended up running around the OUTSIDE of the artwork — on the Sri
Yantra's outer ring instead of on the nine triangles that make it a Sri
Yantra. We already know every line; emit it.

Writes reference PNGs into content-pipeline/wordpic/ref/ so the subjects
can enter the SCAN-STACK pipeline like the rest of the bank, and
centerlines.json beside them for anything that wants the exact geometry.

Provenance: original mathematical constructions. No photo, no scan, no
third-party art — the PD-only gate is satisfied by construction and the
book-art firewall is trivially clean, because this file reproduces them.
"""
import json, math, pathlib
from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parents[1]
REF = ROOT / "content-pipeline/wordpic/ref"
REF.mkdir(parents=True, exist_ok=True)
BLACK, WHITE = 0, 255
SIZE = 1000
STROKE_W = 40
C = SIZE / 2


class Sketch:
    """Collects centerlines, then renders them — one source of truth for
    both the picture and its paths."""

    def __init__(self):
        self.lines = []

    def seg(self, a, b):
        self.lines.append([tuple(a), tuple(b)])

    def arc(self, cx, cy, r, a0, a1, steps=64):
        self.lines.append([
            (cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / steps)),
             cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / steps)))
            for i in range(steps + 1)])

    def circle_arcs(self, cx, cy, r, cuts=2):
        """A circle emitted as open arcs — never one closed loop. The
        closed-ring law: a loop's own slices face each other across the
        ring's width, so every slice collides with its neighbour."""
        span = 360.0 / cuts
        for k in range(cuts):
            self.arc(cx, cy, r, k * span + 6, (k + 1) * span - 6)

    def render(self, w=STROKE_W):
        im = Image.new("L", (SIZE, SIZE), WHITE)
        d = ImageDraw.Draw(im)
        r = w // 2
        for pl in self.lines:
            for a, b in zip(pl, pl[1:]):
                d.line([a, b], fill=BLACK, width=w)
            for (x, y) in pl:
                d.ellipse([x - r, y - r, x + r, y + r], fill=BLACK)
        return im


def on(cx, cy, r, deg):
    a = math.radians(deg)
    return (cx + r * math.cos(a), cy + r * math.sin(a))


def triquetra():
    s = Sketch()
    for k in range(3):
        cx, cy = on(C, C, 210 * 0.58, -90 + k * 120)
        s.circle_arcs(cx, cy, 210, cuts=2)
    return s


def pentagram():
    s = Sketch()
    pts = [on(C, C, 380, -90 + i * 72) for i in range(5)]
    order = [pts[(i * 2) % 5] for i in range(5)]
    for a, b in zip(order, order[1:] + order[:1]):
        s.seg(a, b)
    return s


def yinyang():
    s = Sketch()
    R = 380
    s.circle_arcs(C, C, R, cuts=2)
    s.arc(C, C - R / 2, R / 2, -90, 90)
    s.arc(C, C + R / 2, R / 2, 90, 270)
    for sign in (-1, 1):
        s.circle_arcs(C, C + sign * R / 2, 62, cuts=2)
    return s


def merkaba():
    s = Sketch()
    for base in (-90, 90):
        tri = [on(C, C, 370, base + i * 120) for i in range(3)]
        for a, b in zip(tri, tri[1:] + tri[:1]):
            s.seg(a, b)
    return s


def vegvisir():
    s = Sketch()
    R = 330
    for k in range(8):
        deg = -90 + k * 45
        tip = on(C, C, R, deg)
        perp = deg + 90
        s.seg((C, C), tip)
        if k % 4 == 0:
            s.seg(on(*tip, 78, perp), on(*tip, 78, perp + 180))
        elif k % 4 == 1:
            s.seg(tip, on(*tip, 92, deg - 40))
            s.seg(tip, on(*tip, 92, deg + 40))
        elif k % 4 == 2:
            s.circle_arcs(*tip, 62, cuts=2)
        else:
            mid = on(C, C, R - 90, deg)
            s.seg(on(*mid, 66, perp), on(*mid, 66, perp + 180))
    return s


def helmofawe():
    s = Sketch()
    R = 340
    for k in range(8):
        deg = -90 + k * 45
        tip = on(C, C, R, deg)
        perp = deg + 90
        s.seg((C, C), tip)
        s.seg(tip, on(*tip, 96, deg - 38))
        s.seg(tip, on(*tip, 96, deg + 38))
        for frac in (0.52, 0.78):
            mid = on(C, C, R * frac, deg)
            s.seg(on(*mid, 62, perp), on(*mid, 62, perp + 180))
    return s


def laguz():
    s = Sketch()
    s.seg((C - 90, C - 330), (C - 90, C + 330))
    s.seg((C - 90, C - 330), (C + 210, C - 140))
    return s


def cross():
    s = Sketch()
    s.seg((C, C - 360), (C, C + 360))
    s.seg((C - 250, C - 110), (C + 250, C - 110))
    return s


def diamond():
    s = Sketch()
    v = [(C, C - 350), (C + 300, C), (C, C + 350), (C - 300, C)]
    for a, b in zip(v, v[1:] + v[:1]):
        s.seg(a, b)
    return s


def arrow():
    s = Sketch()
    s.seg((C, C + 340), (C, C - 300))
    s.seg((C, C - 340), (C - 190, C - 150))
    s.seg((C, C - 340), (C + 190, C - 150))
    s.seg((C - 150, C + 250), (C, C + 130))
    s.seg((C + 150, C + 250), (C, C + 130))
    return s


def sriyantra():
    """EXPERT (D4 signed). The nine triangles are emitted EDGE BY EDGE,
    so a word runs along an actual triangle edge — the thing that makes
    it a Sri Yantra hosts the words."""
    s = Sketch()
    downs = [(330, -250, 470), (250, -170, 360), (176, -92, 260),
             (110, -20, 176), (58, 44, 104)]
    ups = [(310, 250, -440), (232, 172, -336), (156, 96, -232), (86, 26, -140)]
    for hw, y0, h in downs + ups:
        v = [(C - hw, C + y0), (C + hw, C + y0), (C, C + y0 + h)]
        for a, b in zip(v, v[1:] + v[:1]):
            s.seg(a, b)
    s.circle_arcs(C, C, 412, cuts=3)
    for k in range(16):
        px, py = on(C, C, 412, k * 22.5)
        s.circle_arcs(px, py, 36, cuts=2)
    sq = [(C - 478, C - 478), (C + 478, C - 478), (C + 478, C + 478), (C - 478, C + 478)]
    for a, b in zip(sq, sq[1:] + sq[:1]):
        s.seg(a, b)
    return s


def metatron():
    """EXPERT (D4 signed). Thirteen circles and the full set of joins —
    the joins ARE the subject, so every one is a centerline."""
    s = Sketch()
    step, rr = 150, 74
    centres = [(C, C)]
    centres += [on(C, C, step, -90 + k * 60) for k in range(6)]
    centres += [on(C, C, step * 2, -90 + k * 60) for k in range(6)]
    for (x, y) in centres:
        s.circle_arcs(x, y, rr, cuts=2)
    for i in range(len(centres)):
        for j in range(i + 1, len(centres)):
            s.seg(centres[i], centres[j])
    return s


SUBJECTS = {
    "triquetra": triquetra, "pentagram": pentagram, "yinyang": yinyang,
    "merkaba": merkaba, "vegvisir": vegvisir, "helmofawe": helmofawe,
    "laguz": laguz, "cross": cross, "diamond": diamond, "arrow": arrow,
    "sriyantra": sriyantra, "metatron": metatron,
}

# tier, kid-safe. D2 as signed (Eric, 2026-08-05: "add the norse
# staves"): pentagram, vegvisir and helmofawe carry kidFilterReview and
# stay out of Kid Mode pending his explicit pass.
SUBJECT_META = {
    "triquetra": ("medium", True), "pentagram": ("medium", False),
    "yinyang": ("medium", True), "merkaba": ("medium", True),
    "vegvisir": ("hard", False), "helmofawe": ("hard", False),
    "laguz": ("easy", True), "cross": ("easy", True),
    "diamond": ("easy", True), "arrow": ("easy", True),
    "sriyantra": ("expert", True), "metatron": ("expert", True),
}

if __name__ == "__main__":
    emitted = {}
    for name, fn in SUBJECTS.items():
        sk = fn()
        # the dense pieces draw thinner so their structure stays legible
        w = 22 if name in ("sriyantra", "metatron") else STROKE_W
        sk.render(w).save(REF / f"{name}.png")
        emitted[name] = [[[round(x, 1), round(y, 1)] for x, y in pl] for pl in sk.lines]
        tier, kid = SUBJECT_META[name]
        print(f"{name:11} {tier:7} kid={'yes' if kid else 'REVIEW':6} centerlines {len(sk.lines):3}")
    (REF / "symbol-centerlines.json").write_text(json.dumps(emitted))
    print(f"refs -> {REF}")
