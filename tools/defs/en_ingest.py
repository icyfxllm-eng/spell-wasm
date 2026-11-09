#!/usr/bin/env python3
"""CC-LAUNCH-EN F9 / A9.1 -- ingest a filled English definitions sheet.

    python3 tools/defs/en_ingest.py build/defs-audit/en-defs-YYYYMMDD/sheet.csv [--apply]

Gates (any failure rejects the WHOLE sheet, after tools/human-audio/ingest.py):
  STAMP     the locked columns (row, word, source, field, kid_visible, text) and
            their order are unchanged since export.
  COMPLETE  every row has a severity of 0, 1, 2 or 3.
  DECOY     the decoy rows (keys/<sheet>-DECOY-KEY.json) are rated above 0;
            at most one may be missed.

On a passing sheet it reports, and with --apply writes:
  - every word with a kid_visible row rated above 0 appended to
    assets/words/kid-exclude/en.txt (Kid Mode is zero-tolerance; an app build ships it);
  - <sheet dir>/def_pools.en.json: the served Definition Match pool with
    kid_register false for those words and any defpool `fix` text applied, for
    the content-only server deploy (rsync def_pools, no restart);
  - reports/en-definitions-pass.md: counts, the Kid Mode removals, and every row
    rated 2 or 3 with its fix, including meaning-cache rows, which have no
    override mechanism today and need a server-side decision.
"""
import argparse
import csv
import hashlib
import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
LOCKED = ["row", "word", "source", "field", "kid_visible", "text"]


def stamp(sheet_id, rows):
    h = hashlib.sha256(sheet_id.encode())
    for r in rows:
        h.update(f"\n{r['row']}\t{r['word']}\t{r['source']}\t{r['field']}\t{r['kid_visible']}\t{r['text']}".encode())
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sheet")
    ap.add_argument("--apply", action="store_true")
    a = ap.parse_args()
    path = pathlib.Path(a.sheet)
    with open(path, encoding="utf-8") as f:
        head = f.readline().split()
        if len(head) < 5 or head[:2] != ["#", "sheet"]:
            sys.exit("REJECT: no sheet header line")
        sheet_id, want = head[2], head[4]
        rows = list(csv.DictReader(f))
    errors = []
    if stamp(sheet_id, rows) != want:
        errors.append("STAMP: a locked column or the row order changed since export")
    bad = [r["row"] for r in rows if r.get("severity", "").strip() not in {"0", "1", "2", "3"}]
    if bad:
        errors.append(f"COMPLETE: {len(bad)} rows have no severity 0-3 (first: {bad[:10]})")
    keyf = ROOT / "keys" / f"{sheet_id}-DECOY-KEY.json"
    if not keyf.exists():
        errors.append(f"DECOY: key {keyf} not found")
        decoys = set()
    else:
        decoys = {str(n) for n in json.loads(keyf.read_text())["decoyRows"]}
        missed = [r["row"] for r in rows if r["row"] in decoys and r.get("severity", "").strip() in {"", "0"}]
        if len(missed) > 1:
            errors.append(f"DECOY: {len(missed)} decoys rated 0 (tolerance 1)")
    if errors:
        print("REJECT " + sheet_id)
        for e in errors:
            print("  " + e)
        sys.exit(1)

    real = [r for r in rows if r["row"] not in decoys]
    sev = lambda r: int(r["severity"])
    kid_remove = sorted({r["word"] for r in real if r["kid_visible"] == "yes" and sev(r) >= 1})
    serious = [r for r in real if sev(r) >= 2]
    counts = {s: sum(1 for r in real if sev(r) == s) for s in range(4)}
    print(f"ACCEPT {sheet_id}: {len(real)} rows rated {counts}; Kid Mode removals {len(kid_remove)}; rows at 2+ {len(serious)}")

    meta = json.loads((path.parent / "meta.json").read_text())
    pool_src = pathlib.Path(meta["defpool"])
    pool = json.loads(pool_src.read_text(encoding="utf-8"))
    fixes = {(r["source"].split("/", 1)[1], r["word"]): r["fix"].strip()
             for r in real if r["source"].startswith("defpool/") and r.get("fix", "").strip()}
    remove = set(kid_remove)
    for tier, items in pool["tiers"].items():
        for it in items:
            if it["word"] in remove:
                it["kid_register"] = False
            if (tier, it["word"]) in fixes:
                it["definition"] = fixes[(tier, it["word"])]

    rep = [f"# English definitions pass ({sheet_id})", "",
           f"Ingested {time.strftime('%Y-%m-%d')} by tools/defs/en_ingest.py. Stamp {meta['stamp'][:16]}..., decoys passed.", "",
           f"- Rows rated: {len(real)} (severity 0: {counts[0]}, 1: {counts[1]}, 2: {counts[2]}, 3: {counts[3]})",
           f"- Kid-visible rows: {sum(r['kid_visible'] == 'yes' for r in real)}",
           f"- Words removed from Kid Mode: {len(kid_remove)}",
           f"- Served words with no cached definition (unreviewed, fetched live): {meta['uncachedWords']} "
           f"({meta['uncachedKidWords']} in Spell Jr's pool)", "",
           "## Removed from Kid Mode", "", ", ".join(kid_remove) or "None.", "",
           "## Rows rated 2 or 3", "", "| word | source | field | severity | text | fix |", "|---|---|---|---|---|---|"]
    rep += [f"| {r['word']} | {r['source']} | {r['field']} | {r['severity']} | {r['text'][:120]} | {r.get('fix', '')[:120]} |" for r in serious]
    if not serious:
        rep.append("| none | | | | | |")
    if a.apply:
        kx = ROOT / "assets" / "words" / "kid-exclude" / "en.txt"
        have = {l.strip() for l in kx.read_text(encoding="utf-8").splitlines()}
        new = [w for w in kid_remove if w not in have]
        if new:
            with open(kx, "a", encoding="utf-8") as f:
                f.write(f"# CC-LAUNCH-EN F9: definitions pass {sheet_id} (a kid-visible definition rated above 0)\n")
                f.write("".join(w + "\n" for w in new))
        (path.parent / "def_pools.en.json").write_text(json.dumps(pool, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        (ROOT / "reports" / "en-definitions-pass.md").write_text("\n".join(rep) + "\n", encoding="utf-8")
        print(f"applied: {len(new)} words added to {kx.relative_to(ROOT)}; patched pool at {path.parent / 'def_pools.en.json'}; "
              f"report at reports/en-definitions-pass.md")
    else:
        print("dry run: pass --apply to write")


if __name__ == "__main__":
    main()
