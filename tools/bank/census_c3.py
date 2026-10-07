#!/usr/bin/env python3
"""CC-BANK-PURITY §0 C3 — consumers of a bank word.  Read-only.

For every file keyed by a bank word, count entries that reference a word the current
purity run would quarantine (Q), respell (R: diacritic fix / traditional zh), or
re-capitalise (D: de nouns).  Uses reports/bank-purity-rows.csv, so it is PROVISIONAL
until C1's HALT is resolved.  Writes reports/bank-purity-c3.csv.
"""
import csv, glob, json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, ALL, load_bank, ROOT

nfc = lambda s: unicodedata.normalize("NFC", s)
Q, R, D = {l: set() for l in ALL}, {l: {} for l in ALL}, {l: set() for l in ALL}
rows = list(csv.DictReader(open(f"{ROOT}/reports/bank-purity-rows.csv", encoding="utf-8")))
bank = {l: {nfc(w.rsplit("|", 1)[-1]) for _, w in load_bank(l)} for l in ALL}
for r in rows:
    l, w, c = r["lang"], nfc(r["word"].rsplit("|", 1)[-1]), r["class"]
    if c in QUARANTINE:
        Q[l].add(w)
    elif c == "misspelled_missing_diacritic":
        new = r["evidence"][2:].strip()
        if new in bank[l]:
            Q[l].add(w)          # P11: correct spelling is already a row, so this one is quarantined
        else:
            R[l][w] = new
    elif c == "traditional_character":
        R[l][w] = r["evidence"]
    elif c == "noun_shown_lowercase":
        D[l].add(w)
    elif c == "not_in_reference_dictionary" and l == "hi":
        Q[l].add(w)   # P8

def tag(l, w):
    w = nfc(w)
    return "Q" if w in Q[l] else "R" if w in R[l] else "D" if w in D[l] else None

out = []
def count(consumer, l, items):
    items = [nfc(i) for i in items]
    c = {"Q": 0, "R": 0, "D": 0}
    for i in items:
        t = tag(l, i)
        if t: c[t] += 1
    out.append((consumer, l, len(items), c["Q"], c["R"], c["D"]))

for l in ALL + ["vi"]:
    if l == "vi": continue
    p = f"{ROOT}/backend/def_pools/{l}.json"
    if os.path.exists(p):
        d = json.load(open(p, encoding="utf-8"))
        count("def_pools", l, [e["word"] for t in d["tiers"].values() for e in t])
    p = f"{ROOT}/config/gloss/{l}.json"
    if os.path.exists(p):
        count("gloss", l, list(json.load(open(p, encoding="utf-8"))["rows"]))
    p = f"{ROOT}/config/practice/{l}.json"
    if os.path.exists(p):
        d = json.load(open(p, encoding="utf-8"))
        ws = d.get("words", []) + d.get("difficult", []) + [b["alt"] for b in d.get("choiceBeats", []) if "alt" in b]
        count("practice", l, ws)

# audio-verdicts: keys "lang|word|variant"
av = json.load(open(f"{ROOT}/config/audio-verdicts.json", encoding="utf-8"))["clips"]
by = {}
for k in av:
    p = k.split("|")
    if len(p) >= 2: by.setdefault(p[0], []).append(p[1])
for l, ws in sorted(by.items()):
    if l in Q: count("audio_verdicts", l, ws)

# ru stress table
ru = re.findall(r'\("([^"]+)", \d+\)', open(f"{ROOT}/src/ru_stress_data.rs", encoding="utf-8").read())
count("ru_stress_data.rs", "ru", ru)

# human audio manifests (en only today)
for p in glob.glob(f"{ROOT}/assets/human-audio/*/*.json") + glob.glob(f"{ROOT}/assets/human-audio/*.json"):
    s = open(p, encoding="utf-8").read()
    lang = os.path.basename(os.path.dirname(p))
    if lang in Q:
        try:
            d = json.load(open(p, encoding="utf-8"))
        except Exception:
            continue
        ws = set(re.findall(r'"([^"\s]{1,40})"', s))
        count("human_audio:" + os.path.relpath(p, ROOT), lang, [w for w in ws if w in bank[lang]])

# zh sandhi / polyphone configs: strings that are bank hanzi
for f in ["zh-sandhi-audit.json", "zh-polyphone-set.json", "zh-loopback-result.json", "zh-tone-probe-words.json", "zh-tone-probe-result.json"]:
    p = f"{ROOT}/config/{f}"
    if os.path.exists(p):
        ws = [w for w in re.findall(r'"([^"]+)"', open(p, encoding="utf-8").read()) if nfc(w) in bank["zh"]]
        count(f, "zh", ws)

# word ids / list hashes: every changed tier changes TIER_HASHES; respelled and re-capitalised words change IDs
for l in ALL:
    n_ids = len(R[l]) + len(D[l])
    out.append(("word_id_changes (ghosts/decks/daily key by id)", l, len(bank[l]), len(Q[l]), len(R[l]), len(D[l])))

with open(f"{ROOT}/reports/bank-purity-c3.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n"); w.writerow(["consumer", "lang", "entries", "to_quarantine", "to_respell", "to_recapitalise"]); w.writerows(out)
print("%-44s %-4s %7s %5s %5s %5s" % ("consumer", "lang", "entries", "Q", "R", "D"))
tot = {}
for o in out:
    if o[0].startswith("word_id"): continue
    print("%-44s %-4s %7d %5d %5d %5d" % o)
    t = tot.setdefault(o[0].split(":")[0], [0, 0, 0, 0]); [t.__setitem__(i, t[i] + o[2 + i]) for i in range(4)]
print(); print(tot)
