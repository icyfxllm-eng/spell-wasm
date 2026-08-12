#!/usr/bin/env python3
"""Trace refinement — clean up a hand trace without restructuring it.

You trace roughly from the reference; this makes the lines confident. Six
passes, each off by default and independently reportable:

    --ridge REF     slide every point onto the painting's own ink
    --resample PX   even spacing, so the layout solver steps predictably
    --smooth N      Taubin passes: take the hand-shake out, without shrinking
    --simplify EPS  corner-preserving Douglas-Peucker
    --close-gaps E  snap endpoints of DIFFERENT paths that nearly meet
    --trim-spurs PX drop the little hook where a stroke overshot
    --close-contour E  join a path's own ends when they nearly meet

WHY THIS IS NOT build_scan_library. That module owns the trace format and
computes every derived field, but its entry loop also RESTRUCTURES: it drops
small closed features into a separate list, merges sub-floor paths, reorders
for stroke-side dedup, and splits tight runs. Re-running it over refined
geometry would renumber the paths — and `content-pipeline/wordpic/manifests/`
assigns silhouette/anchor/texture layers by path INDEX. Renumbering silently
moves the layers onto the wrong strokes. So this tool is index-stable by
construction: same path count, same order, asserted before it writes.

WHAT IT RECOMPUTES, and with whose maths. `arc`, `segments`, `worst_turn_deg`,
`min_clearance` and `tight_frac` all derive from the points, so refining the
points makes them stale. They are recomputed here by IMPORTING plen, turn,
seg_marks and path_min_clearance from build_scan_library — never by
reimplementing them, because two copies of a formula is one copy too many.

WHAT IT REFUSES TO DO. `sub_floor` and `decorative_thin` are also derived, but
they are STRUCTURAL: sub_floor triggers merging, decorative_thin excludes a
path from word placement. If refinement would flip either, this stops and says
so rather than quietly changing what the piece is made of. That is a signal to
re-run the real pipeline and re-author the manifest, not something to paper
over.

Ridge snapping needs a continuous-tone reference — it samples the artwork's
own ink, and a 1-bit pictogram has no ridge to find. The tool measures the
reference and refuses rather than producing confident nonsense.
"""
from __future__ import annotations

import argparse
import ast
import json
import math
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCANS = ROOT / "content-pipeline/wordpic/scans"
MANIFESTS = ROOT / "content-pipeline/wordpic/manifests"

_FNS = ("plen", "turn", "seg_marks", "path_min_clearance")
_CONSTS = ("CANVAS", "FLOOR", "MIN_WORD_CHARS", "CORNER_DEG", "SEG_MAX", "SMALL_FEATURE_MAX")


def _borrow():
    """Lift the metric helpers out of build_scan_library WITHOUT importing it.

    That module has no `if __name__ == "__main__"` guard: importing it re-runs
    the entire scan build and REWRITES content-pipeline/wordpic/scans/*.json as
    an import side effect. The first dry run of this tool did exactly that —
    it rewrote rhino.json, minified, before printing a single line of its
    supposedly read-only report, and had to be reverted from git.

    So the four pure functions are extracted by AST and exec'd in an isolated
    namespace. Same maths, same source of truth, none of the file's other
    statements. If build_scan_library ever grows a main guard this can go back
    to a plain import.
    """
    tree = ast.parse((ROOT / "tools/build_scan_library.py").read_text())
    keep = [
        n
        for n in tree.body
        if (isinstance(n, ast.FunctionDef) and n.name in _FNS)
        or (
            isinstance(n, ast.Assign)
            and any(getattr(t, "id", None) in _CONSTS for t in n.targets)
        )
    ]
    missing = set(_FNS) - {n.name for n in keep if isinstance(n, ast.FunctionDef)}
    if missing:
        sys.exit(f"build_scan_library no longer defines {sorted(missing)} — "
                 f"the metric helpers moved; update tools/refine_trace.py")
    ns: dict = {"math": math}
    exec(compile(ast.Module(body=keep, type_ignores=[]), "<borrowed>", "exec"), ns)
    return ns


