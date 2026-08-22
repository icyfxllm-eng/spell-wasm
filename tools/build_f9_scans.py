#!/usr/bin/env python3
"""CC-MASTERPIECE-RECOG — scan documents for the F9 batch.

The scan is the picture's authored artifact. Everything downstream is derived
from it: build_manifests.py reads it for layers and per-language word specs,
bundle_scans.py folds it into the shipped scans.json, and the masters density
floor counts ITS points, not the guide's.

The measured fields are imported from build_scan_library rather than
reimplemented. min_clearance, the segment marks and the pin are all answers to
questions that must have exactly ONE answer -- a second implementation is a
second answer, and the pin especially is compared byte-for-byte later.

Run after tools/build_f9_rails.py:  python3 tools/build_f9_scans.py
"""
from __future__ import annotations

import json
import math
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]


def _borrow(names):
    """Take the named helpers from build_scan_library WITHOUT running it.

    Importing it normally executes its module body, which rebuilds the whole
    scan library -- doing that here silently rewrote redfuji.json and
    rhino.json, two shipped masters, as a side effect of wanting three
    functions. Only the imports and definitions are executed; every top-level
    statement that DOES something is dropped.

    Borrowing rather than copying is deliberate: min_clearance, the segment
    marks and the pin must have exactly one implementation, because a second
    one is a second answer and the pin is later compared exactly.
    """
    import ast
    src = (ROOT / "tools/build_scan_library.py").read_text()
    tree = ast.parse(src)
    keep = [n for n in tree.body
            if isinstance(n, (ast.Import, ast.ImportFrom, ast.FunctionDef,
                              ast.ClassDef, ast.Assign, ast.AnnAssign))]
    # The module computes paths from __file__ at import time.
    ns: dict = {"__file__": str(ROOT / "tools/build_scan_library.py")}
    exec(compile(ast.Module(body=keep, type_ignores=[]), "build_scan_library", "exec"), ns)
    return [ns[n] for n in names]


fnv, path_min_clearance, seg_marks, plen, turn = _borrow(
    ["fnv", "path_min_clearance", "seg_marks", "plen", "turn"])

RAILS = ROOT / "content-pipeline/wordpic/f9-rails.json"
STAGED = ROOT / "content-pipeline/wordpic/staged-batch-f9.json"
OUT = ROOT / "content-pipeline/wordpic/scans"

ATTRIB = {
    "hare": "after Albrecht Dürer",
    "beetle": "after Albrecht Dürer",
    "wing": "after Albrecht Dürer",
    "banana": "after Maria Sibylla Merian",
    "flamingo": "after John James Audubon",
    "carp": "after Katsushika Hokusai",
    "rose": "after Henry Joseph Redouté",
    "kosonowl": "after Ohara Koson",
}


def points_of(d: str) -> list[list[float]]:
    return [[float(a), float(b)] for a, b in re.findall(r"([-\d.]+) ([-\d.]+)", d)]


def worst_turn(p) -> float:
    """Sharpest sustained turn along the path, in degrees."""
    w = 0.0
    for i in range(1, len(p) - 1):
        w = max(w, turn(p[i - 1], p[i], p[i + 1]))
    return round(w, 1)


def main() -> int:
    rails = json.loads(RAILS.read_text())
    staged = json.loads(STAGED.read_text())["staged"]
    OUT.mkdir(parents=True, exist_ok=True)
    for pid, paths in rails.items():
        pts = [points_of(q["d"]) for q in paths]
        entries = []
        for i, (q, p) in enumerate(zip(paths, pts)):
            others = [x for j, x in enumerate(pts) if j != i]
            entries.append({
                "points": p,
                "arc": round(plen(p), 2),
                "tier": "expert",
                # These rails are authored word carriers, not traced ink: none
                # is a sub-floor fragment and none is decorative.
                "sub_floor": False,
                "merged_into": None,
                "decorative_thin": False,
                "segments": seg_marks(p),
                "worst_turn_deg": worst_turn(p),
                "min_clearance": round(path_min_clearance(p, others), 2),
                "tight_frac": 0.0,
                "feature": q["feature"],
            })
        doc = {
            "subject": pid,
            "tier": "expert",
            "pin_hash": fnv([e["points"] for e in entries]),
            "canvas": 512,
            "required_micro": [],
            "paths": entries,
            "authoring": "hand-traced",
            "attribution": ATTRIB[pid],
        }
        (OUT / f"{pid}.json").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n")
        total = sum(len(e["points"]) for e in entries)
        worst = min(e["min_clearance"] for e in entries)
        print(f"  {pid:10} {len(entries)} paths  {total:4} pts  "
              f"min_clearance {worst:6.2f}  pin {doc['pin_hash']:#x}")
        if total < 500:
            print(f"    WARNING: under the 500-point masters floor")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
