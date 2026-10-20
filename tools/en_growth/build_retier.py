#!/usr/bin/env python3
"""CC-EN-EXPERT-GROWTH F4 -- build the re-tiered English bank (served only behind en.retier).

    python3 tools/en_growth/build_retier.py          # write the retier tier files, ledger and Rust consts
    python3 tools/en_growth/build_retier.py --check  # gate: everything committed matches a fresh build

Sources, all committed:
  * the current bank: assets/words/en/<tier>.txt (unchanged; still what plays with the flag OFF)
  * Eric's grades: calibration/en_graded.jsonl, plus reports/en-staged-review-sample-graded.csv
    (his first staged sheet: keep all, graded tiers agreed)

Rules (reports/en-retier-plan.md, signed 2026-10-17):
  * current Expert rows go where the re-grade puts them; cut -> quarantine list (never deleted, I9);
    unsure -> stays Expert
  * current Easy and Medium rows keep their tier (not re-graded)
  * current HARD rows go to Medium (Eric, 2026-10-19, after reading a random 100 of them: "they all look
    like medium difficulty words"). The old Hard tier was a letter-count tier (7-8 letters), not a
    difficulty tier; see reports/en-regrade-proposal.csv and reports/en-retier-promotion-evidence.md
  * staged words graded keep go to their graded tier; cut / unsure stay out
  * low-band words (Zipf 2.0-2.5) go in ONLY when graded Expert (D4 amendment: Expert candidates only)
  * every new word must pass the PURITY classifier (tools/bank/purity_admit) and the en exclusion
    lists, be lowercase a-z, and appear in exactly one tier

Outputs:
  assets/words/en/retier/{easy,medium,hard,expert}.txt   sorted
  assets/words/en/retier/quarantine.txt                  current rows cut by the re-grade
  assets/words/purity/en-retier.tsv                      PURITY verdicts for the rows not already in en.tsv
  src/word_data_en_retier.rs                             EN_RETIER_* consts (generated)
"""
import csv
import json
import os
import pathlib
import re
import sys
import unicodedata

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(ROOT / "tools" / "bank"))
TIERS = ["easy", "medium", "hard", "expert"]
GRADE_TO_TIER = {"Easy": "easy", "Medium": "medium", "Hard": "hard", "Expert": "expert"}
OUT = ROOT / "assets" / "words" / "en" / "retier"
LEDGER = ROOT / "assets" / "words" / "purity" / "en-retier.tsv"
RUST = ROOT / "src" / "word_data_en_retier.rs"


