"""Per-language answer normalization + charset checks for the lexicon.

Mirrors the Rust engines that validate answers at runtime (src/norm.rs,
viet.rs, pinyin.rs, hangul.rs) so the lexicon `word` field is stored in the
exact form the app compares against. Kept in sync with those by the shared
keyboard-layout SSOT (assets/keyboards/*.json) used for charset checks.
"""
from __future__ import annotations

import json
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent  # repo root
KB = ROOT / "assets" / "keyboards"


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s.strip())


def normalize(word: str, lang: str) -> str:
    """Return the canonical answer form for `lang` (what the player must type)."""
    w = nfc(word)
    if lang == "zh":
        # pinyin: lowercase, v->ü, drop neutral-tone 5 and separators (src/pinyin.rs)
        out = []
        for c in w.lower():
            if c == "v":
                out.append("ü")
            elif c in "5 '-":
                continue
            else:
                out.append(c)
        return "".join(out)
    if lang in ("ja", "ko", "th"):
        return w  # scripts have no case; NFC is the canonical form
    return w.lower()  # Latin-script languages (incl. vi, fil): case-insensitive


def _reachable(lang: str) -> set[str]:
    """Characters the language's keyboard can produce (from the SSOT layout),
    mirroring the Rust/Python charset gate incl. vi tones, ko syllables, zh
    pinyin, th combining marks."""
    code = lang
    path = KB / f"{code}.json"
    if not path.exists():
        # Latin fallback for languages that reuse the English layout.
        return set("abcdefghijklmnopqrstuvwxyz")
    layout = json.loads(path.read_text(encoding="utf-8"))
    chars: set[str] = set()
    for row in layout["rows"]:
        chars.update(row)
    for base, acc in layout.get("longPress", {}).items():
        chars.add(base)
        chars.update(acc)
    if code == "vi":
        tones = ["̀", "́", "̉", "̃", "̣"]
        letter_marks = {"̂", "̆", "̛"}
        for v in [c for c in list(chars) if unicodedata.normalize("NFD", c)[0].lower() in "aeiouy"]:
            dec = unicodedata.normalize("NFD", v)
            base, lm = dec[0], [m for m in dec[1:] if m in letter_marks]
            for t in tones:
                chars.add(unicodedata.normalize("NFC", base + "".join(lm) + t))
    if code == "ko":
        chars.update(chr(u) for u in range(0xAC00, 0xD7A4))
    return chars


def charset_ok(word: str, lang: str) -> bool:
    """True if every char of the (already-normalized) answer is typeable.

    Mandarin entries are pinyin (the digits 1-4 and ü are on its layout); Thai
    allows the whole Thai block. Filipino allows the hyphen.
    """
    reach = _reachable(lang)
    if lang == "zh":
        reach = reach | set("1234ü")
    for c in word:
        if c in reach:
            continue
        if lang == "th" and 0x0E00 <= ord(c) <= 0x0E7F:
            continue
        if lang == "fil" and c == "-":
            continue
        return False
    return True
