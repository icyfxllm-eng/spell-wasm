#!/usr/bin/env python3
"""Lexicon ingestion driver.

Reads one or more sources for a language, normalizes each `word` via the
language engine, validates charset, runs the content filter, merges duplicates
by normalized word, POS-normalizes, flags function words, and emits a
deterministic `data/<lang>/lexicon.jsonl` plus `ingest-report/<lang>.md`.

Offline + deterministic: sources are local files under sources/; same inputs →
byte-identical output. NOT run in CI (no network in builds); committed lexicons
are the build inputs. See tools/lexicon-ingest/README.md.

Usage:
  python3 ingest.py <lang> --plainlist                 # migrate shipped lists
  python3 ingest.py ja --jmdict sources/jmdict-eng.json --wordfreq sources/ja.tsv
  python3 ingest.py --all --plainlist                  # every shipped language
"""
from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from schema import Entry, validate, merge, CONTENT_POS, FUNCTION_POS  # noqa: E402
from normalize import normalize, charset_ok, nfc  # noqa: E402
from parsers import SourceMissing  # noqa: E402
from parsers import plainlist, cedict, kaikki, wordfreq, jmdict, cmudict  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent.parent
LANGS = ["en", "es", "fr", "de", "pt", "it", "nl", "pl", "sv", "nb", "tr", "vi", "ko", "ja", "fil", "zh", "th"]

PARSERS = {  # flag -> (module, needs_path)
    "plainlist": (plainlist, False),
    "cedict": (cedict, True),
    "kaikki": (kaikki, True),
    "wordfreq": (wordfreq, True),
    "jmdict": (jmdict, True),
    "cmudict": (cmudict, True),
}


def load_exclusions(lang: str) -> tuple[list[str], set[str]]:
    exdir = ROOT / "assets" / "words" / "exclusions"
    roots, exact = [], set()
    rf = exdir / "_roots.txt"
    if rf.exists():
        roots = [ln.strip().lower() for ln in rf.read_text(encoding="utf-8").splitlines() if ln.strip() and not ln.startswith("#")]
    pf = exdir / f"{lang}.txt"
    if pf.exists():
        exact = {ln.strip().lower() for ln in pf.read_text(encoding="utf-8").splitlines() if ln.strip() and not ln.startswith("#")}
    return roots, exact


def is_blocked(word: str, roots, exact) -> bool:
    lw = word.lower()
    # Exact match for the per-language list (boundary-safe for CJK/Thai/fil),
    # substring for the shared English roots.
    return lw in exact or any(r and r in lw for r in roots)


def ingest(lang: str, sources: list[tuple[str, str | None]]):
    roots, exact = load_exclusions(lang)
    by_word: dict[str, Entry] = {}
    stats = {"parsed": Counter(), "dropped_charset": 0, "dropped_filter": 0, "dropped_invalid": 0}

    for flag, path in sources:
        module, _ = PARSERS[flag]
        for raw in module.parse(path, lang):
            stats["parsed"][flag] += 1
            raw.word = normalize(raw.word, lang)
            if not charset_ok(raw.word, lang):
                stats["dropped_charset"] += 1
                continue
            probs = validate(raw, lambda w: charset_ok(w, lang))
            if probs:
                stats["dropped_invalid"] += 1
                continue
            if is_blocked(raw.word, roots, exact):
                stats["dropped_filter"] += 1
                continue
            if raw.word in by_word:
                by_word[raw.word] = merge(by_word[raw.word], raw)
            else:
                by_word[raw.word] = raw

    # POS post-processing: function-word-only entries get flagged (pool-excluded).
    for e in by_word.values():
        pos = set(e.pos)
        if pos and pos.issubset(FUNCTION_POS):
            e.flags = e.flags + ["function_word"]

    entries = [by_word[w] for w in sorted(by_word)]  # deterministic sort by word
    return entries, stats


def coverage(entries: list[Entry]) -> dict:
    n = len(entries) or 1
    return {
        "with_freq": sum(1 for e in entries if e.freq_rank is not None) * 100 // n,
        "with_pos": sum(1 for e in entries if e.pos) * 100 // n,
        "with_pron": sum(1 for e in entries if e.pron) * 100 // n,
        "with_grade": sum(1 for e in entries if e.grade) * 100 // n,
        "with_etymology": sum(1 for e in entries if e.etymology) * 100 // n,
    }


def write_outputs(lang: str, entries: list[Entry], stats, sources):
    out = ROOT / "data" / lang / "lexicon.jsonl"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(e.to_json() for e in entries) + ("\n" if entries else ""), encoding="utf-8")

    cov = coverage(entries)
    rep_dir = ROOT / "tools" / "lexicon-ingest" / "ingest-report"
    rep_dir.mkdir(parents=True, exist_ok=True)
    lines = [
        f"# Ingest report — {lang}",
        "",
        f"Entries: **{len(entries)}**  →  `data/{lang}/lexicon.jsonl`",
        "",
        "## Sources",
        *[f"- `{flag}` {'('+path+')' if path else '(shipped lists)'}: {stats['parsed'][flag]} parsed" for flag, path in sources],
        "",
        "## Drops",
        f"- charset: {stats['dropped_charset']}",
        f"- content filter: {stats['dropped_filter']}",
        f"- invalid: {stats['dropped_invalid']}",
        "",
        "## Coverage",
        f"- freq_rank: {cov['with_freq']}%",
        f"- pos: {cov['with_pos']}%",
        f"- pron: {cov['with_pron']}%",
        f"- grade: {cov['with_grade']}%",
        f"- etymology: {cov['with_etymology']}%",
    ]
    (rep_dir / f"{lang}.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return out, cov


def main():
    ap = argparse.ArgumentParser(description="Lexicon ingestion")
    ap.add_argument("lang", nargs="?", help="language code (or use --all)")
    ap.add_argument("--all", action="store_true", help="every shipped language")
    for flag, (_, needs_path) in PARSERS.items():
        if needs_path:
            ap.add_argument(f"--{flag}", metavar="PATH", help=f"{flag} source file")
        else:
            ap.add_argument(f"--{flag}", action="store_true", help=f"{flag} source")
    args = ap.parse_args()

    langs = LANGS if args.all else [args.lang]
    if not langs or langs == [None]:
        ap.error("give a language code or --all")

    total = 0
    for lang in langs:
        sources = []
        for flag, (_, needs_path) in PARSERS.items():
            val = getattr(args, flag)
            if needs_path and val:
                sources.append((flag, val))
            elif not needs_path and val:
                sources.append((flag, None))
        if not sources:
            ap.error("give at least one source (e.g. --plainlist)")
        try:
            entries, stats = ingest(lang, sources)
        except SourceMissing as e:
            print(f"[{lang}] {e}", file=sys.stderr)
            continue
        out, cov = write_outputs(lang, entries, stats, sources)
        total += len(entries)
        print(f"[{lang}] {len(entries)} entries -> {out.relative_to(ROOT)}  "
              f"(freq {cov['with_freq']}% pos {cov['with_pos']}% pron {cov['with_pron']}%)")
    print(f"lexicon-ingest: {total} entries across {len(langs)} language(s).")


if __name__ == "__main__":
    main()
