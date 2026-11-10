#!/usr/bin/env python3
"""CC-LAUNCH-EN rev 4: build the server's English definition files from the reviewed rows.

    python3 tools/defs/build_server_en.py build/defs-audit/en-gen-20261109-r2/reviewed.en.json [--check]

Writes, for the content deploy (Eric deploys; the server serves them only to clients
that ask with src=gen1, so builds already out keep today's content, D-DEF11):
  backend/def_pools_gen1/en.json   Definition Match pool: same shape as en.json
                                   (scripts/build-def-pools.py), reviewed definitions only
  backend/meanings/en.gen1.json    word -> {pos, definition, example, reviewed_definition,
                                   reviewed_sentence, source, source_license}; a field that
                                   is not reviewed ships as "" with its flag false, so
                                   unreviewed text never leaves the repo

Every row is source spell-gen-gen-v2, source_license original (A15.9). Only words still in
the served bank (assets/words/en/<tier>.txt) are written. --check fails if the committed
files differ from a fresh build.
"""
import csv
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
TIERS = ["easy", "medium", "hard", "expert"]
POOL = ROOT / "backend" / "def_pools_gen1" / "en.json"
MEANINGS = ROOT / "backend" / "meanings" / "en.gen1.json"
SOURCE, LICENSE = "spell-gen-gen-v2", "original"
SUFFIXES = ("s", "es", "ed", "d", "ing", "er", "est", "ly")


def bank():
    out = {}
    for t in TIERS:
        for l in (ROOT / "assets" / "words" / "en" / f"{t}.txt").read_text(encoding="utf-8").splitlines():
            w = l.strip()
            if w and not w.startswith("#"):
                out.setdefault(w, t)
    return out


def pos_of(run_rows: pathlib.Path):
    return {(r["word"], r["tier"]): r["pos"] for r in csv.DictReader(open(run_rows, encoding="utf-8"))}


def build(reviewed_path: pathlib.Path):
    rows = json.loads(reviewed_path.read_text(encoding="utf-8"))
    meta = json.loads((reviewed_path.parent / "meta.json").read_text())
    pos = pos_of(pathlib.Path(meta["run"]) / "rows.csv")
    served = bank()
    rows = [r for r in rows if served.get(r["word"]) == r["tier"]]
    for r in rows:
        assert r["source"] == SOURCE and r["source_license"] == LICENSE, r["word"]

    meanings = {}
    for r in sorted(rows, key=lambda r: r["word"]):
        p = pos.get((r["word"], r["tier"]), "")
        meanings[r["word"]] = {
            "pos": "" if p in ("other", "function word") else p,
            "definition": r["definition"] if r["reviewed_definition"] else "",
            "example": r["sentence"] if r["reviewed_sentence"] else "",
            "reviewed_definition": bool(r["reviewed_definition"]),
            "reviewed_sentence": bool(r["reviewed_sentence"]),
            "source": SOURCE, "source_license": LICENSE,
        }

    pools, excl = {t: [] for t in TIERS}, {}
    for r in sorted(rows, key=lambda r: r["word"]):
        if not r["reviewed_definition"]:
            continue
        p = pos.get((r["word"], r["tier"]), "")
        pools[r["tier"]].append({
            "word": r["word"], "definition": r["definition"], "pos": "" if p in ("other", "function word") else p,
            "prompt_grade": True,
            # Same kid_register rule as scripts/build-def-pools.py (easy/medium, <= 60 chars).
            "kid_register": bool(r["kid_visible"]) and r["tier"] in ("easy", "medium") and len(r["definition"]) <= 60,
            "reviewed_definition": True, "reviewed_sentence": bool(r["reviewed_sentence"]),
            "source": SOURCE, "source_license": LICENSE,
        })
    for t, tier_rows in pools.items():
        # Same rules as scripts/build-def-pools.py: identical definitions are not
        # prompt-grade and exclude each other; a word named in another's definition
        # excludes it; plus inflections of one stem (cell / cells) exclude each other.
        seen = {}
        for row in tier_rows:
            seen.setdefault(row["definition"].lower(), []).append(row)
        for group in seen.values():
            if len(group) > 1:
                for row in group:
                    row["prompt_grade"] = False
                    excl.setdefault(row["word"], set()).update(g["word"] for g in group if g["word"] != row["word"])
        vocab = {row["word"].lower(): row["word"] for row in tier_rows}
        for row in tier_rows:
            for tok in re.findall(r"[\w']+", row["definition"].lower()):
                hit = vocab.get(tok)
                if hit and hit != row["word"]:
                    excl.setdefault(row["word"], set()).add(hit)
                    excl.setdefault(hit, set()).add(row["word"])
        words = set(vocab.values())
        for w in words:
            for suf in SUFFIXES:
                if w.endswith(suf) and len(w) - len(suf) >= 2:
                    stem = w[: -len(suf)]
                    for s in (stem, stem + "e") if suf in ("ed", "d", "ing", "er", "est") else (stem,):
                        if s in words and s != w:
                            excl.setdefault(w, set()).add(s)
                            excl.setdefault(s, set()).add(w)
    pool = {"lang": "en", "source": SOURCE, "source_license": LICENSE, "tiers": pools,
            "exclusions": {k: sorted(v) for k, v in sorted(excl.items())}}
    return pool, meanings


def dumps(obj):
    return json.dumps(obj, ensure_ascii=False, separators=(",", ":"), sort_keys=False) + "\n"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    check = "--check" in sys.argv
    pool, meanings = build(pathlib.Path(args[0]))
    out = {POOL: dumps(pool), MEANINGS: dumps(meanings)}
    if check:
        stale = [str(p.relative_to(ROOT)) for p, s in out.items() if not p.exists() or p.read_text(encoding="utf-8") != s]
        if stale:
            sys.exit(f"stale: {stale}")
        print("OK")
        return
    for p, s in out.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(s, encoding="utf-8")
    counts = {t: len(v) for t, v in pool["tiers"].items()}
    kid = {t: sum(r["kid_register"] for r in v) for t, v in pool["tiers"].items()}
    print(json.dumps({"poolRows": counts, "kidRegister": kid, "meanings": len(meanings),
                      "meaningsWithSentence": sum(bool(m["example"]) for m in meanings.values()),
                      "meaningsWithDefinition": sum(bool(m["definition"]) for m in meanings.values())}, indent=1))


if __name__ == "__main__":
    main()
