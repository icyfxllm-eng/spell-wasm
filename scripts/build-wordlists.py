#!/usr/bin/env python3
"""Deterministic word-list build pipeline (localization Phase 3, §3.4).

Reads curated per-tier sources under ``assets/words/{code}/{tier}.txt``, applies
the mechanical gates below, and emits ``src/word_data.rs`` — the ``const`` banks
that ``words.rs`` dispatches over. Run it after editing any source list:

    python3 scripts/build-wordlists.py            # build + gate
    python3 scripts/build-wordlists.py --check     # gate only, no write (CI)

Tier assignment is an INPUT here (human spelling-difficulty curation — "rhythm"
is hard at six letters), not computed from length. Frequency-driven re-tiering
is a future enhancement for when licensed frequency data (hermitdave/
FrequencyWords, wordfreq) is wired in; see assets/words/LICENSES.md.

Mechanical gates — any violation fails the build (non-zero exit):
  1. Charset  — every character of every word (after the strict fold the player
                must reproduce) is reachable on that locale's keyboard,
                assets/keyboards/{code}.json (base rows + long-press). §3.4/1.
  2. Exclusions — no word matches the locale's exclusion list or a shared root.
  3. Balance  — each tier is within ±20% of the English tier's word count.
  4. Determinism — output is a pure, sorted function of the inputs (byte-stable).

Also enforced per word: NFC form, all-alphabetic (no spaces/digits/punctuation),
length 3..=16 (the 16-char cap covers de/nl/sv compounds, §3.3), and dedup
within a language (first tier wins).
"""
import sys
import json
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# CC-LINEUP-SWAP (2026-07-16): it/nl/sv/nb cut — their curated sources are
# archived under archive/wordlists/<code>/, so the pipeline can no longer build
# them. Thai (th) is fully removed from the program — its sources, locale
# catalog, and registry entry are all gone, so it is not listed here.
# fa/ur are NOT here — cut from the lineup (CC-MASTER-PARITY Phase A).
# ar joined on Eric's ungate ruling (2026-07-25): sources assets/words/ar/,
# keyboard ar.json, RTL_SUPPORTED flipped in the same change.
# and listing them before the sources land would just fail the build.
LANGS = ["en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "ru", "sw", "ar"]
TIERS = ["easy", "medium", "hard", "expert"]
MIN_LEN, MAX_LEN = 2, 16
# Tier size gate: a floor (no language starved of words) and a ceiling (sanity).
# This replaced a tight "±20% of English" match — for a spelling game more words in
# one language is variety, not unfairness, so the gate guards against too FEW, not
# against differing sizes. Lets English + others grow big (build-bigbank.py) while
# a not-yet-grown language keeps its smaller curated bank above the floor.
MIN_PER_TIER, MAX_PER_TIER = 30, 800


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s)


def strict_fold(s: str) -> str:
    """NFC + case fold, whitespace removed — what the keyboard must reproduce."""
    return "".join(c for c in nfc(s).casefold() if not c.isspace())


def lenient_fold(s: str) -> str:
    """Accent-stripping fold for exclusion matching (mirrors Rust fold_lenient)."""
    lowered = nfc(s).casefold()
    stripped = "".join(c for c in unicodedata.normalize("NFD", lowered) if not unicodedata.combining(c))
    table = {"ß": "ss", "æ": "ae", "œ": "oe", "ł": "l", "ø": "o", "ı": "i"}
    return "".join(table.get(c, c) for c in stripped if not c.isspace())


def reachable_chars(code: str) -> set:
    layout = json.loads((ROOT / "assets" / "keyboards" / f"{code}.json").read_text(encoding="utf-8"))
    chars = set()
    for row in layout["rows"]:
        chars.update(row)
    for base, accents in layout.get("longPress", {}).items():
        chars.add(base)
        chars.update(accents)
    if code == "vi":
        # The Vietnamese tone row applies any of the five tones to any reachable
        # vowel (mirrors src/viet.rs), so every toned form is typeable too.
        tones = ["̀", "́", "̉", "̃", "̣"]
        letter_marks = {"̂", "̆", "̛"}
        vowels = [c for c in list(chars) if unicodedata.normalize("NFD", c)[0].lower() in "aeiouy"]
        for v in vowels:
            dec = unicodedata.normalize("NFD", v)
            base, lm = dec[0], [m for m in dec[1:] if m in letter_marks]
            for t in tones:
                chars.update(unicodedata.normalize("NFC", base + "".join(lm) + t))
    if code == "ko":
        # Dubeolsik + the Hangul automaton compose every precomposed syllable.
        chars.update(chr(u) for u in range(0xAC00, 0xD7A4))
    return chars


