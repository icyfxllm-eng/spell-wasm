#!/usr/bin/env python3
"""CC-BANK-PURITY F8 — auditor sheets for unconfirmed rows, and their ingest.

    python3 tools/bank/purity_sheet.py export [--out audit] [lang ...]
    python3 tools/bank/purity_sheet.py ingest <sheet.csv> --auditor "Name" [--date YYYY-MM-DD]

export  writes <out>/<lang>/purity-sheet.csv: one row per `pending` ledger row (class and evidence from F1) and one per
        F4 correction (reports/bank-corrections.csv), with blank `verdict` (keep / cut / unsure) and
        `not_for_children` columns for the native auditor.  It changes nothing in the repo.
ingest  applies a returned sheet, one language per file, in one change:
          keep   -> the ledger line becomes `exception` signed by --auditor, reason recorded (a correction stays)
          cut    -> the row moves to assets/words-quarantine/<lang>/<tier>.txt (manifest class `auditor_cut`), its
                    gloss and definition-pool rows are pruned, its ledger line goes
          unsure -> stays pending
          not_for_children = x -> the word is added to assets/words/kid-exclude/<lang>.txt above the F7 marker
        A blank verdict is an error (an unanswered row would silently count as `unsure`).  Then it rebuilds
        src/word_data.rs and refreshes the ledger tier hashes.  After an ingest the language's `pending` count equals
        the sheet's `unsure` count (checked before it exits 0).
"""
import argparse, csv, json, os, re, subprocess, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import purity_gate as g
from seed_kid_lists import MARK

COLS = "lang,tier,word,kind,class,evidence,verdict,not_for_children,note".split(",")
nfc = lambda s: unicodedata.normalize("NFC", s)

def f1_rows():
    """word -> (class, evidence) from the F1 run over the current bank (reports/bank-purity-rows-after.csv)."""
    p = f"{REPO}/reports/bank-purity-rows-after.csv"
    return {(r["lang"], r["word"]): (r["class"], r["evidence"]) for r in csv.DictReader(open(p, encoding="utf-8"))}

def pending_words(lang):
    return [f.split("\t")[0] for f in g.lines(f"{REPO}/assets/words/purity/{lang}.tsv") if f.split("\t")[1] == "pending"]

def export(out, langs):
    f1 = f1_rows()
    corr = {}
    for r in csv.DictReader(open(f"{REPO}/reports/bank-corrections.csv", encoding="utf-8")):
        corr.setdefault(r["lang"], []).append(r)
    for lang in langs:
        tiers = {w: t for t, w in g.bank(REPO, lang)}
        rows = []
        for w in pending_words(lang):
            c, e = f1.get((lang, w), ("not_in_reference_dictionary", ""))
            rows.append((lang, tiers.get(w, ""), w, "pending", c, e, "", "", ""))
        for r in corr.get(lang, []):
            if r["new"] in tiers:
                rows.append((lang, r["tier"], r["new"], "correction", "misspelled_missing_diacritic", f"was {r['old']} ({r['dictionary']})", "", "", ""))
        if not rows:
            continue
        d = f"{out}/{lang}"
        os.makedirs(d, exist_ok=True)
        with open(f"{d}/purity-sheet.csv", "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n"); w.writerow(COLS); w.writerows(rows)
        print(f"{lang}: {sum(1 for r in rows if r[3] == 'pending')} pending, {sum(1 for r in rows if r[3] == 'correction')} corrections -> {d}/purity-sheet.csv")

