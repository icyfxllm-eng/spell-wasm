#!/usr/bin/env python3
"""CC-HUMAN-AUDIO F3: the only write path for audio verdicts (I2).

Takes a returned sheet.csv, its audit folder, and the decoy key. A clip is
playable only once an ACCEPT verdict for it has been ingested here, bound to the
exact bytes the auditor heard (I2). Nothing else in the project writes verdicts.

THE SHEET IS NOT TRUSTED (after tools/ru_stress_ingest.py). Every gate rejects
the WHOLE sheet; there is no partial ingest, because an unverified clip degrades
safely to TTS while a wrongly verified one plays the wrong word to a child.

  STAMP       the sheet's header stamp must equal the key's, and recomputing it
              from the audit folder's clip bytes must give the same value. A
              swapped or re-encoded clip is not the clip the auditor judged.
  AGREEMENT   row, entry and clip columns must match the key byte for byte.
              A sheet that edits a word would verify a clip for a word it does
              not say.
  COMPLETE    every row carries one of the six verdicts. A blank is not an
              accept.
  DECOY       each decoy row plays a different word than it names, so any
              verdict but "accept" catches it. More than one accepted decoy
              rejects the sheet (one blink is tolerated; two is a pattern).

Two recorded exceptions, both Eric's call and never the tool's default
(2026-09-19): --rows LO-HI ingests part of a sheet whose other rows were redone
on a fresh sheet (STAMP and AGREEMENT still cover the whole file), and
--decoy-override REASON accepts a sheet past the decoy tolerance, writing the
miss count and the reason onto every verdict it records.

On success, verdicts go to assets/human-audio/<lang>/verdicts.json, keyed by
entry, with the clip's sha256, the sheet id and the auditor. Decoy rows are
never written. A rejected sheet writes NOTHING.

Run: python3 tools/human-audio/ingest.py --sheet RETURNED.csv \\
       --folder <dir>/audit/<sheet-id> --key <dir>/audit/<sheet-id>-DECOY-KEY.json \\
       --auditor eric [--write]
"""
import argparse
import csv
import hashlib
import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
DECOY_TOLERANCE = 1

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from sheet import VERDICTS, stamp  # noqa: E402  (one definition of both)


def read_sheet(path):
    lines = pathlib.Path(path).read_text(encoding="utf-8-sig").splitlines()
    if not lines or not lines[0].startswith("# sheet "):
        return None, None, "missing the '# sheet <id> stamp <hash>' header line"
    parts = lines[0].split()
    if len(parts) != 5 or parts[3] != "stamp":
        return None, None, "malformed header line"
    rows = list(csv.DictReader(lines[1:]))
    return (parts[2], parts[4]), rows, None


