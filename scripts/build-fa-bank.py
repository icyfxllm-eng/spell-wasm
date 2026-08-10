#!/usr/bin/env python3
"""Grow the Persian (fa) word bank — CC-PERSIAN-FOUNDATION F4.

Persian needs its own grower for the same reason Mandarin does: the generic
build-bigbank.py tiers by LENGTH, and length is the wrong difficulty axis for
Arabic script. A four-letter word with a س/ص ambiguity is harder than a
seven-letter word with none. F5 says difficulty is orthographic depth, so that
is what tiers here.

Pipeline, in order:

  1. Leipzig pes_wikipedia_2021_100K (CC BY 4.0). Note `pes`, not `fas` — the
     Wikipedia corpus is filed under Western Farsi and fas_wikipedia 404s.
  2. canon() — the F1 table. Mirrors src/fa_canon.rs; the two are pinned
     against each other by fa_canon::tests::python_parity, fed by the fixture
     this script emits. Measured recovery on this corpus is small (~0.8%),
     because Leipzig's Persian is already clean — canon earns its place on
     player input and OCR, not here.
  3. fa-legal codepoints + keyboard reachability (assets/keyboards/fa.json).
  4. Stopword filter. Function words are not spelling content; nothing in the
     generic pipeline filters them, which is why German easy shipped full of
     aber/dann/als. Persian gets an explicit list rather than the same hole.
  5. Frequency-rank, take the useful head, then tier by depth within it.
     Rarity is the dominant difficulty signal AND suppresses the proper-noun
     tail; depth grades within it. Sorting by depth FIRST fills the middle
     tiers with rare zero-trap words, which is worse than useless.

Tier sizes respect the generator's ceiling, max(840, poolFloor*1.25) with ar's
floors per D3 — 840/1250/2500/3750. Caseless script, so residual proper nouns
survive, exactly as they do for ar/hi/ko/ja.
"""
import json, os, sys, tarfile, unicodedata as ud, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, ".corpus-cache")
CORPUS = "pes_wikipedia_2021_100K"
OUT = os.path.join(ROOT, "assets", "words", "fa")
FIXTURE = os.path.join(ROOT, "tests", "fixtures", "fa-canon-parity.json")

TIERS = [("easy", 840), ("medium", 1250), ("hard", 2500), ("expert", 3750)]
HEAD = 15000          # the useful vocabulary; beyond this is long-tail noise

# CC-PERSIAN-FOUNDATION F5 — the five homophone-letter sets. Membership count
# is the difficulty axis.
TRAPS = [set("سصث"), set("زذضظ"), set("تط"), set("هح"), set("غق")]

STOP = set("""در به از که را این است با آن یک برای تا بر هم یا نیز اما اگر چون
پس هر بود شد می ها های ای او وی ما شما آنها ولی بین روی زیر بالا کرد کند شود
باشد بوده کرده دیگر بسیار همه هیچ چه کجا کی چرا بله نه آیا خود نیست هستند
بودند کردند شدند دارد دارند داشت داشتند باید نباید توان شاید مانند مثل طبق
درباره علیه سپس بنابراین همچنین ضمن جهت سوی نزد""".split())

FA32 = "ابپتثجچحخدذرزژسشصضطظعغفقکگلمنوهی"
EXTRA = "آءؤئ"          # D1 extension, Eric 2026-08-09
ZWNJ = "‌"
LEGAL = set(FA32 + EXTRA + ZWNJ) | {chr(c) for c in range(0x6F0, 0x6FA)}


def canon(s):
    """The F1 table. Kept byte-identical in behaviour to src/fa_canon.rs."""
    # presentation forms fold FIRST: folding last is not idempotent, because a
    # presentation kaf lands on ARABIC kaf after the kaf row has already run.
    s = "".join(
        ud.normalize("NFKC", c) if ("ﭐ" <= c <= "﷿" or "ﹰ" <= c <= "﻿") else c
        for c in s
    )
    # compose BEFORE stripping marks: a decomposed آ is ا + U+0653, and U+0653
    # is inside the harakat range, so stripping first would silently turn آب
    # into اب.
    s = ud.normalize("NFC", s)
    out = []
    for c in s:
        o = ord(c)
        if c in ("ي", "ى"):        out.append("ی")   # Arabic yeh / alef maksura
        elif c == "ك":                  out.append("ک")   # Arabic kaf
        elif 0x0660 <= o <= 0x0669:          out.append(chr(o - 0x0660 + 0x06F0))
        elif c == "ـ":                  pass                    # tatweel
        elif 0x064B <= o <= 0x065F or o == 0x0670: pass              # harakat
        elif c in ("أ", "إ"):      out.append("ا")   # alef+hamza -> alef
        elif c == "ة":                  out.append("ه")   # teh marbuta -> heh
        else:                                out.append(c)
    return ud.normalize("NFC", "".join(out))