def load_exclusions(code: str):
    exdir = ROOT / "assets" / "words" / "exclusions"
    roots, exact = [], set()
    roots_file = exdir / "_roots.txt"
    if roots_file.exists():
        for line in roots_file.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                roots.append(lenient_fold(line))
    per = exdir / f"{code}.txt"
    if per.exists():
        for line in per.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                exact.add(lenient_fold(line))
    return roots, exact


def is_excluded(word: str, roots, exact) -> bool:
    lf = lenient_fold(word)
    return lf in exact or any(r and r in lf for r in roots)


def build():
    problems = []  # fail the build (§3.4 gates)
    warnings = []  # curation filters — dropped words, reported but not fatal
    banks = {}  # (code, tier) -> [words]
    for code in LANGS:
        reach = reachable_chars(code)
        roots, exact = load_exclusions(code)
        # CJK scripts: one character is a whole word, so allow length 1.
        min_len = 1 if code in ("ko", "ja", "zh") else MIN_LEN
        seen = set()
        for tier in TIERS:
            src = ROOT / "assets" / "words" / code / f"{tier}.txt"
            out = []
            for raw in src.read_text(encoding="utf-8").splitlines():
                w = nfc(raw.strip())
                if not w or w.startswith("#"):
                    continue
                where = f"{code}/{tier}: {w!r}"
                # Curation filters (drop + warn): non-letters (fr apostrophe/hyphen
                # forms, §3.3) and the length cap (de/nl/sv compounds, §3.3).
                # Filipino keeps the hyphen.
                if not all(c.isalpha() or (code == "fil" and c == "-") for c in w):
                    warnings.append(f"{where} — dropped (non-alphabetic)")
                    continue
                if not (min_len <= len(w) <= MAX_LEN):
                    warnings.append(f"{where} — dropped (length {len(w)} outside {MIN_LEN}..{MAX_LEN})")
                    continue
                # Hard gates (fail the build).
                bad = [c for c in strict_fold(w) if c not in reach]
                if bad:
                    problems.append(f"{where} — chars not on {code} keyboard: {''.join(bad)}")
                    continue
                if is_excluded(w, roots, exact):
                    problems.append(f"{where} — matches exclusion list")
                    continue
                key = strict_fold(w)
                if key in seen:
                    continue  # dedup within language, first tier wins
                seen.add(key)
                out.append(w)
            banks[(code, tier)] = out

    # Gate 3: every language/tier has a healthy number of words — a floor (not
    # starved) and a ceiling (no runaway). Sizes may differ across languages.
    for tier in TIERS:
        for code in LANGS:
            n = len(banks[(code, tier)])
            if not (MIN_PER_TIER <= n <= MAX_PER_TIER):
                problems.append(f"size: {code}/{tier} has {n} words, outside [{MIN_PER_TIER},{MAX_PER_TIER}]")

    if warnings:
        print(f"build-wordlists: {len(warnings)} word(s) dropped by curation filters:", file=sys.stderr)
        for w in warnings:
            print("  " + w, file=sys.stderr)
    if problems:
        print(f"build-wordlists: {len(problems)} gate violation(s):", file=sys.stderr)
        for p in problems:
            print("  " + p, file=sys.stderr)
        return None
    return banks


# ---- CC-SPELL-RACING G-B: content-derived stable word IDs -------------------
# FNV-1a-64 of NFC(word). Content-derived so a word's ID never changes when other
# words are added/removed. MUST stay byte-identical to src/wordid.rs (a Rust test
# pins the per-tier hashes below against a fresh Rust recompute).
_FNV64_OFF = 0xCBF29CE484222325
_FNV64_PRIME = 0x100000001B3
_MASK64 = (1 << 64) - 1


