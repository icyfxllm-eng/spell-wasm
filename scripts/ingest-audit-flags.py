#!/usr/bin/env python3
"""Fold an auditor's exported flags into assets/words/exclusions/<lang>.txt.

The audit-preview flag widget (scripts/build-web-audit.sh) exports a file of words
a native reviewer marked to cut, grouped by language:

    # audit flags — ...
    [ar]
    كلمة
    [fa]
    واژه

This appends each word to that language's exclusion list — the SAME list the
production pipeline (build-wordlists.py) and the draft builder (build-draft-banks.py)
already honour — so a flagged word is dropped everywhere on the next rebuild, draft
and production alike. Idempotent: the list is deduped (NFC) and sorted, so
re-ingesting the same export is a no-op.

Usage:  python3 scripts/ingest-audit-flags.py audit-flags.txt
Then:   python3 scripts/build-draft-banks.py   (drops the flagged words from drafts)
"""
import os, sys, unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXDIR = os.path.join(ROOT, "assets", "words", "exclusions")


def parse(path):
    flags, lang = {}, None
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            lang = line[1:-1].strip()
            flags.setdefault(lang, set())
        elif lang is not None:
            flags[lang].add(unicodedata.normalize("NFC", line))
    return flags


def merge(lang, words):
    path = os.path.join(EXDIR, f"{lang}.txt")
    header = f"# {lang} exclusions — words cut from the banks (audit red-pen + manual).\n"
    existing, had_header = [], False
    if os.path.exists(path):
        for raw in open(path, encoding="utf-8"):
            s = raw.strip()
            if s.startswith("#"):
                had_header = True
                continue
            if s:
                existing.append(unicodedata.normalize("NFC", s))
    combined = sorted(set(existing) | set(words))
    added = len(combined) - len(set(existing))
    os.makedirs(EXDIR, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(header if not had_header else header)  # always (re)write a header
        f.write("\n".join(combined) + "\n")
    return added, len(combined)


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: ingest-audit-flags.py <audit-flags.txt>")
    flags = parse(sys.argv[1])
    if not flags:
        print("  no flags found in export")
        return
    for lang in sorted(flags):
        added, total = merge(lang, flags[lang])
        print(f"  {lang}: +{added} new (exclusion list now {total})")
    print("  now run: python3 scripts/build-draft-banks.py  (drops them from the drafts)")


if __name__ == "__main__":
    main()