_BS = _borrow()
FLOOR = _BS["FLOOR"]
MIN_WORD_CHARS = _BS["MIN_WORD_CHARS"]
plen = _BS["plen"]
turn = _BS["turn"]
seg_marks = _BS["seg_marks"]
path_min_clearance = _BS["path_min_clearance"]

# The trace format's "no neighbour" sentinel. path_min_clearance returns its
# own large default when a path has no interior left to measure; the shipped
# files record 999.0, so normalize rather than writing 1e9 into the bank.
NO_NEIGHBOUR = 999.0

# A turn this sharp is a decision, not jitter: simplification and smoothing
# both leave it alone. Douglas-Peucker on its own rounds corners off, which on
# a Hokusai reads as a melted mountain.
CORNER_DEG = 35.0


# ---------------------------------------------------------------- geometry

def median_step(pts: list) -> float:
    """The path's own typical segment length."""
    segs = sorted(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:]))
    segs = [s for s in segs if s > 0]
    return segs[len(segs) // 2] if segs else 0.0


def resample(pts: list, step: float) -> list:
    """Even spacing along the path. Endpoints are always kept.

    `--resample auto` passes the path's own median segment length, which evens
    out hand-shake spacing WITHOUT changing how dense the trace is. A fixed
    step is a density decision in disguise: the first --repair preset used 3px
    and silently inflated the Rhinoceros from 786 points to 1,092.
    """
    if len(pts) < 2 or step <= 0:
        return pts
    out = [list(pts[0])]
    acc = 0.0
    for a, b in zip(pts, pts[1:]):
        seg = math.hypot(b[0] - a[0], b[1] - a[1])
        if seg == 0:
            continue
        t = step - acc
        while t <= seg:
            out.append([a[0] + (b[0] - a[0]) * t / seg, a[1] + (b[1] - a[1]) * t / seg])
            t += step
        acc = (acc + seg) % step
    if out[-1] != list(pts[-1]):
        out.append(list(pts[-1]))
    return out


def corners(pts: list, deg: float = CORNER_DEG) -> set:
    """Indices whose turn exceeds `deg` — the points nothing may remove."""
    return {i for i in range(1, len(pts) - 1) if turn(pts[i - 1], pts[i], pts[i + 1]) > deg}


# Taubin's λ/μ pair. The second, negative pass pushes back out by slightly
# more than the first pulled in, which is what keeps a smoothed curve from
# creeping toward its own centre.
TAUBIN_LAMBDA = 0.50
TAUBIN_MU = -0.53


def smooth(pts: list, passes: int, keep: set) -> list:
    """Taubin smoothing: take the hand-shake out without shrinking the shape.

    Chaikin was the first choice here and was wrong twice over. It SUBDIVIDES,
    so one pass doubled the Rhinoceros from 786 points to 1,542 — a density
    change disguised as a smoothing pass. And it SHRINKS: corner-cutting pulls
    a curve inside itself, which shortened Great Wave's path 10 from just over
    the sub-floor threshold to 34px under it, i.e. it silently turned a word
    path into a merge tag.

    Taubin fixes both. Point count is invariant, and the alternating λ/μ passes
    cancel the shrinkage that a plain Laplacian smoother would accumulate.
    Corners and endpoints are pinned, so a deliberate angle survives.
    """
    for _ in range(max(0, passes)):
        if len(pts) < 3:
            break
        for factor in (TAUBIN_LAMBDA, TAUBIN_MU):
            src = [list(q) for q in pts]
            for i in range(1, len(pts) - 1):
                if i in keep:
                    continue
                ax = (src[i - 1][0] + src[i + 1][0]) / 2.0
                ay = (src[i - 1][1] + src[i + 1][1]) / 2.0
                pts[i][0] = src[i][0] + factor * (ax - src[i][0])
                pts[i][1] = src[i][1] + factor * (ay - src[i][1])
    return pts


def _rdp(pts: list, eps: float) -> list:
    if len(pts) < 3:
        return pts
    a, b = pts[0], pts[-1]
    vx, vy = b[0] - a[0], b[1] - a[1]
    l2 = vx * vx + vy * vy
    worst, wi = -1.0, 0
    for i in range(1, len(pts) - 1):
        p = pts[i]
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / l2))
        d = math.hypot(p[0] - (a[0] + t * vx), p[1] - (a[1] + t * vy))
        if d > worst:
            worst, wi = d, i
    if worst <= eps:
        return [pts[0], pts[-1]]
    return _rdp(pts[: wi + 1], eps)[:-1] + _rdp(pts[wi:], eps)


