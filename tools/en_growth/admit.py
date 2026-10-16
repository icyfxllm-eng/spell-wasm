#!/usr/bin/env python3
"""F3 -- admission of new English rows into STAGING.  Run with .corpus-cache/wf/v/bin/python.

Admits a word only if ALL hold (I1-I3): Wiktionary + SCOWL lowercase single-token headword; at least one
current, non-offensive, non-form-of sense; Zipf in [2.5, 4.0] (frequency never admits, it only bounds the band);
non-null CMUdict key; not already in the bank or quarantine; not on the profanity/blocklist; not a proper noun,
abbreviation, or inflection of a lemma that is itself a headword; passes the PURITY classifier (tools/bank/purity_admit).
Writes assets/words/en/staged.tsv (word, growth_stage, source) and reports/en-staged-report.md. Nothing is added to the bank.
"""
import argparse, collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

HOME = os.path.expanduser("~/repos/spell-wasm")
BAD_TAGS = {"archaic", "obsolete", "offensive", "vulgar", "slur", "derogatory", "dialectal", "dialect", "pejorative",
            "ethnic slur", "rare", "nonstandard", "dated", "uncommon", "informal", "slang", "colloquial", "regional",
            "sometimes offensive", "ethnic", "disparaging", "historical", "jargon-only"}
# Tags that disqualify a sense on their own.  informal/slang/colloquial/dated/historical/regional are NOT in this set:
# they are listed above for the report only.
HARD_BAD = {"archaic", "obsolete", "offensive", "vulgar", "slur", "derogatory", "dialectal", "dialect", "pejorative",
            "ethnic slur", "nonstandard", "disparaging", "sometimes offensive", "rare"}
BAD_POS = {"name", "proper noun", "abbrev", "abbreviation", "acronym", "initialism", "symbol", "prefix", "suffix", "infix",
           "contraction", "character", "letter", "numeral", "phrase", "proverb", "prep_phrase", "particle", "affix", "interfix", "circumfix"}


def good_sense(pos, s):
    if pos in BAD_POS or s["f"]:
        return False
    return not (set(s["t"]) & HARD_BAD)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--hw", default=os.path.join(HOME, ".corpus-cache/en-wikt/headwords.json"))
    ap.add_argument("--lo", type=float, default=2.5)
    ap.add_argument("--hi", type=float, default=4.0)
    ap.add_argument("--out", default=os.path.join(ROOT, "assets/words/en/staged.tsv"))
    ap.add_argument("--report", default=os.path.join(ROOT, "reports/en-staged-report.md"))
    a = ap.parse_args()
    zipf = zipf_fn()
    hw = json.load(open(a.hw, encoding="utf-8"))
    cmu = load_cmu()
    bank = {w for t in TIERS for w in bank_rows()[t]}
    quar = set()
    for t in TIERS:
        p = os.path.join(HOME, f"assets/words-quarantine/en/{t}.txt")
        if os.path.exists(p):
            quar |= {l.strip() for l in open(p, encoding="utf-8") if l.strip()}
    scowl = set()
    for l in open(os.path.join(HOME, ".corpus-cache/dicts/en/en_US.dic"), encoding="utf-8", errors="ignore").read().splitlines()[1:]:
        scowl.add(l.split("/")[0])
    prof = set()
    for p in (os.path.join(HOME, "assets/words/profanity/en.txt"), os.path.join(HOME, "tools/wordpipe/sources/blocklist_en.txt")):
        if os.path.exists(p):
            prof |= {l.strip().lower() for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")}
    funnel = collections.OrderedDict()
    cand = [w for w in hw if w in scowl]; funnel["Wiktionary lowercase headword AND SCOWL headword"] = len(cand)
    cand = [w for w in cand if len(w) >= 4]; funnel["length >= 4"] = len(cand)
    cand = [w for w in cand if a.lo <= zipf(w) <= a.hi]; funnel[f"Zipf in [{a.lo}, {a.hi}]"] = len(cand)
    cand = [w for w in cand if w not in bank and w not in quar]; funnel["not already in bank / quarantine"] = len(cand)
    cand = [w for w in cand if w not in prof]; funnel["not on profanity / blocklist"] = len(cand)
    cand = [w for w in cand if w in cmu]; funnel["has CMUdict key (fail closed)"] = len(cand)
    keep, why = [], collections.Counter()
    gloss = {}
    for w in cand:
        good = [(e["pos"], s) for e in hw[w] for s in e["s"] if good_sense(e["pos"], s)]
        if not good:
            why["no current, standard, non-form-of sense"] += 1
            continue
        keep.append(w); gloss[w] = good[0]
    funnel["has a current, standard, non-offensive, non-form-of sense"] = len(keep)
    # inflected forms of a lemma that is itself a headword (D8): every sense is a form-of sense of a headword
    def lemma_inflection(w):
        for e in hw[w]:
            for s in e["s"]:
                if not s["f"]:
                    return False
        return True
    keep2 = [w for w in keep if not lemma_inflection(w)]
    funnel["not purely an inflected / variant form (D8)"] = len(keep2)
    sys.path.insert(0, os.path.join(HOME, "tools/bank"))
    from purity_admit import admit
    ok, bad = admit("en", keep2)
    funnel["passes PURITY classifier"] = len(ok)
    ok = sorted(ok)
    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write("# CC-EN-EXPERT-GROWTH F3. growth_stage=staged rows are NOT live: flag en.expert_growth, spelling surfaces only.\n")
        f.write("word\tgrowth_stage\tsource\n")
        for w in ok:
            f.write(f"{w}\tstaged\twiktionary+scowl\n")
    json.dump({w: {"pos": gloss[w][0], "gloss": gloss[w][1]["g"]} for w in ok}, open(os.path.join(HOME, ".corpus-cache/en-wikt/staged_gloss.json"), "w"), ensure_ascii=False)
    with open(a.report, "w", encoding="utf-8") as f:
        f.write("# English staged-row admission report (generated)\n\n| Step | Rows |\n|---|---|\n")
        for k, v in funnel.items():
            f.write(f"| {k} | {v:,} |\n")
        f.write(f"\nPURITY rejections: {dict(collections.Counter(bad.values()))}\n")
    for k, v in funnel.items():
        print(f"{v:>8,}  {k}")
    print("purity rejected:", dict(collections.Counter(bad.values())))


if __name__ == "__main__":
    main()
