#!/usr/bin/env python3
"""CC-BANK-PURITY C1 ruling, Filipino — rows the recompute and the evidence run disagree on, for Paul.

    python3 tools/bank/purity_c1.py export [--out audit]
    python3 tools/bank/purity_c1.py apply <review.csv> --signed-by "Name" [--date YYYY-MM-DD] [--dry-run]

Eric's ruling (2026-10-12): neither run is followed wholesale.  The 58 proper nouns the recompute adds stay out (P2).
The rows below go to the Filipino auditor, because P5 keeps Filipinized spellings in and raw English out and the
checker cannot tell which is which (arena, diploma, editor are Filipino words; laptop, video, virus are borderline):

  quarantined  the 86 rows only the recompute quarantines, classed wrong_language_english / wrong_language_other,
               and the 17 rows only the evidence run quarantined as wrong_language_english (`rule` moves those out)

`rule` applies the evidence-only half of Eric's ruling: the 17 leave the bank under a signed override.  The 86 were
already out.  A quarantined row cannot re-enter as `pending` (the ledger law), so the Filipino auditor's `keep` is
the only way back, as a signed exception.

export  <out>/fil/c1-review.csv, blank `decision` (keep / cut / unsure).  Rows already ruled are left out.
apply   keep on a quarantined row  -> back into the bank at its pre-sweep position as a signed `exception`
        cut  on an in_bank row     -> quarantined (manifest class auditor_cut), same cascade as F8
        keep on an in_bank row     -> recorded, nothing moves
        cut  on a quarantined row  -> recorded, stays out
        unsure                     -> nothing; it comes back on the next export
Every decision is appended to assets/words/purity/c1-ruled/fil.txt (word, decision, signer, date).
A blank decision is an error; so is a row that is not in the review set.  No threshold is edited.
"""
import argparse, csv, os, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import purity_gate as g
import purity_sheet as ps

LANG = "fil"
nfc = lambda s: unicodedata.normalize("NFC", s)
COLS = "lang,tier,word,state,class,evidence,decision,note".split(",")
DECISIONS = ("keep", "cut", "unsure")
RULED = f"{REPO}/assets/words/purity/c1-ruled/{LANG}.txt"


def ruled():
    if not os.path.exists(RULED):
        return {}
    out = {}
    for l in open(RULED, encoding="utf-8"):
        f = l.rstrip("\n").split("\t")
        if f and f[0] and f[0][0] != "#" and len(f) > 1:
            out[nfc(f[0])] = f[1]
    return out


def review_set():
    """word -> (tier, state, class, evidence) for the rows this ruling covers and nobody has ruled yet."""
    diff = list(csv.DictReader(open(f"{REPO}/reports/bank-purity-c1-diff.csv", encoding="utf-8")))
    diff = [r for r in diff if r["lang"] == LANG]
    man = {nfc(r["word"]): r for r in csv.DictReader(open(f"{REPO}/assets/words-quarantine/manifest.csv", encoding="utf-8"))
           if r["lang"] == LANG}
    inbank = {nfc(w): t for t, w in g.bank(REPO, LANG)}
    done = ruled()
    out = {}
    for r in diff:
        w = nfc(r["word"])
        if w in done:
            continue
        covered = (r["side"] == "now_only" and r["now_class"] in ("wrong_language_english", "wrong_language_other")) or \
                  (r["side"] == "evidence_only" and r["evidence_class"] == "wrong_language_english")
        if not covered:
            continue
        if w in man:
            out[w] = (man[w]["tier"], "quarantined", man[w]["class"], man[w]["evidence"])
        elif w in inbank:
            out[w] = (inbank[w], "in_bank", "evidence: wrong_language_english", "checker passes it")
    return out


