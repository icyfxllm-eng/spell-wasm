#!/usr/bin/env python3
"""CC-LAUNCH-EN D-SRC4 / Gig A -- ingest a rated generated-text sheet.

    python3 tools/defs/gen_ingest.py <rater sheet.csv> [--eric <Eric's sheet.csv>] [--apply]
    python3 tools/defs/gen_ingest.py <Eric's sheet.csv> --solo [--hold held.json] [--apply]   (rev 4)

The rater (Gig A) rates every field. Eric rates every Kid-visible row (rev 3)
on his own copy of the same sheet; a Kid-visible field is reviewed only when
BOTH gave it 0 (A15.4: severity > 0 from either rater blocks it).

Gates. Any failure rejects the whole sheet.
  STAMP     locked columns and row order unchanged since export.
  SOURCE    (A15.3) every source row from the run's rows.csv is in the sheet with
            the same text: none missing, none added, none changed.
  COMPLETE  (A15.2) every field that has text has a severity of 0-3 (Eric's
            sheet: every Kid-visible field).
  DECOY     (A15.2) a decoy is missed when its bad field is rated 0, or below 2
            for a severe decoy. More than 2 missed, or ANY severe decoy missed:
            reject. 1-2 mild misses: the 150-row chunk around each miss is
            re-queued (its rows stay unreviewed and go to requeue.csv).
  SPOTCHECK (A15.6 / D-DEF5) where Eric rated a field the rater also rated, they
            must give the same severity on at least 95% of fields; otherwise the
            rater's pass is VOID (re-hire, redo; never average).

With --apply, writes into the sheet's directory (build/, never committed --
the text is only shippable as reviewed rows through the server deploy):
  reviewed.en.json  every source row with per-field reviewed_definition /
                    reviewed_sentence booleans, severities, source, source_license
  requeue.csv       rows in re-queued chunks
  pass.md           counts, reviewed rows per tier against N = 40, and every
                    field rated above 0 (Eric's fix queue; D-DEF7: raters flag,
                    they never rewrite)
A fixed row is a NEW row and needs a new rating; this tool never carries a
rating over to changed text (A15.11 follows from SOURCE + STAMP).
"""
from __future__ import annotations

import argparse
import csv
import json
import pathlib
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gen_sheet  # noqa: E402

ROOT = gen_sheet.ROOT
FIELDS = ("definition", "sentence")
N_FLOOR = 40
TIERS = ["easy", "medium", "hard", "expert"]
SEV = {"0", "1", "2", "3"}


def read_sheet(path: pathlib.Path):
    with open(path, encoding="utf-8") as f:
        head = f.readline().split()
        if len(head) < 5 or head[:2] != ["#", "sheet"]:
            raise SystemExit(f"REJECT {path}: no sheet header line")
        rows = list(csv.DictReader(f))
    for r in rows:
        r["row"], r["chunk"] = int(r["row"]), int(r["chunk"])
    return head[2], head[4], rows


def check(path, keys_dir, kid_only=False):
    """Run STAMP, COMPLETE and DECOY on one sheet. Returns (sheet_id, rows, errors, requeue_chunks)."""
    sheet_id, want, rows = read_sheet(path)
    errors = []
    if gen_sheet.stamp(sheet_id, rows) != want:
        errors.append("STAMP: a locked column or the row order changed since export")
    missing = [r["row"] for r in rows for f in FIELDS
               if r[f] and (not kid_only or r["kid_visible"] == "yes") and r[f + "_severity"].strip() not in SEV]
    if missing:
        errors.append(f"COMPLETE: {len(missing)} fields with text have no severity 0-3 (first rows: {missing[:10]})")
    keyf = keys_dir / f"{sheet_id}-DECOY-KEY.json"
    requeue = set()
    if not keyf.exists():
        errors.append(f"DECOY: key {keyf} not found")
        decoys = {}
    else:
        decoys = {d["row"]: d for d in json.loads(keyf.read_text())["decoys"]}
        by_row = {r["row"]: r for r in rows}
        missed = []
        for n, d in decoys.items():
            s = by_row.get(n, {}).get(d["field"] + "_severity", "").strip()
            v = int(s) if s in SEV else 0
            if v < (2 if d["severe"] else 1):
                missed.append(d)
        if any(d["severe"] for d in missed):
            errors.append(f"DECOY: a severe decoy was missed (rows {[d['row'] for d in missed if d['severe']]})")
        elif len(missed) > 2:
            errors.append(f"DECOY: {len(missed)} decoys missed (tolerance 2)")
        else:
            requeue = {d["chunk"] for d in missed}
    return sheet_id, rows, decoys, errors, requeue


