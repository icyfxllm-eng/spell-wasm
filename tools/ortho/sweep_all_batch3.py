#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL fil e/i, o/u same-word pairs (Eric, 2026-11-06: quarantine, keep the more frequent spelling).
Rule, mechanical: in the Leipzig tgl_wikipedia_2021_100K corpus (CC BY 4.0) the kept spelling has at least 20 hits and at
least 3x the other's. Pairs under that bar are NOT touched; they go to reports/ortho-fil-vowel-auditor-sheet.csv.
bola/bula, noon/noong vs nuon/nuong, pares/paris are excluded as probably different words. Run once."""
import csv, collections, os, sys, tarfile, unicodedata
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
nfc = lambda s: unicodedata.normalize("NFC", s)
D, WHO, TAG = "2026-11-06", "Eric 2026-11-06", "CC-ORTHO-SWEEP-ALL: "
CACHE = os.environ.get("PURITY_CACHE") or os.path.expanduser("~/repos/spell-wasm/.corpus-cache")
f = collections.Counter()
with tarfile.open(f"{CACHE}/tgl_wikipedia_2021_100K.tar.gz") as t:
    m = next(x for x in t.getmembers() if x.name.endswith("-words.txt"))
    for l in t.extractfile(m).read().decode("utf-8").splitlines():
        p = l.split("\t")
        if len(p) >= 3: f[nfc(p[1]).lower()] += int(p[2])
SKIP = {"bola", "noon", "noong", "pares"}
pairs = [l.split("\t") for l in open(f"{REPO}/reports/ortho-census/fil-vowel-pairs.tsv", encoding="utf-8").read().split("\n")[1:] if l]
done, held, overrides, variants = [], [], [], []
for a, ta, b, tb, _ in pairs:
    if a in SKIP: continue
    fa, fb = f[a], f[b]; hi, lo = max(fa, fb), min(fa, fb)
    if hi >= 20 and hi >= 3 * max(lo, 1):
        keep, lose = (a, b) if fa >= fb else (b, a)
        ev = TAG + f"same word as {keep}; {keep} kept (more frequent: {f[keep]} vs {f[lose]} in Leipzig tgl_wikipedia_2021_100K); {WHO}"
        assert ps.apply_cuts("fil", {lose}, "variant_duplicate", ev, D), lose
        overrides.append(["fil", lose, "variant_duplicate", WHO, ev])
        variants.append(f"{keep}\t{lose}\tKWF Ortograpiyang Pambansa 2014: e/i and o/u alternate in Filipino spellings; the kept form is the more frequent in Leipzig tgl_wikipedia_2021_100K (CC BY 4.0), {f[keep]} vs {f[lose]}\te_i_o_u\n")
        done.append((keep, lose, f[keep], f[lose]))
    else:
        held.append((a, fa, b, fb))
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as fh:
    csv.writer(fh, lineterminator="\n").writerows(overrides)
ps.rebuild_and_rehash()
with open(f"{REPO}/data/fil/accepted-variants.tsv", "a", encoding="utf-8") as fh: fh.writelines(variants)
with open(f"{REPO}/reports/ortho-fil-vowel-auditor-sheet.csv", "w", newline="", encoding="utf-8") as fh:
    w = csv.writer(fh, lineterminator="\n")
    w.writerow(["spelling_a", "corpus_hits_a", "spelling_b", "corpus_hits_b", "same word? (yes/no)", "spelling to keep in the game"])
    w.writerows([list(h) + ["", ""] for h in held])
print("quarantined", [(k, l) for k, l, _, _ in done], "| held for the auditor:", len(held))
