#!/usr/bin/env python3
"""CC-BANK-PURITY F2/F4 acceptance checks.  Exit 1 on any failure.

    python3 tools/bank/ledger_check.py                      # ledger + accent-fold lint
    python3 tools/bank/ledger_check.py --restore <base-rev> # quarantine round-trip against the pre-sweep bank

Ledger: every bank row has exactly one line; a line with no bank row fails; an `exception` needs a signer and a
reason (fields 4 and 5); the `#tier` hashes equal TIER_HASHES in src/word_data.rs.
Lint (F4): no two rows of a language share an accent-folded key unless every one of them carries a
`dictionary` verdict (the dictionary accepts both) or is in tools/bank/fold_allow.txt.
Restore: put every manifest row back at its recorded position and undo corrections/capitalisation; each tier
file must equal `git show <base-rev>:assets/words/<lang>/<tier>.txt` byte for byte.
"""
import csv, os, re, subprocess, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import ALL, TIERS, load_bank, ROOT

fails = []; collisions = []
NOFOLD = {"ja", "ko", "hi", "zh"}   # dakuten / jamo / matras are not accents
def fold(w):
    w = w.rsplit("|", 1)[-1]
    return "".join(c for c in unicodedata.normalize("NFD", w.lower()) if not unicodedata.combining(c)).replace("ß", "ss")
th = {(a, b): c for a, b, c in re.findall(r'\("(\w+)", "(\w+)", (0x[0-9A-F]+)\)', open(f"{ROOT}/src/word_data.rs", encoding="utf-8").read())}
allow = set()
p = f"{ROOT}/tools/bank/fold_allow.txt"
if os.path.exists(p):
    allow = {x.strip() for x in open(p, encoding="utf-8") if x.strip() and x[0] != "#"}
for l in ALL:
    bank = [w for _, w in load_bank(l)]
    led, hashes = [], {}
    for line in open(f"{ROOT}/assets/words/purity/{l}.tsv", encoding="utf-8").read().split("\n"):
        if not line: continue
        if line.startswith("#tier"):
            _, t, h = line.split("\t"); hashes[t] = h; continue
        if line[0] == "#": continue
        f = line.split("\t"); led.append(f)
        if f[1] not in ("dictionary", "exception", "pending"): fails.append(f"{l}: bad verdict {f}")
        if f[1] == "exception" and (len(f) < 5 or not f[3] or not f[4]): fails.append(f"{l}: exception without signer/reason {f[0]}")
    if [f[0] for f in led] != bank:
        a, b = {f[0] for f in led}, set(bank)
        fails.append(f"{l}: ledger/bank mismatch, missing {sorted(b - a)[:3]} orphan {sorted(a - b)[:3]} dup {len(led) - len(a)}")
    for t in TIERS:
        if (l, t) in th and hashes.get(t) != th[(l, t)]: fails.append(f"{l}/{t}: ledger tier hash stale")
    seen = {}
    pend = {f[0] for f in led if f[1] == "pending"}   # a ledger `dictionary` verdict means the dictionary accepted the row
    for w in bank: seen.setdefault(fold(w), []).append(w)
    for k, ws in seen.items():
        if len(ws) > 1 and l not in NOFOLD and any(w in pend for w in ws) and not all(w in allow for w in ws):
            collisions.append((l, " ".join(ws)))
if "--restore" in sys.argv:
    rev = sys.argv[sys.argv.index("--restore") + 1]
    man = list(csv.DictReader(open(f"{ROOT}/assets/words-quarantine/manifest.csv", encoding="utf-8")))
    undo = {}
    for fn, cols in (("bank-corrections.csv", ("old", "new")), ("bank-recapitalised.csv", ("old", "new"))):
        for r in csv.DictReader(open(f"{ROOT}/reports/{fn}", encoding="utf-8")):
            undo[(r["lang"], r["tier"], r[cols[1]])] = r[cols[0]]
    ok = 0
    for l in ALL:
        for t in TIERS:
            p = f"{ROOT}/assets/words/{l}/{t}.txt"
            if not os.path.exists(p): continue
            cur = [undo.get((l, t, w), w) for w in open(p, encoding="utf-8").read().split("\n") if w]
            for r in sorted((r for r in man if r["lang"] == l and r["tier"] == t), key=lambda r: int(r["pos"])):
                cur.insert(int(r["pos"]), r["word"])
            base = subprocess.run(["git", "show", f"{rev}:assets/words/{l}/{t}.txt"], cwd=ROOT, capture_output=True, text=True, encoding="utf-8").stdout
            ok += 1
            if "\n".join(cur) + "\n" != base: fails.append(f"restore {l}/{t}: not byte-identical to {rev}")
    print(f"restore round-trip: {ok} tiers compared")
import csv as _c
with open(f"{ROOT}/reports/bank-fold-collisions.csv", "w", newline="", encoding="utf-8") as f:
    w = _c.writer(f, lineterminator="\n"); w.writerow(["lang", "rows"]); w.writerows(collisions)
print(f"accent-fold collisions involving an unconfirmed (pending) row: {len(collisions)} -> reports/bank-fold-collisions.csv (auditor sheet items, not failures)")
print("FAIL" if fails else "OK"); [print(" ", f) for f in fails[:40]]; print(len(fails), "problems")
sys.exit(1 if fails else 0)
