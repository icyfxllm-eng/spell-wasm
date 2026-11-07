#!/usr/bin/env python3
"""CC-SOUNDALIKE-LADDER F6: the future Gig B sheet for a ladder language. Generated now, NOT sent.

    python3 tools/soundalike_gigb_sheet.py fr pt       -> reports/soundalike-gigb-<lang>.csv

Order: disagreement sets first (the signals do not agree, so the auditor's ear matters most), then provisional sets, then the
rest. same_lemma_inflection sets are left off: they can never be promoted (typing a plural for a singular is a spelling error,
not another sense), so no auditor time is spent on them; their count is in the summary row. Columns carry statuses and reason
codes only -- no IPA.
"""
import csv, json, pathlib, sys
ROOT = pathlib.Path(__file__).resolve().parent.parent
DISAGREE = {"loopback_flag", "ipa_differ", "key_disagrees", "ipa_missing"}


def rows(lang):
    t = json.loads((ROOT / f"assets/words/{lang}/soundalikes.json").read_text(encoding="utf-8"))
    lp = json.loads((ROOT / f"assets/words/{lang}/soundalike-loopback.json").read_text(encoding="utf-8")).get("sets", {})
    ipa = json.loads((ROOT / f"data/{lang}/soundalike-ipa-check.json").read_text(encoding="utf-8")).get("sets", {}) if (ROOT / f"data/{lang}/soundalike-ipa-check.json").exists() else {}
    out, skipped = [], 0
    for s in t["sets"]:
        if s["kind"] != "true":
            continue
        l = s["ladder"]
        if l["reason"] == "same_lemma_inflection":
            skipped += 1
            continue
        grp = 0 if l["reason"] in DISAGREE else (1 if l["status"] == "provisional" else 2)
        r = lp.get(s["setId"], {})
        out.append((grp, s["setId"], [lang, s["setId"], " / ".join(m["memberId"] for m in s["members"]), l["status"], l["reason"] or "",
                    "+".join(l["origin"]), r.get("worst", ""), r.get("flag") or "", ipa.get(s["setId"], "unchecked"), "", ""]))
    out.sort(key=lambda x: (x[0], x[2][2]))
    return [x[2] for x in out], skipped


if __name__ == "__main__":
    for lang in sys.argv[1:] or ["fr", "pt"]:
        data, skipped = rows(lang)
        p = ROOT / f"reports/soundalike-gigb-{lang}.csv"
        with open(p, "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(["lang", "set_id", "members", "ladder_status", "reason", "origin", "loopback_worst", "loopback_flag", "ipa_check",
                        "Same sound in your accent? (yes/no/unsure)", "Notes"])
            w.writerows(data)
        print(f"{lang}: {len(data)} rows ({skipped} same-lemma inflection sets left off) -> {p.relative_to(ROOT)}")
