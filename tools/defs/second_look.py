#!/usr/bin/env python3
"""CC-LAUNCH-EN rev 4: apply Eric's second look to a solo ingest.

    python3 tools/defs/second_look.py <sheet dir> <marked second-look.csv> \
        --held held.json --flags new_text_flags.json --out-words exclusions/en.txt

Answers in the last column (case and spaces ignored):
  yes            the held field is cleared. If Eric changed its text, the NEW text
                 is a new rewrite: it is cleared only when it passes the code checks
                 (tools/defgen check_field) and has no fresh-context safety flag
                 (--flags, ids "word|tier|field"); otherwise it stays unreviewed.
  no             stays unreviewed (fix queue).
  out, leave out the WORD leaves the game (Eric: "Out means leave it out of the game
                 completely!"); its rows are dropped here, and the word must be in
                 the --out-words exclusion list (build-wordlists.py removes it).
Rewrites reviewed.en.json in place (the solo ingest is kept as reviewed.solo.json)
and writes second-look.md.
"""
import argparse
import csv
import json
import pathlib
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "defgen"))


def lines(p):
    return {l.strip() for l in pathlib.Path(p).read_text(encoding="utf-8").splitlines()
            if l.strip() and not l.startswith("#")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sheet_dir")
    ap.add_argument("marked")
    ap.add_argument("--held", required=True)
    ap.add_argument("--flags", action="append", default=[],
                    help="fresh-context safety flags on Eric's NEW texts (ids word|tier|field)")
    ap.add_argument("--out-words", required=True)
    a = ap.parse_args()
    import defgen  # needs the defgen venv (nltk) only for its word lists and checks
    d = pathlib.Path(a.sheet_dir)
    solo = d / "reviewed.solo.json"
    if not solo.exists():
        shutil.copy(d / "reviewed.en.json", solo)
    rows = json.loads(solo.read_text(encoding="utf-8"))
    held = {tuple(x[:3]) for x in json.loads(pathlib.Path(a.held).read_text())}
    flagged = {f["id"] for p in a.flags for f in json.loads(pathlib.Path(p).read_text())}
    excluded = lines(a.out_words)
    lists = defgen.blocked_terms()
    by = {(r["word"], r["tier"]): r for r in rows}
    answers = [r for r in csv.DictReader(open(a.marked, encoding="utf-8")) if r.get("word")]
    rep = {"cleared": [], "cleared_new_text": [], "kept_off": [], "new_text_failed": [], "out_words": set()}
    for x in answers:
        k3 = (x["word"], x["tier"], x["field"])
        if k3 not in held:
            sys.exit(f"{k3} is not in the held list")
        ans = x[list(x)[-1]].strip().lower()
        r = by[(x["word"], x["tier"])]
        f = x["field"]
        if ans in {"out", "leave out"}:
            rep["out_words"].add(x["word"])
            continue
        if ans != "yes":
            rep["kept_off"].append(k3)
            continue
        text = x["text"].strip()
        if text == r[f]:   # a yes on the same text is Eric's later rating; it replaces his sheet score
            r["reviewed_" + f] = True
            rep["cleared"].append(k3)
            continue
        problems = defgen.check_field(f, text, x["word"], x["tier"], lists)
        if "|".join(k3) in flagged:
            problems.append("fresh-context safety flag")
        if problems:
            rep["new_text_failed"].append(k3 + (text, problems))
            continue
        r[f] = text
        r["reviewed_" + f] = True
        r["second_look"] = "eric rewrite, checked (rev 4)"
        rep["cleared_new_text"].append(k3)
    missing = rep["out_words"] - excluded
    if missing:
        sys.exit(f"out words not in {a.out_words}: {sorted(missing)}")
    rows = [r for r in rows if r["word"] not in rep["out_words"]]
    (d / "reviewed.en.json").write_text(json.dumps(rows, ensure_ascii=False, indent=0), encoding="utf-8")
    kid_unrev = [(r["word"], r["tier"]) for r in rows if r["kid_visible"] and r["definition"] and not r["reviewed_definition"]]
    tiers = {t: sum(r["reviewed_definition"] for r in rows if r["tier"] == t) for t in ("easy", "medium", "hard", "expert")}
    md = ["# Second look applied (rev 4)", "",
          f"- Cleared as written: {len(rep['cleared'])}",
          f"- Cleared with Eric's new text (code checks + safety review passed): {', '.join(k[0] for k in rep['cleared_new_text']) or 'none'}",
          f"- New text that failed the checks (stays unreviewed): "
          + ("; ".join(f"{k[0]} ({', '.join(k[4])})" for k in rep["new_text_failed"]) or "none"),
          f"- Kept off (fix queue): {', '.join(k[0] + ' ' + k[2] for k in rep['kept_off']) or 'none'}",
          f"- Words out of the game: {', '.join(sorted(rep['out_words']))}",
          f"- Rows now: {len(rows)}; reviewed definitions per tier: {tiers}",
          f"- Kid-visible words with an unreviewed definition: {', '.join(w for w, _ in kid_unrev) or 'none'}"]
    (d / "second-look.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("\n".join(md))


if __name__ == "__main__":
    main()
