#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL Phase 1 cleanups (Eric's section 0 resolutions, 2026-11-05).

Applies, through the existing PURITY machinery (tools/bank/purity_sheet.py apply_cuts + rebuild_and_rehash):
  ru   S3   22 same-word e-rows quarantined as yo_duplicate (6 different-word pairs kept)
  fr        evenement/evenement: keep the 1990 form, quarantine the other (variant)
  vi        quarantine the typo row; hi/hy keep one row (variant)
  fil       lalake quarantined (variant of lalaki)
  hi        7 non-nukta rows quarantined (variants of the nukta rows)
  pt        pre-AO90 / PT-PT stored forms renamed to BR AO90 (old form a variant); seccao quarantined as a duplicate of secao
  zh        2 same-hanzi groups: one row each quarantined
  sw        8 ng' rows added from the Leipzig sw corpus, each confirmed by Hunspell sw_TZ (cross-check only)
Rows are moved, never deleted. After the run by hand: pt deem ledger line pending -> dictionary (hunspell pt_BR accepts it) and
deem removed from grandfathered/pt.txt; scripts/build-wordlists.py keeps U+0027 for sw. Run once, from a clean checkout of this branch.
"""
import csv, json, os, re, sys, tarfile, unicodedata
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
nfc = lambda s: unicodedata.normalize("NFC", s)
D, WHO, TAG = "2026-11-05", "Eric 2026-11-05", "CC-ORTHO-SWEEP-ALL: "
TIERS = "easy medium hard expert".split()

def bank(lang):
    out = {}
    for t in TIERS:
        p = f"{REPO}/assets/words/{lang}/{t}.txt"
        if os.path.exists(p):
            for w in open(p, encoding="utf-8").read().split("\n"):
                if w.strip(): out[nfc(w)] = t
    return out

overrides, report = [], []
def cut(lang, words, klass, why):
    b = bank(lang)
    for w in words: assert nfc(w) in b, f"{lang} {w!r} is not a bank row"
    for w in words:
        ev = TAG + why[w] + "; " + WHO
        lp = f"{REPO}/assets/words/purity/{lang}.tsv"
        made = not os.path.exists(lp)          # vi has no PURITY ledger (it is not in purity_gate.LANGS); apply_cuts needs a file to rewrite
        if made: open(lp, "w", encoding="utf-8").write("# placeholder removed after the cut\n")
        got = ps.apply_cuts(lang, {w}, klass, ev, D)
        if made: os.remove(lp)
        assert got, (lang, w)
        overrides.append([lang, w, klass, WHO, ev]); report.append((lang, w, b[nfc(w)], klass, why[w]))

# ---------------- ru S3: the 28 pairs ----------------
KEEP_BOTH = {"все", "всем", "нем", "перед", "узнает", "чем"}   # different words: все/всё, всем/всём, нем (mute)/нём, перед/перёд, узнает/узнаёт, чем/чём
groups = [l.rstrip("\n").split("\t") for l in open(f"{REPO}/reports/ortho-census/c3-collisions-ru.tsv", encoding="utf-8")][1:]
ru_verdicts = []
dups = {}
for norm, ids, stored, tiers in groups:
    members = [x.strip() for x in stored.split(" | ")]
    e_rows = [m for m in members if "ё" not in m]; yo_rows = [m for m in members if "ё" in m]
    assert len(e_rows) == 1 and len(yo_rows) == 1, members
    if norm in KEEP_BOTH:
        ru_verdicts.append((yo_rows[0], e_rows[0], "keep both: different words")); continue
    dups[e_rows[0]] = f"same word as {yo_rows[0]}; the yo row is kept"
    ru_verdicts.append((yo_rows[0], e_rows[0], f"quarantine {e_rows[0]}: yo_duplicate"))
assert len(ru_verdicts) == 28 and len(dups) == 22, (len(ru_verdicts), len(dups))
cut("ru", list(dups), "yo_duplicate", dups)

# ---------------- cleanups ----------------
cut("fr", ["événement"], "variant_duplicate", {"événement": "traditional form of évènement; the 1990 form is kept (Rectifications 1990), the other is an accepted variant"})
cut("vi", ["đựơc"], "misspelled_nonstandard", {"đựơc": "typo for được (ơ and the dot transposed); được is kept"})
cut("vi", ["hý"], "variant_duplicate", {"hý": "i/y variant of hí; hí kept (assumed: i form), hý an accepted variant"})
cut("fil", ["lalake"], "variant_duplicate", {"lalake": "legacy vowel spelling of lalaki; the KWF form is kept, lalake an accepted variant"})

# hi: nukta pairs, generated from the bank so the code points are the bank's own
hib = bank("hi"); NUK = "़"
def strip_nukta(w):
    d = unicodedata.normalize("NFD", w).replace(NUK, "")
    return nfc(d)
pairs = []
for w in hib:
    nuk = unicodedata.normalize("NFD", w)
    if NUK in nuk and strip_nukta(w) in hib and strip_nukta(w) != w: pairs.append((w, strip_nukta(w)))
assert len(pairs) == 7, pairs
cut("hi", [b for a, b in pairs], "variant_duplicate", {b: f"no-nukta spelling of {a}; the nukta row is kept (Central Hindi Directorate), {b} an accepted variant" for a, b in pairs})
SRC = "Central Hindi Directorate, देवनागरी लिपि तथा हिंदी वर्तनी का मानकीकरण: nukta on Perso-Arabic and English loans is optional"
_hv = open(f"{REPO}/data/hi/accepted-variants.tsv", encoding="utf-8").read().split("\n")[:3]
open(f"{REPO}/data/hi/accepted-variants.tsv", "w", encoding="utf-8").write(
    "\n".join(_hv) + "\n" + "".join(f"{a}\t{b}\t{SRC}\tnukta_loan\n" for a, b in pairs))

# pt: rename in place to the BR AO90 form, quarantine the duplicate
def rename(lang, old, new):
    b = bank(lang); assert old in b and new not in b, (old, new)
    t = b[old]
    p = f"{REPO}/assets/words/{lang}/{t}.txt"
    L = open(p, encoding="utf-8").read().split("\n")
    L[[nfc(x) for x in L].index(old)] = new
    open(p, "w", encoding="utf-8").write("\n".join(L))
    lp = f"{REPO}/assets/words/purity/{lang}.tsv"
    L = open(lp, encoding="utf-8").read().split("\n")
    for i, x in enumerate(L):
        if x and x[0] != "#" and nfc(x.split("\t")[0]) == old: L[i] = new + x[len(x.split("\t")[0]):]
    open(lp, "w", encoding="utf-8").write("\n".join(L))
    gp = f"{REPO}/config/gloss/{lang}.json"
    if os.path.exists(gp):
        d = json.load(open(gp, encoding="utf-8"))
        if old in d["rows"]: d["rows"][new] = d["rows"].pop(old)
        open(gp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False, indent=2))
    dp = f"{REPO}/backend/def_pools/{lang}.json"
    if os.path.exists(dp):
        d = json.load(open(dp, encoding="utf-8"))
        for es in d["tiers"].values():
            for e in es:
                if nfc(e["word"]) == old: e["word"] = new
        open(dp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False))
    report.append((lang, old, t, "renamed_to_" + new, "stored form becomes BR AO90; the old form is an accepted variant"))
for old, new in [("pára", "para"), ("dêem", "deem"), ("facto", "fato")]: rename("pt", old, new)
cut("pt", ["secção"], "variant_duplicate", {"secção": "European spelling of seção, already a row; seção kept, secção an accepted variant"})

# zh: same-hanzi groups (pinyin homophones are exempt from I1; identical hanzi are not)
cut("zh", ["hu2li2|狐狸", "you3zhu4|有着"], "duplicate_hanzi", {"hu2li2|狐狸": "same hanzi as hu2li5|狐狸 (hú·li, neutral second syllable is the standard reading)", "you3zhu4|有着": "same hanzi as you3zhe5|有着 (着 is neutral zhe5 here)"})

# ---------------- sw: ng' rows from the Leipzig sw corpus ----------------
CACHE = os.environ.get("PURITY_CACHE") or os.path.expanduser("~/repos/spell-wasm/.corpus-cache")
freq = {}
with tarfile.open(f"{CACHE}/swa_wikipedia_2021_100K.tar.gz") as t:
    m = next(x for x in t.getmembers() if x.name.endswith("-words.txt"))
    for line in t.extractfile(m).read().decode("utf-8").splitlines():
        p = line.split("\t")
        if len(p) >= 3 and "ng" in p[1]:
            w = nfc(p[1]).replace("’", "'").replace("‘", "'")
            if "ng'" in w and w == w.lower(): freq[w] = freq.get(w, 0) + int(p[2])
CAND = ["ng'ombe", "ng'ambo", "kung'aa", "kung'amua", "kunung'unika", "mng'ao", "mng'aro", "king'ora"]   # singular nouns and ku- infinitives
sw = bank("sw")
dic = set(l.split("/")[0].strip().lower() for l in open(f"{CACHE}/dicts/sw/sw_TZ.dic", encoding="utf-8").read().split("\n")[1:])
TIER_LEN = {"easy": (2, 4), "medium": (5, 6), "hard": (7, 8), "expert": (9, 15)}
added = []
for w in CAND:
    assert freq.get(w, 0) >= 3 and w in dic and w not in sw, (w, freq.get(w), w in dic)
    n = sum(1 for c in w if unicodedata.category(c) not in ("Mn", "Mc"))
    tier = next(t for t, (lo, hi) in TIER_LEN.items() if lo <= n <= hi)
    added.append((w, tier, freq[w]))
for w, tier, f in added:
    p = f"{REPO}/assets/words/sw/{tier}.txt"
    open(p, "a", encoding="utf-8").write(w + "\n")
    report.append(("sw", w, tier, "added", f"ng' row from Leipzig swa_wikipedia_2021_100K (freq {f}, curly forms folded); Hunspell sw_TZ confirms"))
# ledger: one line per bank row in bank order
lp = f"{REPO}/assets/words/purity/sw.tsv"
old = open(lp, encoding="utf-8").read().split("\n")
head = [l for l in old if l.startswith("#")]; lines = {nfc(l.split("\t")[0]): l for l in old if l and l[0] != "#"}
for w, _, _ in added: lines[w] = f"{w}\tdictionary\tsw_TZ"
out = list(head)
sb = bank("sw")
for t in TIERS:
    p = f"{REPO}/assets/words/sw/{t}.txt"
    if os.path.exists(p):
        for w in open(p, encoding="utf-8").read().split("\n"):
            if w.strip(): out.append(lines[nfc(w)])
open(lp, "w", encoding="utf-8").write("\n".join(out) + "\n")

with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f:
    csv.writer(f, lineterminator="\n").writerows(overrides)
ps.rebuild_and_rehash()
with open(f"{REPO}/reports/ortho-census/quarantine-batch1.tsv", "w", encoding="utf-8") as f:
    f.write("lang\tword\ttier\tclass\treason\n")
    for r in report: f.write("\t".join(r) + "\n")
with open(f"{REPO}/reports/ortho-census/ru-yo-pairs.tsv", "w", encoding="utf-8") as f:
    f.write("yo_row\te_row\tverdict\n")
    for r in ru_verdicts: f.write("\t".join(r) + "\n")
print(len(overrides), "rows quarantined;", len(added), "sw rows added;", "renamed", sum(1 for r in report if r[3].startswith("renamed")))
