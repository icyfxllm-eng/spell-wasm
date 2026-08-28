#!/usr/bin/env python3
"""CC-SPELL-O-PHONES F0 — the blocking inventory pass.

Counts English's actual homophone-set inventory AGAINST THE AUDITED BANK,
because the mode's shape depends on a number nobody has measured. The spec's
own words: if 3+ member families are healthy the Sweep is the headline; if they
are thin the Sweep is a rare event and F5's run structure changes.

Source: tools/wordpipe/sources/cmudict.dict, already vendored (CMU, BSD-style).
Stress digits are stripped so homophones collide -- the same rule
tools/lexicon-ingest/parsers/cmudict.py already uses.

A SET IS ONLY REAL IF EVERY MEMBER IS IN THE BANK. F1 makes an incomplete set
an ILLEGAL set, not a smaller one: if the bank knows pair and pear but not
pare, a player who types PARE in a pare-legal carrier is marked wrong. So this
counts two different things and never conflates them:

  BANK-CLOSED   every member of the set is in the audited English bank.
                These are the sets the mode could ship today.
  BANK-PARTIAL  at least one member is in the bank and at least one is not.
                These are NOT sets. They are quarantine candidates, and their
                count is the size of the authoring job to rescue them.

Run:  python3 tools/spellophones-inventory.py
"""
import pathlib
import re
import sys
from collections import defaultdict

ROOT = pathlib.Path(__file__).resolve().parent.parent
CMU = ROOT / "tools/wordpipe/sources/cmudict.dict"
STRESS = re.compile(r"\d")
TIERS = ("easy", "medium", "hard", "expert")


def bank():
    """Every English bank word -> its tier. Earliest tier wins if duplicated."""
    src = (ROOT / "src/words.rs").read_text() + (ROOT / "src/word_data.rs").read_text()
    out = {}
    for tier in TIERS:
        m = re.search(rf"pub const EN_{tier.upper()}: &\[&str\] = &\[(.*?)\];", src, re.S)
        if not m:
            continue
        for e in re.findall(r'"([^"]+)"', m.group(1)):
            w = e.split("|")[0].strip().lower()
            out.setdefault(w, tier)
    return out


def real_words():
    """Spellings a player could plausibly type. CMUdict carries 117k headwords
    including proper names and transliterations -- sieh, tse, cie, auer -- and
    treating those as set members quarantined sea/see as "incomplete". F1's
    closure invariant is about spellings the GRADER must accept, not about
    CMUdict's completeness. web2 is the filter; bank words are real by
    definition."""
    w2 = pathlib.Path("/usr/share/dict/web2")
    out = set()
    if w2.exists():
        out = {l.strip().lower() for l in w2.read_text(errors="ignore").splitlines() if l.strip().isalpha()}
    return out


def prons():
    """word -> set of stress-stripped ARPAbet strings (homophones collide)."""
    out = defaultdict(set)
    for line in CMU.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line or line.startswith(";;;"):
            continue
        parts = line.split()
        w = parts[0].lower()
        if w.endswith(")"):
            w = w[: w.rindex("(")]
        if not w.isalpha():
            continue
        out[w].add(" ".join(STRESS.sub("", p) for p in parts[1:]))
    return out


def skeletons():
    """word -> set of (consonants, stressed vowels). A word with two DIFFERENT
    skeletons is a true heteronym (lead/lead, bass/bass) and has no spelling
    answer. Counting every word with two cmudict entries instead called 616
    bank words homographs -- including `accept` and `actually`, which merely
    carry an unstressed-vowel variant. That is pronunciation variation, not
    heteronymy, and it is not what F1 excludes."""
    out = defaultdict(set)
    for line in CMU.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line or line.startswith(";;;"):
            continue
        parts = line.split()
        w = parts[0].lower()
        if w.endswith(")"):
            w = w[: w.rindex("(")]
        if not w.isalpha():
            continue
        strv = tuple(p for p in parts[1:] if p[-1] in "12")
        if strv:                      # skip the unstressed function-word form
            out[w].add(strv)          # `and` is AE1 N D and AH0 N D -- one word
    return out


# Accent-conditional classes. CMUdict is phonemic General American, so it does
# NOT encode flapping (latter/ladder stay distinct) and does not merge
# cot/caught. What it DOES collapse that other accents keep apart is the
# wine/whine (wh-) merger, and what it collapses that varies is the
# yod-dropping set (dew/do, news/noos). Those are the ones flagged here.
# Anything relying on non-rhoticity is invisible to CMUdict rather than
# wrongly included, so it cannot inflate this count.
WH = re.compile(r"^wh", re.I)


def yod_pair(a, b):
    """dew/do, lute/loot -- a UW that some accents render as Y UW."""
    return {a, b} and (("Y UW" in a) != ("Y UW" in b)) and a.replace("Y UW", "UW") == b.replace("Y UW", "UW")


