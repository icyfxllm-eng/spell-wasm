#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""CC-DE-BANK F4 -- the German audit ingest. The ONLY write path for auditor sheets.

    python3 tools/bank/de/de_ingest.py stamp SHEET.xlsx MANIFEST.json     # export side
    python3 tools/bank/de/de_ingest.py ingest SHEET.xlsx MANIFEST.json [--out FILE] [--report FILE]
    python3 tools/bank/de/de_ingest.py apply  SHEET.xlsx MANIFEST.json     # also edit assets/words/de

It rejects the WHOLE sheet, with a row-level error report, and never cleans
anything silently: a sheet is either exactly right or it goes back to the auditor.
Nothing here names a language; the column layout and the guards are the contract.
"""
import hashlib, json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from xlsx_min import read_sheet  # noqa: E402

ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
HEADER = ["row_id", "Word", "Tier", "Decision", "Correct Form", "Same Sound", "Notes"]
LOCKED = (0, 1, 2)                      # row_id, Word, Tier: the auditor may not touch these
DECISIONS = {"Keep", "Cut", "Unsure"}
NOT_A_HOMOPHONE = {"yes", "no", "x"}
END_PUNCT = ".,;:"
TIERS = ["easy", "medium", "hard", "expert"]


def locked_digest(rows):
    """sha256 over the header and every non-editable cell, in row order."""
    h = hashlib.sha256()
    h.update("\x1f".join(HEADER).encode("utf-8") + b"\n")
    for r in rows:
        r = (r + [""] * 7)[:7]
        h.update("\x1f".join(r[i] for i in LOCKED).encode("utf-8") + b"\n")
    return h.hexdigest()


def load(path):
    rows = read_sheet(path, "Words")
    return rows[0] if rows else [], [(r + [""] * 7)[:7] for r in rows[1:] if any(c != "" for c in r)]


def stamp(sheet, manifest):
    header, rows = load(sheet)
    if header[:7] != HEADER:
        raise SystemExit("header differs from %r" % HEADER)
    json.dump({"columns": HEADER, "locked": [HEADER[i] for i in LOCKED], "rows": len(rows),
               "row_ids": [r[0] for r in rows], "sha256": locked_digest(rows)},
              open(manifest, "w", encoding="utf-8"), indent=1, ensure_ascii=False)


def check(header, rows, manifest):
    """Every violation as (row_id, column, code, message). Empty list == acceptable."""
    errs = []
    m = json.load(open(manifest, encoding="utf-8"))
    if header[:7] != HEADER:
        errs.append(("-", "header", "HEADER", "columns differ from the export: %r" % (header[:7],)))
    if len(rows) != m["rows"] or [r[0] for r in rows] != m["row_ids"]:
        errs.append(("-", "rows", "ROWSET", "row set or row order differs from the export (%d rows, expected %d)" % (len(rows), m["rows"])))
    elif locked_digest(rows) != m["sha256"]:
        errs.append(("-", "row_id/Word/Tier", "LOCKED", "a non-editable column differs from the hash-stamped export"))
    for r in rows:
        rid, word, _tier, dec, cf, ss, note = r
        for name, v in (("Correct Form", cf), ("Same Sound", ss), ("Notes", note)):
            if v != v.strip():
                errs.append((rid, name, "WHITESPACE", "leading or trailing whitespace: %r" % v))
        if dec not in DECISIONS:
            errs.append((rid, "Decision", "DECISION", "%r is not one of Keep/Cut/Unsure" % dec))
        if cf and cf[-1] in END_PUNCT:
            errs.append((rid, "Correct Form", "PUNCT", "ends in punctuation: %r" % cf))
        if cf and cf == word:
            errs.append((rid, "Correct Form", "SAME_AS_WORD", "equals Word exactly; leave it empty if nothing is wrong"))
        s = ss.strip()
        if s and (s.lower() in NOT_A_HOMOPHONE or s.lower() == word.strip().lower() or (cf and s.lower() == cf.strip().lower())):
            errs.append((rid, "Same Sound", "NOT_A_HOMOPHONE", "%r is not another word that sounds the same" % ss))
        if dec in ("Cut", "Unsure") and not note.strip():
            errs.append((rid, "Notes", "NOTE_REQUIRED", "%s row has no Note" % dec))
    return errs


def entries(rows):
    """Keep rows -> bank entries (stored spelling = Correct Form, else Word)."""
    out = []
    for rid, word, tier, dec, cf, ss, _n in rows:
        if dec == "Keep":
            e = {"row_id": rid, "word": cf or word, "tier": tier.lower(), "homophones": [ss] if ss else []}
            out.append(e)
    return out


def report_text(errs):
    return "".join("%s\t%s\t%s\t%s\n" % e for e in errs)


def apply_to_lists(rows, words_dir):
    """Cut rows leave the tier lists; Keep rows are rewritten to Correct Form. Returns notes."""
    notes = []
    files = {t: os.path.join(words_dir, t + ".txt") for t in TIERS}
    lines = {t: open(p, encoding="utf-8").read().split("\n") for t, p in files.items()}
    for rid, word, _t, dec, cf, _s, _n in rows:
        hit = [(t, i) for t in TIERS for i, l in enumerate(lines[t]) if l == word]
        if not hit:
            notes.append("%s: %r not in any tier list (nothing to change)" % (rid, word))
            continue
        for t, i in hit:
            if dec == "Cut":
                lines[t][i] = None
            elif dec == "Keep" and cf:
                lines[t][i] = cf
    for t in TIERS:
        seen, out = set(), []
        for l in lines[t]:
            if l is None:
                continue
            if l and not l.startswith("#"):
                if l in seen:
                    notes.append("dedup in %s: %r" % (t, l))
                    continue
                seen.add(l)
            out.append(l)
        open(files[t], "w", encoding="utf-8").write("\n".join(out))
    return notes


def main(argv):
    if len(argv) < 4 or argv[1] not in ("stamp", "ingest", "apply"):
        raise SystemExit(__doc__)
    cmd, sheet, manifest = argv[1:4]
    opt = dict(zip(argv[4::2], argv[5::2]))
    if cmd == "stamp":
        return stamp(sheet, manifest)
    header, rows = load(sheet)
    errs = check(header, rows, manifest)
    if errs:
        txt = report_text(errs)
        if opt.get("--report"):
            open(opt["--report"], "w", encoding="utf-8").write(txt)
        sys.stderr.write("REJECTED: %d violation(s)\nrow_id\tcolumn\tcode\tmessage\n%s" % (len(errs), txt))
        raise SystemExit(1)
    es = entries(rows)
    out = opt.get("--out", os.path.join(ROOT, "data", "de", "bank-b1.json"))
    os.makedirs(os.path.dirname(out), exist_ok=True)
    json.dump({"source": os.path.basename(sheet), "entries": es,
               "cuts": [{"row_id": r[0], "word": r[1], "note": r[6]} for r in rows if r[3] == "Cut"],
               "unsure": [r[0] for r in rows if r[3] == "Unsure"]},
              open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    print("accepted: %d Keep entries, %d Cut, %d Unsure -> %s" % (len(es), sum(r[3] == "Cut" for r in rows), sum(r[3] == "Unsure" for r in rows), out))
    if cmd == "apply":
        for n in apply_to_lists(rows, os.path.join(ROOT, "assets", "words", "de")):
            print("  " + n)


if __name__ == "__main__":
    main(sys.argv)
