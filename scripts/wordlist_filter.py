#!/usr/bin/env python3
"""Deterministic game-eligibility filter for the word-list provenance pipeline.

Reads a raw unmunched Hunspell expansion (one surface form per line), applies
the per-language game-eligibility rules, and emits:
  * wordlists/<lang>.txt          — sorted (LC_ALL=C byte order), deduped, NFC
  * wordlists/<lang>.manifest.json — provenance + per-rule drop counts

Determinism guarantees (see the CC-WORDLIST-SOURCES acceptance criteria):
  * No timestamps in any emitted file (the fixed retrieval date lives only in
    sources/<lang>/PROVENANCE.md). The manifest is a pure function of its inputs.
  * Output ordering is codepoint (C-locale) sort, applied here in Python, so it
    does not depend on the caller's locale.
  * Every dropped word is attributed to exactly one named rule (first match in
    RULE_ORDER), so the manifest counts explain the whole input->output delta.

REVIEW-GATED: the es rule set is a PROPOSAL for Eric. See sources/es/FILTER-RULES.md.
Nothing here writes to assets/words/ or src/ — the shipped list is untouched.
"""
import json
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# Per-language game-eligibility rules.
# Each language maps to an allowed lowercase alphabet and length bounds.
# The RULE pipeline below is shared; only these knobs are per-language.
# ---------------------------------------------------------------------------
LANG_RULES = {
    "es": {
        # "lowercase a-z plus the Spanish accented set". a-z already includes
        # w and k (loanword letters). No ç (that is Catalan/French).
        "alphabet": set("abcdefghijklmnopqrstuvwxyzáéíóúüñ"),
        "min_len": 3,
        "max_len": 15,
    },
}

# Order matters: a word is attributed to the FIRST rule it violates.
RULE_ORDER = [
    "nfc_renormalized",   # informational: NFC changed the bytes (not a drop)
    "has_digit",
    "has_punct_or_space",
    "not_lowercase",      # any uppercase => proper noun / acronym / capitalized-only
    "out_of_charset",
    "too_short",          # length < min_len (also catches single letters)
    "too_long",           # length > max_len
    "duplicate",          # survives all rules but already emitted
]

RULE_DESCRIPTIONS = {
    "nfc_renormalized": "Surface form was not in NFC; re-normalized (kept, logged per D3).",
    "has_digit": "Contains a decimal digit (0-9).",
    "has_punct_or_space": "Contains whitespace, hyphen, apostrophe, period, slash or other punctuation.",
    "not_lowercase": "Contains an uppercase letter (proper noun, acronym, or capitalized-only form).",
    "out_of_charset": "Contains a character outside the language's allowed lowercase alphabet.",
    "too_short": "Fewer than min_len characters (also removes single letters).",
    "too_long": "More than max_len characters.",
    "duplicate": "Identical to an already-emitted word after normalization.",
}


def classify(word: str, alphabet: set, min_len: int, max_len: int):
    """Return (kept_normalized_or_None, drop_rule_or_None, nfc_changed_bool)."""
    nfc = unicodedata.normalize("NFC", word)
    nfc_changed = nfc != word
    w = nfc
    if any(ch.isdigit() for ch in w):
        return None, "has_digit", nfc_changed
    # whitespace or anything unicode-classed as punctuation/symbol/separator
    for ch in w:
        cat = unicodedata.category(ch)
        if ch.isspace() or cat[0] in ("P", "Z", "S", "C"):
            return None, "has_punct_or_space", nfc_changed
    if w != w.lower():
        return None, "not_lowercase", nfc_changed
    if any(ch not in alphabet for ch in w):
        return None, "out_of_charset", nfc_changed
    if len(w) < min_len:
        return None, "too_short", nfc_changed
    if len(w) > max_len:
        return None, "too_long", nfc_changed
    return w, None, nfc_changed


