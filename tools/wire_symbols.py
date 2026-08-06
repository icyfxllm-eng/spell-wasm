#!/usr/bin/env python3
"""Wire the geometric symbol lane into the registry FROM ITS
CENTERLINES (tools/draw_symbols.py emits them). Idempotent: re-running
replaces the twelve entries rather than duplicating them.

Lives in tools/ because it is not a one-off — it is how this lane is
rebuilt, and a scratchpad copy has already been lost twice.

Three laws the sweep taught, all applied here:
  * separation is measured in RENDER space (the engine normalizes into
    a 512 frame with a 22 margin, so a gap authored on a 1000px canvas
    shrinks ~2.1x before a word is placed);
  * crossing lines are SPLIT at their intersections, not rejected — a
    pentagram is five chords that all cross, and rejecting leaves one;
  * hosted spans are TRIMMED clear of their neighbours, because a
    radial hub puts every span's inner end at the same point and the
    WORDS collide there even when the bodies do not.
"""
import json, math, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
LINES = ROOT / "content-pipeline/wordpic/ref/symbol-centerlines.json"
REG = ROOT / "config/wordpic/pictures.json"

META = {
 "triquetra": ("Triquetra", "⛩️", "medium", True, ["trinity knot", "celtic knot"]),
 "pentagram": ("Pentagram", "☆", "medium", False, ["five-pointed star", "pentacle"]),
 "yinyang":   ("Yin and Yang", "☯️", "medium", True, ["taijitu", "yin yang"]),
 "merkaba":   ("Merkaba", "✡️", "medium", True, ["star tetrahedron", "hexagram"]),
 "vegvisir":  ("Vegvísir", "\U0001f9ed", "hard", False, ["wayfinder", "norse compass"]),
 "helmofawe": ("Helm of Awe", "❄️", "hard", False, ["aegishjalmur", "helm"]),
 "laguz":     ("Laguz", "ᛚ", "easy", True, ["rune", "water rune"]),
 "cross":     ("Cross", "✝️", "easy", True, []),
 "diamond":   ("Diamond", "◇", "easy", True, ["rhombus"]),
 "arrow":     ("Arrow", "↑", "easy", True, []),
 "sriyantra": ("Sri Yantra", "\U0001f537", "expert", True, ["yantra", "sacred geometry"]),
 "metatron":  ("Metatron's Cube", "⬡", "expert", True, ["fruit of life", "sacred geometry"]),
}
MAX_HOSTS = {"easy": 6, "medium": 10, "hard": 16, "expert": 28}
PACK = {"sriyantra": "symbols2", "metatron": "symbols2",
        "vegvisir": "symbols2", "helmofawe": "symbols2"}
SEP_RENDER, MIN_LEN_RENDER = 34.0, 46.0


def arclen(p):
    return sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(p, p[1:]))


def dstr(p):
    return "M" + " L".join(f"{x:.1f} {y:.1f}" for x, y in p)


def budget_for(L):
    return [2, 10] if L < 460 else [3, 20]


def densify(pts, step=8.0):
    out = [pts[0]]
    for a, b in zip(pts, pts[1:]):
        seg = math.hypot(b[0] - a[0], b[1] - a[1])
        n = max(1, int(seg / step))
        out += [[a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n]
                for i in range(1, n + 1)]
    return out


def core(pts, frac=0.22):
    """Two spans that MEET at a junction share an endpoint — legal. What
    must stay apart is their bodies."""
    d = densify(pts, 6.0)
    k = max(1, int(len(d) * frac))
    return d[k:len(d) - k] or d


def min_dist(a, b):
    da, db = core(a), core(b)
    return min(math.hypot(pa[0] - pb[0], pa[1] - pb[1]) for pa in da for pb in db[::2])