def depth(w):
    return sum(1 for t in TRAPS if set(w) & t)


def reachable():
    kb = json.load(open(os.path.join(ROOT, "assets", "keyboards", "fa.json"), encoding="utf-8"))
    ch = set()
    for row in kb["rows"]:
        ch.update(row)
    for base, alts in kb.get("longPress", {}).items():
        ch.add(base); ch.update(alts)
    return ch


def corpus_rows():
    os.makedirs(CACHE, exist_ok=True)
    tgz = os.path.join(CACHE, CORPUS + ".tar.gz")
    if not os.path.exists(tgz):
        print(f"    fetching {CORPUS} …", flush=True)
        urllib.request.urlretrieve(
            f"https://downloads.wortschatz-leipzig.de/corpora/{CORPUS}.tar.gz", tgz)
    with tarfile.open(tgz) as t:
        m = next(x for x in t.getmembers() if x.name.endswith("-words.txt"))
        rows = []
        for line in t.extractfile(m).read().decode("utf-8").splitlines():
            p = line.split("\t")
            if len(p) >= 3:
                try:
                    rows.append((p[1], int(p[2])))
                except ValueError:
                    pass
    rows.sort(key=lambda x: -x[1])
    return rows


def main(apply=False):
    reach = reachable()
    rows = corpus_rows()
    seen, pool, recovered = set(), [], 0
    for raw, f in rows:
        if not (2 <= len(raw) <= 16):
            continue
        w = canon(raw)
        if not (2 <= len(w) <= 16):
            continue
        if not w or any(c not in LEGAL for c in w):
            continue
        if any(c not in reach and c != ZWNJ for c in w):
            continue                      # unreachable on the fa keyboard
        if w in STOP or w in seen:
            continue
        seen.add(w)
        pool.append((w, f))
        if w != raw:
            recovered += 1

    print(f"  corpus rows            {len(rows)}")
    print(f"  legal + reachable      {len(pool)}   (canon recovered {recovered})")

    head = pool[:HEAD]
    head.sort(key=lambda x: (depth(x[0]), -x[1]))
    banks, i, prev, ok = {}, 0, -1.0, True
    for name, n in TIERS:
        ws = [w for w, _ in head[i:i + n]]; i += n
        banks[name] = ws
        mt = sum(depth(w) for w in ws) / len(ws)
        if mt < prev:
            ok = False
        prev = mt
        print(f"  {name:7s} n={len(ws):5d}  mean traps={mt:.2f}  mean len={sum(map(len, ws))/len(ws):.1f}")
    print(f"  F5 monotonicity: {'PASS' if ok else 'FAIL'}")
    if not ok:
        sys.exit("ABORT: trap count must be non-decreasing across tiers (F5)")

    if not apply:
        print("\n(dry run)")
        return

    os.makedirs(OUT, exist_ok=True)
    for name, _ in TIERS:
        open(os.path.join(OUT, f"{name}.txt"), "w", encoding="utf-8").write(
            "\n".join(banks[name]) + "\n")

    # Parity fixture: raw corpus forms that canon actually CHANGED, plus the
    # hand-built pathological cases. Rust re-canonicalizes these and must agree.
    changed = [(r, canon(r)) for r, _ in rows[:20000] if canon(r) != r][:400]
    pathological = ["كتاب", "بلي", "بـل",
                    "مَد", "٦٧", "ﻛ", "ﻻ",
                    "آب", "أب", "ة"]
    fixture = {"note": "fa canon parity: Python ingestion vs src/fa_canon.rs. "
                       "Pins that the two agree; the Rust row tests pin that the table is right.",
               "cases": [{"in": a, "out": b} for a, b in changed]
                        + [{"in": p, "out": canon(p)} for p in pathological]}
    os.makedirs(os.path.dirname(FIXTURE), exist_ok=True)
    json.dump(fixture, open(FIXTURE, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"\napplied — {sum(len(v) for v in banks.values())} words, "
          f"parity fixture {len(fixture['cases'])} cases")


if __name__ == "__main__":
    main("--apply" in sys.argv)