def apply_cuts(lang, cuts, klass, evidence, date):
    """Move `cuts` (words of `lang` that are bank rows) out of the bank into assets/words-quarantine/, record them
    in the manifest under `klass`, prune their gloss and definition-pool rows, and drop their ledger and
    grandfathered lines.  Returns the set actually moved.  Does not rebuild: call rebuild_and_rehash() after.
    Shared by F8's `cut` and by purity_batch2 (the C7 proper-noun batch)."""
    cuts = {nfc(w) for w in cuts}
    lines = {}
    for t in g.TIERS:
        p = f"{REPO}/assets/words/{lang}/{t}.txt"
        if os.path.exists(p):
            lines[t] = [x for x in open(p, encoding="utf-8").read().split("\n") if x.strip()]
    in_bank = {nfc(w) for ws in lines.values() for w in ws}
    cuts = {w for w in cuts if w in in_bank}
    # The manifest `pos` is an index in the PRE-SWEEP tier file (ledger_check --restore re-inserts rows in
    # ascending pos).  A row's pre-sweep index is its place once every row already in the manifest is put back.
    mp = f"{REPO}/assets/words-quarantine/manifest.csv"
    prior = {}
    for r in csv.DictReader(open(mp, encoding="utf-8")):
        if r["lang"] == lang:
            prior.setdefault(r["tier"], []).append(int(r["pos"]))
    manifest_rows = []
    for t, ws in lines.items():
        full = list(range(len(ws)))
        for p0 in sorted(prior.get(t, [])):
            full.insert(p0, None)
        kept, moved = [], []
        for i, w in enumerate(ws):
            if nfc(w) in cuts:
                moved.append(w); manifest_rows.append((lang, w, t, full.index(i), klass, evidence, date))
            else:
                kept.append(w)
        if moved:
            open(f"{REPO}/assets/words/{lang}/{t}.txt", "w", encoding="utf-8").write("\n".join(kept) + "\n")
            qd = f"{REPO}/assets/words-quarantine/{lang}"; os.makedirs(qd, exist_ok=True)
            qp = f"{qd}/{t}.txt"
            prev = open(qp, encoding="utf-8").read().split("\n") if os.path.exists(qp) else []
            open(qp, "w", encoding="utf-8").write("\n".join([x for x in prev if x] + moved) + "\n")
    if manifest_rows:
        with open(f"{REPO}/assets/words-quarantine/manifest.csv", "a", newline="", encoding="utf-8") as f:
            csv.writer(f, lineterminator="\n").writerows(manifest_rows)
    gone = {w.rsplit("|", 1)[-1] for w in cuts}
    gp = f"{REPO}/config/gloss/{lang}.json"
    if os.path.exists(gp) and gone:
        d = json.load(open(gp, encoding="utf-8")); d["rows"] = {k: v for k, v in d["rows"].items() if nfc(k) not in gone}
        open(gp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False, indent=2))
    dp = f"{REPO}/backend/def_pools/{lang}.json"
    if os.path.exists(dp) and gone:
        d = json.load(open(dp, encoding="utf-8"))
        d["tiers"] = {t: [e for e in es if nfc(e["word"]) not in gone] for t, es in d["tiers"].items()}
        open(dp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False))
    lp = f"{REPO}/assets/words/purity/{lang}.tsv"
    out = [l for l in open(lp, encoding="utf-8").read().split("\n") if l and (l[0] == "#" or nfc(l.split("\t")[0]) not in cuts)]
    open(lp, "w", encoding="utf-8").write("\n".join(out) + "\n")
    gf = f"{REPO}/assets/words/purity/grandfathered/{lang}.txt"
    if os.path.exists(gf):
        keep = [l for l in open(gf, encoding="utf-8").read().split("\n") if l and (l[0] == "#" or nfc(l) not in cuts)]
        open(gf, "w", encoding="utf-8").write("\n".join(keep) + "\n")
    return cuts

PRESWEEP = "3e54e8d6^"   # the commit before the Phase A sweep; restored rows get their gloss/definition rows back from here

