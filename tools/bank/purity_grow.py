#!/usr/bin/env python3
"""CC-BANK-PURITY F6 companion — grow banks to the signed pool floors and give the new rows their ledger lines.

    python3 tools/bank/purity_grow.py [--tiers hard,expert] <lang> [lang ...]

Runs `scripts/build-bigbank.py --to-floors` (candidates from the language's pinned corpus list, each tier up to its
poolFloor and never past it, the F1 purity admit gate in front of every write, quarantined words never re-admitted),
then writes a `dictionary` ledger line for every row that was not on the ledger, in bank order, signed with the
language's own dictionary source id, and refreshes the ledger tier hashes and src/word_data.rs.
It adds nothing for a tier already at or above its floor.  Easy is excluded unless --tiers names it: a frequency list
fills 2-4 letter Easy with function words (de la que, der und die), which CC-BANK-REBUILD R9 forbids; the content-word
filter needs part-of-speech data this path does not have.  It is not the Korean, Japanese or Hindi rebuild
(CC-BANK-REBUILD): those need dictionary headword sources.
"""
import collections, os, subprocess, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import purity_gate as g
import purity_sheet as ps

nfc = lambda s: unicodedata.normalize("NFC", s)

def main(langs, tiers=None):
    before = {l: {t: len(g.lines(f"{REPO}/assets/words/{l}/{t}.txt")) for t in g.TIERS} for l in langs}
    subprocess.run([sys.executable, f"{REPO}/scripts/build-bigbank.py", "--to-floors", *(["--tiers", tiers] if tiers else []), *langs], check=True, cwd=REPO)
    for l in langs:
        lp = f"{REPO}/assets/words/purity/{l}.tsv"
        old = open(lp, encoding="utf-8").read().split("\n")
        top = [x for x in old if x.startswith("#")]
        by = {nfc(x.split("\t")[0]): x for x in old if x and x[0] != "#"}
        src = collections.Counter(x.split("\t")[2] for x in by.values() if x.split("\t")[1] == "dictionary").most_common(1)[0][0]
        body, new = [], 0
        for t, w in g.bank(REPO, l):
            k = nfc(w)
            if k in by:
                body.append(by[k])
            else:
                body.append(f"{w}\tdictionary\t{src}"); new += 1
        open(lp, "w", encoding="utf-8").write("\n".join(top + body) + "\n")
        after = {t: len(g.lines(f"{REPO}/assets/words/{l}/{t}.txt")) for t in g.TIERS}
        print(f"{l}: +{new} rows  " + " ".join(f"{t[0]}:{before[l][t]}->{after[t]}" for t in g.TIERS))
    ps.rebuild_and_rehash()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    a = sys.argv[1:]
    t = "medium,hard,expert"   # Easy is never grown here: frequency fills it with function words (CC-BANK-REBUILD R9)
    if a[0] == "--tiers":
        t, a = a[1], a[2:]
    main(a, t)