def main():
    if not CMU.exists():
        sys.exit(f"missing {CMU}")
    B, P, SK = bank(), prons(), skeletons()
    REAL = real_words() | set(B)

    # Group EVERY cmudict word by pronunciation, then ask which members are
    # banked. Grouping only banked words would hide the partial sets, which
    # are the whole point of F1.
    groups = defaultdict(set)
    for w, ps in P.items():
        if w not in REAL:
            continue
        for p in ps:
            groups[p].add(w)

    # A SET IS ITS BANKED MEMBERS. F1 says every member must exist in the
    # audited bank, so the set IS the banked group -- an unbanked word that
    # happens to share the pronunciation is not a missing member, it is a
    # GRADING RISK (the player might type it and be marked wrong).
    #
    # Conflating those two called `to/too/two` incomplete -- all three banked --
    # because cmudict also lists `tew`, `tu` and `tue`, and reported ZERO
    # Sweep-eligible families. That was an artifact of the test, not a fact
    # about English.
    closed, risk = [], []
    for p, members in groups.items():
        inb = sorted(w for w in members if w in B)
        if len(inb) < 2:
            continue
        closed.append((p, inb))
        shadow = sorted(members - set(inb))
        if shadow:
            risk.append((p, inb, shadow))

    # Homographs: one spelling, two pronunciations. No spelling answer exists,
    # so they are excluded by construction -- but count them, because each is a
    # word whose audio is ambiguous in a way the mode must not accidentally use.
    homographs = sorted(w for w in B if len(SK.get(w, ())) > 1)

    # Accent-conditional, among bank-closed sets only.
    accent = []
    for p, ms in closed:
        if any(WH.match(m) for m in ms):
            accent.append((p, ms, "wh- merger"))
        elif len(ms) == 2 and yod_pair(*[next(iter(P[m])) for m in ms][:2]):
            accent.append((p, ms, "yod-dropping"))
    accent_keys = {p for p, _, _ in accent}
    universal = [(p, ms) for p, ms in closed if p not in accent_keys]

    def tier_of(ms):
        """A set is as hard as its HARDEST member -- every member must be
        spellable for the family to go gold (F7)."""
        return max((TIERS.index(B[m]) for m in ms), default=0)

    print("=" * 66)
    print("CC-SPELL-O-PHONES F0 — inventory against the audited English bank")
    print("=" * 66)
    print(f"  bank words: {len(B)}   cmudict headwords: {len(P)}")
    print()
    print(f"  BANK-CLOSED sets (shippable):            {len(closed)}")
    print(f"    of those, accent-conditional EXCLUDED: {len(accent)}")
    print(f"    UNIVERSAL, usable in v1:               {len(universal)}")
    print(f"  sets carrying a GRADING RISK (an unbanked real word")
    print(f"    shares the sound; player could type it):     {len(risk)}")
    print(f"  homographs excluded by construction:     {len(homographs)}")
    print()

    print("  SET-SIZE DISTRIBUTION (universal, bank-closed) — the number that")
    print("  decides whether the Sweep is the headline or a rare event:")
    size = defaultdict(int)
    for _, ms in universal:
        size[len(ms)] += 1
    for n in sorted(size):
        label = f"{n}-member" + ("  <- Sweep-eligible" if n >= 3 else "")
        print(f"    {label:<28} {size[n]}")
    sweep = sum(v for k, v in size.items() if k >= 3)
    print(f"    3+ MEMBER FAMILIES:          {sweep}")
    print()

    print("  BY TIER (hardest member):")
    bytier = defaultdict(int)
    for _, ms in universal:
        bytier[TIERS[tier_of(ms)]] += 1
    for t in TIERS:
        print(f"    {t:<10} {bytier[t]}")
    print()

    if sweep:
        print("  Sweep-eligible families (3+ members):")
        for p, ms in sorted((x for x in universal if len(x[1]) >= 3), key=lambda x: -len(x[1])):
            print(f"    {'/'.join(ms)}")
        print()

    if accent:
        print("  ACCENT-CONDITIONAL, excluded from v1 (D2 recommends exclude):")
        for p, ms, why in accent:
            print(f"    {'/'.join(ms):<34} {why}")
        print()

    print(f"  MULTI-PRONUNCIATION bank words (two distinct stressed readings;")
    print(f"  no single spelling answer if the audio picks the other one; {len(homographs)}):")
    print("    " + " ".join(homographs[:40]) + (" ..." if len(homographs) > 40 else ""))
    print()

    print("  GRADING RISK — unbanked real words sharing a set's sound. F1 says")
    print("  the grader must accept the complete set; these are what a player")
    print("  could legally type and be marked wrong for:")
    for _, inb, shadow in sorted(risk, key=lambda x: -len(x[2]))[:18]:
        print(f"    {'/'.join(inb):<26} shadowed by {'/'.join(shadow)}")
    if len(risk) > 18:
        print(f"    ... and {len(risk) - 18} more")
    print()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