def restore_rows(lang, words, signer, reason, date, base=PRESWEEP):
    """Put quarantined rows back into the bank at their pre-sweep position, each with a signed `exception` ledger
    line.  A quarantined row never re-enters as `pending`: the only way back is a named person's keep.
    Removes their manifest and quarantine-file lines and restores their gloss and definition-pool rows from `base`
    when the current file has none.  Returns the words restored.  Does not rebuild: call rebuild_and_rehash()."""
    want = {nfc(w) for w in words}
    mp = f"{REPO}/assets/words-quarantine/manifest.csv"
    with open(mp, encoding="utf-8") as f:
        rd = csv.DictReader(f); head = rd.fieldnames; man = list(rd)
    done = set()
    for t in g.TIERS:
        p = f"{REPO}/assets/words/{lang}/{t}.txt"
        if not os.path.exists(p):
            continue
        cur = [x for x in open(p, encoding="utf-8").read().split("\n") if x.strip()]
        qs = sorted((r for r in man if r["lang"] == lang and r["tier"] == t), key=lambda r: int(r["pos"]))
        sel = {nfc(r["word"]) for r in qs} & want
        if not sel:
            continue
        ent = [("c", w) for w in cur]
        for r in qs:
            ent.insert(int(r["pos"]), ("q", r["word"]))
        new = [w for k, w in ent if k == "c" or nfc(w) in sel]
        open(p, "w", encoding="utf-8").write("\n".join(new) + "\n")
        qp = f"{REPO}/assets/words-quarantine/{lang}/{t}.txt"
        if os.path.exists(qp):
            ql = [x for x in open(qp, encoding="utf-8").read().split("\n") if x]
            open(qp, "w", encoding="utf-8").write("\n".join(x for x in ql if nfc(x) not in sel) + "\n")
        done |= sel
    man = [r for r in man if not (r["lang"] == lang and nfc(r["word"]) in done)]
    with open(mp, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=head, lineterminator="\n"); w.writeheader(); w.writerows(man)
    # ledger: bank order, signed exception for each restored row
    lp = f"{REPO}/assets/words/purity/{lang}.tsv"
    old = open(lp, encoding="utf-8").read().split("\n")
    top = [l for l in old if l.startswith("#")]
    by = {nfc(l.split("\t")[0]): l for l in old if l and l[0] != "#"}
    body = []
    for t, w in g.bank(REPO, lang):
        k = nfc(w)
        body.append(by[k] if k in by and k not in done else f"{w}\texception\tpurity-restore\t{signer}\t{reason} ({date})")
    open(lp, "w", encoding="utf-8").write("\n".join(top + body) + "\n")
    # gloss and definition pool rows the sweep pruned
    def show(path):
        r = subprocess.run(["git", "show", f"{base}:{path}"], cwd=REPO, capture_output=True, text=True, encoding="utf-8")
        return json.loads(r.stdout) if r.returncode == 0 and r.stdout.strip() else None
    gp = f"{REPO}/config/gloss/{lang}.json"
    old_g = show(f"config/gloss/{lang}.json")
    if old_g and os.path.exists(gp):
        d = json.load(open(gp, encoding="utf-8"))
        for k, v in old_g["rows"].items():
            if nfc(k.rsplit("|", 1)[-1]) in done and k not in d["rows"]:
                d["rows"][k] = v
        open(gp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False, indent=2))
    dp = f"{REPO}/backend/def_pools/{lang}.json"
    old_d = show(f"backend/def_pools/{lang}.json")
    if old_d and os.path.exists(dp):
        d = json.load(open(dp, encoding="utf-8"))
        have = {nfc(e["word"]) for es in d["tiers"].values() for e in es}
        for t, es in old_d["tiers"].items():
            for e in es:
                if nfc(e["word"]) in done and nfc(e["word"]) not in have:
                    d["tiers"].setdefault(t, []).append(e)
        open(dp, "w", encoding="utf-8").write(json.dumps(d, ensure_ascii=False))
    return done

def rebuild_and_rehash():
    """Regenerate src/word_data.rs and refresh every ledger's #tier hashes."""
    subprocess.run([sys.executable, f"{REPO}/scripts/build-wordlists.py"], check=True, cwd=REPO, stdout=subprocess.DEVNULL)
    th = g.tier_hashes(REPO)
    for l in g.LANGS:
        p = f"{REPO}/assets/words/purity/{l}.tsv"
        L = open(p, encoding="utf-8").read().split("\n")
        for i, x in enumerate(L):
            if x.startswith("#tier\t"):
                t = x.split("\t")[1]
                if (l, t) in th:
                    L[i] = f"#tier\t{t}\t{th[(l, t)]}"
        open(p, "w", encoding="utf-8").write("\n".join(L))

