#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL F0.3: write tests/ortho/golden/<lang>.tsv for es, fr, pt.

Columns: lane, target, typed, tier, voice_id, expected, note. tier is easy|medium|hard|expert|jr.
expected is one of correct | correct_other_sense | close | miss | pending_table.
  pending_table = the spec's answer depends on a CC-SENSE-CUE table that does not hold the pair yet (fr and pt have no
  table; es lacks pairs whose words are not both in the bank) or that disagrees with the voice (es casa/caza, see note).
  The Rust test asserts such a row is NOT graded correct by the profile and that the table does not accept it as
  correct_other_sense today, so the day the table lands the row fails and must be re-labelled.

Expected values are derived from the spec rules (F0.2 and the language sections), not from running the grader.
Deterministic: the same banks give the same files.
"""
import json, os, random, unicodedata
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TIERS = "easy medium hard expert".split()
nfc = lambda s: unicodedata.normalize("NFC", s)
nfd = lambda s: unicodedata.normalize("NFD", s)
VOICE = {"es": "es-ES-Neural2-G", "fr": "fr-FR-Neural2-F", "pt": "pt-BR-Neural2-B"}
KEEP = {"es": "ñ", "fr": "", "pt": "ç"}     # letters_not_marks

def bank(lang):
    d = {}
    for t in TIERS:
        for w in open(f"{REPO}/assets/words/{lang}/{t}.txt", encoding="utf-8").read().split("\n"):
            if w.strip(): d[nfc(w).lower()] = t
    return d

def strip(w, keep):
    out = []
    for c in w:
        if c in keep: out.append(c); continue
        out.append("".join(d for d in nfd(c) if not "̀" <= d <= "ͯ"))
    return nfc("".join(out))

def variants(lang):
    p = f"{REPO}/data/{lang}/accepted-variants.tsv"
    if not os.path.exists(p): return []
    return [l.split("\t")[:2] for l in open(p, encoding="utf-8").read().split("\n") if l and not l.startswith("#")]

def table_pairs(lang):
    d = json.load(open(f"{REPO}/assets/words/{lang}/soundalikes.json", encoding="utf-8"))
    pairs = set()
    for s in d["sets"]:
        if s["kind"] != "true": continue
        ms = [m["memberId"] for m in s["members"]]
        for a in ms:
            for b in ms:
                if a != b: pairs.add((a, b))
    return pairs

def build(lang):
    B = bank(lang); V = VOICE[lang]; keep = KEEP[lang]
    rows = []
    add = lambda lane, tgt, typed, tier, exp, note="": rows.append((lane, tgt, typed, tier, V, exp, note))
    rnd = random.Random(f"ortho-golden-{lang}")
    # ---- homophone lane ----
    rules = json.load(open(f"{REPO}/data/{lang}/sound-alike-rules.json", encoding="utf-8"))
    kind = {r["id"]: r["kind"] for r in rules["rules"]}
    tp = table_pairs(lang)
    for s in rules["seeds"]:
        ws, rule = s["words"], s["rule"]
        for a in ws:
            for b in ws:
                if a == b: continue
                tier = B.get(a, "medium")
                if kind[rule] == "accent":
                    note = f"{rule} is an accent rule for the {V} voice: not a homophone pair"
                    if (a, b) in tp: add("homophone", a, b, tier, "pending_table", note + "; the current SENSE-CUE table still lists it as a true set (it was built for a seseante voice)")
                    else: add("homophone", a, b, tier, "miss", note)
                elif (a, b) in tp: add("homophone", a, b, tier, "correct_other_sense")
                else: add("homophone", a, b, tier, "pending_table", "pair not in the SENSE-CUE table yet (words not both in the bank, or no generator for this language)")
    pool = sorted(w for w in B if w.isalpha() and 4 <= len(w) <= 7)
    for w in rnd.sample(pool, 14):   # controls: one letter changed, not a homophone, not a row
        i = rnd.randrange(1, len(w) - 1)
        typed = w[:i] + ("k" if w[i] != "k" else "m") + w[i + 1:]
        if typed in B: continue
        add("homophone", w, typed, B[w], "miss", "control: one letter changed, not a same-sound pair")
    # ---- variant lane ----
    for st, ac in variants(lang):
        n = nfc(st).lower()
        if n not in B: continue
        mangled = ac[:-1] + ("x" if ac[-1] != "x" else "z")
        for tier in TIERS + ["jr"]:
            add("variant", st, ac, tier, "correct", "listed variant")
            add("variant", st, st, tier, "correct", "the stored form")
            add("variant", st, mangled, tier, "miss", "an unlisted spelling")
    # ---- friction lane ----
    marked = sorted(w for w in B if strip(w, "") != w and " " not in w and "-" not in w)
    sample = rnd.sample(marked, min(40, len(marked)))
    for w in sample:
        t = B[w]
        st_all = strip(w, keep)
        for tier in (t, "jr"):
            if st_all == w:
                continue
            row = st_all in B
            if tier == "jr": exp = "miss" if row else "correct"
            else:
                exp = "miss" if row else "close"
            add("friction", w, st_all, tier, exp, "marks left out" + (" (typed form is itself a bank row)" if row else ""))
    # ñ / ç are letters: dropping the mark is another letter at standard tiers; Spell Jr treats it as a mark (D14)
    if keep:
        base = {"ñ": "n", "ç": "c"}[keep]
        letter_rows = sorted(w for w in B if keep in w and " " not in w)
        for w in rnd.sample(letter_rows, min(10, len(letter_rows))):
            typed = strip(w, "").replace(keep, base)
            row = typed in B
            add("friction", w, typed, B[w], "miss", f"{keep} is a letter, not a mark")
            add("friction", w, typed, "jr", "miss" if row else "correct", f"{keep} is a mark for Spell Jr" + (" (typed form is itself a bank row)" if row else ""))
    for w in rnd.sample(sorted(B), 12):
        add("friction", w, w.upper(), B[w], "correct", "capital letters fold")
        add("friction", w, " " + w + " ", B[w], "correct", "edge spaces are trimmed")
        add("friction", w, nfd(w), B[w], "correct", "NFD input grades as NFC")
    if lang == "fr":
        for w in sorted(x for x in B if "œ" in x or "æ" in x)[:6]:
            typed = w.replace("œ", "oe").replace("æ", "ae")
            row = typed in B
            for tier in TIERS + ["jr"]:
                if row: exp = "miss"
                elif tier in ("easy", "jr"): exp = "correct"
                else: exp = "close"
                add("friction", w, typed, tier, exp, "oe/ae stand-in for œ/æ")
    # Eric 2026-11-06 (CC-ORTHO-FIX-FIL-ROMANCE 3): real accent-only pairs from the banks. The table decides same-sound pairs;
    # pending_table = the spec answer (correct as valid_other_sense) waits on SENSE-CUE holding the pair. Different sound = miss.
    for a, b, exp, note in {
        "es": [("dónde", "donde", "pending_table", "tilde-only, same sound: the es sound-alike table decides (it holds no tilde-only pairs yet)"),
               ("donde", "dónde", "pending_table", "tilde-only, same sound: the table decides"),
               ("quién", "quien", "pending_table", "tilde-only, same sound: the table decides"),
               ("quien", "quién", "pending_table", "tilde-only, same sound: the table decides")],
        "fr": [("classé", "classe", "miss", "different sound"), ("classe", "classé", "miss", "different sound")],
        "pt": [("porquê", "porque", "pending_table", "the table decides; pair tagged for the Gig B Same sound? column"),
               ("porque", "porquê", "pending_table", "the table decides; pair tagged for the Gig B Same sound? column")],
    }.get(lang, []):
        if nfc(a) in B and nfc(b) in B:
            for tier in (B[nfc(a)], "jr"):
                add("friction", a, b, tier, exp, note)
    return rows

for lang in ("es", "fr", "pt"):
    rows = build(lang)
    lanes = {r[0] for r in rows}
    path = f"{REPO}/tests/ortho/golden/{lang}.tsv"
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write("# CC-ORTHO-SWEEP-ALL F0.3 golden file, generated by tools/ortho/make_golden.py. lane\ttarget\ttyped\ttier\tvoice_id\texpected\tnote\n")
        for r in rows: f.write("\t".join(r) + "\n")
    from collections import Counter
    print(lang, len(rows), dict(Counter(r[0] for r in rows)), dict(Counter(r[5] for r in rows)))
