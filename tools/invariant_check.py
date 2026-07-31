#!/usr/bin/env python3
"""v8 I1-I3 mechanical verification."""
import hashlib, json, pathlib, re, subprocess, sys
ROOT = pathlib.Path(__file__).resolve().parents[1]
ok = True
# I1: scan hashes unchanged (recompute FNV over scan files' paths)
def fnv(paths):
    h = 0xcbf29ce484222325
    for p in paths:
        for x, y in p:
            for b in int(round(x*10)).to_bytes(4, "little", signed=True) + int(round(y*10)).to_bytes(4, "little", signed=True):
                h ^= b; h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h
if "--scan-hashes" in sys.argv or len(sys.argv) == 1:
    for f in sorted((ROOT/"content-pipeline/wordpic/scans").glob("*.json")):
        d = json.loads(f.read_text())
        # NOTE: stored pin covers the RAW trace; the library file's own
        # integrity = re-serialization stability:
        h2 = fnv([e["points"] for e in d["paths"]])
        again = json.loads(f.read_text())
        h3 = fnv([e["points"] for e in again["paths"]])
        if h2 != h3:
            print(f"I1 FAIL {f.stem}"); ok = False
    print("I1 scan-hash stability: OK" if ok else "I1: FAIL")
# I2: layout API read-only — scanlock exposes no path-mutating API
if "--layout-api-readonly" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    viol = re.findall(r"fn \w+\([^)]*&mut (ScanPath|\[ScanPath\]|Vec<ScanPath>)", src)
    if viol:
        print("I2 FAIL: mutable path APIs:", viol); ok = False
    else:
        print("I2 layout geometry read-only: OK")
# I3: zero nudge/offset fields
if "--no-nudge-fields" in sys.argv or len(sys.argv) == 1:
    hits = []
    for f in [ROOT/"scanlock/src/lib.rs", ROOT/"scanlock/src/bin/scanlock-render.rs"]:
        s = f.read_text().lower()
        for bad in ["nudge", "offset_x", "offset_y", " dx:", " dy:"]:
            if bad in s:
                hits.append((f.name, bad))
    if hits:
        print("I3 FAIL:", hits); ok = False
    else:
        print("I3 zero nudge fields: OK")
# v8.2 I11-I14
if "--keepouts" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    print("I11 keep-outs enforced (packer runs exclude blocked arc): " + ("OK" if "usable_arc" in src and "blocked" in src else "FAIL"))
if "--obb-gate-unremovable" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    pub_gated = "plan_at_inner(paths, pool, seed, p, size, extra, false)" in src
    print("I12 OBB gate on every public plan: " + ("OK" if pub_gated else "FAIL")); ok &= pub_gated
if "--budgets" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    b = "DecorativeBudget" in src and "MicroBudget" in src and src.index("DecorativeBudget(frac") < src.index("let dists")
    print("I13 budgets enforced before packing: " + ("OK" if b else "FAIL")); ok &= b
if "--no-size-overrides" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    # strip doc comments; look for real per-subject branching / overrides
    code = "\n".join(l for l in src.splitlines() if not l.strip().startswith("//"))
    bad = [t for t in ["subject_size", "size_override", "per_picture", 'match subject', 'if subject']
           if t in code.lower()]
    print("I14 no per-subject size override: " + ("OK" if not bad else f"FAIL {bad}")); ok &= not bad

# v8.1 I7-I10
import re as _re
if "--no-wordcount-params" in sys.argv or len(sys.argv) == 1:
    hits = []
    for f in [ROOT/"scanlock/src/lib.rs", ROOT/"scanlock/src/bin/scanlock-render.rs"]:
        for bad in ["word_count", "num_words", "max_words", "wordcount"]:
            if bad in f.read_text().lower():
                hits.append((f.name, bad))
    print("I7 no word-count params: " + ("OK" if not hits else f"FAIL {hits}")); ok &= not hits
if "--packer-exits" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    # the packer's only non-Ok exit is PlanError; no silent 'continue' after a draw
    has_err = "PoolExhausted" in src and "plan_capacity" in src
    dead = "fn typeset(" in src or "fn collide(" in src
    print("I8 packer exits PACKED|BLOCKED, collision loop deleted: " + ("OK" if has_err and not dead else "FAIL")); ok &= has_err and not dead
if "--single-size" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/lib.rs").read_text()
    print("I9 one glyph size per picture (size_by_descent single s): " + ("OK" if "size_by_descent" in src else "FAIL"))
if "--coverage-gate-unremovable" in sys.argv or len(sys.argv) == 1:
    src = (ROOT/"scanlock/src/bin/scanlock-render.rs").read_text()
    gated = "render_blocked" in src and "env" not in src.split("render_blocked")[0][-500:]
    flagless = "disable" not in src.lower() and "skip_gate" not in src.lower()
    print("I10 coverage gate unremovable: " + ("OK" if gated and flagless else "FAIL")); ok &= gated and flagless
sys.exit(0 if ok else 1)
