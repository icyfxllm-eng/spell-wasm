#!/usr/bin/env python3
"""CC-RUSSIAN-STRESS Phase 2 — auto-annotate, then hand it to a human.

Produces the auditor's sheet: one row per Russian bank entry that needs a
stress index, carrying a MARKED STRING the auditor can eyeball in a second.
Nobody can check an integer; everybody can check `молоко́`.

THE RULE THIS FILE EXISTS TO ENFORCE (v2 Feature 2): stress belongs to the
SURFACE FORM in the bank, never to its lemma. Russian stress moves under
inflection -- рука́ becomes ру́ку, вода́ becomes во́ду, окно́ becomes о́кна,
нача́ть becomes на́чал. An annotator that resolves an inflected entry to its
lemma and reuses the lemma's index is wrong across large swathes of the bank
AND FAILS SILENTLY, because the spelling still looks correct and only a
listener notices. The spec calls this the highest-probability defect in the
whole workstream, so:

    an exact surface-form match is a VALUE.
    a lemma-only match is a FLAG, with an EMPTY index.
    no match is a FLAG, with an EMPTY index.

A guess is never written. The auditor fills the blanks; the machine only ever
offers what it can prove.

SOURCE. Not vendored, and not downloaded by this tool -- the pipeline's
"no network in builds" rule, and the same SourceMissing contract
tools/lexicon-ingest/parsers/ already uses. Fetch one and pass --source.

Run:  python3 tools/ru_stress_annotate.py --source PATH [--out sheet.csv]
      python3 tools/ru_stress_annotate.py --fixture     # exercise the machinery
"""
import argparse
import csv
import json
import os
import pathlib
import re
import sys

ROOT = pathlib.Path(os.environ.get("SPELL_ROOT",
                                  pathlib.Path(__file__).resolve().parent.parent))
VOWELS = "аеёиоуыэюя"
ACUTE = "́"

SOURCES = {
    "kaikki-ru": "https://kaikki.org/dictionary/Russian/kaikki.org-dictionary-Russian.jsonl",
    "opencorpora": "http://opencorpora.org/files/export/dict/dict.opcorpora.txt.zip",
}

# v2 Feature 2's decoy gate: deliberately mis-stressed rows, WEIGHTED TOWARD
# INFLECTED FORMS whose lemma stress differs -- the exact failure the sheet is
# meant to catch. An auditor who pattern-matches the lemma will pass the easy
# decoys and fail these.
# DECOYS ARE CORRUPTED REAL ROWS, not extra ones.
#
# The first design appended hand-picked words with wrong marks. Two of them --
# начал and город -- are real ru bank entries, so the sheet carried each twice
# with different marks: an obvious tell, and corrupt data. Worse, an auditor
# who never notices still "passes" because the duplicate looks like an export
# bug rather than a wrong answer.
#
# A decoy is now an ordinary pre-filled row whose mark has been moved to the
# wrong vowel. Indistinguishable by construction -- there is nothing to spot
# except that the stress is wrong, which is precisely the skill being tested.
DECOY_COUNT = 6

# TIERS THAT DISPLAY STRESS. v2 Feature 4: "Marked stress applies to easy and
# normal only. Hard and expert use natural unmarked speech: decoding reduced
# vowels unaided is the advanced listening skill." So the audit is 1135 words,
# not 5961 -- hard and expert keep all 4973 of their entries and play exactly
# as they do now, they simply never carry a mark.
AUDITED_TIERS = ("EASY", "MEDIUM")


def bank():
    src = (ROOT / "src/word_data.rs").read_text()
    out = []
    for tier in AUDITED_TIERS:
        m = re.search(rf"pub const RU_{tier}: &\[&str\] = &\[(.*?)\];", src, re.S)
        if not m:
            continue
        for e in re.findall(r'"([^"]+)"', m.group(1)):
            w = e.split("|")[0]
            if sum(1 for c in w if c in VOWELS) > 1:   # monosyllables take null
                out.append((w, tier.lower()))
    return out


def mark(word, i):
    """The probe's lesson, enforced here too: a mark on a consonant measures
    nothing, so it is never written."""
    if i is None or i >= len(word) or word[i] not in VOWELS:
        return None
    return word[: i + 1] + ACUTE + word[i + 1 :]


def load_source(path):
    """surface form -> stress index, EXACT SURFACE FORMS ONLY.

    kaikki.org publishes extracted Wiktionary as JSONL, one entry per line.
    Stress does not live on the headword -- it lives in `forms[].form`, and
    those are INFLECTED forms carrying their OWN stress:

        собака -> соба́ка, соба́ки
        зонт   -> зонта́, зонты́

    which is exactly what Feature 2 requires. Indexing by each form's own bare
    spelling makes surface-form matching the natural thing to do rather than a
    discipline to maintain: the lemma never enters the mapping, so a lemma's
    stress cannot leak onto an inflected entry.

    Returns (forms, lemmas, conflicts).
      forms     bare spelling -> index, where the source is unanimous
      lemmas    headword-only matches; a FLAG, never a value
      conflicts bare spelling -> {indices}, where the source disagrees with
                itself. These are stress homographs -- за́мок / замо́к -- and
                they must NOT be auto-filled: picking one silently picks a
                word. They go to the auditor blank, and they are the rows v3
                F6's senseDiscriminator exists for.
    """
    p = pathlib.Path(path)
    if not p.exists():
        print(f"  source not found: {p}")
        print("  fetch one of these and pass --source (this tool never downloads):")
        for name, url in SOURCES.items():
            print(f"    {name:<12} {url}")
        sys.exit(2)

    seen = {}          # bare -> set of indices
    lemmas = {}
    with p.open(encoding="utf-8", errors="ignore") as fh:
        for line in fh:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                d = json.loads(line)
            except Exception:
                continue
            head = d.get("word", "")
            cands = [f.get("form", "") for f in d.get("forms", []) if isinstance(f, dict)]
            cands.append(head)
            for marked in cands:
                if not marked or ACUTE not in marked:
                    continue
                bare = marked.replace(ACUTE, "")
                i = marked.index(ACUTE) - 1
                if i < 0 or i >= len(bare) or bare[i] not in VOWELS:
                    continue          # a mark that is not on a vowel is not data
                seen.setdefault(bare, set()).add(i)
            if head and ACUTE not in head:
                lemmas.setdefault(head, True)

    forms = {w: next(iter(ix)) for w, ix in seen.items() if len(ix) == 1}
    conflicts = {w: ix for w, ix in seen.items() if len(ix) > 1}
    return forms, lemmas, conflicts


