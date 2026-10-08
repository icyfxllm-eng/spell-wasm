#!/usr/bin/env python3
"""CC-BANK-PURITY Phase A — F3 quarantine, F4 corrections, F5 German capitalisation, cascade.

    python3 tools/bank/phase_a.py [--date YYYY-MM-DD]

Reads reports/bank-purity-rows.csv (F1 output) and rewrites THIS checkout:
  assets/words/<lang>/<tier>.txt            kept rows (corrected / capitalised in place)
  assets/words-quarantine/<lang>/<tier>.txt moved rows, original order
  assets/words-quarantine/manifest.csv      lang,word,tier,pos,class,evidence,date   (pos = original line index)
  reports/bank-corrections.csv              F4: lang,old,new,tier,dictionary
  reports/bank-recapitalised.csv            F5: lang,old,new,tier
  config/gloss/<lang>.json, backend/def_pools/<lang>.json   rows keyed by a word that left the bank are pruned,
                                            rows keyed by a respelled / capitalised word are re-keyed
Idempotent only against a fresh F1 run: run purity_check.py first, then this once.
"""
import csv, json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from purity_check import QUARANTINE, ALL, TIERS, ROOT

nfc = lambda s: unicodedata.normalize("NFC", s)
date = sys.argv[sys.argv.index("--date") + 1] if "--date" in sys.argv else "2026-10-07"
DICTNAME = {"es": "es_ES", "fr": "fr", "de": "de_DE_frami", "pt": "pt_BR", "pl": "pl_PL", "ru": "ru_RU", "sw": "sw_TZ",
            "ko": "ko_KR", "ar": "ar", "hi": "hi_IN", "en": "en_US", "ja": "JMdict_e", "zh": "CC-CEDICT", "fil": "kaikki-tagalog"}
rows = list(csv.DictReader(open(f"{ROOT}/reports/bank-purity-rows.csv", encoding="utf-8")))
lines = {}
for l in ALL:
    for t in TIERS:
        p = f"{ROOT}/assets/words/{l}/{t}.txt"
        if os.path.exists(p):
            lines[(l, t)] = [x.strip() for x in open(p, encoding="utf-8").read().split("\n") if x.strip()]
bank = {l: {nfc(w) for (ll, _), ws in lines.items() if ll == l for w in ws} for l in ALL}

act = {}   # (lang, word) -> ("q", class, evidence) | ("fix", new, kind)
for r in rows:
    l, w, c, ev = r["lang"], nfc(r["word"]), r["class"], r["evidence"]
    if c in QUARANTINE or (c == "not_in_reference_dictionary" and l == "hi"):
        act[(l, w)] = ("q", c, ev)
    elif c == "misspelled_missing_diacritic":
        new = nfc(ev[2:].strip())
        act[(l, w)] = ("q", c, f"P11 duplicate of {new}") if new in bank[l] else ("fix", new, "diacritic")
    elif c == "traditional_character":
        py, _, han = w.partition("|")
        new = nfc(f"{py}|{ev}") if ev and "|" not in ev else None
        act[(l, w)] = ("q", c, f"simplified form {ev} is already a row") if (new is None or new in bank[l]) else ("fix", new, "simplified")
    elif c == "noun_shown_lowercase":
        act[(l, w)] = ("fix", w[:1].upper() + w[1:], "capital")