def ingest(rater_path, eric_path=None, apply=False, keys_dir=None, solo=False, hold=frozenset()):
    """solo (rev 4, Eric 2026-11-09): rater_path is ERIC's sheet and the only human rating;
    no second rater, no spot-check. hold: (word, tier, field) keys that stay unreviewed
    until Eric's second look (rev 4: his own rewrites that the code checks or the
    fresh-context safety review flagged)."""
    keys_dir = keys_dir or ROOT / "keys"
    sheet_id, rows, decoys, errors, requeue = check(rater_path, keys_dir)
    meta = json.loads((rater_path.parent / "meta.json").read_text())
    src = {(r["word"], r["tier"]): r for r in gen_sheet.source_rows(pathlib.Path(meta["run"]))}
    real = [r for r in rows if r["row"] not in decoys]
    got = {(r["word"], r["tier"]): r for r in real}
    if set(src) != set(got) or len(got) != len(real):
        errors.append(f"SOURCE: {len(set(src) - set(got))} source rows missing, {len(set(got) - set(src))} added, "
                      f"{len(real) - len(got)} duplicated")
    else:
        changed = [k for k, s in src.items() if any(s[f] != got[k][f] for f in FIELDS)]
        if changed:
            errors.append(f"SOURCE: {len(changed)} rows differ from the run's text (first: {changed[:5]})")
    eric = None
    if eric_path and solo:
        errors.append("solo mode takes one sheet (Eric's); do not pass --eric")
    if eric_path and not solo:
        e_id, e_rows, _, e_err, e_requeue = check(eric_path, keys_dir, kid_only=True)
        if e_id != sheet_id:
            e_err.append(f"Eric's sheet is {e_id}, not {sheet_id}")
        errors += [f"ERIC {e}" for e in e_err]
        requeue |= e_requeue
        eric = {r["row"]: r for r in e_rows}
    if errors:
        return {"verdict": "REJECT", "sheet": sheet_id, "errors": errors}

    sev = lambda r, f: int(r[f + "_severity"]) if r[f + "_severity"].strip() in SEV else None
    both = agree = 0
    if eric:
        for r in real:
            for f in FIELDS:
                a, b = sev(r, f), sev(eric[r["row"]], f)
                if a is not None and b is not None:
                    both += 1
                    agree += a == b
    disagreement = (both - agree) / both if both else None
    if disagreement is not None and disagreement > 0.05:
        return {"verdict": "VOID", "sheet": sheet_id, "spotcheckFields": both,
                "disagreement": round(disagreement, 4),
                "errors": [f"SPOTCHECK: rater and Eric disagree on {disagreement:.1%} of {both} fields (limit 5%)"]}

    out, flagged, req_rows, held_rows = [], [], [], []
    for r in real:
        o = {"word": r["word"], "tier": r["tier"], "kid_visible": r["kid_visible"] == "yes",
             "source": "spell-gen-gen-v2", "source_license": "original",
             "reviewer": "eric (solo, rev 4)" if solo else "gig-a + eric"}
        for f in FIELDS:
            s = sev(r, f)
            e = sev(eric[r["row"]], f) if eric else None
            ok = bool(r[f]) and s == 0 and r["chunk"] not in requeue
            if o["kid_visible"] and not solo:
                ok = ok and e == 0   # Eric rates every Kid-visible row; either rater > 0 blocks it
            o[f] = r[f]
            o[f + "_severity"] = s
            o["eric_" + f + "_severity"] = e
            held = (r["word"], r["tier"], f) in hold
            if held and ok:
                held_rows.append((r, f))
            o["reviewed_" + f] = ok and not held
            if r[f] and ((s or 0) > 0 or (e or 0) > 0):
                flagged.append((r, f, s, e))
        out.append(o)
        if r["chunk"] in requeue:
            req_rows.append(r)
    tiers = {t: sum(o["reviewed_definition"] for o in out if o["tier"] == t) for t in TIERS}
    kid_unreviewed = sum(1 for o in out if o["kid_visible"] and not o["reviewed_definition"] and o["definition"])
    res = {"verdict": "ACCEPT", "sheet": sheet_id, "mode": "solo (rev 4)" if solo else "rater + Eric",
           "rows": len(out), "heldForSecondLook": len(held_rows), "requeuedChunks": sorted(requeue),
           "requeuedRows": len(req_rows), "fieldsFlagged": len(flagged),
           "reviewedDefinitionsPerTier": tiers, "tiersBelowN": [t for t, n in tiers.items() if n < N_FLOOR],
           "kidVisibleUnreviewedDefinitions": kid_unreviewed,
           "spotcheckFields": both, "disagreement": None if disagreement is None else round(disagreement, 4)}
    if apply:
        d = rater_path.parent
        (d / "reviewed.en.json").write_text(json.dumps(out, ensure_ascii=False, indent=0), encoding="utf-8")
        with open(d / "requeue.csv", "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["row", "chunk", "word", "tier"])
            w.writerows([r["row"], r["chunk"], r["word"], r["tier"]] for r in req_rows)
        rep = [f"# Generated English text: rating pass ({sheet_id})", "",
               f"Ingested {time.strftime('%Y-%m-%d')} by tools/defs/gen_ingest.py.", "",
               "```", json.dumps(res, indent=1), "```", "",
               "## Fields rated above 0 (fix queue)", "",
               "| row | word | tier | kid | field | rater | Eric | text | note |", "|---|---|---|---|---|---|---|---|---|"]
        rep += [f"| {r['row']} | {r['word']} | {r['tier']} | {r['kid_visible']} | {f} | {s} | {'' if e is None else e} | "
                f"{r[f][:100]} | {r.get('note', '')[:80]} |" for r, f, s, e in flagged] or ["| none |||||||||"]
        rep += ["", "## Held for Eric's second look (rev 4)", "", "| row | word | tier | field | text |", "|---|---|---|---|---|"]
        rep += [f"| {r['row']} | {r['word']} | {r['tier']} | {f} | {r[f][:120]} |" for r, f in held_rows] or ["| none |||||"]
        (d / "pass.md").write_text("\n".join(rep) + "\n", encoding="utf-8")
        res["wrote"] = [str(d / n) for n in ("reviewed.en.json", "requeue.csv", "pass.md")]
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sheet")
    ap.add_argument("--eric")
    ap.add_argument("--solo", action="store_true", help="rev 4: the sheet is Eric's, the only rating")
    ap.add_argument("--hold", help="JSON list of [word, tier, field] kept unreviewed until a second look")
    ap.add_argument("--apply", action="store_true")
    a = ap.parse_args()
    hold = frozenset(tuple(x[:3]) for x in json.loads(pathlib.Path(a.hold).read_text())) if a.hold else frozenset()
    res = ingest(pathlib.Path(a.sheet), pathlib.Path(a.eric) if a.eric else None, a.apply, solo=a.solo, hold=hold)
    print(json.dumps(res, indent=1))
    if res["verdict"] != "ACCEPT":
        sys.exit(1)


if __name__ == "__main__":
    main()
