#!/usr/bin/env python3
"""CC-BANK-PURITY F6 — the door every bank-growing script goes through.

    from purity_admit import admit_new                  # scripts/ put tools/bank on sys.path first
    banks = admit_new(lang, banks)                       # {tier: [word,...]} -> same shape, bad new rows removed

F1 (tools/bank/purity_check.py) classifies every word that is not already in the lang's committed ledger.
Only `pass` is kept; the rest are printed with their class and never written.  Rows already in the ledger are
passed through untouched (they were judged at Phase A or by a signed exception).  A language outside the 14 purity
languages (draft banks for languages not yet launched) is passed through with a notice: there is no F1 for it.

Fails closed: if the reference dictionaries are not in the corpus cache this exits non-zero rather than writing
unchecked rows.  Fetch them with `python3 tools/bank/fetch_sources.py`.
After a writer runs, refresh the ledger:
    python3 tools/bank/purity_check.py --all --out reports/bank-purity-rows-after.csv
    python3 tools/bank/gen_ledger.py reports/bank-purity-rows-after.csv
"""
import os, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
REPO = os.path.dirname(os.path.dirname(HERE))
LANGS = "en es fr de pt pl ko ja fil zh ru ar hi sw".split()

def _ledger_words(lang):
    p = f"{REPO}/assets/words/purity/{lang}.tsv"
    if not os.path.exists(p):
        return set()
    return {l.split("\t")[0] for l in open(p, encoding="utf-8") if l.strip() and l[0] != "#"}

def admit(lang, words):
    """-> (accepted, {rejected word: class}) for a flat list of candidate words."""
    if lang not in LANGS:
        print(f"purity_admit: {lang} is not one of the 14 purity languages; its rows are NOT checked", file=sys.stderr)
        return list(words), {}
    import purity_check as pc
    from census_c6 import fold_lenient, load
    known = _ledger_words(lang)
    nfc = lambda s: unicodedata.normalize("NFC", s)
    fresh = [w for w in dict.fromkeys(words) if nfc(w) not in known and w not in known]
    res = pc.classify(lang, [("easy", w) for w in fresh]) if fresh else {}
    prof = load(f"{REPO}/assets/words/profanity/{lang}.txt") or set()
    for w, (c, e) in pc.overrides(lang).items():
        if w in res and res[w][0] not in pc.QUARANTINE:
            res[w] = (c, e)
    rejected = {}
    for w in fresh:
        c = res[w][0]
        if c != "pass":
            rejected[w] = c
        elif fold_lenient(w.rsplit("|", 1)[-1]) in prof:
            rejected[w] = "profanity_list"
    return [w for w in words if w not in rejected], rejected

def admit_new(lang, banks):
    flat = [w for t in banks for w in banks[t]]
    ok, bad = admit(lang, flat)
    if bad:
        from collections import Counter
        print(f"purity_admit[{lang}]: {len(bad)} candidate(s) not written —", dict(Counter(bad.values())), file=sys.stderr)
    return {t: [w for w in banks[t] if w not in bad] for t in banks}
