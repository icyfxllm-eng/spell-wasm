#!/usr/bin/env python3
"""Build a native-speaker review packet for one language's gloss table.

    python3 tools/audit/gloss_packet.py es --pair ja
    python3 tools/audit/gloss_packet.py ja --pair es

Writes audit/<lang>/gloss-packet.md (the brief) and audit/<lang>/gloss-rows.csv
(every row, with blank verdict columns for the auditor). It never edits the
gloss table: `audited` is a human claim (config/gloss/README.md), so this only
prepares the review. Sits beside audit_lang.py's word-bank files, which that
script writes by name and never clears.

Every column is computed from the same data the app reads: bank tiers from
assets/words/<lang>/<tier>.txt, kid safety from assets/words/kid-exclude with
the app's lenient fold (src/norm.rs, src/kid_filter.rs), and each English
meaning's tier from the English bank, which Translate resolves across all four
tiers (translate.rs, en_by_concept).
"""
import argparse
import csv
import json
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TIERS = ["easy", "medium", "hard", "expert"]

# Common English words with more than one everyday sense. A HINT list for the
# sense check, not a dictionary: a row not listed here can still be ambiguous.
POLYSEMOUS = set("""
back bank bark bat bear bill book bow box break bright case change check clear club
coat cold count cross date duck face fair fall fan file fine fire fly foot free glass
ground hand hard head key kind land last leave left letter lie light line long match
mean mind mine miss nail note novel open order palm park part pass pen pick pitch plant
play point pound present press right ring rock roll round row rule run safe saw scale
seal second set shot show side sign sink sound space spring square stamp star stick
still store story string suit swallow table tap tear tie time tip tire top train trip
trunk turn type watch wave way well wind yard
""".split())


def fold_lenient(s: str) -> str:
    """Mirror of src/norm.rs fold_lenient."""
    lowered = unicodedata.normalize("NFC", s).lower()
    stripped = "".join(c for c in unicodedata.normalize("NFD", lowered) if not ("̀" <= c <= "ͯ"))
    stripped = stripped.replace("ß", "ss").replace("æ", "ae").replace("œ", "oe")
    stripped = stripped.translate({0x142: "l", 0xF8: "o", 0x131: "i"})
    return "".join(c for c in stripped if not c.isspace())


def kid_set(lang: str) -> set:
    p = ROOT / "assets" / "words" / "kid-exclude" / f"{lang}.txt"
    if not p.exists():
        return set()
    return {fold_lenient(l.strip()) for l in p.read_text(encoding="utf-8").splitlines()
            if l.strip() and not l.strip().startswith("#")}


def kid_allowed(lang: str, word: str) -> bool:
    """Mirror of src/kid_filter.rs kid_allowed (key is the last '|' segment)."""
    excluded = kid_set(lang)
    return not excluded or fold_lenient(word.rsplit("|", 1)[-1]) not in excluded


def tier_index(lang: str) -> dict:
    out = {}
    for t in TIERS:
        p = ROOT / "assets" / "words" / lang / f"{t}.txt"
        if p.exists():
            for line in p.read_text(encoding="utf-8").splitlines():
                w = line.strip()
                if w and w not in out:
                    out[w] = t
    return out


def en_tier_index() -> dict:
    out = {}
    for t in TIERS:
        p = ROOT / "assets" / "words" / "en" / f"{t}.txt"
        for line in p.read_text(encoding="utf-8").splitlines():
            w = line.strip().split("|")[0].lower()
            if w and w not in out:
                out[w] = t
    return out