# a fix may land on a word another fix (or a kept row) also produces: first one wins, later ones are quarantined
quar_lines = {}; manifest = []; corrections = []; recap = []; rekey = {l: {} for l in ALL}; gone = {l: set() for l in ALL}
seen_l = {}
for (l, t), ws in lines.items():
    kept = []; seen = seen_l.setdefault(l, set())   # tiers are visited easy..expert, so the earliest tier wins
    for i, w in enumerate(ws):
        a = act.get((l, nfc(w)))
        new = w
        if a and a[0] == "q":
            quar_lines.setdefault((l, t), []).append(w); manifest.append((l, w, t, i, a[1], a[2], date)); gone[l].add(nfc(w).rsplit("|", 1)[-1]); continue
        if a and a[0] == "fix":
            new = a[1]
        if nfc(new) in seen:
            quar_lines.setdefault((l, t), []).append(w); manifest.append((l, w, t, i, "duplicate_after_fix", f"collides with {new}", date)); gone[l].add(nfc(w).rsplit("|", 1)[-1]); continue
        if a and a[0] == "fix":
            if a[2] == "capital": recap.append((l, w, new, t))
            else: corrections.append((l, w, new, t, DICTNAME[l]))
            rekey[l][nfc(w).rsplit("|", 1)[-1]] = nfc(new).rsplit("|", 1)[-1]
        seen.add(nfc(new)); kept.append(new)
    open(f"{ROOT}/assets/words/{l}/{t}.txt", "w", encoding="utf-8").write("\n".join(kept) + "\n")
for (l, t), ws in quar_lines.items():
    os.makedirs(f"{ROOT}/assets/words-quarantine/{l}", exist_ok=True)
    open(f"{ROOT}/assets/words-quarantine/{l}/{t}.txt", "w", encoding="utf-8").write("\n".join(ws) + "\n")
manifest.sort(key=lambda m: (ALL.index(m[0]), TIERS.index(m[2]), m[3]))
def write(path, head, data):
    with open(path, "w", newline="", encoding="utf-8") as f:
        cw = csv.writer(f, lineterminator="\n"); cw.writerow(head); cw.writerows(data)
write(f"{ROOT}/assets/words-quarantine/manifest.csv", "lang,word,tier,pos,class,evidence,date".split(","), manifest)
write(f"{ROOT}/reports/bank-corrections.csv", "lang,old,new,tier,dictionary".split(","), sorted(corrections, key=lambda r: (ALL.index(r[0]), r[3], r[1])))
write(f"{ROOT}/reports/bank-recapitalised.csv", "lang,old,new,tier".split(","), sorted(recap, key=lambda r: (ALL.index(r[0]), r[3], r[1])))

# cascade -------------------------------------------------------------------
stats = []; pruned_leak = []
for l in ALL:
    live = {nfc(w).rsplit("|", 1)[-1] for (ll, _), ws in lines.items() if ll == l for w in ws}   # old forms
    p = f"{ROOT}/config/gloss/{l}.json"
    if os.path.exists(p):
        d = json.load(open(p, encoding="utf-8")); out = {}
        for k, v in d["rows"].items():
            n = nfc(k)
            if n in gone[l] and n not in rekey[l]:
                continue
            nk = rekey[l].get(n, k)
            out.setdefault(nk, v)
        stats.append(("gloss", l, len(d["rows"]), len(out))); d["rows"] = out
        open(p, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False, indent=2))
    p = f"{ROOT}/backend/def_pools/{l}.json"
    if os.path.exists(p):
        d = json.load(open(p, encoding="utf-8")); before = after = 0
        for t, es in d["tiers"].items():
            seen, out = set(), []
            for e in es:
                before += 1; n = nfc(e["word"])
                if n in gone[l] and n not in rekey[l]:
                    continue
                if n in rekey[l]:
                    # the definition was written for the wrong spelling ("Swiss spelling of <new>"); if it now names its own
                    # key it is a self-leak, so the row is pruned and the pool rebuild refills it
                    if re.search(r"(?<!\w)" + re.escape(rekey[l][n]) + r"(?!\w)", e["definition"], re.I):
                        pruned_leak.append((l, e["word"], rekey[l][n])); continue
                e["word"] = rekey[l].get(n, e["word"])
                if nfc(e["word"]) in seen: continue
                seen.add(nfc(e["word"])); out.append(e)
            d["tiers"][t] = out; after += len(out)
        stats.append(("def_pools", l, before, after))
        open(p, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False))
print(f"quarantined {len(manifest)}, corrected {len(corrections)}, recapitalised {len(recap)}")
print("def-pool rows pruned as self-leaks after respelling:", len(pruned_leak))
for s in stats: print("%-10s %-4s %6d -> %6d" % s)