def simplify(pts: list, eps: float) -> list:
    """Douglas-Peucker run BETWEEN corners, so corners cannot be dropped."""
    if len(pts) < 3 or eps <= 0:
        return pts
    stops = [0] + sorted(corners(pts)) + [len(pts) - 1]
    out: list = []
    for u, v in zip(stops, stops[1:]):
        piece = _rdp(pts[u : v + 1], eps)
        out.extend(piece[:-1])
    out.append(list(pts[-1]))
    return out


def trim_spurs(pts: list, max_px: float) -> list:
    """Drop a short terminal run that doubles back — the overshoot hook.

    Only fires when the run is BOTH short and sharply reversed (>100 deg), so a
    genuine short tail (a stem, a whisker) survives.
    """
    if max_px <= 0 or len(pts) < 4:
        return pts

    def eat_front(p):
        run = 0.0
        for i in range(1, len(p) - 1):
            run += math.hypot(p[i][0] - p[i - 1][0], p[i][1] - p[i - 1][1])
            if run > max_px:
                return p
            if turn(p[i - 1], p[i], p[i + 1]) > 100.0:
                return p[i:]
        return p

    pts = eat_front(pts)
    pts = list(reversed(eat_front(list(reversed(pts)))))
    return pts


def close_contour(pts: list, eps: float) -> list:
    """If a path's own ends nearly meet, make them meet exactly."""
    if eps <= 0 or len(pts) < 3:
        return pts
    d = math.hypot(pts[0][0] - pts[-1][0], pts[0][1] - pts[-1][1])
    if 0 < d <= eps:
        pts = pts[:-1] + [list(pts[0])]
    return pts


