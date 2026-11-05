#!/usr/bin/env python3
"""Ingest the ja auditor's verdicts on reports/ja-dual-script-auditor-sheet.csv and rewrite data/ja/dual-script.tsv from them (D-NOBYSA).

    python3 tools/ortho/ja_dual_ingest.py <returned-sheet.csv> --auditor "Name" [--date YYYY-MM-DD]

verdict column: yes (also written in katakana, accept it) | no (hiragana only). A blank verdict is an error, so an unanswered row cannot silently count as no."""
import argparse, csv, datetime, os, sys
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ap = argparse.ArgumentParser(); ap.add_argument("sheet"); ap.add_argument("--auditor", required=True); ap.add_argument("--date", default=str(datetime.date.today()))
a = ap.parse_args()
rows = list(csv.DictReader(open(a.sheet, encoding="utf-8")))
bad = [r["row"] for r in rows if r["verdict"].strip().lower() not in ("yes", "no")]
if bad: sys.exit(f"{len(bad)} rows without a yes/no verdict, first: {bad[:5]}")
yes = sorted(r["row"] for r in rows if r["verdict"].strip().lower() == "yes")
with open(f"{REPO}/data/ja/dual-script.tsv", "w", encoding="utf-8") as f:
    f.write("# CC-ORTHO-SWEEP-DE-JA v1 F5 / D8: rows tagged dual_script. Only these rows may accept the other script (katakana for a hiragana key).\n")
    f.write(f"# Source: ja auditor {a.auditor}, {a.date} (reports/ja-dual-script-auditor-sheet.csv). Not derived from JMdict or Wiktionary (D-NOBYSA).\n")
    f.write("# Data only; src/accepted.rs reads it.\n")
    for w in yes: f.write(f"{w}\tdual_script\n")
print(len(yes), "yes,", len(rows) - len(yes), "no")
