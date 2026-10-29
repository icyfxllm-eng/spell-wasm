#!/usr/bin/env python3
"""CLIP-REPORT: words by "Can't tell" report count, so Eric can quarantine them.

Reads the rows of workers/telemetry/reports/clips.sql (wrangler's --json answer)
on stdin, turns each item_hash back into a word by hashing every bank entry
with the client's own function, and prints one line per word:

    reports  lang  word  variant  source(s)

    bash scripts/clip_report.sh                       # live, via wrangler
    python3 scripts/clip_report.py < answer.json      # a saved answer

A report is evidence, not a verdict (one player on a muted phone is one
report). Read the SOURCE column first: a word reported mostly as native_tts is
the on-device fallback voice, not a bad clip, and quarantining it fixes
nothing. Reports mostly from server_cache or human are the clip's fault.

The hash is FNV-1a (64-bit) of "<lang>|<bank entry>", exactly telemetry's
`item_hash`. A hash that matches no bank entry is listed as (unknown).
"""
import collections
import glob
import json
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK = (1 << 64) - 1


def fnv1a(data: bytes) -> int:
    h = FNV_OFFSET
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & MASK
    return h


def item_hash(lang: str, entry: str) -> str:
    return f"{fnv1a(f'{lang}|{entry}'.encode('utf-8')):016x}"


CONST = re.compile(r'pub const ([A-Z0-9_]+): &\[&str\] = &\[(.*?)\];', re.S)
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


def bank_entries(root=ROOT):
    """(lang, entry) for every bank constant in src/word_data*.rs.

    A constant's language is its name up to the first underscore (EN_EASY ->
    en). Anything else a bank holds under another prefix is simply not found,
    and its hashes print as (unknown).
    """
    for path in sorted(glob.glob(os.path.join(root, "src", "word_data*.rs"))):
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        for name, body in CONST.findall(text):
            lang = name.split("_")[0].lower()
            for lit in STRING.findall(body):
                yield lang, json.loads(f'"{lit}"')
    for path in sorted(glob.glob(os.path.join(root, "audio_clarity", "words-*.json"))):
        lang = os.path.basename(path)[len("words-"):-len(".json")]
        with open(path, encoding="utf-8") as fh:
            for row in json.load(fh):
                yield lang, row["word"]


def build_index(entries):
    index = {}
    for lang, entry in entries:
        index[(lang, item_hash(lang, entry))] = entry
    return index


def summarize(rows, index):
    """rows: dicts with lang, item_hash, variant, source, reports."""
    words = collections.OrderedDict()
    for r in rows:
        key = (r["lang"], r["item_hash"])
        w = words.setdefault(key, {"reports": 0, "variants": collections.Counter(), "sources": collections.Counter()})
        n = int(r.get("reports", 1))
        w["reports"] += n
        w["variants"][r["variant"]] += n
        w["sources"][r["source"]] += n
    out = []
    for (lang, h), w in words.items():
        word = index.get((lang, h))
        out.append({
            "reports": w["reports"],
            "lang": lang,
            "word": word if word is not None else f"(unknown {h})",
            "variants": dict(w["variants"]),
            "sources": dict(w["sources"]),
        })
    out.sort(key=lambda x: (-x["reports"], x["lang"], x["word"]))
    return out


def fmt(counter):
    return ",".join(f"{k}:{v}" for k, v in sorted(counter.items(), key=lambda kv: (-kv[1], kv[0])))


def selftest():
    """The same vectors src/telemetry/mod.rs pins, so client and report agree."""
    assert f"{fnv1a(b''):016x}" == "cbf29ce484222325"
    assert f"{fnv1a(b'a'):016x}" == "af63dc4c8601ec8c"
    rows = [{"lang": "en", "item_hash": item_hash("en", "cat"), "variant": "normal", "source": "human", "reports": 2},
            {"lang": "en", "item_hash": item_hash("en", "cat"), "variant": "slow", "source": "native_tts", "reports": 1}]
    out = summarize(rows, {("en", item_hash("en", "cat")): "cat"})
    assert out[0]["word"] == "cat" and out[0]["reports"] == 3, out
    print("clip_report selftest: OK")


def main():
    if "--selftest" in sys.argv:
        return selftest()
    try:
        rows = json.load(sys.stdin)[0]["results"]
    except Exception as e:
        sys.exit(f"  (could not read the answer: {e})")
    if not rows:
        print("  (no reports)")
        return
    out = summarize(rows, build_index(bank_entries()))
    cols = ["reports", "lang", "word", "variants", "sources"]
    table = [[str(o["reports"]), o["lang"], o["word"], fmt(o["variants"]), fmt(o["sources"])] for o in out]
    w = [max(len(c), *(len(r[i]) for r in table)) for i, c in enumerate(cols)]
    print("  " + "  ".join(c.ljust(n) for c, n in zip(cols, w)))
    print("  " + "  ".join("-" * n for n in w))
    for r in table:
        print("  " + "  ".join(c.ljust(n) for c, n in zip(r, w)))
    print(f"  ({len(table)} words)")


if __name__ == "__main__":
    main()
