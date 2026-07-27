#!/usr/bin/env python3
"""Chinese definitions (Eric's "build the chinese definitions", 2026-07-27).

Wiktionary's REST definition endpoint omits Chinese sections, so Chinese gets
a BUNDLED gloss table instead: CC-CEDICT (creativecommons CC BY-SA 4.0 — the
canonical free Chinese-English dictionary, attribution in NOTICES.md) filtered
to the words the app can actually serve — every hanzi in the zh bank
(src/words.rs "pinyin|hanzi" entries) — emitted to backend/zh_glosses.json,
which /api/meaning?lang=zh serves exactly like every other language.

Run after changing the zh bank:
    python3 scripts/build-zh-glosses.py [path-to-cedict.txt]
(downloads CC-CEDICT to .corpus-cache/ if no path given)
"""
import gzip, json, os, re, sys, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "backend", "zh_glosses.json")
CEDICT_URL = "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"

def bank_hanzi():
    src = open(os.path.join(ROOT, "src", "words.rs"), encoding="utf-8").read()
    # "pinyin|hanzi" entries in the ZH_* consts.
    return sorted({m.group(1) for m in re.finditer(r'"[a-z0-9]+(?:[a-z0-9]*)\|([^"]+)"', src)})

def cedict_lines(path):
    if path:
        return open(path, encoding="utf-8")
    cache = os.path.join(ROOT, ".corpus-cache")
    os.makedirs(cache, exist_ok=True)
    gz = os.path.join(cache, "cedict.txt.gz")
    if not os.path.exists(gz):
        print("  fetching CC-CEDICT …")
        urllib.request.urlretrieve(CEDICT_URL, gz)
    return gzip.open(gz, "rt", encoding="utf-8")

def clean_glosses(raw):
    """First 1-3 content glosses: drop classifier notes (CL:), cross-references
    ("variant of X"), and pinyin bracket noise."""
    out = []
    for g in raw:
        g = g.strip()
        if not g or g.startswith("CL:"):
            continue
        if re.match(r"^(variant of|old variant of|used in|see) ", g):
            continue
        g = re.sub(r"\[[a-zA-Z0-9: ]+\]", "", g).strip()  # inline pinyin brackets
        if g:
            out.append(g)
        if len(out) == 3:
            break
    return "; ".join(out)

def main():
    words = set(bank_hanzi())
    print(f"  zh bank: {len(words)} hanzi words")
    glosses = {}
    src = sys.argv[1] if len(sys.argv) > 1 else None
    for line in cedict_lines(src):
        if line.startswith("#"):
            continue
        m = re.match(r"^(\S+) (\S+) \[([^\]]+)\] /(.+)/\s*$", line)
        if not m:
            continue
        trad, simp, pinyin, defs = m.groups()
        if simp not in words or simp in glosses:
            continue
        definition = clean_glosses(defs.split("/"))
        if definition:
            glosses[simp] = {"definition": definition, "pinyin": pinyin}
    missing = sorted(words - set(glosses))
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(glosses, f, ensure_ascii=False, indent=0, sort_keys=True)
    print(f"  wrote {len(glosses)}/{len(words)} glosses -> {os.path.relpath(OUT, ROOT)}")
    if missing:
        print(f"  no CEDICT entry ({len(missing)}): {' '.join(missing[:20])}{' …' if len(missing) > 20 else ''}")

if __name__ == "__main__":
    main()