def check(sheet_path, folder, key_path, rows_range=None, decoy_override=None):
    """Returns (verdict rows to write, list of problems). Any problem = reject.

    rows_range (lo, hi): judge and write only these rows; the rest of the sheet
    is ignored (it was redone on another sheet). STAMP and AGREEMENT still cover
    the WHOLE sheet, so the file is still proven to be the one the auditor saw.
    decoy_override: a recorded reason to accept a sheet past the decoy tolerance
    (Eric's call, not the tool's); every verdict written carries it."""
    key = json.loads(pathlib.Path(key_path).read_text())
    head, rows, err = read_sheet(sheet_path)
    if err:
        return None, [err]
    problems = []
    sheet_id, sheet_stamp = head
    if sheet_id != key["sheet_id"]:
        problems.append(f"STAMP: sheet id {sheet_id} is not the key's {key['sheet_id']}")
    if sheet_stamp != key["stamp"]:
        problems.append("STAMP: the sheet's stamp does not match the key's")
    live = []
    for k in key["rows"]:
        clip = pathlib.Path(folder) / "clips" / k["clip"]
        digest = hashlib.sha256(clip.read_bytes()).hexdigest() if clip.exists() else "missing"
        if digest != k["clip_sha256"]:
            problems.append(f"STAMP: row {k['row']} clip {k['clip']} is not the audited file")
        live.append({"row": k["row"], "entry": k["entry"], "clip": k["clip"], "clip_sha256": digest})
    if stamp(key["sheet_id"], live) != key["stamp"]:
        problems.append("STAMP: recomputed stamp over the audit folder does not match")

    by_row = {str(k["row"]): k for k in key["rows"]}
    if len(rows) != len(by_row):
        problems.append(f"AGREEMENT: sheet has {len(rows)} rows, key has {len(by_row)}")
    missed, out = [], []
    for r in rows:
        k = by_row.get((r.get("row") or "").strip())
        if k is not None and rows_range and not rows_range[0] <= k["row"] <= rows_range[1]:
            continue  # outside the range being ingested
        if k is None:
            problems.append(f"AGREEMENT: unknown row {r.get('row')!r}")
            continue
        if r.get("entry") != k["entry"] or r.get("clip") != k["clip"]:
            problems.append(f"AGREEMENT: row {k['row']} entry/clip edited "
                            f"({r.get('entry')!r}/{r.get('clip')!r})")
            continue
        v = (r.get("verdict") or "").strip().lower()
        if v not in VERDICTS:
            problems.append(f"COMPLETE: row {k['row']} ({k['entry']}) verdict {v!r} "
                            f"is not one of {VERDICTS}")
            continue
        if k["decoy"]:
            if v == "accept":
                missed.append(k)
            continue
        out.append({"entry": k["entry"], "verdict": v, "clip_sha256": k["clip_sha256"],
                    "commons_sha1": k["commons_sha1"], "speaker": k["speaker"],
                    "license": k["license"], "note": (r.get("note") or "").strip()})
    if len(missed) > DECOY_TOLERANCE and decoy_override:
        for o in out:
            o["decoy_override"] = f"{len(missed)} decoys missed; accepted anyway: {decoy_override}"
    elif len(missed) > DECOY_TOLERANCE:
        problems.append(f"DECOY: {len(missed)} decoys accepted (tolerance {DECOY_TOLERANCE}): rows "
                        + ", ".join(str(m["row"]) for m in missed))
    return out, problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sheet", required=True)
    ap.add_argument("--folder", required=True)
    ap.add_argument("--key", required=True)
    ap.add_argument("--auditor", required=True)
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--rows", help="ingest only rows LO-HI (the rest was redone elsewhere)")
    ap.add_argument("--decoy-override", metavar="REASON",
                    help="accept past the decoy tolerance; the reason is recorded on every verdict")
    a = ap.parse_args()
    key = json.loads(pathlib.Path(a.key).read_text())
    rng = tuple(int(x) for x in a.rows.split("-")) if a.rows else None
    out, problems = check(a.sheet, a.folder, a.key, rng, a.decoy_override)
    if problems:
        print(f"REJECTED {key['sheet_id']}: nothing written")
        for p in problems[:40]:
            print("  " + p)
        if len(problems) > 40:
            print(f"  ... and {len(problems) - 40} more")
        sys.exit(1)
    counts = {}
    for r in out:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    print(f"OK {key['sheet_id']}: {len(out)} verdicts {counts}")
    if not a.write:
        print("dry run: pass --write to record them")
        return
    dest = ROOT / "assets" / "human-audio" / key["lang"] / "verdicts.json"
    dest.parent.mkdir(parents=True, exist_ok=True)
    store = json.loads(dest.read_text()) if dest.exists() else {"lang": key["lang"], "entries": {}}
    when = time.strftime("%Y-%m-%d")
    for r in out:
        store["entries"].setdefault(r["entry"], []).append(
            {**{k: v for k, v in r.items() if k != "entry"},
             "sheet": key["sheet_id"] + (f" rows {a.rows}" if a.rows else ""),
             "auditor": a.auditor, "ingested": when})
    store["entries"] = dict(sorted(store["entries"].items()))
    dest.write_text(json.dumps(store, ensure_ascii=False, indent=1) + "\n")
    print(f"wrote {dest}")


if __name__ == "__main__":
    main()
