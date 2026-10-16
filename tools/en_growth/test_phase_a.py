#!/usr/bin/env python3
"""CC-EN-EXPERT-GROWTH Phase A tests A1-A3.  Run: .corpus-cache/wf/v/bin/python tools/en_growth/test_phase_a.py

A1 en_pron_coverage, A2 en_difficulty_deterministic, A3 en_calibration.
A3 is only ENFORCED once calibration/en_expert.csv carries 'STATUS: APPROVED'; while it is
DRAFT it reports the numbers and does not fail (nothing may be tuned against an unsigned set).
"""
import collections, json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

fails = []
cal_path = os.path.join(ROOT, "calibration/en_expert.csv")
cal_lines = open(cal_path, encoding="utf-8").read().splitlines()
approved = any("STATUS: APPROVED" in l for l in cal_lines[:2])
cal = [l.split(",") for l in cal_lines if l and not l.startswith("#") and l != "word,expected"]
extra = tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False)
extra.write("\n".join(w for w, _ in cal) + "\n"); extra.close()

# A1
pron = json.load(open(os.path.join(ROOT, "assets/words/en/en_pron.json")))
if pron["bank_hash"] != bank_hash():
    fails.append("A1: en_pron.json is stale against the bank (bank_hash differs)")
cmu = load_cmu()
rows = [w for t in TIERS for w in bank_rows()[t]]
for w in rows:
    if w not in pron["keys"] and w not in pron["null"]:
        fails.append(f"A1: {w} has neither a key nor a null listing")
for w in ("rhythm", "accommodate", "liaison", "mnemonic", "pharaoh"):
    if w in cmu and pron["keys"].get(w) != [" ".join(p) for p in cmu[w]]:
        fails.append(f"A1: key for {w} differs from CMUdict")
print(f"A1 keyed={len(pron['keys'])} null={pron['null']}")

# A2
outs = []
for i in range(2):
    o = tempfile.NamedTemporaryFile(suffix=".json", delete=False).name
    subprocess.run([sys.executable, os.path.join(ROOT, "tools/en_growth/score.py"), "--extra", extra.name, "--out", o], check=True, stdout=subprocess.DEVNULL)
    outs.append(open(o, "rb").read())
if outs[0] != outs[1]:
    fails.append("A2: two score runs differ")
shipped = os.path.join(ROOT, "assets/words/en/en_difficulty.json")
if open(shipped, "rb").read() != outs[0]:
    fails.append("A2: committed en_difficulty.json is stale versus a fresh run")
print("A2 deterministic:", outs[0] == outs[1])

# A3
cfg = json.load(open(os.path.join(ROOT, "config/en_difficulty.json")))
sc = json.loads(outs[0])["scores"]
cls = collections.defaultdict(list)
for w, l in cal:
    if w in sc:
        cls[l].append(sc[w])
edges = cfg["band_boundaries"].get("expert")
if edges is None:
    # report-only: the best-separating threshold on this set
    cands = sorted({x for v in cls.values() for x in v})
    best = max(cands, key=lambda t: sum(x >= t for x in cls["expert"]) / len(cls["expert"]) - sum(x >= t for x in cls["not_expert"]) / len(cls["not_expert"]))
    t = best
else:
    t = edges
e = sum(x >= t for x in cls["expert"]) / len(cls["expert"])
n = sum(x >= t for x in cls["not_expert"]) / len(cls["not_expert"])
bar = cfg["calibration_bar"]
ok = e >= bar["expert_in_min"] and n <= bar["not_expert_in_max"]
print(f"A3 threshold={t:.3f} expert_in={e:.0%} (need >={bar['expert_in_min']:.0%}) not_expert_in={n:.0%} (need <={bar['not_expert_in_max']:.0%}) -> {'PASS' if ok else 'FAIL'} [{'ENFORCED' if approved else 'report only: set is DRAFT'}]")
if approved and not ok:
    fails.append("A3: calibration bar not met")
for f in fails:
    print("FAIL", f)
sys.exit(1 if fails else 0)
