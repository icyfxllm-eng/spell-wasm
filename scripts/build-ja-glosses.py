#!/usr/bin/env python3
"""Japanese definitions from JMdict, keyed by READING.

Japanese had the worst definition coverage of any shipped language: 941 of
4,979 bank words, 81% with nothing to show. en.wiktionary simply has no entry
for most of them, so no amount of refetching helps -- the source is the
problem. This is the same answer Chinese already got in build-zh-glosses.py: a
bundled dictionary, filtered to the words the app serves.

KEYED BY READING, which is the whole difficulty. The ja bank is 100% hiragana
-- the player spells kana, never kanji -- while JMdict is organised around
headwords that are usually kanji. So the lookup goes through <r_ele><reb>, and
a kana reading is famously ambiguous: あいこう is 愛好, 愛校 and more.

That ambiguity costs less here than it looks. The bank word IS the kana, so
whichever sense the hint carries, the spelling the player must produce is
identical -- ambiguity costs hint precision, not answer correctness. Where it
does bite is definition-match, and build-def-pools.py already handles it:
identical definitions inside one tier lose prompt_grade and exclude each other.

To spend that budget well this prefers entries JMdict marks COMMON (ichi1,
news1, spec1, gai1, nf01-nf48) over whichever entry happens to parse first.

Parsed with regex, not ElementTree: JMdict declares its part-of-speech tags as
DTD entities (<pos>&n;</pos>), and stdlib XML parsing fails on them.

JMdict is (C) the Electronic Dictionary Research and Development Group,
CC BY-SA 4.0. Recorded in NOTICES.md, which ships to players as notices.html.

Run after changing the ja bank:
    python3 scripts/build-ja-glosses.py [path-to-JMdict_e.gz]
(downloads to .corpus-cache/ if no path given)
"""
import gzip, html, json, os, re, sys, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "backend", "ja_glosses.json")
JMDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"

RE_ENTRY = re.compile(r"<entry>(.*?)</entry>", re.S)
RE_RELE = re.compile(r"<r_ele>(.*?)</r_ele>", re.S)
RE_REB = re.compile(r"<reb>(.*?)</reb>", re.S)
RE_PRI = re.compile(r"<re_pri>(.*?)</re_pri>", re.S)
RE_SENSE = re.compile(r"<sense>(.*?)</sense>", re.S)
# A gloss with no g_type is the plain translation. g_type="expl" is a long
# explanatory note ("polite set phrase used when meeting or parting from
# someone") -- true, and not what a hint card wants first.
RE_PLAIN_GLOSS = re.compile(r"<gloss>(.*?)</gloss>", re.S)
RE_POS = re.compile(r"<pos>&([a-z0-9-]+);</pos>")

COMMON = {"ichi1", "news1", "spec1", "gai1"}
RE_NF = re.compile(r"^nf(\d+)$")

# A reading whose only sense is a pointer is not a definition, the same rule
# build-def-pools.py applies to Wiktionary.
RE_XREF_ONLY = re.compile(
    r"^\(?(see|cf\.?|abbr\.? of|abbreviation of|short for|archaic|obsolete)\b", re.I)


def bank_readings():
    src = open(os.path.join(ROOT, "src", "word_data.rs"), encoding="utf-8").read()
    out = set()
    for tier in ("EASY", "MEDIUM", "HARD", "EXPERT"):
        m = re.search(rf'pub const JA_{tier}: &\[&str\] = &\[(.*?)\];', src, re.S)
        if m:
            out.update(re.findall(r'"([^"]+)"', m.group(1)))
    return out


def jmdict_text(path):
    if path:
        return gzip.open(path, "rt", encoding="utf-8") if path.endswith(".gz") \
            else open(path, encoding="utf-8")
    cache = os.path.join(ROOT, ".corpus-cache")
    os.makedirs(cache, exist_ok=True)
    gz = os.path.join(cache, "JMdict_e.gz")
    if not os.path.exists(gz):
        print("  fetching JMdict_e …")
        urllib.request.urlretrieve(JMDICT_URL, gz)
    return gzip.open(gz, "rt", encoding="utf-8")


def priority(pris):
    """How strongly JMdict marks this reading as common. Higher wins."""
    score = 0
    for p in pris:
        if p in COMMON:
            score += 10
        m = RE_NF.match(p)
        if m:
            # nf01 is the most frequent band, nf48 the least.
            score += max(0, 50 - int(m.group(1)))
    return score


def definition_of(body):
    """Up to three plain glosses from the FIRST sense that has any."""
    for sense in RE_SENSE.findall(body):
        glosses = [html.unescape(g).strip() for g in RE_PLAIN_GLOSS.findall(sense)]
        glosses = [g for g in glosses if g and not RE_XREF_ONLY.match(g)]
        if glosses:
            pos = RE_POS.search(sense)
            return "; ".join(glosses[:3]), (pos.group(1) if pos else "")
    return "", ""


def main():
    want = bank_readings()
    if not want:
        raise SystemExit("build-ja-glosses: no JA_* consts in src/word_data.rs")
    print(f"  ja bank: {len(want)} readings")

    best = {}      # reading -> (score, definition, pos)
    rival = {}     # reading -> set of distinct definitions seen at top score
    src = sys.argv[1] if len(sys.argv) > 1 else None
    with jmdict_text(src) as fh:
        blob = fh.read()
    for body in RE_ENTRY.findall(blob):
        definition, pos = definition_of(body)
        if not definition:
            continue
        for rele in RE_RELE.findall(body):
            m = RE_REB.search(rele)
            if not m:
                continue
            reb = m.group(1)
            if reb not in want:
                continue
            score = priority(RE_PRI.findall(rele))
            prev = best.get(reb)
            if prev is None or score > prev[0]:
                best[reb] = (score, definition, pos)
                rival[reb] = {definition}
            elif score == prev[0]:
                rival[reb].add(definition)

    glosses = {
        r: {
            "definition": d,
            "pos": p,
            # Several entries tie at the top score with different meanings, so
            # the one chosen is arbitrary among them. Recorded rather than
            # hidden: a consumer can decline to use it as a quiz prompt.
            "ambiguous": len(rival[r]) > 1,
        }
        for r, (s, d, p) in best.items()
    }
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(glosses, f, ensure_ascii=False, indent=0, sort_keys=True)
    amb = sum(1 for v in glosses.values() if v["ambiguous"])
    missing = sorted(want - set(glosses))
    print(f"  wrote {len(glosses)}/{len(want)} glosses -> {os.path.relpath(OUT, ROOT)}")
    print(f"  {amb} of them tie between senses and are marked ambiguous")
    if missing:
        print(f"  no JMdict entry ({len(missing)}): {' '.join(missing[:15])}"
              f"{' …' if len(missing) > 15 else ''}")


if __name__ == "__main__":
    main()
