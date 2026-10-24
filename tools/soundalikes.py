#!/usr/bin/env python3
"""CC-SENSE-CUE v1.1 F7 -- the ONE read API for sound-alike tables (Python side).

Every Python consumer reads sound-alike sets through soundalikes(); nothing else
opens assets/words/<lang>/soundalikes.json (scripts/soundalike-api-check.mjs
enforces it). The Rust side is src/soundalikes.rs and the node side is
scripts/lib/soundalikes.mjs; all three read the same files the same way.

    soundalikes(lang, voice=None) -> {"status", "voice", "generator", "sets": [...]}

status is "verified", "none-by-construction" or "missing" (F2). A language with
no file reads as "missing" with no sets. Passing a voice that differs from the
one the table was built for also reads as "missing" (I6) -- but the sets are
still returned, because H1 keeps Normal grading exactly as it is today.
"""
import hashlib
import json
import pathlib
import re
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parent.parent
# The registry's fifteen languages (src/consts.rs LANGS_BASE). Every one gets a
# table file, so "checked and empty" and "never built" are told apart (I4).
LANGS = ("en", "es", "fr", "de", "pt", "pl", "vi", "ko", "ja", "fil", "zh", "ru", "ar", "sw", "hi", "fa")
STATUSES = ("verified", "none-by-construction", "missing")
KINDS = ("true", "accent")


def table_path(lang, root=ROOT, variant=None):
    """variant "retier": English's second table, built over the current bank plus the
    re-tier rows (CC-EN-EXPERT-GROWTH), read only while en.retier is on."""
    name = f"soundalikes-{variant}.json" if variant else "soundalikes.json"
    return pathlib.Path(root) / "assets" / "words" / lang / name


def nfc(s):
    return unicodedata.normalize("NFC", s)


def set_id(lang, members):
    """[S1] Stable set identity: the language plus the sorted NFC member list.
    An unchanged set regenerates to the same id (I2)."""
    body = "\n".join(sorted(nfc(m) for m in members))
    return f"{lang}:{hashlib.sha256(body.encode('utf-8')).hexdigest()[:12]}"


def layer1_voices(root=ROOT):
    """lang -> voice id, parsed from backend/app.py (LANG_VOICES and AZURE_VOICES).
    The backend is the single source of truth for which voice a language plays."""
    src = (pathlib.Path(root) / "backend" / "app.py").read_text(encoding="utf-8")
    out = {}
    for name in ("LANG_VOICES", "AZURE_VOICES"):
        m = re.search(rf"^{name} = \{{(.*?)^\}}", src, re.S | re.M)
        if not m:
            continue
        for lang, voice in re.findall(r'"([a-z]+)":\s*\("[^"]+",\s*"([^"]+)"\)', m.group(1)):
            out[lang] = voice
    return out


def soundalikes(lang, voice=None, root=ROOT, variant=None):
    p = table_path(lang, root, variant)
    if not p.exists():
        return {"lang": lang, "status": "missing", "voice": None, "generator": None, "sets": []}
    t = json.loads(p.read_text(encoding="utf-8"))
    if voice is not None and t.get("voice") != voice:
        t = dict(t, status="missing", voice_mismatch=True)
    return t


def member_words(s):
    return [m["memberId"] for m in s["members"]]


def sets_of_kind(lang, kinds=("true",), voice=None, root=ROOT, variant=None):
    """Member-word lists of every set whose kind is in `kinds`.
    SENSE-CUE grading, the bonus and audio exemptions read ("true",);
    CC-IMPOSSIBLE reads ("true", "accent")."""
    return [member_words(s) for s in soundalikes(lang, voice, root, variant)["sets"] if s["kind"] in kinds]
