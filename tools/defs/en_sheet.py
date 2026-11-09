#!/usr/bin/env python3
"""CC-LAUNCH-EN F9 / A9.1 -- the English definitions locked sheet (export).

    python3 tools/defs/en_sheet.py [--server ~/spellgame-server] [--out build/defs-audit]

Every English definition that can render, one row per text a player can see:
  defpool   backend/def_pools/en.json as SERVED (the server copy, not the repo
            copy): Definition Match prompts and distractors.
  meaning   the server's cached dictionaryapi.dev lookup (cache/meaning_v1_<md5>.json)
            for every word a player can be served: its definition (play hint,
            result card, Spelldoku, Spell Search) and its example (the Sentence
            button). A served word with NO cache entry is fetched live from
            dictionaryapi.dev the first time anyone asks; it cannot be reviewed
            here and is listed in uncached.txt instead of the sheet.

kid_visible is conservative: Definition Match rows that are kid_register and
pass the kid word list (what defmatch_screen.rs serves to Kid Mode), and the
meaning of every word left in any legacy tier after the kid filter (Spell Jr's
play pool), since the Definition and Sentence buttons are not kid-gated.

THE SHEET IS A LOCKED SHEET (after tools/human-audio/sheet.py): a stamp over
everything the reviewer must not change, and DECOY rows -- invented definitions
on real words that any careful reviewer rates above 0 -- whose key goes to
keys/ (gitignored), never to the reviewer. Rows are sorted kid-visible first.

Severity (the reviewer fills `severity`, 0-3; `fix` is optional replacement text):
  0  fine for a child
  1  not for a child: violence, death, injury, drugs or alcohol, crime, bodily
     or medical detail, frightening, or adult themes, stated plainly
  2  sexual reference, slur or hateful framing, graphic violence, or self-harm
  3  explicit sexual content, a slur used as one, or anything unfit for any player
Kid Mode is zero-tolerance: any kid_visible row above 0 removes the word from
Kid Mode at ingest (tools/defs/en_ingest.py).
"""
import argparse
import csv
import hashlib
import json
import os
import pathlib
import random
import re
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
TIERS = ["easy", "medium", "hard", "expert"]
DECOY_COUNT = 8
# Mechanical pre-screen, shown to the reviewer as a hint only. It never rates.
SCREEN = re.compile(r"\b(sex\w*|genital\w*|penis|vagina|breast\w*|naked|nude|rape\w*|kill\w*|murder\w*|dead|death|die[ds]?|dying|"
                    r"suicide|blood\w*|wound\w*|corpse|gun\w*|weapon\w*|bomb\w*|war|drug\w*|alcohol\w*|drunk\w*|beer|wine|"
                    r"cocaine|heroin|tobacco|cigar\w*|slave\w*|whip\w*|torture\w*|execut\w*|prostitut\w*|feces|urin\w*|"
                    r"excrement|vomit\w*|diseas\w*|cancer|abort\w*|pregnan\w*|hell|damn\w*|slur|derogatory|offensive|vulgar)\b",
                    re.IGNORECASE)
# Invented definitions on real words. Plainly not for a child, phrased like a dictionary.
DECOY_TEXTS = [
    "A strong alcoholic drink made by distilling fermented grain.",
    "To kill someone deliberately and unlawfully.",
    "A weapon that fires bullets from a metal tube.",
    "An illegal drug that is smoked or injected for its effects.",
    "The act of torturing a prisoner to obtain information.",
    "A place where people are held as slaves and forced to work.",
    "The dead body of a person, especially one that has been murdered.",
    "To injure someone so badly that they bleed heavily.",
]


def lines(p):
    return [l.strip() for l in open(p, encoding="utf-8") if l.strip() and not l.startswith("#")]


def kid_excluded():
    return set(lines(ROOT / "assets" / "words" / "kid-exclude" / "en.txt"))


def meaning(cache, word):
    p = cache / f"meaning_v1_{hashlib.md5(word.encode()).hexdigest()}.json"
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except ValueError:
        return None


