#!/usr/bin/env python3
"""Stage the Expert-band admissions and write the F7 review sheet.  Run with .corpus-cache/wf/v/bin/python.

Reads the admit.py candidates (assets/words/en/staged.tsv, all rows that passed F3) and a score file that covers them
(score.py --extra <candidates> --out <file>), keeps rows at or above config band_boundaries.expert, and rewrites
staged.tsv with only those rows.  The 200-row F7 sample and the candidate definitions go to a CSV for the auditor.
Candidate definitions are FLAGS FOR THE AUDITOR; they are never shown to a player.
"""
import argparse, csv, json, os, random, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

HOME = os.path.expanduser("~/repos/spell-wasm")
ap = argparse.ArgumentParser()
ap.add_argument("--scores", required=True)
ap.add_argument("--sample", type=int, default=200)
a = ap.parse_args()
cfg = json.load(open(os.path.join(ROOT, "config/en_difficulty.json")))
t = cfg["band_boundaries"]["expert"]
sc = json.load(open(a.scores))["scores"]
p = os.path.join(ROOT, "assets/words/en/staged.tsv")
rows = [l.split("\t")[0] for l in open(p, encoding="utf-8").read().splitlines()[2:] if l.strip()]
keep = sorted(w for w in rows if w in sc and sc[w] >= t)
with open(p, "w", encoding="utf-8") as f:
    f.write("# CC-EN-EXPERT-GROWTH F3. growth_stage=staged rows are NOT live: flag en.expert_growth, spelling surfaces only.\n")
    f.write("word\tgrowth_stage\tsource\n")
    for w in keep:
        f.write(f"{w}\tstaged\twiktionary+scowl\n")
gl = json.load(open(os.path.join(HOME, ".corpus-cache/en-wikt/staged_gloss.json")))
random.seed(20261016)
samp = random.sample(keep, min(a.sample, len(keep)))
os.makedirs(os.path.join(ROOT, "reports"), exist_ok=True)
with open(os.path.join(ROOT, "reports/en-staged-review-sample.csv"), "w", encoding="utf-8", newline="") as f:
    w = csv.writer(f)
    w.writerow(["word", "pos", "candidate_definition (flag only, unaudited)", "verdict (keep/cut/unsure)", "note"])
    for x in sorted(samp):
        g = gl.get(x, {})
        w.writerow([x, g.get("pos", ""), g.get("gloss", ""), "", ""])
print(f"{len(rows)} candidates -> {len(keep)} staged at score >= {t}; sample {len(samp)}")