def read_list(p):
    return [l.strip() for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")]


def exclusions():
    """Same folding as scripts/build-wordlists.py for English (lenient, not accent-sensitive)."""
    def fold(s):
        s = unicodedata.normalize("NFD", unicodedata.normalize("NFC", s).lower())
        return "".join(c for c in s if not 0x300 <= ord(c) <= 0x36F and not c.isspace())
    ex = ROOT / "assets" / "words" / "exclusions"
    roots = [fold(l) for l in read_list(ex / "_roots.txt")] if (ex / "_roots.txt").exists() else []
    exact = {fold(l) for l in read_list(ex / "en.txt")} if (ex / "en.txt").exists() else set()
    return lambda w: fold(w) in exact or any(r and r in fold(w) for r in roots)


def build():
    current = {t: read_list(ROOT / "assets" / "words" / "en" / f"{t}.txt") for t in TIERS}
    tier_of = {w: t for t in TIERS for w in current[t]}
    grades = {}
    for x in csv.DictReader(open(ROOT / "reports" / "en-staged-review-sample-graded.csv", encoding="utf-8")):
        grades[x["word"]] = {"keep_cut": "keep", "est_tier": x["est_tier"], "group": "staged"}
    for line in open(ROOT / "calibration" / "en_graded.jsonl", encoding="utf-8"):
        if line.strip():
            x = json.loads(line)
            grades[x["word"]] = x          # later workbooks win
    placed = {t: [] for t in TIERS}
    quarantine, problems, held = [], [], []
    # current rows
    for w, t in tier_of.items():
        g = grades.get(w)
        if t == "expert" and g:
            if g["keep_cut"] == "cut":
                quarantine.append(w)
                continue
            if g["keep_cut"] == "unsure":
                placed["expert"].append(w)
                continue
            placed[GRADE_TO_TIER[g["est_tier"]]].append(w)
        elif t == "hard":
            placed["medium"].append(w)   # signed 2026-10-19: the 800 old letter-count Hard rows are Medium
        else:
            placed[t].append(w)
    # new rows
    new = []
    for w, g in grades.items():
        if w in tier_of or g["keep_cut"] != "keep" or g.get("est_tier") not in GRADE_TO_TIER:
            continue
        if g.get("group") == "low-band" and g["est_tier"] != "Expert":
            continue
        new.append((w, GRADE_TO_TIER[g["est_tier"]]))
    excluded = exclusions()
    from purity_admit import admit
    words = [w for w, _ in new]
    ok, rejected = admit("en", words)
    ok = set(ok)
    for w, t in sorted(new):
        if re.fullmatch(r"[a-z]{17,}", w):
            # The bank's 16-letter cap (scripts/build-wordlists.py, spec 3.3) is a
            # standing rule, so these are held out and listed, not a build failure.
            held.append(f"{w}\t{t}\tlonger than the 16-letter cap")
        elif not re.fullmatch(r"[a-z]{2,16}", w):
            problems.append(f"{w}: not a plain lowercase word of 2-16 letters")
        elif excluded(w):
            problems.append(f"{w}: on the English exclusion list")
        elif w not in ok:
            problems.append(f"{w}: rejected by PURITY ({rejected.get(w)})")
        else:
            placed[t].append(w)
    seen = {}
    for t in TIERS:
        for w in placed[t]:
            if w in seen:
                problems.append(f"{w}: in both {seen[w]} and {t} (tier partition)")
            seen[w] = t
    for t in TIERS:
        placed[t] = sorted(set(placed[t]))
    # Same per-tier ceiling as scripts/build-wordlists.py: max(840, 1.25 x pool floor).
    floors = json.loads((ROOT / "config" / "bank_floors.json").read_text())["languages"]["en"]["poolFloors"]
    for i, t in enumerate(TIERS):
        cap = max(int(floors[i] * 1.25), 840)
        if len(placed[t]) > cap:
            problems.append(f"{t}: {len(placed[t])} rows is over the {cap}-row cap (raise the en pool floor)")
    ledger_words = sorted(w for w, _ in new if w in ok and not excluded(w) and re.fullmatch(r"[a-z]{2,16}", w))
    return placed, sorted(quarantine), ledger_words, problems, sorted(held)


def render(placed, quarantine, ledger_words, held):
    files = {}
    head = "# GENERATED by tools/en_growth/build_retier.py -- do not hand-edit. Served only behind the en.retier flag.\n"
    for t in TIERS:
        files[OUT / f"{t}.txt"] = head + "\n".join(placed[t]) + "\n"
    files[OUT / "quarantine.txt"] = ("# GENERATED. Current English Expert rows cut by Eric's re-grade (2026-10-17).\n"
                                     "# Quarantined, never deleted (CC-EN-EXPERT-GROWTH I9). Still in the live bank while en.retier is OFF.\n"
                                     + "\n".join(quarantine) + "\n")
    files[OUT / "held.txt"] = ("# GENERATED. Graded words kept out of the retier bank, with the reason.\n"
                               + "".join(h + "\n" for h in held))
    files[LEDGER] = ("# CC-BANK-PURITY ledger for English re-tier rows that are not in en.tsv. word<TAB>verdict<TAB>source.\n"
                     "# Written only by tools/en_growth/build_retier.py from tools/bank/purity_admit.\n"
                     + "".join(f"{w}\tdictionary\ten_US\n" for w in ledger_words))
    rs = ["// @generated by tools/en_growth/build_retier.py -- DO NOT EDIT BY HAND.",
          "// The re-tiered English bank (CC-EN-EXPERT-GROWTH F4), served only when the",
          "// en.retier flag is ON and the profile is not Spell Jr / Kid Mode (see words::en_tier).", ""]
    for t in TIERS:
        rs.append(f"pub const EN_RETIER_{t.upper()}: &[&str] = &[" + ",".join(json.dumps(w) for w in placed[t]) + "];")
    files[RUST] = "\n".join(rs) + "\n"
    return files


def main():
    placed, quarantine, ledger_words, problems, held = build()
    if problems:
        print("build_retier: problems (nothing written):")
        for p in problems[:50]:
            print("  " + p)
        return 1
    files = render(placed, quarantine, ledger_words, held)
    if "--check" in sys.argv:
        stale = [str(p.relative_to(ROOT)) for p, body in files.items()
                 if not p.exists() or p.read_text(encoding="utf-8") != body]
        if stale:
            print("build_retier: STALE --", ", ".join(stale), "-- regenerate with python3 tools/en_growth/build_retier.py")
            return 1
        print("build_retier: OK -- retier tiers, quarantine, ledger and Rust consts current")
        return 0
    OUT.mkdir(parents=True, exist_ok=True)
    for p, body in files.items():
        p.write_text(body, encoding="utf-8")
    print("retier tiers:", {t: len(placed[t]) for t in TIERS}, "| quarantined", len(quarantine),
          "| new rows in the retier ledger", len(ledger_words), "| held out", len(held))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
