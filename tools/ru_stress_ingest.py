#!/usr/bin/env python3
"""CC-RUSSIAN-STRESS Phase 3 — the only write path for stress data.

Takes the auditor's filled sheet, converts `stressed_form` back to a
`stress_index`, and emits the generated table. Nothing else in the project may
write stress data, exactly as with definitions: one path, auditable, gated.

THE SHEET IS NOT TRUSTED. It comes back from a human who was paid per row, and
the failure that matters is not malice but a lazy pass -- bulk-filling a column
with the lemma's stress, which is correct often enough to look right and wrong
exactly where Russian is hard. Four gates stand between a returned sheet and a
written table, and ANY of them rejects the whole sheet rather than dropping a
row:

  AGREEMENT   strip_stress(stressed_form) must equal the answer key, byte for
              byte. A sheet that quietly edits a word -- a typo, an autocorrect,
              a stray space from a spreadsheet -- would otherwise write stress
              data keyed to a word the bank does not contain, and I1's promise
              that no diacritic reaches grading rests on the key being untouched.

  WELL-FORMED exactly one U+0301, and it sits on a vowel. A mark on a consonant
              is the defect this project has now produced three times by hand.

  Ё           an entry containing ё must be indexed on the ё. It is inherently
              stressed; an index elsewhere is a data error, not a variant.

  DECOY       six pre-filled rows carry a deliberately WRONG mark. The auditor
              is expected to move them. More than one left uncorrected rejects
              the sheet and gates payment (v2 Feature 2). One miss is tolerated
              because anyone can blink; two is a pattern.

A rejected sheet writes NOTHING. There is no partial ingest, because a
half-written table is worse than no table: I5 excludes uncovered entries from
selection, so an absent row degrades safely while a wrong row does not.

Run:  python3 tools/ru_stress_ingest.py --sheet S.csv --key K.csv [--write]
"""
import argparse
import csv
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VOWELS = "аеёиоуыэюя"
ACUTE = "́"
OUT_RS = ROOT / "src/ru_stress_data.rs"
AUDIT_CLAIM = ROOT / "config/ru-stress-audit.json"
DECOY_TOLERANCE = 1          # one blink; two is a pattern


def index_of(marked):
    """stressed_form -> (bare, index) or (bare, None) with a reason."""
    if marked.count(ACUTE) != 1:
        return None, f"expected exactly one stress mark, found {marked.count(ACUTE)}"
    bare = marked.replace(ACUTE, "")
    i = marked.index(ACUTE) - 1
    if i < 0 or i >= len(bare):
        return bare, "the mark is not after a letter"
    if bare[i] not in VOWELS:
        return bare, f"the mark sits on {bare[i]!r}, which is not a vowel"
    if "ё" in bare and bare[i] != "ё":
        return bare, "ё is inherently stressed; the index must point at it"
    return bare, i


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sheet", required=True)
    ap.add_argument("--key", required=True)
    ap.add_argument("--write", action="store_true", help="emit the table (only if every gate passes)")
    args = ap.parse_args()

    rows = list(csv.DictReader(open(args.sheet, encoding="utf-8")))
    key = {r["word"]: r for r in csv.DictReader(open(args.key, encoding="utf-8"))}

    accepted, blank, errors = {}, 0, []
    for r in rows:
        word, marked = r["word"], (r.get("stressed_form") or "").strip()
        if not marked:
            blank += 1
            continue
        bare, res = index_of(marked)
        if bare != word:                       # AGREEMENT
            errors.append(f"{word!r}: stressed_form strips to {bare!r} — the answer key was edited")
            continue
        if not isinstance(res, int):           # WELL-FORMED / Ё
            errors.append(f"{word!r}: {res}")
            continue
        accepted[word] = res

    # DECOY GATE — run before anything is written.
    missed, caught = [], []
    for w, k in key.items():
        submitted = next((r["stressed_form"].strip() for r in rows if r["word"] == w), "")
        if submitted == k["planted_wrong"]:
            missed.append(f"{w}: still marked {k['planted_wrong']} — correct is {k['correct']}")
        elif submitted == k["correct"]:
            caught.append(w)

    print(f"  sheet: {len(rows)} rows")
    print(f"    accepted        {len(accepted)}")
    print(f"    left blank      {blank}")
    print(f"    malformed       {len(errors)}")
    print(f"  decoys: {len(caught)}/{len(key)} corrected, {len(missed)} missed "
          f"(tolerance {DECOY_TOLERANCE})")
    for m in missed:
        print(f"      MISSED {m}")
    for e in errors[:10]:
        print(f"      ERROR  {e}")
    if len(errors) > 10:
        print(f"      ... and {len(errors) - 10} more")

    reject = []
    if errors:
        reject.append(f"{len(errors)} malformed row(s)")
    if len(missed) > DECOY_TOLERANCE:
        reject.append(f"{len(missed)} decoys missed (>{DECOY_TOLERANCE})")
    if reject:
        print(f"\n  SHEET REJECTED: {'; '.join(reject)}")
        print("  Nothing written. A partial ingest is worse than none: an absent row")
        print("  is excluded from selection by I5, a wrong row is not.")
        return 1

    print("\n  SHEET ACCEPTED")
    if not args.write:
        print("  (dry run — pass --write to emit the table)")
        return 0

    # THE INGEST DOES NOT CERTIFY ITS OWN OUTPUT. It reads the human claim and
    # copies it; it never writes that file. Four passing gates mean the sheet is
    # well-formed and the auditor was awake -- not that the answers are right.
    claim = json.loads(AUDIT_CLAIM.read_text(encoding="utf-8"))
    audited = bool(claim.get("audited"))
    print(f"  audit claim: audited={audited} by={claim.get('by')!r} ({AUDIT_CLAIM.name})")
    if not audited:
        print("  -> table ships DARK: stress_index returns None until a human signs off.")

    items = sorted(accepted.items())
    body = "\n".join(f'    ("{w}", {i}),' for w, i in items)
    OUT_RS.write_text(f'''// GENERATED by tools/ru_stress_ingest.py -- do not hand-edit.
// CC-RUSSIAN-STRESS Phase 3: the audited stress table.
//
// {len(items)} entries, sorted, so lookup is a binary search. The index is a
// CHARACTER index into the answer key, never a byte offset -- every Cyrillic
// letter is two bytes and a byte-indexed reader would mark the wrong vowel.
//
// Only the ingest writes this file, and only after the agreement, well-formed,
// ё and decoy gates all pass on the whole sheet.
/// The human audit claim, copied from config/ru-stress-audit.json. The ingest
/// reads it and never writes it. While false, `stress_index` returns None for
/// every word and I5 excludes those entries -- the data is present, inert, and
/// diffable, which is what makes the eventual audit reviewable.
pub const AUDITED: bool = {str(audited).lower()};

pub const RU_STRESS: [(&str, u8); {len(items)}] = [
{body}
];
''', encoding="utf-8")
    print(f"  wrote {OUT_RS} ({len(items)} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
