#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL fil: apply the reviewer's per-pair verdict sheet (pasted by Eric 2026-11-06). Run once.
Merges (keep first, quarantine second, second an accepted variant): rows 1-8, 12, 13.
Quarantine only: espirito (16). Cut: babaeng, dae, dai (18, 20); babaing and lalakeng reclassified as linker forms.
kuponan and kabuoan (14, 19) were quarantined as variants earlier; they are now plain misspellings, so their variant rows go.
HELD (not touched): 9 komplikado/kumplikado and 10 politika/pulitika (Low: the reviewer says Paul decides), 11 follows 10,
17 sunod-sunod/sunud-sunod (accepted at Easy and Spell Jr only: variants carry no tier gate)."""
import csv, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, f"{REPO}/tools/bank")
import purity_sheet as ps
D, WHO, TAG = "2026-11-06", "Eric 2026-11-06 (fil reviewer sheet)", "CC-ORTHO-SWEEP-ALL: "
MERGE = [("istasyon","estasyon"),("istilo","estilo"),("istruktura","estruktura"),("iskultura","eskultura"),("kemikal","kimikal"),
         ("kompanya","kumpanya"),("kombinasyon","kumbinasyon"),("kompetisyon","kumpetisyon"),("tradisyonal","tradisyunal"),("inhinyeriya","inhenyeriya")]
overrides = []
def cut(w, klass, why):
    ev = TAG + why + "; " + WHO
    assert ps.apply_cuts("fil", {w}, klass, ev, D), w
    overrides.append(["fil", w, klass, WHO, ev])
for keep, lose in MERGE: cut(lose, "variant_duplicate", f"same word as {keep}; {keep} kept per the reviewer sheet, {lose} an accepted variant")
cut("espirito", "misspelled_nonstandard", "espiritu is the Filipino standard (Espiritu Santo); espirito is the Spanish/Portuguese spelling")
cut("babaeng", "not_a_dictionary_headword", "babae plus the -ng linker, not a headword; babae is stored")
cut("dae", "wrong_language_other", "not a Tagalog/Filipino word (reviewer: corpus leakage)")
cut("dai", "wrong_language_other", "not a Tagalog/Filipino word; dai is Bikol 'not' (reviewer: corpus leakage)")
with open(f"{REPO}/tools/bank/purity_overrides.csv", "a", newline="", encoding="utf-8") as f: csv.writer(f, lineterminator="\n").writerows(overrides)
# reclassify four rows quarantined earlier as variants
RE = {"babaing": ("not_a_dictionary_headword", "babae plus the -ng linker (old spelling), not a headword; babae is stored"),
      "lalakeng": ("not_a_dictionary_headword", "lalaki plus the -ng linker (old spelling), not a headword; lalaki is stored"),
      "kuponan": ("misspelled_nonstandard", "almost certainly a misspelling of koponan, not a variant"),
      "kabuoan": ("misspelled_nonstandard", "misspelling of kabuuan: buo plus a suffix becomes -uu- (2014 rules)")}
def rewrite(path, widx, cidx, eidx, hdr):
    rows = list(csv.reader(open(path, encoding="utf-8")))
    n = 0
    for r in rows:
        if len(r) > eidx and r[0] == "fil" and r[widx] in RE:
            r[cidx] = RE[r[widx]][0]; r[eidx] = TAG + RE[r[widx]][1] + "; " + WHO; n += 1
    csv.writer(open(path, "w", newline="", encoding="utf-8"), lineterminator="\n").writerows(rows); return n
a = rewrite(f"{REPO}/assets/words-quarantine/manifest.csv", 1, 4, 5, True); b = rewrite(f"{REPO}/tools/bank/purity_overrides.csv", 1, 2, 4, False)
assert (a, b) == (4, 4), (a, b)
ps.rebuild_and_rehash()
# variant file: drop the four conflicting rows, add the ten merges
p = f"{REPO}/data/fil/accepted-variants.tsv"
DROP = {("lalaking","lalakeng"),("babaeng","babaing"),("koponan","kuponan"),("kabuuan","kabuoan")}
lines = [l for l in open(p, encoding="utf-8").read().split("\n") if l and tuple(l.split("\t")[:2]) not in DROP]
SRC = "KWF Ortograpiyang Pambansa 2014: e/i and o/u alternate in Filipino spellings; the kept form follows the fil reviewer's verdict sheet (2026-11-06)"
lines += [f"{k}\t{l}\t{SRC}\te_i_o_u" for k, l in MERGE]
open(p, "w", encoding="utf-8").write("\n".join(lines) + "\n")
print("cut", len(overrides), "| reclassified", a, "| variants now", sum(1 for l in lines if not l.startswith("#")))
