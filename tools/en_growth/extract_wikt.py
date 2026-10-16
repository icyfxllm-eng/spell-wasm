#!/usr/bin/env python3
"""Stream the kaikki English dump into a compact per-headword tag table (build input, not committed).

For every lowercase, single-token, ASCII-alphabetic English headword keep, per sense:
pos, tags (tags + raw_tags, lowercased), whether the sense is a form-of / alt-of, and the first gloss.
Output: .corpus-cache/en-wikt/headwords.json
"""
import json, os, sys
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.environ.get("KAIKKI") or os.path.join(os.path.expanduser("~/repos/spell-wasm"), ".corpus-cache/en-wikt/kaikki.org-dictionary-English.jsonl")
OUT = os.path.join(os.path.dirname(SRC), "headwords.json")
out = {}
n = 0
with open(SRC, encoding="utf-8") as f:
    for line in f:
        n += 1
        d = json.loads(line)
        w = d.get("word", "")
        if not (w.isascii() and w.isalpha() and w.islower()):
            continue
        pos = d.get("pos", "")
        senses = []
        for s in d.get("senses", []):
            tags = [t.lower() for t in (s.get("tags") or []) + (s.get("raw_tags") or [])]
            glosses = s.get("glosses") or []
            formof = bool(s.get("form_of") or s.get("alt_of") or any(
                t in ("form-of", "alt-of", "alternative", "misspelling", "abbreviation", "initialism", "acronym",
                      "obsolete spelling", "archaic spelling", "dated spelling", "nonstandard spelling") for t in tags))
            senses.append({"t": tags, "f": formof, "g": (glosses[0] if glosses else "")[:300]})
        out.setdefault(w, []).append({"pos": pos, "s": senses})
json.dump(out, open(OUT, "w", encoding="utf-8"), ensure_ascii=False)
print(n, "entries ->", len(out), "headwords", OUT)