def split_at_crossings(lines):
    def inter(p1, p2, p3, p4):
        x1, y1 = p1; x2, y2 = p2; x3, y3 = p3; x4, y4 = p4
        den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4)
        if abs(den) < 1e-9:
            return None
        t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den
        u = ((x1 - x3) * (y1 - y2) - (y1 - y3) * (x1 - x2)) / den
        return t if 0.02 < t < 0.98 and 0.02 < u < 0.98 else None
    out = []
    for i, pl in enumerate(lines):
        cuts = set()
        for a_i in range(len(pl) - 1):
            a, b = pl[a_i], pl[a_i + 1]
            for j, other in enumerate(lines):
                if i == j:
                    continue
                for b_i in range(len(other) - 1):
                    t = inter(a, b, other[b_i], other[b_i + 1])
                    if t is not None:
                        cuts.add((a_i, round(t, 4)))
        if not cuts:
            out.append(pl)
            continue
        pieces, cur = [], [pl[0]]
        for a_i in range(len(pl) - 1):
            a, b = pl[a_i], pl[a_i + 1]
            for t in sorted(t for (idx, t) in cuts if idx == a_i):
                pt = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
                cur.append(pt); pieces.append(cur); cur = [pt]
            cur.append(b)
        pieces.append(cur)
        out += [p for p in pieces if len(p) >= 2]
    return out


def main():
    lines = json.loads(LINES.read_text())
    reg = json.loads(REG.read_text())
    for pid, (name, icon, tier, kid_ok, aliases) in META.items():
        reg["pictures"] = [x for x in reg["pictures"] if x["id"] != pid]
        all_lines = [[tuple(q) for q in pl] for pl in lines[pid]]
        xs = [q[0] for pl in all_lines for q in pl]
        ys = [q[1] for pl in all_lines for q in pl]
        span = max(max(xs) - min(xs), max(ys) - min(ys)) or 1.0
        scale = (512 - 2 * 22) / span
        sep_draw, min_len_draw = SEP_RENDER / scale, MIN_LEN_RENDER / scale
        spans = split_at_crossings(all_lines)
        cands = sorted((p for p in spans if arclen(p) >= min_len_draw),
                       key=arclen, reverse=True)
        kept = []
        for p in cands:
            if len(kept) >= MAX_HOSTS[tier]:
                break
            if all(min_dist(p, k) >= sep_draw for k in kept):
                kept.append(p)

        def trimmed(span, others):
            d = densify(span, 6.0)
            ok = [all(min(math.hypot(pt[0] - q[0], pt[1] - q[1]) for q in densify(o, 8.0)) >= sep_draw
                      for o in others) for pt in d]
            best, cur = (0, 0), None
            for i, good in enumerate(ok + [False]):
                if good and cur is None:
                    cur = i
                elif not good and cur is not None:
                    if i - cur > best[1] - best[0]:
                        best = (cur, i)
                    cur = None
            a, b = best
            return d[a:b] if b - a >= 2 else []

        final = []
        for i, span in enumerate(kept):
            others = [k for j, k in enumerate(kept) if j != i]
            t = trimmed(span, others) if others else densify(span, 6.0)
            if arclen(t) >= min_len_draw:
                final.append(t)

        entry = {
            "id": pid, "tier": tier, "subject": pid, "icon": icon, "name": name,
            "kid": kid_ok, "wash": False, "pack": PACK.get(pid, "symbols1"),
            "categories": ["symbols"], "canonicalCategory": "symbols",
            "aliases": aliases, "extractionClass": "LINE",
            "guide": [dstr(p) for p in all_lines][:200],
            "paths": [{"mode": "flow", "d": dstr(p), "order": i + 1,
                       "budget": budget_for(arclen(p)), "band": 2, "arch": "line",
                       "feature": f"{pid} stroke {i + 1}"} for i, p in enumerate(final)],
            "provenance": {
                "title": name, "artist": "SpellGame original",
                "source": "repo:tools/draw_symbols.py",
                "sourceUrl": "repo:tools/draw_symbols.py",
                "pdBasis": "original-artwork: mathematical construction, centerline-emitted",
                "retrieved": "2026-08-05"},
        }
        if not kid_ok:
            entry["kidFilterReview"] = True   # D2: awaiting Eric's pass
        reg["pictures"].append(entry)
        print(f"  {pid:11} {tier:7} hosts {len(final):2}  guides {len(entry['guide']):3}")
    REG.write_text(json.dumps(reg, ensure_ascii=False, indent=1))
    print("total pictures:", len(reg["pictures"]))


if __name__ == "__main__":
    main()
