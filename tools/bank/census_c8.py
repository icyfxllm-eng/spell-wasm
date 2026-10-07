#!/usr/bin/env python3
"""CC-BANK-PURITY §0 C8 — rows that are in CC-BANK-COMPLETE's U but in this file's quarantine classes.

Report only; the U-engine (src/bank_complete.rs) is not touched.  U is rebuilt here with the
engine's own first three gates (rank <= T4 floor, orthographic, profanity) from the Leipzig
frequency list the bank was built from.  Gates 4-5 are Pending in the engine, so U is the
PROVISIONAL set.  PROVISIONAL on C1.  Writes reports/bank-purity-c8.csv.
"""
import csv, json, os, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, ALL, leipzig_freq, load_bank, ROOT, find
from census_c6 import fold_lenient, load

CORPUS = {"en": "eng_wikipedia_2016_100K", "es": "spa_wikipedia_2021_100K", "fr": "fra_wikipedia_2021_100K",
          "de": "deu_wikipedia_2021_100K", "pt": "por_wikipedia_2021_100K", "pl": "pol_wikipedia_2021_100K",
          "ko": "kor_wikipedia_2021_100K", "ja": "jpn_wikipedia_2021_100K", "fil": "tgl_wikipedia_2021_100K",
          "ru": "rus_wikipedia_2021_100K", "ar": "ara_wikipedia_2021_100K", "hi": "hin_wikipedia_2021_100K",
          "sw": "swa_wikipedia_2021_100K"}
floors = json.load(open(f"{ROOT}/config/bank_floors.json"))["languages"]
nfc = lambda s: unicodedata.normalize("NFC", s)

def ranked(corpus):
    """Leipzig *-words.txt order is frequency order; rank = 1-based position of the cased token."""
    import tarfile
    with tarfile.open(find(corpus + ".tar.gz")) as t:
        for m in t.getmembers():
            if m.name.endswith("-words.txt"):
                out = []
                for line in t.extractfile(m).read().decode("utf-8").splitlines():
                    p = line.split("\t")
                    if len(p) >= 3:
                        out.append(p[1])
                return out
    return []

quar = {}
for r in csv.DictReader(open(f"{ROOT}/reports/bank-purity-rows.csv", encoding="utf-8")):
    if r["class"] in QUARANTINE:
        quar.setdefault(r["lang"], {})[nfc(r["word"].rsplit("|", 1)[-1])] = r["class"]

out, table = [], []
for lang in ALL:
    if lang not in CORPUS:
        continue
    t4 = floors[lang]["tiers"][-1]
    prof = load(f"{ROOT}/assets/words/profanity/{lang}.txt") or set()
    U = set()
    for rank, tok in enumerate(ranked(CORPUS[lang]), 1):
        if rank > t4:
            break
        tok = nfc(tok)
        if (not tok or any(c.isspace() or c.isdigit() or c == "." for c in tok) or any(c.isupper() for c in tok)):
            continue
        if fold_lenient(tok) in prof:
            continue
        U.add(tok)
    bank = {nfc(w.rsplit("|", 1)[-1]) for _, w in load_bank(lang)}
    hits = sorted(w for w in quar.get(lang, {}) if w in U)
    for w in hits:
        out.append((lang, w, quar[lang][w]))
    table.append((lang, t4, len(U), len(quar.get(lang, {})), len(hits)))
with open(f"{ROOT}/reports/bank-purity-c8.csv", "w", newline="", encoding="utf-8") as f:
    cw = csv.writer(f, lineterminator="\n"); cw.writerow(["lang", "word", "class"]); cw.writerows(out)
print("lang  T4floor  |U|(100K corpus ceiling)  quarantine rows  of which in U")
for t in table: print("%-4s %7d %9d %14d %14d" % t)
print("total in U:", sum(t[4] for t in table))
import random
random.Random(1).shuffle(out)
print("30 examples:", [(a, b, c) for a, b, c in sorted(out[:30])])