def word_id(word: str) -> int:
    h = _FNV64_OFF
    for b in unicodedata.normalize("NFC", word).encode("utf-8"):
        h = ((h ^ b) * _FNV64_PRIME) & _MASK64
    return h


def list_hash(words) -> int:
    """FNV-1a-64 over each word's 8-byte big-endian ID, in `words` order."""
    h = _FNV64_OFF
    for w in words:
        for b in word_id(w).to_bytes(8, "big"):
            h = ((h ^ b) * _FNV64_PRIME) & _MASK64
    return h


def check_id_collisions(banks):
    """No two distinct words in one language may share a 64-bit ID. The
    determinism the racing ghosts rely on assumes this holds; the gate makes it
    an invariant rather than a hope. (Never expected at 64-bit.)"""
    problems = []
    for code in LANGS:
        seen = {}
        for tier in TIERS:
            for w in banks[(code, tier)]:
                wid = word_id(w)
                if wid in seen and seen[wid] != w:
                    problems.append(f"{code}: word-ID collision 0x{wid:016x} — {seen[wid]!r} vs {w!r}")
                seen[wid] = w
    return problems


def render(banks) -> str:
    def esc(w):
        return w.replace("\\", "\\\\").replace('"', '\\"')

    lines = [
        "// @generated by scripts/build-wordlists.py — DO NOT EDIT BY HAND.",
        "// Edit the curated sources in assets/words/{code}/{tier}.txt and re-run:",
        "//   python3 scripts/build-wordlists.py",
        "// The pipeline gates charset (vs assets/keyboards/*), exclusions, tier",
        "// balance (±20% of English) and determinism before writing this file.",
        "",
    ]
    for code in LANGS:
        for tier in TIERS:
            name = f"{code.upper()}_{tier.upper()}"
            # Deterministic: canonical codepoint sort within each tier.
            words = sorted(banks[(code, tier)])
            joined = ",".join(f'"{esc(w)}"' for w in words)
            lines.append(f"pub const {name}: &[&str] = &[{joined}];")
        lines.append("")

    # CC-SPELL-RACING G-B: golden per-(lang,tier) wordListHash (FNV-1a-64 over the
    # tier's ordered word IDs, SAME sorted order as the arrays above). src/wordid.rs
    # recomputes these in Rust and a test asserts equality — the drift guard that
    # keeps the build (Python) and runtime (Rust) hashes identical, which the ghost
    # determinism depends on. Empty tiers are omitted.
    lines.append("/// (lang, tier, wordListHash) — see the note in build-wordlists.py.")
    lines.append("pub const TIER_HASHES: &[(&str, &str, u64)] = &[")
    for code in LANGS:
        for tier in TIERS:
            words = sorted(banks[(code, tier)])
            if words:
                lines.append(f'    ("{code}", "{tier}", 0x{list_hash(words):016X}),')
    lines.append("];")
    return "\n".join(lines) + "\n"


def main():
    check_only = "--check" in sys.argv
    banks = build()
    if banks is None:
        sys.exit(1)
    id_problems = check_id_collisions(banks)
    if id_problems:
        print("build-wordlists: word-ID collisions (CC-SPELL-RACING G-B):", file=sys.stderr)
        for p in id_problems:
            print(f"  {p}", file=sys.stderr)
        sys.exit(1)
    total = sum(len(v) for v in banks.values())
    out = ROOT / "src" / "word_data.rs"
    rendered = render(banks)
    if check_only:
        current = out.read_text(encoding="utf-8") if out.exists() else ""
        if current != rendered:
            print("build-wordlists: src/word_data.rs is stale — run without --check", file=sys.stderr)
            sys.exit(1)
        print(f"build-wordlists: OK (check) — {total} words across {len(LANGS)} locales, gates green.")
    else:
        out.write_text(rendered, encoding="utf-8")
        print(f"build-wordlists: wrote {out.relative_to(ROOT)} — {total} words across {len(LANGS)} locales, gates green.")


if __name__ == "__main__":
    main()