def ingest(path, auditor, date):
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    if not rows:
        sys.exit("empty sheet")
    langs = {r["lang"] for r in rows}
    if len(langs) != 1:
        sys.exit(f"one language per sheet, got {sorted(langs)}")
    lang = langs.pop()
    bad = [r["word"] for r in rows if r["verdict"].strip().lower() not in ("keep", "cut", "unsure")]
    if bad:
        sys.exit(f"{len(bad)} row(s) have no keep/cut/unsure verdict (first: {bad[:5]}); refusing a half-answered sheet")
    verdict = {nfc(r["word"]): r["verdict"].strip().lower() for r in rows}
    kind = {nfc(r["word"]): r["kind"] for r in rows}
    cuts = {w for w, v in verdict.items() if v == "cut"}
    cuts = apply_cuts(lang, cuts, "auditor_cut", f"{auditor} {date}", date)
    # keeps: a pending ledger line becomes a signed exception
    lp = f"{REPO}/assets/words/purity/{lang}.tsv"
    out = []
    for line in open(lp, encoding="utf-8").read().split("\n"):
        if not line:
            continue
        f = line.split("\t")
        w = nfc(f[0]) if line[0] != "#" else None
        if w and verdict.get(w) == "keep" and kind.get(w) == "pending" and f[1] == "pending":
            line = f"{f[0]}\texception\t{f[2]}\t{auditor}\tauditor keep ({date})"
        out.append(line)
    open(lp, "w", encoding="utf-8").write("\n".join(out) + "\n")
    # grandfathered shrinks for the keeps (apply_cuts already shrank it for the cuts)
    gf = f"{REPO}/assets/words/purity/grandfathered/{lang}.txt"
    if os.path.exists(gf):
        done = {w for w, v in verdict.items() if v == "keep"}
        keep = [l for l in open(gf, encoding="utf-8").read().split("\n") if l and (l[0] == "#" or nfc(l) not in done)]
        open(gf, "w", encoding="utf-8").write("\n".join(keep) + "\n")
    # Jr list
    nfc_rows = [nfc(r["word"]).rsplit("|", 1)[-1] for r in rows if r["not_for_children"].strip().lower() in ("x", "yes", "y", "1")]
    if nfc_rows:
        kp = f"{REPO}/assets/words/kid-exclude/{lang}.txt"
        s = open(kp, encoding="utf-8").read()
        hand, _, gen = s.partition(MARK)
        have = {nfc(l.strip()) for l in hand.split("\n")}
        add = [w for w in nfc_rows if w not in have]
        open(kp, "w", encoding="utf-8").write(hand.rstrip("\n") + "\n" + "".join(f"{w}\n" for w in add) + (MARK + gen if _ else ""))
    rebuild_and_rehash()
    left = len(pending_words(lang))
    unsure = sum(1 for w, v in verdict.items() if v == "unsure" and kind[w] == "pending")
    print(f"{lang}: {len([w for w in verdict if verdict[w]=='keep' and kind[w]=='pending'])} kept, {len(cuts)} cut, {unsure} unsure; pending now {left}")
    if left != unsure:
        sys.exit(f"ACCEPTANCE FAILED: pending {left} != sheet unsure {unsure} (rows on the ledger that were not on this sheet?)")

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("export"); e.add_argument("--out", default="audit"); e.add_argument("langs", nargs="*")
    i = sub.add_parser("ingest"); i.add_argument("sheet"); i.add_argument("--auditor", required=True); i.add_argument("--date", default="2026-10-08")
    a = ap.parse_args()
    if a.cmd == "export":
        export(a.out, a.langs or g.LANGS)
    else:
        ingest(a.sheet, a.auditor, a.date)
