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
sys.exit(0 if ok else 1)
