#!/usr/bin/env python3
"""Emit the RAW provenance surface index for a language (CC-WORDLIST-SOURCES).

Reads a raw `unmunch` expansion (one Hunspell surface form per line) and emits
the full set of surface forms BEFORE any game-eligibility filter, as:

  * sources/<lang>/surface-index.txt  — NFC + lowercased + deduped, C-sorted

This is the PROVENANCE index: it answers only "does this word EXIST in the
open-licensed source?", so it deliberately keeps forms the game filter drops
(too long, etc.). It is NOT the playable list (that is wordlists/<lang>.txt).
Validating provenance against this raw set — rather than the length/charset
filtered wordlists/<lang>.txt — is what keeps long-but-legitimate headwords
(e.g. `electrodomestico`) from failing provenance purely as a length artifact.

Determinism: the output is a pure function of the raw input. NFC normalization,
Unicode lowercasing and a codepoint (C-locale) sort are all locale-independent,
so two runs over the same unmunch output are byte-identical. No timestamps.

Nothing here writes to assets/words/ or src/ — the shipped list is untouched.
"""
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def build_surface_set(raw_path: Path) -> set:
    surface = set()
    with raw_path.open("r", encoding="utf-8") as fh:
        for line in fh:
            w = line.rstrip("\n").rstrip("\r")
            if w == "":
                continue
            # Provenance cares only about EXISTENCE: normalize to NFC and
            # lowercase so a curated lowercase headword matches regardless of the
            # source form's capitalization or Unicode composition.
            surface.add(unicodedata.normalize("NFC", w).lower())
    return surface


def main():
    if len(sys.argv) < 3:
        print("usage: surface_index.py <lang> <raw_unmunched>", file=sys.stderr)
        sys.exit(2)
    lang = sys.argv[1]
    raw_path = Path(sys.argv[2])
    if not raw_path.is_file():
        print(f"surface_index: raw unmunch file not found: {raw_path}",
              file=sys.stderr)
        sys.exit(1)

    surface = build_surface_set(raw_path)

    # Deterministic C-locale (codepoint) ordering, independent of shell locale.
    ordered = sorted(surface, key=lambda s: s.encode("utf-8"))

    out = ROOT / "sources" / lang / "surface-index.txt"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(w + "\n" for w in ordered), encoding="utf-8")

    print(f"surface_index({lang}): {len(ordered)} unique surface forms "
          f"-> {out.relative_to(ROOT)}", file=sys.stderr)


if __name__ == "__main__":
    main()
