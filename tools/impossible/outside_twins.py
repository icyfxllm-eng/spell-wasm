#!/usr/bin/env python3
"""CC-IMPOSSIBLE outside-twin sweep (Eric 2026-10-22): pool words that share a coarse sound key with a word
of a FULL dictionary that is not in our bank. The Impossible gates compare only against bank words; this
measures the wider I1 gap.

  outside_twins.py <lang> <wordlist>   (wordlist: one word per line, e.g. `unmunch` output; for ko the .dic)

Keys are the rules net (tools/impossible/rules_nets.py) for fr de pt pl ko fil sw, plus coarse keys for
es and ru defined here (those two use a SENSE-CUE table in the game, not the net). Over-flagging is expected:
every hit is a CANDIDATE for review, not a verdict. Writes reports/outside-twins/<lang>.csv.
"""
import collections, csv, json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(__file__))
import rules_nets as R

def key_es(w):
    w = unicodedata.normalize("NFC", w.lower())
    w = w.replace("ñ", "N")
    w = R.strip_marks(w).replace("N", "ñ")
    w = R.subs(w, [(r"gu(?=[ei])", "g"), (r"g(?=[ei])", "j"), (r"qu|c(?=[aoulr])|k", "k"), (r"c(?=[ei])|z", "s"), (r"ll", "y"), (r"v", "b"), (r"w", "u"),
                   (r"^h|h", ""), (r"x", "ks"), (r"rr", "r")])
    return R.collapse(w)

def key_ru(w):
    w = w.lower().replace("ё", "е")
    w = R.subs(w, [("[ъь]", ""), ("тс(?=я)", "ц"), ("(тьс|тс)я", "цa")])
    w = w.replace("б", "п") if w.endswith("б") else w
    for a, b in (("в", "ф"), ("г", "к"), ("д", "т"), ("ж", "ш"), ("з", "с"), ("б", "п")):
        if w.endswith(a): w = w[:-1] + b
    return R.collapse(w)

KEYS = dict(R.NETS); KEYS.update({"es": key_es, "ru": key_ru})

def inflection_pair(a, b):
    """True when the two differ only at the end (plural / verb ending / gender): an inflection, not a rival spelling."""
    a, b = a.lower(), b.lower()
    n = 0
    while n < min(len(a), len(b)) and a[n] == b[n]: n += 1
    return n >= min(len(a), len(b)) - 1 and n >= 4

def main(lang, path):
    key = KEYS[lang]
    pool = json.load(open(f"assets/impossible/impossible_pool.{lang}.json"))["words"]
    want = {key(unicodedata.normalize("NFC", p)) for p in pool}  # only keys a pool word has
    cased = lang == "de"
    full = collections.defaultdict(set)
    ok = re.compile(r"^[^\W\d_]+$", re.U)
    for line in open(path, encoding="utf8", errors="ignore"):
        w = line.strip().split("/")[0]
        if not w or not ok.match(w): continue
        if not cased and w[0].isupper(): continue  # proper nouns
        w = unicodedata.normalize("NFC", w)
        k = key(w)
        if k in want: full[k].add(w)
    rows = []
    for p in pool:
        pn = unicodedata.normalize("NFC", p)
        twins = sorted(t for t in full.get(key(pn), ()) if t.lower() != pn.lower())
        inner = [t for t in twins if not inflection_pair(pn, t)]
        if twins:
            rows.append((p, len(inner), " ".join((inner or twins)[:6]), "inflection-only" if not inner else "internal"))
    os.makedirs("reports/outside-twins", exist_ok=True)
    with open(f"reports/outside-twins/{lang}.csv", "w", newline="", encoding="utf8") as f:
        cw = csv.writer(f); cw.writerow(["pool_word", "n_internal_twins", "twins_sample", "kind"]); cw.writerows(rows)
    internal = sum(1 for r in rows if r[3] == "internal")
    print(f"{lang}: pool {len(pool)}; any outside same-key word {len(rows)} ({100*len(rows)/len(pool):.1f}%); internal-spelling twins (not mere inflections) {internal} ({100*internal/len(pool):.1f}%)")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
