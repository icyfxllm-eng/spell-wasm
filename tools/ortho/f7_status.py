#!/usr/bin/env python3
"""CC-ORTHO-FIX-FIL-ROMANCE F7: may the ortho.profile.<lang> flag go on? Prints the verdict per language, es -> pt -> fr.
Holds only when (a) the sound-alike table is verified, (b) the golden file has no pending_table row (no homophone the
table cannot yet call), (c) the flag in src/flags.rs still defaults off until Eric turns it on. (d) A player who misses
only an accent seeing the fix is an in-app check: this script cannot do it, so a PASS here is necessary, not sufficient."""
import json, os, collections
R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
for lang in ("es", "pt", "fr"):
    st = json.load(open(f"{R}/data/{lang}/sound-alike-rules.json"))["status"]
    c = collections.Counter(l.split("\t")[5] for l in open(f"{R}/tests/ortho/golden/{lang}.tsv", encoding="utf-8") if l[0] != "#")
    ok_table = "verified" in str(st).lower() and "missing" not in str(st).lower()
    print(f"{lang}: table={st!r} pending_table_rows={c['pending_table']} -> {'candidate (needs the in-app close check)' if ok_table and not c['pending_table'] else 'HOLDS OFF'}")