def fixture_source():
    """A hand-made stand-in so the sheet machinery is testable without a 1GB
    dump. Deliberately partial: one exact form, one lemma-only, one absent."""
    return ({"молоко": 5, "рука": 3}, {"руку": True}, {})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source")
    ap.add_argument("--fixture", action="store_true")
    ap.add_argument("--out", default="build/ru-stress-sheet.csv")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    forms, lemmas, conflicts = fixture_source() if args.fixture else load_source(args.source)
    rows = bank()
    if args.limit:
        rows = rows[: args.limit]

    out = pathlib.Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    counts = {"value": 0, "lemma-flag": 0, "unmatched": 0, "conflict": 0}
    with out.open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(["word", "tier", "stressed_form", "source", "note"])
        for word, tier in rows:
            if word in conflicts:
                # The source disagrees with itself: two stresses, two words.
                # Auto-filling would silently choose one. Blank, flagged.
                w.writerow([word, tier, "", "stress-homograph",
                            "source gives more than one stress for this spelling"])
                counts["conflict"] += 1
                continue
            if word in forms:
                m = mark(word, forms[word])
                if m:
                    w.writerow([word, tier, m, "exact", ""])
                    counts["value"] += 1
                    continue
            if word in lemmas:
                # A lemma match is a FLAG. The index is left EMPTY on purpose:
                # writing the lemma's stress here is the silent defect.
                w.writerow([word, tier, "", "lemma-only",
                            "stress moves under inflection — do not copy the lemma"])
                counts["lemma-flag"] += 1
                continue
            w.writerow([word, tier, "", "", ""])
            counts["unmatched"] += 1

    # Corrupt real pre-filled rows, spread across the sheet.
    body = list(csv.reader(out.open(encoding="utf-8")))
    header, data = body[0], body[1:]
    # Prefer rows whose stress is NOT on the first vowel: those are where a
    # lemma-copying auditor goes wrong, which is the failure worth testing.
    cands = []
    for n, r in enumerate(data):
        word, _tier, marked, src = r[0], r[1], r[2], r[3]
        if src != "exact" or ACUTE not in marked:
            continue
        i = marked.index(ACUTE) - 1
        vowels = [k for k, c in enumerate(word) if c in VOWELS]
        if len(vowels) > 1 and i != vowels[0]:
            cands.append((n, word, marked, i, vowels))
    step = max(1, len(cands) // (DECOY_COUNT + 1))
    planted = []
    for k in range(DECOY_COUNT):
        pick = k * step
        if pick >= len(cands):
            break
        n, word, marked, i, vowels = cands[pick]
        wrong_i = next((v for v in vowels if v != i), None)
        if wrong_i is None:
            continue
        bad = mark(word, wrong_i)
        if not bad or bad == marked:
            continue
        data[n][2] = bad          # the row now carries a WRONG mark
        planted.append((word, bad, marked, "stress moved off its real vowel"))
    with out.open("w", newline="", encoding="utf-8") as fh:
        wr = csv.writer(fh)
        wr.writerow(header)
        wr.writerows(data)
    key = out.with_name(out.stem + "-DECOY-KEY.csv")
    with key.open("w", newline="", encoding="utf-8") as kh:
        kw = csv.writer(kh)
        kw.writerow(["word", "planted_wrong", "correct", "why"])
        kw.writerows(planted)
    print(f"  DECOY KEY -> {key}   (never send this to the auditor)")

    total = sum(counts.values())
    print(f"  wrote {out}  ({total} rows + {len(planted)} decoys, scattered and unlabelled)")
    print(f"    exact surface match (a VALUE) : {counts['value']}")
    print(f"    stress-homograph (FLAG, empty): {counts['conflict']}")
    print(f"    lemma-only (FLAG, empty)      : {counts['lemma-flag']}")
    print(f"    unmatched  (FLAG, empty)      : {counts['unmatched']}")
    print(f"    -> the auditor fills {counts['lemma-flag'] + counts['unmatched'] + counts['conflict']} blanks")
    print(f"    -> and verifies {counts['value']} the machine could prove")
    print("  decoys are weighted toward inflected forms whose LEMMA stress differs;")
    print("  more than one miss rejects the sheet (v2 Feature 2).")
    print("  easy+medium only — hard and expert are unmarked by design (Feature 4),")
    print("  so their 4973 entries need no stress data and are not in this sheet.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
