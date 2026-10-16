#!/usr/bin/env python3
"""CC-EN-EXPERT-GROWTH Phase B tests A4-A7.  Needs the build cache (.corpus-cache: Wiktionary headwords, SCOWL).
Run: .corpus-cache/wf/v/bin/python tools/en_growth/test_phase_b.py

A7 is a DATA-LEVEL check only: no runtime serves staged rows yet, so the 'Normal-mode fixture' half of A7 is not covered."""
import json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

HOME = os.path.expanduser("~/repos/spell-wasm")
fails = []
staged = [l.split("\t")[0] for l in open(os.path.join(ROOT, "assets/words/en/staged.tsv"), encoding="utf-8").read().splitlines()[2:] if l.strip()]
zipf = zipf_fn()
cmu = load_cmu()
hw_path = os.path.join(HOME, ".corpus-cache/en-wikt/headwords.json")
if not os.path.exists(hw_path):
    sys.exit("needs .corpus-cache/en-wikt/headwords.json (tools/en_growth/extract_wikt.py)")
hw = json.load(open(hw_path, encoding="utf-8"))
scowl = {l.split("/")[0] for l in open(os.path.join(HOME, ".corpus-cache/dicts/en/en_US.dic"), encoding="utf-8", errors="ignore").read().splitlines()[1:]}

# A4
for w in staged:
    z = zipf(w)
    if not (2.5 <= z <= 4.0):
        fails.append(f"A4: {w} Zipf {z}")
    if w not in cmu:
        fails.append(f"A4: {w} has no CMUdict key")
print(f"A4 {len(staged)} staged rows checked")

# A5
sys.path.insert(0, os.path.join(ROOT, "tools/en_growth"))
from admit import good_sense
for w in staged:
    if w not in hw or w not in scowl:
        fails.append(f"A5: {w} is not a headword in both licence-cleared dictionaries")
    elif not any(good_sense(e["pos"], s) for e in hw[w] for s in e["s"]):
        fails.append(f"A5: {w} has no current, standard sense")
print("A5 done")

# A6
sys.path.insert(0, os.path.join(ROOT, "tools/bank"))
from purity_admit import admit
ok, bad = admit("en", staged)
for w, c in bad.items():
    fails.append(f"A6: {w} fails PURITY ({c})")
prof = set()
for p in (os.path.join(ROOT, "assets/words/profanity/en.txt"), os.path.join(ROOT, "tools/wordpipe/sources/blocklist_en.txt")):
    if os.path.exists(p):
        prof |= {l.strip().lower() for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")}
for w in staged:
    if w in prof:
        fails.append(f"A6: {w} is on the profanity/blocklist")
    if not (w.isascii() and w.isalpha() and w.islower()):
        fails.append(f"A6: {w} is not a plain lowercase word")
bank = {w for t in TIERS for w in bank_rows()[t]}
for w in staged:
    if w in bank:
        fails.append(f"A6: {w} duplicates a bank row")
print("A6 done")

# A7 (data level): staged rows must not be in the definition pools, and no definition surface may read staged.tsv
dp = json.load(open(os.path.join(ROOT, "backend/def_pools/en.json")))
pooled = {r["word"] for t in dp["tiers"].values() for r in t}
for w in staged:
    if w in pooled:
        fails.append(f"A7: {w} already has a definition-pool row (needs an audit record first)")
hits = subprocess.run(["git", "grep", "-l", "staged.tsv", "--", "src", "backend/*.py", "*.js"], cwd=ROOT, capture_output=True, text=True).stdout.split()
for h in hits:
    fails.append(f"A7: {h} references staged.tsv; a serving path must be reviewed against the definitions gate first")
print("A7 data-level checks done; runtime fixture NOT covered (no staged serving path exists yet)")
for f in fails[:40]:
    print("FAIL", f)
print(f"{len(fails)} failures")
sys.exit(1 if fails else 0)