def rule(date, signer, dry):
    """Eric's ruling, applied: the evidence-only rows the evidence run classed wrong_language_english leave the bank.
    A signed override (so F1 agrees) plus the F8 quarantine move.  They go on Paul's sheet as `quarantined`."""
    inbank = {nfc(w): t for t, w in g.bank(REPO, LANG)}
    diff = [r for r in csv.DictReader(open(f"{REPO}/reports/bank-purity-c1-diff.csv", encoding="utf-8"))
            if r["lang"] == LANG and r["side"] == "evidence_only" and r["evidence_class"] == "wrong_language_english"]
    words = [nfc(r["word"]) for r in diff if nfc(r["word"]) in inbank]
    print(f"{LANG}: {len(words)} evidence-only wrong_language_english rows in the bank: {' '.join(words)}")
    if dry:
        return
    op = f"{HERE}/purity_overrides.csv"
    have = {(r["lang"], r["word"]) for r in csv.DictReader(open(op, encoding="utf-8"))}
    with open(op, "a", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        for x in words:
            if (LANG, x) not in have:
                w.writerow((LANG, x, "wrong_language_english", f"{signer} {date}",
                            "C1 ruling: fil evidence-only English rows out; the Filipino auditor may restore via c1-review.csv"))
    ps.apply_cuts(LANG, set(words), "wrong_language_english", f"C1 ruling {signer} {date}: evidence run", date)
    ps.rebuild_and_rehash()

def export(out):
    rs = review_set()
    d = f"{out}/{LANG}"
    os.makedirs(d, exist_ok=True)
    with open(f"{d}/c1-review.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n"); w.writerow(COLS)
        for word, (t, st, c, e) in sorted(rs.items(), key=lambda kv: (kv[1][1], g.TIERS.index(kv[1][0]), kv[0])):
            w.writerow((LANG, t, word, st, c, e, "", ""))
    n = {s: sum(1 for v in rs.values() if v[1] == s) for s in ("quarantined", "in_bank")}
    print(f"{LANG}: {n['quarantined']} quarantined + {n['in_bank']} in bank -> {d}/c1-review.csv")


def apply(path, signer, date, dry):
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    if not rows:
        sys.exit("empty review")
    if {r["lang"] for r in rows} != {LANG}:
        sys.exit(f"this ruling is {LANG} only")
    blank = [r["word"] for r in rows if r["decision"].strip().lower() not in DECISIONS]
    if blank:
        sys.exit(f"{len(blank)} row(s) have no keep/cut/unsure decision (first: {blank[:5]}); refusing a half-answered review")
    rs = review_set()
    dec = {nfc(r["word"]): r["decision"].strip().lower() for r in rows}
    stray = [w for w in dec if w not in rs]
    if stray:
        sys.exit(f"{len(stray)} row(s) are not in the open review set (first: {stray[:5]})")
    restore = {w for w, d in dec.items() if d == "keep" and rs[w][1] == "quarantined"}
    cut = {w for w, d in dec.items() if d == "cut" and rs[w][1] == "in_bank"}
    print(f"{LANG}: {len(restore)} restored, {len(cut)} cut from the bank, "
          f"{sum(1 for w, d in dec.items() if d != 'unsure') - len(restore) - len(cut)} confirmed as they stand, "
          f"{sum(1 for d in dec.values() if d == 'unsure')} unsure")
    if dry:
        print("dry run: nothing written")
        return
    ps.restore_rows(LANG, restore, signer, "auditor keep: a Filipino word, not raw English or a name", date)
    ps.apply_cuts(LANG, cut, "auditor_cut", f"{signer} {date} (C1 evidence-only)", date)
    os.makedirs(os.path.dirname(RULED), exist_ok=True)
    head = "" if os.path.exists(RULED) else "# C1 Filipino ruling: word<TAB>decision<TAB>signer<TAB>date (purity_c1.py)\n"
    with open(RULED, "a", encoding="utf-8") as f:
        f.write(head)
        for w, d in sorted(dec.items()):
            if d != "unsure":
                f.write(f"{w}\t{d}\t{signer}\t{date}\n")
    ps.rebuild_and_rehash()
    print("next: python3 tools/bank/purity_gate.py && python3 tools/bank/ledger_check.py --restore 3e54e8d6^ && bash scripts/gate.sh")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("export"); e.add_argument("--out", default="audit")
    r = sub.add_parser("rule"); r.add_argument("--signed-by", default="Eric"); r.add_argument("--date", default="2026-10-12")
    r.add_argument("--dry-run", action="store_true")
    a = sub.add_parser("apply"); a.add_argument("review"); a.add_argument("--signed-by", required=True)
    a.add_argument("--date", default="2026-10-12"); a.add_argument("--dry-run", action="store_true")
    x = ap.parse_args()
    if x.cmd == "export":
        export(x.out)
    elif x.cmd == "rule":
        rule(x.date, x.signed_by, x.dry_run)
    else:
        apply(x.review, x.signed_by, x.date, x.dry_run)
