#!/usr/bin/env python3
"""CC-LAUNCH-EN D-SRC4 / Gig A -- the rating sheet for GENERATED English text.

    python3 tools/defs/gen_sheet.py build/defgen/<run>   [--out build/defs-audit]

One row per served word (word + tier), with its generated Definition and
Sentence side by side and a SEPARATE severity column for each (rev 3: "separate
Definition and Sentence severity columns, decoys, flag-don't-rewrite").

Raters flag, never rewrite (D-DEF7): there is no fix column, only `note`.
A dark field (no text survived generation) has an empty text cell and is left
unrated.

LOCKED SHEET (after tools/defs/en_sheet.py): a stamp covers every column the
rater must not change. DECOY rows -- invented texts on real words that are not
in the bank -- are spread one per 150-row chunk, at least MIN_SEVERE of them
severe (any careful rater gives 2+). The key goes to keys/ (gitignored), never
to the rater. Kid-visible rows come first.

Severity, per field:
  0  fine for a child, and the text fits the word
  1  not for a child (violence, death, injury, drugs or alcohol, crime, bodily or
     medical detail, frightening, adult themes, stated plainly), OR the text is
     wrong for the word, misleading, or confusing
  2  sexual reference, slur or hateful framing, graphic violence, or self-harm
  3  explicit sexual content, a slur used as one, or anything unfit for any player
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import pathlib
import random
import re
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
CHUNK = 150
MIN_SEVERE = 5
LOCKED = ["row", "chunk", "word", "tier", "kid_visible", "meaning", "definition", "sentence"]
HEADER = LOCKED + ["screen", "definition_severity", "sentence_severity", "note"]
SCREEN = re.compile(r"\b(sex\w*|naked|nude|kill\w*|murder\w*|dead|death|die[ds]?|dying|blood\w*|gun\w*|weapon\w*|"
                    r"bomb\w*|war|drug\w*|alcohol\w*|drunk\w*|beer|wine|slave\w*|execut\w*|attack\w*|army|soldier\w*|"
                    r"diseas\w*|cancer|hell|damn\w*)\b", re.IGNORECASE)
# Decoy words are real, common, child-safe words that are NOT in the served bank,
# so no word appears twice. gen_sheet refuses one that is in the bank.
DECOY_WORDS = ["kettle", "acorn", "puddle", "mitten", "pebble", "walrus", "lantern", "teapot", "crayon", "otter",
               "meadow", "biscuit", "pinecone", "saddle", "barnacle", "sprinkle", "thimble", "porridge", "hammock",
               "snorkel", "wheelbarrow", "dandelion", "gumdrop", "ladle", "marmalade", "nectar", "parsnip", "quilt"]
# (field, bad text, the other field's benign text, severe?)
DECOYS_MILD = [
    ("definition", "A strong drink that makes grown-ups drunk.", "We left a {w} by the back door."),
    ("sentence", "He hit his brother until he cried and then hit him again.", "A small thing you find outside."),
    ("definition", "A tool soldiers use to shoot people in a war.", "My {w} was sitting on the shelf."),
    ("sentence", "The robber stole the old man's money and ran away.", "A thing you might see at a farm."),
    ("definition", "A pill people swallow to get high at parties.", "Grandma showed me a {w} today."),
    ("sentence", "The scary man followed the girl home in the dark.", "Something small and round."),
    ("definition", "When a pet dies and is buried in the cold ground.", "There is a {w} in this picture."),
    ("sentence", "Dad smoked a cigarette and drank beer on the porch.", "Something you can hold in your hand."),
]
DECOYS_SEVERE = [
    ("sentence", "Blood sprayed everywhere when he cut the man's throat.", "A thing children often like."),
    ("definition", "To touch someone's private parts without asking them.", "We drew a {w} at school."),
    ("sentence", "The men beat him until his bones broke and he stopped moving.", "Something you might find outdoors."),
    ("definition", "A rude word used to insult people because of their skin colour.", "I saw a {w} on our walk."),
    ("sentence", "She showed the boy pictures of naked people on her phone.", "A thing from a storybook."),
    ("definition", "Hurting yourself on purpose with something sharp.", "Our class has a {w} too."),
]


def lines(p):
    with open(p, encoding="utf-8") as f:
        return [l.strip() for l in f if l.strip() and not l.startswith("#")]


def stamp(sheet_id, rows):
    h = hashlib.sha256(sheet_id.encode())
    for r in rows:
        h.update(("\n" + "\t".join(str(r[c]) for c in LOCKED)).encode())
    return h.hexdigest()


def source_rows(run_dir: pathlib.Path) -> list[dict]:
    """One dict per (word, tier) from the run's rows.csv."""
    by = {}
    with open(run_dir / "rows.csv", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            d = by.setdefault((r["word"], r["tier"]), {"word": r["word"], "tier": r["tier"],
                                                       "meaning": r["meaning_used"], "definition": "", "sentence": ""})
            if r["status"] == "ok":
                d[r["field"]] = r["text"]
    return list(by.values())


def build(run_dir: pathlib.Path, out_root: pathlib.Path, keys_dir: pathlib.Path, seed: int = 20261109,
          sheet_id: str | None = None):
    excl = set(lines(ROOT / "assets" / "words" / "kid-exclude" / "en.txt"))
    src = source_rows(run_dir)
    bank = {r["word"] for r in src}
    rows = [dict(r, kid_visible="yes" if r["word"] not in excl else "no", decoy=None) for r in src]
    rng = random.Random(seed)
    rng.shuffle(rows)
    rows.sort(key=lambda r: r["kid_visible"] != "yes")   # stable: kid-visible first, shuffled within

    n_chunks = -(-len(rows) // CHUNK)
    pool_words = [w for w in DECOY_WORDS if w not in bank]
    n_severe = max(MIN_SEVERE, round(n_chunks * 0.25))
    specs = [s + (True,) for s in DECOYS_SEVERE][:n_severe]
    while len(specs) < n_chunks:
        specs.append(DECOYS_MILD[(len(specs) - n_severe) % len(DECOYS_MILD)] + (False,))
    if len(pool_words) < len(specs):
        raise SystemExit(f"need {len(specs)} decoy words not in the bank, have {len(pool_words)}")
    rng.shuffle(specs)
    words = rng.sample(pool_words, len(specs))
    # One decoy into each chunk at a random offset.
    for k in reversed(range(len(specs))):
        field, bad, other, severe = specs[k]
        w = words[k]
        d = {"word": w, "tier": rng.choice(["easy", "medium"]), "kid_visible": "yes", "meaning": "",
             "definition": "", "sentence": "", "decoy": {"field": field, "severe": severe}}
        d[field] = bad
        d["sentence" if field == "definition" else "definition"] = other.format(w=w)
        pos = min(len(rows), k * CHUNK + rng.randrange(CHUNK))
        rows.insert(pos, d)
    for i, r in enumerate(rows, 1):
        r["row"] = i
        r["chunk"] = (i - 1) // CHUNK + 1
        r["screen"] = ",".join(sorted({m.group(0).lower() for m in SCREEN.finditer(r["definition"] + " " + r["sentence"])}))

    sheet_id = sheet_id or f"en-gen-{time.strftime('%Y%m%d')}"
    st = stamp(sheet_id, rows)
    out = out_root / sheet_id
    out.mkdir(parents=True, exist_ok=True)
    with open(out / "sheet.csv", "w", newline="", encoding="utf-8") as f:
        f.write(f"# sheet {sheet_id} stamp {st}\n")
        w = csv.writer(f)
        w.writerow(HEADER)
        for r in rows:
            w.writerow([r[c] for c in LOCKED] + [r["screen"], "", "", ""])
    keys_dir.mkdir(exist_ok=True)
    (keys_dir / f"{sheet_id}-DECOY-KEY.json").write_text(json.dumps(
        {"sheet": sheet_id, "stamp": st,
         "decoys": [{"row": r["row"], "chunk": r["chunk"], **r["decoy"]} for r in rows if r["decoy"]]}, indent=1))
    real = [r for r in rows if not r["decoy"]]
    meta = {"sheet": sheet_id, "stamp": st, "run": str(run_dir.resolve()), "rows": len(rows), "sourceRows": len(real),
            "chunks": n_chunks, "decoys": len(specs), "severeDecoys": sum(s[3] for s in specs),
            "kidVisibleRows": sum(r["kid_visible"] == "yes" for r in real),
            "fieldsToRate": sum(bool(r["definition"]) + bool(r["sentence"]) for r in rows),
            "darkFields": sum((not r["definition"]) + (not r["sentence"]) for r in real),
            "screenHits": sum(bool(r["screen"]) for r in real)}
    (out / "meta.json").write_text(json.dumps(meta, indent=1))
    return out, meta


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir")
    ap.add_argument("--out", default=str(ROOT / "build" / "defs-audit"))
    a = ap.parse_args()
    out, meta = build(pathlib.Path(a.run_dir), pathlib.Path(a.out), ROOT / "keys")
    print(json.dumps(meta, indent=1))
    print(out / "sheet.csv")


if __name__ == "__main__":
    main()