def main():
    if len(sys.argv) < 3:
        print("usage: wordlist_filter.py <lang> <raw_unmunched> "
              "[--hunspell VER] [--unmunch-tool NAME]", file=sys.stderr)
        sys.exit(2)
    lang = sys.argv[1]
    raw_path = Path(sys.argv[2])
    hunspell_ver = "unknown"
    unmunch_tool = "unmunch (hunspell)"
    args = sys.argv[3:]
    for i, a in enumerate(args):
        if a == "--hunspell" and i + 1 < len(args):
            hunspell_ver = args[i + 1]
        if a == "--unmunch-tool" and i + 1 < len(args):
            unmunch_tool = args[i + 1]

    if lang not in LANG_RULES:
        print(f"error: no game-eligibility rules defined for '{lang}'. "
              f"Add a LANG_RULES entry.", file=sys.stderr)
        sys.exit(1)
    cfg = LANG_RULES[lang]

    registry = json.loads((ROOT / "sources" / "registry.json").read_text("utf-8"))
    src = registry["sources"].get(lang)
    if src is None:
        print(f"error: no registry entry for '{lang}' in sources/registry.json",
              file=sys.stderr)
        sys.exit(1)

    drop_counts = {r: 0 for r in RULE_ORDER}
    nfc_renormalized = 0
    total_in = 0
    kept = set()

    with raw_path.open("r", encoding="utf-8") as fh:
        for line in fh:
            word = line.rstrip("\n").rstrip("\r")
            if word == "":
                continue
            total_in += 1
            norm, rule, nfc_changed = classify(
                word, cfg["alphabet"], cfg["min_len"], cfg["max_len"])
            if nfc_changed:
                nfc_renormalized += 1
            if rule is not None:
                drop_counts[rule] += 1
                continue
            if norm in kept:
                drop_counts["duplicate"] += 1
                continue
            kept.add(norm)

    # nfc_renormalized is informational (D3), not a drop bucket.
    drop_counts["nfc_renormalized"] = nfc_renormalized

    # Deterministic C-locale (codepoint) ordering, independent of shell locale.
    out_words = sorted(kept, key=lambda s: s.encode("utf-8"))

    out_txt = ROOT / "wordlists" / f"{lang}.txt"
    out_txt.parent.mkdir(parents=True, exist_ok=True)
    out_txt.write_text("".join(w + "\n" for w in out_words), encoding="utf-8")

    total_dropped = sum(v for r, v in drop_counts.items() if r != "nfc_renormalized")
    manifest = {
        "language": lang,
        "generator": "scripts/wordlist.sh + scripts/wordlist_filter.py",
        "note": "REVIEW-GATED output. Not wired into the shipped app. "
                "es rule set is a proposal for Eric (sources/es/FILTER-RULES.md).",
        "source": {
            "name": src["name"],
            "url": src["url"],
            "license": src["license_spdx"],
            "tier": src["tier"],
            "version": src["version"],
            "commit": src["commit"],
            "artifact": src["artifact"],
            "tarball_sha256": src["tarball_sha256"],
        },
        "tools": {
            "unmunch": unmunch_tool,
            "hunspell_version": hunspell_ver,
            "python": f"{sys.version_info.major}.{sys.version_info.minor}",
            "unicodedata_version": unicodedata.unidata_version,
        },
        "eligibility_rules": {
            "alphabet": "".join(sorted(cfg["alphabet"])),
            "min_len": cfg["min_len"],
            "max_len": cfg["max_len"],
            "order": RULE_ORDER,
            "descriptions": RULE_DESCRIPTIONS,
        },
        "counts": {
            "unmunched_in": total_in,
            "nfc_renormalized": nfc_renormalized,
            "dropped_by_rule": {r: drop_counts[r] for r in RULE_ORDER
                                if r != "nfc_renormalized"},
            "total_dropped": total_dropped,
            "kept_out": len(out_words),
        },
    }
    out_manifest = ROOT / "wordlists" / f"{lang}.manifest.json"
    out_manifest.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8")

    print(f"filter({lang}): {total_in} in -> {len(out_words)} out "
          f"({total_dropped} dropped, {nfc_renormalized} NFC-renormalized)",
          file=sys.stderr)


if __name__ == "__main__":
    main()