def is_short_hiragana(word: str) -> bool:
    return 0 < len(word) <= 2 and all("ぁ" <= c <= "ゟ" or c == "ー" for c in word)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("lang")
    ap.add_argument("--pair", required=True, help="the other launch language, for coverage")
    a = ap.parse_args()
    lang, pair = a.lang, a.pair

    doc = json.loads((ROOT / "config" / "gloss" / f"{lang}.json").read_text(encoding="utf-8"))
    pair_rows = json.loads((ROOT / "config" / "gloss" / f"{pair}.json").read_text(encoding="utf-8"))["rows"]
    pair_concepts = set(pair_rows.values())
    rows = sorted(doc["rows"].items())
    tiers = tier_index(lang)
    en_tiers = en_tier_index()

    records = []
    for i, (word, meaning) in enumerate(rows, 1):
        en_tier = en_tiers.get(meaning.lower(), "")
        flags = []
        if meaning.lower() in POLYSEMOUS:
            flags.append("sense-check")
        if fold_lenient(word) == fold_lenient(meaning):
            flags.append("same-spelling-as-english")
        if is_short_hiragana(word):
            flags.append("short-hiragana-homophone-risk")
        if not kid_allowed(lang, word):
            flags.append("word-kid-excluded")
        if not kid_allowed("en", meaning):
            flags.append("meaning-kid-excluded")
        if not en_tier:
            flags.append("english-meaning-not-in-bank")
        records.append({
            "row": i, "word": word, "english_meaning": meaning,
            "word_tier": tiers.get(word, "NOT IN BANK"),
            "english_meaning_tier": en_tier or "NOT IN BANK",
            "kid_safe": "yes" if kid_allowed(lang, word) and kid_allowed("en", meaning) else "no",
            f"also_in_{pair}": "yes" if meaning in pair_concepts else "no",
            "flags": " ".join(flags),
            "verdict (keep/fix/cut)": "", "fix_word": "", "fix_meaning": "", "note": "",
        })

    out = ROOT / "audit" / lang
    out.mkdir(parents=True, exist_ok=True)
    # utf-8-sig so spreadsheet apps open non-Latin text correctly.
    with (out / "gloss-rows.csv").open("w", encoding="utf-8-sig", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(records[0].keys()))
        w.writeheader()
        w.writerows(records)

    def section(title, flag, why):
        hits = [r for r in records if flag in r["flags"].split()]
        lines = [f"## {title}  ({len(hits)})", "", why, ""]
        lines += [f"- **{r['word']}** → {r['english_meaning']}  _(row {r['row']}, {r['word_tier']})_" for r in hits] or ["- none"]
        return "\n".join(lines) + "\n"

    shared = sum(1 for r in records if r[f"also_in_{pair}"] == "yes")
    not_in_bank = [r for r in records if r["word_tier"] == "NOT IN BANK"]
    by_tier = {t: sum(1 for r in records if r["word_tier"] == t) for t in TIERS}
    name = {"es": "Spanish", "ja": "Japanese"}.get(lang, lang)
    pname = {"es": "Spanish", "ja": "Japanese"}.get(pair, pair)

    md = [f"# {name} ({lang}) gloss review packet", "",
          f"**What you are checking.** Each row pairs a {name} word from the app's word bank with the",
          "English meaning it stands for. Spell Translate shows a word in another language only through",
          "that meaning, so a wrong or ambiguous meaning would show a child the wrong word. Nothing here",
          f"is live yet: {name} stays hidden in Translate until you sign the file.", "",
          f"**Rows:** {len(records)}  ·  **by tier:** " + ", ".join(f"{t} {n}" for t, n in by_tier.items())
          + f"  ·  **shared with {pname}:** {shared}", "",
          "## How to review", "",
          "Open `gloss-rows.csv` and fill the last four columns for every row:", "",
          "- **keep** — the word and meaning match, and the meaning is the sense a child would expect.",
          "- **fix** — right idea, wrong detail. Put the better word in `fix_word` and/or the better English",
          f"  meaning in `fix_meaning`. A fixed word must already be in the {name} bank, and a fixed meaning",
          "  must be a common English word; if neither is possible, choose cut.",
          "- **cut** — the row should not exist (no good match, or the word is wrong for children).",
          "- **note** — anything else worth knowing.", "",
          "The flagged sections below are where a second look matters most. They come from mechanical",
          "checks, so an unflagged row can still be wrong.", "",
          section("Sense check", "sense-check",
                  "The English meaning has more than one common sense (a ring on a finger, or a phone ringing). "
                  "Confirm the word matches the sense a child would read first. Hint list only — not exhaustive."),
          ]
    if lang == "es":
        md.append(section("Same spelling as the English meaning", "same-spelling-as-english",
                          "Confirm each is the natural Spanish word, not a loan kept only because it matched."))
    if lang == "ja":
        md.append(section("Short hiragana words", "short-hiragana-homophone-risk",
                          "The Japanese bank holds no kanji, so every word is hiragana. One- and two-kana words "
                          "often have several meanings that only kanji tell apart (あめ: rain or candy). Confirm the "
                          "English meaning is the one a child would hear first. Also note any word normally written "
                          "in katakana."))
    md.append(section("Not kid-safe", "word-kid-excluded",
                      "On the kid exclusion list; Spell Jr never shows these. Confirm, or say if they are fine."))
    md.append(section("English meaning not kid-safe", "meaning-kid-excluded",
                      "The English meaning itself is on the English kid exclusion list."))
    md += ["## Mechanical sanity checks (expect none)", "",
           section("English meaning not in the English bank", "english-meaning-not-in-bank",
                   "Translate resolves English across all four tiers, so every meaning in the English bank can "
                   "answer; gloss-check forbids a meaning outside it."),
           f"**Not in the {name} bank:** {len(not_in_bank)} (gloss-check forbids this, so expect 0).", "",
           "## Signing off", "",
           f"When the review comes back: apply the fixes to `config/gloss/{lang}.json`, run",
           "`node scripts/gloss-check.mjs`, then set `\"audited\": true` and `\"auditor\"` to the reviewer's",
           "full name. The build refuses an audited file with no named auditor.", ""]
    (out / "gloss-packet.md").write_text("\n".join(md), encoding="utf-8")
    counts = {f: sum(1 for r in records if f in r["flags"].split()) for f in
              ("sense-check", "same-spelling-as-english", "short-hiragana-homophone-risk",
               "word-kid-excluded", "meaning-kid-excluded", "english-meaning-not-in-bank")}
    print(f"audit/{lang}: {len(records)} rows, shared with {pair}: {shared}, flags: {counts}")


if __name__ == "__main__":
    main()