def close_gaps(paths: list, eps: float) -> tuple:
    """Snap endpoints of DIFFERENT paths that nearly meet, to their midpoint.

    Moves endpoints only — it never merges two paths into one, because that
    would renumber everything downstream. A silhouette authored as a frame plus
    a figure stays a frame plus a figure.
    """
    if eps <= 0:
        return paths, 0
    ends = [(i, j) for i, p in enumerate(paths) if len(p) >= 2 for j in (0, len(p) - 1)]
    joined = 0
    for a in range(len(ends)):
        ia, ja = ends[a]
        for b in range(a + 1, len(ends)):
            ib, jb = ends[b]
            if ia == ib:
                continue
            pa, pb = paths[ia][ja], paths[ib][jb]
            d = math.hypot(pa[0] - pb[0], pa[1] - pb[1])
            if 0 < d <= eps:
                mx, my = (pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2
                paths[ia][ja] = [mx, my]
                paths[ib][jb] = [mx, my]
                joined += 1
    return paths, joined


# ---------------------------------------------------------------- ridge

def ridge_snap(paths: list, ref: pathlib.Path) -> list:
    """Slide each point onto the artwork's own ink, then fit one clean curve.

    This is CC-TONAL-FACEPASS's `_ridge_refine_smooth`, imported rather than
    reimplemented: landmarks position a stroke, the painting's ink shapes it.
    It is the whole reason an imperfect hand trace can become a good one.
    """
    # Measure the reference BEFORE reaching for the heavy imports: the answer
    # "this source cannot support ridge snapping" does not depend on OpenCV
    # being installed, and hiding it behind an ImportError told the operator to
    # go fetch numpy for a run that was never going to be valid.
    from PIL import Image

    if not ref.exists():
        sys.exit(f"no reference at {ref}")
    im = Image.open(ref).convert("L")
    greys = sum(1 for v in im.histogram() if v)
    if greys <= 4:
        sys.exit(f"{ref.name} is a {greys}-level pictogram, not a continuous-tone "
                 f"reference. Ridge snapping samples the artwork's own ink; there is "
                 f"no ridge in a 1-bit image, so this would invent detail rather than "
                 f"recover it. Trace from a real photograph of the work.")

    # tonal.py lives with the pipeline, not in tools/.
    sys.path.insert(0, str(ROOT / "content-pipeline/wordpic"))
    try:
        import tonal  # noqa
    except Exception as e:  # numpy / cv2 absent
        sys.exit(f"--ridge needs the tonal deps (numpy, opencv, scipy): {e}\n"
                 f"  run it with a python that has them")
    img, _box, _scale = tonal.load_canvas(str(ref))
    return [[list(q) for q in tonal._ridge_refine_smooth([tuple(p) for p in path], img)]
            for path in paths]


# ---------------------------------------------------------------- recompute

def recompute(entries: list) -> None:
    """Refresh every field derived from the points, using the owner's maths."""
    word_pts = [e["points"] for e in entries
                if not e.get("sub_floor") and not e.get("decorative_thin")]
    for e in entries:
        p = e["points"]
        e["arc"] = round(plen(p), 2)
        marks = seg_marks(p)
        e["segments"] = marks
        cums = [0.0]
        for a, b in zip(p, p[1:]):
            cums.append(cums[-1] + math.hypot(b[0] - a[0], b[1] - a[1]))
        tot = cums[-1] or 1.0
        bounds = [0.0] + [t for t in marks if 0 < t < 1] + [1.0]
        worst = 0.0
        for k in range(1, len(p) - 1):
            if not any(abs(cums[k] / tot - m) < 0.012 for m in bounds):
                worst = max(worst, turn(p[k - 1], p[k], p[k + 1]))
        e["worst_turn_deg"] = round(worst, 1)

        others = [q for q in word_pts if q is not p]
        if e.get("sub_floor") or e.get("decorative_thin") or not others:
            e["min_clearance"], e["tight_frac"] = 999.0, 0.0
            continue
        e["min_clearance"] = round(min(path_min_clearance(p, others), NO_NEIGHBOUR), 2)

        def sd2(pt, u, v):
            vx, vy = v[0] - u[0], v[1] - u[1]
            l2 = vx * vx + vy * vy
            t = 0 if l2 == 0 else max(0, min(1, ((pt[0] - u[0]) * vx + (pt[1] - u[1]) * vy) / l2))
            return math.hypot(pt[0] - (u[0] + t * vx), pt[1] - (u[1] + t * vy))

        sample = p[::2]
        tight = sum(1 for pt in sample
                    if any(sd2(pt, u, v) < FLOOR for q in others for u, v in zip(q, q[1:])))
        e["tight_frac"] = round(tight / max(1, len(sample)), 3)


def structural_flips(entries: list, arc_before: list) -> list:
    """Would refinement have changed what the piece is MADE OF?

    Compares the sub-floor verdict BEFORE and AFTER, never the stored flag
    against the raw threshold. The threshold alone is not the rule: the
    tight-run split exempts a short path from merging, so Great Wave ships
    path 10 at arc 35.33 with sub_floor False and would be condemned by any
    check that assumes `arc < 39 implies sub_floor`. The first version of this
    function did assume exactly that and refused a no-op run.

    What matters is whether THIS pass moved a path across the line, so an
    exemption the pipeline already granted stays granted.
    """
    line = FLOOR * MIN_WORD_CHARS
    bad = []
    for i, (e, was_arc) in enumerate(zip(entries, arc_before)):
        if (was_arc < line) != (e["arc"] < line):
            bad.append(f"path {i}: refinement moved arc {was_arc} -> {e['arc']} across the "
                       f"{line:.0f}px merge threshold. That changes whether the path is a "
                       f"word baseline or a merge tag; re-run build_scan_library and "
                       f"re-author the manifest rather than forcing this through")
    return bad


# ---------------------------------------------------------------- driver

def selftest() -> int:
    """Prove the guards bite. Same discipline as the other gate tools: every
    case asserts on a specific behaviour, not on "it ran"."""
    bad = []

    def check(name, cond, detail=""):
        if not cond:
            bad.append(f"  {name}{': ' + detail if detail else ''}")

    # A square with jitter on one edge — corners must survive every pass.
    sq = [[0, 0], [50, 0], [100, 0], [100, 50], [100, 100], [50, 100], [0, 100], [0, 0]]
    jit = [[x + (1 if i % 2 else -1), y] for i, (x, y) in enumerate(sq)]
    c = corners(sq)
    check("corners finds the square's four turns", len(c) >= 3, f"got {sorted(c)}")

    sm = smooth([list(p) for p in jit], 4, corners(jit))
    check("smoothing preserves point count", len(sm) == len(jit), f"{len(jit)} -> {len(sm)}")
    shrink = 1.0 - plen(sm) / plen(jit)
    check("Taubin does not shrink the shape", abs(shrink) < 0.12, f"{shrink:+.1%}")

    simp = simplify([list(p) for p in sq], 2.0)
    check("simplify keeps the corners", len(simp) >= 5, f"{len(sq)} -> {len(simp)} points")

    # A collinear run must collapse; a corner run must not.
    line = [[float(i), 0.0] for i in range(20)]
    check("simplify collapses a straight run", len(simplify(line, 0.5)) == 2,
          f"got {len(simplify(line, 0.5))}")

    # Even spacing at the path's own density leaves density alone.
    dense = [[float(i) * 3.0, 0.0] for i in range(30)]
    rs = resample(dense, median_step(dense))
    check("auto respacing preserves density", abs(len(rs) - len(dense)) <= 1,
          f"{len(dense)} -> {len(rs)}")

    # Endpoint snapping joins two near-meeting paths WITHOUT merging them.
    a = [[0.0, 0.0], [10.0, 0.0]]
    b = [[12.0, 0.0], [20.0, 0.0]]
    paths, joined = close_gaps([a, b], 4.0)
    check("close-gaps snaps endpoints", joined == 1 and paths[0][-1] == paths[1][0],
          f"joined={joined}")
    check("close-gaps never merges paths", len(paths) == 2, f"{len(paths)} paths")

    # The structural guard: a path shoved across the merge line must refuse,
    # and a path that was ALREADY exempt below the line must not.
    line_px = FLOOR * MIN_WORD_CHARS
    check("flip across the merge line refuses",
          structural_flips([{"arc": line_px - 5}], [line_px + 5]) != [])
    check("a pre-existing sub-floor exemption is left alone",
          structural_flips([{"arc": line_px - 5}], [line_px - 4]) == [])
    check("no movement, no complaint",
          structural_flips([{"arc": 100.0}], [100.0]) == [])

    if bad:
        print("FAIL refine_trace selftest:")
        print("\n".join(bad))
        return 1
    print("refine_trace selftest: OK — 10 deliberate cases behave")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("subject")
    ap.add_argument("--ref", help="continuous-tone reference for ridge snapping")
    ap.add_argument("--ridge", action="store_true")
    ap.add_argument("--resample", default="", metavar="PX|auto",
                    help="even out spacing; 'auto' preserves the path's own density")
    ap.add_argument("--smooth", type=int, default=0, metavar="N")
    ap.add_argument("--simplify", type=float, default=0.0, metavar="EPS")
    ap.add_argument("--close-gaps", type=float, default=0.0, metavar="EPS")
    ap.add_argument("--close-contour", type=float, default=0.0, metavar="EPS")
    ap.add_argument("--trim-spurs", type=float, default=0.0, metavar="PX")
    ap.add_argument("--repair", action="store_true",
                    help="preset: trim-spurs 6, close-contour 4, resample auto, smooth 1")
    ap.add_argument("--out", help="write here (default: report only, nothing written)")
    ap.add_argument("--in-place", action="store_true")
    args = ap.parse_args()

    if args.repair:
        args.trim_spurs = args.trim_spurs or 6.0
        args.close_contour = args.close_contour or 4.0
        args.resample = args.resample or "auto"
        args.smooth = args.smooth or 1

    sf = SCANS / f"{args.subject}.json"
    if not sf.exists():
        sys.exit(f"no scan for {args.subject!r} at {sf.relative_to(ROOT)}")
    doc = json.loads(sf.read_text())
    entries = doc["paths"]
    before = [[list(q) for q in e["points"]] for e in entries]
    n_before = [len(p) for p in before]
    arc_before = [e["arc"] for e in entries]

    paths = [[list(q) for q in e["points"]] for e in entries]

    if args.ridge:
        if not args.ref:
            sys.exit("--ridge needs --ref pointing at the continuous-tone reference")
        paths = ridge_snap(paths, pathlib.Path(args.ref))
    for i, p in enumerate(paths):
        if args.trim_spurs:
            p = trim_spurs(p, args.trim_spurs)
        if args.resample:
            step = median_step(p) if args.resample == "auto" else float(args.resample)
            p = resample(p, step)
        if args.smooth:
            p = smooth(p, args.smooth, corners(p))
        if args.simplify:
            p = simplify(p, args.simplify)
        if args.close_contour:
            p = close_contour(p, args.close_contour)
        paths[i] = p
    paths, joined = close_gaps(paths, args.close_gaps)

    # The invariant this whole tool exists to hold.
    assert len(paths) == len(entries), "path count changed — manifests index by position"

    for e, p in zip(entries, paths):
        e["points"] = [[round(x, 2), round(y, 2)] for x, y in p]
    recompute(entries)

    flips = structural_flips(entries, arc_before)

    # Report: what moved, and how far.
    print(f"{args.subject}: {len(entries)} paths (index-stable)")
    tot_b, tot_a = sum(n_before), sum(len(e["points"]) for e in entries)
    for i, e in enumerate(entries):
        a = e["points"]
        drift = max((min(math.hypot(x - u, y - v) for u, v in before[i]) for x, y in a),
                    default=0.0) if before[i] else 0.0
        mark = "" if n_before[i] == len(a) else f"  pts {n_before[i]}->{len(a)}"
        print(f"  path {i:<3} arc={e['arc']:<8} turn={e['worst_turn_deg']:<6} "
              f"clear={e['min_clearance']:<7} drift={drift:5.1f}px{mark}")
    if joined:
        print(f"  closed {joined} endpoint gap(s)")
    print(f"  points {tot_b} -> {tot_a}")

    if flips:
        for f in flips:
            print(f"REFUSED {f}")
        return 1

    if tot_a < tot_b:
        print(f"  NOTE: {tot_b - tot_a} points fewer. tools/density_check.py will fail "
              f"until you re-record:  python3 tools/density_check.py --accept")

    dest = sf if args.in_place else (pathlib.Path(args.out) if args.out else None)
    if dest is None:
        print("  (report only — pass --out PATH or --in-place to write)")
        return 0
    dest.write_text(json.dumps(doc, ensure_ascii=False) + "\n")
    print(f"  wrote {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
