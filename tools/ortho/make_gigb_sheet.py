#!/usr/bin/env python3
"""CC-ORTHO-SWEEP-ALL: Gig B sheet rows for the sound-alike seeds and near-sound examples (es, fr, pt).
Two columns the auditor fills: "Same sound in your accent?" (yes/no/unsure) and "Correct spelling where you live?"
(the spelling they write, or blank if the stored one). Written to reports/ortho-gigb-<lang>.csv. Ingest only through the
existing locked-sheet path."""
import csv, json, os
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HEAD = ["lang", "kind", "rule", "word_a", "word_b", "voice_id", "Same sound in your accent?", "Correct spelling where you live?", "source"]
for lang in ("es", "fr", "pt"):
    sa = json.load(open(f"{REPO}/data/{lang}/sound-alike-rules.json", encoding="utf-8"))
    ns = json.load(open(f"{REPO}/data/{lang}/near-sound-rules.json", encoding="utf-8"))
    kind = {r["id"]: r["kind"] for r in sa["rules"]}
    rows = []
    for s in sa["seeds"]:
        for i, a in enumerate(s["words"]):
            for b in s["words"][i + 1:]:
                rows.append([lang, "sound-alike:" + kind[s["rule"]], s["rule"], a, b, sa["voice_id"], "", "", s["source"]])
    for g in sa.get("gigb_pairs", []):
        rows.append([lang, "tagged-pair", "accent_only_bank_pair", g["words"][0], g["words"][1], sa["voice_id"], "", "", g["source"]])
    for r in ns["rules"]:
        for a, b in r.get("examples", []):
            rows.append([lang, "near-sound", r["id"], a, b, sa["voice_id"], "", "", r["source"]])
    with open(f"{REPO}/reports/ortho-gigb-{lang}.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n"); w.writerow(HEAD); w.writerows(rows)
    print(lang, len(rows), "rows")