def stamp(sheet_id, rows):
    h = hashlib.sha256(sheet_id.encode())
    for r in rows:
        h.update(f"\n{r['row']}\t{r['word']}\t{r['source']}\t{r['field']}\t{r['kid_visible']}\t{r['text']}".encode())
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--server", default=os.path.expanduser("~/spellgame-server"))
    ap.add_argument("--out", default=str(ROOT / "build" / "defs-audit"))
    a = ap.parse_args()
    server = pathlib.Path(a.server)
    cache = server / "cache"
    pool_path = server / "backend" / "def_pools" / "en.json"
    pool = json.loads(pool_path.read_text(encoding="utf-8"))
    excl = kid_excluded()

    banks = {t: lines(ROOT / "assets" / "words" / "en" / f"{t}.txt") for t in TIERS}
    served = sorted({w for ws in banks.values() for w in ws})
    kid_words = {w for ws in banks.values() for w in ws if w not in excl}

    rows, uncached = [], []
    for tier in TIERS:
        for r in pool["tiers"].get(tier, []):
            kid = bool(r.get("kid_register")) and r["word"] not in excl
            rows.append({"word": r["word"], "source": f"defpool/{tier}", "field": "definition",
                         "kid_visible": "yes" if kid else "no", "text": r["definition"]})
    for w in served:
        m = meaning(cache, w)
        if m is None:
            uncached.append(w)
            continue
        for field in ("definition", "example"):
            text = (m.get(field) or "").strip()
            if text:
                rows.append({"word": w, "source": "meaning", "field": field,
                             "kid_visible": "yes" if w in kid_words else "no", "text": text})

    rng = random.Random(20261108)
    decoy_words = rng.sample(sorted(kid_words), DECOY_COUNT)
    decoys = [{"word": w, "source": "meaning", "field": "definition", "kid_visible": "yes", "text": t, "decoy": True}
              for w, t in zip(decoy_words, DECOY_TEXTS)]
    rows += decoys
    rng.shuffle(rows)
    rows.sort(key=lambda r: r["kid_visible"] != "yes")   # stable: kid-visible first, shuffled within
    for i, r in enumerate(rows, 1):
        r["row"] = i
        r["screen"] = ",".join(sorted({m.group(0).lower() for m in SCREEN.finditer(r["text"])}))

    sheet_id = f"en-defs-{time.strftime('%Y%m%d')}"
    st = stamp(sheet_id, rows)
    out = pathlib.Path(a.out) / sheet_id
    out.mkdir(parents=True, exist_ok=True)
    with open(out / "sheet.csv", "w", newline="", encoding="utf-8") as f:
        f.write(f"# sheet {sheet_id} stamp {st}\n")
        w = csv.writer(f)
        w.writerow(["row", "word", "source", "field", "kid_visible", "text", "screen", "severity", "fix", "note"])
        for r in rows:
            w.writerow([r["row"], r["word"], r["source"], r["field"], r["kid_visible"], r["text"], r["screen"], "", "", ""])
    (out / "uncached.txt").write_text(
        "# Served English words with no cached definition: fetched live from dictionaryapi.dev on first use, unreviewed.\n"
        "# kid = also in Spell Jr's play pool.\n"
        + "".join(f"{w}\t{'kid' if w in kid_words else ''}\n" for w in uncached), encoding="utf-8")
    keys = ROOT / "keys"
    keys.mkdir(exist_ok=True)
    (keys / f"{sheet_id}-DECOY-KEY.json").write_text(json.dumps(
        {"sheet": sheet_id, "stamp": st, "decoyRows": sorted(r["row"] for r in rows if r.get("decoy"))}, indent=1))
    meta = {"sheet": sheet_id, "stamp": st, "defpool": str(pool_path),
            "defpoolSha256": hashlib.sha256(pool_path.read_bytes()).hexdigest(),
            "rows": len(rows), "kidVisibleRows": sum(r["kid_visible"] == "yes" for r in rows),
            "screenHits": sum(bool(r["screen"]) for r in rows), "decoys": DECOY_COUNT,
            "servedWords": len(served), "uncachedWords": len(uncached),
            "uncachedKidWords": sum(w in kid_words for w in uncached)}
    (out / "meta.json").write_text(json.dumps(meta, indent=1))
    print(json.dumps(meta, indent=1))


if __name__ == "__main__":
    main()
