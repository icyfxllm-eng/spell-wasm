#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL F0.5 / D-NOBYSA provenance gate.

Every shipped variant row (data/<lang>/accepted-variants.tsv) and every sound-alike seed
(data/<lang>/sound-alike-rules.json) of an orthography-profile language must carry a source, and the source may not name
a licence or corpus listed in data/licences/share-alike.txt. Those names may appear only in a checked_by field.
Usage: provenance-gate.py [--selftest]. Exit 1 on any violation.
"""
import json, os, sys, tempfile
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LANGS = "es fr pt pl ru vi ko fil zh sw hi".split()

def banned(repo):
    return [l.strip().lower() for l in open(f"{repo}/data/licences/share-alike.txt", encoding="utf-8") if l.strip() and not l.startswith("#")]

def check(repo, langs=LANGS):
    bad, ban = [], banned(repo)
    def hit(src): return next((b for b in ban if b in src.lower()), None)
    for lang in langs:
        p = f"{repo}/data/{lang}/accepted-variants.tsv"
        if os.path.exists(p):
            for i, l in enumerate(open(p, encoding="utf-8").read().split("\n"), 1):
                if not l or l.startswith("#"): continue
                c = l.split("\t")
                if len(c) < 4 or not c[2].strip(): bad.append(f"{p}:{i}: variant row needs stored, accepted, source, kind"); continue
                if (b := hit(c[2])): bad.append(f"{p}:{i}: source names {b!r}")
        p = f"{repo}/data/{lang}/sound-alike-rules.json"
        if os.path.exists(p):
            for s in json.load(open(p, encoding="utf-8")).get("seeds", []):
                src = s.get("source", "")
                if not src.strip(): bad.append(f"{p}: seed {s.get('words')} has no source")
                elif (b := hit(src)): bad.append(f"{p}: seed {s.get('words')} source names {b!r}")
    return bad

def selftest():
    import shutil
    d = tempfile.mkdtemp()
    os.makedirs(f"{d}/data/licences"); os.makedirs(f"{d}/data/fr")
    shutil.copy(f"{REPO}/data/licences/share-alike.txt", f"{d}/data/licences/")
    open(f"{d}/data/fr/accepted-variants.tsv", "w", encoding="utf-8").write("# c\nmaître\tmaitre\tRectifications 1990\trect1990\n")
    assert check(d, ["fr"]) == [], "a clean file must pass"
    open(f"{d}/data/fr/accepted-variants.tsv", "a", encoding="utf-8").write("île\tile\tWiktionary entry (CC BY-SA 3.0)\tx\n")
    got = check(d, ["fr"])
    assert len(got) == 1 and "ba" not in got[0] and "names" in got[0], got
    open(f"{d}/data/fr/accepted-variants.tsv", "w", encoding="utf-8").write("a\tb\t\tk\n")
    assert check(d, ["fr"]), "a row with no source must fail"
    print("provenance-gate selftest ok: clean passes, a planted BY-SA row and an unsourced row fail")

if __name__ == "__main__":
    if "--selftest" in sys.argv: selftest(); sys.exit(0)
    bad = check(REPO)
    for b in bad: print("PROVENANCE FAIL:", b)
    print(f"provenance-gate: {len(bad)} violation(s)")
    sys.exit(1 if bad else 0)
